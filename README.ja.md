# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>Codex CLI や Codex App の作業を、ひとつの接頭辞で最後まで進められます。<br>
やりたい作業の前に <code>$cap</code> を付けるだけで CCC が管理します。</em></p>

CCC は、Codex CLI と Codex App に対応した Rust ベースの orchestration layer です。`$cap` を public entrypoint として保ち、内部の作業フローを管理して、大きなタスクでも手動の切り替えを減らします。

## いつ使うか

計画、編集、レビュー、再起動後の引き継ぎが必要な作業など、Codex に最初から最後まで任せたいときに使います。

## インストール、更新、削除

基本のインストール方法:

```text
cargo install codex-cli-captain
ccc setup
```

その後、Codex CLI を完全に終了して新しいセッションを開始し、次を実行します。

```text
ccc check-install
```

既存の Cargo install を更新するときは次を実行します。

```text
cargo install codex-cli-captain --force
ccc setup
```

その後、Codex CLI を再起動して `ccc check-install` で確認します。

Cargo install を削除する場合は次を実行します。

```text
cargo uninstall codex-cli-captain
```

CCC-managed の整理も必要なら、まず `ccc uninstall --dry-run` で確認し、内容が正しいときだけ `ccc uninstall --confirm` を実行します。

レガシー release-bundle fallback のみ:

```text
curl -fsSL https://github.com/HoRi0506/Codex-Cli-Captain-Release/releases/download/v0.0.11/install.sh | bash
```

Windows PowerShell:

```text
iwr -UseB https://github.com/HoRi0506/Codex-Cli-Captain-Release/releases/download/v0.0.11/install.ps1 | iex
```

## その他の設定

`~/.config/ccc/ccc-config.toml` で CCC role の model、reasoning tier、fast-mode を変更できます。変更後は `ccc setup` を実行し、Codex CLI を再起動してから `ccc check-install` を確認します。

Codex plugin hooks は opt-in です。使う場合は `~/.codex/config.toml` で `[features] plugin_hooks = true` を設定し、Codex CLI を再起動して `/hooks` review で CCC hook を承認し、その後 `ccc check-install` を実行します。

## 対応 OS

CCC は macOS、Windows、Linux を対象にしていますが、環境によっては Windows と Linux が正常に動かない場合があります。インストール後に `ccc check-install` で確認してください。

## 0.0.11 メモ

`0.0.11` は Odyssey transition baseline です。現在の CCC インストール導線を整理した版で、release-bundle fallback の利用は引き続き可能なまま、以後の Odyssey 作業は次の段階に続きます。
