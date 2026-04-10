<div align="center">

# USBoverSSH

### The Ultimate USB over SSH Solution

### Unified USB Device Sharing Platform

### ⚛ Rust-Powered ⚛ TUI-First ⚛

#### Secure USB Tunneling, Device Management, and Cross-Platform Support in One Framework

USBoverSSH is a unified USB device sharing and tunneling toolkit built entirely in Rust. It provides a comprehensive TUI workspace alongside a powerful CLI, enabling users to perform USB device enumeration, secure SSH tunneling, USB/IP protocol implementation, and device attachment from a single modular framework.

The platform integrates advanced capabilities including cross-platform USB device enumeration (Linux sysfs, macOS/Windows via nusb), SSH tunneling with russh, USB/IP server implementation (Linux), device filtering by multiple criteria, persistent connections with auto-reconnect, daemon mode for background operation, and kernel module management. USBoverSSH supports modern USB ecosystems across Linux (full server + client), macOS (client only), and Windows (client only).

With comprehensive device filtering (bus ID, VID:PID, serial, product name, device class), multi-host configuration management, VHCI attachment/detachment (Linux), and an interactive TUI with tabbed interface, USBoverSSH enables users to securely share and access USB devices across machines within one unified environment.

Connect your devices and begin advanced USB over SSH tunneling and management.

<br>

