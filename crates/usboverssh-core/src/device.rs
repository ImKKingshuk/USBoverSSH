//! USB Device Types and Management
//!
//! Provides cross-platform USB device enumeration and filtering.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// USB device speed classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceSpeed {
    /// USB 1.0 Low Speed (1.5 Mbps)
    Low,
    /// USB 1.1 Full Speed (12 Mbps)
    Full,
    /// USB 2.0 High Speed (480 Mbps)
    High,
    /// USB 3.0 SuperSpeed (5 Gbps)
    Super,
    /// USB 3.1 SuperSpeed+ (10 Gbps)
    SuperPlus,
    /// USB 3.2 SuperSpeed+ (20 Gbps)
    SuperPlus2,
    /// Unknown speed
    Unknown,
}

impl DeviceSpeed {
    /// USB/IP speed constant for VHCI attachment
    pub fn to_usbip_speed(&self) -> u32 {
        match self {
            Self::Low => 1,
            Self::Full => 2,
            Self::High => 3,
            Self::Super => 5,
            Self::SuperPlus | Self::SuperPlus2 => 6,
            Self::Unknown => 3, // Default to High
        }
    }

    /// Parse from speed string (e.g., "480", "5000")
    pub fn from_speed_mbps(mbps: u32) -> Self {
        match mbps {
            0..=2 => Self::Low,
            3..=15 => Self::Full,
            16..=500 => Self::High,
            501..=6000 => Self::Super,
            6001..=12000 => Self::SuperPlus,
            _ => Self::SuperPlus2,
        }
    }

    /// Human-readable speed string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "1.5 Mbps (USB 1.0)",
            Self::Full => "12 Mbps (USB 1.1)",
            Self::High => "480 Mbps (USB 2.0)",
            Self::Super => "5 Gbps (USB 3.0)",
            Self::SuperPlus => "10 Gbps (USB 3.1)",
            Self::SuperPlus2 => "20 Gbps (USB 3.2)",
            Self::Unknown => "Unknown",
        }
    }
}

impl fmt::Display for DeviceSpeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// USB device class codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceClass {
    Audio,
    Comm,
    Hid,
    Physical,
    Image,
    Printer,
    MassStorage,
    Hub,
    CdcData,
    SmartCard,
    ContentSecurity,
    Video,
    PersonalHealthcare,
    AudioVideo,
    Billboard,
    UsbTypeCBridge,
    Diagnostic,
    WirelessController,
    Miscellaneous,
    ApplicationSpecific,
    VendorSpecific,
    Unknown(u8),
}

impl DeviceClass {
    /// Parse from USB class code
    pub fn from_code(code: u8) -> Self {
        match code {
            0x01 => Self::Audio,
            0x02 => Self::Comm,
            0x03 => Self::Hid,
            0x05 => Self::Physical,
            0x06 => Self::Image,
            0x07 => Self::Printer,
            0x08 => Self::MassStorage,
            0x09 => Self::Hub,
            0x0a => Self::CdcData,
            0x0b => Self::SmartCard,
            0x0d => Self::ContentSecurity,
            0x0e => Self::Video,
            0x0f => Self::PersonalHealthcare,
            0x10 => Self::AudioVideo,
            0x11 => Self::Billboard,
            0x12 => Self::UsbTypeCBridge,
            0xdc => Self::Diagnostic,
            0xe0 => Self::WirelessController,
            0xef => Self::Miscellaneous,
            0xfe => Self::ApplicationSpecific,
            0xff => Self::VendorSpecific,
            code => Self::Unknown(code),
        }
    }

    /// Short name for display
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Audio => "Audio",
            Self::Comm => "COM",
            Self::Hid => "HID",
            Self::Physical => "Physical",
            Self::Image => "Image",
            Self::Printer => "Printer",
            Self::MassStorage => "Storage",
            Self::Hub => "Hub",
            Self::CdcData => "CDC",
            Self::SmartCard => "SmartCard",
            Self::ContentSecurity => "Security",
            Self::Video => "Video",
            Self::PersonalHealthcare => "Health",
            Self::AudioVideo => "AV",
            Self::Billboard => "Billboard",
            Self::UsbTypeCBridge => "USB-C",
            Self::Diagnostic => "Diag",
            Self::WirelessController => "Wireless",
            Self::Miscellaneous => "Misc",
            Self::ApplicationSpecific => "App",
            Self::VendorSpecific => "Vendor",
            Self::Unknown(_) => "Unknown",
        }
    }
}

