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

現在の公開バージョン: `0.0.13-pre`.

サポート対象プラットフォーム: `darwin-arm64`、`darwin-x86_64`、`linux-arm64`、`linux-x86_64`、`windows-x86_64`.

## インストール

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

更新する場合も同じインストールコマンドを再実行し、Codex CLI を再起動してから `ccc check-install` を実行してください。

## 基本的な使い方

依頼の前に `$cap` を付けると、CCC が作業を管理します。

```text
$cap 認証フローをリファクタリングして、テストが通る状態を保って
```

CCC では `$cap` が entrypoint です。LongWay、task card、checklist、fan-in、status、restart handoff は CCC が管理します。

作業範囲も一緒に書くと、`$cap` の依頼がより正確になります。たとえば `$cap README の古い案内だけを整理して` のように書けます。

## 推奨ロール設定

CCC を日常的に使う場合は、ChatGPT Pro $100 plan を開始点として推奨します。`$cap` workflow は captain と specialist handoff を繰り返すため、Codex usage を多めに使うことがあるためです。Reasoning は作業スタイルと作業リスクに合わせて調整してください。広い計画、リスクの高いコード変更、レビューでは高い reasoning を維持し、狭く反復的で低リスクな作業では下げてもかまいません。

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
