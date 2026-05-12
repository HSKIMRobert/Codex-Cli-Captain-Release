# Codex-Cli-Captain

[English](./README.md) | [한국어](./README.ko.md) | [日本語](./README.ja.md)

Codex CLI の `$cap` リクエストを captain-first の流れで実行する Rust ランタイムです。

現在のソースバージョン: `0.0.15-pre`.

> サポート対象の release target は `darwin-arm64`、`darwin-x86_64`、`linux-arm64`、`linux-x86_64`、`windows-x86_64` です。macOS target は通常サポートされ、動作する想定です。Linux と Windows target も提供していますが、platform-specific な問題が残っている可能性があります。

## ローカルインストールと更新

```bash
cargo build --offline
ccc setup
```

その後 Codex CLI を完全に終了し、新しいセッションを開始して確認します。

```bash
ccc check-install
```

既存の local source checkout を更新する場合は、最新 source を取得して再ビルドし、`ccc setup` を再実行してください。その後 Codex CLI を完全に再起動し、`ccc check-install` で確認します。`setup` は現在のバイナリと `ccc-config.toml` を基準に MCP 登録、packaged `$cap` skill、CCC-managed custom agent を更新します。リリース installer は既定で `v0.0.15-pre` に固定されており、`CCC_VERSION` は意図した override のときだけ使います。インストール処理は新しい bundle を active path に切り替える前に stage し、以前の release bundle を rollback 用に保持し、CCC-managed plugin cache/version entry と legacy `skills/cap` のコピーだけを整理します。non-CCC の Codex config は保持されます。TypeScript/JavaScript LSP 設定は、将来の `lsp_diagnostics`、`lsp_references`、`lsp_definition`、`lsp_prepare_rename`、`lsp_rename` 用 config surface として記録されます。必要なら `npm install -g typescript typescript-language-server` でサーバーを用意できます。`0.0.15-pre` では runtime LSP execution は deferred で、CCC は language server を起動しません。任意の `rust-analyzer` は Rust 専用の local navigation support で、必要なら `rustup component add rust-analyzer` で入れられます。

## 設定変更の反映

`~/.config/ccc/ccc-config.toml` で各ロールの model、reasoning、fast mode を変更できます。新しく生成されるインストール用 `~/.config/ccc/ccc-config.toml` は `way`、`explorer`、`code specialist`、`verifier` に reasoning `variant = "high"`、`documenter`、`companion_reader`、`companion_operator` に reasoning `variant = "medium"` を使い、すべての role で `fast_mode = true` を維持します。`ccc setup` は既存のユーザー変更済み値を保持したまま不足している生成デフォルトを補完し、古い CCC 生成デフォルトをアップグレードします。変更後は Codex CLI に次を貼り付けてください。

```text
Run:
ccc setup

Then fully exit Codex CLI.
Start a new Codex CLI session.
Then run:
ccc check-install
```

`setup` は `ccc-config.toml` を基準に MCP 登録、`$cap` skill、CCC-managed custom agent を再同期します。

## `0.0.15-pre` の動作

- `$cap` は public entrypoint です。
- 既定の specialist 対象は設定済みの `ccc_*` custom agent です。`worker` と `explorer` のような generic label は、operator が明示的に override しない限り無効です。
- `ccc memory` は opt-in で、既定では unconfigured です。
- SSL Skill Registry は routing、planning、review 用の bounded evidence として提供され、persisted run state を置き換えません。
- `ccc status --subagents --text` と `ccc checklist --subagents --text` は、可能な場所で callsign と stable ID を一緒に表示します。例: `[x] scout-b completed child=ccc_scout role=Observer(ccc_scout)/explorer task="Inspect scope"`。
- `ccc status --projection --json '{...}'` と `ccc checklist --projection --json '{...}'` は、workspace root の `CCC_LONGWAY_PROJECTION.md` 1 つを更新します。`git diff -- CCC_LONGWAY_PROJECTION.md` で LongWay/subagent projection を確認でき、次の projection 更新で上書きされます。
- projection heading は、request language を検出できる場合その言語に合わせます。韓国語の `$cap` request は韓国語 projection label で表示されます。
- terminal host-subagent update は CCC active handle を release し、完了した host agent thread をまだ close する必要がある場合は status に表示します。
- mutation 完了は specialist fan-in の後に行われ、review-sensitive 変更の最終 gate は arbiter review です。
- `0.0.15-pre` は callsign mapping の案内を維持しつつ、oh-my-openagent に着想を得た workflow set（github-triage、get-unpublished-changes、remove-deadcode、ai-slop-remover、lsp-safe-refactor、review-work、pre-publish-review、hyperplan、git-master、publish、release-command-discipline、release-note、readme-maintenance、changelog、role-ownership、lane-conflict、fallback-classification、filesystem-evidence）を扱います。

