//! Detach USB device command

use crate::Config;
use anyhow::Result;
use colored::Colorize;

/// Run the detach command
pub async fn run(device_pattern: String, _config: &Config) -> Result<()> {
    println!(
        "{} {} {} ...\n",
        "🔌".bright_cyan(),
        "Detaching device".bright_yellow(),
        device_pattern.bright_white()
    );

    // Check if pattern is "all"
    if device_pattern.to_lowercase() == "all" {
        detach_all().await?;
    } else {
        detach_device(&device_pattern).await?;
    }

    Ok(())
}

/// Detach a specific device
async fn detach_device(_pattern: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        use std::path::Path;

        let vhci_base = Path::new("/sys/bus/usb/devices/platform");
        let mut found = false;

        // Find VHCI status files
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

                let content = fs::read_to_string(status_entry.path())?;

                for line in content.lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 7 {
                        let port: u32 = parts[1].parse().unwrap_or(0);
                        let status: u32 = parts[2].parse().unwrap_or(0);
                        let bus_id = parts[6];

                        // Status 6 = VDEV_ST_USED
                        if status == 6 {
                            // Check if matches pattern
                            if bus_id.contains(pattern)
                                || pattern == "*"
                                || pattern.to_lowercase() == "all"
                            {
                                // Detach
                                let detach_path = vhci_path.join("detach");
                                fs::write(&detach_path, port.to_string())?;

                                println!(
                                    "  {} Detached {} (port {})",
                                    "✓".bright_green(),
                                    bus_id.bright_white(),
                                    port
                                );
                                found = true;
                            }
                        }
                    }
                }
            }
        }

        if !found {
            println!(
                "  {} No devices matching '{}' are currently attached",
                "ℹ".bright_blue(),
                pattern.bright_yellow()
            );
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("  {} Detach is only supported on Linux", "⚠".yellow());
    }

    Ok(())
}

/// Detach all devices
async fn detach_all() -> Result<()> {
    detach_device("*").await
}
