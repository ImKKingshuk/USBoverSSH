//! Platform-specific implementations
//!
//! Provides cross-platform USB device enumeration and kernel interface.

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows_impl;

use crate::device::DeviceInfo;
use crate::error::Result;

/// Enumerate all USB devices on the system
pub fn enumerate_devices() -> Result<Vec<DeviceInfo>> {
    #[cfg(target_os = "linux")]
    {
        linux::enumerate_devices()
    }
    
    #[cfg(target_os = "macos")]
    {
        macos::enumerate_devices()
    }
    
    #[cfg(target_os = "windows")]
    {
        windows_impl::enumerate_devices()
    }
    
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(crate::error::Error::PlatformNotSupported(
            std::env::consts::OS.to_string(),
        ))
    }
}

/// Check if USB/IP kernel modules are available
pub fn check_usbip_available() -> Result<bool> {
    #[cfg(target_os = "linux")]
    {
        linux::check_usbip_available()
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        // USB/IP client functionality works via userspace on non-Linux
        Ok(true)
    }
}

/// Load required kernel modules (Linux only)
pub fn load_kernel_modules(server_mode: bool) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        linux::load_kernel_modules(server_mode)
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        let _ = server_mode;
        Ok(())
    }
}

/// Get the platform name
pub fn platform_name() -> &'static str {
    #[cfg(target_os = "linux")]
    { "Linux" }
    
    #[cfg(target_os = "macos")]
    { "macOS" }
    
    #[cfg(target_os = "windows")]
    { "Windows" }
    
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    { "Unknown" }
}
