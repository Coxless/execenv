use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "execenv",
    version,
    about = "Decrypt encrypted config and exec a command with injected env vars"
)]
pub struct Cli {
    /// Secret source provider (MVP: sops only)
    #[arg(long, value_enum)]
    pub provider: Provider,

    /// Path to the encrypted config file
    #[arg(long)]
    pub file: PathBuf,

    /// Format of the decrypted content
    #[arg(long, value_enum, default_value_t = Format::Dotenv)]
    pub format: Format,

    /// Drop the parent process's environment; only pass decrypted vars to the child
    #[arg(long)]
    pub clean_env: bool,

    /// Command to execute with injected env vars (everything after --)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true, num_args = 1..)]
    pub command: Vec<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Provider {
    Sops,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    Dotenv,
    Json,
}
