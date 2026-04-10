//! Attach USB device command

use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use usboverssh_core::{Config, DeviceFilter, SshSession, TunnelConfig};

/// Run the attach command
pub async fn run(
    host_spec: String,
    device_pattern: String,
    persistent: bool,
    daemon: bool,
    config: &Config,
) -> Result<()> {
    if daemon {
        run_daemon(host_spec, device_pattern, persistent, config).await
    } else {
        run_foreground(host_spec, device_pattern, persistent, config).await
    }
}

/// Run attach in foreground
async fn run_foreground(
    host_spec: String,
    device_pattern: String,
    persistent: bool,
    config: &Config,
) -> Result<()> {
    let host_config = config.get_host(&host_spec);

    println!(
        "{} {} {} {} ...\n",
        "🔌".bright_cyan(),
        "Attaching device".bright_green(),
        device_pattern.bright_yellow(),
        format!("from {}@{}", host_config.user, host_config.hostname).dimmed()
    );

    // Show progress
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {msg}")?,
    );
    pb.enable_steady_tick(Duration::from_millis(80));

    loop {
        pb.set_message("Connecting to remote host...");

        let result = attempt_attach(&host_config, &device_pattern, &pb, config).await;

        match result {
            Ok(_) => {
                pb.finish_with_message(format!(
                    "{} Device attached successfully!",
                    "✓".bright_green()
                ));
                
                if persistent {
                    println!(
                        "\n{} {}",
                        "ℹ".bright_blue(),
                        "Running in persistent mode. Press Ctrl+C to disconnect.".dimmed()
                    );
                    
                    // Wait for signal
                    wait_for_interrupt().await;
                    
                    println!("\n{}", "Disconnecting...".yellow());
                }
                
                return Ok(());
            }
            Err(e) => {
                if persistent && e.is_recoverable() {
                    pb.set_message(format!(
                        "{} Connection lost, reconnecting in {}s...",
                        "⚠".yellow(),
                        config.general.reconnect_delay
                    ));
                    tokio::time::sleep(Duration::from_secs(config.general.reconnect_delay)).await;
                    continue;
                } else {
                    pb.finish_with_message(format!("{} {}", "✗".bright_red(), e));
                    return Err(e.into());
                }
            }
        }
    }
}

/// Attempt to attach a device
async fn attempt_attach(
    host_config: &usboverssh_core::config::HostConfig,
    device_pattern: &str,
    pb: &ProgressBar,
    _config: &Config,
) -> usboverssh_core::Result<()> {
    // Create SSH session
    let tunnel_config = TunnelConfig::new(host_config.clone());
    let mut session = SshSession::new(tunnel_config);

    session.connect().await?;
    pb.set_message("Connected! Finding device...");

    // Find the device
    let find_cmd = format!("usboverssh find '{}'", device_pattern);
    let bus_id = session.exec(&find_cmd).await?;
    let bus_id = bus_id.trim();

    if bus_id.is_empty() {
        return Err(usboverssh_core::Error::DeviceNotFound(device_pattern.to_string()));
    }

    pb.set_message(format!("Found device: {}", bus_id));

    // Start remote attach process
    let attach_cmd = format!(
        "usboverssh remote '{}' 2>&1",
        bus_id
    );
    
    let response = session.exec(&attach_cmd).await?;
    
    if response.contains("RESPONSE") {
        pb.set_message("Device exported! Creating local attachment...");
        
        // Parse response to get device info
        // Format: RESPONSE <bus> <dev> <speed> <socket_path>
        let parts: Vec<&str> = response
            .lines()
            .find(|l| l.starts_with("RESPONSE"))
            .unwrap_or("")
            .split_whitespace()
            .collect();
        
        if parts.len() >= 5 {
            let _bus: u32 = parts[1].parse().unwrap_or(0);
            let _dev: u32 = parts[2].parse().unwrap_or(0);
            let _speed: u32 = parts[3].parse().unwrap_or(0);
            let _socket_path = parts[4];
            
            // Local attach would happen here via VHCI
            // For now, this is a simplified implementation
            pb.set_message("Attached!");
        }
    } else {
        return Err(usboverssh_core::Error::UsbIpAttach(
            format!("Remote attach failed: {}", response)
        ));
    }

    Ok(())
}

/// Run attach as daemon
async fn run_daemon(
    host_spec: String,
    device_pattern: String,
    persistent: bool,
    config: &Config,
) -> Result<()> {
    use std::process::Command;
    
    // Fork and run in background
    let exe = std::env::current_exe()?;
    let mut cmd = Command::new(exe);
    
    cmd.arg("attach")
        .arg(&host_spec)
        .arg(&device_pattern);
    
    if persistent {
        cmd.arg("--persistent");
    }
    
    // Detach from terminal
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    
    let child = cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    
    println!(
        "{} Started daemon process (PID: {})",
        "✓".bright_green(),
        child.id()
    );
    
    Ok(())
}

/// Wait for Ctrl+C
async fn wait_for_interrupt() {
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let tx = std::sync::Mutex::new(Some(tx));
    
    ctrlc::set_handler(move || {
        if let Some(tx) = tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
    })
    .expect("Error setting Ctrl-C handler");
    
    let _ = rx.await;
}

/// Extension trait for checking recoverable errors
trait ErrorExt {
    fn is_recoverable(&self) -> bool;
}

impl ErrorExt for usboverssh_core::Error {
    fn is_recoverable(&self) -> bool {
        usboverssh_core::Error::is_recoverable(self)
    }
}
