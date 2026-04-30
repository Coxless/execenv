# execenv

SOPS で暗号化された設定ファイルをメモリ上で復号し、`execve` を使ってターゲットプロセスへ環境変数を注入するツールです。平文はディスクにも子プロセスにも残りません。

→ English: [README.md](README.md)

## 動作の仕組み

1. `sops --decrypt` を呼び出し、出力を**メモリ上のみ**にキャプチャします（平文はディスクに書き出しません）。
2. 結果を dotenv または フラット JSON としてパースし、キーバリューのマップを作成します。
3. `execve` によって**自プロセスをターゲットコマンドに置き換え**ます。このとき親プロセスの環境変数は引き継がず、復号した変数のみを渡します。

execenv は子プロセスをスポーンするのではなく、自身を置き換えます。そのため、平文を保持していたプロセスはアプリが起動した瞬間に消滅します。

## インストール

**npm 経由**（Linux x64 のみ。`sops` が PATH に必要）:

```bash
npm install -g @coxless/execenv
```

**ソースからビルド**（Rust 1.93 以上が必要）:

```bash
cargo build --release
# バイナリ: target/release/execenv
```

または直接インストール:

```bash
cargo install --path .
```

> **注意:** `PR_SET_DUMPABLE` による保護（`/proc/<pid>/mem` 読み取りとコアダンプの防止）は Linux 専用です。macOS/BSD でもビルドできますが、prctl 呼び出しはスキップされます。

## クイックスタート

1. `.env` を暗号化する:

   ```bash
   age-keygen -o ~/.config/sops/age/keys.txt
   export SOPS_AGE_RECIPIENTS=$(grep 'public key' ~/.config/sops/age/keys.txt | cut -d' ' -f4)
   sops --encrypt --input-type dotenv --output-type dotenv .env > .env.enc
   ```

2. execenv 経由でアプリを起動する:

   ```bash
   execenv --provider sops --file .env.enc -- your-app arg1 arg2
   ```

## SOPS + age のセットアップ

[sops](https://github.com/getsops/sops)（v3.x）と
[age](https://github.com/FiloSottile/age) をインストールしてください。

**鍵の生成:**

```bash
age-keygen -o ~/.config/sops/age/keys.txt
```

**環境変数の設定:**

```bash
# 暗号化用
export SOPS_AGE_RECIPIENTS=$(grep 'public key' ~/.config/sops/age/keys.txt | cut -d' ' -f4)

# 復号用（ローカル開発）
export SOPS_AGE_KEY_FILE=~/.config/sops/age/keys.txt
```

**`.env` ファイルを暗号化:**

```bash
sops --encrypt --input-type dotenv --output-type dotenv .env > .env.enc
```

**CI 環境の場合:** `SOPS_AGE_KEY_FILE` にシークレットマネージャーから取得した age 秘密鍵のパスを設定し、`.env.enc` のみをリポジトリにコミットしてください。

## セキュリティモデル

| 仕組み | 効果 |
|---|---|
| `execve`（spawn ではない） | 平文を保持するプロセスがアプリ起動時に消滅します。メモリ上の秘密情報が残り続けません。 |
| `Zeroizing<T>` | 復号した文字列や envp バッファはスコープを抜けた時点でメモリ消去されます（エラーパスも含む）。 |
| `PR_SET_DUMPABLE 0` | execenv の実行中、`/proc/<pid>/mem` からの読み取りとコアダンプを防ぎます（Linux のみ）。 |

> **注意:** `PR_SET_DUMPABLE` は `execve` をまたぐとリセットされます。ターゲットプロセスは OS のデフォルト設定で起動するため、root ユーザーによる ptrace は可能です。

## AWS Secrets Manager

### 前提条件

- AWS クレデンシャルが解決できる状態であること（環境変数 / `~/.aws/credentials` / IAM ロール）。
- 対象シークレットへの `secretsmanager:GetSecretValue` 権限が必要。

### 使い方

```bash
# JSON シークレット（デフォルト形式）
execenv --provider aws-secrets-manager --secret-id prod/myapp/env -- node server.js

# ARN で指定
execenv --provider aws-secrets-manager \
  --secret-id arn:aws:secretsmanager:ap-northeast-1:123456789012:secret:prod/myapp/env-XXXXXX \
  -- ./bin/server

# リージョンを明示
execenv --provider aws-secrets-manager \
  --secret-id prod/myapp/env \
  --aws-region ap-northeast-1 \
  -- ./bin/server

# dotenv 形式のシークレット
execenv --provider aws-secrets-manager \
  --secret-id prod/myapp/dotenv \
  --format dotenv \
  -- ./bin/server
```

シークレット値はデフォルトで **フラット JSON**（`--format json`）として解釈されます。
dotenv 形式のシークレット文字列を使う場合は `--format dotenv` を指定してください。

## 制限事項

- **PATH は引き継がれません。** execenv は暗号化ファイルの変数のみを注入します。ターゲットコマンドが PATH を必要とする場合は、`.env` に `PATH=/usr/bin:/bin:…` を含めてください。
- **フラットな文字列マップのみ対応。** ネストした JSON オブジェクトや配列はサポートしていません。すべての値は文字列である必要があります。
- **`PR_SET_DUMPABLE` は Linux 専用。** macOS/BSD ではこの保護は適用されません。

## ガイド

- [Next.js プロジェクトでの手動テスト手順](docs/manual_test_nextjs.ja.md)

## 開発

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test                          # ユニットテスト + 統合テスト（sops がなければ E2E はスキップ）
EXECENV_REQUIRE_E2E=1 cargo test    # sops または age-keygen がない場合はテスト失敗
cargo build --release
```

## ライセンス

MIT
