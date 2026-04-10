// Unit tests for device management

use usboverssh::device::{DeviceFilter, DeviceSpeed, glob_match};

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
