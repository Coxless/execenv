use anyhow::{anyhow, Context, Result};
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

pub struct AwsSecretsManagerProvider {
    secret_id: String,
    region: Option<String>,
}

impl AwsSecretsManagerProvider {
    pub fn new(secret_id: String, region: Option<String>) -> Self {
        Self { secret_id, region }
    }
}

impl Provider for AwsSecretsManagerProvider {
    fn load(&self) -> Result<Zeroizing<String>> {
        let rt = tokio::runtime::Runtime::new()
            .context("failed to create async runtime")?;

        rt.block_on(async {
            let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
            if let Some(ref region_str) = self.region {
                loader = loader.region(
                    aws_sdk_secretsmanager::config::Region::new(region_str.clone()),
                );
            }
            let config = loader.load().await;
            let client = aws_sdk_secretsmanager::Client::new(&config);

            let resp = client
                .get_secret_value()
                .secret_id(&self.secret_id)
                .send()
                .await
                .map_err(|e| anyhow!("failed to get secret `{}`: {}", self.secret_id, e))?;

            // Copy values out before dropping the SDK response so we hold
            // the plaintext only in Zeroizing-managed memory.
            let secret_str: Option<String> = resp.secret_string().map(str::to_owned);
            let secret_bin: Option<Vec<u8>> = if secret_str.is_none() {
                resp.secret_binary().map(|b| b.as_ref().to_vec())
            } else {
                None
            };
            drop(resp);

            let secret = match (secret_str, secret_bin) {
                (Some(s), _) => Zeroizing::new(s),
                (None, Some(bytes)) => {
                    let s = String::from_utf8(bytes)
                        .map_err(|_| anyhow!("secret binary is not valid UTF-8"))?;
                    Zeroizing::new(s)
                }
                (None, None) => {
                    anyhow::bail!("secret `{}` has no value", self.secret_id)
                }
            };

            Ok(secret)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aws_provider_stores_secret_id_and_region() {
        let p = AwsSecretsManagerProvider::new("my-secret".to_string(), None);
        assert_eq!(p.secret_id, "my-secret");
        assert!(p.region.is_none());

        let p = AwsSecretsManagerProvider::new(
            "my-secret".to_string(),
            Some("ap-northeast-1".to_string()),
        );
        assert_eq!(p.region.as_deref(), Some("ap-northeast-1"));
    }

    #[test]
    #[ignore = "requires localstack"]
    fn aws_provider_returns_err_for_missing_secret() {
        std::env::set_var("AWS_ENDPOINT_URL", "http://localhost:4566");
        std::env::set_var("AWS_DEFAULT_REGION", "us-east-1");
        std::env::set_var("AWS_ACCESS_KEY_ID", "test");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");

        let p = AwsSecretsManagerProvider::new("nonexistent/secret".to_string(), None);
        let result = p.load();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        // Error message must not contain secret value
        assert!(msg.contains("nonexistent/secret"));
    }

    #[test]
    #[ignore = "requires localstack"]
    fn aws_provider_json_secret_round_trips() {
        use crate::parser::parse_json;

        std::env::set_var("AWS_ENDPOINT_URL", "http://localhost:4566");
        std::env::set_var("AWS_DEFAULT_REGION", "us-east-1");
        std::env::set_var("AWS_ACCESS_KEY_ID", "test");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");

        let p = AwsSecretsManagerProvider::new("test/execenv".to_string(), None);
        let secret = p.load().expect("load should succeed");
        let map = parse_json(&secret).expect("parse should succeed");
        assert_eq!(map["HELLO"].as_str(), "world");
    }

    #[test]
    #[ignore = "requires localstack"]
    fn aws_provider_dotenv_secret_round_trips() {
        use crate::parser::parse_dotenv;

        std::env::set_var("AWS_ENDPOINT_URL", "http://localhost:4566");
        std::env::set_var("AWS_DEFAULT_REGION", "us-east-1");
        std::env::set_var("AWS_ACCESS_KEY_ID", "test");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");

        let p = AwsSecretsManagerProvider::new("test/execenv-dotenv".to_string(), None);
        let secret = p.load().expect("load should succeed");
        let map = parse_dotenv(&secret).expect("parse should succeed");
        assert_eq!(map["HELLO"].as_str(), "world");
    }
}
