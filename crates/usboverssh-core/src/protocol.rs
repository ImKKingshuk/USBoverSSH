//! USB/IP Protocol Implementation
//!
//! Implements the USB/IP protocol for sharing USB devices over network.

use crate::device::{DeviceInfo, DeviceSpeed};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// USB/IP protocol version
pub const USBIP_VERSION: u16 = 0x0111;

/// USB/IP operation codes
/// Note: USB/IP protocol reuses some codes in different contexts; we encode them uniquely here
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    /// Request device list
    ReqDevlist,
    /// Device list reply
    RepDevlist,
    /// Import (attach) request
    ReqImport,
    /// Import reply
    RepImport,
    /// URB submit
    CmdSubmit,
    /// URB submit reply
    RetSubmit,
    /// Unlink URB
    CmdUnlink,
    /// Unlink reply 
    RetUnlink,
}

impl OpCode {
    pub fn to_u16(self) -> u16 {
        match self {
            Self::ReqDevlist => 0x8005,
            Self::RepDevlist => 0x0005,
            Self::ReqImport => 0x8003,
            Self::RepImport => 0x0003,
            Self::CmdSubmit => 0x0001,
            Self::RetSubmit => 0x0003, // Same as RepImport, context-dependent
            Self::CmdUnlink => 0x0002,
            Self::RetUnlink => 0x0004,
        }
    }

    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x8005 => Some(Self::ReqDevlist),
            0x0005 => Some(Self::RepDevlist),
            0x8003 => Some(Self::ReqImport),
            0x0001 => Some(Self::CmdSubmit),
            0x0002 => Some(Self::CmdUnlink),
            0x0004 => Some(Self::RetUnlink),
            0x0003 => Some(Self::RepImport), // Could be RetSubmit depending on context
            _ => None,
        }
    }
}


/// USB/IP device status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum DeviceStatus {
    Available = 1,
    Used = 2,
    Error = 3,
}

/// USB/IP protocol header (common for all messages)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbIpHeader {
    /// Protocol version
    pub version: u16,
    /// Operation code
    pub code: u16,
    /// Status (reply) or reserved (request)
    pub status: u32,
}

impl UsbIpHeader {
    /// Create a new request header
    pub fn request(code: OpCode) -> Self {
        Self {
            version: USBIP_VERSION,
            code: code.to_u16(),
            status: 0,
        }
    }

    /// Create a new reply header
    pub fn reply(code: OpCode, status: u32) -> Self {
        Self {
            version: USBIP_VERSION,
            code: code.to_u16(),
            status,
        }
    }

    /// Serialize to network byte order
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut buf = [0u8; 8];
        buf[0..2].copy_from_slice(&self.version.to_be_bytes());
        buf[2..4].copy_from_slice(&self.code.to_be_bytes());
        buf[4..8].copy_from_slice(&self.status.to_be_bytes());
        buf
    }

    /// Deserialize from network byte order
    pub fn from_bytes(buf: &[u8; 8]) -> Self {
        Self {
            version: u16::from_be_bytes([buf[0], buf[1]]),
            code: u16::from_be_bytes([buf[2], buf[3]]),
            status: u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]),
        }
    }

    /// Read from async stream
    pub async fn read_from<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Self> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf).await.map_err(|e| {
            Error::UsbIpProtocol(format!("Failed to read header: {}", e))
        })?;
        Ok(Self::from_bytes(&buf))
    }

    /// Write to async stream
    pub async fn write_to<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_bytes()).await.map_err(|e| {
            Error::UsbIpProtocol(format!("Failed to write header: {}", e))
        })?;
        Ok(())
    }
}

/// USB/IP device descriptor (for device list)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbIpDeviceDescriptor {
    /// Sysfs path (e.g., "/sys/devices/...")
    pub path: String,
    /// Bus ID (e.g., "1-1.2")
    pub bus_id: String,
    /// Bus number
    pub bus_num: u32,
    /// Device number
    pub dev_num: u32,
    /// Device speed
    pub speed: u32,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Device revision
    pub device_revision: u16,
    /// Device class
    pub device_class: u8,
    /// Device subclass
    pub device_subclass: u8,
    /// Device protocol
    pub device_protocol: u8,
    /// Configuration value
    pub configuration_value: u8,
    /// Number of configurations
    pub num_configurations: u8,
    /// Number of interfaces
    pub num_interfaces: u8,
}