impl fmt::Display for DeviceClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.short_name())
    }
}

/// Complete USB device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Bus ID (e.g., "3-1.2")
    pub bus_id: String,
    /// Vendor ID (e.g., 0x1234)
    pub vendor_id: u16,
    /// Product ID (e.g., 0x5678)
    pub product_id: u16,
    /// Device class
    pub device_class: DeviceClass,
    /// Bus number
    pub bus_num: u8,
    /// Device number
    pub dev_num: u8,
    /// Device speed
    pub speed: DeviceSpeed,
    /// Manufacturer name (if available)
    pub manufacturer: Option<String>,
    /// Product name (if available)
    pub product: Option<String>,
    /// Serial number (if available)
    pub serial: Option<String>,
    /// Number of configurations
    pub num_configurations: u8,
    /// Is device currently attached via USB/IP?
    pub is_attached: bool,
    /// Is device bound to usbip-host driver?
    pub is_bound: bool,
}

impl DeviceInfo {
    /// Get vendor:product ID string (e.g., "03f0:e111")
    pub fn vid_pid(&self) -> String {
        format!("{:04x}:{:04x}", self.vendor_id, self.product_id)
    }

    /// Get display name (product name or vid:pid)
    pub fn display_name(&self) -> String {
        self.product
            .clone()
            .unwrap_or_else(|| self.vid_pid())
    }

    /// Get full description
    pub fn description(&self) -> String {
        let mut parts = vec![self.bus_id.clone()];
        parts.push(self.vid_pid());
        
        if let Some(ref product) = self.product {
            parts.push(product.clone());
        }
        if let Some(ref manufacturer) = self.manufacturer {
            parts.push(format!("({})", manufacturer));
        }
        if let Some(ref serial) = self.serial {
            parts.push(format!("[{}]", serial));
        }
        
        parts.join(" ")
    }

    /// Check if device matches a filter
    pub fn matches(&self, filter: &DeviceFilter) -> bool {
        filter.matches(self)
    }
}

impl fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:<10} {:04x}:{:04x} {:>8} {}",
            self.bus_id,
            self.vendor_id,
            self.product_id,
            self.device_class.short_name(),
            self.display_name()
        )
    }
}

/// Device filter for selecting devices
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceFilter {
    /// Bus ID pattern (e.g., "3-1*")
    pub bus_id: Option<String>,
    /// Vendor ID
    pub vendor_id: Option<u16>,
    /// Product ID
    pub product_id: Option<u16>,
    /// Serial number pattern
    pub serial: Option<String>,
    /// Product name pattern
    pub product: Option<String>,
    /// Device class
    pub device_class: Option<DeviceClass>,
}

impl DeviceFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse filter from string pattern
    ///
    /// Supports formats:
    /// - Bus ID: "3-1.2"
    /// - VID:PID: "1234:5678"
    /// - Serial/Product name: any other string
    pub fn parse(pattern: &str) -> Self {
        let mut filter = Self::new();
        
        // Check for bus ID pattern (e.g., "3-1.2")
        if regex::Regex::new(r"^\d+-\d+(\.\d+)*$")
            .unwrap()
            .is_match(pattern)
        {
            filter.bus_id = Some(pattern.to_string());
            return filter;
        }
        
        // Check for VID:PID pattern (e.g., "03f0:e111")
        if let Some(caps) = regex::Regex::new(r"^([0-9a-fA-F]{1,4}):([0-9a-fA-F]{1,4})$")
            .unwrap()
            .captures(pattern)
        {
            filter.vendor_id = Some(u16::from_str_radix(&caps[1], 16).unwrap_or(0));
            filter.product_id = Some(u16::from_str_radix(&caps[2], 16).unwrap_or(0));
            return filter;
        }
        
        // Otherwise, treat as product/serial pattern
        filter.product = Some(pattern.to_string());
        filter.serial = Some(pattern.to_string());
        filter
    }

    /// Check if a device matches this filter
    pub fn matches(&self, device: &DeviceInfo) -> bool {
        // Check bus ID
        if let Some(ref pattern) = self.bus_id {
            if !glob_match(pattern, &device.bus_id) {
                return false;
            }
        }

        // Check vendor ID
        if let Some(vid) = self.vendor_id {
            if device.vendor_id != vid {
                return false;
            }
        }

        // Check product ID
        if let Some(pid) = self.product_id {
            if device.product_id != pid {
                return false;
            }
        }

        // Check serial (OR with product)
        if self.serial.is_some() || self.product.is_some() {
            let serial_match = self.serial.as_ref().map_or(false, |s| {
                device.serial.as_ref().map_or(false, |ds| glob_match(s, ds))
            });
            let product_match = self.product.as_ref().map_or(false, |p| {
                device.product.as_ref().map_or(false, |dp| glob_match(p, dp))
            });
            
            // If both are specified, either can match
            if self.serial.is_some() && self.product.is_some() {
                if !serial_match && !product_match {
                    return false;
                }
            } else if self.serial.is_some() && !serial_match {
                return false;
            } else if self.product.is_some() && !product_match {
                return false;
            }
        }

        // Check device class
        if let Some(ref class) = self.device_class {
            if device.device_class != *class {
                return false;
            }
        }

        true
    }
}

