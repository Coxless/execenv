# Next.js プロジェクトでの手動テスト手順

`execenv` を既存の Next.js プロジェクトに接続し、秘密情報が正しくアプリへ渡ることをローカルで確認するための手順書です。

## 前提条件

| ツール | 確認コマンド |
|---|---|
| `sops` v3.x | `sops --version` |
| `age` / `age-keygen` | `age-keygen --version` |
| Node.js（mise / nvm / system いずれでも可） | `node --version` |
| `execenv` バイナリ | `execenv --help` |

```bash
# execenv を PATH に入れる（ビルド済みであれば）
cargo install --path /path/to/execenv
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

## 2. Next.js 用 `.env` の作成

検証対象の Next.js プロジェクト直下に `.env` を作成します。

```dotenv
# サーバー専用シークレット（process.env で参照。クライアントには漏れない）
MY_SECRET=hello-from-execenv

# クライアントに到達する変数（NEXT_PUBLIC_ プレフィックスはビルド時に静的展開される）
NEXT_PUBLIC_API_BASE=https://api.example.test

# execenv は親シェルの PATH を引き継がない。
# next / node / npx を解決するために、自前で PATH を含める。
PATH=/usr/local/bin:/usr/bin:/bin
```

### PATH について

`execenv` は暗号化ファイルに書かれた変数のみを `execve` に渡します。親シェルの `PATH` は一切引き継ぎません（`src/executor.rs` の設計上の仕様）。Next.js の `next` コマンドは `node_modules/.bin/` にあるため、`PATH` に Node.js の bin ディレクトリを含めないと起動に失敗します。

**mise 利用時の例:**

```bash
# mise が管理している node のパスを確認
mise which node
# 例: /home/youruser/.local/share/mise/installs/node/22.19.0/bin/node
```

`.env` の `PATH` をそのディレクトリに合わせてください:

```dotenv
PATH=/home/youruser/.local/share/mise/installs/node/22.19.0/bin:/usr/local/bin:/usr/bin:/bin
```

**nvm 利用時の例:**

```dotenv
PATH=/home/youruser/.nvm/versions/node/v22.19.0/bin:/usr/local/bin:/usr/bin:/bin
```

### `.gitignore` の確認

```bash
# .env は gitignore に入れる（平文シークレットをコミットしない）
echo '.env' >> .gitignore
```

---

## 3. `.env.enc` の作成

```bash
sops --encrypt --input-type dotenv --output-type dotenv .env > .env.enc
```

> **注意:** プロジェクトルートに `.sops.yaml` が存在する場合、そこで定義された暗号化ルールが適用されます。意図しない設定が当たる場合は、`sops` の `--age` フラグで明示的に指定するか、`.sops.yaml` を確認してください。

`.env.enc` のみをリポジトリにコミットし、`.env` は絶対にコミットしないでください。

---

## 4. `execenv` 経由で `next dev` を起動

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

### 5-1. サーバー側：`process.env.MY_SECRET` の到達確認

Next.js プロジェクトに以下のファイルを **一時的に** 追加します（App Router 想定）。

**`app/api/check/route.ts`:**

```ts
export async function GET() {
  return Response.json({ secret: process.env.MY_SECRET ?? null });
}
```

別ターミナルで確認:

```bash
curl http://localhost:3000/api/check
# 期待値: {"secret":"hello-from-execenv"}
```

> 確認が終わったらこのファイルを削除してください。シークレット値を API 経由で公開したままにしないでください。

### 5-2. クライアント側：`NEXT_PUBLIC_API_BASE` のバンドル到達確認

`app/page.tsx`（または任意のページ）に以下を **一時的に** 追加します。

```tsx
<p data-testid="api-base">{process.env.NEXT_PUBLIC_API_BASE}</p>
```

ブラウザで `http://localhost:3000` を開き、`https://api.example.test` が表示されることを確認します。

または curl でも確認できます:

```bash
curl -s http://localhost:3000 | grep api-base
# 期待値: <p data-testid="api-base">https://api.example.test</p>
```

> **重要:** `NEXT_PUBLIC_*` 変数はビルド時（`next dev` の初回コンパイル時を含む）に JavaScript バンドルへ静的展開されます。`.env.enc` の値を変更した場合は `execenv ... -- npx next dev` を再起動してください。HMR（ホットリロード）では追従しません。

> 確認が終わったら追加した `<p>` タグを削除してください。

---

## トラブルシューティング

| 症状 | 原因 | 対処 |
|---|---|---|
| `command not found: next` / `npx` | `.env` の `PATH` に node が無い | `mise which node` などで実パスを確認し `.env` の `PATH` を更新 |
| `no key could decrypt the data` | `SOPS_AGE_KEY_FILE` 未設定 | `export SOPS_AGE_KEY_FILE=~/.config/sops/age/keys.txt` |
| クライアント側で値が `undefined` | `NEXT_PUBLIC_` プレフィックス忘れ、または再起動が必要 | プレフィックスを確認 → dev サーバを再起動 |
| `failed to exec ...` (execenv 起動直後) | NUL バイトや `=` を含む不正なキー名 | 平文の `.env` を見直して再暗号化 |
| ページには表示されるが API では `null` | `NEXT_PUBLIC_` が付いており Server Component ではアクセス可 | `NEXT_PUBLIC_` プレフィックス無しのキーを別途 `.env` に追加 |
