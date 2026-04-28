# execenv

Decrypt a SOPS-encrypted config file in memory and hand off to your target
command via `execve`, so secrets never persist in a child process or on disk.

→ 日本語版: [README.ja.md](README.ja.md)

## How it works

1. Calls `sops --decrypt` and captures the output **in memory** — no plaintext ever touches disk.
2. Parses the result as dotenv or flat JSON into key/value pairs.
3. **Replaces itself** with your target command via `execve`, injecting those variables and discarding the parent environment.

Because execenv *replaces* itself (not spawns a child), the process that held
the plaintext is gone the moment your app starts.

## Install

**Via npm** (Linux x64; requires `sops` on PATH):

```bash
npm install -g @coxless/execenv
```

**From source** (requires Rust 1.93+):

```bash
cargo build --release
# binary: target/release/execenv
```

Or:

```bash
cargo install --path .
```

> **Note:** `PR_SET_DUMPABLE` hardening (prevents `/proc/<pid>/mem` reads and
> core dumps) is Linux-only. The binary builds on macOS/BSD but the prctl call
> is silently skipped.

## Quick start

1. Encrypt your `.env`:

   ```bash
   age-keygen -o ~/.config/sops/age/keys.txt
   export SOPS_AGE_RECIPIENTS=$(grep 'public key' ~/.config/sops/age/keys.txt | cut -d' ' -f4)
   sops --encrypt --input-type dotenv --output-type dotenv .env > .env.enc
   ```

2. Run your app through execenv:

   ```bash
   execenv --provider sops --file .env.enc -- your-app arg1 arg2
   ```

## SOPS setup with age

Install [sops](https://github.com/getsops/sops) (v3.x) and
[age](https://github.com/FiloSottile/age).

**Generate a key:**

```bash
age-keygen -o ~/.config/sops/age/keys.txt
```

**Set environment variables:**

```bash
# for encryption
export SOPS_AGE_RECIPIENTS=$(grep 'public key' ~/.config/sops/age/keys.txt | cut -d' ' -f4)

# for decryption (local dev)
export SOPS_AGE_KEY_FILE=~/.config/sops/age/keys.txt
```

**Encrypt a `.env` file:**

```bash
sops --encrypt --input-type dotenv --output-type dotenv .env > .env.enc
```

**In CI:** set `SOPS_AGE_KEY_FILE` to your age private key path (from a
secret manager) and commit only the `.env.enc`.

## Security model

| Mechanism | Effect |
|---|---|
| `execve` (not `spawn`) | The process holding plaintext is replaced, not duplicated. Secrets vanish from memory when your app starts. |
| `Zeroizing<T>` | Decrypted strings and envp byte buffers are wiped when they go out of scope — including on error paths. |
| `PR_SET_DUMPABLE 0` | Prevents `/proc/<pid>/mem` reads and core dumps while execenv is running (Linux only). |

> **Caveat:** `PR_SET_DUMPABLE` is reset across the `execve` boundary. The
> target process starts with the OS default and is ptrace-able by root.

## Limitations

- **PATH is not propagated.** execenv injects only the variables from the
  encrypted file. Include `PATH=/usr/bin:/bin:…` in your `.env` if needed.
- **Flat maps only.** Nested JSON objects and arrays are not supported; all
  values must be strings.
- **sops only (MVP).** Additional providers (Vault, AWS Secrets Manager, etc.)
  are not yet implemented.
- **Linux-only `PR_SET_DUMPABLE`.** The prctl call is silently skipped on
  macOS/BSD.

## Guides

- [Manual test with Next.js (ja)](docs/manual_test_nextjs.ja.md)

## Development

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test                          # unit + integration (skips E2E if sops not on PATH)
EXECENV_REQUIRE_E2E=1 cargo test    # fail if sops or age-keygen are missing
cargo build --release
```

## License

MIT
