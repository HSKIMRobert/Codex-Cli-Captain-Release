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

現在の公開リリース: `0.0.2`.

CCC は Codex CLI のための captain-first orchestration layer です。`$cap` だけを public entrypoint として維持し、LongWay/task-card/fan-in state を保存し、specialist 作業を設定済みの `ccc_*` agent にルーティングしてから captain review に戻します。

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

release installer は既定で `v0.0.2` に固定されています。`CCC_VERSION` は意図的に別の release を入れる場合だけ設定してください。

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

## 推奨ロール設定

通常の CCC 利用では ChatGPT Pro $100 プランを開始点として推奨します。`$cap` workflow は captain と specialist の handoff を繰り返すため、Codex 使用量が増える可能性があります。

| CCC role | Stable agent ID | Display callsign | 推奨モデル | Reasoning | 説明 |
| --- | --- | --- | --- | --- | --- |
| `orchestrator` | `captain` | `Captain` | `gpt-5.5` | `medium` | host-owned routing label であり managed `ccc_*` specialist ではありません |
| `way` | `ccc_tactician` | `Executor` | `gpt-5.5` | `high` | 計画と bounded next-move の選択 |
| `explorer` | `ccc_scout` | `Observer` | `gpt-5.4-mini` | `high` | read-only repo evidence |
| `code specialist` | `ccc_raider` | `Marauder` | `gpt-5.5` | `high` | code/config mutation と repair |
| `documenter` | `ccc_scribe` | `Adjutant` | `gpt-5.4-mini` | `medium` | README と operator text |
| `verifier` | `ccc_arbiter` | `Arbiter` | `gpt-5.5` | `high` | captain-mediated review、risk、regression、acceptance check |
| `sentinel` | `ccc_sentinel` | `Overseer` | `gpt-5.4-mini` | `high` | run-scoped guardrail classification と status visibility |
| `companion_reader` | `ccc_companion_reader` | `Probe` | `gpt-5.4-mini` | `medium` | 低コストの filesystem/docs/web/git/gh 読み取り作業 |
| `companion_operator` | `ccc_companion_operator` | `SCV` | `gpt-5.4-mini` | `medium` | 低コストの git/gh mutation と狭い tool 作業 |

Stable `ccc_*` ID が routing truth です。StarCraft callsign は display-only です。
