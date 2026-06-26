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

現在の crates.io release は `v0.0.22` です。crates.io package は installed
workflow green gate の後に publish 済みで、registry install による
post-publish verification も通過しています。

## Release Card

| 項目 | 状態 |
| --- | --- |
| Version | `v0.0.22` |
| crates.io | `codex-cli-captain 0.0.22` publish 済み |
| Git tag | `v0.0.22` |
| Release commit | `cd5fb5d` |
| Published install | `cargo install codex-cli-captain --version 0.0.22 --force` 成功 |
| Installed verification | `ccc --version` -> `0.0.22` |
| Natural-command E2E smoke | 短い `ccc 'README 설치 안내가 0.0.22 후보 기준으로 명확한지 검증해줘'` request が WAVE、Scout/graph-card preflight、ODYSSEY lane contract、host `update_plan` ACK、App-native Scribe/Arbiter lifecycle receipt、fan-in、LSP safety、reverify CLEAR、Stop closeout、`CCC WAVE DEACTIVE` まで通過しました。 |
| Final gates | `install_runtime_ready=true`, `install_release_blocking_gaps=0`, `workflow_parity_gaps=0` |

Highlights:

- 短い自然言語の CCC request が、operator が WAVE、Scout、ODYSSEY、role
  lane、Arbiter、Stop closeout を説明しなくても installed orchestrator
  path に入ります。
- Tactician、Scout、Raider、Scribe、Ghost、Arbiter、Sentinel、Oracle の
  role-specific CCC agent が install されます。
- installed workflow green proof gate、docs read-only proof、App-native
  lifecycle receipt、fan-in、Arbiter reverify、LSP safety、Stop closeout を
  含みます。

## インストール

直接インストールする場合:

```bash
cargo install codex-cli-captain --force
ccc setup --with-plugin --recreate-config
```

その後、Codex CLI を完全に再起動して確認します。

```bash
codex mcp list
codex plugin marketplace list
codex plugin list
ccc check-install --text
ccc status --text
```

AI エージェントにインストールを任せる場合は、次を貼り付けてください。

```text
Codex CLI 用の Codex-Cli-Captain をインストールまたは更新して。
crates.io に publish 済みの `v0.0.22` release を使い、install 後に
plugin support まで含めて Codex CLI 連携を更新して。

1. まず現在の状態を確認して。
   - command -v ccc || true
   - ccc があれば ccc --version
   - ccc があれば ccc check-install --text

2. インストールまたは更新して。
   - cargo install codex-cli-captain --force
   - ccc setup --with-plugin --recreate-config

3. 再起動前に確認して。
   - codex mcp list
   - codex plugin marketplace list
   - codex plugin list
   - ccc check-install --text

4. check が要求する場合は、Codex CLI / Codex App を完全に再起動するよう案内して。

5. 再起動後に確認して。
   - command -v ccc
   - ccc --version
   - codex mcp list
   - codex plugin marketplace list
   - codex plugin list
   - ccc check-install --text
   - ccc status --text

6. `codex plugin list` に `ccc@ccc-dev` が installed/enabled として表示されない場合は、
   plugin hooks に依存する前に `ccc setup --with-plugin --recreate-config` をもう一度実行して。

7. plugin hooks、graph-card planning context、または LSP lifecycle proof がまだ
   missing の場合は、小さな明示的な $ccc request を 1 回実行し、host が
   WAVE/PostToolUse/LSP/Stop lifecycle proof を観測できるようにしてから、次を再実行して。
   - ccc check-install --text
   - ccc status --text

8. $ccc、MCP registration、hooks、custom agents、Graph Context、LSP safety surface が current か、
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
| `ccc setup --with-plugin --recreate-config` | install または update 後に Codex CLI 連携、plugin marketplace/cache、hooks、skills、agents、config を更新します。 |
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

Scope note: `v0.0.22` は、短い自然言語の `$ccc` request が、operator が
workflow を説明しなくても installed orchestrator path に入るようにします。
検証済みの release 範囲には WAVE activation、Scout/graph-card preflight、
ODYSSEY lane contract、host update_plan mirror/ACK、
Tactician/Scout/Raider/Scribe/Ghost/Arbiter/Sentinel/Oracle の role surface、
App-native lifecycle receipt、fan-in、Arbiter reverify、docs read-only proof、
LSP safety、Stop closeout、`CCC WAVE DEACTIVE` が含まれます。

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
ccc setup --with-plugin --recreate-config
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