安定した `ccc_*` ID が source of truth で、callsign は display-only です。`captain/orchestrator` は Command Center（または Captain、managed された `ccc_*` role ではありません）に対応します。`ccc_tactician/way` は Executor、`ccc_scout/explorer` は Observer、`ccc_raider/code specialist` は Marauder、`ccc_scribe/documenter` は Adjutant、`ccc_arbiter/verifier` は Arbiter、`ccc_sentinel` は Overseer、`ccc_companion_reader` は Probe、`ccc_companion_operator` は SCV に対応します。

Host UI layer が `Closed Carver [ccc_scout]` のような outer notification を出すことはありますが、その文言は host-managed であり CCC が保証する出力ではありません。CCC-controlled の status/projection output は `Observer(ccc_scout)` のように callsign と stable ID を併記します。

| Stable ID | Config role | Callsign | Theme |
| --- | --- | --- | --- |
| `ccc_tactician` | `way` | Executor | `starcraft_display_callsign` |
| `ccc_scout` | `explorer` | Observer | `starcraft_display_callsign` |
| `ccc_raider` | `code specialist` | Marauder | `starcraft_display_callsign` |
| `ccc_scribe` | `documenter` | Adjutant | `starcraft_display_callsign` |
| `ccc_arbiter` | `verifier` | Arbiter | `starcraft_display_callsign` |
| `ccc_sentinel` | `sentinel` | Overseer | `starcraft_display_callsign` |
| `ccc_companion_reader` | `companion_reader` | Probe | `starcraft_display_callsign` |
| `ccc_companion_operator` | `companion_operator` | SCV | `starcraft_display_callsign` |

将来の optional agent は文書上だけの扱いで、`0.0.15-pre` の runtime role ではありません: `ccc_release_arbiter`/Judicator、`ccc_qa_runner`/Valkyrie、`ccc_lsp_scout`/Science Vessel。

Per-agent workflow mapping は advisory です。Observer は `github-triage` と `get-unpublished-changes`、Marauder は `remove-deadcode`、`ai-slop-remover`、LSP safe refactor、Arbiter は `review-work` と `pre-publish-review`、Executor は `hyperplan`、SCV は `git-master`、publish、release command discipline、Adjutant は release note/README/changelog、Overseer は role ownership/lane conflict/fallback classification、Probe は lightweight GitHub/filesystem evidence collection を担当します。

## 推奨ロール設定

| CCC role | Stable agent ID | Display callsign | 推奨モデル | Reasoning | 用途 |
| --- | --- | --- | --- | --- | --- |
| `orchestrator` | `captain` | `Captain` | `gpt-5.5` | `medium` | host-owned routing label、managed `ccc_*` specialist ではありません |
| `way` | `ccc_tactician` | `Executor` | `gpt-5.5` | `high` | 計画と次の作業選択 |
| `explorer` | `ccc_scout` | `Observer` | `gpt-5.4-mini` | `high` | 読み取り専用の repo 調査 |
| `code specialist` | `ccc_raider` | `Marauder` | `gpt-5.5` | `high` | コード/config の変更と修復 |
| `documenter` | `ccc_scribe` | `Adjutant` | `gpt-5.4-mini` | `medium` | README、リリースノート、利用者向け文言 |
| `verifier` | `ccc_arbiter` | `Arbiter` | `gpt-5.5` | `high` | captain 主導のレビュー、リスク、回帰確認 |
| `companion_reader` | `ccc_companion_reader` | `Probe` | `gpt-5.4-mini` | `medium` | 低コストの filesystem/docs/web/git/gh 読み取り作業 |
| `companion_operator` | `ccc_companion_operator` | `SCV` | `gpt-5.4-mini` | `medium` | 低コストの git/gh 変更と狭い tool 実行 |

`gpt-5.5` は ChatGPT 認証の Codex で高価値ロールに推奨されるモデルです。現在のアカウントや実行経路でまだ利用できない場合、その高価値ロールは rollout が届くまで `gpt-5.4` を使います。

## 動作

`$cap` は CCC の public entrypoint です。CCC orchestration には `$cap` を直接使い、host planning surface は CCC contract ではありません。

CCC の流れは次のように分かれます。

1. `PLAN_SEQUENCE`: captain が intent を確認し、設定された Way agent に planning を渡します。host Plan Mode は background Way engine として使えず、CCC planning を所有または置換してはいけません。
2. Way は pending LongWay と candidate task card を作ります。この段階は read-only です。
3. operator が pending LongWay を承認します。
4. `EXECUTE_SEQUENCE`: captain が承認済み LongWay を読み直し、task card を materialize して scheduler/router から specialist に割り当てます。
5. specialist result は result envelope と compact fan-in として戻り、checklist/status/fan-in が progress truth になります。
6. captain は continue, repair, replan, reclaim, complete, restart handoff のどれを選ぶか判断し、mutation 完了は specialist fan-in と arbiter review の後にのみ確定します。

