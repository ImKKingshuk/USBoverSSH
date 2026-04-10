//! Error types for USBoverSSH
//!
//! Provides a comprehensive error hierarchy for all operations.

use thiserror::Error;

/// Result type alias using our custom Error
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for USBoverSSH operations
#[derive(Error, Debug)]
pub enum Error {
    // ═══════════════════════════════════════════════════════════════════════
    // Device Errors
    // ═══════════════════════════════════════════════════════════════════════
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Multiple devices match pattern '{pattern}': {matches:?}")]
    MultipleDevicesMatch {
        pattern: String,
        matches: Vec<String>,
    },

    #[error("No USB devices available")]
    NoDevicesAvailable,

    #[error("Device {0} is busy or already attached")]
    DeviceBusy(String),

    #[error("Device {0} cannot be detached - not currently attached")]
    DeviceNotAttached(String),

    #[error("USB enumeration failed: {0}")]
    UsbEnumeration(String),

    // ═══════════════════════════════════════════════════════════════════════
    // SSH/Tunnel Errors
    // ═══════════════════════════════════════════════════════════════════════
    #[error("SSH connection failed: {0}")]
    SshConnection(String),

    #[error("SSH authentication failed for user '{user}' on host '{host}'")]
    SshAuthentication { user: String, host: String },

    #[error("SSH tunnel creation failed: {0}")]
    TunnelCreation(String),

    #[error("SSH tunnel disconnected unexpectedly")]
    TunnelDisconnected,

    #[error("SSH key not found: {0}")]
    SshKeyNotFound(String),

    #[error("SSH key passphrase required")]
    SshPassphraseRequired,

    #[error("Remote host unreachable: {0}")]
    HostUnreachable(String),

    // ═══════════════════════════════════════════════════════════════════════
    // USB/IP Protocol Errors
    // ═══════════════════════════════════════════════════════════════════════
    #[error("USB/IP protocol error: {0}")]
    UsbIpProtocol(String),

    #[error("USB/IP version mismatch: expected {expected}, got {actual}")]
    UsbIpVersionMismatch { expected: u16, actual: u16 },

    #[error("USB/IP attach failed: {0}")]
    UsbIpAttach(String),

    #[error("USB/IP detach failed: {0}")]
    UsbIpDetach(String),

    #[error("VHCI port not available for speed {0:?}")]
    VhciPortUnavailable(crate::device::DeviceSpeed),

    // ═══════════════════════════════════════════════════════════════════════
    // Configuration Errors
    // ═══════════════════════════════════════════════════════════════════════
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Invalid configuration file: {0}")]
    ConfigParse(String),

    #[error("Configuration file not found: {0}")]
    ConfigNotFound(String),

    // ═══════════════════════════════════════════════════════════════════════
    // Platform Errors
    // ═══════════════════════════════════════════════════════════════════════
    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("Kernel module not loaded: {module} - run: {suggestion}")]
    KernelModuleNotLoaded { module: String, suggestion: String },

    #[error("Insufficient permissions: {0}")]
    PermissionDenied(String),

    // ═══════════════════════════════════════════════════════════════════════
    // Server Errors
    // ═══════════════════════════════════════════════════════════════════════
    #[error("Server error: {0}")]
    Server(String),

    #[error("Server already running on port {0}")]
    ServerAlreadyRunning(u16),

    #[error("Failed to bind to address: {0}")]
    ServerBindFailed(String),

    // ═══════════════════════════════════════════════════════════════════════
    // Pool Errors
    // ═══════════════════════════════════════════════════════════════════════
    #[error("Device pool error: {0}")]
    Pool(String),

    // ═══════════════════════════════════════════════════════════════════════
    // Generic Errors
    // ═══════════════════════════════════════════════════════════════════════
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Timeout after {0} seconds")]
    Timeout(u64),

    #[error("Operation cancelled")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create a new "other" error with a message
    pub fn other<S: Into<String>>(msg: S) -> Self {
        Self::Other(msg.into())
    }

    /// Check if this error is recoverable (can retry)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::TunnelDisconnected
                | Self::HostUnreachable(_)
                | Self::Timeout(_)
                | Self::SshConnection(_)
        )
    }

    /// Get a suggested fix for this error
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            Self::KernelModuleNotLoaded { suggestion, .. } => Some(suggestion),
            Self::PermissionDenied(_) => Some("Try running with sudo or as root"),
            Self::SshKeyNotFound(_) => Some("Generate a key with: ssh-keygen -t ed25519"),
            Self::SshAuthentication { .. } => {
                Some("Check your SSH key is added to the remote host's authorized_keys")
            }
            _ => None,
        }
    }
}
