# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>Codex CLI や Codex App で end-to-end に進めたいですか?<br>
でも高性能モデルで最初から最後まで回すのは気になりますか?<br>
それなら CCC を使ってみませんか?<br>
やりたいことの前に <code>$cap</code> を付けるだけです。<br>
すると、ちょっと驚くことが起きるはずです。</em></p>

このリリースはローカル Codex plugin marketplace 経由で CCC をインストールします。パッケージには CCC plugin manifest、`.mcp.json`、plugin 提供の `$cap` skill が含まれ、installer は `ccc@ccc-local` plugin を有効化し、従来の直接 `mcp_servers.ccc` 登録と standalone `~/.codex/skills/cap` copy を削除します。公開 operator entrypoint は引き続き `$cap` です。

現在の公開バージョン: `0.0.15-pre`.

サポート対象の release target は `darwin-arm64`、`darwin-x86_64`、`linux-arm64`、`linux-x86_64`、`windows-x86_64` です。macOS target は通常サポートされ、動作する想定です。Linux と Windows target も提供していますが、platform-specific な問題が残っている可能性があります。

## インストール

macOS または Linux:

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
curl -fsSL https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.sh | bash

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
codex mcp list
```

Windows PowerShell:

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
iwr -UseB https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.ps1 | iex

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
codex mcp list
```

更新する場合も同じインストールコマンドを再実行し、Codex CLI を再起動してから `codex mcp list` を実行してください。Installer はローカル `ccc-local` marketplace を更新し、`plugins."ccc@ccc-local"` を有効化し、CCC が plugin として読み込まれるように従来の直接 `mcp_servers.ccc` block と standalone `$cap` skill を削除します。

CCC source や Rust 中心の repo で作業する場合、任意で Rust LSP を入れると便利です。

```bash
rustup component add rust-analyzer
```

安定した `ccc_*` ID は引き続き routing contract で、callsign は display-only です。`ccc_tactician` は Executor、`ccc_scout` は Observer、`ccc_raider` は Marauder、`ccc_scribe` は Adjutant、`ccc_arbiter` は Arbiter、`ccc_sentinel` は Overseer、`ccc_companion_reader` は Probe、`ccc_companion_operator` は SCV です。0.0.15-pre metadata には oh-my-openagent に着想を得た workflow set も含まれます: `github-triage`, `hyperplan`, `work-with-pr`, `pre-publish-review`, `git-master`, `review-work`, `remove-deadcode`, `get-unpublished-changes`, `ai-slop-remover`, `rust-analyzer-lsp`.

Host UI layer が `Closed Carver [ccc_scout]` のような outer notification を出すことはありますが、その文言は host-managed であり CCC が保証する出力ではありません。CCC-controlled の status/projection output は `Observer(ccc_scout)` のように callsign と stable ID を併記します。

## 推奨ロール設定

CCC を日常的に使う場合は、ChatGPT Pro $100 plan を開始点として推奨します。`$cap` workflow は captain と specialist handoff を繰り返すため、Codex usage を多めに使うことがあるためです。Reasoning は作業スタイルと作業リスクに合わせて調整してください。広い計画、リスクの高いコード変更、レビューでは高い reasoning を維持し、狭く反復的で低リスクな作業では下げてもかまいません。

| CCC role | Stable agent ID | Display callsign | 推奨モデル | Reasoning | 用途 |
| --- | --- | --- | --- | --- | --- |
| `orchestrator` | `captain` | `Captain` | `gpt-5.5` | `medium` | host-owned routing label、managed `ccc_*` specialist ではありません |
| `way` | `ccc_tactician` | `Executor` | `gpt-5.5` | `high` | 計画と次の作業選択 |
| `explorer` | `ccc_scout` | `Observer` | `gpt-5.4-mini` | `high` | 読み取り専用の repo 調査 |
| `code specialist` | `ccc_raider` | `Marauder` | `gpt-5.5` | `high` | コード/config の変更と修復 |
| `documenter` | `ccc_scribe` | `Adjutant` | `gpt-5.4-mini` | `medium` | README、リリースノート、利用者向け文言 |
| `verifier` | `ccc_arbiter` | `Arbiter` | `gpt-5.5` | `high` | レビュー、リスク、回帰確認 |
| `companion_reader` | `ccc_companion_reader` | `Probe` | `gpt-5.4-mini` | `medium` | 低コストの filesystem/docs/web/git/gh 読み取り作業 |
| `companion_operator` | `ccc_companion_operator` | `SCV` | `gpt-5.4-mini` | `medium` | 低コストの git/gh 変更と狭い tool 実行 |

`gpt-5.5` は ChatGPT 認証の Codex で高価値ロールに推奨されるモデルです。現在のアカウントや実行経路でまだ利用できない場合、そのロールは rollout が届くまで `gpt-5.4` を使います。
