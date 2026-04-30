# AWS Secrets Manager 対応計画

## 概要

`execenv` に AWS Secrets Manager プロバイダを追加する。
既存の `Provider` トレイトを実装した `AwsSecretsManagerProvider` を新設し、
CLI で `--provider aws-secrets-manager --secret-id <ARN_or_name>` と指定すると、
シークレットを取得して `execve` に渡す。

セキュリティ不変条件（`Zeroizing`・`execvpe`・`PR_SET_DUMPABLE`）は既存実装と同等に維持する。

---

## ステップ一覧

| # | タイトル | 説明 |
|---|---------|------|
| 1 | 依存クレートの追加 | AWS SDK (`aws-sdk-secretsmanager`) と非同期ランタイム (`tokio`) を `Cargo.toml` に追加する |
| 2 | `AwsSecretsManagerProvider` 実装 | `src/provider.rs` に新プロバイダを実装し、`Provider` トレイトを満たす |
| 3 | CLI 拡張 | `--provider aws-secrets-manager` と `--secret-id` 引数を `src/cli.rs` に追加する |
| 4 | `main.rs` の接続 | `main.rs` でプロバイダを切り替え、非同期ブロック内でシークレットを取得する |
| 5 | パーサ対応（JSON シークレット） | AWS Secrets Manager の JSON シークレットをフラット展開する既存パーサを確認・整合する |
| 6 | 単体テスト | `AwsSecretsManagerProvider` のモックテストを追加する |
| 7 | 統合テスト | `LocalStack` を使った E2E テストを追加する |
| 8 | ドキュメント更新 | `README.md` / `README.ja.md` に使い方を追記する |

---

## ステップ詳細

### ステップ 1 — 依存クレートの追加

**変更ファイル:** `Cargo.toml`

追加するクレート:

```toml
[dependencies]
aws-config           = { version = "1", features = ["behavior-version-latest"] }
aws-sdk-secretsmanager = "1"
tokio                = { version = "1", features = ["rt-multi-thread", "macros"] }
```

`tokio` は非同期ランタイムとして必要。`aws-config` はリージョン・クレデンシャル解決を担う。

**確認ポイント:**
- `rust-version = "1.93.0"` と各クレートの MSRV が衝突しないこと。
- `cargo build` がエラーなく通ること。

---

### ステップ 2 — `AwsSecretsManagerProvider` 実装

**変更ファイル:** `src/provider.rs`

```rust
pub struct AwsSecretsManagerProvider {
    secret_id: String,
    region: Option<String>,
}

impl AwsSecretsManagerProvider {
    pub fn new(secret_id: String, region: Option<String>) -> Self {
        Self { secret_id, region }
    }
}

impl Provider for AwsSecretsManagerProvider {
    fn load(&self) -> Result<Zeroizing<String>> {
        // tokio::runtime::Runtime::new()?.block_on(async { ... })
        // aws_config::from_env() でクレデンシャル解決
        // client.get_secret_value().secret_id(&self.secret_id).send().await
        // SecretString / SecretBinary を Zeroizing<String> に変換して返す
        todo!()
    }
}
```

**セキュリティ要件:**
- `SecretString` / `SecretBinary` は取得直後に `Zeroizing<String>` へ移す。
- AWS SDK 内部の `String` は `drop` 直後に上書きされないが、`Zeroizing` ラッパが execenv 管理領域を保護すれば十分とする（SDK 自体はセキュリティバウンダリ外）。
- エラー時にシークレット値がログに出ないよう `anyhow::bail!` のメッセージを設計する。

---

### ステップ 3 — CLI 拡張

**変更ファイル:** `src/cli.rs`

```rust
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ProviderKind {
    Sops,
    AwsSecretsManager,
}

// 追加フィールド
/// AWS Secrets Manager シークレット ID（ARN または名前）
#[arg(long, required_if_eq("provider", "aws-secrets-manager"))]
pub secret_id: Option<String>,

/// AWS リージョン（省略時は SDK のデフォルト解決）
#[arg(long)]
pub aws_region: Option<String>,
```

`--file` は `sops` プロバイダでのみ必須にし、`aws-secrets-manager` 選択時は不要とする（`required_if_eq` で制御）。

---

### ステップ 4 — `main.rs` の接続

**変更ファイル:** `src/main.rs`

