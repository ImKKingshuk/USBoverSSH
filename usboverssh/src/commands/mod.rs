//! CLI Command Handlers

use serde::Serialize;

pub mod attach;
pub mod config;
pub mod detach;
pub mod list;
pub mod serve;
pub mod status;

/// Output format for commands
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}
