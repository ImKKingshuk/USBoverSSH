//! USBoverSSH TUI - Interactive Terminal UI
//!
//! A powerful cross-platform tool for connecting USB devices securely over SSH.

mod tui;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use tracing::Level;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use usboverssh::Config;

/// USBoverSSH TUI - Connect USB devices securely over SSH
#[derive(Parser)]
#[command(
    name = "usboverssh-tui",
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

    // Run TUI
    if !args.quiet {
        print_banner();
    }

    tui::run(args.connect, config).await
}
