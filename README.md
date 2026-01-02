# USBoverSSH

<div align="center">

```
╦ ╦╔═╗╔═╗┌─┐┬  ┬┌─┐┬─┐╔═╗╔═╗╦ ╦
║ ║╚═╗╠═╣│ │└┐┌┘├┤ ├┬┘╚═╗╚═╗╠═╣
╚═╝╚═╝╩═╩└─┘ └┘ └─┘┴└─╚═╝╚═╝╩ ╩
```

**🔌 The Ultimate USB over SSH Solution**

[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-GPL--3.0-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Linux%20|%20macOS%20|%20Windows-green?style=flat-square)](https://github.com/ImKKingshuk/USBoverSSH)

*Connect USB devices between machines securely over SSH*

</div>

---

## ✨ Features

- 🔐 **Secure SSH Tunneling** - All USB traffic encrypted through SSH
- 🖥️ **Cross-Platform** - Linux, macOS, and Windows client support
- 🎨 **Interactive TUI** - Beautiful terminal UI for device management
- 🔌 **USB/IP Protocol** - Industry-standard USB over network
- ⚡ **Hot-Plug Support** - Automatic device detection and reconnection
- 📁 **Configuration Files** - TOML-based settings with named hosts
- 🔄 **Persistent Mode** - Auto-reconnect on connection drops
- 🌐 **Multi-Host** - Connect to multiple servers simultaneously

## 🚀 Quick Start

### List Local USB Devices

```bash
usboverssh list
```

### List Remote USB Devices

```bash
usboverssh list [email protected]
```

### Attach a Remote Device

```bash
# By VID:PID
usboverssh attach [email protected] 0xXXXX:0xXXXX

# By product name
usboverssh attach [email protected] "Example Device"

# With persistent reconnection
usboverssh attach [email protected] 0xXXXX:0xXXXX --persistent
```

### Detach a Device

```bash
usboverssh detach 0xXXXX:0xXXXX

# Detach all
usboverssh detach all
```

### Interactive TUI

```bash
usboverssh tui
# or simply
usboverssh
```

### Run as USB/IP Server

```bash
usboverssh serve --address xxx.xxx.xxx.xxx --port xxxx
```

## 🎮 TUI Controls

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch between panes |
| `↑` / `k`, `↓` / `j` | Navigate items |
| `Enter` | Activate selected item |
| `a` | Attach selected device |
| `d` | Detach selected device |
| `r` / `F5` | Refresh device list |
| `c` | Connect to new host |
| `h` | Show hosts panel |
| `?` / `F1` | Toggle help |
| `q` / `Esc` | Quit / Close popup |

## ⚙️ Configuration

Configuration file location:

- **Linux/macOS**: `~/.config/usboverssh/config.toml`
- **Windows**: `%APPDATA%\usboverssh\config.toml`

Generate a default config:

```bash
usboverssh config init
```

## 📋 Requirements

### Linux (Server & Client)

- Kernel with USB/IP support (`usbip-core`, `usbip-host`, `vhci-hcd`)
- Root/sudo for kernel module loading

### macOS (Client Only)

- No special requirements

### Windows (Client Only)

- USB/IP driver (USBIP-WIN)

### Load Kernel Modules (Linux)

```bash
# On the USB/IP server (exporting devices)
sudo modprobe usbip-host

# On the USB/IP client (attaching devices)
sudo modprobe vhci-hcd
```

## 📚 CLI Reference

```
USAGE:
    usboverssh [OPTIONS] [COMMAND]

COMMANDS:
    list        List USB devices (local or remote)
    attach      Attach a remote USB device
    detach      Detach an attached device
    status      Show currently attached devices
    serve       Start USB/IP server
    tui         Interactive TUI mode
    config      Configuration management
    completions Generate shell completions
    help        Print help information

OPTIONS:
    -v, --verbose       Increase verbosity
    -q, --quiet         Suppress output
    -c, --config <FILE> Configuration file path
        --format <FMT>  Output format (text, json)
    -h, --help          Print help
    -V, --version       Print version
```

## 🔒 Security Considerations

- All USB traffic is encrypted through SSH
- SSH key-based authentication recommended
- Only bind server to localhost when using SSH tunnels
- Firewall the USB/IP port  from untrusted networks

## 📄 License

This project is licensed under the **GNU General Public License v3.0** - see the [LICENSE](LICENSE) file for details.

## ⚠️ Disclaimer

🌟 **The developer of this tool is not responsible for any type of activity done by you using this tool. Use at your own risk.** 🌟

USBoverSSH is designed for legitimate USB device sharing over secure connections. Always ensure you have proper authorization before accessing remote USB devices. Unauthorized access may violate privacy and security laws.

---

<div align="center">

**Made with ❤️ by [@ImKKingshuk](https://github.com/ImKKingshuk)**

*If you find this project useful, please consider giving it a ⭐️*

</div>
