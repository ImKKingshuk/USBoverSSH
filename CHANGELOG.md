# Changelog

All notable changes to this project will be documented in this file.

## [v1.0.0] - 2026-04-10

### 🚀 Initial Release

- **Core Platform**:
  - Rust CLI with comprehensive subcommands (list, attach, detach, status, serve, tui, config, completions)
  - Full-screen TUI by default as the primary product surface
  - Async SSH communication with russh library
  - Configuration management via TOML
  - Structured logging with tracing
  - Comprehensive error handling with unique error types

- **Device Management**:
  - Cross-platform USB device enumeration (Linux: sysfs, macOS/Windows: nusb)
  - List local USB devices with class filtering
  - List remote USB devices via SSH
  - Device information retrieval (bus ID, VID:PID, class, speed, manufacturer, product, serial)
  - Attach remote USB devices (by VID:PID, product name, bus ID)
  - Detach attached devices (specific or all)
  - Show currently attached devices (VHCI status on Linux)
  - Device filtering by multiple criteria (bus ID, VID:PID, serial, product name, device class)
  - Persistent mode with auto-reconnect
  - Daemon mode for background operation

- **SSH Tunneling**:
  - SSH connection establishment with russh
  - Key-based authentication (ed25519, rsa, ecdsa)
  - Remote command execution
  - Unix socket forwarding for USB/IP
  - Connection state management
  - Keep-alive support
  - Server host key verification

- **USB/IP Server**:
  - USB/IP protocol implementation (Linux only)
  - TCP and Unix socket listeners
  - Device export with filtering
  - Export all devices option
  - Device list request handling
  - Import (attach) request handling
  - Kernel module auto-loading (Linux)

- **Configuration**:
  - TOML-based configuration system
  - Named host configurations
  - SSH settings (identity file, config file, agent forwarding, keepalive)
  - General settings (reconnect delay, max reconnect attempts, connection timeout, verbosity)
  - Logging settings (level, format, file, color)
  - TUI settings (refresh interval, mouse, theme, show serial/speed)
  - Auto-attach rules with device filters
  - Configuration commands (init, show, path, add-host)

- **Platform Support**:
  - Linux: Full support (server + client) with kernel modules (usbip-host, vhci-hcd)
  - macOS: Client only (device enumeration via nusb, SSH tunneling, USB/IP client)
  - Windows: Client only (device enumeration via nusb, SSH tunneling, USB/IP client)
  - Kernel module checking and loading (Linux)
  - VHCI port management (Linux)
  - Device binding/unbinding (Linux)

- **TUI Features**:
  - Tabbed interface (Local Devices, Remote Devices, Attached, Hosts)
  - Device lists with status indicators
  - Host connection management
  - Attach/detach operations
  - Help popup with keybindings
  - Connect dialog
  - Status bar with keybinding hints