impl UsbIpDeviceDescriptor {
    /// Create from DeviceInfo
    pub fn from_device_info(device: &DeviceInfo) -> Self {
        Self {
            path: format!("/sys/bus/usb/devices/{}", device.bus_id),
            bus_id: device.bus_id.clone(),
            bus_num: device.bus_num as u32,
            dev_num: device.dev_num as u32,
            speed: device.speed.to_usbip_speed(),
            vendor_id: device.vendor_id,
            product_id: device.product_id,
            device_revision: 0,
            device_class: 0,
            device_subclass: 0,
            device_protocol: 0,
            configuration_value: 1,
            num_configurations: device.num_configurations,
            num_interfaces: 1,
        }
    }

    /// Serialize to bytes (312 bytes fixed size)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = vec![0u8; 312];
        
        // Path (256 bytes, null-terminated)
        let path_bytes = self.path.as_bytes();
        let path_len = path_bytes.len().min(255);
        buf[0..path_len].copy_from_slice(&path_bytes[..path_len]);
        
        // Bus ID (32 bytes, null-terminated)
        let bus_id_bytes = self.bus_id.as_bytes();
        let bus_id_len = bus_id_bytes.len().min(31);
        buf[256..256 + bus_id_len].copy_from_slice(&bus_id_bytes[..bus_id_len]);
        
        // Numeric fields (network byte order)
        buf[288..292].copy_from_slice(&self.bus_num.to_be_bytes());
        buf[292..296].copy_from_slice(&self.dev_num.to_be_bytes());
        buf[296..300].copy_from_slice(&self.speed.to_be_bytes());
        
        buf[300..302].copy_from_slice(&self.vendor_id.to_be_bytes());
        buf[302..304].copy_from_slice(&self.product_id.to_be_bytes());
        buf[304..306].copy_from_slice(&self.device_revision.to_be_bytes());
        
        buf[306] = self.device_class;
        buf[307] = self.device_subclass;
        buf[308] = self.device_protocol;
        buf[309] = self.configuration_value;
        buf[310] = self.num_configurations;
        buf[311] = self.num_interfaces;
        
        buf
    }

    /// Deserialize from bytes
    pub fn from_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < 312 {
            return Err(Error::UsbIpProtocol("Device descriptor too short".into()));
        }
        
        // Read path (null-terminated)
        let path = read_cstring(&buf[0..256]);
        let bus_id = read_cstring(&buf[256..288]);
        
        Ok(Self {
            path,
            bus_id,
            bus_num: u32::from_be_bytes([buf[288], buf[289], buf[290], buf[291]]),
            dev_num: u32::from_be_bytes([buf[292], buf[293], buf[294], buf[295]]),
            speed: u32::from_be_bytes([buf[296], buf[297], buf[298], buf[299]]),
            vendor_id: u16::from_be_bytes([buf[300], buf[301]]),
            product_id: u16::from_be_bytes([buf[302], buf[303]]),
            device_revision: u16::from_be_bytes([buf[304], buf[305]]),
            device_class: buf[306],
            device_subclass: buf[307],
            device_protocol: buf[308],
            configuration_value: buf[309],
            num_configurations: buf[310],
            num_interfaces: buf[311],
        })
    }
}

/// USB/IP interface descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbIpInterfaceDescriptor {
    pub interface_class: u8,
    pub interface_subclass: u8,
    pub interface_protocol: u8,
    pub padding: u8,
}

impl UsbIpInterfaceDescriptor {
    pub fn to_bytes(&self) -> [u8; 4] {
        [
            self.interface_class,
            self.interface_subclass,
            self.interface_protocol,
            self.padding,
        ]
    }

    pub fn from_bytes(buf: &[u8; 4]) -> Self {
        Self {
            interface_class: buf[0],
            interface_subclass: buf[1],
            interface_protocol: buf[2],
            padding: buf[3],
        }
    }
}

/// USB/IP URB command header
#[derive(Debug, Clone)]
pub struct UsbIpCmdSubmit {
    /// Sequence number
    pub seqnum: u32,
    /// Device ID (bus << 16 | dev)
    pub devid: u32,
    /// Direction (0 = out, 1 = in)
    pub direction: u32,
    /// Endpoint number
    pub ep: u32,
    /// Transfer flags
    pub transfer_flags: u32,
    /// Transfer buffer length
    pub transfer_buffer_length: u32,
    /// Start frame (for isochronous)
    pub start_frame: u32,
    /// Number of ISO packets
    pub number_of_packets: u32,
    /// Interval
    pub interval: u32,
    /// Setup packet (8 bytes)
    pub setup: [u8; 8],
}

/// USB/IP protocol handler
pub struct UsbIpProtocol;

