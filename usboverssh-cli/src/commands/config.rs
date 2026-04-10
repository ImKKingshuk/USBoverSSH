//! Configuration management commands

use crate::ConfigAction;
use anyhow::Result;
use colored::Colorize;
use usboverssh_core::Config;

/// Run config subcommand
pub async fn run(action: ConfigAction, config: &Config, quiet: bool) -> Result<()> {
    match action {
        ConfigAction::Show => show_config(config, quiet),
        ConfigAction::Path => show_path(quiet),
        ConfigAction::Init { force } => init_config(force, quiet),
        ConfigAction::AddHost { name, spec } => add_host(name, spec, config, quiet),
    }
}

/// Show current configuration
fn show_config(config: &Config, _quiet: bool) -> Result<()> {
    let toml = toml::to_string_pretty(config)?;
    println!("{}", toml);
    Ok(())
}

/// Show configuration file path
fn show_path(quiet: bool) -> Result<()> {
    if let Some(path) = Config::default_path() {
        if quiet {
            println!("{}", path.display());
        } else {
            println!(
                "{} {}",
                "Configuration file:".bright_cyan(),
                path.display().to_string().bright_white()
            );

            if path.exists() {
                println!("  {} File exists", "✓".bright_green());
            } else {
                println!(
                    "  {} File does not exist (using defaults)",
                    "ℹ".bright_blue()
                );
                println!(
                    "  {} Run '{}' to create one",
                    "→".dimmed(),
                    "usboverssh config init".bright_yellow()
                );
            }
        }
    } else {
        if !quiet {
            println!("{} Could not determine configuration path", "⚠".yellow());
        }
    }
    Ok(())
}

/// Initialize configuration file
fn init_config(force: bool, quiet: bool) -> Result<()> {
    let path = Config::default_path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine configuration path"))?;

    if path.exists() && !force {
        if !quiet {
            println!(
                "{} Configuration file already exists at:\n  {}\n",
                "⚠".yellow(),
                path.display().to_string().bright_white()
            );
            println!(
                "  Use '{}' to overwrite.",
                "usboverssh config init --force".bright_yellow()
            );
        }
        return Ok(());
    }

    // Create parent directories
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Generate example config
    let example = usboverssh_core::config::generate_example_config();
    std::fs::write(&path, &example)?;

    if !quiet {
        println!(
            "{} Created configuration file:\n  {}\n",
            "✓".bright_green(),
            path.display().to_string().bright_white()
        );
        println!("  Edit this file to customize USBoverSSH settings.");
    }

    Ok(())
}

/// Add a host to configuration
fn add_host(name: String, spec: String, config: &Config, quiet: bool) -> Result<()> {
    use usboverssh_core::config::HostConfig;

    let mut config = config.clone();
    let host_config = HostConfig::parse(&spec);

    config.hosts.insert(name.clone(), host_config.clone());
    config.save()?;

    if !quiet {
        println!(
            "{} Added host '{}':\n",
            "✓".bright_green(),
            name.bright_yellow()
        );
        println!(
            "  {} {}",
            "Hostname:".dimmed(),
            host_config.hostname.bright_white()
        );
        println!("  {} {}", "User:".dimmed(), host_config.user.bright_white());
        println!(
            "  {} {}",
            "Port:".dimmed(),
            host_config.port.to_string().bright_white()
        );
        println!();
        println!(
            "  You can now use '{}' instead of '{}'",
            format!("usboverssh list {}", name).bright_cyan(),
            format!("usboverssh list {}", spec).dimmed()
        );
    }

    Ok(())
}
