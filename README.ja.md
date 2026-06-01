# Codex-Cli-Captain

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.ko.md">한국어</a> ·
  <a href="./README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="./docs/assets/ccc-banner.png" alt="CCC Codex-Cli-Captain banner" width="100%">
</p>

<p align="center"><em>Codex CLI の作業を、計画、実行、fan-in、レビュー、証跡までひとつの流れで進めます。</em></p>

CCC は Codex CLI 作業の公開コントロール面です。公開ワークフローでは `$ccc` を使い、CCC が LongWay plan、task card、specialist routing、fan-in、review boundary、status projection、インストール検証を CCC-owned evidence surface 上で管理します。

現在の公開リリース: `0.0.13`。

## CCC が提供するもの

| Surface | 動作 |
| --- | --- |
| `$ccc` entry | host transcript state に依存しない、CCC を通す作業向けの public workflow prefix です。 |
| LongWay planning | 複数手順の作業、再起動時の引き継ぎ、後続作業の連続性のために plan/checklist state を保持します。 |
| Specialist routing | CCC は設定された `ccc_*` role にルーティングし、generic transcript text ではなく簡潔な fan-in を記録します。 |
| Review boundary | 通常の検証は Captain-owned です。より高リスクの release, destructive, security, operator-requested 作業は review に昇格します。 |
| Status proof | `ccc status --text`、app-panel output、checklist/projection、`ccc check-install --text` で現在の証跡と stale boundary を確認できます。 |
| Tool readiness | Graphify context、LSP readiness、hooks、memory、skill registry、install surface を、advisory metadata を runtime truth と誤認しない形で報告します。 |

## AI エージェントでインストールする

このブロックを Codex CLI、ChatGPT、または shell コマンドを実行できる別の AI エージェントに貼り付けてください。目的は binary のインストールだけではなく、現在の状態確認、インストールまたは更新、setup の実行、Codex CLI の再起動または再起動案内、そして結果の検証です。

```text
Codex CLI 用の Codex-Cli-Captain 0.0.13 をインストールまたは更新してください。

次の手順を慎重に進めてください。

1. まず現在の状態を確認します。
   - `command -v ccc || true` を実行します。
   - `ccc` があれば `ccc --version` と `ccc check-install --text` を実行します。
   - 既存 binary が current, shadowed, stale, missing のどれかを報告します。

2. Cargo からインストールまたは更新します。
   - `cargo install codex-cli-captain --force` を実行します。
   - `ccc setup` を実行します。

3. Codex CLI を完全に終了して新しいセッションを開始するよう伝えるか、このホストが安全に対応しているなら Codex CLI cache を再読み込みするよう伝えます。

4. 再起動後に次を検証します。
   - `command -v ccc` を実行します。
   - `ccc --version` を実行します。
   - `ccc check-install --text` を実行します。
   - `ccc status --text` を実行します。
   - `ccc status --app-panel --text` を実行します。

5. 結果を要約します。
   - インストールされた version
   - `$ccc` entry skill が current かどうか
   - MCP registration が一致しているかどうか
   - custom agents が synced されているかどうか
   - restart または cache reload がまだ必要かどうか
   - PATH shadowing、stale plugin cache、legacy bundle warning の有無
```

直接インストールする場合は次を実行します。

```bash
cargo install codex-cli-captain --force
ccc setup
```

その後、Codex CLI を完全に再起動してから次で確認します。

```bash
ccc check-install --text
ccc status --text
ccc status --app-panel --text
```

## 使い方

CCC-managed 作業を始めるには、依頼で `$ccc` を使います。

```text
$ccc release docs を更新して、まず現在のコードベース証跡を確認し、その後で変更点を報告して
```

フォローアップでも `$ccc` か、CCC が出力した continuation hint を使い続けます。CCC は workspace の `.ccc` runtime artifact 配下に LongWay/checklist/fan-in state を保存し、`ccc status` と app-panel view で現在の状態を表示します。

ホストがまだ legacy skill entry を公開している場合、その entry が compatibility path として動作し続けることがあります。

- macOS の cross-asset provenance execution は、release script が non-macOS binary を実行しようとすると停止することがあります。`v0.0.13` では cross asset に temporary wrapper と Zig linker settings を使いました。
- crates.io-installed binary は Cargo が published crate から再ビルドするため、`source_commit=unknown` や partial binary provenance を報告することがあります。
- release 作業用の local files、たとえば tarball や checksum は upload 後もこの release repo checkout に残ることがあります。意図して commit していない限り source proof とはみなしません。

## 互換性メモ

| Area | Status |
| --- | --- |
| macOS | サポート済みで、local maintainer smoke path で検証済みです。 |
| Linux | release asset は publish 済みです。install 後に環境で `ccc check-install --text` で確認してください。 |
| Windows | release asset は publish 済みです。install 後に環境で `ccc check-install --text` で確認してください。 |
| Codex plugin hooks | 任意です。有効化した場合は Codex CLI を再起動し、`ccc check-install --text` で確認してください。 |

## 更新

```bash
cargo install codex-cli-captain --force
ccc setup
```

Codex CLI を再起動してから `ccc check-install --text` を実行します。

## 削除

```bash
ccc uninstall --dry-run
ccc uninstall --confirm
cargo uninstall codex-cli-captain
```

まず dry-run を実行して path を確認してください。`cargo uninstall` は Cargo binary だけを削除し、`ccc uninstall --confirm` は unrelated Codex data を残したまま CCC-managed Codex surface を削除します。

## 公開 surface boundary

`$ccc` は公開 user entrypoint です。internal role name、fan-in artifact、Graphify detail、LSP state、hook、memory readiness は CCC 自身の proof surface であり、host-owned transcript claim として扱うべきではありません。host spawn/toast label は host が描画するため、CCC status と異なる場合があります。
