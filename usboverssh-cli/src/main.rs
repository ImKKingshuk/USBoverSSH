//! USBoverSSH CLI - Ultimate USB over SSH Tool
//!
//! A powerful cross-platform tool for connecting USB devices securely over SSH.

mod commands;
mod tui;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
use std::io;
use tracing::Level;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// USBoverSSH - Connect USB devices securely over SSH
#[derive(Parser)]
#[command(
    name = "usboverssh",
    author = "ImKKingshuk <https://github.com/ImKKingshuk>",
    version,
    about = "🔌 Ultimate USB over SSH - Connect USB devices securely over SSH",
    long_about = r#"
╔══════════════════════════════════════════════════════════════════════╗
║                         USBoverSSH v2.0                              ║
║                  Ultimate USB over SSH Solution                      ║
╠══════════════════════════════════════════════════════════════════════╣
║  Connect USB devices between machines securely over SSH.             ║
║  Supports Linux, macOS, and Windows clients.                         ║
║                                                                      ║
║  Features:                                                           ║
║    • Cross-platform USB device enumeration                           ║
║    • Secure SSH tunneling with key authentication                    ║
║    • Interactive TUI for device management                           ║
║    • Persistent connections with auto-reconnect                      ║
║    • Configuration file support                                      ║
║    • Daemon mode with proper logging                                 ║
╚══════════════════════════════════════════════════════════════════════╝
"#,
    after_help = "Run 'usboverssh <command> --help' for more information on a command.",
    styles = get_styles(),
)]
pub struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Path to configuration file
    #[arg(short, long, global = true, env = "USBOVERSSH_CONFIG")]
    config: Option<String>,

    /// Output format (text, json)
    #[arg(long, default_value = "text", global = true)]
    format: OutputFormat,

    #[command(subcommand)]
    command: Option<Commands>,
}

/// Output format for commands
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

/// CLI subcommands
#[derive(Subcommand)]
pub enum Commands {
    /// List USB devices (local or remote)
    #[command(visible_alias = "ls")]
    List {
        /// Remote host (user@host[:port])
        #[arg(value_name = "HOST")]
        host: Option<String>,

        /// Show all details
        #[arg(short, long)]
        all: bool,

        /// Filter by device class (hid, storage, audio, etc.)
        #[arg(short = 'c', long)]
        class: Option<String>,
    },

    /// Attach a remote USB device to this machine
    #[command(visible_alias = "a")]
    Attach {
        /// Remote host (user@host[:port])
        #[arg(value_name = "HOST")]
        host: String,

        /// Device pattern (bus-id, vid:pid, or name)
        #[arg(value_name = "DEVICE")]
        device: String,

        /// Keep connection persistent (auto-reconnect)
        #[arg(short, long)]
        persistent: bool,

        /// Run in background (daemon mode)
        #[arg(short, long)]
        daemon: bool,
    },

    /// Detach an attached USB device
    #[command(visible_alias = "d")]
    Detach {
        /// Device pattern or "all" to detach everything
        #[arg(value_name = "DEVICE")]
        device: String,
    },

    /// Show currently attached devices
    #[command(visible_alias = "st")]
    Status,

    /// Start USB/IP server for exporting devices
    Serve {
        /// Listen address
        #[arg(short, long, default_value = "127.0.0.1")]
        address: String,

        /// Listen port
        #[arg(short, long, default_value = "3240")]
        port: u16,

        /// Export all devices
        #[arg(long)]
        all: bool,

        /// Device patterns to export
        #[arg(value_name = "DEVICE")]
        devices: Vec<String>,
    },

    /// Interactive TUI mode
    #[command(visible_alias = "ui")]
    Tui {
        /// Connect to hosts from config on startup
        #[arg(short, long)]
        connect: bool,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

/// Config subcommand actions
#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    /// Show configuration file path
    Path,
    /// Generate example configuration
    Init {
        /// Force overwrite existing config
        #[arg(short, long)]
        force: bool,
    },
    /// Add a host to configuration
    AddHost {
        /// Host name (alias)
        #[arg(value_name = "NAME")]
        name: String,
        /// Host specification (user@host[:port])
        #[arg(value_name = "SPEC")]
        spec: String,
    },
}

/// Get custom clap styles
fn get_styles() -> clap::builder::Styles {
    use clap::builder::styling::{AnsiColor, Color, Style};

    clap::builder::Styles::styled()
        .usage(
            Style::new()
                .bold()
                .fg_color(Some(Color::Ansi(AnsiColor::Cyan))),
        )
        .header(
            Style::new()
                .bold()
                .fg_color(Some(Color::Ansi(AnsiColor::Cyan))),
        )
        .literal(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))))
        .placeholder(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow))))
}

/// Print the fancy banner
fn print_banner() {
    let banner = r#"
    ╦ ╦╔═╗╔╗ ┌─┐┬  ┬┌─┐┬─┐╔═╗╔═╗╦ ╦
    ║ ║╚═╗╠╩╗│ │└┐┌┘├┤ ├┬┘╚═╗╚═╗╠═╣
    ╚═╝╚═╝╚═╝└─┘ └┘ └─┘┴└─╚═╝╚═╝╩ ╩
"#;
    println!("{}", banner.bright_cyan());
    println!(
        "      {} {} • {} {}\n",
        "v".dimmed(),
        env!("CARGO_PKG_VERSION").bright_green(),
        "by".dimmed(),
        "@ImKKingshuk".bright_yellow()
    );
}

/// Initialize logging
fn init_logging(verbose: u8, quiet: bool) {
    let level = if quiet {
        Level::ERROR
    } else {
        match verbose {
            0 => Level::INFO,
            1 => Level::DEBUG,
            _ => Level::TRACE,
        }
    };

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level.to_string()));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(verbose > 1))
        .with(filter)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose, cli.quiet);

    // Load configuration
    let config = usboverssh_core::Config::load_or_default()?;

    // Handle commands
    match cli.command {
        Some(Commands::List { host, all, class }) => {
            if !cli.quiet {
                print_banner();
            }
            commands::list::run(host, all, class, &config, cli.format).await
        }

        Some(Commands::Attach {
            host,
            device,
            persistent,
            daemon,
        }) => {
            if !cli.quiet && !daemon {
                print_banner();
            }
            commands::attach::run(host, device, persistent, daemon, &config).await
        }

        Some(Commands::Detach { device }) => {
            if !cli.quiet {
                print_banner();
            }
            commands::detach::run(device, &config).await
        }

        Some(Commands::Status) => {
            if !cli.quiet {
                print_banner();
            }
            commands::status::run(&config, cli.format).await
        }

        Some(Commands::Serve {
            address,
            port,
            all,
            devices,
        }) => {
            if !cli.quiet {
                print_banner();
            }
            commands::serve::run(address, port, all, devices, &config).await
        }

        Some(Commands::Tui { connect }) => tui::run(connect, config).await,

        Some(Commands::Config { action }) => {
            commands::config::run(action, &config, cli.quiet).await
        }

        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "usboverssh", &mut io::stdout());
            Ok(())
        }

        None => {
            // No command - run TUI by default
            print_banner();
            println!(
                "{}",
                "Run 'usboverssh --help' for usage information.\n".dimmed()
            );
            tui::run(false, config).await
        }
    }
}
