use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "execenv",
    version,
    about = "Decrypt encrypted config and exec a command with injected env vars"
)]
pub struct Cli {
    /// Secret source provider
    #[arg(long, value_enum)]
    pub provider: ProviderKind,

    /// Path to the encrypted config file (required for --provider sops)
    #[arg(long, required_if_eq("provider", "sops"))]
    pub file: Option<PathBuf>,

    /// Format of the decrypted content [default: dotenv for sops, json for aws-secrets-manager]
    #[arg(long, value_enum)]
    pub format: Option<Format>,

    /// Drop the parent process's environment; only pass decrypted vars to the child
    #[arg(long)]
    pub clean_env: bool,

    /// AWS Secrets Manager secret ID (ARN or name); required for --provider aws-secrets-manager
    #[arg(long, required_if_eq("provider", "aws-secrets-manager"))]
    pub secret_id: Option<String>,

    /// AWS region override (default: SDK environment resolution)
    #[arg(long)]
    pub aws_region: Option<String>,

    /// Command to execute with injected env vars (everything after --)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true, num_args = 1..)]
    pub command: Vec<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ProviderKind {
    Sops,
    AwsSecretsManager,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    Dotenv,
    Json,
}
