//! CLI Command Handlers

use serde::Serialize;

pub mod attach;
pub mod config;
pub mod detach;
pub mod list;
pub mod serve;
pub mod status;

/// Output format for commands
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

/// Config subcommand actions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    /// Show configuration file path
    Path,
    /// Generate example configuration
    Init {
        /// Force overwrite existing config
        force: bool,
    },
    /// Add a host to configuration
    AddHost {
        /// Host name (alias)
        name: String,
        /// Host specification (user@host[:port])
        spec: String,
    },
}
