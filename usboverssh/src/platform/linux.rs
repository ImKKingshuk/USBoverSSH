//! Linux-specific USB device enumeration and USB/IP support
//!
//! Uses sysfs for device enumeration and kernel module management.

use crate::device::{DeviceClass, DeviceInfo, DeviceSpeed};
use crate::error::{Error, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

const SYSFS_USB_DEVICES: &str = "/sys/bus/usb/devices";
const SYSFS_USB_DRIVERS: &str = "/sys/bus/usb/drivers";
const USBIP_HOST_DRIVER: &str = "usbip-host";
const VHCI_HCD_MODULE: &str = "vhci-hcd";

/// Enumerate all USB devices on Linux via sysfs
pub fn enumerate_devices() -> Result<Vec<DeviceInfo>> {
    let mut devices = Vec::new();

    let entries = fs::read_dir(SYSFS_USB_DEVICES)
        .map_err(|e| Error::UsbEnumeration(format!("Cannot read {}: {}", SYSFS_USB_DEVICES, e)))?;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip non-device entries (like usb1, usb2 root hubs without bus ID format)
        // Valid bus IDs look like: 1-1, 1-1.2, 3-2.4.1, etc.
        if !is_valid_bus_id(&name) {
            continue;
        }

        let path = entry.path();

        // Read device attributes
        if let Some(device) = parse_device_from_sysfs(&path, &name) {
            devices.push(device);
        }
    }

    // Sort by bus number, then device number
    devices.sort_by(|a, b| a.bus_num.cmp(&b.bus_num).then(a.dev_num.cmp(&b.dev_num)));

    Ok(devices)
}

/// Check if a string is a valid USB bus ID format
pub fn is_valid_bus_id(name: &str) -> bool {
    // Valid formats: "1-1", "1-1.2", "3-2.4.1"
    if !name.contains('-') {
        return false;
    }

    let parts: Vec<&str> = name.splitn(2, '-').collect();
    if parts.len() != 2 {
        return false;
    }

    // First part should be a number (bus)
    if parts[0].parse::<u8>().is_err() {
        return false;
    }

    // Second part should be port.port.port format
    for segment in parts[1].split('.') {
        if segment.parse::<u8>().is_err() {
            return false;
        }
    }

    true
}

