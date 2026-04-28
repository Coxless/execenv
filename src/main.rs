mod cli;
mod executor;
mod parser;
mod provider;

use clap::Parser;
use cli::Cli;
use provider::{Provider, SopsProvider};

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
    let p = SopsProvider::new(args.file, args.format);
    let secret = p.load()?;
    let map = parser::parse(&secret, args.format)?;
    drop(secret);
    match executor::exec(args.command, map, !args.clean_env)? {}
}
