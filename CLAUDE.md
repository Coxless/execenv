# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`execenv` is a Rust CLI tool that decrypts encrypted config files (e.g. `.env.enc`) in memory and injects the result as environment variables into a target process via `execve`, replacing itself in the process table so secrets never persist in memory beyond the hand-off.

## Commands

```bash
# Build
cargo build
cargo build --release        # single binary output

# Run
cargo run -- --provider sops --file .env.enc -- <cmd> [args...]

# Test
cargo test                   # all tests
cargo test <test_name>       # single test
cargo test --test integration_test  # integration tests only

# Lint / format
cargo clippy
cargo fmt
```

Tool versions are managed with `mise` (see `.mise.toml`): Rust 1.93.0.

## Architecture

```
execenv
  ├─ Provider trait  →  load() -> Zeroizing<String>
  │     └─ SopsProvider: shells out to `sops --decrypt`, captures stdout
  │
  ├─ Parser
  │     ├─ parse_dotenv()  →  HashMap<String, String>  (via dotenvy)
  │     └─ parse_json()    →  HashMap<String, String>  (via serde_json, flat only)
  │
  └─ Executor
        └─ nix::unistd::execvpe — replaces the current process (no child spawn)
```

Key crates: `clap` (args), `anyhow` (errors), `dotenvy`, `serde_json`, `nix` (execve / prctl), `zeroize`.

## Security Invariants

- **Never `spawn` a child process** to run the target command — use `execvpe` so the parent process (which holds secrets in memory) is replaced, not duplicated.
- Secrets must be wrapped in `Zeroizing<T>` so they are wiped when they go out of scope.
- Call `prctl(PR_SET_DUMPABLE, 0)` early to prevent `/proc/<pid>/mem` reads and core dumps.
- Secrets must not appear in stdout, stderr, or logs at any point.
- Variables should be constructed as late as possible (immediately before `execve`) to minimise memory dwell time.

## Implementation Status

The project is pre-code (planning phase). Implementation follows these steps:

1. Project init (`Cargo.toml`, `src/main.rs`)
2. CLI arg parsing with `clap`
3. SOPS Provider
4. dotenv / JSON parsers
5. `execve` Executor — **MVP complete at this step**
6. Security hardening (`zeroize`, `PR_SET_DUMPABLE`)
7. Integration tests + `README.md`
