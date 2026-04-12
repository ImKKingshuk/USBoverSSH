# Security Policy

## Sensitive Data Handling

USBoverSSH is a USB device sharing and tunneling tool that interacts with USB devices and remote systems via SSH. This document outlines security best practices for contributors and users.

### SSH Access and Remote System Interaction

USBoverSSH requires SSH access to function:

- **SSH Authentication**: Remote hosts must have SSH server enabled and authorize the connecting machine
- **Device Access**: The tool interacts with USB device enumeration and USB/IP protocol for device sharing
- **Root Access**: Some features may require root access for kernel module loading (Linux) and VHCI operations
- **USB/IP Server**: The server mode exports USB devices which can be accessed by authorized clients

**Always ensure**:

- You authorize only trusted machines for SSH access
- Use SSH key-based authentication instead of passwords
- Understand the implications of sharing USB devices remotely
- Firewall USB/IP ports from untrusted networks
- Disable USB/IP server when not in use
- Verify SSH host keys before connecting

### What's Excluded from Git

The following sensitive data types are automatically excluded via `.gitignore`:

1. **Credentials & Keys**
   - Private keys (*.pem,*.key, id_rsa, id_ed25519, etc.)
   - Certificates (*.crt,*.cer, *.p12,*.pfx)
   - SSH known_hosts files
   - Environment files (.env, .env.*)
   - API key files (secrets.json, credentials.json)

2. **Configuration Files**
   - User configuration files with host credentials
   - SSH config files with sensitive information
   - Configuration files with passwords or tokens

### Security Scanning

This repository uses:

- **cargo-deny**: Dependency vulnerability scanner
- **clippy**: Rust linter with security-focused checks
- **typos**: Spell checker to prevent typosquatting vulnerabilities

### Reporting Security Issues

If you discover a security vulnerability in USBoverSSH, please report it privately:

- Do not open a public GitHub issue
- Contact the maintainers directly through GitHub Security Advisories
- Provide detailed information about the vulnerability

### Data Privacy

USBoverSSH is designed to share USB devices between machines securely over SSH. Users must:

- Understand which devices they are sharing and their implications
- Ensure proper authorization before accessing remote USB devices
- Be aware that sharing certain devices may have security implications
- Follow manufacturer guidelines for device sharing
- Use SSH tunneling to encrypt USB traffic in transit

### Safe Usage Guidelines

- **Verify Hosts**: Always verify SSH host keys before connecting to remote systems
- **Use SSH Keys**: Prefer SSH key-based authentication over passwords
- **Firewall Ports**: Restrict USB/IP server ports to trusted networks or bind to localhost with SSH tunneling
- **Limit Device Sharing**: Only share devices that need to be shared
- **Monitor Connections**: Monitor active SSH and USB/IP connections
- **Update Regularly**: Keep USBoverSSH and dependencies updated for security patches
- **Review Configuration**: Regularly review configuration files for unauthorized changes

### SSH Security Best Practices

- **Key Management**: Use strong SSH keys and protect private keys
- **Known Hosts**: Enable strict host key checking and verify fingerprints
- **Agent Forwarding**: Use SSH agent forwarding cautiously and only when necessary
- **Config File**: Secure SSH config file with appropriate permissions
- **Disable Unused**: Disable SSH services on systems that don't need remote access

### USB/IP Security Considerations

- **Network Isolation**: USB/IP traffic should be isolated or tunneled through SSH
- **Device Filtering**: Use device filters to limit which devices are exported
- **Access Control**: Implement proper access control on USB/IP server ports
- **Audit Logs**: Monitor USB/IP server logs for unauthorized access attempts
- **Linux Server Only**: USB/IP server functionality is Linux-only; macOS and Windows are client-only

## License

USBoverSSH is licensed under GPL-3.0. See LICENSE file for details.
