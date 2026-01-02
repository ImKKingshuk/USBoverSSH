//! USB/IP Server command

use anyhow::Result;
use colored::Colorize;
use usboverssh_core::{Config, DeviceFilter};

/// Run the serve command
pub async fn run(
    address: String,
    port: u16,
    export_all: bool,
    devices: Vec<String>,
    _config: &Config,
) -> Result<()> {
    println!(
        "{} {} {}:{}\n",
        "🖥".bright_cyan(),
        "Starting USB/IP server on".bright_green(),
        address.bright_white(),
        port.to_string().bright_yellow()
    );

    // Build device filters
    let filters: Vec<DeviceFilter> = devices
        .iter()
        .map(|d| DeviceFilter::parse(d))
        .collect();

    // Create server configuration
    let server_config = usboverssh_core::server::ServerConfig {
        listen_addr: Some(address.clone()),
        listen_port: port,
        unix_socket: None,
        device_filters: filters.clone(),
        export_all,
    };

    // Create and start server
    let server = usboverssh_core::Server::new(server_config)?;

    // List available devices
    let available = server.available_devices().await?;

    if available.is_empty() {
        println!(
            "  {} {}",
            "⚠".yellow(),
            "No exportable devices found.".dimmed()
        );
    } else {
        println!(
            "  {} Exporting {} device(s):\n",
            "✓".bright_green(),
            available.len()
        );

        for device in &available {
            println!(
                "    {} {} {}",
                "•".bright_cyan(),
                device.bus_id.bright_white(),
                device.display_name().dimmed()
            );
        }
    }

    println!();
    println!(
        "  {} {}",
        "ℹ".bright_blue(),
        "Press Ctrl+C to stop the server.".dimmed()
    );
    println!();

    // Setup Ctrl+C handler
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let shutdown_tx = std::sync::Mutex::new(Some(shutdown_tx));
    
    let server_handle = tokio::spawn(async move {
        server.run().await
    });

    ctrlc::set_handler(move || {
        if let Some(tx) = shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
    })?;

    // Wait for shutdown signal
    let _ = shutdown_rx.await;

    println!("\n{} Shutting down server...", "⏳".yellow());

    // Stop server
    server_handle.abort();

    println!("{} Server stopped.", "✓".bright_green());

    Ok(())
}