Host planning UI は operator が入力する内容を整える助けにはなりますが、CCC は Way agent 内で host Plan Mode を trigger したり依存したりしません。衝突する場合は CCC persisted LongWay、checklist、fan-in、resolve state が優先です。

0.0.15-pre は docs-and-release-gates の pre-release で、stricter intent-state-machine の動作を引き継ぎつつ callsign mapping の案内を追加し、oh-my-openagent に着想を得た workflow set（github-triage、get-unpublished-changes、remove-deadcode、ai-slop-remover、lsp-safe-refactor、review-work、pre-publish-review、hyperplan、git-master、publish、release-command-discipline、release-note、readme-maintenance、changelog、role-ownership、lane-conflict、fallback-classification、filesystem-evidence）を追跡します。full runtime parity や完成した rebuild ではありません。LSP runtime execution は deferred のままで、CCC は language server を起動しません。

ロールは `ccc-config.toml` で選びます。host Codex は captain として LongWay、routing、lifecycle、fan-in、review、validation、commit boundary を担当します。ordinary `$cap` の作業はまず適切な specialist に委任し、read-only 調査は `ccc_scout`、docs/operator text は `ccc_scribe`、code/config 変更は `ccc_raider`、review 判断は `ccc_arbiter` が担当します。captain が直接作業するのは、明示的な fallback、些細な operator-side 修正、または CCC が明確に degraded したと記録できる場合に限ります。設定済みの `ccc_*` custom agent が既定の specialist 名であり、明示的な override がない限り generic な `worker` と `explorer` label は使いません。

軽い filesystem/docs/fetch/git/gh 作業は、tool route に specialist owner がある場合、captain セッションに残さず設定済みの mini companion ロールへルーティングします。git と `gh` の読み取りは `companion_reader`、git と `gh` の変更は captain が明示的な fallback または degradation 理由を記録しない限り `companion_operator` が担当します。

`raider` のプロンプトには、既存のモジュール境界を尊重すること、実際の重複削減やテスト容易性がある場合だけ helper を分けること、大きすぎる関数や無関係なリライトを避けることを明記しています。

`v0.0.15-pre` の運用ポリシーでは、レビューは全タスクに自動で付くものではなく、captain が必要時だけ明示的に開く条件付きの手順です。レビューアは bounded な検証入力として扱い、accept/reassign/close の判断は captain が持ち続けます。新しいレビューを始める前に、hardware, memory, 同一マシンの concurrency 負荷も考慮してください。Long-session status は必要に応じて checkpoint/resume guidance を出し、`/compact`、`/new`、`/exit` は operator choice のままにします。

この草案では、サブエージェントの結果が戻ったら、captain はそれを accept, close, または unsatisfactory として扱えます。unsatisfactory の出力は rationale と次のアクションを LongWay/task-card state に記録するべきです。CCC は unsatisfactory または needs-work の結果を bounded specialist follow-up に正規化し、CCC が specialist 経路で repair や reassignment をルーティングできる場合は captain が local repair を直接行うべきではありません。元の scope がまだ有効なら、captain は同じ specialist に対して missing delta, risk, correction target だけに絞った prompt で、1 回だけ bounded repair を送ります。role や approach が間違っていたなら、より適した specialist に 1 回だけ bounded reassignment を送ります。以前の unsatisfactory 結果は history にそのまま見える必要があり、CCC は subagent-to-subagent handoff、unbounded retry、明示的な replan/re-scope なしの scope widening、explicit reason なしの silent degraded fallback を行わない方向で実装されるべきです。

計画中の介入経路は captain-owned です。ユーザーが subagent 実行中に介入する場合、その要求は captain 経由でのみ流れるべきです。captain は介入を bounded delta と rationale として LongWay/task-card state に記録し、clarification-only、bounded scope amendment、direction/risk correction のいずれかに分類したうえで、安全なら同じ worker の修正、forced interruption が未対応または scope が大きく変わった場合は reclaim、より適した specialist があれば reassignment のうち 1 つだけを選ぶ方針です。stale output は引き続き表示され、captain が明示的に merge しない限り選択済みの経路を静かに上書きできません。介入は dissatisfaction repair と同じ bounded retry/reassign budget を消費するため、無限 amend loop、明示的な replan/re-scope なしの scope widening、介入だけの duplicate mutable worker は許可されません。

