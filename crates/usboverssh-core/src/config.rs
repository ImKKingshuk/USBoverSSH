//! Configuration Management
//!
//! Handles loading, saving, and managing USBoverSSH configuration.

use crate::error::{Error, Result};
use crate::device::DeviceFilter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// General settings
    pub general: GeneralConfig,
    /// SSH settings
    pub ssh: SshConfig,
    /// Logging settings
    pub logging: LoggingConfig,
    /// TUI settings
    pub tui: TuiConfig,
    /// Named host configurations
    pub hosts: HashMap<String, HostConfig>,
    /// Auto-attach rules
    pub auto_attach: Vec<AutoAttachRule>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            ssh: SshConfig::default(),
            logging: LoggingConfig::default(),
            tui: TuiConfig::default(),
            hosts: HashMap::new(),
            auto_attach: Vec::new(),
        }
    }
}

impl Config {
    /// Load configuration from default location
    pub fn load_or_default() -> Result<Self> {
        if let Some(path) = Self::default_path() {
            if path.exists() {
                return Self::load(&path);
            }
        }
        Ok(Self::default())
    }

    /// Load configuration from a specific path
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("Failed to read config: {}", e)))?;
        
        toml::from_str(&contents)
            .map_err(|e| Error::ConfigParse(format!("Invalid config: {}", e)))
    }

    /// Save configuration to default location
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path()
            .ok_or_else(|| Error::Config("Could not determine config path".into()))?;
        self.save_to(&path)
    }

    /// Save configuration to a specific path
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::Config(format!("Failed to create config dir: {}", e)))?;
        }
        
        let contents = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(format!("Failed to serialize config: {}", e)))?;
        
        std::fs::write(path, contents)
            .map_err(|e| Error::Config(format!("Failed to write config: {}", e)))?;
        
        Ok(())
    }

    /// Get default configuration file path
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("usboverssh").join("config.toml"))
    }

    /// Get a host configuration by name or parse as user@host
    pub fn get_host(&self, name_or_spec: &str) -> HostConfig {
        // Check if it's a named host
        if let Some(host) = self.hosts.get(name_or_spec) {
            return host.clone();
        }
        
        // Parse as user@host[:port]
        HostConfig::parse(name_or_spec)
    }
}

/// General application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Reconnect delay in seconds (for persistent mode)
    pub reconnect_delay: u64,
    /// Maximum reconnection attempts (0 = unlimited)
    pub max_reconnect_attempts: u32,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Enable verbose output
    pub verbose: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            reconnect_delay: 2,
            max_reconnect_attempts: 0,
            connection_timeout: 30,
            verbose: false,
        }
    }
}

/// SSH connection settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SshConfig {
    /// Default SSH port
    pub default_port: u16,
    /// Default SSH key path
    pub identity_file: Option<PathBuf>,
    /// SSH config file path
    pub config_file: Option<PathBuf>,
    /// Enable SSH agent forwarding
    pub agent_forwarding: bool,
    /// ControlMaster socket directory
    pub control_path: Option<PathBuf>,
    /// Keep-alive interval in seconds
    pub keepalive_interval: u64,
    /// Enable strict host key checking
    pub strict_host_key_checking: bool,
}

impl Default for SshConfig {
    fn default() -> Self {
        let home = dirs::home_dir();
        Self {
            default_port: 22,
            identity_file: home.as_ref().map(|h| h.join(".ssh").join("id_ed25519")),
            config_file: home.as_ref().map(|h| h.join(".ssh").join("config")),
            agent_forwarding: true,
            control_path: dirs::runtime_dir().or_else(|| Some(PathBuf::from("/tmp"))),
            keepalive_interval: 30,
            strict_host_key_checking: true,
        }
    }
}

/// Logging settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Log format (text, json)
    pub format: String,
    /// Log file path (None for stderr)
    pub file: Option<PathBuf>,
    /// Enable colored output
    pub color: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "text".to_string(),
            file: None,
            color: true,
        }
    }
}