/// Parse device information from sysfs
fn parse_device_from_sysfs(path: &Path, bus_id: &str) -> Option<DeviceInfo> {
    let read_attr = |attr: &str| -> Option<String> {
        fs::read_to_string(path.join(attr))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let read_attr_hex = |attr: &str| -> Option<u16> {
        read_attr(attr).and_then(|s| u16::from_str_radix(&s, 16).ok())
    };

    let read_attr_u8 = |attr: &str| -> Option<u8> { read_attr(attr).and_then(|s| s.parse().ok()) };

    // Required attributes
    let vendor_id = read_attr_hex("idVendor")?;
    let product_id = read_attr_hex("idProduct")?;
    let bus_num = read_attr_u8("busnum")?;
    let dev_num = read_attr_u8("devnum")?;

    // Optional attributes
    let manufacturer = read_attr("manufacturer");
    let product = read_attr("product");
    let serial = read_attr("serial");
    let device_class_code = read_attr_u8("bDeviceClass").unwrap_or(0);
    let num_configurations = read_attr_u8("bNumConfigurations").unwrap_or(1);

    // Parse speed
    let speed = read_attr("speed")
        .and_then(|s| s.parse::<u32>().ok())
        .map(DeviceSpeed::from_speed_mbps)
        .unwrap_or(DeviceSpeed::Unknown);

    // Check if device is bound to usbip-host
    let usbip_driver_path = Path::new(SYSFS_USB_DRIVERS)
        .join(USBIP_HOST_DRIVER)
        .join(bus_id);
    let is_bound = usbip_driver_path.exists();

    // Check usbip_status to see if attached
    let usbip_status_path = path.join("usbip_status");
    let is_attached = fs::read_to_string(&usbip_status_path)
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .map(|status| status == 2) // SDEV_ST_USED
        .unwrap_or(false);

    Some(DeviceInfo {
        bus_id: bus_id.to_string(),
        vendor_id,
        product_id,
        device_class: DeviceClass::from_code(device_class_code),
        bus_num,
        dev_num,
        speed,
        manufacturer,
        product,
        serial,
        num_configurations,
        is_attached,
        is_bound,
    })
}

/// Check if USB/IP kernel modules are available
pub fn check_usbip_available() -> Result<bool> {
    // Check if modules exist
    let modules_dir = Path::new("/lib/modules");
    if !modules_dir.exists() {
        return Ok(false);
    }

    // Try to find usbip modules in kernel modules
    let uname = Command::new("uname")
        .arg("-r")
        .output()
        .map_err(|e| Error::Other(format!("Failed to get kernel version: {}", e)))?;

    let kernel_version = String::from_utf8_lossy(&uname.stdout).trim().to_string();
    let module_path = modules_dir
        .join(&kernel_version)
        .join("kernel/drivers/usb/usbip");

    Ok(module_path.exists())
}

/// Load required USB/IP kernel modules
pub fn load_kernel_modules(server_mode: bool) -> Result<()> {
    if server_mode {
        // Server needs usbip-host
        load_module(USBIP_HOST_DRIVER)?;
    } else {
        // Client needs vhci-hcd
        load_module(VHCI_HCD_MODULE)?;
    }
    Ok(())
}

/// Load a kernel module using modprobe
fn load_module(module: &str) -> Result<()> {
    let output = Command::new("modprobe")
        .arg("-v")
        .arg(module)
        .output()
        .map_err(|e| Error::Other(format!("Failed to run modprobe: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::KernelModuleNotLoaded {
            module: module.to_string(),
            suggestion: format!("sudo modprobe {}", module),
        });
    }

    Ok(())
}

/// Bind a device to the usbip-host driver
pub fn bind_device(bus_id: &str) -> Result<()> {
    let driver_path = Path::new(SYSFS_USB_DRIVERS).join(USBIP_HOST_DRIVER);

    // Add to match_busid
    write_sysfs(&driver_path.join("match_busid"), format!("add {}", bus_id))?;

    // Unbind from current driver
    let usb_driver_path = Path::new(SYSFS_USB_DRIVERS).join("usb");
    if usb_driver_path.join(bus_id).exists() {
        write_sysfs(&usb_driver_path.join("unbind"), bus_id)?;
    }

    // Bind to usbip-host
    write_sysfs(&driver_path.join("bind"), bus_id)?;

    Ok(())
}

/// Unbind a device from the usbip-host driver
pub fn unbind_device(bus_id: &str) -> Result<()> {
    let driver_path = Path::new(SYSFS_USB_DRIVERS).join(USBIP_HOST_DRIVER);

    // Unbind from usbip-host
    write_sysfs(&driver_path.join("unbind"), bus_id)?;

    // Remove from match_busid
    write_sysfs(&driver_path.join("match_busid"), format!("del {}", bus_id))?;

    // Rebind to original driver
    write_sysfs(&driver_path.join("rebind"), bus_id)?;

    Ok(())
}

/// Attach a device socket to usbip-host
pub fn attach_device_socket(bus_id: &str, socket_fd: i32) -> Result<()> {
    let device_path = Path::new(SYSFS_USB_DEVICES).join(bus_id);
    write_sysfs(&device_path.join("usbip_sockfd"), socket_fd.to_string())?;
    Ok(())
}

/// Find an available VHCI port for the given speed
pub fn find_vhci_port(speed: DeviceSpeed) -> Result<(String, u32)> {
    let hub_type = if speed.to_usbip_speed() >= 5 {
        "ss"
    } else {
        "hs"
    };

    let platform_path = Path::new(SYSFS_USB_DEVICES).join("platform");

    for entry in fs::read_dir(&platform_path)
        .map_err(|e| Error::VhciPortUnavailable(speed))?
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("vhci_hcd") {
            continue;
        }

        let vhci_path = entry.path();

        // Check status files
        for status_entry in fs::read_dir(&vhci_path).into_iter().flatten().flatten() {
            let status_name = status_entry.file_name().to_string_lossy().to_string();
            if !status_name.starts_with("status") {
                continue;
            }

            let status_content = fs::read_to_string(status_entry.path())
                .map_err(|_| Error::VhciPortUnavailable(speed))?;

            for line in status_content.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let hub = parts[0];
                    let port: u32 = parts[1].parse().unwrap_or(0);
                    let sta: u32 = parts[2].parse().unwrap_or(0);

                    // VDEV_ST_NULL = 4 (available)
                    if sta == 4 && hub == hub_type {
                        return Ok((vhci_path.to_string_lossy().to_string(), port));
                    }
                }
            }
        }
    }

    Err(Error::VhciPortUnavailable(speed))
}

/// Attach a device to VHCI
pub fn vhci_attach(
    vhci_path: &str,
    port: u32,
    socket_fd: i32,
    devid: u32,
    speed: u32,
) -> Result<()> {
    let attach_path = Path::new(vhci_path).join("attach");
    let attach_data = format!("{} {} {} {}", port, socket_fd, devid, speed);
    write_sysfs(&attach_path, attach_data)?;
    Ok(())
}

/// Detach a device from VHCI
pub fn vhci_detach(vhci_path: &str, port: u32) -> Result<()> {
    let detach_path = Path::new(vhci_path).join("detach");
    write_sysfs(&detach_path, port.to_string())?;
    Ok(())
}

/// Write to a sysfs file
fn write_sysfs<P: AsRef<Path>, S: AsRef<str>>(path: P, data: S) -> Result<()> {
    fs::write(path.as_ref(), data.as_ref()).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            Error::PermissionDenied(format!("Cannot write to {:?}", path.as_ref()))
        } else {
            Error::Io(e)
        }
    })
}
