mod cli;
mod executor;
mod parser;
mod provider;

use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;
use clap::Parser;
use cli::{Cli, Format, ProviderKind};
use provider::{Provider, SopsProvider};
use zeroize::Zeroizing;

// PR_SET_DUMPABLE is reset on execve, so this only protects execenv's own
// runtime window — the target child's dumpable bit is governed by suid bits
// and /proc/sys/fs/suid_dumpable.
#[cfg(target_os = "linux")]
fn harden_process() {
    if let Err(e) = nix::sys::prctl::set_dumpable(false) {
        eprintln!("execenv: warning: PR_SET_DUMPABLE failed: {e}");
    }
}
#[cfg(not(target_os = "linux"))]
fn harden_process() {}

/// Parse a secrets file into a list of secret IDs.
/// Each non-blank line (after stripping `#` comments) is one secret ID.
fn parse_secrets_file(path: &Path) -> anyhow::Result<Vec<String>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read secrets file `{}`", path.display()))?;
    let ids = content
        .lines()
        .filter_map(|line| {
            let s = line.find('#').map_or(line, |i| &line[..i]).trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_owned())
            }
        })
        .collect();
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn parse_secrets_file_strips_comments_and_blanks() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "# full-line comment").unwrap();
        writeln!(f, "secret/one").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "secret/two  # inline comment").unwrap();
        writeln!(f, "  secret/three  ").unwrap();

        let ids = parse_secrets_file(f.path()).unwrap();
        assert_eq!(ids, vec!["secret/one", "secret/two", "secret/three"]);
    }

    #[test]
    fn parse_secrets_file_missing_path_errors() {
        let result = parse_secrets_file(Path::new("/nonexistent/path/secrets.txt"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("failed to read"));
    }

    #[test]
    fn parse_secrets_file_empty_file_returns_empty() {
        let f = NamedTempFile::new().unwrap();
        let ids = parse_secrets_file(f.path()).unwrap();
        assert!(ids.is_empty());
    }
}

fn main() -> anyhow::Result<()> {
    harden_process();
    let args = Cli::parse();

    match args.provider {
        ProviderKind::Sops => {
            let file = args.file.expect("clap ensures --file is set for sops");
            let fmt = args.format.unwrap_or(Format::Dotenv);
            let secret = SopsProvider::new(file, fmt).load()?;
            let map = parser::parse(&secret, fmt)?;
            drop(secret);
            match executor::exec(args.command, map, !args.clean_env)? {}
        }

        ProviderKind::AwsSecretsManager => {
            let fmt = args.format.unwrap_or(Format::Json);

            // Collect secret IDs from --secret-id and/or --secrets-file (additive).
            let mut secret_ids: Vec<String> = Vec::new();
            if let Some(id) = args.secret_id {
                secret_ids.push(id);
            }
            if let Some(path) = args.secrets_file {
                secret_ids.extend(parse_secrets_file(&path)?);
            }
            if secret_ids.is_empty() {
                anyhow::bail!(
                    "--provider aws-secrets-manager requires --secret-id or --secrets-file"
                );
            }

            // Fetch all secrets with a single client connection.
            let secrets = provider::load_secrets(&secret_ids, args.aws_region)?;

            // Parse each secret and merge into one environment map.
            // Later secrets override earlier ones on key conflicts.
            let mut merged: HashMap<String, Zeroizing<String>> = HashMap::new();
            for (secret, id) in secrets.iter().zip(&secret_ids) {
                let map = parser::parse(secret, fmt)
                    .with_context(|| format!("failed to parse secret `{id}`"))?;
                merged.extend(map);
            }
            drop(secrets);

            match executor::exec(args.command, merged, !args.clean_env)? {}
        }
    }
}
