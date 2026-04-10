//! USBoverSSH TUI - Interactive Terminal UI
//!
//! A powerful cross-platform tool for connecting USB devices securely over SSH.

mod tui;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
use std::io;
use tracing::Level;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use usboverssh::Config;

/// USBoverSSH TUI - Connect USB devices securely over SSH
#[derive(Parser)]
#[command(
    name = "usboverssh",
    author = "ImKKingshuk <https://github.com/ImKKingshuk>",
    version,
    about = "🔌 USBoverSSH TUI - Interactive terminal UI for USB device management",
    styles = get_styles(),
)]
pub struct TuiArgs {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Suppress all output except errors
    #[arg(short, long)]
    quiet: bool,

    /// Path to configuration file
    #[arg(short, long, env = "USBOVERSSH_CONFIG")]
    config: Option<String>,

    /// Connect to hosts from config on startup
    #[arg(short, long)]
    connect: bool,

    /// Run in CLI/headless mode instead of TUI
    #[arg(long, alias = "headless")]
    cli: bool,

    /// CLI subcommands (only used with --cli flag)
    #[arg(short, long, global = true)]
    format: usboverssh::OutputFormat,

    #[command(subcommand)]
    command: Option<Commands>,
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
    let args = TuiArgs::parse();

    // Initialize logging
    init_logging(args.verbose, args.quiet);

    // Load configuration
    let config = Config::load_or_default()?;

    // If --cli flag is specified or if a subcommand is provided, run CLI mode
    if args.cli || args.command.is_some() {
        match args.command {
            Some(Commands::List { host, all, class }) => {
                if !args.quiet {
                    print_banner();
                }
                usboverssh::commands::list::run(host, all, class, &config, args.format).await
            }

            Some(Commands::Attach {
                host,
                device,
                persistent,
                daemon,
            }) => {
                if !args.quiet && !daemon {
                    print_banner();
                }
                usboverssh::commands::attach::run(host, device, persistent, daemon, &config).await
            }

            Some(Commands::Detach { device }) => {
                if !args.quiet {
                    print_banner();
                }
                usboverssh::commands::detach::run(device, &config).await
            }

            Some(Commands::Status) => {
                if !args.quiet {
                    print_banner();
                }
                usboverssh::commands::status::run(&config, args.format).await
            }

            Some(Commands::Serve {
                address,
                port,
                all,
                devices,
            }) => {
                if !args.quiet {
                    print_banner();
                }
                usboverssh::commands::serve::run(address, port, all, devices, &config).await
            }

            Some(Commands::Config { action }) => {
                let lib_action = match action {
                    ConfigAction::Show => usboverssh::commands::ConfigAction::Show,
                    ConfigAction::Path => usboverssh::commands::ConfigAction::Path,
                    ConfigAction::Init { force } => {
                        usboverssh::commands::ConfigAction::Init { force }
                    }
                    ConfigAction::AddHost { name, spec } => {
                        usboverssh::commands::ConfigAction::AddHost { name, spec }
                    }
                };
                usboverssh::commands::config::run(lib_action, &config, args.quiet).await
            }

            Some(Commands::Completions { shell }) => {
                let mut cmd = TuiArgs::command();
                generate(shell, &mut cmd, "usboverssh", &mut io::stdout());
                Ok(())
            }

            None => {
                // --cli flag without command - show help
                println!(
                    "{}",
                    "Run 'usboverssh --help' for usage information.\n".dimmed()
                );
                Ok(())
            }
        }
    } else {
        // Default TUI mode
        if !args.quiet {
            print_banner();
        }

        tui::run(args.connect, config).await
    }
}
