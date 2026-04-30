use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use zeroize::{Zeroize, Zeroizing};

use crate::cli::Format;

pub trait Provider {
    fn load(&self) -> Result<Zeroizing<String>>;
}

// ---------------------------------------------------------------------------
// Sops
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// AWS Secrets Manager — shared async helpers
// ---------------------------------------------------------------------------

async fn build_aws_config(region: Option<&str>) -> aws_config::SdkConfig {
    let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
    if let Some(r) = region {
        loader =
            loader.region(aws_sdk_secretsmanager::config::Region::new(r.to_owned()));
    }
    loader.load().await
}

async fn fetch_secret_value(
    client: &aws_sdk_secretsmanager::Client,
    secret_id: &str,
) -> Result<Zeroizing<String>> {
    let resp = client
        .get_secret_value()
        .secret_id(secret_id)
        .send()
        .await
        .map_err(|e| anyhow!("failed to get secret `{}`: {}", secret_id, e))?;

    // Copy out before dropping the SDK response so plaintext lives only in
    // Zeroizing-managed memory.
    let secret_str: Option<String> = resp.secret_string().map(str::to_owned);
    let secret_bin: Option<Vec<u8>> = if secret_str.is_none() {
        resp.secret_binary().map(|b| b.as_ref().to_vec())
    } else {
        None
    };
    drop(resp);

    match (secret_str, secret_bin) {
        (Some(s), _) => Ok(Zeroizing::new(s)),
        (None, Some(bytes)) => {
            let s = String::from_utf8(bytes).map_err(|_| {
                anyhow!("secret `{}` binary is not valid UTF-8", secret_id)
            })?;
            Ok(Zeroizing::new(s))
        }
        (None, None) => anyhow::bail!("secret `{}` has no value", secret_id),
    }
}

// ---------------------------------------------------------------------------
// AWS Secrets Manager — public API
// ---------------------------------------------------------------------------

/// Fetch one or more secrets from AWS Secrets Manager, reusing a single
/// client connection. Returns raw plaintext in the same order as `secret_ids`.
pub fn load_secrets(
    secret_ids: &[String],
    region: Option<String>,
) -> Result<Vec<Zeroizing<String>>> {
    if secret_ids.is_empty() {
        return Ok(Vec::new());
    }

    let rt = tokio::runtime::Runtime::new()
        .context("failed to create async runtime")?;

    rt.block_on(async {
        let config = build_aws_config(region.as_deref()).await;
        let client = aws_sdk_secretsmanager::Client::new(&config);

        let mut results = Vec::with_capacity(secret_ids.len());
        for id in secret_ids {
            results.push(fetch_secret_value(&client, id).await?);
        }
        Ok(results)
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn localstack_env() {
        std::env::set_var("AWS_ENDPOINT_URL", "http://localhost:4566");
        std::env::set_var("AWS_DEFAULT_REGION", "us-east-1");
        std::env::set_var("AWS_ACCESS_KEY_ID", "test");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");
    }

    #[test]
    fn load_secrets_empty_ids_returns_empty() {
        let result = load_secrets(&[], None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    #[ignore = "requires localstack"]
    fn load_secrets_missing_secret_errors() {
        localstack_env();
        let ids = vec!["nonexistent/secret".to_string()];
        let result = load_secrets(&ids, None);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("nonexistent/secret"));
    }

    #[test]
    #[ignore = "requires localstack"]
    fn load_secrets_json_round_trips() {
        use crate::parser::parse_json;
        localstack_env();

        let ids = vec!["test/execenv".to_string()];
        let mut secrets = load_secrets(&ids, None).expect("load should succeed");
        let map = parse_json(&secrets.remove(0)).expect("parse should succeed");
        assert_eq!(map["HELLO"].as_str(), "world");
    }

    #[test]
    #[ignore = "requires localstack"]
    fn load_secrets_dotenv_round_trips() {
        use crate::parser::parse_dotenv;
        localstack_env();

        let ids = vec!["test/execenv-dotenv".to_string()];
        let mut secrets = load_secrets(&ids, None).expect("load should succeed");
        let map = parse_dotenv(&secrets.remove(0)).expect("parse should succeed");
        assert_eq!(map["HELLO"].as_str(), "world");
    }

    #[test]
    #[ignore = "requires localstack"]
    fn load_secrets_multiple_ids() {
        localstack_env();
        let ids = vec![
            "test/execenv".to_string(),
            "test/execenv2".to_string(),
        ];
        let results = load_secrets(&ids, None).expect("load should succeed");
        assert_eq!(results.len(), 2);
    }
}
