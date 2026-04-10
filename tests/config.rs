// Unit tests for configuration

use usboverssh::config::HostConfig;

#[test]
fn test_host_parse() {
    let config = HostConfig::parse("[email protected]:2222");
    assert_eq!(config.user, "testuser");
    assert_eq!(config.hostname, "server.local");
    assert_eq!(config.port, 2222);
}

#[test]
fn test_host_parse_no_port() {
    let config = HostConfig::parse("[email protected]");
    assert_eq!(config.user, "testuser");
    assert_eq!(config.hostname, "example.com");
    assert_eq!(config.port, 22);
}

#[test]
fn test_host_parse_no_user() {
    let config = HostConfig::parse("myserver");
    assert_eq!(config.hostname, "myserver");
    assert_eq!(config.port, 22);
}
