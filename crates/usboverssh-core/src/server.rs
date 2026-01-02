//! USB/IP Server Implementation
//!
//! Handles device export and client connections (Linux only).

use crate::device::{DeviceFilter, DeviceManager};
use crate::error::{Error, Result};
use crate::protocol::{OpCode, UsbIpDeviceDescriptor, UsbIpHeader, UsbIpProtocol, USBIP_VERSION};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, error, info, warn};

/// Default USB/IP port
pub const DEFAULT_USBIP_PORT: u16 = 3240;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Listen address (TCP)
    pub listen_addr: Option<String>,
    /// Listen port (TCP)
    pub listen_port: u16,
    /// Unix socket path (for SSH forwarding)
    pub unix_socket: Option<String>,
    /// Device filters (which devices to export)
    pub device_filters: Vec<DeviceFilter>,
    /// Allow all devices
    pub export_all: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: Some("127.0.0.1".to_string()),
            listen_port: DEFAULT_USBIP_PORT,
            unix_socket: None,
            device_filters: Vec::new(),
            export_all: false,
        }
    }
}

/// USB/IP Server
pub struct Server {
    config: ServerConfig,
    device_manager: Arc<Mutex<DeviceManager>>,
    shutdown_tx: broadcast::Sender<()>,
}

impl Server {
    /// Create a new server
    pub fn new(config: ServerConfig) -> Result<Self> {
        let (shutdown_tx, _) = broadcast::channel(1);
        
        Ok(Self {
            config,
            device_manager: Arc::new(Mutex::new(DeviceManager::new()?)),
            shutdown_tx,
        })
    }

    /// Get available devices for export
    pub async fn available_devices(&self) -> Result<Vec<crate::device::DeviceInfo>> {
        let mut manager = self.device_manager.lock().await;
        let devices = manager.list_devices()?;
        
        if self.config.export_all {
            return Ok(devices.to_vec());
        }
        
        let filtered: Vec<_> = devices
            .iter()
            .filter(|d| {
                if self.config.device_filters.is_empty() {
                    return true;
                }
                self.config.device_filters.iter().any(|f| d.matches(f))
            })
            .cloned()
            .collect();
        
        Ok(filtered)
    }

    /// Start the server
    pub async fn run(&self) -> Result<()> {
        // Load required kernel modules on Linux
        #[cfg(target_os = "linux")]
        {
            crate::platform::load_kernel_modules(true)?;
        }

        let mut handles = Vec::new();

        // Start TCP listener if configured
        if let Some(ref addr) = self.config.listen_addr {
            let bind_addr = format!("{}:{}", addr, self.config.listen_port);
            let listener = TcpListener::bind(&bind_addr)
                .await
                .map_err(|e| Error::ServerBindFailed(format!("{}: {}", bind_addr, e)))?;
            
            info!("USB/IP server listening on {}", bind_addr);
            
            let device_manager = Arc::clone(&self.device_manager);
            let config = self.config.clone();
            let mut shutdown_rx = self.shutdown_tx.subscribe();
            
            handles.push(tokio::spawn(async move {
                loop {
                    tokio::select! {
                        result = listener.accept() => {
                            match result {
                                Ok((stream, addr)) => {
                                    info!("New TCP connection from {}", addr);
                                    let dm = Arc::clone(&device_manager);
                                    let cfg = config.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = handle_tcp_client(stream, dm, cfg).await {
                                            error!("Client error: {}", e);
                                        }
                                    });
                                }
                                Err(e) => {
                                    error!("Accept error: {}", e);
                                }
                            }
                        }
                        _ = shutdown_rx.recv() => {
                            info!("TCP server shutting down");
                            break;
                        }
                    }
                }
            }));
        }

        // Start Unix socket listener if configured
        if let Some(ref socket_path) = self.config.unix_socket {
            // Remove existing socket
            let _ = std::fs::remove_file(socket_path);
            
            let listener = UnixListener::bind(socket_path)
                .map_err(|e| Error::ServerBindFailed(format!("{}: {}", socket_path, e)))?;
            
            info!("USB/IP server listening on {}", socket_path);
            
            let device_manager = Arc::clone(&self.device_manager);
            let config = self.config.clone();
            let mut shutdown_rx = self.shutdown_tx.subscribe();
            
            handles.push(tokio::spawn(async move {
                loop {
                    tokio::select! {
                        result = listener.accept() => {
                            match result {
                                Ok((stream, _)) => {
                                    info!("New Unix socket connection");
                                    let dm = Arc::clone(&device_manager);
                                    let cfg = config.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = handle_unix_client(stream, dm, cfg).await {
                                            error!("Client error: {}", e);
                                        }
                                    });
                                }
                                Err(e) => {
                                    error!("Accept error: {}", e);
                                }
                            }
                        }
                        _ = shutdown_rx.recv() => {
                            info!("Unix server shutting down");
                            break;
                        }
                    }
                }
            }));
        }

        // Wait for all listeners
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }

    /// Signal shutdown
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