Codex が `Too many open files (os error 24)` のような file descriptor 圧迫を報告した場合、新しい reviewer や specialist を追加で開かないでください。各 active host agent を terminal lifecycle update として記録し、captain が merge または reclaim したうえで、host session でその agent を close し、thread/file handle が解放されるまで単一経路で進めます。

transcript folding で長い status block が隠れる場合は、subagent-only または projection 経路を使います。

```bash
ccc status --subagents --text --json '{"run_id":"..."}'
ccc status --projection --json '{"run_id":"..."}'
git diff -- CCC_LONGWAY_PROJECTION.md
```

projection file は表示用 artifact だけです。persisted run state、task card、lifecycle、fan-in が引き続き source of truth です。

## リリース衛生

release repo は installer、docs、packaged `$cap` skill、コンパイル済み `ccc` バイナリだけを置く方針です。リリース asset 作成時に可能ならバイナリの symbol を削除し、公開前に sensitive string scan を実行します。

## 0.0.15 オペレーター向けガイダンス

0.0.15 文書では、`$cap` public contract、specialist-first routing、callsign mapping、release-gate hygiene、checkpoint/resume guidance、active-handle cleanup、verification/fan-in visibility を現在の release-facing 안내として整理します。

- comments や annotations を求められたら、内容を平坦化したり並べ替えたりせず、指定された chronological block format をそのまま保ちます。
- OMO sisyphus や harness という表現は外部プロセスとの連携ではなく、CCC の operating shape として解釈します。captain 1人、bounded specialist routing、そして各 specialist の結果が次の判断前に captain に戻る流れを維持します。
- 複雑またはリスクの高い解釈は、Way/tactician に渡す前に captain が operator に確認します。
- 役割分担は明示します。scout と companion_reader は evidence を集め、Way/tactician は plan を作り、raider と companion_operator は mutate し、scribe は docs/operator text を担当し、arbiter は risk と acceptance を確認し、captain が fan-in を管理します。各 specialist handoff では captain と Way が task-specific expertise framing を付け、subagent に role, stance, thinking mode を明示します。
- routed host subagent が fan-in 前に止まったら fallback reason を記録し、degraded captain-local fallback の前に bounded retry、reassign、または codex exec worker harness で回収します。
- 小さな docs 作業で optional review だけが必要な場合は、bounded status polling と visible follow-up を維持し、無期限に待たせず reclaim, retry, reassign を使います。
- routing がずれたら captain が drift を記録し、matching CCC specialist に再ルーティングしたうえで、adoption や repair の結果を merge 前に review します。
- LongWay row は operator が作業を追いやすいときだけ optional owner identity を表示できます。例: `[ ] Mill [ccc_scribe] : Clarify 0.0.15 docs routing requirements`.
- `ccc graph` と `ccc_code_graph` は CCC-owned の graph-facing surface のまま維持します。`graph_context` が有効で Graphify が ready のとき、既存の graph-facing surface は Graphify-backed provider/routing shim 経由で動作します。これは config-gated で default-off です。Graphify output は read-only evidence のまま保ち、Graphify が missing/stale の場合は legacy graph backend ではなく normal scout/source evidence に fallback します。新しい public graph command は追加しません。
- `ccc memory` は opt-in workspace memory です。preview/write 確認後に user preference、repeated rule、verified project fact だけを保存し、LongWay/run state/latest work result/inference-only observation は memory truth として扱いません。
- Status は current task-card owner が推論された specialist family と合わない場合に assignment-quality routing drift warning を表示します。
- release asset packaging、`install.sh`/`install.ps1` repair、`gh release upload`/`gh release edit` は、まず適切な specialist または operator role にルーティングします。
- docs/translation request は generated routing defaults が適用されるとき `ccc_scribe` にルーティングします。
- `$cap` は単独で動作します。`/plan` や `/goal` を CCC entry path として文書化しません。
- PLAN_SEQUENCE と EXECUTE_SEQUENCE は分離します。広い、危険、曖昧、release/branch/multi-file の作業は pending LongWay 承認前に実行しません。
- public `skills/cap/SKILL.md` は薄く保ちます。内部 routing、lifecycle、fan-in、fallback、context、compatibility policy は `CCC_MEMORY.md` または persisted `captain_instruction` guidance に置きます。
- Planned row の canonical truth は `longway.planned_rows` です。`phase_rows[].planned_rows` は status/checklist projection 専用で、matching `task_card_id` row だけを phase 下に表示し、unmatched row は top-level Planned row として残します。
- long-session rollover guidance は先に checkpoint を要求し、`/compact`、`/new`、`/exit` のどれを使うかは operator が選びます。

現在の release notes は [`docs/release/notes/v0.0.15-pre.md`](./docs/release/notes/v0.0.15-pre.md) を参照してください。
