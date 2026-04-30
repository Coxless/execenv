mod cli;
mod executor;
mod parser;
mod provider;

use clap::Parser;
use cli::{Cli, Format, ProviderKind};
use provider::{AwsSecretsManagerProvider, Provider, SopsProvider};

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

fn main() -> anyhow::Result<()> {
    harden_process();
    let args = Cli::parse();

    let (secret, format) = match args.provider {
        ProviderKind::Sops => {
            let file = args.file.expect("clap ensures --file is set for sops");
            let fmt = args.format.unwrap_or(Format::Dotenv);
            let secret = SopsProvider::new(file, fmt).load()?;
            (secret, fmt)
        }
        ProviderKind::AwsSecretsManager => {
            let secret_id = args
                .secret_id
                .expect("clap ensures --secret-id is set for aws-secrets-manager");
            let fmt = args.format.unwrap_or(Format::Json);
            let secret = AwsSecretsManagerProvider::new(secret_id, args.aws_region).load()?;
            (secret, fmt)
        }
    };

    let map = parser::parse(&secret, format)?;
    drop(secret);
    match executor::exec(args.command, map, !args.clean_env)? {}
}
