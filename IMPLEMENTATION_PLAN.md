# execenv 実装計画書

## 前提

- 言語: Rust
- MVP スコープ: SOPS Provider のみ
- 各 Step は「プラン確認 → 実装」の順で進める

---

## Step 1: プロジェクト初期化

**目標**: ビルド・テストが通る骨格を作る

### 成果物
- `Cargo.toml`（依存クレート定義）
- `src/main.rs`（エントリポイント、引数なしで起動できる）
- CI 用 `.github/workflows/ci.yml`（任意）

### 依存クレート（確定分）
| クレート | 用途 |
|---|---|
| `clap` | CLI 引数パース |
| `anyhow` | エラーハンドリング |
| `dotenvy` | dotenv パース |
| `serde_json` | JSON パース |
| `nix` | execve / PR_SET_DUMPABLE |
| `zeroize` | メモリ消去 |

---

## Step 2: CLI 引数パース

**目標**: `execenv --provider sops --file .env.enc -- next dev` を受け取れる

### 実装内容
- `clap` derive マクロで構造体定義
- `--provider <sops>` (現状 sops 固定)
- `--file <path>` (暗号化ファイルパス)
- `--format <dotenv|json>` (デフォルト: dotenv)
- `-- <cmd> [args...]` (サブコマンド)
- `--help` / `--version` 自動生成

### 受け入れ基準
- `cargo run -- --help` が正常出力
- 必須引数欠落時に明示エラー

---

## Step 3: SOPS Provider

**目標**: 暗号化ファイルを復号してメモリ上の文字列として返す

### 実装内容
- `Provider` トレイト定義

```rust
trait Provider {
    fn load(&self) -> anyhow::Result<Zeroizing<String>>;
}
```

- `SopsProvider` 実装
  - `std::process::Command` で `sops --decrypt` を呼び出し
  - stdout をキャプチャして返す
  - 失敗時は sops の stderr を含めたエラーを返す
- `Zeroizing<String>` でスコープ外に出たら自動消去

### 受け入れ基準
- 実際の `.env.enc` を復号できる
- `sops` が PATH にない場合の明示エラー

---

## Step 4: パーサー

**目標**: 復号文字列を `HashMap<String, String>` に変換する

### 実装内容
- `parse_dotenv(input: &str) -> HashMap<String, String>`
  - `dotenvy` の `from_read` を利用
  - コメント・空行を無視
- `parse_json(input: &str) -> HashMap<String, String>`
  - `serde_json` でフラットな `{"KEY": "VALUE"}` をパース
  - ネスト・配列は現時点でエラー

### 受け入れ基準
- 標準的な `.env` 形式をすべてパースできる
- 単体テストで各形式をカバー

---

## Step 5: Executor（execve）

**目標**: 環境変数を注入したうえで、子プロセスではなく自プロセスを置き換えて実行する

### 実装内容
- `nix::unistd::execvpe` でプロセス置き換え
- 現在の環境変数を引き継がず、パースした変数のみ渡す（or マージオプション検討）
- execve 失敗時のエラーハンドリング（コマンドが存在しない等）

### セキュリティポイント
- `execve` 後は親プロセスが消滅するため、メモリは OS が回収
- 子プロセスとして `spawn` しないこと（変数がメモリに残り続ける）

### 受け入れ基準
- `execenv ... -- env` を実行すると注入した変数のみが表示される
- `execenv ... -- sh -c 'echo $SECRET'` が正しい値を出力する

---

## Step 6: セキュリティ強化

**目標**: メモリ・プロセス観測に対する防御を追加する

### 実装内容
1. **zeroize**: `Zeroizing<String>` で復号文字列・パース後マップを保護
2. **PR_SET_DUMPABLE**: `prctl(PR_SET_DUMPABLE, 0)` でコアダンプ・ptrace を制限
3. **変数の生存期間最小化**: `execve` 直前まで変数を構築せず、直後にスコープを終わらせる

### 受け入れ基準
- `execenv` 実行中に `/proc/<pid>/mem` から平文が読めないことを確認
- `strace -e read` で sops stdout が外部に漏れていないこと

---

## Step 7: 統合テスト・ドキュメント

**目標**: E2E で動作確認し、リリース可能な状態にする

### 実装内容
- `tests/integration_test.rs`: テスト用 `.env.enc` を使った E2E テスト
- `README.md`: インストール・使い方・SOPS セットアップ手順
- `cargo clippy` / `cargo fmt` をパス
- `cargo build --release` で単一バイナリを生成

---

## 実装順序サマリー

```
Step 1: プロジェクト初期化      ← 今ここから始める
Step 2: CLI 引数パース
Step 3: SOPS Provider
Step 4: パーサー
Step 5: Executor (execve)       ← MVP 完成ライン
Step 6: セキュリティ強化
Step 7: 統合テスト・ドキュメント
```

各 Step は「プラン確認 → 実装 → 受け入れ基準の確認」の順で進める。
