// Unit tests for Linux platform support

#[cfg(target_os = "linux")]
use usboverssh_core::platform;

#[test]
#[cfg(target_os = "linux")]
fn test_valid_bus_id() {
    assert!(platform::is_valid_bus_id("1-1"));
    assert!(platform::is_valid_bus_id("3-1.2"));
    assert!(platform::is_valid_bus_id("1-1.2.3.4"));
    assert!(!platform::is_valid_bus_id("usb1"));
    assert!(!platform::is_valid_bus_id("1-"));
    assert!(!platform::is_valid_bus_id("-1"));
}
