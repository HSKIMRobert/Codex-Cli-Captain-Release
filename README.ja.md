# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>Codex CLI の作業を、ひとつの公開エントリから計画し、委任し、検証し、完了させます。</em></p>

CCC は Codex CLI のための小さな control plane です。すぐ答えればよいだけ
ではなく、計画、順序立てた作業、specialist の支援、review、完了証跡が
必要な作業では `$ccc` を使います。

現在の release: `v0.0.16` (`v0.0.16` tag、GitHub Release card、crates.io package は publish 済みです)。

## インストール

直接インストールする場合:

```bash
cargo install codex-cli-captain --force
ccc setup
```

その後、Codex CLI を完全に再起動して確認します。

```bash
ccc check-install --text
ccc status --text
```

AI エージェントにインストールを任せる場合は、次を貼り付けてください。

```text
Codex CLI 用の Codex-Cli-Captain をインストールまたは更新して。
crates.io に publish 済みの `v0.0.16` release を使い、install 後に
`ccc setup` で Codex CLI 連携を更新して。

1. まず現在の状態を確認して。
   - command -v ccc || true
   - ccc があれば ccc --version
   - ccc があれば ccc check-install --text

2. インストールまたは更新して。
   - cargo install codex-cli-captain --force
   - ccc setup

3. Codex CLI を完全に再起動するよう案内して。

4. 再起動後に確認して。
   - command -v ccc
   - ccc --version
   - ccc check-install --text
   - ccc status --text

5. $ccc、MCP registration、hooks、custom agents、Graph Context、LSP safety surface が current か、
   restart や PATH cleanup がまだ必要かを報告して。
```

## コマンド

Codex CLI の中では次を入力します。

| コマンド | 用途 |
| --- | --- |
| `$ccc "task"` | CCC 管理の Codex 作業を開始します。 |

ターミナルでは次を実行します。

| コマンド | 用途 |
| --- | --- |
| `cargo install codex-cli-captain --force` | crates.io から CCC binary をインストールまたは更新します。 |
| `ccc setup` | install または update 後に Codex CLI 連携を更新します。 |
| `ccc check-install --text` | install、hooks、skills、agents、Graph Context を確認します。 |
| `ccc status --text` | 現在の作業状態と次の行動を確認します。 |
| `ccc activity --json` | bounded activity と proof path を確認します。 |
| `ccc readiness-runbook --json` | readiness gap と次の bounded command を確認します。 |

## CCC がしてくれること

| 機能 | ユーザーにとっての意味 |
| --- | --- |
| 計画を先に作る | 広い作業は編集前に明確な plan を作ります。 |
| Captain orchestration | host Codex が指揮し、必要な specialist 作業を CCC がルーティングします。 |
| 静かな進捗管理 | status、checklist、fan-in を transcript noise ではなく CCC surface に保持します。 |
| 証跡ベースの完了 | 現在の validation と review evidence を完了判断の基準にします。 |

## 使い方

`$ccc` で始めます。

```text
$ccc release docs を更新して、まず現在の証跡を確認し、その後で変更点を報告して
```

フォローアップでも `$ccc` を使うか、CCC が persisted run を作ったときに
表示する continuation command を使います。

`$ccc` に向いている作業:

- 複数手順の code または docs 作業
- release、install、verification に敏感な作業
- 保存された plan から続ける作業
- subagent fan-in や review が役立つ作業

小さな単発の質問なら、通常の Codex CLI だけで十分なことが多いです。

## 更新

```bash
cargo install codex-cli-captain --force
ccc setup
```

Codex CLI を再起動してから実行します。

```bash
ccc check-install --text
```

## 削除

```bash
cargo uninstall codex-cli-captain
```

`cargo uninstall` は Cargo binary を削除します。ローカルの CCC-managed
hook、MCP、skill file を手動で削除する前に Codex config を確認してください。

## メモ

- `$ccc` が公開エントリです。role task card、fan-in、Graph Context、
  hooks、review detail は CCC 内部の proof surface です。
- Codex plugin hooks は任意です。有効にした場合は Codex CLI を再起動し、
  `ccc check-install --text` で確認してください。
- macOS は maintainer smoke の主な経路です。Linux と Windows build は各
  target 環境で `ccc check-install --text` により確認してください。
- crates.io から install した binary は Cargo が published crate から再ビルド
  するため、binary provenance が partial に見えることがあります。
