use anyhow::{Context, Result, anyhow};
use std::path::PathBuf;
use zeroize::Zeroizing;

use crate::cli::Format;

pub trait Provider {
    fn load(&self) -> Result<Zeroizing<String>>;
}

#[derive(Debug, Clone, Copy)]
pub enum SopsFormat {
    Dotenv,
    Json,
}

impl SopsFormat {
    fn as_str(self) -> &'static str {
        match self {
            SopsFormat::Dotenv => "dotenv",
            SopsFormat::Json => "json",
        }
    }
}

impl From<Format> for SopsFormat {
    fn from(f: Format) -> Self {
        match f {
            Format::Dotenv => SopsFormat::Dotenv,
            Format::Json => SopsFormat::Json,
        }
    }
}

pub struct SopsProvider {
    file: PathBuf,
    format: SopsFormat,
}

impl SopsProvider {
    pub fn new(file: PathBuf, format: SopsFormat) -> Self {
        Self { file, format }
    }
}

impl Provider for SopsProvider {
    fn load(&self) -> Result<Zeroizing<String>> {
        let fmt = self.format.as_str();

        let output = std::process::Command::new("sops")
            .arg("--decrypt")
            .args(["--input-type", fmt])
            .args(["--output-type", fmt])
            .arg(&self.file)
            .output()
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => anyhow!(
                    "`sops` command not found in PATH. Install sops: https://github.com/getsops/sops"
                ),
                _ => anyhow::Error::new(e).context("failed to spawn sops"),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("sops exited with {}: {}", output.status, stderr.trim());
        }

        let s = String::from_utf8(output.stdout).context("sops output was not valid UTF-8")?;
        Ok(Zeroizing::new(s))
    }
}
