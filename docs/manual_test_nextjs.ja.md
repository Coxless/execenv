# Next.js プロジェクトでの手動テスト手順

`execenv` をリポジトリ内の `manual-test/` Next.js プロジェクトに接続し、秘密情報が正しくアプリへ渡ることをローカルで確認するための手順書です。

`manual-test/` は App Router + TypeScript + Tailwind 4 構成で、`app/components/ServerEnvDisplay.tsx` が `TEST_ENV` と `NEXT_PUBLIC_TEST_ENV` の両方をページに表示します。依存関係は `pnpm` で管理し、ツールバージョンは `manual-test/.mise.toml` で固定されています（node 24.14.1 / pnpm 10.21.0）。

## 前提条件

| ツール | 確認コマンド |
|---|---|
| `sops` v3.x | `sops --version` |
| `age` / `age-keygen` | `age-keygen --version` |
| `pnpm` (mise 経由) | `cd manual-test && pnpm --version` |
| `execenv` バイナリ | `execenv --help` |

```bash
# execenv をビルドして PATH に入れる（リポジトリルートで）
cargo install --path .
```

> **注意:** `sops` と `age` のインストール手順は [README.ja.md](../README.ja.md) の「SOPS + age のセットアップ」を参照してください。

---

## 1. age 鍵の準備

以下のコマンドで鍵ファイルを生成し、環境変数をセットします（既に鍵がある場合はスキップ）。

```bash
age-keygen -o ~/.config/sops/age/keys.txt

# 復号用（ローカル開発）
export SOPS_AGE_KEY_FILE=~/.config/sops/age/keys.txt

# 暗号化用
export SOPS_AGE_RECIPIENTS=$(grep 'public key' ~/.config/sops/age/keys.txt | cut -d' ' -f4)
```

---

## 2. `manual-test/.env` の確認と PATH 追加

`manual-test/.env` にはテスト用の値がすでに用意されています。

```dotenv
NEXT_PUBLIC_TEST_ENV="Test value1"
TEST_ENV="Test value2"
```

`execenv` はこのファイルに書かれた変数のみを `execve` に渡し、親シェルの `PATH` は一切引き継ぎません（`src/executor.rs` の設計上の仕様）。`next` / `node` を解決するために `PATH` 行を追加してください。

```dotenv
NEXT_PUBLIC_TEST_ENV="Test value1"
TEST_ENV="Test value2"
PATH=/home/youruser/.local/share/mise/installs/node/24.14.1/bin:/usr/local/bin:/usr/bin:/bin
```

mise で管理している Node.js バイナリの実パスは以下で確認できます。

```bash
cd manual-test
mise which node
# 例: /home/youruser/.local/share/mise/installs/node/24.14.1/bin/node
# → PATH には末尾の /node を除いた bin/ ディレクトリを指定する
```

> **`.gitignore` について:** `*.env.enc` はリポジトリルートの `.gitignore` で無視済みです。`.env` はテスト用ダミー値のみのためコミット対象ですが、実際のシークレットを書いた場合はコミットしないでください。

---

## 3. `.env.enc` の作成

```bash
cd manual-test
sops --encrypt --input-type dotenv --output-type dotenv .env > .env.enc
```

> **注意:** プロジェクトルートに `.sops.yaml` が存在する場合、そこで定義された暗号化ルールが適用されます。意図しない設定が当たる場合は、`sops` の `--age` フラグで明示的に指定するか、`.sops.yaml` を確認してください。

`.env.enc` のみをリポジトリにコミットし、`.env` は絶対にコミットしないでください。

---

## 4. `execenv` 経由で `next dev` を起動

```bash
cd manual-test
```

以下の 3 形式のうち、環境に合ったものを使ってください。

| 形式 | コマンド | 用途 |
|---|---|---|
| **A. npx 経由（推奨）** | `execenv --provider sops --file .env.enc -- npx next dev` | PATH に node が入っていれば `npx` が `next` を解決 |
| **B. 相対パス直叩き** | `execenv --provider sops --file .env.enc -- ./node_modules/.bin/next dev` | node が PATH 不要で最小構成 |
| **C. node コマンド経由** | `execenv --provider sops --file .env.enc -- node node_modules/next/dist/bin/next dev` | A・B が動かない場合のフォールバック |

正常起動すると `next dev` のログが流れます。`execenv` プロセス自体は `execve` によって `next` プロセスに置き換わっているため、起動後は PID として残りません。

### よくあるエラー

```
failed to exec `npx`: No such file or directory
```
→ `.env` の `PATH` に node のバイナリディレクトリが含まれていません。セクション 2 の PATH 設定を見直してください。

```
no key could decrypt the data
```
→ `SOPS_AGE_KEY_FILE` が未設定です。`export SOPS_AGE_KEY_FILE=~/.config/sops/age/keys.txt` を実行してください。

---

## 5. 検証手順

### 5-1. ページでの確認（推奨）

`http://localhost:3000` を開くと、`ServerEnvDisplay` コンポーネントが `.env.enc` から注入された値をページに表示します。

| 表示項目 | 期待値 |
|---|---|
| `NEXT_PUBLIC_TEST_ENV` | `.env.enc` に書いた値（例: `Test value1`） |
| `TEST_ENV` | `.env.enc` に書いた値（例: `Test value2`） |

`(undefined)` と表示された場合は、該当する変数が `execenv` 経由で渡っていません。セクション 2 の `.env` と `.env.enc` を見直してください。

### 5-2. サーバー側：API エンドポイントで確認

`TEST_ENV` をより明示的に検証したい場合は、以下のファイルを **一時的に** 追加します。

**`app/api/check/route.ts`:**

```ts
export async function GET() {
  return Response.json({ testEnv: process.env.TEST_ENV ?? null });
}
```

別ターミナルで確認:

```bash
curl http://localhost:3000/api/check
# 期待値: {"testEnv":"Test value2"}
```

> 確認が終わったらこのファイルを削除してください。シークレット値を API 経由で公開したままにしないでください。

### 5-3. クライアント側：`NEXT_PUBLIC_TEST_ENV` のバンドル到達確認

`NEXT_PUBLIC_TEST_ENV` は `ServerEnvDisplay` でページに表示されるため、追加ファイルは不要です。ブラウザで表示を確認するだけで十分です。

> **重要:** `NEXT_PUBLIC_*` 変数はビルド時（`next dev` の初回コンパイル時を含む）に JavaScript バンドルへ静的展開されます。`.env.enc` の値を変更した場合は dev サーバーを再起動してください。HMR（ホットリロード）では追従しません。

---

## トラブルシューティング

| 症状 | 原因 | 対処 |
|---|---|---|
| `command not found: next` / `npx` | `.env` の `PATH` に node が無い | `cd manual-test && mise which node` で実パスを確認し `.env` の `PATH` を更新 |
| `no key could decrypt the data` | `SOPS_AGE_KEY_FILE` 未設定 | `export SOPS_AGE_KEY_FILE=~/.config/sops/age/keys.txt` |
| 画面に `(undefined)` と表示される | 変数が渡っていない、または再起動が必要 | `.env.enc` を再確認 → dev サーバを再起動 |
| `NEXT_PUBLIC_TEST_ENV` が `undefined` | `NEXT_PUBLIC_` プレフィックス忘れ、または再起動が必要 | プレフィックスを確認 → dev サーバを再起動 |
| `failed to exec ...` (execenv 起動直後) | NUL バイトや `=` を含む不正なキー名 | 平文の `.env` を見直して再暗号化 |
