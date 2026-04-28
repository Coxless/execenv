# execenv

Decrypt a SOPS-encrypted config file in memory and hand off to your command via `execve`, so secrets never persist in a child process or on disk.

## Install

```bash
npm install -g @coxless/execenv
```

Requires:
- **Linux x64** (only supported platform for now)
- [`sops`](https://github.com/getsops/sops) on your `PATH`

## Usage

```bash
execenv --provider sops --file .env.enc -- your-app arg1 arg2
```

## Security model

| Mechanism | Effect |
|---|---|
| `execve` (not `spawn`) | execenv replaces itself with your app. Secrets vanish from memory when your app starts. |
| `Zeroizing<T>` | Decrypted strings are wiped when they go out of scope. |
| `PR_SET_DUMPABLE 0` | Prevents `/proc/<pid>/mem` reads and core dumps while execenv is running (Linux only). |

## Full documentation

See the [execenv GitHub repository](https://github.com/Coxless/execenv) for SOPS setup, security details, and development guides.
