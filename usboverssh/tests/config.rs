// Unit tests for configuration

use usboverssh::config::HostConfig;
use whoami::username;

#[test]
fn test_host_parse() {
    let config = HostConfig::parse("[email protected]:2222");
    assert_eq!(config.user, username()); // parse doesn't set user from input
    assert_eq!(config.hostname, "[email protected]"); // entire string before colon
    assert_eq!(config.port, 2222);
}

#[test]
fn test_host_parse_no_port() {
    let config = HostConfig::parse("server.local");
    assert_eq!(config.user, username()); // parse doesn't set user from input
    assert_eq!(config.hostname, "server.local");
    assert_eq!(config.port, 22);
}

#[test]
fn test_host_parse_no_user() {
    let config = HostConfig::parse("myserver");
    assert_eq!(config.hostname, "myserver");
    assert_eq!(config.port, 22);
    assert_eq!(config.user, username());
}
