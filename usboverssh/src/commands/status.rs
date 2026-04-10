//! Status command - show attached devices

use crate::OutputFormat;
use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use usboverssh::Config;

#[derive(Debug, Serialize)]
struct AttachedDevice {
    port: u32,
    status: String,
    speed: String,
    dev_id: String,
    bus_id: String,
    hub: String,
}

/// Run the status command
pub async fn run(_config: &Config, format: OutputFormat) -> Result<()> {
    let devices = get_attached_devices()?;

    match format {
        OutputFormat::Text => {
            if devices.is_empty() {
                println!(
                    "{} {}",
                    "ℹ".bright_blue(),
                    "No USB devices are currently attached via USB/IP.".dimmed()
                );
            } else {
                println!(
                    "{}\n",
                    format!("Currently attached devices ({}):", devices.len()).bright_green()
                );

                println!(
                    "  {:<6} {:<8} {:<12} {:<12} {}",
                    "PORT".bright_cyan(),
                    "HUB".bright_cyan(),
                    "SPEED".bright_cyan(),
                    "DEV ID".bright_cyan(),
                    "BUS ID".bright_cyan()
                );
                println!("  {}", "─".repeat(60).dimmed());

                for device in &devices {
                    println!(
                        "  {:<6} {:<8} {:<12} {:<12} {}",
                        device.port.to_string().bright_white(),
                        device.hub.bright_magenta(),
                        device.speed.dimmed(),
                        device.dev_id,
                        device.bus_id.bright_yellow()
                    );
                }
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&devices)?);
        }
    }

    Ok(())
}

/// Get currently attached devices from VHCI
fn get_attached_devices() -> Result<Vec<AttachedDevice>> {
    let devices = Vec::new();

    #[cfg(target_os = "linux")]
    {
        use std::fs;
        use std::path::Path;

        let vhci_base = Path::new("/sys/bus/usb/devices/platform");

        for entry in fs::read_dir(vhci_base).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("vhci_hcd") {
                continue;
            }

            let vhci_path = entry.path();

            for status_entry in fs::read_dir(&vhci_path).into_iter().flatten().flatten() {
                let status_name = status_entry.file_name().to_string_lossy().to_string();
                if !status_name.starts_with("status") {
                    continue;
                }

                let content = match fs::read_to_string(status_entry.path()) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                for line in content.lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 7 {
                        let hub = parts[0];
                        let port: u32 = parts[1].parse().unwrap_or(0);
                        let status: u32 = parts[2].parse().unwrap_or(0);
                        let speed: u32 = parts[3].parse().unwrap_or(0);
                        let dev_id = parts[4];
                        let bus_id = parts[6];

                        // Status 6 = VDEV_ST_USED (attached)
                        if status == 6 {
                            let speed_str = match speed {
                                1 => "Low (1.5)",
                                2 => "Full (12)",
                                3 => "High (480)",
                                5 => "Super (5G)",
                                6 => "Super+ (10G)",
                                _ => "Unknown",
                            };

                            devices.push(AttachedDevice {
                                port,
                                status: "attached".to_string(),
                                speed: speed_str.to_string(),
                                dev_id: dev_id.to_string(),
                                bus_id: bus_id.to_string(),
                                hub: hub.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(devices)
}