/// TUI appearance settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TuiConfig {
    /// Refresh interval in milliseconds
    pub refresh_interval: u64,
    /// Enable mouse support
    pub mouse: bool,
    /// Color theme (default, nord, gruvbox, etc.)
    pub theme: String,
    /// Show device serial numbers
    pub show_serial: bool,
    /// Show device speed
    pub show_speed: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            refresh_interval: 1000,
            mouse: true,
            theme: "default".to_string(),
            show_serial: true,
            show_speed: true,
        }
    }
}

/// Named host configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HostConfig {
    /// Hostname or IP address
    pub hostname: String,
    /// SSH port
    pub port: u16,
    /// SSH username
    pub user: String,
    /// SSH identity file for this host
    pub identity_file: Option<PathBuf>,
    /// USB device filters to display/attach
    pub device_filters: Vec<DeviceFilter>,
    /// Description for this host
    pub description: Option<String>,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            hostname: String::new(),
            port: 22,
            user: whoami::username(),
            identity_file: None,
            device_filters: Vec::new(),
            description: None,
        }
    }
}

impl HostConfig {
    /// Parse host specification like "user@host:port"
    pub fn parse(spec: &str) -> Self {
        let mut config = Self::default();
        
        let (user_host, port) = if let Some(idx) = spec.rfind(':') {
            if let Ok(p) = spec[idx + 1..].parse::<u16>() {
                (&spec[..idx], Some(p))
            } else {
                (spec, None)
            }
        } else {
            (spec, None)
        };
        
        if let Some(port) = port {
            config.port = port;
        }
        
        if let Some(idx) = user_host.find('@') {
            config.user = user_host[..idx].to_string();
            config.hostname = user_host[idx + 1..].to_string();
        } else {
            config.hostname = user_host.to_string();
        }
        
        config
    }

    /// Get full SSH destination string (user@host)
    pub fn ssh_destination(&self) -> String {
        format!("{}@{}", self.user, self.hostname)
    }
}

/// Auto-attach rule for hotplug events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoAttachRule {
    /// Rule name
    pub name: String,
    /// Device filter
    pub filter: DeviceFilter,
    /// Target host to attach to
    pub host: String,
    /// Enable this rule
    pub enabled: bool,
}

/// Generate example configuration file
pub fn generate_example_config() -> String {
    let example = Config {
        general: GeneralConfig::default(),
        ssh: SshConfig::default(),
        logging: LoggingConfig::default(),
        tui: TuiConfig::default(),
        hosts: {
            let mut map = HashMap::new();
            map.insert(
                "example_server".to_string(),
                HostConfig {
                    hostname: "example.com".to_string(),
                    port: 22,
                    user: "username".to_string(),
                    identity_file: Some(PathBuf::from("~/.ssh/example_key")),
                    device_filters: vec![],
                    description: Some("Example server".to_string()),
                },
            );
            map
        },
        auto_attach: vec![AutoAttachRule {
            name: "Example Device".to_string(),
            filter: DeviceFilter {
                vendor_id: Some(0x0000),
                product_id: None,
                ..Default::default()
            },
            host: "example_server".to_string(),
            enabled: false,
        }],
    };

    format!(
        "# USBoverSSH Configuration File\n\
         # Location: {}\n\n{}",
        Config::default_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "~/.config/usboverssh/config.toml".to_string()),
        toml::to_string_pretty(&example).unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_parse() {
        let config = HostConfig::parse("[email protected]:2222");
        assert_eq!(config.user, "testuser");
        assert_eq!(config.hostname, "server.local");
        assert_eq!(config.port, 2222);
    }

    #[test]
    fn test_host_parse_no_port() {
        let config = HostConfig::parse("[email protected]");
        assert_eq!(config.user, "testuser");
        assert_eq!(config.hostname, "example.com");
        assert_eq!(config.port, 22);
    }

    #[test]
    fn test_host_parse_no_user() {
        let config = HostConfig::parse("myserver");
        assert_eq!(config.hostname, "myserver");
        assert_eq!(config.port, 22);
    }
}