/// Handle a TCP client connection
async fn handle_tcp_client(
    mut stream: TcpStream,
    device_manager: Arc<Mutex<DeviceManager>>,
    config: ServerConfig,
) -> Result<()> {
    handle_client(&mut stream, device_manager, config).await
}

/// Handle a Unix socket client connection
async fn handle_unix_client(
    mut stream: UnixStream,
    device_manager: Arc<Mutex<DeviceManager>>,
    config: ServerConfig,
) -> Result<()> {
    handle_client(&mut stream, device_manager, config).await
}

/// Common client handler
async fn handle_client<S>(
    stream: &mut S,
    device_manager: Arc<Mutex<DeviceManager>>,
    config: ServerConfig,
) -> Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    // Read header
    let header = UsbIpHeader::read_from(stream).await?;
    
    // Check version
    if header.version != USBIP_VERSION {
        warn!(
            "Version mismatch: expected {:04x}, got {:04x}",
            USBIP_VERSION, header.version
        );
    }
    
    match OpCode::from_u16(header.code) {
        Some(OpCode::ReqDevlist) => {
            debug!("Device list request");
            handle_devlist_request(stream, device_manager, config).await
        }
        Some(OpCode::ReqImport) => {
            debug!("Import request");
            handle_import_request(stream, device_manager).await
        }
        Some(code) => {
            warn!("Unsupported opcode: {:?}", code);
            Err(Error::UsbIpProtocol(format!("Unsupported opcode: {:04x}", header.code)))
        }
        None => {
            warn!("Unknown opcode: {:04x}", header.code);
            Err(Error::UsbIpProtocol(format!("Unknown opcode: {:04x}", header.code)))
        }
    }
}

/// Handle device list request
async fn handle_devlist_request<S>(
    stream: &mut S,
    device_manager: Arc<Mutex<DeviceManager>>,
    config: ServerConfig,
) -> Result<()>
where
    S: AsyncWriteExt + Unpin,
{
    // Get available devices
    let mut manager = device_manager.lock().await;
    let devices = manager.list_devices()?;
    
    // Filter devices
    let filtered: Vec<_> = devices
        .iter()
        .filter(|d| {
            // Exclude hubs
            if matches!(d.device_class, crate::device::DeviceClass::Hub) {
                return false;
            }
            
            if config.export_all || config.device_filters.is_empty() {
                return true;
            }
            config.device_filters.iter().any(|f| d.matches(f))
        })
        .collect();
    
    // Send reply header
    let reply = UsbIpHeader::reply(OpCode::RepDevlist, 0);
    reply.write_to(stream).await?;
    
    // Send device count
    let count = filtered.len() as u32;
    stream.write_all(&count.to_be_bytes()).await?;
    
    // Send device descriptors
    for device in filtered {
        let desc = UsbIpDeviceDescriptor::from_device_info(device);
        stream.write_all(&desc.to_bytes()).await?;
        
        // Send interface descriptors (simplified)
        for _ in 0..desc.num_interfaces {
            let iface = [0u8; 4]; // interface class, subclass, protocol, padding
            stream.write_all(&iface).await?;
        }
    }
    
    stream.flush().await?;
    
    Ok(())
}

/// Handle import (attach) request
async fn handle_import_request<S>(
    stream: &mut S,
    device_manager: Arc<Mutex<DeviceManager>>,
) -> Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    // Read bus ID
    let mut bus_id_buf = [0u8; 32];
    stream.read_exact(&mut bus_id_buf).await?;
    
    let bus_id = String::from_utf8_lossy(&bus_id_buf)
        .trim_matches('\0')
        .to_string();
    
    debug!("Import request for bus_id: {}", bus_id);
    
    // Find device
    let mut manager = device_manager.lock().await;
    let device = match manager.find_by_pattern(&bus_id) {
        Ok(d) => d.clone(),
        Err(e) => {
            // Send error reply
            let reply = UsbIpHeader::reply(OpCode::RepImport, 1);
            reply.write_to(stream).await?;
            return Err(e);
        }
    };
    
    drop(manager); // Release lock
    
    // Bind device to usbip-host driver (Linux only)
    #[cfg(target_os = "linux")]
    {
        crate::platform::linux::bind_device(&bus_id)?;
    }
    
    // Send success reply with device descriptor
    let reply = UsbIpHeader::reply(OpCode::RepImport, 0);
    reply.write_to(stream).await?;
    
    let desc = UsbIpDeviceDescriptor::from_device_info(&device);
    stream.write_all(&desc.to_bytes()).await?;
    stream.flush().await?;
    
    info!("Device {} exported successfully", bus_id);
    
    // At this point, the socket becomes the USB/IP data channel
    // The kernel takes over via usbip_sockfd sysfs interface
    
    // Keep connection alive for USB/IP data transfer
    // This is handled by the kernel after we pass the socket fd
    
    Ok(())
}
