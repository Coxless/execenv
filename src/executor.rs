use std::collections::HashMap;
use std::convert::Infallible;
use std::ffi::CString;

use anyhow::{Context, Result};
use nix::unistd::execvpe;

pub fn exec(command: Vec<String>, env: HashMap<String, String>) -> Result<Infallible> {
    debug_assert!(
        !command.is_empty(),
        "cli layer must enforce non-empty command"
    );

    let program = command[0].clone();

    let argv: Vec<CString> = command
        .into_iter()
        .enumerate()
        .map(|(i, s)| {
            CString::new(s).with_context(|| format!("argv[{i}] contains an interior NUL byte"))
        })
        .collect::<Result<_>>()?;

    let envp: Vec<CString> = env
        .into_iter()
        .map(|(k, v)| {
            CString::new(format!("{k}={v}"))
                .with_context(|| format!("env var `{k}` contains an interior NUL byte"))
        })
        .collect::<Result<_>>()?;

    execvpe(&argv[0], &argv, &envp).with_context(|| format!("failed to exec `{program}`"))
}