/// Simple glob matching (supports * wildcard)
fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern_lower = pattern.to_lowercase();
    let text_lower = text.to_lowercase();
    
    if !pattern_lower.contains('*') {
        return text_lower.contains(&pattern_lower);
    }
    
    let parts: Vec<&str> = pattern_lower.split('*').collect();
    let mut pos = 0;
    
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        
        if let Some(found) = text_lower[pos..].find(part) {
            if i == 0 && found != 0 {
                return false; // Must match from start if no leading *
            }
            pos += found + part.len();
        } else {
            return false;
        }
    }
    
    // If pattern doesn't end with *, must match to end
    if !pattern_lower.ends_with('*') && pos != text_lower.len() {
        return false;
    }
    
    true
}

/// USB Device Manager for enumeration and selection
pub struct DeviceManager {
    /// Cached device list
    devices: Vec<DeviceInfo>,
}

impl DeviceManager {
    /// Create a new device manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            devices: Vec::new(),
        })
    }

    /// Refresh and return all USB devices
    pub fn list_devices(&mut self) -> Result<&[DeviceInfo]> {
        self.refresh()?;
        Ok(&self.devices)
    }

    /// Refresh device list from system
    pub fn refresh(&mut self) -> Result<()> {
        self.devices = crate::platform::enumerate_devices()?;
        Ok(())
    }

    /// Find a single device matching the filter
    pub fn find_device(&mut self, filter: &DeviceFilter) -> Result<&DeviceInfo> {
        self.refresh()?;
        
        let matches: Vec<_> = self.devices.iter().filter(|d| d.matches(filter)).collect();
        
        match matches.len() {
            0 => Err(Error::DeviceNotFound(format!("{:?}", filter))),
            1 => Ok(matches[0]),
            _ => Err(Error::MultipleDevicesMatch {
                pattern: format!("{:?}", filter),
                matches: matches.iter().map(|d| d.bus_id.clone()).collect(),
            }),
        }
    }

    /// Find device by pattern string
    pub fn find_by_pattern(&mut self, pattern: &str) -> Result<&DeviceInfo> {
        let filter = DeviceFilter::parse(pattern);
        self.find_device(&filter)
    }

    /// Get all devices matching a filter
    pub fn filter_devices(&mut self, filter: &DeviceFilter) -> Result<Vec<&DeviceInfo>> {
        self.refresh()?;
        Ok(self.devices.iter().filter(|d| d.matches(filter)).collect())
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new().expect("Failed to create DeviceManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_parse_bus_id() {
        let filter = DeviceFilter::parse("3-1.2");
        assert_eq!(filter.bus_id, Some("3-1.2".to_string()));
    }

    #[test]
    fn test_filter_parse_vid_pid() {
        let filter = DeviceFilter::parse("1234:5678");
        assert_eq!(filter.vendor_id, Some(0x1234));
        assert_eq!(filter.product_id, Some(0x5678));
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("foo", "foobar"));
        assert!(glob_match("*bar", "foobar"));
        assert!(glob_match("foo*", "foobar"));
        assert!(glob_match("*ob*", "foobar"));
        assert!(!glob_match("baz", "foobar"));
    }

    #[test]
    fn test_device_speed() {
        assert_eq!(DeviceSpeed::from_speed_mbps(480).to_usbip_speed(), 3);
        assert_eq!(DeviceSpeed::from_speed_mbps(5000).to_usbip_speed(), 5);
    }
}
