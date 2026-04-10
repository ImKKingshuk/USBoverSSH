//! USBoverSSH Core Library
//!
//! Core functionality for USB over SSH - the ultimate USB/IP implementation.
//!
//! # Features
//!
//! - Cross-platform USB device enumeration
//! - SSH tunnel management with multiplexing
//! - USB/IP protocol implementation
//! - Hot-plug event handling
//! - Configuration management
//!
//! # Example
//!
//! ```rust,no_run
//! use usboverssh_core::{Config, DeviceManager, SshTunnel};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::load_or_default()?;
//!     let mut manager = DeviceManager::new()?;
//!     
//!     for device in manager.list_devices()? {
//!         println!("{}", device);
//!     }
//!     
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod device;
pub mod error;
pub mod platform;
pub mod protocol;
pub mod server;
pub mod tunnel;

pub use config::Config;
pub use device::{glob_match, DeviceFilter, DeviceInfo, DeviceManager, DeviceSpeed};
pub use error::{Error, Result};
pub use protocol::UsbIpProtocol;
pub use server::Server;
pub use tunnel::{SshSession, SshTunnel, TunnelConfig};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name
pub const APP_NAME: &str = "usboverssh";
