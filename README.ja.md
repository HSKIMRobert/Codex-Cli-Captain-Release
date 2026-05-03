# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>Codex CLI で end-to-end の作業を進めたいですか？<br>
でも、高性能モデルで end-to-end を回すのは少し気になりますか？<br>
そんなときは CCC を試してみてください。<br>
やりたいことの前に <code>$cap</code> を付けるだけです。<br>
きっと想像以上の展開になります。</em></p>

現在の公開バージョン: `0.0.11-pre`.

<table>
<tr>
<td>

<strong>注意 - プレリリースの推奨デフォルト経路</strong><br><br>
<code>0.0.11-pre</code> が現在の公開プレリリース default です。macOS は正式サポート対象で、macOS arm64 はローカルでインストールと動作を確認済みです。Linux/Windows asset は初期テスト用に提供しており、同じ <code>ccc setup</code> / <code>ccc check-install</code> の流れで動作する想定ですが、実際の Linux/Windows 環境での検証はまだ十分ではありません。必要であれば <code>CCC_VERSION=v0.0.10-pre</code> で以前のプレリリースをインストールできます。

</td>
</tr>
</table>

## 0.0.11-pre リリースカード

`0.0.11-pre` は walking skeleton と follow-up hardening のためのリリースであり、full parity や rebuild 完了のリリースではありません。このリリースでは `$cap` を public CCC entrypoint とし、host `/plan` と `/goal` は optional host affordance として扱います。`$cap` はどちらの host surface がなくても動作します。

このリリースは、PLAN_SEQUENCE と EXECUTE_SEQUENCE、実行前の pending LongWay approval、checklist/status/fan-in を運用上の truth とする流れ、persisted state から作業を続けるための restart handoff を公開文書に反映します。

検証サマリー:

- Source repo PR #2 から PR #7 までが merge 済みで、release-facing source docs commit は `a92d25f2f73d5be06c96a150a0016d1cc48d3cbc` です。
- 最終実装 slice で `cargo test -p ccc` が通過しました。
- Public release repo 文書更新の前に release-facing source docs が merge 済みです。

`0.0.11-pre` で意図的に延期した項目:

- Loop 5 Intervention / Review-retry-replan-reclaim parity projection.
- Schema contract promotion.
- `setup_config` config surface change.
- Full runtime parity projection.

## インストールと更新

macOS/Linux では Codex CLI に次を貼り付けてください。

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
curl -fsSL https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.sh | bash

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

Windows PowerShell では次を使ってください。

```text
Install Codex-Cli-Captain from https://github.com/HoRi0506/Codex-Cli-Captain-Release by running:
iwr -UseB https://raw.githubusercontent.com/HoRi0506/Codex-Cli-Captain-Release/main/install.ps1 | iex

After installation finishes, fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

更新時も同じ install コマンドをもう一度実行してください。現在の release asset を取得し、現在の install 先と `ccc` command shim/binary を更新し、setup と `ccc check-install` を再実行します。その後、Codex CLI を完全に再起動すると、更新された `$cap` skill と custom agent が読み込まれます。

## 設定変更の反映

`~/.config/ccc/ccc-config.toml` を編集した後、Codex CLI に次を貼り付けてください。新しく生成されるインストール用 `~/.config/ccc/ccc-config.toml` は `explorer` に reasoning `variant = "high"` を、`documenter`、`companion_reader`、`companion_operator` に `variant = "medium"` を使い、すべての role は `fast_mode = true` を維持します。`ccc setup` は既存のユーザー変更済み値を保持したまま不足している生成デフォルトを補完し、古い CCC 生成デフォルトをアップグレードします。

生成デフォルトは captain の reasoning をデフォルトで medium に維持したまま、handoff のコストを下げます。設定済みの mini/specialist role はより速い service path を使い、delegated worker に渡す作業説明は必要な長さに短くし、`$cap` と custom-agent instructions も compact に保ちます。

```text
Run:
ccc setup

Then fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

CCC を日常的に使う場合は、ChatGPT Pro $100 plan を開始点として推奨します。`$cap` workflow は captain と specialist handoff を繰り返すため、Codex usage を多めに使うことがあるためです。Reasoning は作業スタイルと作業リスクに合わせて調整してください。広い計画、リスクの高いコード変更、レビューでは高い reasoning を維持し、狭く反復的で低リスクな作業では下げてもかまいません。

## 推奨ロール設定

| CCC role | Agent | 推奨モデル | Reasoning | 用途 |
| --- | --- | --- | --- | --- |
| `orchestrator` | `captain` | `gpt-5.5` | `medium` | LongWay 管理と最終ルーティング判断 |
| `way` | `tactician` | `gpt-5.5` | `high` | 計画と次の作業選択 |
| `explorer` | `scout` | `gpt-5.4-mini` | `high` | 読み取り専用の repo 調査 |
| `code specialist` | `raider` | `gpt-5.5` | `high` | コード/config の変更と修復 |
| `documenter` | `scribe` | `gpt-5.4-mini` | `medium` | README、リリースノート、利用者向け文言 |
| `verifier` | `arbiter` | `gpt-5.5` | `high` | レビュー、リスク、回帰確認 |
| `companion_reader` | `companion_reader` | `gpt-5.4-mini` | `medium` | 低コストの filesystem/docs/web/git/gh 読み取り作業 |
| `companion_operator` | `companion_operator` | `gpt-5.4-mini` | `medium` | 低コストの git/gh 変更と狭い tool 実行 |

`gpt-5.5` は ChatGPT 認証の Codex で高価値ロールに推奨されるモデルです。現在のアカウントや実行経路でまだ利用できない場合、そのロールは rollout が届くまで `gpt-5.4` を使います。

サブエージェント結果が unsatisfactory または needs-work の場合、CCC はそれを bounded specialist follow-up に正規化し、可能であれば local captain repair よりも specialist 経由の repair や reassignment を優先します。
