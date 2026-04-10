//! Windows-specific USB device enumeration
//!
//! Uses nusb crate for cross-platform USB enumeration.
//! Note: USB/IP server functionality is Linux-only; Windows is client-only.

use crate::device::{DeviceClass, DeviceInfo, DeviceSpeed};
use crate::error::{Error, Result};

/// Enumerate USB devices on Windows using nusb
pub fn enumerate_devices() -> Result<Vec<DeviceInfo>> {
    let mut devices = Vec::new();
    
    for device in nusb::list_devices().map_err(|e| {
        Error::UsbEnumeration(format!("Failed to enumerate USB devices: {}", e))
    })? {
        let device_info = DeviceInfo {
            bus_id: format!("{:03}-{:03}", device.bus_number(), device.device_address()),
            vendor_id: device.vendor_id(),
            product_id: device.product_id(),
            device_class: DeviceClass::from_code(device.class()),
            bus_num: device.bus_number(),
            dev_num: device.device_address(),
            speed: parse_speed(&device),
            manufacturer: device.manufacturer_string().map(|s| s.to_string()),
            product: device.product_string().map(|s| s.to_string()),
            serial: device.serial_number().map(|s| s.to_string()),
            num_configurations: 1, // nusb doesn't expose this directly
            is_attached: false,
            is_bound: false,
        };
        
        devices.push(device_info);
    }
    
    // Sort by bus number, then device number
    devices.sort_by(|a, b| {
        a.bus_num.cmp(&b.bus_num).then(a.dev_num.cmp(&b.dev_num))
    });
    
    Ok(devices)
}

/// Parse device speed from nusb device info
fn parse_speed(device: &nusb::DeviceInfo) -> DeviceSpeed {
    match device.speed() {
        Some(nusb::Speed::Low) => DeviceSpeed::Low,
        Some(nusb::Speed::Full) => DeviceSpeed::Full,
        Some(nusb::Speed::High) => DeviceSpeed::High,
        Some(nusb::Speed::Super) => DeviceSpeed::Super,
        Some(nusb::Speed::SuperPlus) => DeviceSpeed::SuperPlus,
        _ => DeviceSpeed::Unknown,
    }
}

