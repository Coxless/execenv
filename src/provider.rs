use anyhow::{anyhow, Result};
use std::path::PathBuf;
use zeroize::{Zeroize, Zeroizing};

use crate::cli::Format;

pub trait Provider {
    fn load(&self) -> Result<Zeroizing<String>>;
}

pub struct SopsProvider {
    file: PathBuf,
    format: Format,
}

impl SopsProvider {
    pub fn new(file: PathBuf, format: Format) -> Self {
        Self { file, format }
    }
}

impl Provider for SopsProvider {
    fn load(&self) -> Result<Zeroizing<String>> {
        let fmt = match self.format {
            Format::Dotenv => "dotenv",
            Format::Json => "json",
        };

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

        let stderr = Zeroizing::new(output.stderr);
        if !output.status.success() {
            let stderr_msg = String::from_utf8_lossy(&stderr);
            anyhow::bail!("sops exited with {}: {}", output.status, stderr_msg.trim());
        }

        let s = match String::from_utf8(output.stdout) {
            Ok(s) => s,
            Err(e) => {
                let mut bytes = e.into_bytes();
                bytes.zeroize();
                anyhow::bail!("sops output was not valid UTF-8");
            }
        };
        Ok(Zeroizing::new(s))
    }
}
