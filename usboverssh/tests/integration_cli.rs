// Integration tests for CLI commands

use usboverssh::config::Config;

#[test]
fn test_config_load_or_default() {
    let config = Config::load_or_default();
    assert!(config.is_ok());
}

#[test]
fn test_config_validation() {
    let config = Config::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_default_path() {
    let path = Config::default_path();
    assert!(path.is_some());
    let binding = path.unwrap();
    let path_str = binding.to_string_lossy();
    assert!(path_str.contains("usboverssh"));
    assert!(path_str.contains("config.toml"));
}

#[test]
fn test_config_generate_example() {
    let example = usboverssh::config::generate_example_config();
    assert!(example.contains("# USBoverSSH Configuration File"));
    assert!(example.contains("[general]"));
    assert!(example.contains("[ssh]"));
}

#[test]
fn test_host_config_parse() {
    use usboverssh::config::HostConfig;

    let config = HostConfig::parse("user@host:2222");
    assert_eq!(config.port, 2222);
    assert_eq!(config.user, "user");
    assert_eq!(config.hostname, "host");
}

#[test]
fn test_device_filter_parse() {
    use usboverssh::device::DeviceFilter;

    let filter = DeviceFilter::parse("1234:5678");
    assert_eq!(filter.vendor_id, Some(0x1234));
    assert_eq!(filter.product_id, Some(0x5678));
}

#[test]
fn test_device_filter_parse_bus_id() {
    use usboverssh::device::DeviceFilter;

    let filter = DeviceFilter::parse("1-2.3");
    assert_eq!(filter.bus_id, Some("1-2.3".to_string()));
}
