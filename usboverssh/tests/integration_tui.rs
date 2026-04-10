// Integration tests for TUI workflows

use usboverssh::config::{Config, HostConfig};
use usboverssh::device::{DeviceFilter, DeviceInfo, DeviceClass, DeviceSpeed};

#[test]
fn test_config_with_hosts() {
    let mut config = Config::default();
    
    config.hosts.insert(
        "test_server".to_string(),
        HostConfig {
            hostname: "example.com".to_string(),
            port: 2222,
            user: "testuser".to_string(),
            identity_file: None,
            device_filters: vec![],
            description: Some("Test server".to_string()),
        },
    );

    let host = config.get_host("test_server");
    assert_eq!(host.hostname, "example.com");
    assert_eq!(host.port, 2222);
}

#[test]
fn test_config_get_host_parse() {
    let config = Config::default();
    let host = config.get_host("user@host:2222");
    assert_eq!(host.port, 2222);
}

#[test]
fn test_device_filter_matches() {
    let device = DeviceInfo {
        bus_id: "1-1.2".to_string(),
        vendor_id: 0x1234,
        product_id: 0x5678,
        device_class: DeviceClass::Hid,
        bus_num: 1,
        dev_num: 3,
        speed: DeviceSpeed::High,
        manufacturer: Some("Test".to_string()),
        product: Some("Device".to_string()),
        serial: None,
        num_configurations: 1,
        is_attached: false,
        is_bound: false,
    };

    let filter = DeviceFilter {
        vendor_id: Some(0x1234),
        product_id: None,
        bus_id: None,
        product: None,
        serial: None,
        device_class: None,
    };

    assert!(device.matches(&filter));
}

#[test]
fn test_device_info_display_name() {
    let device = DeviceInfo {
        bus_id: "1-1.2".to_string(),
        vendor_id: 0x1234,
        product_id: 0x5678,
        device_class: DeviceClass::Hid,
        bus_num: 1,
        dev_num: 3,
        speed: DeviceSpeed::High,
        manufacturer: Some("Test".to_string()),
        product: Some("Device".to_string()),
        serial: None,
        num_configurations: 1,
        is_attached: false,
        is_bound: false,
    };

    let name = device.display_name();
    assert!(name.contains("Device"));
}

#[test]
fn test_device_info_vid_pid() {
    let device = DeviceInfo {
        bus_id: "1-1.2".to_string(),
        vendor_id: 0x1234,
        product_id: 0x5678,
        device_class: DeviceClass::Hid,
        bus_num: 1,
        dev_num: 3,
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

#[test]
fn test_config_validation_with_invalid_host() {
    let mut config = Config::default();
    
    config.hosts.insert(
        "".to_string(),
        HostConfig {
            hostname: "".to_string(),
            port: 0,
            user: "".to_string(),
            identity_file: None,
            device_filters: vec![],
            description: None,
        },
    );

    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_config_validation_with_valid_config() {
    let mut config = Config::default();
    
    config.hosts.insert(
        "test_server".to_string(),
        HostConfig {
            hostname: "example.com".to_string(),
            port: 2222,
            user: "testuser".to_string(),
            identity_file: None,
            device_filters: vec![],
            description: Some("Test server".to_string()),
        },
    );

    let result = config.validate();
    assert!(result.is_ok());
}
