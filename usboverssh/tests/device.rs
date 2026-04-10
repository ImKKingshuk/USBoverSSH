// Unit tests for device management

use usboverssh::device::{glob_match, DeviceClass, DeviceFilter, DeviceInfo, DeviceSpeed};

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

#[test]
fn test_device_speed_to_usbip_speed() {
    assert_eq!(DeviceSpeed::Low.to_usbip_speed(), 1);
    assert_eq!(DeviceSpeed::Full.to_usbip_speed(), 2);
    assert_eq!(DeviceSpeed::High.to_usbip_speed(), 3);
    assert_eq!(DeviceSpeed::Super.to_usbip_speed(), 5);
    assert_eq!(DeviceSpeed::SuperPlus.to_usbip_speed(), 6);
}

#[test]
fn test_device_speed_from_mbps() {
    assert_eq!(DeviceSpeed::from_speed_mbps(1), DeviceSpeed::Low);
    assert_eq!(DeviceSpeed::from_speed_mbps(12), DeviceSpeed::Full);
    assert_eq!(DeviceSpeed::from_speed_mbps(480), DeviceSpeed::High);
    assert_eq!(DeviceSpeed::from_speed_mbps(5000), DeviceSpeed::Super);
}

#[test]
fn test_device_class_from_code() {
    assert_eq!(DeviceClass::from_code(0x01), DeviceClass::Audio);
    assert_eq!(DeviceClass::from_code(0x08), DeviceClass::MassStorage);
    assert_eq!(DeviceClass::from_code(0x09), DeviceClass::Hub);
    assert_eq!(DeviceClass::from_code(0xff), DeviceClass::VendorSpecific);
}

#[test]
fn test_device_class_short_name() {
    assert_eq!(DeviceClass::Audio.short_name(), "Audio");
    assert_eq!(DeviceClass::MassStorage.short_name(), "Storage");
    assert_eq!(DeviceClass::Hub.short_name(), "Hub");
}

#[test]
fn test_device_filter_parse_glob() {
    let filter = DeviceFilter::parse("*");
    assert!(filter.product.is_some() || filter.serial.is_some());
}

#[test]
fn test_device_info_vid_pid() {
    let device = DeviceInfo {
        bus_id: "1-2.3".to_string(),
        vendor_id: 0x1234,
        product_id: 0x5678,
        device_class: DeviceClass::Unknown(0),
        bus_num: 1,
        dev_num: 2,
        speed: DeviceSpeed::High,
        manufacturer: Some("Test".to_string()),
        product: Some("Device".to_string()),
        serial: None,
        num_configurations: 1,
        is_attached: false,
        is_bound: false,
    };

    assert_eq!(device.vid_pid(), "1234:5678");
}
