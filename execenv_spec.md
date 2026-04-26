# execenv — セキュア環境変数ランタイム仕様書

## 1. 概要

`execenv` は、暗号化された設定情報（.envなど）を安全に復号し、
プロセス内で環境変数として注入した上で任意のコマンドを実行するCLIツールである。

特徴：
- ファイルに平文を書き出さない
- パイプを使わない（露出最小化）
- 子プロセス限定で環境変数を注入
- Providerプラグインにより複数のシークレットソースに対応

---

## 2. 目的

- .envファイルの平文保存を防ぐ
- CI/CDおよびローカル開発で安全に環境変数を扱う
- Next.jsなどのフレームワークとシームレスに統合する

---

## 3. アーキテクチャ

```
execenv
  ├─ Provider (sops / aws / 1password)
  │     └─ load() → string (dotenv or JSON)
  │
  ├─ Parser (dotenv / json)
  │     └─ { KEY: VALUE }
  │
  └─ Executor
        └─ execve(target, env)
```

---

## 4. CLI仕様

### 基本

```bash
execenv --provider sops --file .env.enc -- next dev
```

まずは、MVPとしてSOPSのみをサポートすることを想定する。

---

## 5. Providerインターフェース

```ts
type Provider = {
  load(): Promise<string>
}
```

### SOPS Provider（例）

```ts
load() {
  return execFileSync("sops", [
    "--decrypt",
    "--input-type", "dotenv",
    "--output-type", "dotenv",
    ".env.enc"
  ], { encoding: "utf-8" });
}
```

---

## 6. フォーマット対応

- dotenv
- JSON

```bash
execenv --format dotenv
execenv --format json
```

---

## 7. セキュリティ設計

### 基本方針

- ファイルに書き出さない
- stdoutに出さない
- ログに出さない
- メモリ滞在時間を最小化

### 強化オプション（Rust実装時）

- zeroize（メモリ消去）
- execve（プロセス置き換え）
- ptrace制限（PR_SET_DUMPABLE）
- バッファコピー最小化

---

## 8. 実行フロー

```
1. Providerがシークレットを取得（メモリ内）
2. ParserがKEY=VALUE形式に変換
3. 環境変数として構築
4. execveでターゲットプロセスを起動
5. 親プロセスは消滅
```

---

## 9. セキュリティ考慮事項

- rootユーザーからの観測は防げない
- 同一ユーザーによるptraceは制限が必要
- 長時間保持しない設計が重要

---

## 10. 将来拡張

- キャッシュ機構
- Secret rotation対応
- Kubernetes連携
- WASM provider
- リモート実行

---

## 11. 想定ユースケース

- Next.js開発環境
- CI/CDパイプライン
- ローカルRAG/AIツール
- セキュアCLI実行環境

---

## 12. まとめ

execenvは「dotenvの進化系」として、
セキュリティを重視した環境変数管理を提供する。

- dotenv → ファイルベース
- dotenvx → CLIラッパー
- execenv → セキュアランタイム

## 13. 備考
- 汎用コマンドProvider:

以下の汎用コマンドProviderは検討したが、セキュリティ上リスクになるため、実装は見送ることとする。

```bash
execenv --provider cmd -- "sops --decrypt .env.enc" -- next dev
```

- 言語:
Rustで実装する。