```rust
let secret = match args.provider {
    ProviderKind::Sops => {
        SopsProvider::new(args.file.unwrap(), args.format).load()?
    }
    ProviderKind::AwsSecretsManager => {
        AwsSecretsManagerProvider::new(
            args.secret_id.unwrap(),
            args.aws_region,
        ).load()?
    }
};
```

AWS Secrets Manager から取得した値はデフォルトで JSON とする（`Format::Json`）。
`--format dotenv` 指定も受け付け、テキスト形式のシークレットを dotenv としてパースできるようにする。

---

### ステップ 5 — パーサ対応（JSON シークレット）

**変更ファイル:** `src/parser.rs`（必要に応じて）

AWS Secrets Manager は以下の形式でシークレットを返す:

- **JSON オブジェクト** — `{"DB_HOST":"…","DB_PASS":"…"}` → 既存 `parse_json()` がそのまま使える
- **プレーンテキスト** — `dotenv` 形式か単一値。`parse_dotenv()` で対応。

既存実装の確認のみで追加変更は不要な可能性が高い。確認して不足があれば修正する。

---

### ステップ 6 — 単体テスト

**変更ファイル:** `src/provider.rs`（`#[cfg(test)]` モジュール）または `tests/` 以下の新ファイル

モック戦略:
- `Provider` トレイトを受け取る関数群は既存のままテスト可能。
- `AwsSecretsManagerProvider` のネットワーク呼び出し部分は、`trait` を介して差し替えるか、環境変数 `AWS_ENDPOINT_URL` で LocalStack を指す。
- 単体テストでは `mockall` クレートを使い、SDK クライアントをモックすることを検討する。

テストケース:
1. 正常系: JSON シークレットが `HashMap` に展開される
2. 正常系: dotenv 形式シークレットが `HashMap` に展開される
3. 異常系: シークレット未存在 → `anyhow::Error` が返る
4. 異常系: クレデンシャル未設定 → `anyhow::Error` が返る

---

### ステップ 7 — 統合テスト（LocalStack）

**変更ファイル:** `tests/integration_test.rs` または `tests/aws_integration_test.rs`（新規）

LocalStack を使った E2E テスト:

```bash
# LocalStack 起動（CI では docker-compose で管理）
docker run -d -p 4566:4566 localstack/localstack

# シークレット作成
aws --endpoint-url=http://localhost:4566 secretsmanager create-secret \
  --name test/execenv \
  --secret-string '{"HELLO":"world"}'
```

テストコード:

```rust
#[test]
#[ignore = "requires localstack"]
fn aws_provider_e2e() {
    // AWS_ENDPOINT_URL=http://localhost:4566 を設定
    // AwsSecretsManagerProvider::new("test/execenv", None).load()
    // parse_json() で展開
    // env["HELLO"] == "world" を確認
}
```

CI (`GitHub Actions`) では `services:` に `localstack` コンテナを追加し、`--ignored` テストを実行するジョブを別途設ける。

---

### ステップ 8 — ドキュメント更新

**変更ファイル:** `README.md`, `README.ja.md`

追記内容:

```
## AWS Secrets Manager

### 前提条件

- AWS クレデンシャルが解決できる状態（環境変数 / ~/.aws/credentials / IAM ロール）
- シークレットへの `secretsmanager:GetSecretValue` 権限

### 使い方

# JSON シークレット
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
```

---

## 実装順序の根拠

1. **ステップ 1 → 2 → 3 → 4** は依存関係順（ビルドが通るようになってから CLI・接続を実装）。
2. **ステップ 5** はステップ 2 完了後に既存パーサが十分か判明するため、後回し。
3. **ステップ 6・7** はコア実装（1〜4）完了後に実施。ステップ 7 はインフラ（LocalStack）が必要なため独立。
4. **ステップ 8** は最後。

---

## セキュリティチェックリスト

- [ ] `SecretString` の値を `Zeroizing<String>` に移した直後に SDK の戻り値を `drop` する
- [ ] エラーメッセージにシークレット値が含まれないことを確認する
- [ ] `PR_SET_DUMPABLE` は `main.rs` 冒頭で呼ばれているため変更不要
- [ ] `execvpe` 呼び出しまでシークレットが `Zeroizing<HashMap<...>>` に格納されていることを確認する
- [ ] `--secret-id` / `--aws-region` の値がログや `--help` 出力に残らないか確認する
