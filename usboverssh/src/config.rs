//! Configuration Management
//!
//! Handles loading, saving, and managing USBoverSSH configuration.

use crate::device::DeviceFilter;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Validate a file path for security
fn validate_file_path(path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy();

    // Check for path traversal attempts
    if path_str.contains("..") {
        return Err(Error::Config(format!(
            "Path traversal detected in '{}'",
            path_str
        )));
    }

    // Check for null bytes
    if path_str.contains('\0') {
        return Err(Error::Config(format!(
            "Null byte detected in path '{}'",
            path_str
        )));
    }

    Ok(())
}

/// Validate host configuration
fn validate_host_config(name: &str, host: &HostConfig) -> Result<()> {
    // Validate host name is not empty
    if name.is_empty() {
        return Err(Error::Config("Host name cannot be empty".to_string()));
    }

    // Validate hostname is not empty
    if host.hostname.is_empty() {
        return Err(Error::Config(format!("Host '{}' has empty hostname", name)));
    }

    // Validate port range
    if host.port == 0 {
        return Err(Error::Config(format!("Host '{}' has invalid port 0", name)));
    }

    // Validate username is not empty
    if host.user.is_empty() {
        return Err(Error::Config(format!("Host '{}' has empty username", name)));
    }

    // Validate identity file path if specified
    if let Some(ref path) = host.identity_file {
        validate_file_path(path)?;
    }

    Ok(())
}

/// Validate auto-attach rule
fn validate_auto_attach_rule(index: usize, rule: &AutoAttachRule) -> Result<()> {
    // Validate rule name is not empty
    if rule.name.is_empty() {
        return Err(Error::Config(format!(
            "Auto-attach rule at index {} has empty name",
            index
        )));
    }

    // Validate target host is not empty
    if rule.host.is_empty() {
        return Err(Error::Config(format!(
            "Auto-attach rule '{}' has empty target host",
            rule.name
        )));
    }

    Ok(())
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

        let config: Self = toml::from_str(&contents)
            .map_err(|e| Error::ConfigParse(format!("Invalid config: {}", e)))?;

        config.validate()?;
        Ok(config)
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

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate general settings
        self.general.validate()?;

        // Validate SSH settings
        self.ssh.validate()?;

        // Validate logging settings
        self.logging.validate()?;

        // Validate TUI settings
        self.tui.validate()?;

        // Validate all host configurations
        for (name, host) in &self.hosts {
            validate_host_config(name, host)?;
        }

        // Validate auto-attach rules
        for (i, rule) in self.auto_attach.iter().enumerate() {
            validate_auto_attach_rule(i, rule)?;
        }

        Ok(())
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

impl GeneralConfig {
    pub fn validate(&self) -> Result<()> {
        if self.reconnect_delay == 0 {
            return Err(Error::Config(
                "reconnect_delay must be greater than 0".to_string(),
            ));
        }
        if self.reconnect_delay > 3600 {
            return Err(Error::Config(
                "reconnect_delay must be less than 3600 seconds (1 hour)".to_string(),
            ));
        }
        if self.connection_timeout == 0 {
            return Err(Error::Config(
                "connection_timeout must be greater than 0".to_string(),
            ));
        }
        if self.connection_timeout > 3600 {
            return Err(Error::Config(
                "connection_timeout must be less than 3600 seconds (1 hour)".to_string(),
            ));
        }
        Ok(())
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

impl SshConfig {
    pub fn validate(&self) -> Result<()> {
        // Validate port range (1-65535)
        if self.default_port == 0 {
            return Err(Error::Config("default_port cannot be 0".to_string()));
        }

        // Validate keepalive interval
        if self.keepalive_interval == 0 {
            return Err(Error::Config(
                "keepalive_interval must be greater than 0".to_string(),
            ));
        }
        if self.keepalive_interval > 3600 {
            return Err(Error::Config(
                "keepalive_interval must be less than 3600 seconds (1 hour)".to_string(),
            ));
        }

        Ok(())
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

impl LoggingConfig {
    pub fn validate(&self) -> Result<()> {
        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.level.as_str()) {
            return Err(Error::Config(format!(
                "Invalid log level '{}'. Must be one of: {}",
                self.level,
                valid_levels.join(", ")
            )));
        }

        // Validate log format
        let valid_formats = ["text", "json"];
        if !valid_formats.contains(&self.format.as_str()) {
            return Err(Error::Config(format!(
                "Invalid log format '{}'. Must be one of: {}",
                self.format,
                valid_formats.join(", ")
            )));
        }

        // Validate file path if specified
        if let Some(ref path) = self.file {
            validate_file_path(path)?;
        }

        Ok(())
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

impl TuiConfig {
    pub fn validate(&self) -> Result<()> {
        // Validate refresh interval
        if self.refresh_interval < 100 {
            return Err(Error::Config(
                "refresh_interval must be at least 100ms".to_string(),
            ));
        }
        if self.refresh_interval > 60000 {
            return Err(Error::Config(
                "refresh_interval must be less than 60000ms (1 minute)".to_string(),
            ));
        }

        Ok(())
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
