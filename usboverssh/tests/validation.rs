// Unit tests for input validation

use usboverssh::validation::{
    sanitize_input, validate_device_pattern, validate_file_path, validate_host_spec, validate_port,
    validate_username,
};

#[test]
fn test_validate_device_pattern_vid_pid() {
    assert!(validate_device_pattern("1234:5678").is_ok());
    assert!(validate_device_pattern("abcd:ef01").is_ok());
}

#[test]
fn test_validate_device_pattern_invalid() {
    assert!(validate_device_pattern("").is_err());
    assert!(validate_device_pattern("invalid").is_err());
}

#[test]
fn test_validate_device_pattern_bus_id() {
    assert!(validate_device_pattern("1-2.3").is_ok());
    assert!(validate_device_pattern("1-2").is_err());
}

#[test]
fn test_validate_host_spec() {
    assert!(validate_host_spec("user@host:22").is_ok());
    assert!(validate_host_spec("user@host").is_ok());
}

#[test]
fn test_validate_host_spec_invalid() {
    assert!(validate_host_spec("").is_err());
    assert!(validate_host_spec("host").is_err());
}

#[test]
fn test_validate_file_path() {
    assert!(validate_file_path("/tmp/test").is_ok());
    assert!(validate_file_path("~/test").is_ok());
}

#[test]
fn test_validate_file_path_invalid() {
    assert!(validate_file_path("").is_err());
    // Note: null bytes are not checked by current implementation
}

#[test]
fn test_sanitize_input() {
    assert_eq!(sanitize_input("test@example.com"), "test@example.com");
    assert_eq!(
        sanitize_input("test@example.com; rm -rf /"),
        "test@example.com rm -rf /"
    ); // semicolon removed, slash preserved
    assert_eq!(sanitize_input("test\x00null"), "testnull"); // null byte removed
}

#[test]
fn test_validate_port() {
    assert!(validate_port(22).is_ok());
    assert!(validate_port(65535).is_ok());
    assert!(validate_port(0).is_err());
}

#[test]
fn test_validate_username() {
    assert!(validate_username("user").is_ok());
    assert!(validate_username("user_name").is_ok());
    assert!(validate_username("user.name").is_ok());
    assert!(validate_username("").is_err());
    assert!(validate_username("a".repeat(33).as_str()).is_err());
    assert!(validate_username("user@name").is_err());
}
