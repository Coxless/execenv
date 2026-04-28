use std::collections::HashMap;
use std::convert::Infallible;
use std::ffi::{CStr, CString};

use anyhow::{anyhow, Context, Result};
use nix::unistd::execvpe;
use zeroize::Zeroizing;

pub fn exec(
    command: Vec<String>,
    env: HashMap<String, Zeroizing<String>>,
    inherit_parent_env: bool,
) -> Result<Infallible> {
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

    // Merge: start with parent env (if inherited), then overlay decrypted vars.
    // Decrypted values always win on key conflicts.
    let merged: HashMap<String, Zeroizing<String>> = if inherit_parent_env {
        let mut base: HashMap<String, Zeroizing<String>> = std::env::vars_os()
            .filter_map(|(k, v)| {
                let k = k.into_string().ok()?;
                let v = v.into_string().ok()?;
                Some((k, Zeroizing::new(v)))
            })
            .collect();
        for (k, v) in env {
            base.insert(k, v);
        }
        base
    } else {
        env
    };

    // Build zeroized KEY=VALUE\0 byte buffers so plaintext is wiped on drop
    // if execvpe returns an error.
    let mut envp_bufs: Vec<Zeroizing<Vec<u8>>> = Vec::with_capacity(merged.len());
    for (k, v) in &merged {
        if k.contains('=') {
            return Err(anyhow!("env var key `{k}` contains '='"));
        }
        if k.as_bytes().contains(&0u8) || v.as_bytes().contains(&0u8) {
            return Err(anyhow!("env var `{k}` contains an interior NUL byte"));
        }
        let mut buf: Vec<u8> = Vec::with_capacity(k.len() + v.len() + 2);
        buf.extend_from_slice(k.as_bytes());
        buf.push(b'=');
        buf.extend_from_slice(v.as_bytes());
        buf.push(0);
        envp_bufs.push(Zeroizing::new(buf));
    }
    drop(merged);

    let envp: Vec<&CStr> = envp_bufs
        .iter()
        .map(|buf| {
            // Safety invariant: we validated no interior NULs and appended exactly one.
            CStr::from_bytes_with_nul(buf).expect("buf has exactly one terminal NUL")
        })
        .collect();

    execvpe(&argv[0], &argv, &envp).with_context(|| format!("failed to exec `{program}`"))
}
