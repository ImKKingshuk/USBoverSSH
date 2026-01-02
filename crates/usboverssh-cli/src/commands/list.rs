//! List USB devices command

use crate::OutputFormat;
use anyhow::Result;
use colored::Colorize;
use usboverssh_core::{Config, DeviceFilter, DeviceManager};

/// Run the list command
pub async fn run(
    host: Option<String>,
    all: bool,
    class_filter: Option<String>,
    config: &Config,
    format: OutputFormat,
) -> Result<()> {
    if let Some(ref host_spec) = host {
        // Remote listing
        list_remote(host_spec, all, class_filter, config, format).await
    } else {
        // Local listing
        list_local(all, class_filter, format).await
    }
}

/// List local USB devices
async fn list_local(
    all: bool,
    class_filter: Option<String>,
    format: OutputFormat,
) -> Result<()> {
    let mut manager = DeviceManager::new()?;
    let devices = manager.list_devices()?;

    if devices.is_empty() {
        match format {
            OutputFormat::Text => {
                println!("{}", "No USB devices found.".yellow());
            }
            OutputFormat::Json => {
                println!("[]");
            }
        }
        return Ok(());
    }

    // Apply class filter
    let devices: Vec<_> = devices
        .iter()
        .filter(|d| {
            if let Some(ref class) = class_filter {
                d.device_class.short_name().to_lowercase().contains(&class.to_lowercase())
            } else {
                true
            }
        })
        .collect();

    match format {
        OutputFormat::Text => {
            println!(
                "{}\n",
                format!("Found {} USB device(s) on this machine:", devices.len()).bright_green()
            );

            // Header
            println!(
                "  {:<12} {:<11} {:<10} {}",
                "BUS-ID".bright_cyan(),
                "VID:PID".bright_cyan(),
                "CLASS".bright_cyan(),
                "PRODUCT".bright_cyan()
            );
            println!("  {}", "─".repeat(70).dimmed());

            for device in &devices {
                let status_indicator = if device.is_attached {
                    "●".bright_green()
                } else if device.is_bound {
                    "○".yellow()
                } else {
                    "○".dimmed()
                };

                let class_badge = format!("[{}]", device.device_class.short_name()).bright_magenta();

                print!(
                    "  {} {:<10} {:04x}:{:04x}   {:<10} ",
                    status_indicator,
                    device.bus_id.bright_white(),
                    device.vendor_id,
                    device.product_id,
                    class_badge,
                );

                // Product name
                if let Some(ref product) = device.product {
                    print!("{}", product.bright_white());
                } else {
                    print!("{}", "(Unknown)".dimmed());
                }

                // Manufacturer
                if let Some(ref manufacturer) = device.manufacturer {
                    print!(" {}", format!("({})", manufacturer).dimmed());
                }

                println!();

                // Extended info
                if all {
                    if let Some(ref serial) = device.serial {
                        println!("              {} {}", "Serial:".dimmed(), serial);
                    }
                    println!(
                        "              {} {}",
                        "Speed:".dimmed(),
                        device.speed.as_str()
                    );
                }
            }

            println!();
            println!(
                "  {} = attached  {} = exported  {} = available",
                "●".bright_green(),
                "○".yellow(),
                "○".dimmed()
            );
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&devices)?;
            println!("{}", json);
        }
    }

    Ok(())
}

/// List remote USB devices via SSH
async fn list_remote(
    host_spec: &str,
    all: bool,
    class_filter: Option<String>,
    config: &Config,
    format: OutputFormat,
) -> Result<()> {
    use usboverssh_core::{SshSession, TunnelConfig};

    let host_config = config.get_host(host_spec);

    match format {
        OutputFormat::Text => {
            println!(
                "{} {}@{}:{} ...\n",
                "Connecting to".dimmed(),
                host_config.user.bright_cyan(),
                host_config.hostname.bright_white(),
                host_config.port.to_string().dimmed()
            );
        }
        OutputFormat::Json => {}
    }

    let tunnel_config = TunnelConfig::new(host_config.clone());
    let mut session = SshSession::new(tunnel_config);

    session.connect().await?;

    // Execute remote device list command
    // For now, we'll execute `usboverssh list --format json` on the remote
    let output = session
        .exec("usboverssh list --format json 2>/dev/null || lsusb -v 2>/dev/null || echo '[]'")
        .await?;

    session.disconnect().await?;

    // Try to parse as JSON first
    if let Ok(devices) = serde_json::from_str::<Vec<usboverssh_core::DeviceInfo>>(&output) {
        match format {
            OutputFormat::Text => {
                println!(
                    "{}\n",
                    format!(
                        "Found {} USB device(s) on {}:",
                        devices.len(),
                        host_config.hostname
                    )
                    .bright_green()
                );

                for device in &devices {
                    let class_badge =
                        format!("[{}]", device.device_class.short_name()).bright_magenta();

                    println!(
                        "  {:<10} {:04x}:{:04x}   {:<10} {}",
                        device.bus_id.bright_white(),
                        device.vendor_id,
                        device.product_id,
                        class_badge,
                        device.display_name().bright_white(),
                    );
                }
            }
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&devices)?);
            }
        }
    } else {
        // Fallback: print raw output
        match format {
            OutputFormat::Text => {
                println!("{}", output);
            }
            OutputFormat::Json => {
                println!("{{\"error\": \"Could not parse device list\", \"raw\": {:?}}}", output);
            }
        }
    }

    Ok(())
}