[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux%20%7C%20Windows-brightgreen)]()
[![Version](https://img.shields.io/badge/Release-v1.0.0-red)]()
[![License](https://img.shields.io/badge/License-GPLv3-blue)]()

<br>

</div>

## Installation

## Quick Start

### TUI (Default)

```bash
usboverssh
```

### CLI (Headless)

```bash
# List local USB devices
usboverssh list

# List remote USB devices
usboverssh list [email protected]

# Attach a remote device
usboverssh attach [email protected] 0xXXXX:0xXXXX

# Show attached devices
usboverssh status
```

## Product Priority

- **TUI is the main product and default experience.** Use `usboverssh` for day-to-day device management, host connections, and operator-guided operations.
- **CLI is the secondary surface.** Use `usboverssh list`, `attach`, `detach`, and other commands for quick one-off tasks, scripting, and automation.

### TUI Keybindings

| Action | Keys |
|--------|------|
| Quit | q or Esc |
| Navigate panes | Tab / Shift+Tab |
| Navigate items | Arrow keys or j/k |
| Select/Activate | Enter |
| Attach device | a |
| Detach device | d |
| Refresh devices | r or F5 |
| Connect to host | c |
| Show hosts panel | h |
| Toggle status panel | s |
| Help | ? or F1 |

## Current Capabilities (v1.0.0)

### Core Platform

- ✅ Rust CLI with subcommands: list, attach, detach, status, serve, tui, config, completions
- 🔧 Full-screen TUI by default (`usboverssh`) as the primary product surface
- ✅ Async SSH communication with russh library
- ✅ Configuration management via TOML
- ✅ Structured logging with tracing
- ✅ Error handling with comprehensive error types

### Rust Core

- ✅ Cross-platform USB device enumeration (Linux: sysfs, macOS/Windows: nusb)
- ✅ SSH tunneling with key-based authentication
- ✅ USB/IP protocol implementation (server-side for Linux)
- ✅ Device filtering (bus ID, VID:PID, serial, product name, device class)
- ✅ Device manager with find and filter operations
- ✅ Configuration with named hosts and auto-attach rules
- ✅ Persistent connections with auto-reconnect
- ✅ Daemon mode for background operation
- ✅ Kernel module management (Linux: usbip-host, vhci-hcd)
- ✅ VHCI attachment/detachment (Linux only)
- ✅ Shell completion generation

---

## Feature Matrix

### Device Management

- ✅ List local USB devices (all, with class filtering)
- ✅ List remote USB devices via SSH
- ✅ Device information retrieval (bus ID, VID:PID, class, speed, manufacturer, product, serial)
- ✅ Attach remote USB devices (by VID:PID, product name, bus ID)
- ✅ Detach attached devices (specific or all)
- ✅ Show currently attached devices (VHCI status)
- ✅ Device filtering by multiple criteria
- ✅ Persistent mode with auto-reconnect
- ✅ Daemon mode for background operation

### USB/IP Server

- ✅ Start USB/IP server (Linux only)
- ✅ TCP and Unix socket listeners
- ✅ Device export with filtering
- ✅ Export all devices option
- ✅ Device list request handling
- ✅ Import (attach) request handling
- ✅ Kernel module auto-loading (Linux)

### SSH Tunneling

- ✅ SSH connection establishment with russh
- ✅ Key-based authentication (ed25519, rsa, ecdsa)
- ✅ Remote command execution
- ✅ Unix socket forwarding
- ✅ Connection state management
- ✅ Keep-alive support
- ✅ Server host key verification

### Configuration

- ✅ TOML-based configuration
- ✅ Named host configurations
- ✅ SSH settings (identity file, config file, agent forwarding)
- ✅ General settings (reconnect delay, timeout, verbosity)
- ✅ Logging settings (level, format, file, color)
- ✅ TUI settings (refresh interval, mouse, theme, show serial/speed)
- ✅ Auto-attach rules with device filters
- ✅ Configuration init, show, path, add-host commands

### Platform Support

- ✅ Linux: Full support (server + client) with kernel modules
- ✅ macOS: Client only (device enumeration via nusb)
- ✅ Windows: Client only (device enumeration via nusb)
- ✅ Kernel module checking and loading (Linux)
- ✅ VHCI port management (Linux)
- ✅ Device binding/unbinding (Linux)

### TUI Features

- ✅ Tabbed interface (Local Devices, Remote Devices, Attached, Hosts)
- ✅ Device lists with status indicators
- ✅ Host connection management
- ✅ Attach/detach operations
- ✅ Help popup
- ✅ Connect dialog
- ✅ Status bar with keybinding hints
- ✅ Auto-refresh
- ✅ Mouse support

---

## Requirements

- **OS**: macOS, Linux, Windows
- **Rust**: 1.75+ (for building from source)
- **Linux Server**: Kernel with USB/IP support (usbip-core, usbip-host modules)
- **Linux Client**: Kernel with VHCI support (vhci-hcd module)
- **macOS/Windows**: No special requirements (client only)
- **SSH**: SSH key for authentication (recommended)

## Configuration

USBoverSSH stores configuration in the platform-appropriate data directory:

- **Linux**: `~/.config/usboverssh/config.toml`
- **macOS**: `~/Library/Application Support/usboverssh/config.toml`
- **Windows**: `%APPDATA%\usboverssh\config.toml`

Configuration includes:

- Named host configurations (hostname, port, user, identity file, device filters)
- SSH settings (default port, identity file, config file, agent forwarding, keepalive)
- General settings (reconnect delay, max reconnect attempts, connection timeout)
- Logging settings (level, format, file, color)
- TUI settings (refresh interval, mouse, theme, show serial/speed)
- Auto-attach rules (device filters, target host, enabled status)

## Platform Support

### Linux (Full Support)

- USB device enumeration via sysfs
- USB/IP server functionality
- VHCI client attachment
- Kernel module management (usbip-host, vhci-hcd)
- Device binding/unbinding
- All features supported

### macOS (Client Only)

- USB device enumeration via nusb
- SSH tunneling and remote device listing
- USB/IP client functionality (userspace)
- No server functionality

### Windows (Client Only)

- USB device enumeration via nusb
- SSH tunneling and remote device listing
- USB/IP client functionality (userspace)
- No server functionality

## Device Classes Supported

USB device classes recognized and filtered:

- Audio, Communication (COM), HID, Physical, Image, Printer, Mass Storage, Hub
- CDC Data, SmartCard, Content Security, Video, Personal Healthcare, Audio/Video
- Billboard, USB Type-C Bridge, Diagnostic, Wireless Controller
- Miscellaneous, Application Specific, Vendor Specific

## Disclaimer

**USBoverSSH: The Ultimate USB over SSH Solution** is developed for USB device sharing and tunneling purposes. It should be used responsibly and in compliance with all applicable laws and regulations. The developer of this tool is not responsible for any misuse or illegal activities conducted with this tool.

USB device sharing should only be performed with proper authorization and understanding of the implications. Accessing remote USB devices may affect system functionality and security. Always ensure proper authorization before using USBoverSSH for device sharing. Always adhere to ethical practices and comply with all applicable laws and regulations.

## License

This project is licensed under the GPL-3.0-only License.

<h3 align="center">Happy USB Tunneling with USBoverSSH! 🚀</h3>
