//! TUI Application State

use std::collections::HashMap;
use std::time::{Duration, Instant};
use usboverssh_core::{Config, DeviceInfo, DeviceManager};

/// Active pane in the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pane {
    LocalDevices,
    RemoteDevices,
    AttachedDevices,
    Hosts,
}

/// Popup dialog type
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Popup {
    None,
    Help,
    Connect,
    Error(String),
    Confirm { title: String, message: String },
}

/// Host connection status
#[derive(Debug, Clone)]
pub struct HostStatus {
    pub name: String,
    pub hostname: String,
    pub connected: bool,
    #[allow(dead_code)]
    pub devices: Vec<DeviceInfo>,
    #[allow(dead_code)]
    pub last_error: Option<String>,
}

/// Main application state
#[derive(Debug, Clone)]
pub struct App {
    /// Configuration
    #[allow(dead_code)]
    pub config: Config,
    /// Current active pane
    pub active_pane: Pane,
    /// Local USB devices
    pub local_devices: Vec<DeviceInfo>,
    /// Remote devices by host
    pub remote_devices: HashMap<String, Vec<DeviceInfo>>,
    /// Attached devices
    pub attached_devices: Vec<AttachedDevice>,
    /// Connected hosts
    pub hosts: Vec<HostStatus>,
    /// Selected index per pane
    pub selected: HashMap<Pane, usize>,
    /// Current popup
    pub popup: Popup,
    /// Status bar message
    pub status_message: Option<(String, Instant)>,
    /// Show status panel
    pub show_status_panel: bool,
    /// Last refresh time
    pub last_refresh: Instant,
    /// Refresh interval
    pub refresh_interval: Duration,
}

/// Attached device info
#[derive(Debug, Clone)]
pub struct AttachedDevice {
    pub port: u32,
    pub bus_id: String,
    pub host: String,
    pub speed: String,
}

impl App {
    /// Create new app state
    pub fn new(config: Config) -> Self {
        let refresh_interval = Duration::from_millis(config.tui.refresh_interval);

        let mut selected = HashMap::new();
        selected.insert(Pane::LocalDevices, 0);
        selected.insert(Pane::RemoteDevices, 0);
        selected.insert(Pane::AttachedDevices, 0);
        selected.insert(Pane::Hosts, 0);

        // Initialize hosts from config
        let hosts: Vec<HostStatus> = config
            .hosts
            .iter()
            .map(|(name, host)| HostStatus {
                name: name.clone(),
                hostname: host.hostname.clone(),
                connected: false,
                devices: Vec::new(),
                last_error: None,
            })
            .collect();

        Self {
            config,
            active_pane: Pane::LocalDevices,
            local_devices: Vec::new(),
            remote_devices: HashMap::new(),
            attached_devices: Vec::new(),
            hosts,
            selected,
            popup: Popup::None,
            status_message: None,
            show_status_panel: true,
            last_refresh: Instant::now(),
            refresh_interval,
        }
    }

    /// Check if popup is open
    pub fn is_popup_open(&self) -> bool {
        !matches!(self.popup, Popup::None)
    }

    /// Close current popup
    pub fn close_popup(&mut self) {
        self.popup = Popup::None;
    }

    /// Toggle help popup
    pub fn toggle_help(&mut self) {
        if matches!(self.popup, Popup::Help) {
            self.popup = Popup::None;
        } else {
            self.popup = Popup::Help;
        }
    }

    /// Switch to next pane
    pub fn next_pane(&mut self) {
        self.active_pane = match self.active_pane {
            Pane::LocalDevices => Pane::RemoteDevices,
            Pane::RemoteDevices => Pane::AttachedDevices,
            Pane::AttachedDevices => Pane::Hosts,
            Pane::Hosts => Pane::LocalDevices,
        };
    }

    /// Switch to previous pane
    pub fn prev_pane(&mut self) {
        self.active_pane = match self.active_pane {
            Pane::LocalDevices => Pane::Hosts,
            Pane::RemoteDevices => Pane::LocalDevices,
            Pane::AttachedDevices => Pane::RemoteDevices,
            Pane::Hosts => Pane::AttachedDevices,
        };
    }

    /// Get item count for current pane
    fn current_item_count(&self) -> usize {
        match self.active_pane {
            Pane::LocalDevices => self.local_devices.len(),
            Pane::RemoteDevices => self.remote_devices.values().map(|v| v.len()).sum(),
            Pane::AttachedDevices => self.attached_devices.len(),
            Pane::Hosts => self.hosts.len(),
        }
    }

    /// Select previous item
    pub fn select_prev(&mut self) {
        let count = self.current_item_count();
        if count == 0 {
            return;
        }

        let current = self.selected.get(&self.active_pane).copied().unwrap_or(0);
        let new = if current == 0 { count - 1 } else { current - 1 };
        self.selected.insert(self.active_pane, new);
    }

