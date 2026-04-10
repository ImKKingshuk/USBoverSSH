//! Input Validation Module
//!
//! Provides validation for user inputs to prevent injection attacks and ensure data integrity.

use crate::error::{Error, Result};
use regex::Regex;
use std::path::PathBuf;
use std::sync::OnceLock;

/// VID:PID format (e.g., 1234:5678)
fn vid_pid_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[0-9a-fA-F]{4}:[0-9a-fA-F]{4}$").unwrap())
}

/// Bus ID format (e.g., 1-2.3)
fn bus_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[0-9]+-[0-9]+\.[0-9]+$").unwrap())
}

/// Host specification (user@host[:port])
fn host_spec_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+(?::\d+)?$").unwrap())
}

/// Hostname (for validation)
fn hostname_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$").unwrap())
}

/// Port number
fn port_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[1-9][0-9]{0,4}$").unwrap())
}

/// Validate device pattern (VID:PID or bus ID)
pub fn validate_device_pattern(pattern: &str) -> Result<()> {
    if pattern.is_empty() {
        return Err(Error::Config("Device pattern cannot be empty".to_string()));
    }

    // Check if it's a VID:PID pattern
    if pattern.contains(':') {
        if !vid_pid_re().is_match(pattern) {
            return Err(Error::Config(format!(
                "Invalid VID:PID format '{}'. Expected format: XXXX:XXXX (hex digits)",
                pattern
            )));
        }
    } else {
        // Check if it's a bus ID pattern
        if !bus_id_re().is_match(pattern) {
            return Err(Error::Config(format!(
                "Invalid bus ID format '{}'. Expected format: X-Y.Z (e.g., 1-2.3)",
                pattern
            )));
        }
    }

    Ok(())
}

/// Validate host specification (user@host[:port])
pub fn validate_host_spec(spec: &str) -> Result<()> {
    if spec.is_empty() {
        return Err(Error::Config(
            "Host specification cannot be empty".to_string(),
        ));
    }

    if !host_spec_re().is_match(spec) {
        return Err(Error::Config(format!(
            "Invalid host specification '{}'. Expected format: user@host[:port]",
            spec
        )));
    }

    // Validate hostname part
    if let Some(at_pos) = spec.find('@') {
        let host_part = &spec[at_pos + 1..];
        let host_without_port = host_part.split(':').next().unwrap_or(host_part);

        if !hostname_re().is_match(host_without_port) {
            return Err(Error::Config(format!(
                "Invalid hostname '{}'. Hostname must contain only alphanumeric characters, dots, and hyphens",
                host_without_port
            )));
        }

        // Validate port if present
        if let Some(colon_pos) = host_part.find(':') {
            let port = &host_part[colon_pos + 1..];
            if !port_re().is_match(port) {
                let port_num: u16 = port.parse().map_err(|_| {
                    Error::Config(format!("Invalid port number '{}': must be 1-65535", port))
                })?;

                if port_num == 0 {
                    return Err(Error::Config("Port cannot be 0".to_string()));
                }
            }
        }
    }

    Ok(())
}

/// Validate file path and prevent path traversal
pub fn validate_file_path(path: &str) -> Result<PathBuf> {
    if path.is_empty() {
        return Err(Error::Config("File path cannot be empty".to_string()));
    }

    let path = PathBuf::from(path);

    // Check for path traversal attempts
    let path_str = path.to_string_lossy();
    if path_str.contains("..") {
        return Err(Error::Config(
            "Path traversal not allowed: path contains '..'".to_string(),
        ));
    }

    // Check for absolute paths (optional - depends on security requirements)
    // if path.is_absolute() {
    //     return Err(Error::Config("Absolute paths not allowed".to_string()));
    // }

    Ok(path)
}

/// Sanitize user input by removing potentially dangerous characters
pub fn sanitize_input(input: &str) -> String {
    input
        .chars()
        .filter(|c| {
            // Allow alphanumeric, common safe characters
            c.is_alphanumeric()
                || *c == '-'
                || *c == '_'
                || *c == '.'
                || *c == '@'
                || *c == ':'
                || *c == '/'
                || *c == ' '
        })
        .collect()
}

/// Validate and sanitize device pattern
pub fn validate_and_sanitize_device_pattern(pattern: &str) -> Result<String> {
    let sanitized = sanitize_input(pattern);
    validate_device_pattern(&sanitized)?;
    Ok(sanitized)
}

/// Validate and sanitize host specification
pub fn validate_and_sanitize_host_spec(spec: &str) -> Result<String> {
    let sanitized = sanitize_input(spec);
    validate_host_spec(&sanitized)?;
    Ok(sanitized)
}

/// Validate port number
pub fn validate_port(port: u16) -> Result<()> {
    if port == 0 {
        return Err(Error::Config("Port cannot be 0".to_string()));
    }

    // u16 max is 65535, so no need to check upper bound
    Ok(())
}

/// Validate username (for SSH user)
pub fn validate_username(username: &str) -> Result<()> {
    if username.is_empty() {
        return Err(Error::Config("Username cannot be empty".to_string()));
    }

    if username.len() > 32 {
        return Err(Error::Config(
            "Username too long (max 32 characters)".to_string(),
        ));
    }

    // Allow alphanumeric, underscore, hyphen, dot
    let valid = username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.');

    if !valid {
        return Err(Error::Config(format!(
            "Invalid username '{}': only alphanumeric, underscore, hyphen, and dot allowed",
            username
        )));
    }

    Ok(())
}
