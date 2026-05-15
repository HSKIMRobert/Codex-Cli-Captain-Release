# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>Codex CLI や Codex App の作業を最後まで進めたいですか？<br>
高性能モデルで全体を走らせるコストが気になりますか？<br>
それなら CCC を使ってみませんか？<br>
やりたいことの前に <code>$cap</code> を付けるだけです。<br>
そこから面白いことが起こるかもしれません。</em></p>

現在の公開リリース: `0.0.3`.

CCC は Codex CLI のための captain-first orchestration layer です。`$cap` だけを public entrypoint として維持し、LongWay/task-card/fan-in state を保存し、specialist 作業を managed agent にルーティングしてから captain review に戻します。

## インストール、更新、削除

基本のインストール経路:

```text
Cargo で Codex-Cli-Captain をインストールします:
cargo install codex-cli-captain

インストールが終わったら次を実行します:
ccc setup

その後、Codex CLI を完全に終了します。
新しい Codex CLI セッションを開始します。
その後、次を実行します:
ccc check-install
```

Windows PowerShell も同じ Cargo ベースの経路を使います。

既存の Cargo install を更新する場合は、`cargo install codex-cli-captain --force` を再実行してから `ccc setup` を実行してください。その後 Codex CLI を完全に再起動し、`ccc check-install` で確認します。

Cargo でインストールした binary を削除する場合は、`cargo uninstall codex-cli-captain` を実行してください。CCC-managed な整理も必要なら、まず `ccc uninstall --dry-run` で計画を確認し、内容が正しいときだけ `ccc uninstall --confirm` を実行します。MCP registration、`ccc-config.toml`、skills、custom agent を削除する前に `ccc check-install` で現在の状態を確認してください。

レガシー release-bundle fallback のみ:

macOS または Linux:

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
curl -fsSL https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.sh | bash

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

Windows PowerShell:

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
iwr -UseB https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.ps1 | iex

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

更新する場合も `cargo install codex-cli-captain --force` を再実行してから `ccc setup` を実行してください。その後 Codex CLI を完全に再起動し、`ccc check-install` を実行してください。installer は新しい bundle を active path に切り替える前に stage し、以前の release bundle を rollback 用に保持し、CCC-managed plugin と `$cap` ファイルを更新します。stale cache/version entry と legacy packaged cap copy のうち CCC が管理するものだけを整理し、non-CCC Codex config は保持します。

release-bundle fallback installer は、`v0.0.3` fallback bundle asset が publish および検証されるまでは既定で `v0.0.2` に固定されています。Cargo が `0.0.3` の基本インストール経路です。`CCC_VERSION` は意図的に別の release-bundle fallback を入れる場合だけ設定してください。

## 基本的な使い方

CCC-managed task は、リクエストの前に `$cap` を付けて始めます。

```text
$cap Refactor the auth flow and keep tests passing
```

CCC は LongWay、task card、checklist/projection、fan-in、status、restart handoff を所有します。host `/plan`、`/goal`、graph command は CCC public entrypoint ではありません。

## 設定変更の反映

`~/.config/ccc/ccc-config.toml` で各 CCC role の model、reasoning tier、fast-mode を変更できます。変更後は Codex CLI に次を貼り付けてください。

```text
Run:
ccc setup

Then fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

`ccc setup` はユーザーが変更した値を保持しつつ、不足している generated default を補完し、MCP registration、packaged `$cap` skill、CCC-managed custom agent を再同期します。

## 公開される動作

- `$cap` が公開 entrypoint です。
- CCC は計画、変更、レビュー、fan-in のために内部 managed role を使います。内部 routing の詳細は runtime state と release-work docs に置き、public README には列挙しません。
- `ccc setup` は現在の binary と `ccc-config.toml` から packaged `$cap` skill、MCP registration、plugin cache、managed agent を更新します。
- `ccc check-install` は active binary、Cargo candidate、plugin/cache discovery、packaged `$cap` skill、stale path、restart の必要性を報告します。