impl UsbIpProtocol {
    /// Create import (attach) request
    pub fn create_import_request(bus_id: &str) -> Vec<u8> {
        let mut buf = Vec::with_capacity(40);
        
        // Header
        let header = UsbIpHeader::request(OpCode::ReqImport);
        buf.extend_from_slice(&header.to_bytes());
        
        // Bus ID (32 bytes, null-terminated)
        let mut bus_id_buf = [0u8; 32];
        let bus_id_bytes = bus_id.as_bytes();
        let len = bus_id_bytes.len().min(31);
        bus_id_buf[..len].copy_from_slice(&bus_id_bytes[..len]);
        buf.extend_from_slice(&bus_id_buf);
        
        buf
    }

    /// Create device list request
    pub fn create_devlist_request() -> Vec<u8> {
        let header = UsbIpHeader::request(OpCode::ReqDevlist);
        header.to_bytes().to_vec()
    }

    /// Parse import reply
    pub async fn parse_import_reply<R: AsyncRead + Unpin>(
        reader: &mut R
    ) -> Result<UsbIpDeviceDescriptor> {
        // Read header
        let header = UsbIpHeader::read_from(reader).await?;
        
        if header.status != 0 {
            return Err(Error::UsbIpAttach(format!(
                "Import failed with status: {}",
                header.status
            )));
        }
        
        // Read device descriptor
        let mut buf = [0u8; 312];
        reader.read_exact(&mut buf).await.map_err(|e| {
            Error::UsbIpProtocol(format!("Failed to read device descriptor: {}", e))
        })?;
        
        UsbIpDeviceDescriptor::from_bytes(&buf)
    }

    /// Parse device list reply
    pub async fn parse_devlist_reply<R: AsyncRead + Unpin>(
        reader: &mut R
    ) -> Result<Vec<UsbIpDeviceDescriptor>> {
        // Read header
        let header = UsbIpHeader::read_from(reader).await?;
        
        if header.status != 0 {
            return Err(Error::UsbIpProtocol(format!(
                "Device list failed with status: {}",
                header.status
            )));
        }
        
        // Read device count
        let mut count_buf = [0u8; 4];
        reader.read_exact(&mut count_buf).await.map_err(|e| {
            Error::UsbIpProtocol(format!("Failed to read device count: {}", e))
        })?;
        let count = u32::from_be_bytes(count_buf);
        
        let mut devices = Vec::with_capacity(count as usize);
        
        for _ in 0..count {
            // Read device descriptor
            let mut buf = [0u8; 312];
            reader.read_exact(&mut buf).await.map_err(|e| {
                Error::UsbIpProtocol(format!("Failed to read device: {}", e))
            })?;
            
            let device = UsbIpDeviceDescriptor::from_bytes(&buf)?;
            
            // Read interface descriptors
            for _ in 0..device.num_interfaces {
                let mut iface_buf = [0u8; 4];
                reader.read_exact(&mut iface_buf).await.map_err(|e| {
                    Error::UsbIpProtocol(format!("Failed to read interface: {}", e))
                })?;
            }
            
            devices.push(device);
        }
        
        Ok(devices)
    }
}

/// Helper to read null-terminated C string from buffer
fn read_cstring(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_serialization() {
        let header = UsbIpHeader::request(OpCode::ReqDevlist);
        let bytes = header.to_bytes();
        let parsed = UsbIpHeader::from_bytes(&bytes);
        
        assert_eq!(parsed.version, USBIP_VERSION);
        assert_eq!(parsed.code, OpCode::ReqDevlist as u16);
    }

    #[test]
    fn test_device_descriptor() {
        use crate::device::{DeviceClass, DeviceSpeed};
        
        let device = DeviceInfo {
            bus_id: "1-1.2".to_string(),
            vendor_id: 0x1234,
            product_id: 0x5678,
            device_class: DeviceClass::Hid,
            bus_num: 1,
            dev_num: 3,
            speed: DeviceSpeed::High,
            manufacturer: Some("Test".to_string()),
            product: Some("Device".to_string()),
            serial: None,
            num_configurations: 1,
            is_attached: false,
            is_bound: false,
        };
        
        let desc = UsbIpDeviceDescriptor::from_device_info(&device);
        let bytes = desc.to_bytes();
        let parsed = UsbIpDeviceDescriptor::from_bytes(&bytes).unwrap();
        
        assert_eq!(parsed.bus_id, "1-1.2");
        assert_eq!(parsed.vendor_id, 0x1234);
        assert_eq!(parsed.product_id, 0x5678);
    }
}
