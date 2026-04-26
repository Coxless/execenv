mod cli;
mod executor;
mod parser;
mod provider;

use clap::Parser;
use cli::Cli;
use provider::{Provider, SopsProvider};

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let p = SopsProvider::new(args.file, args.format.into());
    let secret = p.load()?;
    let map = parser::parse(&secret, args.format)?;
    drop(secret);
    match executor::exec(args.command, map)? {}
}
