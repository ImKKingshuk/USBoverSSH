//! Audit Logging Module
//!
//! Provides structured logging for security-relevant events with JSON format and rotation.

use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tracing::{debug, info};

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Timestamp in UTC
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event: AuditEvent,
    /// User who performed the action (if available)
    pub user: Option<String>,
    /// Host (if applicable)
    pub host: Option<String>,
    /// Device identifier (if applicable)
    pub device: Option<String>,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuditEvent {
    /// Device attached
    DeviceAttach {
        device: String,
        host: String,
        persistent: bool,
    },
    /// Device detached
    DeviceDetach { device: String, host: String },
    /// SSH connection established
    SshConnect {
        host: String,
        user: String,
        method: String,
    },
    /// SSH connection failed
    SshConnectFailed {
        host: String,
        user: String,
        reason: String,
    },
    /// SSH connection closed
    SshDisconnect { host: String, duration_secs: u64 },
    /// USB/IP server started
    ServerStart { address: String, port: u16 },
    /// USB/IP server stopped
    ServerStop,
    /// Configuration loaded
    ConfigLoad { path: String },
    /// Configuration modified
    ConfigModify { path: String, changes: Vec<String> },
    /// Authentication attempt
    AuthAttempt {
        user: String,
        success: bool,
        method: String,
    },
    /// Rate limit exceeded
    RateLimitExceeded { client: String, endpoint: String },
}

/// Audit logger configuration
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Log file path
    pub log_path: PathBuf,
    /// Maximum log file size in bytes (default: 10MB)
    pub max_file_size: u64,
    /// Maximum number of log files to keep (default: 5)
    pub max_files: usize,
    /// Enable JSON format
    pub json_format: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        let mut log_path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        log_path.push(".usboverssh");
        log_path.push("audit.log");

        Self {
            log_path,
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_files: 5,
            json_format: true,
        }
    }
}

/// Audit logger
#[derive(Debug)]
pub struct AuditLogger {
    config: AuditConfig,
}

impl AuditLogger {
    /// Create new audit logger
    pub fn new(config: AuditConfig) -> Result<Self> {
        // Ensure log directory exists
        if let Some(parent) = config.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        Ok(Self { config })
    }

    /// Create with default config
    pub fn with_defaults() -> Result<Self> {
        Self::new(AuditConfig::default())
    }

    /// Log an audit entry
    pub fn log(&self, entry: AuditEntry) -> Result<()> {
        self.rotate_if_needed()?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.log_path)?;

        if self.config.json_format {
            let json = serde_json::to_string(&entry)
                .map_err(|e| crate::error::Error::Other(e.to_string()))?;
            writeln!(file, "{}", json)?;
        } else {
            writeln!(file, "[{}] {:?}", entry.timestamp, entry.event)?;
        }

        file.flush()?;
        Ok(())
    }

    /// Log device attach event
    pub fn log_device_attach(&self, device: String, host: String, persistent: bool) -> Result<()> {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            event: AuditEvent::DeviceAttach {
                device: device.clone(),
                host: host.clone(),
                persistent,
            },
            user: None,
            host: Some(host),
            device: Some(device),
            metadata: serde_json::json!({}),
        };
        self.log(entry)
    }

    /// Log device detach event
    pub fn log_device_detach(&self, device: String, host: String) -> Result<()> {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            event: AuditEvent::DeviceDetach {
                device: device.clone(),
                host: host.clone(),
            },
            user: None,
            host: Some(host),
            device: Some(device),
            metadata: serde_json::json!({}),
        };
        self.log(entry)
    }

    /// Log SSH connect event
    pub fn log_ssh_connect(&self, host: String, user: String, method: String) -> Result<()> {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            event: AuditEvent::SshConnect {
                host: host.clone(),
                user: user.clone(),
                method: method.clone(),
            },
            user: Some(user),
            host: Some(host),
            device: None,
            metadata: serde_json::json!({}),
        };
        self.log(entry)
    }

    /// Log SSH connect failed event
    pub fn log_ssh_connect_failed(&self, host: String, user: String, reason: String) -> Result<()> {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            event: AuditEvent::SshConnectFailed {
                host: host.clone(),
                user: user.clone(),
                reason: reason.clone(),
            },
            user: Some(user),
            host: Some(host),
            device: None,
            metadata: serde_json::json!({ "reason": reason }),
        };
        self.log(entry)
    }

    /// Log SSH disconnect event
    pub fn log_ssh_disconnect(&self, host: String, duration_secs: u64) -> Result<()> {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            event: AuditEvent::SshDisconnect {
                host: host.clone(),
                duration_secs,
            },
            user: None,
            host: Some(host),
            device: None,
            metadata: serde_json::json!({ "duration_secs": duration_secs }),
        };
        self.log(entry)
    }

    /// Check and rotate log file if needed
    fn rotate_if_needed(&self) -> Result<()> {
        if !self.config.log_path.exists() {
            return Ok(());
        }

        let metadata = std::fs::metadata(&self.config.log_path)?;
        let file_size = metadata.len();

        if file_size >= self.config.max_file_size {
            self.rotate_logs()?;
        }

        Ok(())
    }

    /// Rotate log files
    fn rotate_logs(&self) -> Result<()> {
        // Remove oldest log if we have too many
        if self.config.max_files > 0 {
            let oldest = format!(
                "{}.{}",
                self.config.log_path.display(),
                self.config.max_files
            );
            if Path::new(&oldest).exists() {
                std::fs::remove_file(&oldest)?;
            }

            // Shift existing logs
            for i in (1..self.config.max_files).rev() {
                let old_name = format!("{}.{}", self.config.log_path.display(), i);
                let new_name = format!("{}.{}", self.config.log_path.display(), i + 1);

                if Path::new(&old_name).exists() {
                    std::fs::rename(&old_name, &new_name)?;
                }
            }

            // Move current log to .1
            let rotated = format!("{}.1", self.config.log_path.display());
            std::fs::rename(&self.config.log_path, &rotated)?;
        }

        Ok(())
    }
}

/// Global audit logger instance
static GLOBAL_LOGGER: OnceLock<AuditLogger> = OnceLock::new();

/// Initialize global audit logger
pub fn init_global_logger(config: AuditConfig) -> Result<()> {
    let logger = AuditLogger::new(config)?;
    GLOBAL_LOGGER
        .set(logger)
        .expect("Failed to set global logger");
    info!("Audit logger initialized");
    Ok(())
}

/// Get global audit logger
pub fn global_logger() -> Option<&'static AuditLogger> {
    GLOBAL_LOGGER.get()
}

/// Convenience function to log to global logger
pub fn log(entry: AuditEntry) -> Result<()> {
    if let Some(logger) = global_logger() {
        logger.log(entry)
    } else {
        debug!("Audit logger not initialized, skipping log");
        Ok(())
    }
}