    /// Select next item
    pub fn select_next(&mut self) {
        let count = self.current_item_count();
        if count == 0 {
            return;
        }

        let current = self.selected.get(&self.active_pane).copied().unwrap_or(0);
        let new = (current + 1) % count;
        self.selected.insert(self.active_pane, new);
    }

    /// Activate selected item
    pub async fn activate_selected(&mut self) {
        match self.active_pane {
            Pane::LocalDevices => {
                // Show device details
            }
            Pane::RemoteDevices => {
                // Attach device
                self.attach_selected().await;
            }
            Pane::AttachedDevices => {
                // Detach device
                self.detach_selected().await;
            }
            Pane::Hosts => {
                // Connect/disconnect host
                let idx = self.selected.get(&Pane::Hosts).copied().unwrap_or(0);
                if idx < self.hosts.len() {
                    let host = &self.hosts[idx];
                    if host.connected {
                        self.set_status(format!("Disconnecting from {}...", host.name));
                    } else {
                        self.set_status(format!("Connecting to {}...", host.name));
                    }
                }
            }
        }
    }

    /// Refresh local devices
    pub async fn refresh_devices(&mut self) {
        self.set_status("Refreshing devices...".to_string());

        // Refresh local devices
        if let Ok(mut manager) = DeviceManager::new() {
            if let Ok(devices) = manager.list_devices() {
                self.local_devices = devices.to_vec();
            }
        }

        // Refresh attached devices
        self.refresh_attached();

        self.last_refresh = Instant::now();
        self.set_status("Devices refreshed".to_string());
    }

    /// Refresh attached devices
    fn refresh_attached(&mut self) {
        self.attached_devices.clear();

        #[cfg(target_os = "linux")]
        {
            use std::fs;
            use std::path::Path;

            let vhci_base = Path::new("/sys/bus/usb/devices/platform");

            for entry in fs::read_dir(vhci_base).into_iter().flatten().flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with("vhci_hcd") {
                    continue;
                }

                for status_entry in fs::read_dir(entry.path()).into_iter().flatten().flatten() {
                    let status_name = status_entry.file_name().to_string_lossy().to_string();
                    if !status_name.starts_with("status") {
                        continue;
                    }

                    if let Ok(content) = fs::read_to_string(status_entry.path()) {
                        for line in content.lines().skip(1) {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 7 {
                                let port: u32 = parts[1].parse().unwrap_or(0);
                                let status: u32 = parts[2].parse().unwrap_or(0);
                                let speed: u32 = parts[3].parse().unwrap_or(0);
                                let bus_id = parts[6];

                                if status == 6 {
                                    self.attached_devices.push(AttachedDevice {
                                        port,
                                        bus_id: bus_id.to_string(),
                                        host: "remote".to_string(),
                                        speed: speed.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Attach selected device
    pub async fn attach_selected(&mut self) {
        self.set_status("Attaching device...".to_string());
        // Implementation would use SshSession and VHCI
    }

    /// Detach selected device
    pub async fn detach_selected(&mut self) {
        let idx = self
            .selected
            .get(&Pane::AttachedDevices)
            .copied()
            .unwrap_or(0);
        if idx >= self.attached_devices.len() {
            return;
        }

        let device = &self.attached_devices[idx];
        self.set_status(format!("Detaching {}...", device.bus_id));

        #[cfg(target_os = "linux")]
        {
            use std::fs;
            use std::path::Path;

            let vhci_base = Path::new("/sys/bus/usb/devices/platform");

            for entry in fs::read_dir(vhci_base).into_iter().flatten().flatten() {
                let detach_path = entry.path().join("detach");
                if detach_path.exists() {
                    if fs::write(&detach_path, device.port.to_string()).is_ok() {
                        self.set_status(format!("Detached {}", device.bus_id));
                        break;
                    }
                }
            }
        }

        self.refresh_attached();
    }

    /// Open connect dialog
    pub fn open_connect_dialog(&mut self) {
        self.popup = Popup::Connect;
    }

    /// Open hosts panel
    pub fn open_hosts_panel(&mut self) {
        self.active_pane = Pane::Hosts;
    }

    /// Toggle status panel
    pub fn toggle_status_panel(&mut self) {
        self.show_status_panel = !self.show_status_panel;
    }

    /// Set status message
    pub fn set_status(&mut self, message: String) {
        self.status_message = Some((message, Instant::now()));
    }

    /// Connect to all hosts in config
    pub async fn connect_all_hosts(&mut self) {
        for host in &mut self.hosts {
            host.connected = false; // Would actually connect
        }
    }

    /// Periodic tick
    pub async fn tick(&mut self) {
        // Clear old status messages
        if let Some((_, instant)) = &self.status_message {
            if instant.elapsed() > Duration::from_secs(3) {
                self.status_message = None;
            }
        }

        // Auto-refresh
        if self.last_refresh.elapsed() > self.refresh_interval {
            self.refresh_attached();
            self.last_refresh = Instant::now();
        }
    }
}
