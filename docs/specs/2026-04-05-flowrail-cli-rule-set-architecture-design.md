# flowrail — Tiny Workflow Engine CLI for LLM (Rule Set Architecture)

## Overview

`flowrail` は LLM エージェント向けの超軽量ワークフローエンジン CLI。YAML で宣言された決定論的 state machine を、冪等かつ拡張可能に駆動する。

**テーマ**: 治具（flowrail）— ワークを固定して精密な作業を可能にする道具。LLM のワークフローを固定して正しく動かす。

**Linux 哲学**: Do One Thing and Do It Well. flowrail の One Thing = 「YAML で宣言された決定論的 state machine を、冪等かつ拡張可能に駆動する」のみ。監査、regate、inner-loop 等の**ワークフロー semantics は全て YAML 側の rule set に委譲される**。

**方向性**: dotfiles 内で実証 → 将来的に汎用 OSS 化（Claude Code, Cursor, Codex 等で利用可能）

**Supersedes**: `2026-04-05-flowrail-cli-design.md`（3 層プラガビリティモデル版）。当 spec は rule set を単一のプラガブル単位とするミニマリスト再設計である。

## 背景と課題

### 前 spec の問題点

`2026-04-05-flowrail-cli-design.md`（前バージョン）は 3 層プラガビリティモデル（Core / Strategy / Integration）を採用したが、**Core に audit/verify、regate、inner-loop 等のワークフロー semantics が組み込まれており flowrail がまだ大きい**という課題があった。Linux 哲学「Do One Thing and Do It Well」の観点では、flowrail は**決定論的 state machine の駆動のみ**に集中すべきで、監査や回復戦略は外出しが自然である。

### 根本原因

「ワークフローの決定論的な部分」と「ワークフロー固有の semantics」を混同していた。前者は flowrail の責務（=One Thing）、後者は YAML で宣言されるべき外部知識。

### 新方向性のコア

- **Phase 1 の flowrail コアは以下 5 つの概念のみを知る**:
  1. State machine（phase transition）
  2. Artifact lifecycle
  3. Rule set の import 解決と評価
  4. 4 つの core primitive check（file_exists, cmd_exit, regex_match, git_status）
  5. Hook executor
- **Phase 1 Infrastructure（概念ではなく全サブコマンド共通基盤）**:
  - Event Stream（`run.id` 付き JSONL）
  - Deterministic Mode（`FLOWRAIL_NOW` / `FLOWRAIL_SEED`）
- **Phase 2 で追加される Testing Framework（5 primitives、別章扱い）**: Recording / Replay / Assertion / State Diff / Pipeline Test Runner — rule set 作者の品質保証のみ、N 回実行・統計・可視化は外部ツール
- 監査・回復・反復・スナップショット（ハンドオーバー用途を含む）等は全て rule set として YAML に宣言される
- rule set は最小単位の responsibility で分割され、`imports:` で組み合わせて使う
- Primitive（最小責務） → Recipe（responsibility cohesive） → Pipeline（ユーザー特化）の 3 レイヤー

## 設計原則

### 7 つのコア原則

1. **flowrail は LLM を知らない** — stdin/stdout の JSON/Markdown で通信するだけ。プラットフォーム固有の知識はゼロ
2. **flowrail は高次 workflow semantics を知らない** — audit の Evidence Plan / 累積診断 / エスカレーション、regate strategy、inner-loop の HARD-GATE / 同一テスト 2 連続 PAUSE、autonomy の状況依存 if-then ルール等の**高次の workflow semantics** は全て rule set に外出し。**特に regate semantics (trigger 発火条件、rewind 先、failure classification) も全て rule set の `triggers:` セクションで宣言される**。flowrail core に専用の `flowrail run regate` コマンドは存在せず、`flowrail run step` が triggers を自動評価して `regate` / `pause` / `complete` のいずれかを決定・実行する。
   ただし flowrail core は以下の**低次の語彙**を知る（rule set 評価エンジンとして必要最小限）:
   - **8 つの built-in directive**: `confirm`, `classify_then_regate`, `produce_artifact`, `validation_question`, `regate_action`, `hook_command`, `phase_confirm`, `classifier_question`
   - **4 つの LLM 応答型**: verdict (`PASS` / `FAIL`), check result, classifier response, validation result
   - **rule set の実行モデル**: imports 解決、parameter binding、template expansion、lifecycle event 発火
3. **YAML が SSOT** — pipeline.yml + rule set ファイル群が唯一の仕様定義
4. **state.json が実行状態の SSOT** — フェーズ進捗、artifact 状態、trigger 履歴、snapshot 履歴を全て持つ。LLM のコンテキストに依存しない
5. **冪等性（Idempotency）** — 全サブコマンドは idempotent。atomic write + compare-and-set
6. **Hook-First Extensibility** — 外部連携（Linear, Slack, custom scripts）は全て lifecycle hook 経由。flowrail コア不変
7. **Tiny by Constraint** — 明示的に小さく保つ。**Phase 1: flowrail core ~3,500-3,800 LOC、Phase 2 完了時: ~4,400 LOC、バイナリ ≤ 20MB (Phase 2 完了時点計測)**。以下は **non-goals**:
   - LLM プラットフォーム固有の知識
   - DAG 並列実行（線形パイプラインのみ）
   - ワークフロー定義の動的生成
   - GUI（TUI のみ）
   - プラグインの動的ロード（hook は外部コマンド呼び出しのみ）
   - **高度な Testing DSL**（複雑な assertion 言語 / カスタム matcher / golden file 自動更新 / diff visualization / N 回実行統計 / LLM バージョン tracking）。Phase 2 で追加される Testing Framework は最小限の **5 primitives のみ**: Recording / Replay / Assertion / State Diff / Pipeline Test Runner。assertion は `verdict` + `checks_passed` / `failed_checks` の set 比較のみ。N 回実行・consistency 計算・可視化は外部ツールの責務
   - **高次 workflow semantics の組み込み**（audit / regate / inner-loop / autonomy は全て rule set に外出し）
   - **並列実行**: 同一 `.flowrail/{pipeline}-{branch}.state.json` に対する複数プロセスの並行 `flowrail run *` 呼び出しは undefined behavior。Phase 2 以降の拡張候補
8. **Separation by Audience** — flowrail は 2 種類の利用者（**Agent** と **Human**）を想定し、それぞれに最適化された**別バイナリ**を提供する。
   - **`flowrail` (agent CLI)**: LLM エージェント・CI/CD・スクリプト向け。stdin/stdout JSON 通信が主、security-first、**minimal dependencies** を重視。依存は `flowrail-core` + `clap` のみで、TUI/GUI ライブラリ (ratatui, crossterm 等) を一切含まない
   - **`flowrail-tui` (human CLI)**: 開発者・運用者が手動で workflow を監視・デバッグする TUI。`ratatui` + `crossterm` 等の rich UI 依存を含む
   - この分離は **Cargo workspace の crate 分離**で保証される (feature flag ではなく独立 crate / 独立 binary)。agent CLI への TUI/GUI 依存の混入は構造的に不可能
   - `flowrail-core` は両バイナリが共通で参照する pure library (state machine / rule set resolver / primitive checks / hook executor)
   - 詳細は後述の "Binary Separation" セクション参照

## アーキテクチャ

### 全体図

```
┌─────────────────────────────────────────────────────────────┐
│  LLM (Claude Code / Cursor / Codex / ...)                   │
│                                                             │
│  ┌──────────────┐                                           │
│  │   SKILL      │  超薄い (~20行)。flowrail の呼び出しループのみ │
│  └──────┬───────┘                                           │
│         │ flowrail run next / verify / step                      │
│         ▼                                                   │
│  ┌─────────────────────────────────────────────┐            │
│  │  flowrail CLI (Phase 1: ~3,500-3,800 LOC,         │            │
│  │           Phase 2 完了: ~4,400 LOC)          │            │
│  │  ┌───────────────┐  ┌──────────────────┐    │            │
│  │  │ State Machine │  │ Rule Set Resolver│    │            │
│  │  │ phase/artifact│  │ import+bind+eval │    │            │
│  │  └───────┬───────┘  └────────┬─────────┘    │            │
│  │          │                   │              │            │
│  │  ┌───────▼───────────────────▼─────────┐    │            │
│  │  │     4 Core Primitive Check Types    │    │            │
│  │  │ file_exists | cmd_exit |            │    │            │
│  │  │ regex_match | git_status            │    │            │
│  │  └─────────────────────────────────────┘    │            │
│  │                                              │            │
│  │  ┌──────────────┐  ┌────────┐  ┌──────┐    │            │
│  │  │Hook Executor │  │Lint/Fmt│  │  TUI │    │            │
│  │  │ env+stdin    │  │        │  │      │    │            │
│  │  └──────────────┘  └────────┘  └──────┘    │            │
│  └──────┬───────────────────────────────────────┘            │
│         │                                                    │
└─────────┼────────────────────────────────────────────────────┘
          │
   ┌──────┴──────────────────────────────────┐
   ▼                                         ▼
┌─────────────────┐              ┌──────────────────────────┐
│  YAML Universe  │              │  .flowrail/ (local state)     │
│                 │              │  {pipeline}-{branch}.json│
│  ┌───────────┐  │              └──────────────────────────┘
│  │ Pipeline  │  Layer 3: ユーザー特化ワークフロー
│  │   .yml    │  (feature-dev.yml, debug-flow.yml, triage.yml)
│  └─────┬─────┘
│        │ imports
│  ┌─────▼─────┐  Layer 2: Recipe rule sets (標準同梱 ~10 個 + ユーザー定義)
│  │  Recipe   │  (audit-gate.yml, regate-on-test-fail.yml,
│  │   .yml    │   phase-summary-yaml.yml, ...)
│  └─────┬─────┘
│        │ imports
│  ┌─────▼─────┐  Layer 1: Primitive rule sets (標準同梱 ~15 個)
│  │ Primitive │  (check-file-exists.yml, check-command.yml,
│  │   .yml    │   check-git-status.yml, ...)
│  └───────────┘
└─────────────────┘
```

### 通信モデル

LLM が `flowrail` を呼ぶ。`flowrail` が LLM を呼ぶことはない。

```
LLM: flowrail run next          → CLI: 次 phase の情報 + rule set 評価結果
LLM: (phase 実行)
LLM: flowrail run verify        → CLI: rule set の checks を実行、結果を出力
LLM: flowrail run step          → CLI: 状態遷移、triggers 自動評価、次 phase / regate / pause / complete
```

全サブコマンドの出力は JSON（デフォルト）または Markdown（`--format md`）。エラーは stderr、正常出力は stdout。

## flowrail Core の責務

**Phase 1 の flowrail Core が知っている概念は 5 つのみ**：

| 概念 | 内容 | 不変条件 |
|------|------|----------|
| **State Machine** | phase transition (pending → running → verifying → completed / failed / skipped / paused / aborted) | 状態遷移は atomic、hash-based compare-and-set、single-write transaction (phase 遷移 + trigger 評価 + trigger_history 追記を単一 state.json 書き込みで完結) |
| **Artifact Lifecycle** | artifact の存在・検証状態 (pending → produced → verified / failed) | produced_by と consumed_by の位相順序は flowrail が強制 |
| **Rule Set Resolver** | imports を再帰的に解決、parameter binding、template expansion、primitive への変換、pipeline-root `uses:` を各 phase の global-trigger table に展開 | 循環 import は**実行時に `max_depth` 超過で顕在化**（lint では cycle detection を行わない、rule set 作者の自己責任）。層間逆依存の enforcement は flowrail core の関心外（"Conventions & Best Practices" 参照）|
| **4 Core Primitive Checks** | file_exists / cmd_exit / regex_match / git_status を実装 | 決定論的、副作用なし、idempotent |
| **Hook Executor** | lifecycle event で外部コマンド発火 (env + stdin JSON)、state commit 後の post-action として fire-and-forget | 失敗はワークフローを止めない、macOS + Linux のみサポート (POSIX `sh -c`) |

**Phase 1 Infrastructure (概念ではなく全サブコマンド共通基盤):**

| Infrastructure | 内容 |
|---------------|------|
| **Event Stream** | 全 event に `run.id` (UUID) 付与、JSONL 形式、`FLOWRAIL_EVENTS_FILE=<path>` で出力先指定 |
| **Deterministic Mode** | `--deterministic` フラグまたは `FLOWRAIL_DETERMINISTIC=1` で有効化、`FLOWRAIL_NOW=<iso8601>` でタイムスタンプ固定、`FLOWRAIL_SEED=<uuid>` で `run.id` 固定 |

**Phase 2 以降で追加される Testing Framework (別章扱い、5 primitives):**

- Recording Mode (`--record <path>`)
- Recording Replay (`--replay-from <path>`)
- Assertion Mode (`--assert-recording <baseline>` / `--assert-level strict|loose`)
- State Diff (`flowrail state diff <a> <b> [--ignore <fields>]`)
- Pipeline Test Runner (`flowrail pipeline test` で rule set の `tests:` 実行)

これらは Phase 1 の flowrail core 5 概念には含まれず、Phase 2 の独立章で定義される。原則 7 の「最小限の 5 primitives のみ」と整合する。

**flowrail Core が知らないもの:**
- `sections_present`, `tests_pass`, `build_passes`, `git_committed` 等の高次 check → 標準 recipe rule set で実現
- `audit` の Evidence Plan / 累積診断, `regate` strategy, `inner-loop` の HARD-GATE / 同一テスト 2 連続 PAUSE, `handover`（= snapshot のユースケース名）, `autonomy` の状況依存 if-then 等の**高次 workflow semantics** → 全て rule set
- LLM プラットフォーム固有の知識
- DAG 並列実行、動的ワークフロー生成、プラグイン動的ロード
- 複数プロセスによる state.json への並列書き込み（単一プロセス前提、compare-and-set で衝突検出時は即 exit）

**flowrail Core が知っている低次の語彙 (原則 2 参照):**
- 8 built-in directive: `confirm` / `classify_then_regate` / `produce_artifact` / `validation_question` / `regate_action` / `hook_command` / `phase_confirm` / `classifier_question`
- 4 LLM 応答型: verdict / check result / classifier response / validation result

## 3 レイヤー構造

### Layer 1: Primitive Rule Sets

最小責務、parametrized、flowrail の core primitive 1〜数個を薄く wrap した再利用単位。

```yaml
# rules/primitives/check-file-exists.yml
kind: rule-set
name: check-file-exists
version: 1
description: "ファイルまたは glob にマッチするファイルが存在するか"
params:
  path: { type: string, required: true }
checks:
  - primitive: file_exists
    args: { path: "{{ path }}" }
```

```yaml
# rules/primitives/check-command.yml
kind: rule-set
name: check-command
version: 1
description: "任意のコマンドの exit code が期待値か"
params:
  cmd: { type: string, required: true }
  expected_exit: { type: integer, default: 0 }
checks:
  - primitive: cmd_exit
    args: { command: "{{ cmd }}", expected: "{{ expected_exit }}" }
```

**責務範囲:**
- 1 primitive rule set = 1 つの最小責務
- core primitive への型安全な wrap
- parameterize のインターフェース化
- 複数パイプラインで流用されることを前提とする

### Layer 2: Recipe Rule Sets

複数の primitive rule set を import し、responsibility cohesive な単位でまとめた「参考実装」。flowrail プロジェクトが標準 recipe として同梱し、ユーザーは自由に import・改変・追加できる。

```yaml
# rules/recipes/audit-gate.yml
kind: rule-set
name: audit-gate
version: 1
description: "成果物の機械的・意味的な検証ゲート（監査ドメイン）"
imports:
  - rules/primitives/check-file-exists.yml
  - rules/recipes/check-git-committed.yml
  - rules/recipes/check-sections-present.yml
  - rules/primitives/validation-question.yml
params:
  artifact_path: { type: string, required: true }
  required_sections: { type: list[string], default: [] }
  require_committed: { type: bool, default: true }
  validation_questions: { type: list[object], default: [] }
uses:
  - check-file-exists:
      path: "{{ artifact_path }}"
  - check-git-committed:
      path: "{{ artifact_path }}"
    when: "{{ require_committed }}"
  - check-sections-present:
      file: "{{ artifact_path }}"
      sections: "{{ required_sections }}"
    when: "{{ required_sections | length > 0 }}"
  - for-each: "{{ validation_questions }}"
    as: q
    call:
      validation-question:
        text: "{{ q.text }}"
        against: "{{ q.against }}"
```

**責務範囲:**
- 1 recipe = 1 ドメイン（監査、regate 戦略、phase summary、TDD 反復 等）
- 現行 `workflow-engine/modules/*.md`（audit, regate, inner-loop 等）の代替
- ユーザーはコピーして改変可能、独自 recipe も追加可能

### Layer 3: Pipeline YAML

ユーザー特化のワークフロー定義。recipe rule set を import し、phase と artifact を宣言する。

```yaml
# claude/skills/feature-dev/pipeline.yml (skill 配下に配置、現行構造を維持)
kind: pipeline
name: feature-dev
version: 4

flags:
  --linear:
    enables:
      integrations: [linear-sync]
  --accept:
    enables:
      phases: [accept-test]
  --doc:
    enables:
      phases: [doc-audit]
  --iterations:
    type: integer
    default: 3
    binds_to_param:
      rule_set: review-parallel
      param: iteration_count

imports:
  - rules/recipes/audit-gate.yml
  - rules/recipes/regate-on-test-fail.yml
  - rules/recipes/phase-summary-yaml.yml

settings:
  default_snapshot_hint_after_phases: 3
  max_phase_retries: 3

artifacts:
  spec_file:
    type: file
    pattern: "docs/superpowers/specs/{date}-*-design.md"
    produced_by: design
    consumed_by: [spec-review, plan]
  # ...

phases:
  - id: design
    confirm: after
    uses:
      - audit-gate:
          artifact_path: "{{ artifact.spec_file }}"
          required_sections: [requirements, components, impact-analysis]
  # ...

# Pipeline-level uses: pipeline-global な trigger / hook 宣言
# flowrail pipeline lint 時に compile-time で normalize され、各 phase の triggers に展開される
# (global-trigger table として state.json に persist される)
uses:
  - regate-on-test-fail:
      rewind_to: execute_impl
      max_retries: 3

integrations:
  - name: linear-sync
    enabled_by: --linear
    hooks:
      on_phase_complete: [sync_phase_summary, sync_evidence]   # 複数 hook を配列で宣言可能 (並列実行)
      on_trigger_fired: [sync_regate]
      on_snapshot_created: [sync_session]
      on_pipeline_complete: [sync_complete]
```

### レイヤー間の依存 (推奨 Convention)

> **注記**: 以下は **flowrail core が強制しない推奨 convention** である。詳細は後述の "Conventions & Best Practices" セクションを参照。flowrail core は rule set の構造的品質保証 (層間逆依存検証、cycle detection) を関与せず、rule set 作者の自己責任または別の静的解析ツールに委ねる (原則 7 "Tiny by Constraint"、"YAML Universe (Future)" 参照)。

```
Pipeline ──imports──▶ Recipe ──imports──▶ Primitive ──uses──▶ flowrail core
   △                     △                    │
   │                     │                    ▼
   │                     │              4 primitive checks
   │                     │              (file_exists, cmd_exit,
   │                     │               regex_match, git_status)
   │                     │
   └─── 直接 Primitive 可能（任意） ───┘
```

**推奨 (convention)**:
- **逆依存は推奨されない**: Primitive が Recipe を import すると責務混線、Recipe が Pipeline を import すると循環リスク
- **Pipeline は Primitive を直接 import 可能**: 薄い wrap が不要な場合のショートカット
- **同層内 import は許可**: Recipe → Recipe, Primitive → Primitive は通常パターン (`audit-gate` が `check-git-committed` 等を import する)

**flowrail core が強制しないもの**:
- layer metadata (旧設計では `layer: primitive|recipe|pipeline-local` 必須フィールドだったが撤回)
- cycle detection (実行時に `max_depth` 超過で顕在化するため事前検出は行わない)
- 同層/逆依存の validation

これらは rule set 作者の自己責任、または rule set 作者が独自の静的解析ツールを用いて検証する。

## Conventions & Best Practices

本セクションは **flowrail core が強制しない推奨プラクティス**を記述する。原則 7 "Tiny by Constraint" に従い、これらは rule set 作者のガイドラインであり、flowrail core による自動検証・enforcement は行わない。

### 3 レイヤー構造の推奨

rule set を以下の 3 レイヤーに分けることを推奨する:

1. **Primitive Rule Set** (~15 個、標準 catalog)
   - 1 primitive rule set = 1 つの最小責務
   - flowrail core primitive を薄く wrap、parameterize のインターフェース化
   - 複数 pipeline で流用されることを前提

2. **Recipe Rule Set** (~10 個、標準 catalog)
   - 1 recipe = 1 ドメイン (監査、regate 戦略、phase summary 等)
   - 複数の primitive rule set を import し、responsibility cohesive にまとめる
   - ユーザーはコピーして改変可能、独自 recipe も追加可能

3. **Pipeline YAML** (ユーザー特化)
   - `feature-dev.yml`, `debug-flow.yml` 等のユーザー定義
   - recipe rule set を import し、phase と artifact を宣言する

### 逆依存の回避

以下は**推奨**であり、flowrail core が強制するものではない:

- Primitive は Recipe を import しない (責務混線を避ける)
- Recipe は Pipeline を import しない (循環リスク)
- Pipeline は Primitive を直接 import してもよい (薄い wrap が不要な場合のショートカット)

### 循環参照の回避

- rule set の `uses:` / `imports:` で循環 (A → B → A) を作らない
- flowrail core は事前検出せず、実行時に `max_depth` 超過で顕在化する (デフォルト `max_depth=100`)
- 必要に応じて rule set 作者が独自の静的解析ツールでチェックする

### カタログ配置

- flowrail プロジェクト同梱の標準 catalog は `catalog/primitives/` と `catalog/recipes/` に配置
- ユーザー定義の rule set は任意の場所に配置可能 (flowrail core は directory layout に依存しない)

### Rule Set の自己検証 (Phase 2)

Phase 2 で rule set 作者は `tests:` セクションを用いて自身の rule set の期待動作を宣言できる。`flowrail pipeline test` で実行。詳細は "Phase 2: Testing Framework" セクション参照。

これらは**推奨**であり、作者は独自のテスト戦略を採用することも許容される。

## YAML Universe (Future Vision)

本セクションは将来の構想を記述する。Phase 1-4 の実装スコープ外だが、設計判断の根拠として重要。

### 構想

rule set を **Web で誰でも公開/取得できるエコシステム**を構築する。Rust crate の `crates.io`、Node.js の `npm`、Python の `PyPI` に相当する rule set marketplace。

- rule set 作者は自作の rule set を Web に公開できる
- flowrail ユーザーは URL / パッケージ名で他者の rule set を取得・利用できる
- marketplace の具体形態: git-based (GitHub raw URL), centralized registry, content-addressable hash による配信 等 (将来設計)

### エコシステム成立の前提: flowrail core が品質保証機構を持たない

YAML Universe が成立するには、flowrail core が以下を**持たない**ことが必須である:

1. **`layer` metadata 必須化の撤回** (完了 — 本 spec で撤回済み)
   - 理由: marketplace 参入障壁を下げる。rule set 作者全員に「3 層構造を理解し layer を宣言せよ」と要求すると、多様な発想の rule set が生まれない
   - 代わりに: 3 レイヤーは "Conventions" として推奨されるのみ

2. **cycle detection の事前検出を行わない**
   - 理由: 作者が独自の階層構造や相互参照パターンを採用する余地を残す
   - 代わりに: 実行時の `max_depth` 超過で顕在化、エラーメッセージで循環の可能性を案内

3. **構造的品質保証を外部ツールに委ねる**
   - 将来、flowrail とは別プロジェクトで `flowrail-lint` (仮称) のような静的解析ツールが作られる可能性
   - 品質保証は第三者製ツールによる多様な分析手法を許容する

### flowrail core の責務と marketplace の関係

| 責務 | flowrail core | marketplace エコシステム (将来) |
|------|--------------|-----------------------------|
| rule set の解析・実行 | ✅ core の責務 | — |
| rule set の構造検証 (layer, cycle, 逆依存) | ❌ 関与しない | ✅ 第三者製静的解析ツール |
| rule set の配信・バージョン管理 | ❌ 関与しない | ✅ marketplace 本体 |
| rule set の品質スコア・レビュー | ❌ 関与しない | ✅ marketplace (コミュニティ) |

### 原則 7 との整合

本構想は原則 7 "Tiny by Constraint" の必然的帰結である。flowrail core を極限まで小さく保つことで、エコシステム全体の多様性と柔軟性を最大化する。

## Rule Set Schema

### 完全スキーマ

```yaml
kind: rule-set                # 必須。常に "rule-set"
name: <identifier>            # 必須。他 rule set から参照される名前
version: <integer>            # 必須。semantic version
description: <string>         # 任意。人間向け説明
imports:                      # 任意。他 rule set への依存
  - <path-to-other-rule-set.yml>
params:                       # 任意。外部から渡されるパラメータ定義
  <param_name>:
    type: string | integer | bool | list[string] | list[object] | object
    required: true | false    # default: false
    default: <value>          # type に応じた default

# --- 以下、rule set が宣言する action/section ---

checks:                       # 任意。verification chain（機械的チェック）
  - primitive: <primitive_name>    # file_exists | cmd_exit | regex_match | git_status
    args: <primitive_args>
    when: <template_expression>   # 任意、条件付き実行
  - call: <rule_set_name>           # 他 rule set を呼び出し（import 済みであること）
    params: <params_to_pass>
    when: <template_expression>

validations:                  # 任意。LLM への質問（意味的チェック）
  - id: <slug>                # 必須。--validation-result <id>=<result> の参照キー
    question: <string>        # 必須。LLM に提示する質問
    against: <string> | list[<string>]  # 任意。user_request | codebase | test_results | fail_details | findings | <artifact_name>

uses:                         # 任意。recipe 層で使う。imports した rule set を呼び出す
  - <rule_set_name>:
      <param_name>: <value>
    when: <template_expression>
  - for-each: <template_expression>
    as: <variable_name>
    call:
      <rule_set_name>:
        <param_name>: <value>

triggers:                     # 任意。regate トリガー宣言
  - name: <trigger_name>
    condition: <condition_expression>
    action: regate | pause | classify_then_regate
    rewind_to: <phase_id> | current
    max_retries: <integer>
    on_exhausted: pause | fail | complete   # 任意、default: pause (max_retries 超過時の fallback)
    classifier:               # action=classify_then_regate の時のみ
      question: <string>
      against: <string> | list[<string>]   # Rule set 評価コンテキストの変数参照
      responses:
        <response_label>:
          rewind_to: <phase_id>
          action: regate | pause
      on_unknown_response: pause | fail | escalate   # 任意、default: pause (未知ラベル時)
      on_timeout: pause | fail | escalate            # 任意、default: pause (LLM 応答タイムアウト時)
      timeout_seconds: <integer>                     # 任意、default: 300

on_phase_complete:            # 任意。phase 完了時の post-hook 相当動作
  - produce_artifact:
      name: <artifact_name>
      type: file | inline
      pattern: <template_path>
      content_template: <template_string>
  - call: <rule_set_name>

on_pipeline_start:            # 任意。pipeline 開始時の動作
  - <action>

pre_pipeline_start:           # 任意。LLM 側で実行すべき前処理スキルファイル
  skill_file: <path>

tests:                        # Phase 2 で追加。Phase 1 では未サポート
  - name: <string>              # 人間可読なテスト名
    given:
      params:                   # rule set に渡す params
        <param_name>: <value>
      replay:                   # 任意。LLM 応答の固定（Recording Replay と同形式）
        validation-question:
          - { id: <slug>, result: pass | fail }     # validations[].id を参照
        classifier-question:
          - { trigger: <name>, response: <label> }  # triggers[].name を参照
    expect:
      verdict: PASS | FAIL        # 期待する最終 verdict
      checks_passed: [<name>, ...]  # 任意。通過すべき check 名
      failed_checks: [<name>, ...]  # 任意。失敗すべき check 名
```

### `max_retries` の優先順位ルール

複数箇所で `max_retries` が宣言されうる。以下の順で決定される:

1. **trigger 側**: `triggers[].max_retries` が指定されていれば最優先
2. **pipeline 側**: 未指定なら `settings.max_phase_retries` (pipeline.yml)
3. **default**: どちらも未指定なら `3`

max_retries 超過時は `triggers[].on_exhausted` の action に従う (default: `pause`、state は `aborted` 終端)。

### フィールド意味論

- **`imports`**: 他の rule set への依存を宣言。flowrail はこれを再帰的に resolve する。循環は lint で検出
- **`params`**: 呼び出し元（pipeline or 別 rule set）から値を受け取るインターフェース。型は最小限（flowrail は複雑な型システムを持たない）
- **`checks`** vs **`uses`**: 
  - `checks`: 機械的 verification（primitive 呼び出し or 他 rule set の call）
  - `uses`: 複合的な呼び出しで、主に recipe 層で複数の primitive rule set を組み合わせる時に使う
  - 実質的には両者とも「rule set の呼び出し」を表現するが、意味論的に分ける（`checks` は短命の評価、`uses` は責務の委譲）
- **`when`**: 条件付き実行。template expression で bool 値に評価。false なら skip
- **`for-each`**: 反復実行。template expression が list に評価され、各要素を変数として bind
- **`triggers`**: regate の発火条件。flowrail は `flowrail run step` 実行時 (verify 完了後) にこれを評価し、action (regate / pause / complete) を自動実行
- **`classifier`**: LLM に分類を依頼する特殊な trigger action。Failure Router の代替
- **`on_phase_complete`**: phase 完了時に実行される動作。主に phase summary artifact の生成などに使う
- **`tests`**: この rule set 自体の自己テスト宣言。`flowrail pipeline test` で一括実行。各テストは `given` で params と replay 応答を指定し、`expect` で期待 verdict と通過/失敗 check を指定する。rule set 作者が自分の rule set の期待振る舞いを実行可能な形で担保するための最小限の DSL

### Template Expression

Rule set の `{{ ... }}` は minijinja 互換の template。以下をサポート：

- `{{ param_name }}` — parameter への参照
- `{{ artifact.spec_file }}` — artifact への参照（resolved_path を返す）
- `{{ phase.id }}` — 現在 phase の ID
- `{{ list | length }}` — フィルター
- `{{ list | join ',' }}` — フィルター
- `{{ list | json }}` — JSON シリアライズ
- `{% if condition %}...{% endif %}` — 条件分岐
- `{% for item in list %}...{% endfor %}` — 反復

## Rule Set Resolution & Evaluation

### 解決フェーズ（flowrail pipeline lint / flowrail run init 時）

1. **Parse**: pipeline.yml と全 import 先 rule set を YAML パース
2. **Schema 検証**: 各 rule set が `rule-set.schema.json` に適合するか
3. **Import 解決**: `imports` を再帰的に resolve、循環検出
4. **Parameter 型検証**: `uses` 呼び出しで渡される params が被呼び出し側の `params` スキーマと合致するか
5. **Template 静的解析**: `{{ ... }}` 内の参照が resolvable か（未定義 param の検出）
6. **Dependency graph 構築**: artifact の produced_by/consumed_by から位相順序を計算、循環検出

### 評価フェーズ（flowrail run verify / flowrail run step 時）

1. **Parameter binding**: pipeline.yml の `uses` で渡された値を rule set の params に束縛
2. **Template expansion**: `{{ ... }}` を現在の context で展開
3. **`when` 評価**: false なら skip
4. **`for-each` 展開**: list の各要素を順次評価
5. **`checks` 実行**: primitive 呼び出し or 子 rule set 呼び出し
6. **結果の集約**: 全 checks の結果を verification_results として state.json に記録

### 評価コンテキスト

Rule set 評価時に利用可能な context 変数：

```
{
  "pipeline": { "name", "version", "flags" },
  "phase": {
    "id", "status", "started_at", "attempt",
    "verdict"         # "PASS" | "FAIL" | null。verification + validation 結果から導出
  },
  "artifact": { "<name>": { "status", "resolved_path", ... } },
  "param": { "<param_name>": <value> },

  # --- 以下、built-in context keys (against で参照可能) ---
  "user_request": <string>,    # パイプライン起動時のタスク記述
  "codebase": <marker>,        # codebase 全体を指すマーカー (LLM が解釈)
  "test_results": {            # 直前の verification の結果サマリ
    "passed_checks": [<name>, ...],
    "failed_checks": [<name>, ...],
    "total": <integer>
  },
  "fail_details": {            # 直前の失敗詳細 (trigger 発火時のみ有効)
    "failed_checks": [ { "primitive": <name>, "stderr": <string>, "exit_code": <integer> }, ... ],
    "rule_set": <name>
  },
  "findings": [                # review フェーズで集約された指摘
    { "category": <string>, "severity": <string>, "description": <string> }, ...
  ],
  "validation_results": {      # question_id 単位の結果
    "<id>": { "result": "pass|fail|skip", "evidence": <string> }
  }
}
```

**`phase.verdict` の導出ルール**:
- `verification.check` が全て pass + `validation_results` が全て pass → `"PASS"`
- いずれか fail → `"FAIL"`
- phase が `verifying` 未達 → `null`

## 4 Core Primitive Checks

flowrail core に組み込まれる決定論的な check type。全ての高次 check はこれらの組み合わせで実現される。

### `file_exists`

```yaml
primitive: file_exists
args:
  path: <string>          # ファイルパスまたは glob
  min_count: <integer>    # 任意、default: 1。glob の場合の最小マッチ数
  max_count: <integer>    # 任意、default: null（上限なし）
```

**実装**: `std::fs::metadata` + `glob::glob`

**戻り値**: `{ result: "pass" | "fail", matched_paths: [...], count: N }`

### `cmd_exit`

```yaml
primitive: cmd_exit
args:
  command: <string>            # sh -c で実行
  expected: <integer>          # 期待 exit code、default: 0
  timeout_seconds: <integer>   # 任意、default: 300
  capture_output: <bool>       # 任意、default: true
```

**実装**: `std::process::Command` with `sh -c`

**戻り値**: `{ result: "pass" | "fail", exit_code: N, stdout_bytes: N, stderr: "<truncated>" }`

**用途**: tests_pass, build_passes, lint_clean 等のプロジェクト固有チェックを rule set で組み合わせる時のベース

### `regex_match`

```yaml
primitive: regex_match
args:
  file: <string>              # 対象ファイルパス
  pattern: <string>           # 正規表現（Rust `regex` crate 互換）
  must_match: <bool>          # 任意、default: true。false なら "マッチしないこと" を検証
  min_matches: <integer>      # 任意、default: 1
```

**実装**: `regex` crate + `std::fs::read_to_string`

**戻り値**: `{ result: "pass" | "fail", match_count: N, matched_lines: [...] }`

**用途**: sections_present（Markdown ヘッダ検出）、特定 import の存在確認、禁止語検出など

### `git_status`

```yaml
primitive: git_status
args:
  mode: <string>              # "clean" | "committed" | "no-conflicts" | "has-changes"
  path: <string>              # 任意。特定パスに限定
```

**実装**: `std::process::Command` で `git status --porcelain` / `git ls-files` を直接呼び出し

**戻り値**: `{ result: "pass" | "fail", details: { untracked: [...], modified: [...], ... } }`

**用途**: git_committed, no_conflicts, no_uncommitted_changes 等

### 設計原則

- **副作用なし**: primitive は read-only。ファイル書き込み・コミット・ネットワークアクセスなし
- **決定論的**: 同じ入力 → 同じ出力（`deterministic mode` で時刻・UUID を固定化すればバイト一致）
- **高速**: 10ms 以下を目標（典型的なファイルアクセス）
- **自己完結**: 外部 crate 依存は最小限（glob, regex のみ）

## Standard Rule Sets Catalog

flowrail プロジェクトが標準で同梱する rule set の一覧。

### Primitive Rule Sets（~15 個）

| 名前 | 責務 | 使用 core primitive |
|------|------|---------------------|
| `check-file-exists` | ファイル/glob 存在確認 | file_exists |
| `check-command` | 任意コマンドの exit code | cmd_exit |
| `check-regex` | ファイル内容の regex マッチ | regex_match |
| `check-git-status` | git 状態（clean, committed, no-conflicts） | git_status |
| `validation-question` | LLM への単一質問 | - (pure 宣言) |
| `regate-action` | regate trigger の基本宣言 | - (pure 宣言) |
| `produce-file-artifact` | ファイル型 artifact の生成宣言 | - (pure 宣言) |
| `produce-inline-artifact` | inline 型 artifact の生成宣言 | - (pure 宣言) |
| `hook-command` | hook lifecycle event に外部コマンド登録 | - (pure 宣言) |
| `phase-confirm-before` | phase 開始前の PAUSE 宣言 | - (pure 宣言) |
| `phase-confirm-after` | phase 完了後の PAUSE 宣言 | - (pure 宣言) |
| `phase-skip-unless-flag` | フラグ条件付き skip 宣言 | - (pure 宣言) |
| `template-file-content` | template 展開で artifact の content 生成 | - |
| `classifier-question` | LLM に分類質問を出す | - |
| `for-each-item` | list 反復の糖衣 | - |

### Recipe Rule Sets（~10 個、標準 catalog）

| 名前 | 責務 | import する primitive |
|------|------|----------------------|
| `check-git-committed` | ファイルが git に commit 済み | check-git-status, check-command |
| `check-sections-present` | Markdown のセクション存在 | check-regex (+ for-each) |
| `check-build-passes` | プロジェクト build コマンドが成功 | check-command |
| `check-tests-pass` | プロジェクト test コマンドが成功 | check-command |
| `check-lint-clean` | プロジェクト lint コマンドが clean | check-command |
| `audit-gate` | 成果物の機械的・意味的検証ゲート | check-file-exists, check-git-committed, check-sections-present, validation-question |
| `regate-on-test-fail` | テスト失敗時の regate 戦略 | regate-action |
| `regate-on-review-findings` | レビュー findings 時の regate | regate-action |
| `regate-router` | 失敗原因の LLM 分類 → 適切な rewind | classifier-question, regate-action |
| `phase-summary-yaml` | phase 完了時に構造化サマリー YAML 生成 | template-file-content, produce-file-artifact |

ユーザーはこれらをコピーして改変したり、独自 recipe を追加できる。

## Inner Loop as Sub-Phases

### 現行 Inner Loop との対応

現行 `workflow-engine/modules/inner-loop.md` は Execute フェーズ内の sub-step state machine（Impl → TestEnrich → Verify + Failure Router）。新アーキテクチャでは **3 つの通常 phase** として展開する。

### 展開後の phase 構成

```yaml
phases:
  - id: execute_impl
    confirm: null  # auto
    phase_file: phases/execute-impl.md
    # LLM が feature-implementer x N タスクで TDD 実装

  - id: execute_test_enrich
    confirm: null
    phase_file: phases/execute-test-enrich.md
    # LLM が トレーサビリティ → ギャップ分析 → テスト拡充

  - id: execute_verify
    confirm: null
    uses:
      - audit-gate:
          artifact_path: "{{ artifact.code_changes }}"
          require_committed: false
          validation_questions:
            - { text: "設計→計画→実装の変換が意味的に正しいか", against: [spec_file, implementation_plan] }
      - check-tests-pass
      - check-lint-clean
      - check-build-passes

uses:
  - regate-router:
      trigger: verify_failure
      on_impl_bug: execute_impl
      on_test_defect: execute_test_enrich
      on_ambiguous_requirement: pause
```

### Failure Router の rule set 化

現行 Failure Router（テスト失敗 → 実装バグ/テスト不備/要件曖昧の分類）は `regate-router` recipe として表現：

```yaml
# rules/recipes/regate-router.yml
kind: rule-set
name: regate-router
params:
  trigger: { type: string, required: true }
  on_impl_bug: { type: string, required: true }
  on_test_defect: { type: string, required: true }
  on_ambiguous_requirement: { type: string, default: "pause" }
triggers:
  - name: "{{ trigger }}"
    condition: "phase.verdict == fail"
    action: classify_then_regate
    classifier:
      - question: "失敗原因は『実装バグ』『テスト不備』『要件曖昧』のどれか？"
        against: [test_results, fail_details]
        responses:
          implementation_bug:
            rewind_to: "{{ on_impl_bug }}"
          test_defect:
            rewind_to: "{{ on_test_defect }}"
          ambiguous_requirement:
            action: "{{ on_ambiguous_requirement }}"
```

### `flowrail run step` による triggers 自動評価

`regate-router` recipe 側で宣言された `triggers:` は、LLM が `flowrail run step` を呼ぶたびに flowrail core が自動評価する。verify が失敗した phase では、step 実行時に以下の流れで動作する:

1. flowrail が現 phase の `triggers:` を順次評価
2. `condition` を満たす trigger の `action` を実行:
   - `regate` → 指定 `rewind_to` に state を巻き戻し
   - `classify_then_regate` → `classifier` 質問を LLM に投げ、応答に従って rewind
   - `pause` → state を `paused` にして step を終了（LLM へ escalation）
   - `complete` → 通常の前進
3. 該当 trigger がなければ通常の前進

flowrail core に「regate サブコマンド」は存在しない。回復戦略は全て rule set の `triggers:` で宣言され、`flowrail run step` が一元的に処理する。

### Resume セマンティクス

現行の `current_substep` ベース resume は不要。各 sub-phase は通常の phase なので、`flowrail snapshot restore <label>` (または `flowrail run next` による直接再開) が自動的に中断位置から再開する。

### 利点

- flowrail core が sub-step 概念を持たなくて済む
- regate の rewind_to が sub-phase 単位で指定可能
- 他パイプラインで inner-loop が不要なら sub-phase を省略するだけ
- 現行の `inner-loop.md`（219 行）は廃止

## State Model

### State ファイルの配置

```
{project-root}/.flowrail/{pipeline}-{branch}.state.json       # 実行状態 (SSOT)
{project-root}/.flowrail/snapshots/<label>.json               # 独立 snapshot (checkpoint)
{project-root}/.flowrail/snapshots/<label>.meta.json          # snapshot メタデータ
```

`.flowrail/` は `.gitignore` に追加。

**snapshot の独立性**: snapshot は state に対する操作ではなく、独立したリソースとして扱われる (kubectl 等の慣習に準拠)。state.json は現在の実行状態の SSOT であり、snapshot は任意時点での state の不変コピー。snapshot は独自のライフサイクル (create → list → restore → prune) を持ち、`flowrail snapshot <verb>` でのみ操作される。

### State Schema

```json
{
  "$schema": "flowrail-state.schema.json",
  "pipeline": "feature-dev",
  "pipeline_path": "claude/skills/feature-dev/pipeline.yml",
  "version": 4,
  "branch": "feat/add-auth",
  "workspace": {
    "root": "/path/to/repo",
    "worktree_path": "/path/to/worktree",
    "is_worktree": true
  },
  "flags": ["--linear", "--doc"],
  "started_at": "2026-04-05T10:00:00Z",
  "updated_at": "2026-04-05T12:30:00Z",
  "session": 1,
  "current_phase": "execute_impl",
  "resolved_imports": [
    "rules/recipes/audit-gate.yml",
    "rules/recipes/regate-on-test-fail.yml",
    "..."
  ],

  "phases": {
    "design": {
      "status": "completed",
      "started_at": "...",
      "completed_at": "...",
      "attempt": 1,
      "verdict": "PASS",
      "verification_results": [
        {
          "rule_set": "audit-gate",
          "checks": [
            { "primitive": "file_exists", "result": "pass", "args": {...}, "details": {...} },
            { "primitive": "regex_match", "result": "pass", "args": {...}, "details": {...} }
          ]
        }
      ],
      "validation_record": [
        { "id": "req-capture", "question": "ユーザー要求を全て要件として捕捉しているか", "result": "pass", "evidence": "全要件が ...", "recorded_at": "..." }
      ],
      "concerns": [                             // 後続フェーズへの伝播用 (現行 phase-summary.md 互換)
        { "target_phase": "spec-review", "content": "...", "recorded_at": "..." }
      ],
      "directives": [
        { "target_phase": "spec-review", "content": "...", "recorded_at": "..." }
      ]
    },
    "execute_impl": {
      "status": "running",
      "started_at": "...",
      "attempt": 2,
      "verdict": null,
      "inner_loop_state": {                     // execute 系 phase のみ。resume 時に使用
        "completed_tasks": ["T1", "T2"],
        "remaining_tasks": ["T3", "T4", "T5"],
        "last_commit": "591c21c",
        "failure_history": []
      }
    },
    "execute_verify": {
      "status": "paused",                        // 新規: paused 状態 (classifier 質問未解決)
      "pending_classifier": {                    // paused 時のみ有効
        "trigger_name": "verify_failure",
        "question": "失敗原因は『実装バグ』『テスト不備』『要件曖昧』のどれか？",
        "against": ["test_results", "fail_details"],
        "bound_params": {...},
        "asked_at": "..."
      }
    },
    "aborted_phase_example": {
      "status": "aborted",                       // 新規: aborted 終端 (max_retries 超過)
      "abort_reason": "max_retries_exhausted",
      "aborted_at": "..."
    }
  },

  "artifacts": {
    "spec_file": {
      "status": "verified",
      "resolved_path": "docs/superpowers/specs/2026-04-05-add-auth-design.md",
      "verified_at": "...",
      "verification_hash": "sha256:..."
    },
    "code_changes": {
      "status": "pending"
    }
  },

  "trigger_history": [
    {
      "trigger": "verify_failure",
      "rule_set": "regate-router",
      "from_phase": "execute_verify",
      "to_phase": "execute_impl",
      "attempt": 1,
      "action": "classify_then_regate",
      "classifier_result": "implementation_bug",
      "at": "..."
    }
  ],

  "snapshot_history": [
    {
      "label": "pre-refactor",
      "session": 1,
      "phase": "execute_impl",
      "path": ".flowrail/snapshots/pre-refactor.json",
      "at": "...",
      "reason": "manual_checkpoint"
    }
  ],

  "hook_history": [
    {
      "event": "on_phase_complete",
      "phase": "design",
      "command": "linear-sync phase",
      "exit": 0,
      "duration_ms": 1234,
      "at": "..."
    }
  ],

  "integrations": {                             // integration-scoped persisted fields
    "linear": {                                 // linear-sync が使用 (hook では書き戻せないため state schema 側で持つ)
      "ticket_id": "CLA-5",
      "document_id": "workflow-report-cla-5-b50b8db30faf",
      "document_url": "https://linear.app/...",
      "last_synced_phase": "spec-review"
    }
  }
}
```

**Integration-scoped persisted fields**: Hook は fire-and-forget のため hook 結果を state に書き戻せない (原則維持)。しかし `linear_ticket_id` / `document_id` のような integration がセッション間で保持すべき値は必要。これらは `state.integrations.<integration_name>` 配下の decalared field として state schema に含める。`pre_pipeline_start` 時の `resolve_ticket` で初期値を設定し、以降は read-only 参照のみ (hook は書き戻さない)。

### Phase Lifecycle

```
pending → running → verifying → completed
                              ↘ failed → (regate) → pending (rewind先)
                              ↘ paused (classifier 質問未解決 or autonomy PAUSE)
                                   ↓
                                 (LLM 応答 / ユーザー介入)
                                   ↓
                              → running (resume)
         skipped (skip_unless 条件不一致)
         aborted (max_retries 超過、on_exhausted: pause|fail|complete に従い遷移)
```

- **`paused`**: classifier trigger が未解決 or autonomy の `phase-confirm` で待機中。state.json の `phases[<id>].pending_classifier` に未解決情報を保持。resume は `flowrail run step --classifier-response <label>` で (LLM 応答注入) または `flowrail run next` で (confirm 解除後)
- **`aborted`**: `max_retries` 超過時の終端状態。`phases[<id>].abort_reason` に理由記録。回復には `flowrail state reset --to-phase <id>` または `flowrail snapshot restore <label>` が必要

### Artifact Lifecycle

```
pending → produced → verified
                   ↘ failed → (regate トリガー)
```

`flowrail` は verification まで（rule set の checks 評価）。validation は LLM が行い、結果を `flowrail run step --validation-result <id>=pass|fail` で report (複数 question 対応、question_id 単位)。

## Idempotency Contract

全サブコマンドの冪等性保証。同じ状態で N 回実行しても副作用は 1 回と同等。

| コマンド | 冪等性 | 中断点からの再開挙動 | Hook 発火 |
|---------|--------|---------|-----------|
| `flowrail pipeline lint` | Pure | 副作用なし | — |
| `flowrail pipeline fmt` | Strict | 既にフォーマット済みなら no-op (mtime 不変) | — |
| `flowrail pipeline test` (Phase 2) | Pure | 副作用なし | — |
| `flowrail pipeline init` | Strict | 既存 pipeline が同名なら error exit (`--force` で上書き) | — |
| `flowrail run init` | Strict | 既存 state あり同じ pipeline/flags なら no-op。異なる場合 error exit | 初回のみ `on_pipeline_start` 発火、no-op 時は発火しない |
| `flowrail run next` | Pure | read-only、同じ state なら同じ出力 | — |
| `flowrail run verify` | Strict | 毎回 checks を実再実行。state.json への書き込みは single-write transaction | — |
| `flowrail run step` | Observable | 現 phase が既に completed なら no-op (`on_phase_complete` も再発火しない)。triggers 由来の rewind は `trigger_history` にエントリ追加。phase 遷移 + trigger 評価 + trigger_history 追記は single-write transaction、hook 発火は state commit 後の post-action (commit 前 crash → 再実行で前状態から; commit 後 hook 前 crash → hook 未発火のみ影響) | state commit 後のみ `on_phase_complete` / `on_trigger_fired` 発火 |
| `flowrail state show` | Pure | read-only | — |
| `flowrail state list` | Pure | read-only | — |
| `flowrail state diff` (Phase 2) | Pure | read-only、2 つの state を比較のみ | — |
| `flowrail state reset` | Strict | 指定 `--to-phase` 以降を pending に戻す、既にその状態なら no-op | — |
| `flowrail state prune` | Strict | 削除対象が無ければ no-op | — |
| `flowrail snapshot create` | **Same-Label Error** | 同一 label が既存の場合 error exit (kubectl 慣習)、`--force` で上書き、auto-label (timestamp) 時のみ常に新規作成 | `snapshot.created` イベント発行、no-op 時は発行しない |
| `flowrail snapshot restore` | Strict | restore 対象が指定 snapshot と同一状態なら no-op、異なれば state 置換。worktree/artifact の整合性は別途検証 | `snapshot.restored` イベント発行、no-op 時は発行しない |
| `flowrail snapshot list` | Pure | read-only | — |
| `flowrail snapshot prune` | Strict | 削除対象が無ければ no-op | — |

### 実装原則

- **Atomic Writes**: state.json への書き込みは `tempfile → fsync → rename` パターン
- **Single-Write Transaction**: `flowrail run step` は phase 遷移 + trigger 評価 + trigger_history 追記を 1 回の state.json 書き込みで完結。hook は state commit 後の post-action
- **Compare-and-Set**: state 更新前に現在の `updated_at` / hash を検証。衝突時は即 exit code 1 + `E_STATE_CONFLICT` エラー (race detection、静かに壊さない)
- **Deterministic Serialization**: JSON 出力はキー順固定、タイムスタンプはミリ秒精度固定
- **No Hidden Side Effects**: state.json / event log 以外への書き込みは行わない
- **単一プロセス前提**: 同一 `.flowrail/{pipeline}-{branch}.state.json` に対する並列 `flowrail run *` 呼び出しは undefined behavior。flock(2) 等の排他制御は Phase 2 以降の拡張候補 (現状は compare-and-set で early-fail)
- **破損 state.json の扱い**: 起動時に schema validation を実行し、失敗時は「直前の snapshot から手動復元 (`flowrail snapshot restore <label>`) を促す」エラー exit。自動修復はしない

## Hook System

flowrail の唯一の拡張機構。コアは閉じた決定論的状態機械であり、**全ての柔軟性・連携・カスタマイズは hook で提供する**。

### Lifecycle Events（完全一覧）

| イベント | 発火タイミング |
|---------|---------------|
| `on_pipeline_start` | `flowrail run init` 直後 |
| `on_phase_start` | phase の state が running に遷移した時 |
| `on_phase_complete` | phase の state が completed に遷移した時 |
| `on_phase_fail` | verification 失敗で phase の state が failed に遷移した時 |
| `on_verify_fail` | 個別の check が失敗した時 |
| `on_trigger_fired` | `flowrail run step` が rule set の `triggers:` を発火した時 (旧 `on_regate`) |
| `on_snapshot_created` | `flowrail snapshot create` 実行時 (旧 `on_handover`) |
| `on_snapshot_restored` | `flowrail snapshot restore` 実行時 |
| `on_pipeline_complete` | 全 phase completed 時 |
| `on_pipeline_abort` | max_retries 超過や手動中断時 |

### Hook Contract

**宣言:**
```yaml
integrations:
  - name: linear-sync
    enabled_by: --linear
    hooks:
      # 文字列: 単一 hook
      on_pipeline_start: "linear-sync workflow-start"
      # 文字列配列: 複数 hook (並列実行)
      on_phase_complete: ["linear-sync phase-summary", "linear-sync evidence"]
      on_trigger_fired: "linear-sync regate"
      on_snapshot_created: "linear-sync snapshot"
      on_pipeline_complete: "linear-sync complete"
    pre_pipeline_start:         # LLM 側で実行する前処理
      skill_file: "linear-sync/resolve_ticket.md"
```

**Schema:** `integrations[].hooks.<event>` は **string または list[string]** を受け付ける (jsonschema の `oneOf`)。現行 feature-dev pipeline.yml の `on_phase_complete: [sync_phase_summary, sync_evidence]` 記法との互換性を保つ。

**実行仕様:**
- **実行形式**: `sh -c "<command>"` (POSIX shell 前提、macOS + Linux のみサポート、Windows は Phase 2 以降の拡張候補)
- **環境変数**:
  - `FLOWRAIL_PIPELINE` — パイプライン名
  - `FLOWRAIL_PHASE` — 現在の phase ID
  - `FLOWRAIL_EVENT` — 発火したイベント名
  - `FLOWRAIL_STATE_FILE` — state.json の絶対パス
  - `FLOWRAIL_SESSION` — セッション番号
  - `FLOWRAIL_RUN_ID` — 現 run の UUID (Event Stream と突き合わせ可能)
  - `FLOWRAIL_ARTIFACT_<NAME>` — 該当 artifact のパス（該当する場合）
- **stdin JSON**: 構造化イベントデータが JSON で流れる（後述）、最大 1MB (超過時は state_file への参照のみ渡す)
- **実行モード**: Fire-and-forget (非同期)、ただし **state commit 後の post-action** として実行 (flowrail run step の single-write transaction 完了後)
- **タイムアウト**: 15 分 (デフォルト)、`settings.hook_timeout_seconds` で設定可能、超過時は `SIGTERM → 5 秒待機 → SIGKILL`、子プロセスは `setsid` で独立 pgid (orphan 防止)
- **失敗時**: stderr に warning を出すのみ。ワークフローは継続
- **並列実行**: 同一イベントに複数 hook があれば並列実行、各 hook の成否は `hook_history[].per_hook_results` に個別記録

### stdin JSON Schema

Hook 起動時、stdin に以下の JSON が流れる：

```json
{
  "event": "on_phase_complete",
  "timestamp": "2026-04-05T12:00:00.000Z",
  "pipeline": "feature-dev",
  "session": 1,
  "phase": {
    "id": "execute_verify",
    "verdict": "PASS",
    "started_at": "2026-04-05T11:45:00Z",
    "completed_at": "2026-04-05T12:00:00Z",
    "duration_seconds": 900,
    "attempt": 1
  },
  "verification_results": [
    {
      "rule_set": "audit-gate",
      "checks": [
        { "primitive": "file_exists", "target": "docs/specs/*.md", "result": "pass" },
        { "primitive": "cmd_exit", "target": "npm test", "result": "pass", "details": { "exit": 0, "passed": 24, "failed": 0 } }
      ]
    }
  ],
  "validation_results": [
    { "question": "ユーザー要求を全て要件として捕捉しているか", "result": "pass" }
  ],
  "artifacts_produced": [
    { "name": "code_changes", "type": "git_range", "resolved": "HEAD~3..HEAD" }
  ],
  "event_data": {},
  "state_hash": "sha256:abc123...",
  "state_file": "/abs/path/to/.flowrail/feature-dev-master.state.json"
}
```

Hook 実装側は `jq` や `python -c 'import json,sys; print(json.load(sys.stdin))'` 等で stdin をパース。

### `pre_pipeline_start` / `pre_phase_start` — LLM 側前処理

Hook は fire-and-forget なのでユーザー対話が必要な前処理（例: linear-sync の resolve_ticket）は表現できない。これらは pipeline.yml の `pre_*` フィールドで、**LLM が読み実行するスキルファイル**として宣言される：

```yaml
integrations:
  - name: linear-sync
    pre_pipeline_start:
      skill_file: "linear-sync/resolve_ticket.md"
```

flowrail はこのフィールドを `flowrail run next` の最初の呼び出しで LLM に出力に含めるだけ。実行は LLM の責務。

### non-goals

以下は **提供しない**:
- 動的ロードされるプラグイン（WASM, shared library 等）
- Hook 間の依存関係宣言
- Hook の結果を state.json に書き戻す機構
- Hook からのワークフロー操作（regate 等）

## Binary Separation (原則 8 の具体化)

flowrail は Cargo workspace として構成され、3 つの crate に分離される。

### Crate 構成

| Crate | 種別 | 役割 | Audience | Phase |
|-------|-----|------|----------|-------|
| **`flowrail-core`** | library | state machine, rule set resolver, 4 primitive checks, hook executor, 8 directives, YAML 抽象層 (`src/yaml/`) | (内部のみ、両 binary が依存) | Phase 1 |
| **`flowrail`** | binary | agent 用 CLI (`flowrail <resource> <verb>` 形式の subcommand 群)、stdin/stdout JSON protocol | LLM エージェント, CI/CD, スクリプト | Phase 1 |
| **`flowrail-tui`** | binary | 人間用 TUI CLI、ratatui-based、リアルタイム state 可視化 | 開発者・運用者 (手動) | Phase 3 |

### 依存関係フロー

```
flowrail-core (pure library)
     │
     ├──▶ flowrail (agent bin, minimal deps)
     │       └─ deps: flowrail-core + clap + miette[fancy-no-backtrace]
     │
     └──▶ flowrail-tui (human bin, UI deps isolated)
             └─ deps: flowrail-core + ratatui + crossterm + miette[fancy-no-backtrace]
```

**重要**: `flowrail` と `flowrail-tui` の間に依存関係は**存在しない**。両者は独立して `flowrail-core` を参照する。

これにより:
- `cargo build -p flowrail` で agent バイナリのみビルド → ratatui/crossterm を一切取り込まない
- `cargo tree -p flowrail` で agent 用依存グラフを独立に audit 可能
- `cargo audit` / `cargo deny` を crate 単位で適用可能
- バイナリサイズ削減 (agent CLI は TUI 依存を含まないため)

### Agent CLI と Human CLI の要求の違い

| 項目 | `flowrail` (agent) | `flowrail-tui` (human) |
|------|-------------------|----------------------|
| 重視 | security, minimal deps, determinism, scriptability | UX, interactivity, visualization |
| I/O | JSON / plain text (パイプ前提) | rich terminal UI |
| 起動 | per-command, fast startup | long-running session |
| 依存数 | 最小限 | UI 要件で必要なもの |
| 監査 | `cargo audit` は常時 green を維持 | 情報参照 (許容範囲は広い) |

### Feature Flag ではなく Crate 分離を選ぶ理由

単一 crate + feature flag (`--features tui`) で TUI を切り替える案もあり得たが**却下**した:

1. **ビルド時分離の保証**: feature flag では「間違って TUI feature を enable したまま agent ビルド」が起こり得る。別 crate なら不可能
2. **依存グラフの明確さ**: `cargo tree -p flowrail` で agent の依存を独立に確認できる (feature flag だと条件分岐を手動でトレース必要)
3. **audit の独立性**: `cargo audit` の結果を crate 単位で解釈できる
4. **MSRV の独立管理**: 将来 TUI crate だけ MSRV 1.88 に bump する等の自由度 (現状は workspace 統一)
5. **リリースサイクルの独立**: agent CLI と TUI CLI を別タイミングで release 可能

## Subcommands

### 設計ルール

Linux 哲学「Do One Thing and Do It Well」に準拠し、サブコマンドは **リソース + 動詞** の形式で統一する (kubectl / helm / AWS CLI の設計慣習)。

1. 全 subcommand は `flowrail <resource> <verb>` 形式 (`flowrail help` の独立トップレベルを除く)
2. トップレベルリソース: `pipeline` / `run` / `state` / `snapshot` / `help` の **5 つ**
3. `snapshot` は独立リソース (state に対する操作ではない、独立ライフサイクルを持つ)
4. **TUI は別バイナリ `flowrail-tui` として提供** (Cargo workspace の別 crate、Phase 3 で実装)。agent CLI (`flowrail`) の supply chain attack 面を最小化するため、TUI 依存 (ratatui, crossterm 等) を同一バイナリに混ぜない (原則 8 "Separation by Audience" 参照)
4. 動詞の慣習: `init` (新規リソース作成) / `create` (子リソース追加) / `restore` (外部状態から戻す) / `show` / `list` / `reset` / `prune` / `diff` / `test` は kubectl 準拠
5. 引数名は slug-case + long form (`--rule-set`, `--to-phase`, `--validation-result`, `--classifier-response`)
6. `flowrail help` は独立トップレベル (man page 的役割)

### 全コマンド一覧

```bash
# ========================================
# Phase 1: Core subcommands (MVP)
# ========================================

# Pipeline リソース — pipeline.yml と rule set の静的ツーリング
flowrail pipeline lint [path...]
flowrail pipeline fmt  [path...] [--check|--diff]
flowrail pipeline init <name> --template <template>     # Phase 4 で実装

# Run リソース — パイプラインの駆動
flowrail run init   --pipeline <path> [<pipeline-declared flag>...]
               [--dry-run] [--deterministic]
flowrail run next   [--format md|json] [--dry-run]
flowrail run verify [--rule-set <name>] [--report <check>=pass|fail]
               [--dry-run] [--deterministic]
flowrail run step   [--validation-result <id>=pass|fail|skip]... # repeatable
               [--validation-result-file <path>]             # JSON 配列から読む
               [--classifier-response <label>]
               [--dry-run] [--deterministic]

# State リソース — 実行状態の観察・整理
flowrail state show  [--format md|json]
flowrail state list
flowrail state reset --to-phase <id>
flowrail state prune [--completed]

# Snapshot リソース — 独立 checkpoint (独立ライフサイクル)
flowrail snapshot create  [--label <name>] [--force]  # same-label は error, --force で上書き
flowrail snapshot restore <label>
flowrail snapshot list
flowrail snapshot prune   [--older-than <days>] [--all]

# TUI / Help
flowrail tui                                            # Phase 3 で実装
flowrail help [<resource>] [<verb>]

# ========================================
# Phase 2: Testing Framework (延期)
# ========================================

# Phase 2 で追加される Testing 系コマンド / フラグ
flowrail pipeline test [path...] [--filter <pattern>]   # Phase 2: rule set の tests: 実行
flowrail state diff    <a> <b> [--ignore <fields>]       # Phase 2: state.json 差分比較

# Phase 2 で追加される run サブコマンドのフラグ
flowrail run verify --replay-from <path>                 # Phase 2: recording replay
flowrail run step   --record <path>                      # Phase 2: 1 run を JSONL 記録
               --replay-from <path>                 # Phase 2: YAML scenario or recording.jsonl
               --assert-recording <baseline>        # Phase 2: baseline と比較
               --assert-level strict|loose          # Phase 2: 比較レベル
```

**グローバルフラグ (全 subcommand で有効):**
- `--deterministic`: Deterministic Mode 有効化 (Phase 1 Infrastructure)
- `FLOWRAIL_EVENTS_FILE=<path>`: Event Stream 出力先 (環境変数、Phase 1 Infrastructure)
- `FLOWRAIL_NOW=<iso8601>`, `FLOWRAIL_SEED=<seed>`: Deterministic Mode 用環境変数

---

### Pipeline リソース

#### `flowrail pipeline lint [path...]`

pipeline.yml + rule set のスキーマ + セマンティック検証。

**検証レイヤー:**

1. **構文検証** — YAML パース可能か
2. **スキーマ検証** — `pipeline.schema.json` / `rule-set.schema.json` (Draft 2020-12) に適合
3. **セマンティック検証:**
   - `imports` が実在するファイルを指すか
   - `imports` に循環がないか
   - `uses` で呼ばれる rule set が import 済みか
   - `params` の型と `uses` で渡される値の整合性
   - `{{ ... }}` template 内の参照が resolvable か
   - `produced_by` / `consumed_by` が phases に存在するか
   - artifact の位相順序に矛盾がないか
   - `triggers.rewind_to` / `regate-router.on_*` が有効な phase を指すか
   - `integrations.hooks` のキーが有効な lifecycle event か (`on_trigger_fired`, `on_snapshot_created` 等)

**出力例:**

```
$ flowrail pipeline lint
✗ claude/skills/feature-dev/pipeline.yml
  error[E001]: unknown rule set in `uses`
    --> claude/skills/feature-dev/pipeline.yml:42:5
     |
  42 |   - audit-gate-typo:
     |     ^^^^^^^^^^^^^^^ not found in imports
    help: did you mean "audit-gate"?

  error[E002]: circular import detected
    --> rules/recipes/foo.yml imports rules/recipes/bar.yml imports rules/recipes/foo.yml

  warn[W001]: parameter "require_committed" is declared but never used

  2 errors, 1 warning
```

終了コード: `0` = clean, `1` = warnings, `2` = errors

#### `flowrail pipeline fmt [path...] [--check|--diff]`

YAML 正規化フォーマット。rule set と pipeline 両方に適用。

**正規化ルール:**
- **rule-set.yml のキー順**: `kind → layer → name → version → description → imports → params → settings → checks → validations → uses → triggers → on_phase_complete → on_pipeline_start → pre_pipeline_start → tests`
- **pipeline.yml のキー順**: `kind → name → version → description → imports → flags → settings → artifacts → phases → uses → triggers → integrations → pre_pipeline_start → on_pipeline_start → on_pipeline_complete`
- phases / artifacts / imports: 配列順保持
- インデント: 2 spaces
- 末尾改行: あり
- 文字列引用: 必要時のみ (`true` / `null` / 数値文字列 / 特殊文字含む場合は quote)

**フラグ:**
- `--check`: フォーマット済みか確認のみ（CI 用）
- `--diff`: 差分表示

#### `flowrail pipeline test [path...] [--filter <pattern>]` — **Phase 2**

> **Phase 2 で実装**。Phase 1 では未提供。

rule set に宣言された `tests:` セクションを一括実行。

**動作:**
1. 指定パス配下 (デフォルトは `rules/`) を再帰的に走査し、`tests:` を宣言している rule set を収集
2. 各 test について `given.params` + `given.replay` で rule set を評価 (実 LLM は呼ばない)
3. 最終 verdict + 通過/失敗 check を `expect` と照合
4. 結果をサマリ出力

**フラグ:**
- `--filter <pattern>`: テスト名の glob フィルタ
- `--format md|json`: 出力形式

**出力例:**

```
$ flowrail pipeline test rules/recipes/audit-gate.yml
PASS  rules/recipes/audit-gate.yml::valid spec file passes
FAIL  rules/recipes/audit-gate.yml::missing sections fails
       expected: verdict=FAIL, failed_checks=[check-sections-present]
       actual:   verdict=PASS, failed_checks=[]

1 passed, 1 failed (2 total)
```

終了コード: `0` = all passed, `1` = any failure

#### `flowrail pipeline init <name> --template <template>` — **Phase 4**

新パイプラインのスキャフォールド（Phase 4 で実装）。

---

### Run リソース

#### `flowrail run init --pipeline <path> [<pipeline-declared flag>...]`

パイプライン実行開始。pipeline.yml を lint し、state.json を初期化。`on_pipeline_start` hook 発火。pipeline.yml の `flags:` セクションで宣言されたフラグ（例: `--linear`、`--accept`、`--iterations 5`）を直接指定できる。

#### `flowrail run next [--format md|json]`

次に実行すべき phase の情報を出力（rule set 評価結果を含む）。read-only。

```json
{
  "phase": {
    "id": "design",
    "phase_file": "phases/design.md",
    "confirm": "after"
  },
  "requires_artifacts": [],
  "produces_artifacts": [
    { "name": "spec_file", "type": "file", "pattern": "docs/superpowers/specs/{date}-*-design.md" }
  ],
  "rule_sets_applied": [
    {
      "name": "audit-gate",
      "bound_params": { "artifact_path": "docs/superpowers/specs/{date}-*-design.md", "required_sections": ["requirements", "components"] },
      "checks_planned": [
        { "primitive": "file_exists", "target": "..." },
        { "primitive": "regex_match", "target": "...", "pattern": "^## requirements" }
      ],
      "validations_planned": [
        { "question": "ユーザーの要求を全て要件として捕捉しているか", "against": "user_request" }
      ]
    }
  ],
  "pre_phase_skills": []
}
```

#### `flowrail run verify [--rule-set <name>] [--report <check>=pass|fail]`

現 phase の rule set を評価、checks を実行。

**実装する評価ロジック:**
- import 解決済みの rule set をツリー順に評価
- 各 check は primitive を呼び出し or 子 rule set の call
- `when` / `for-each` を評価
- 結果を verification_results として state.json に記録

`--report <check>=pass|fail` は LLM 側で外部検証した結果を flowrail の verification 結果に注入するためのオプション。

#### `flowrail run step [options]`

状態機械を 1 ステップ進める。verify 通過後の phase 完了、triggers の自動評価、次 phase への遷移 or regate/pause/complete の実行を一元的に処理する。

**Phase 1 オプション:**

| オプション | 用途 |
|----------|------|
| `--validation-result <id>=pass\|fail\|skip` (repeatable) | LLM による意味的検証結果の報告 (question_id 単位、複数指定可) |
| `--validation-result-file <path>` | JSON 配列ファイルから validation 結果を読み込み (多数の question 向け) |
| `--classifier-response <label>` | classifier trigger への LLM 応答 |
| `--dry-run` | 予定のみ出力、state 書き込みなし (Phase 1 Infrastructure) |
| `--deterministic` | Deterministic Mode (Phase 1 Infrastructure) |

**Phase 2 オプション (Testing Framework):**

| オプション | 用途 |
|----------|------|
| `--record <path>` | この 1 run の全 event + LLM 応答を JSONL に記録 |
| `--replay-from <path>` | YAML scenario または recording.jsonl から応答を replay |
| `--assert-recording <baseline>` | baseline recording と比較、差分があれば exit 1 |
| `--assert-level strict\|loose` | strict = byte-identical, loose = 意味的等価 |

**Stdin JSON input (Phase 1)**: 複数 validation の結果を stdin から JSON で受け取る:
```bash
echo '{"validations":[{"id":"req-capture","result":"pass"},{"id":"test-coverage","result":"fail","evidence":"3 questions missing"}]}' | flowrail run step
```

`--validation-result`, `--validation-result-file`, stdin JSON は排他。同時指定はエラー。

**動作:**
1. 現 phase を `completed` に遷移 (または `failed` のまま triggers 評価へ)
2. `integrations.hooks.on_phase_complete` / `on_phase_fail` 発火
3. 現 phase の `triggers:` (rule set 由来) を順次評価
4. 該当 trigger の `action` を実行:
   - `regate` → `rewind_to` に state 巻き戻し、`on_trigger_fired` hook 発火、`trigger_history` に追記
   - `classify_then_regate` → classifier LLM 質問が未解決なら `paused` + classifier 質問を next に埋め込み、解決済みなら応答に従い rewind
   - `pause` → `paused` 状態で step 終了 (LLM へ escalation)
   - `complete` → 次 phase へ遷移
5. 発火 trigger がなければ次 phase へ通常遷移、全 phase 完了なら `on_pipeline_complete` 発火

**冪等性:** 既に `completed` の phase に対する step 呼び出しは no-op。triggers 由来の巻き戻しは `trigger_history` にエントリを追加するため Observable。

---

### State リソース

#### `flowrail state show [--format md|json]`

```
Pipeline: feature-dev (branch: feat/add-auth)
Session: 1 | Started: 2026-04-05T10:00:00Z

Phases:
  ✓ design               completed  10:00 - 10:45
  ✓ spec-review          completed  10:45 - 11:00
  ✓ plan                 completed  11:00 - 11:30
  ✓ plan-review          completed  11:30 - 11:45
  ▶ execute_impl         running    11:45 -
  · execute_test_enrich  pending
  · execute_verify       pending
  - accept-test          skipped    (--accept not set)
  · doc-audit            pending
  · review               pending
  · integrate            pending

Artifacts:
  ✓ spec_file            verified
  ✓ implementation_plan  verified
  · code_changes         pending

Triggers fired: 1 (verify_failure@execute_verify → execute_impl [impl_bug])
Snapshots: 2 (pre-refactor, mid-impl)
```

#### `flowrail state list`

現プロジェクト配下の state ファイル (各 pipeline × 各 branch) を一覧表示。

#### `flowrail state diff <a> <b> [--ignore <fields>]` — **Phase 2**

> **Phase 2 で実装**。Phase 1 では未提供。Testing Framework の 4 番目 primitive。

2 つの state.json を比較。

- Deterministic mode 下では byte-identical 比較
- そうでない場合は semantic diff (phase 遷移差、artifact 差、trigger 履歴差)
- `--ignore timestamps,durations,uuids` で無視フィールド指定可能
- 出力は unified diff 風または JSON

**用途:** baseline recording と current recording の比較、異なるセッション間の整合性確認。

#### `flowrail state reset --to-phase <id>`

指定 phase 以降を `pending` に戻す。引数 `--to-phase` は**必須** (scope を明示する設計原則)。既にその状態なら no-op。

#### `flowrail state prune [--completed]`

完了済みの古い state エントリを掃除する。`--completed` フラグで完了済みの過去 state を一括削除。冪等。

---

### Snapshot リソース

`snapshot` は state に対する操作ではなく独立したリソース (独立ライフサイクルを持つ)。state.json は現在の実行状態の SSOT、snapshot は任意時点での state の不変コピー。保存先は `.flowrail/snapshots/<label>.json`。

#### `flowrail snapshot create [--label <name>] [--force]`

現在の state を checkpoint として独立ファイルに保存。session インクリメント。`on_snapshot_created` hook 発火。`snapshot_history` にエントリ追加。

**Label semantics:**
- **label 明示時**: 同一 label の snapshot が既存の場合は **error exit** (kubectl / git branch create 慣習)。silently 上書きしない。`--force` フラグで明示的に上書き可能。
- **label 省略時**: `snapshot-<ISO8601>` 形式の自動生成 (常に一意、常に新規作成)。

不変 checkpoint としての信頼性を担保するため、silently 上書きは避ける設計。

#### `flowrail snapshot restore <label>`

指定 snapshot を現在の state として復元。`.flowrail/snapshots/<label>.json` を読み込み、artifact の存在を再検証。次の `flowrail run next` で中断 phase から再開。`on_snapshot_restored` hook 発火。

#### `flowrail snapshot list`

保存済み snapshot の一覧 (label、作成時刻、session、phase)。read-only。

#### `flowrail snapshot prune [--older-than <days>] [--all]`

古い snapshot の選別削除。`--older-than N` で N 日以上前の snapshot を削除、`--all` で全 snapshot 削除。冪等。

---

### `flowrail-tui` (別バイナリ、Phase 3)

TUI は **`flowrail` とは別バイナリ `flowrail-tui`** として提供される (Cargo workspace の別 crate、Phase 3 で実装)。`flowrail tui` というサブコマンドは**存在しない**。

ratatui ベースのインタラクティブ TUI で、state.json を watch してリアルタイムに phase / artifact / trigger の状態を表示する。ratatui + crossterm 等の UI 依存は `flowrail-tui` crate のみに局在し、`flowrail` (agent CLI) の依存グラフには一切含まれない。

詳細は "Binary Separation" セクションおよび原則 8 "Separation by Audience" を参照。

---

### `flowrail help [<resource>] [<verb>]`

独立トップレベルコマンド。man page 的な役割を持つ。

- `flowrail help` — 全コマンドの概要
- `flowrail help run` — `run` リソース配下の全 verb の説明
- `flowrail help run step` — `flowrail run step` の詳細 (オプション、動作、用例)
- `flowrail help snapshot create` — 個別 verb の詳細

clap の derive API + 静的生成の man page コンテンツを利用。Testing Framework 関連オプションの用例もここに集約される。

## 設計判断と代替案 (Alternatives Considered)

高影響な設計判断について代替案・選定理由・トレードオフを記録する。詳細な brainstorming 履歴は **[Linear CLA-19](https://linear.app/neko-neko/issue/CLA-19/flowrail-cli-rule-set-architecture-brainstorming-決定事項-2026-04-05)** 本文 + [comment-9fbad1d8](https://linear.app/neko-neko/issue/CLA-19#comment-9fbad1d8) を参照。ここでは要点のみ記載。

### 1. Approach C 採用 (最小 core + rule set 拡張 + Testing Framework)

| 代替案 | 概要 | 捨てた理由 |
|--------|------|-----------|
| **Approach A** (超ミニマリスト、~1,500 LOC) | 4 primitive + state machine のみ、rule set は外部ツール | 拡張性と標準 catalog の欠如、rule set の評価エンジンが存在しないと pipeline.yml を書いても実行できない |
| **Approach B** (primitive 内蔵、~3,000 LOC) | 10 種類の高次 check (sections_present, tests_pass, build_passes 等) を flowrail core に内蔵 | Linux 哲学違反、check 追加のたびに flowrail release が必要、OSS 化時の拡張性欠如 |
| **Approach C** ✓ (Phase 1: ~3,500-3,800 LOC, Phase 2 完了: ~4,400 LOC) | 最小 core + rule set resolver + 4 primitive + Phase 2 で Testing Framework | 決定論性と拡張性のバランス、rule set で高次機能を追加可能、標準 recipe catalog 同梱 |

**選定理由**: 決定論性 (Approach A 超え) と拡張性 (Approach B 超え) のバランス。rule set による拡張で flowrail release に依存せず機能追加が可能。

### 2. 言語: Rust

| 代替案 | 捨てた理由 |
|--------|-----------|
| Go | `enum + match` による state machine 網羅性保証が弱い (sum type 非ネイティブ)、バイナリサイズは Rust と同程度 |
| TypeScript (Node.js / Deno / Bun) | 起動速度 (~100ms) が LLM ループ用途で致命的、依存ランタイム |
| Python | 起動速度、依存ランタイム、型安全性不足 |

**選定理由**: 起動速度 ~1ms、`enum + match` の網羅性保証、シングルバイナリ配布、LLM ワークフロー駆動 CLI に最適。

### 3. Rule set を YAML で表現

| 代替案 | 捨てた理由 |
|--------|-----------|
| Lua / Starlark / Rhai | 実行サンドボックスが必要 (DoS 対策)、LLM への可読性低下、静的解析困難 |
| Dhall | 学習曲線、実装負荷、YAML 比でエコシステム弱 |
| TOML | 階層構造の表現力不足、複数文書 (anchors, merge keys) 非対応 |

**選定理由**: LLM と人間の両方に可読、静的解析容易、エコシステム成熟、Jinja2 風 template (minijinja) で動的値も表現可能。DoS 対策は serde-saphyr の budget control で担保。

### 4. Testing Framework を flowrail core に含める (Phase 2)

| 代替案 | 捨てた理由 |
|--------|-----------|
| 外部 CLI `flowrail-test` として分離 | flowrail との data model 共有コスト、CLI 分断によるユーザー混乱 |
| Phase 2 以降も延期、外部シェルスクリプトで代替 | rule set 作者の品質保証が弱い、LLM 揺らぎ検知の標準化不能 |
| **Phase 2 で 5 primitives を flowrail core に含める** ✓ | rule set 作者の品質保証を標準化、N 回実行・統計・可視化は外部ツールに委譲 (原則 7 維持) |

**選定理由**: Phase 1 では最小限 (5 concepts only)、Phase 2 で Testing Framework を追加。原則 7 の「複雑な assertion DSL は non-goal」を維持しつつ、rule set 作者の基本的な品質保証ニーズをカバー。

### 5. `flowrail snapshot create` の冪等性: Same-Label Error (Last-Write-Wins ではなく)

| 代替案 | 捨てた理由 |
|--------|-----------|
| **Last-Write-Wins** (同名 label を silently 上書き) | 不変 checkpoint と位置付けと矛盾、rewind 先として信頼不能、handover 用途で過去 state を silently 失う |
| **Same-Label Error + `--force`** ✓ | kubectl / git branch create 慣習準拠、事故防止、明示的な上書き意図の要求 |

**選定理由**: snapshot は「任意時点の state の不変コピー」という位置付け。silently 上書きは事故の元。`--force` で明示的な上書きを許可。

### その他の判断 (1-2 行の根拠のみ)

- **serde_yml 0.0.12 → serde-saphyr**: 0.0.x + unmaintained (RUSTSEC-2025-0068) のため、panic-free + budget control + merge key 対応の serde-saphyr に差し替え。薄い抽象層 `src/yaml/` で将来の差し替え可能性を担保
- **minijinja**: Jinja2 互換の軽量 template engine (~50KB)、LLM が理解しやすい syntax
- **git CLI 直接呼び出し (git2/gix ではなく)**: C 依存を排除、ユーザーの git 設定との完全一致、flowrail の git 操作は軽量なため CLI オーバーヘッドを受容可能
- **4 Core Primitive Checks**: `file_exists` / `cmd_exit` / `regex_match` / `git_status` の 4 つで高次 check を全て合成可能 (例: `tests_pass` = `cmd_exit` with `expected: 0`)。5 個目以降を追加するなら rule set 側で合成する方針

---

## Feasibility Mapping (Phase A 前完成版)

> **Phase A 実装開始前の必須 gate**。本セクションは、現行 dotfiles の `workflow-engine/modules/*.md` + `feature-dev/regate/*.md` + 4 パイプライン (feature-dev / debug-flow / triage / linear-sync) に存在する非自明機能を flowrail core + rule set で表現可能かを 7 × 4 の mapping 表で評価し、各機能について **救済 (flowrail core / rule set schema で実装)** または **退行 (許容された機能低下)** のいずれかに分類する。

### 評価方法

**縦軸 (7 カテゴリ)**: 現行 `workflow-engine/modules/*.md` 7 ファイル + `feature-dev/regate/*.md` 3 ファイル

| # | カテゴリ | 現行ファイル | LOC |
|---|---------|-------------|-----|
| (a) | Audit | `workflow-engine/modules/audit.md` | 463 |
| (b) | Inner Loop | `workflow-engine/modules/inner-loop.md` | 219 |
| (c) | Autonomy | `workflow-engine/modules/autonomy.md` | 143 |
| (d) | Phase Summary | `workflow-engine/modules/phase-summary.md` | 100 |
| (e) | Linear-sync | `linear-sync/SKILL.md` | 322 |
| (f) | Triage | `triage/SKILL.md` | 196 |
| (g) | Regate | `feature-dev/regate/{audit-failure,test-failure,review-findings}.md` | 計 ~130 |

**横軸 (4 パイプライン)**: 既存の実運用パイプライン

| パイプライン | 構造 | 典型フェーズ数 | 使用モジュール |
|------------|------|--------------|--------------|
| feature-dev | `pipeline.yml` + `phases/*.md` | 9 | audit, autonomy, regate, resume, phase-summary, context-budget |
| debug-flow | 同上 | 8 | 同上 |
| triage | `SKILL.md` 単体 (`pipeline.yml` 不在) | 4 | （宣言的 modules 未使用、対話型） |
| linear-sync | integration supplement | 0 (hook として呼ばれる) | （on_pipeline_start / on_phase_complete / on_regate / on_handover / on_pipeline_complete） |

**セル表記**:
- ○ = そのパイプラインで実際に機能が活性化される
- △ = 機能は定義されているが、該当パイプラインでは限定的に発火 / 稀
- × = 該当パイプラインでは使用されない
- — = 対象外 (integration 性質上、該当しない)

**凡例**: 各機能の右側に **[救済]** / **[退行]** / **[部分救済]** の分類マーカーを付与。

---

### (a) Audit — 10 機能 (`workflow-engine/modules/audit.md` 463 行)

| # | 機能 | feature-dev | debug-flow | triage | linear-sync |
|---|-----|:-:|:-:|:-:|:-:|
| a1 | phase-auditor Agent 起動 (`audit: required`) **[退行]** | ○ integrate 以外の全 phase | ○ 同 | × | — |
| a2 | Audit Team swarm (`--swarm` 3 メンバー協調) **[退行]** | △ `--swarm` 指定時のみ | △ 同 | × | — |
| a3 | audit_target projection (audit-only phase) **[救済]** | ○ `plan-review` が `implementation_plan` を検証 | ○ `fix-plan-review` が `fix_plan` を検証 | × | — |
| a4 | cumulative_diagnosis (attempt 間累積) **[部分救済]** | ○ Fix Dispatch ループで使用 | ○ 同 | × | — |
| a5 | escalation != null での即 PAUSE **[退行]** | ○ | ○ | × | — |
| a6 | Evidence Plan 動的生成 (accept-test / doc-audit / execute で activity type 別) **[退行]** | ○ | ○ | × | — |
| a7 | validation_record 累積注入 (下流 auditor に) **[救済]** | ○ | ○ | × | — |
| a8 | Fix Dispatch 戦略 (phase 別に executor 決定) **[退行]** | ○ {execute}/{accept-test}/{review}/{doc-audit} は `feature-implementer` に委任 | ○ 同 | × | — |
| a9 | Re-gate + Re-review ループ ({execute}→{accept-test}→{doc-audit}→{review} 再実行) **[退行]** | ○ {review} で findings 発生時 | ○ 同 | × | — |
| a10 | Audit Gate Lite ({integrate} は Agent 起動せず直接検証) **[退行]** | ○ `no_conflicts` / `no_uncommitted_changes` を orchestrator が検証 | ○ 同 | × | — |

**救済/退行の詳細**:

- **[救済 a3]** `audit_target` projection: flowrail core の built-in directive `validation_question` を上流 artifact にも適用できるよう拡張する。done-criteria 相当の recipe で `audit_target: <upstream_name>` を宣言できる形に schema を拡張（本 spec の "8 Built-in Directives" §validation_question を参照）。これは audit-only phase (plan-review / fix-plan-review) の存在価値そのものなので救済必須
- **[救済 a7]** `validation_record` 累積: flowrail core の `state.phases[].validation_record` フィールドで保持し、後続 phase の directive 評価時に自動注入する。本 spec §State Schema に定義済み
- **[部分救済 a4]** `cumulative_diagnosis`: flowrail core の `state.phases[].audit.attempts[]` として最低限の attempt 履歴を保持するが、`diff_from_previous` の自動算出までは Phase 1-2 では対象外。rule set が `classifier_question` で diff を LLM に算出させる形で代替
- **[退行 a1, a2]** phase-auditor / Audit Team swarm: flowrail core は Agent を起動しない (Agent 起動は Claude Code の役目)。代わりに rule set の `hook_command` directive で phase-auditor 相当の外部スクリプト呼び出しを宣言する。`--swarm` は rule set 作者が swarm 版 hook を用意する責任
- **[退行 a6]** Evidence Plan 動的生成: 各 rule set recipe の params として activity type を受け取り、静的な evidence collection 指示を rule set 作者が記述する。動的生成は非対応
- **[退行 a8]** Fix Dispatch 戦略: rule set の `produce_artifact` directive で fix executor を phase 毎に宣言する。現行の「phase 別テーブル」は rule set 側の責務
- **[退行 a9]** Re-gate + Re-review ループ: rule set の `regate_action` directive + `verification_chain` recipe で表現可能だが、現行の複雑な step フロー (Step 1→2→3→4→5) は rule set 作者が書く責任。flowrail core は単に `rewind_to` に従う
- **[退行 a10]** Audit Gate Lite: rule set の `cmd_exit` primitive で `git status` / `git diff --check` を直接呼び出す

---

### (b) Inner Loop — 9 機能 (`workflow-engine/modules/inner-loop.md` 219 行)

| # | 機能 | feature-dev | debug-flow | triage | linear-sync |
|---|-----|:-:|:-:|:-:|:-:|
| b1 | HARD-GATE「Verify 未通過で Audit Gate 不可」 **[退行]** | ○ {execute} 内 | ○ 同 | × | — |
| b2 | 同一テスト 2 連続同理由 PAUSE **[部分救済]** | ○ Failure Router | ○ 同 | × | — |
| b3 | 要件曖昧さ検出で即 PAUSE (iteration 回数無視) **[退行]** | ○ | ○ | × | — |
| b4 | Impl 1 回のみ + TestEnrich/Verify ループ制約 **[退行]** | ○ | ○ | × | — |
| b5 | lint/fmt のオーケストレーター直接自動修正 **[退行]** | ○ | ○ | × | — |
| b6 | current_substep ベース resume (Impl/TestEnrich/Verify) **[救済]** | ○ {execute} | ○ 同 | × | — |
| b7 | completed_tasks / remaining_tasks 個別スキップ **[救済]** | ○ | ○ | × | — |
| b8 | Failure Router (テスト/lint/型/ビルド分岐) **[退行]** | ○ | ○ | × | — |
| b9 | 3 iteration 超過で PAUSE **[退行]** | ○ | ○ | × | — |

**救済/退行の詳細**:

- **[救済 b6, b7]** `inner_loop_state` の mid-phase resume: flowrail core の state schema に `phases[].sub_phase` と `phases[].completed_tasks[] / remaining_tasks[]` を追加する (本 spec §State Schema に定義済み)。これは最重要の救済候補。理由: execute フェーズは最もコンテキストを消費するため handover 中断が頻発する。sub_phase を持たないと、Impl 完了後の TestEnrich から再開できず、Impl を 2 回実行してしまう
- **[部分救済 b2]** 同一テスト 2 連続同理由 PAUSE: flowrail core は state に `failure_history[]` を保持するが、「同理由」の判定は LLM / rule set が `classifier_question` directive で実施する。flowrail core は history を提供するだけ
- **[退行 b1, b3, b4]** HARD-GATE / 要件曖昧さ検出 / Impl 1 回制約: rule set の phase 遷移条件 + `classify_then_regate` directive で表現する。3 sub-phase 展開 (execute-impl / execute-test-enrich / execute-verify) は本 spec §Inner Loop 3 Sub-phase Expansion に定義済み
- **[退行 b5]** lint/fmt 自動修正: rule set の `hook_command` directive で `cargo fmt` / `cargo clippy --fix` 等を phase pre hook として呼び出す
- **[退行 b8, b9]** Failure Router / 3 iteration PAUSE: rule set の `classify_then_regate` directive でテスト失敗を分類し、`settings.max_failure_iterations: 3` で PAUSE を制御

---

### (c) Autonomy — 10 機能 (`workflow-engine/modules/autonomy.md` 143 行、30+ if-then エントリ)

| # | 機能 | feature-dev | debug-flow | triage | linear-sync |
|---|-----|:-:|:-:|:-:|:-:|
| c1 | brainstorming 設計質問 → PAUSE **[退行]** | ○ {design} | ○ {rca} | × | — |
| c2 | worktree テスト失敗 → PAUSE **[退行]** | ○ {design} | ○ {rca} | × | — |
| c3 | UI 関連キーワード検出 → PAUSE (accept-test 有効化提案) **[退行]** | ○ {accept-test} | ○ 同 | × | — |
| c4 | VRT 差分検出 → PAUSE **[退行]** | ○ {accept-test} | ○ 同 | × | — |
| c5 | フレーキーテスト検出 → AUTO (報告のみ、ブロック無し) **[退行]** | ○ | ○ | × | — |
| c6 | Context 逼迫 → PAUSE (全フェーズ共通) **[退行]** | ○ | ○ | △ triage は短いため発火頻度低 | — |
| c7 | `--codex` 接続失敗 → warning + 続行 **[退行]** | ○ | ○ | × | — |
| c8 | 3 連続 findings → PAUSE (根本設計見直し促進) **[退行]** | ○ {spec-review}/{plan-review}/{review} | ○ {fix-plan-review}/{review} | × | — |
| c9 | audit escalation != null → 即 PAUSE **[退行]** | ○ | ○ | × | — |
| c10 | 3 Autonomy Mode (INTERACTIVE / AUTONOMOUS / AUTONOMOUS+GATE) **[退行]** | ○ phase 別に付与 | ○ 同 | △ 全 phase が INTERACTIVE 相当 | — |

**救済/退行の詳細**:

- **[退行 (全体)]** autonomy の 30+ if-then ルールは、rule set の `phase_confirm` primitive + `classifier_question` directive で 1:1 移植する方針。LOC は 143 → ~200 行に増えるが (rule set の冗長性)、静的検証可能な形式に変換されるメリットがある
- **[退行 c3, c4]** UI キーワード検出 / VRT 差分検出: rule set の pre_phase hook で `cmd_exit` + `regex_match` primitive を組み合わせて実装 (rule set 作者の責任)
- **[退行 c6]** Context 逼迫検出: flowrail core は文字数計測を行わないため、rule set の `context-budget` recipe で `settings.max_phase_context: 150k` を宣言し、閾値超過時に `phase_confirm` で handover を提案する
- **[退行 c10]** 3 Mode 分類: rule set の phase metadata で `autonomy_mode: interactive | autonomous | autonomous_gate` を宣言できるように schema を拡張。flowrail core はこのフラグを参照して `phase_confirm` directive のデフォルト動作を切り替える

> **補足**: autonomy の「動的状況判定 (LLM が if-then 条件を評価する)」は flowrail core では対応しない。rule set 側で明示的な `classifier_question` + `classify_then_regate` の組み合わせで表現するため、rule set 作者が条件を静的に書き下す必要がある。これは tiny 原則との trade-off

---

### (d) Phase Summary — 6 機能 (`workflow-engine/modules/phase-summary.md` 100 行)

| # | 機能 | feature-dev | debug-flow | triage | linear-sync |
|---|-----|:-:|:-:|:-:|:-:|
| d1 | concerns / directives の target_phase 伝播 **[救済]** | ○ 全 phase | ○ 同 | × | ○ consume 側 (sync 時に記録) |
| d2 | validation_record の question 単位累積 **[救済]** | ○ | ○ | × | ○ consume 側 |
| d3 | inner_loop_state の mid-phase handover **[救済]** | ○ {execute} | ○ 同 | × | — |
| d4 | evidence の linear_sync 区分 (inline / attached / reference_only) **[救済]** | ○ | ○ | × | ○ 3 区分を使い分け |
| d5 | regate_history 追跡 **[救済]** | ○ | ○ | × | ○ consume 側 (sync_regate) |
| d6 | Phase 完了毎の `{phase_id}.yml` 保存 **[退行]** | ○ `.agents/handover/{branch}/{fp}/phase-summaries/` | ○ 同 | × | — |

**救済/退行の詳細**:

- **[救済 d1, d2, d3, d4, d5]** flowrail core の `state.phases[]` に以下のフィールドを追加することで全て救済する (本 spec §State Schema に定義済み):
  - `state.phases[].concerns[]` (target_phase 付き)
  - `state.phases[].directives[]` (target_phase 付き)
  - `state.phases[].validation_record[]` (criterion / verdict / evidence)
  - `state.phases[].sub_phase` + `state.phases[].inner_loop_state` (completed_tasks, remaining_tasks, failure_history)
  - `state.phases[].evidence[]` (type, content, linear_sync 区分)
  - `state.phases[].regate_history[]`
- **[退行 d6]** 個別 YAML ファイル (`phase-summaries/{phase_id}.yml`) は廃止。flowrail の `state.json` に統合される (本 spec §Migration Strategy 参照)。continue skill は `state.json` から直接 Phase 情報を読み出すように更新される

---

### (e) Linear-sync — 8 機能 (`linear-sync/SKILL.md` 322 行)

| # | 機能 | feature-dev | debug-flow | triage | linear-sync |
|---|-----|:-:|:-:|:-:|:-:|
| e1 | `resolve_ticket` の AskUserQuestion 対話 (pre_pipeline_start) **[救済]** | ○ `--linear` 時 activation | ○ 同 | × (triage は Phase 4 で Issue を作る) | — (self) |
| e2 | `sync_phase` の structured payload (test_results / audit_observations / evidence_files) **[退行]** | ○ on_phase_complete hook | ○ 同 | × | — |
| e3 | `linear_ticket_id` / `document_id` のセッション間持続化 **[救済]** | ○ `project-state.json` | ○ 同 | × | — |
| e4 | `read_phase_summary` フォールバック復元 (Linear からの読み戻し) **[部分救済]** | ○ continue skill から呼ばれる | ○ 同 | × | — |
| e5 | `sync_regate` / `sync_handover` / `sync_complete` **[退行]** | ○ 各イベントで発火 | ○ 同 | × | — |
| e6 | Workflow Report Document 生成 (`templates/document.md`) **[退行]** | ○ on_pipeline_start | ○ 同 | × | — |
| e7 | コメント冪等性チェック (`## Phase {N}:` で始まるコメントの更新 vs 新規) **[退行]** | ○ | ○ | × | — |
| e8 | Error handling (API 失敗 → warn 続行、添付失敗 → local path 記載) **[退行]** | ○ | ○ | × | — |

**救済/退行の詳細**:

- **[救済 e1]** `resolve_ticket` 対話: flowrail core の `pre_pipeline_start` hook phase + built-in directive `classifier_question` で表現する。rule set 作者が「linear-resolve-ticket」recipe を定義し、feature-dev / debug-flow の pipeline 冒頭で `uses: linear-resolve-ticket` と宣言する
- **[救済 e3]** `linear_ticket_id` 持続化: flowrail core の `state.integrations.linear.{ticket_id, document_id}` で保持 (本 spec §State Schema に定義済み)。この persistence がないと毎セッション resolve_ticket を再実行することになり体験が悪い
- **[部分救済 e4]** `read_phase_summary` fallback: flowrail の `state.json` が single source of truth なら、Linear からの復元は不要になる (ローカル state.json が欠損した場合の disaster recovery としてのみ必要)。Phase 1-2 では非対応、Phase 3+ の検討事項
- **[退行 e2, e5, e6, e7, e8]** 各種 sync_* / Workflow Report Document / 冪等性 / Error handling: hook 実装側 (シェルスクリプト or 外部ツール) の責務。flowrail core は hook の成功/失敗を監視するだけ。hook は `FLOWRAIL_STATE_FILE` 環境変数で `state.json` を受け取り、必要な情報を抽出して Linear API を叩く

> **再設計の方向性**: `linear-sync` skill は、(1) `pre_pipeline_start` に相当する `resolve_ticket.md` (LLM 対話) と (2) 各 hook イベント用の CLI スクリプト (`sync_phase.sh`, `sync_regate.sh` 等) に分解する。(1) は rule set として他 pipeline から再利用可能、(2) は hook 実装として flowrail core の hook_command directive から呼ばれる

---

### (f) Triage — 6 機能 (`triage/SKILL.md` 196 行)

| # | 機能 | feature-dev | debug-flow | triage | linear-sync |
|---|-----|:-:|:-:|:-:|:-:|
| f1 | URL 引数対話型フロー (4 Phase: Data Collection → Context Exploration → Analysis → Linear Registration) **[救済 (再定義)]** | × | × | ○ 全フェーズ | × |
| f2 | 外部リンク探索 (Slack CLI / gh CLI / WebFetch fallback) **[退行]** | × | × | ○ Phase 2 | × |
| f3 | AskUserQuestion による探索選択 (任意 / 推奨 / all / none) **[救済]** | × | × | ○ Phase 2 | × |
| f4 | `pipeline.yml` 不在で `SKILL.md` 単体動作 **[救済 (再定義)]** | × | × | ○ | ○ supplement |
| f5 | ツール選択 LLM 判断 (専用 CLI 優先、WebFetch fallback) **[退行]** | × | × | ○ Phase 1 | × |
| f6 | Issue 作成承認フロー (preview → approve) **[救済]** | × | × | ○ Phase 4 | × |

**救済/退行の詳細**:

- **[救済 f1 (再定義)]** 4 Phase フロー: flowrail の pipeline.yml で再定義する。`triage/pipeline.yml` を新規作成し、`phases: [data-collection, context-exploration, analysis, linear-registration]` として宣言。現行 `triage/SKILL.md` 単体動作は廃止し、薄い entry wrapper にする
- **[救済 f3]** AskUserQuestion 探索選択: flowrail core の built-in directive `classifier_question` で表現。`options: [推奨1, 推奨2, 任意3, none, all]` 相当を指定可能にする
- **[救済 f4 (再定義)]** pipeline.yml 不在動作: 廃止。triage も他 pipeline と同じく pipeline.yml を持つ形に統一。理由: flowrail core の一貫性 (原則 3 "One Source of Truth")
- **[救済 f6]** Issue 作成承認フロー: built-in directive `confirm` で preview 表示 + 承認待ち
- **[退行 f2]** 外部リンク探索: phase hook で `cmd_exit` primitive を使って `slack search <keyword>` / `gh issue list --search <keyword>` を呼ぶ。具体的なコマンドとパース処理は rule set 作者が書く責任
- **[退行 f5]** ツール選択の LLM 判断: flowrail core は LLM を起動しない。rule set が明示的に「urlpattern=slack.com なら slack CLI を使う、それ以外は WebFetch」という条件分岐を `classify_then_regate` directive で記述する責任

---

### (g) Regate — 8 機能 (`feature-dev/regate/{audit-failure,test-failure,review-findings}.md` 計 ~130 行)

| # | 機能 | feature-dev | debug-flow | triage | linear-sync |
|---|-----|:-:|:-:|:-:|:-:|
| g1 | flaky 検出 (同一テスト 2 回交互で判定) **[部分救済]** | ○ test-failure.md | ○ 同 | × | — |
| g2 | concern の後続フェーズ伝播 (flaky 検出時) **[救済]** | ○ | ○ | × | ○ consume 側 |
| g3 | fix_instruction 組み立てと regate_context 注入 **[救済]** | ○ 全 regate strategy | ○ 同 | × | — |
| g4 | verification_chain フル実行 (execute → review → accept-test) **[救済]** | ○ test-failure / review-findings | ○ 同 | × | — |
| g5 | `rewind_to: plan` (設計起因 blocker 時) **[救済]** | ○ review-findings | ○ `rewind_to: fix-plan` に変換 | × | — |
| g6 | `rewind_to: current` (コード変更なし、現フェーズ re-audit) **[救済]** | ○ audit-failure | ○ 同 | × | — |
| g7 | max_retries 超過での 3 択 escalation (retry / skip / abort) **[救済]** | ○ audit-failure | ○ 同 | × | — |
| g8 | severity 分類 (blocker → 即 regate / quality → threshold 3 件) **[救済]** | ○ review-findings | ○ 同 | × | — |

**救済/退行の詳細**:

- **[救済 g2-g8]** regate 全般は、rule set の `regate-*` recipe (audit-failure / test-failure / review-findings) + built-in directive `classify_then_regate` + `regate_action` で完全移植可能。本 spec §8 Built-in Directives にこの語彙セットが定義されている。規約:
  - `rewind_to: <phase_id>` は `regate_action` の params
  - `verification_chain: [<phase_ids>]` は rule set の settings
  - severity threshold は `classifier_question` の `threshold: 3` params
- **[部分救済 g1]** flaky 検出: flowrail core は `state.phases[].test_results[]` の履歴を 2 回分保持するが、「2 回交互で flaky」の判定は rule set の `classifier_question` directive で LLM に委ねる。理由: test runner 出力の多様性を flowrail core がパースするのは tiny 原則に反する

---

### Feasibility 総括

| カテゴリ | 救済 | 部分救済 | 退行 | 機能数合計 |
|---------|:-:|:-:|:-:|:-:|
| (a) Audit | 2 | 1 | 7 | 10 |
| (b) Inner Loop | 2 | 1 | 6 | 9 |
| (c) Autonomy | 0 | 0 | 10 | 10 |
| (d) Phase Summary | 5 | 0 | 1 | 6 |
| (e) Linear-sync | 2 | 1 | 5 | 8 |
| (f) Triage | 4 | 0 | 2 | 6 |
| (g) Regate | 7 | 1 | 0 | 8 |
| **合計** | **22** | **4** | **31** | **57** |

**判断原則** (Phase A 適用):

1. **救済優先順位**:
   - **最優先** (原則 2 "flowrail core は低次語彙を知る" 準拠): State Schema 拡張で救済できる機能群 — d1-d5 (concerns/directives/validation_record/inner_loop_state/evidence), e3 (linear ticket_id persistence)
   - **第二優先** (built-in directive で救済): a3 (audit_target), a7 (validation_record), b6-b7 (inner_loop_state resume), e1 (resolve_ticket 対話), f1/f3/f4/f6 (triage 再定義), g2-g8 (regate 全般)
   - **第三優先 (部分救済で妥協)**: a4 (cumulative_diagnosis の diff 算出は rule set 責任), b2 (同一テスト 2 連続判定は LLM), e4 (Linear 読み戻しは Phase 3+), g1 (flaky 判定は LLM)

2. **退行の許容理由**:
   - **Agent 起動 (a1, a2)**: flowrail core は LLM を呼ばない。hook 経由で Claude Code 側に任せる
   - **動的判断 (c1-c10 の大半)**: 30+ if-then は rule set の静的記述で書き下す。LOC は増えるが静的検証可能
   - **hook 実装 (a6, a8, b5, e2, e5-e8, f2, f5)**: 外部スクリプト or CLI に外出し。flowrail core は hook_command directive で起動するだけ
   - **`--swarm` (a2)**: rule set 作者が swarm 版 hook を別途用意

3. **非対応機能リスト** (spec の公開時にユーザーに明示同意を取る項目):
   - Audit Team swarm (3 メンバー協調): Phase 1-2 では非対応
   - 動的 Evidence Plan 生成: Phase 1-2 では rule set 作者が静的記述
   - `cumulative_diagnosis` の `diff_from_previous` 自動算出: LLM 依存
   - `read_phase_summary` Linear 読み戻し fallback: Phase 3+ で再評価
   - ツール選択の LLM 判断 (triage): rule set 作者が条件を書き下す

4. **LOC 影響**:
   - rule set への外出しで LOC が増加: dotfiles 散文 ~1,079 行 → rule set YAML ~400 行 + built-in directive 相当のコード ~600 行 = ~1,000 行 (実質イーブン、ただし静的検証可能な形式になる)
   - State Schema 拡張で flowrail core に追加: ~200 行 (serde model + state transition validator)
   - 本 Feasibility Mapping を反映した最終見積: flowrail core Phase 1 は ~3,500-3,800 LOC 維持、Phase 2 Testing Primitives 完了時 ~4,400 LOC (原則 7 tiny by constraint の boundary 内)

### Phase A 着手条件チェックリスト

- [x] 7 カテゴリ × 4 パイプラインの 1:1 mapping 作成済み (57 機能の網羅)
- [x] 各機能に救済/部分救済/退行の分類マーカー付与
- [x] State Schema 拡張が必要な機能の特定 (7 機能: d1-d5, e3, b6-b7, a7 相当)
- [x] Built-in Directive で救済される機能の特定 (15 機能: a3, b2*, b6-b7, c (部分), e1, f1/f3/f4/f6, g2-g8)
- [x] 非対応機能リストをユーザー同意前提として明示
- [x] LOC 影響の再見積 (原則 7 tiny の boundary 内に収まることを確認)
- [ ] **残タスク (Phase A 着手時)**: 本 mapping を元に、flowrail-core の新規 module (`crates/flowrail-core/src/directives/` 等) の責務分割設計

---

## Impact Analysis (Phase A 前完成版)

> **Phase A 実装開始前の必須 gate**。本設計は dotfiles 内の workflow-engine + 4 パイプライン + 周辺 skill の全面刷新を伴うため、逆依存・共有状態・暗黙の契約・副作用リスクを洗い出す。

### 1. Reverse Dependencies (逆依存)

本設計は以下のファイル群に直接/間接的な影響を与える。file:line 参照は commit 591c21c 時点 (2026-04-05) の dotfiles を基準とする。

#### 1.1 直接依存 (Phase C-D で必ず更新)

| 分類 | ファイル | 影響 LOC | 更新内容 |
|------|---------|---------|---------|
| **workflow-engine 本体** | `claude/skills/workflow-engine/SKILL.md` | ~114 行 | flowrail run ループに書き換え (~30 行に削減) |
| **workflow-engine modules** | `claude/skills/workflow-engine/modules/audit.md` | 463 | rule set `audit-gate` recipe + `validation_question` directive に置換 |
| | `claude/skills/workflow-engine/modules/inner-loop.md` | 219 | 3 sub-phase 展開 + `regate-router` recipe に置換 |
| | `claude/skills/workflow-engine/modules/autonomy.md` | 143 | rule set の `phase_confirm` primitive + 30+ `classifier_question` に展開 |
| | `claude/skills/workflow-engine/modules/regate.md` | 29 | `rules/recipes/regate-*.yml` + `classify_then_regate` directive に置換 |
| | `claude/skills/workflow-engine/modules/resume.md` | 80 | `flowrail snapshot restore` + `state.integrations` に置換 |
| | `claude/skills/workflow-engine/modules/phase-summary.md` | 100 | `state.phases[].{concerns,directives,validation_record}` 統合 |
| | `claude/skills/workflow-engine/modules/context-budget.md` | 45 | `settings.default_snapshot_hint_after_phases` に置換 |
| **feature-dev** | `claude/skills/feature-dev/pipeline.yml` | ~210 | 新形式 (`imports`, `uses`, `params`) で書き換え |
| | `claude/skills/feature-dev/phases/*.md` | 9 ファイル | 新 phase_file 規約 (`executor`, `produce_artifact` の rule set 参照) |
| | `claude/skills/feature-dev/done-criteria/*.md` | 9 ファイル | `audit: required / lite` + `operations` → recipe params に変換 |
| | `claude/skills/feature-dev/regate/*.md` | 3 ファイル | `rules/recipes/regate-{audit-failure,test-failure,review-findings}.yml` に置換 |
| **debug-flow** | `claude/skills/debug-flow/pipeline.yml:1-200` | ~200 | feature-dev と同様 |
| | `claude/skills/debug-flow/phases/*.md` | 8 ファイル | 同上 |
| | `claude/skills/debug-flow/done-criteria/*.md` | 8 ファイル | 同上 |
| | `claude/skills/debug-flow/regate/*.md` | 3 ファイル | 同上 (rewind_to: fix-plan / rewind_to: current を含む) |
| **triage** | `claude/skills/triage/SKILL.md:1-196` | 196 | `claude/skills/triage/pipeline.yml` (新規) + `phases/{data-collection,context-exploration,analysis,linear-registration}.md` に分解 |
| **linear-sync** | `claude/skills/linear-sync/SKILL.md:1-322` | 322 | `resolve_ticket.md` (LLM 対話、rule set として他 pipeline から `uses`) + `scripts/sync_{phase,regate,handover,evidence,complete}.sh` (hook 実装) に分解 |
| | `claude/skills/linear-sync/templates/{document,comment,handover-comment}.md` | 3 ファイル | hook script の文字列テンプレートに移動 |

**合計直接影響**: 1,079 行 (modules) + 約 2,000 行 (pipelines/phases/done-criteria/regate 含む) = **~3,000 行**

#### 1.2 間接依存 (Phase D-E で調整)

| ファイル | 現行責務 | 影響箇所 (file:line) | 更新内容 |
|---------|---------|------------------|---------|
| `claude/skills/continue/SKILL.md` | Pipeline Detection、resume | L51 (`pipeline` フィールド読み取り)、L90-94 (phase-summaries fallback)、L122 (project-state.json 読み込み)、L147 (更新)、L153 (handover.md 再生成)、L185 (cleanup 7 days) | flowrail `state.json` スキーマ認識を追加。現行 `.agents/handover/{branch}/{fingerprint}/` 構造と新 `.flowrail/{pipeline}-{branch}.state.json` の両方を読める形に拡張 |
| `claude/skills/handover/SKILL.md` | project-state.json 生成、handover.md 生成、phase-summaries/ 管理 | L31-34 (保存先)、L127-128 (phase_summaries マッピング)、L157 (linear.issue_id)、L165-200 (pipeline 判定と phase-summaries ディレクトリ作成)、L254 (cleanup) | flowrail state との責務分担を確立。handover skill は "session summary + kanban" に特化、flowrail は "pipeline run state" を担当 |
| `claude/skills/kanban/SKILL.md` | タスク管理、handover 同期 | L35-38 (`.agents/handover/` 配下の project-state.json 読み込み、active_tasks 参照) | flowrail state からの active_tasks 読み出しに対応 (project-state.json と併存可能) |
| `claude/skills/doc-check/SKILL.md` | md frontmatter `depends-on` 走査 | L5-7 (depends-on + 本文 Markdown リンク)、L14 (scripts/doc-check.sh 起動) | 新 `docs/specs/` / `docs/plans/` のパス構造に対応 (flowrail プロジェクトでは dotfiles との path convention が異なる) |
| post-commit hook | state 参照 | N/A (現行 dotfiles に hook 未設置、将来追加時に考慮) | 該当なし (2026-04-05 時点で `/Users/nishikataseiichi/.claude/hooks/` ディレクトリは未作成) |

**間接依存の総括**: 現行 continue / handover / kanban は `.agents/handover/` + `phase-summaries/*.yml` 構造に密結合しているため、flowrail への移行では migration strategy Phase D (skill 薄いラッパー化) で段階的に対応する。

### 2. Shared State (共有状態)

#### 2.1 State Schema 対比

| 状態カテゴリ | 現行 (dotfiles) | 新設計 (flowrail) | 共存戦略 |
|------|------|-------|---------|
| Pipeline 実行状態 | `.agents/handover/{branch}/{fingerprint}/project-state.json` | `.flowrail/state.json` (worktree root 基準) | Phase D まで両方書き込み、Phase E で一斉切替 |
| Phase Summary | `.agents/handover/{branch}/{fingerprint}/phase-summaries/{phase_id}.yml` (individual YAML) | `state.json.phases[].{concerns,directives,validation_record,inner_loop_state,evidence,regate_history}` に統合 | `flowrail state export --format yaml` で個別 YAML への変換ツールを Phase D で提供 (continue skill との互換性維持用) |
| Snapshot | `.agents/handover/.../handover.md` (human-readable) | `.flowrail/snapshots/{label}.json` (machine-readable) + `flowrail state view > handover.md` (人間向け export) | 別ドメインとして独立運用 |
| Linear 連携状態 | 未 persist (hook が毎回 `resolve_ticket` を実行) | `state.json.integrations.linear.{ticket_id, document_id, last_synced_phase}` | `pre_pipeline_start` の `resolve_ticket` で初期設定、以降 persist |
| inner_loop_state | `phase-summaries/execute.yml` 内に埋め込み | `state.phases[<execute>].inner_loop_state.{current_substep, impl_progress, loop_iteration, failure_history}` | schema 変換ツールで移行 |
| 実行中ログ | 未構造化 (Claude 自身のコンテキスト) | `.flowrail/runs/{run_id}/events.jsonl` (JSONL event stream) | 新機能、現行に相当なし |

#### 2.2 `.flowrail/` ディレクトリ構造

```
<workspace-root>/            # git worktree root
├── .flowrail/
│   ├── state.json           # current pipeline state (single source of truth)
│   ├── state.json.tmp.*     # atomic write temp files (起動時に検出・削除)
│   ├── snapshots/
│   │   ├── pre-execute.json # user-created snapshots (flowrail snapshot create)
│   │   └── auto-{ts}.json   # auto snapshots (phase 遷移時)
│   └── runs/
│       └── {run_id}/        # UUID v7 (時系列順)
│           ├── events.jsonl
│           └── hook-logs/
│               ├── sync_phase.log
│               └── sync_regate.log
├── .agents/                 # 現行 (dotfiles 互換、移行期間のみ)
│   └── handover/
│       └── {branch}/
│           └── {fingerprint}/
│               ├── project-state.json
│               ├── handover.md
│               └── phase-summaries/
```

#### 2.3 環境変数の競合

| 環境変数 | 設定者 | 用途 | 競合リスク |
|---------|-------|------|-----------|
| `FLOWRAIL_NOW` | flowrail core (Deterministic Mode) | 現在時刻の決定論的固定 | 他ツールと競合なし (prefix で固有) |
| `FLOWRAIL_SEED` | flowrail core | 乱数 seed 固定 | 同上 |
| `FLOWRAIL_STATE_FILE` | flowrail core → hook | hook に state.json path を渡す | 同上 |
| `FLOWRAIL_RUN_ID` | flowrail core → hook | 現在の run_id (UUID v7) | 同上 |
| `FLOWRAIL_PHASE` | flowrail core → hook | 現在の phase_id | 同上 |
| `CLAUDE_PROJECT_DIR` | Claude Code | プロジェクトルート | flowrail は読み取るのみ、設定しない |
| `CLAUDE_SESSION_ID` | Claude Code | セッション ID | 同上 (linear-sync hook が参照) |

**結論**: `FLOWRAIL_*` prefix で全環境変数を名前空間化しているため、Claude Code やシェル環境との競合リスクはない。ただし linear-sync hook は `CLAUDE_SESSION_ID` を既存契約として使い続ける必要がある (下記 §3 参照)。

### 3. Implicit Contracts (暗黙の契約)

Phase A 着手前に明示的に宣言すべき契約。現行 dotfiles では暗黙的に維持されていた規約が、flowrail 移行で破壊されないかを検証する。

#### 3.1 linear-sync hook との env var 互換性

現行 linear-sync は以下の環境変数を参照する (commit 591c21c 時点の `claude/skills/linear-sync/SKILL.md` L189-200 参照):

| 既存変数 | 用途 | flowrail での扱い |
|---------|------|------------------|
| `CLAUDE_SESSION_ID` | Layer 3 session 情報の記録 | **維持必須**。flowrail hook は Claude 側が設定した変数をそのまま transparent に hook script に渡す |
| branch name (`git rev-parse --abbrev-ref HEAD`) | `resolve_ticket` の推定 | **維持**。flowrail は branch を state に記録するが、hook が直接 git を呼ぶことも許容 |

**新規変数**: flowrail が hook に渡す `FLOWRAIL_*` 変数は **追加のみ**、既存 `CLAUDE_*` 変数は変更・削除しない。既存 linear-sync の sync_session 関数 (L236-254) は hook スクリプトに 1:1 移植される (環境変数契約は維持)。

#### 3.2 既存 session.session_id フォーマット

現行 `project-state.json` の `session.session_id` は free-form string (例: `"unknown"`, `"claude-<uuid>"`)。flowrail state の `session.session_id` は同じ形式を引き継ぎ、フォーマット制約を追加しない (continue/handover skill が互換性を保つため)。

#### 3.3 既存 Phase Summary スキーマ

現行 `phase-summaries/{phase_id}.yml` は以下のフィールドを持つ (handover/SKILL.md L127-128 参照、`phase-summary.md` L18-62 で規定):

```yaml
phase: <N>
phase_name: <name>
status: completed | failed | in_progress
timestamp: <ISO8601>
attempt: <N>
audit_verdict: PASS | FAIL
artifacts: {...}
decisions: [...]
concerns: [...]
directives: [...]
evidence: [...]
inner_loop_state: {...}
validation_record: [...]
regate_history: [...]
```

flowrail state の `state.phases[]` 要素は **このスキーマの全フィールドを superset として含む**。変換は 1:1 の field projection で可能 (損失なし)。新フィールド (`sub_phase`, `run_id_ref` 等) は新規追加のみ。

#### 3.4 `phases/*.md` のセクション見出し規約

現行 `phases/*.md` は以下の h2 を持つことが暗黙の契約:

- `## 実行手順`
- `## 成果物定義`
- `## Phase Summary テンプレート`

flowrail 移行後も、rule set が `phases[].phase_file` でこれらの phase.md を Read する以上、見出し規約は維持される (rule set 作者が phase.md を書き直す場合も同じ規約に従う)。

#### 3.5 done-criteria の `audit: required | lite` 契約

現行 `done-criteria/*.md` frontmatter の `audit: required` / `audit: lite` は workflow-engine が直接参照する。flowrail では `rules/recipes/audit-gate.yml` recipe の params として受け取る (migration は 1:1 の字句置換)。

```yaml
# before (done-criteria/execute.md)
audit: required

# after (pipeline.yml で execute phase の uses に記述)
phases:
  - id: execute
    uses:
      - recipe: audit-gate
        params:
          level: required
```

#### 3.6 `.agents/handover/` と `.flowrail/` の coexistence

**移行期間中の契約**:
- Phase C-D: 両方書き込み (flowrail が `.flowrail/state.json` を正として書き、同時に `.agents/handover/.../project-state.json` にも mirror 書き込み)
- Phase E: `.flowrail/` のみ。`.agents/handover/` は continue/handover/kanban skill 側で一定期間読み取り互換性を維持した後に廃止
- kanban / continue / handover skill は、両方のパスが存在する場合は `.flowrail/state.json` を優先する

### 4. Side Effect Risks (副作用リスク)

#### 4.1 Cargo 初回ビルド時のネットワーク依存

**リスク**: flowrail を初めて clone した環境で `cargo build --workspace` を実行すると、workspace dependencies (clap, serde, serde-saphyr, jsonschema, minijinja, miette, thiserror, notify, uuid, regex, glob 等) を crates.io からダウンロード + ソースビルドするため、**初回 5-10 分程度のネットワーク I/O + CPU 負荷**が発生する。

**緩和策**:
- `Cargo.lock` をコミット対象 (`.gitignore` から除外済み、`CLAUDE.md` に明記) にして、依存 crate のバージョンを固定
- CI では `cargo-cache` や `sccache` で build artifact をキャッシュ
- ユーザーには `README.md` で「初回ビルドは 5-10 分かかる」旨を明記
- オフライン環境では `cargo vendor` で vendored dependency を事前配置する方法を紹介

**境界ケース**: crate.io が一時的に停止している場合、Cargo は `--offline` フラグなしではビルドに失敗する。Phase 1 では CI のみこのリスクを負う (開発者は手元 cache を使う)。

#### 4.2 git 操作の副作用

flowrail core は以下の git CLI を直接呼び出す (git2/gix 等の C 依存ライブラリは `CLAUDE.md` で禁止):

| git 操作 | 呼び出し場所 | 副作用 |
|---------|-------------|-------|
| `git status --porcelain` | `git_status` primitive, `integrate` phase | 読み取りのみ、副作用なし |
| `git rev-parse --abbrev-ref HEAD` | state 初期化時の branch 検出 | 読み取りのみ |
| `git rev-parse HEAD` | run_id 生成時の commit sha 記録 | 読み取りのみ |
| `git diff <range>` | Re-gate trigger (execute 以降) | 読み取りのみ |
| `git log --oneline -<N>` | continue skill 互換の commit 履歴表示 | 読み取りのみ |
| `git worktree list` | worktree 検出 | 読み取りのみ |

**重要**: flowrail core は **git への書き込み操作を一切行わない** (commit / branch / push / reset / rebase 等は禁止)。rule set が hook 経由で `git commit` 等を呼ぶ場合は、rule set 作者の責任。

#### 4.3 Snapshot ファイルによる file system pressure

**リスク**: `flowrail snapshot create` を頻繁に呼ぶと `.flowrail/snapshots/` 配下に JSON ファイルが累積し、大きなパイプラインでは数百 KB × 数十 snapshot = 数 MB の file system pressure。

**緩和策**:
- `flowrail snapshot prune --older-than 7d` サブコマンドで古い snapshot を削除
- `.gitignore` で `.flowrail/` 配下を常に除外 (git リポジトリへの混入防止)
- 自動 snapshot は phase 遷移時のみ、手動 snapshot は user-initiated のみ
- `settings.max_auto_snapshots: 10` で自動 snapshot の上限を宣言

#### 4.4 Atomic write crash

**リスク**: `.flowrail/state.json` を書き込み中にプロセスが kill されると `.flowrail/state.json.tmp.*` が orphan として残る。次回起動時に整合性が崩れる。

**緩和策**:
- flowrail core は起動時に `.flowrail/*.tmp.*` を検出・ログ出力・削除する (Phase 1 の Task として実装)
- 書き込みは常に `tmp + rename` の 2 段階 (POSIX の `rename(2)` atomic 保証を利用)
- crash recovery テストを Phase 1 Task 22 の統合検証に含める

#### 4.5 Hook fire-and-forget silent failure

**リスク**: linear-sync hook など integration の処理失敗が workflow をブロックしない設計 (linear-sync/SKILL.md L305 の Error Handling 方針) のため、重要な sync エラーが気付かれずに流れる。

**緩和策**:
- hook 実行結果を `.flowrail/runs/{run_id}/hook-logs/{hook_name}.log` に常に記録
- exit code != 0 の hook はログに warning として記録し、`flowrail run next` の出力で human readable に警告
- Phase 1 では flowrail core は exit code を見るだけ、詳細な監視は Phase 2+

#### 4.6 Worktree 内の `.flowrail/` 配置

**リスク**: git worktree で作業している場合、`.flowrail/` を **worktree root** に置くか **git common dir** に置くかで snapshot の scope が変わる。

**決定 (本 spec での確定事項)**: **worktree root** に配置する。理由:
- 各 worktree は独立したパイプラインを実行する想定 (feature branch 単位)
- worktree 間で state を共有する必要がない
- `git worktree add` で新しい worktree を作る時に `.flowrail/` が空で初期化される (既存 worktree の state を引き継がない)

**トレードオフ**: worktree を削除すると `.flowrail/` も消える。ユーザーが worktree 削除前に `flowrail snapshot export --to <path>` で外部にバックアップする運用を README で案内。

#### 4.7 既存 project-state.json との混在

**リスク**: Phase C-D の移行期間中、handover skill が `.agents/handover/.../project-state.json` を書き、flowrail が `.flowrail/state.json` を書く。両方が同じ pipeline run を記録する場合、どちらが正か不明瞭。

**緩和策**:
- flowrail が最新の書き込み先 (`source_of_truth: flowrail | handover_skill`) を `state.json.meta` に記録
- handover skill は flowrail state を読んで自分の project-state.json を生成する形に段階的に移行 (Phase D)
- 両方を同時に手動編集することは非推奨 (README で明記)

### 5. Must-Verify Checklist (Phase A 完了時に検証)

Phase A (flowrail 実装) 完了時、以下の 15 項目を全て PASS させてから Phase B (Standard Rule Set Catalog) に進む。各項目は具体的な検証コマンド or 手順を持つ。

- [ ] **V1: continue skill 互換性** — `/continue` skill が flowrail `.flowrail/state.json` を Pipeline Detection で認識し、`pipeline` フィールドを正しく読み取れる
  - 検証: handover で flowrail state を生成 → 新セッションで `/continue` 起動 → Pipeline Detection が発火することを確認
- [ ] **V2: handover skill 互換性** — `/handover` skill の project-state.json 生成が flowrail state と coexist できる (両方が同時に存在しても互いを壊さない)
  - 検証: flowrail state 存在下で `/handover` 実行 → `.agents/handover/.../project-state.json` が flowrail state を mirror した内容で作成されることを確認
- [ ] **V3: linear-sync resolve_ticket** — linear-sync の `resolve_ticket` が `pre_pipeline_start` phase で AskUserQuestion を発火でき、確定した `ticket_id` が `state.integrations.linear.ticket_id` に persist される
  - 検証: `flowrail run init --pipeline feature-dev --linear` → AskUserQuestion が出る → ticket 選択後に state.json を確認
- [ ] **V4: kanban skill 互換性** — kanban skill の handover 同期が flowrail state の `active_tasks` を読み出せる
  - 検証: flowrail state に active_tasks がある状態で kanban skill を起動 → tasks が表示されることを確認
- [ ] **V5: worktree 内の state 参照** — 複数 worktree が同時に存在する場合、`flowrail run step` が正しい worktree の state.json を参照する (他 worktree の state を誤読しない)
  - 検証: 2 つの worktree を作成 → 各々で別 pipeline を実行 → state.json が worktree root に隔離されることを確認
- [ ] **V6: flowrail run step 冪等性** — 同一 phase の `flowrail run step` を 2 回実行しても副作用が二重にならない (中断点再開、partial failure からの回復)
  - 検証: phase 実行中に kill → 再度 step → state が前回の続きから再開されることを確認
- [ ] **V7: paused 遷移 + `--classifier-response`** — `classifier_question` directive が発火した phase で pipeline が paused 状態になり、`flowrail run step --classifier-response <id>=<value>` で resume できる
  - 検証: 分類質問が発火する rule set を実行 → paused 確認 → response 指定で resume → 正しい分岐に進むことを確認
- [ ] **V8: fmt の key order** — `flowrail pipeline fmt` が pipeline.yml と rule-set.yml で異なる key order を適用する (kind: pipeline vs kind: rule-set で区別)
  - 検証: 両方の YAML を作成 → fmt 実行 → 出力の key 順序が各々の規約に従うことを確認
- [ ] **V9: 標準 rule set catalog 一貫性** — `rules/primitives/*.yml` (15 個) と `rules/recipes/*.yml` (10 個) が spec の定義と完全一致
  - 検証: `flowrail pipeline lint rules/primitives/*.yml rules/recipes/*.yml` が 0 エラー、かつ spec の "Standard Rule Set Catalog" 表との diff が 0
- [ ] **V10: atomic write crash recovery** — `.flowrail/state.json.tmp.*` が残っている状態で flowrail 起動 → 検出・削除される
  - 検証: tmp ファイルを手動で作成 → `flowrail run next` → tmp が削除され warning がログ出力される
- [ ] **V11: env var pass-through** — `CLAUDE_SESSION_ID`, `CLAUDE_PROJECT_DIR` が flowrail 経由で hook script に transparent に渡る
  - 検証: `echo "$CLAUDE_SESSION_ID" > /tmp/test` を実行する hook を仕込む → flowrail 経由で実行 → /tmp/test に値が書かれることを確認
- [ ] **V12: schema migration — phase-summaries → state.json** — 現行の `phase-summaries/*.yml` を `flowrail state import` で読み込めて、`state.phases[]` に正しく projection される
  - 検証: 既存 phase-summaries/design.yml + spec-review.yml を `flowrail state import` → state.json の phases[0], phases[1] に同等情報が入ることを確認
- [ ] **V13: Determinism Mode** — `FLOWRAIL_NOW=<fixed>` + `FLOWRAIL_SEED=<fixed>` で同一入力から同一出力が得られる (JSON output のバイト一致)
  - 検証: 同じ pipeline を 2 回実行 → JSON output の sha256 が一致することを確認
- [ ] **V14: Event Stream 構造** — `.flowrail/runs/{run_id}/events.jsonl` が各 step 毎に 1 行の JSON event を記録し、`command.invoked` / `command.completed` を含む
  - 検証: 1 pipeline 実行 → JSONL が生成 → `jq '.event_type'` で全 event type が定義通り列挙されることを確認
- [ ] **V15: Cargo.lock の固定** — `Cargo.lock` がコミット対象であり、clone 直後の `cargo build --locked --workspace` が成功する
  - 検証: CI で `--locked` flag 付きビルドを常時実行

### 実装への反映

Impact Analysis の結果を Phase 1 Plan に以下の形で反映する:

1. **Task 22 (実パイプライン統合検証)** に V1-V5 (skill 互換性) の検証を含める
2. **Task 20 (Event Stream 基盤)** に V14 の検証を含める
3. **Task 21 (Deterministic Mode)** に V13 の検証を含める
4. **Task 23 (dotfiles bin への symlink + 全体 smoke)** に V11 (env var pass-through) の検証を含める
5. **Task 24 (Phase 1 完了マニフェスト)** に V1-V15 の全 PASS を gate として明示

---

## Supported Platforms

### Phase 1-2 MVP: macOS + Linux のみ

flowrail は **POSIX shell** を前提とする実装を含むため、Phase 1-2 では macOS および Linux のみをサポートする。

**POSIX 依存箇所**:
- `cmd_exit` primitive: `sh -c "<command>"` で外部コマンド実行
- Hook Executor: `sh -c "<command>"` で hook コマンド実行
- `setsid` による process group 分離 (hook の orphan 防止)
- `flock(2)` (Phase 2 以降の並列実行対応時)

### Windows サポート (Phase 2 以降の拡張候補)

Windows サポートは以下のアプローチで Phase 2 以降に検討する:

1. **Git for Windows (`sh.exe`) 検出**: PATH 上に `sh.exe` があれば利用
2. **WSL 要求**: エラーメッセージで WSL 利用を promote
3. **Native fallback**: `cmd /C` + Windows 特有のパス正規化

ただし `ratatui` + `crossterm` (Phase 3 TUI) は Windows native サポート済みのため、Phase 3 で Windows 対応を検討する際は core subcommand (`flowrail pipeline lint/fmt`) から段階的に対応する。

### 明示的な Non-Support (Phase 1-2)

- Windows (Git Bash 除く): `cmd_exit` / hook の `sh -c` が動作しない
- Bare Windows: 上記の fallback なし

ユーザーが Windows で flowrail を起動すると、起動時に明示的なエラーメッセージ:
```
Error: flowrail currently supports macOS and Linux only.
       Windows support is planned for Phase 2+ (tracked in CLA-X).
       For now, please use WSL2 or Git Bash as a workaround.
```

---

## Migration Strategy

現行 `workflow-engine/` + `feature-dev/` + `debug-flow/` + `linear-sync/` + `triage/` を新アーキテクチャに移行する手順。

### Phase A: flowrail 実装

1. Rust プロジェクト `tools/flowrail/` 作成
2. Phase 1: `flowrail pipeline lint` + `flowrail pipeline fmt`（現行 pipeline.yml に対応）
3. Phase 2: `flowrail run` + `flowrail state` + `flowrail snapshot` + `flowrail pipeline test` + rule set resolver + 4 core primitive + Testing Framework
4. Phase 3: `flowrail tui`
5. Phase 4: `flowrail pipeline init` + 安定化

### Phase B: Standard Rule Set Catalog 作成

`rules/primitives/*.yml` 15 個と `rules/recipes/*.yml` 10 個を作成し、`claude/rules/` 配下に配置 (skills と並列、OSS 化時の切り出しやすさを優先)。

### Phase C: パイプライン移行

1. `claude/skills/feature-dev/pipeline.yml` を新形式で書き換え (kind: pipeline + layer メタデータ + imports + flags 等)
2. `claude/skills/debug-flow/pipeline.yml` を新形式で書き換え
3. `claude/skills/triage/pipeline.yml` を新規作成 (現行 triage SKILL.md を分解)
4. Inner Loop を 3 sub-phase に展開
5. Autonomy ルールを `phases[].confirm` + rule set に置き換え

### Phase D: Skill の薄いラッパー化

1. `workflow-engine/SKILL.md` を ~30 行に削減（flowrail 呼び出しループのみ）
2. `workflow-engine/modules/*.md` を全削除（rule set が代替）
3. `feature-dev/SKILL.md` / `debug-flow/SKILL.md` を薄いエントリーポイントに
4. `linear-sync/SKILL.md` を、`resolve_ticket.md`（LLM 対話）と CLI スクリプト（hook 実装）に分解

### Phase E: 検証と切り替え

1. flowrail + 新 rule set で feature-dev / debug-flow / triage を実行
2. 現行と新の動作差分を確認
3. snapshot (create/restore) / trigger 発火 / linear 統合 の確認
4. 旧ファイルを archive、新ファイルに一斉置換

### 削除される既存コード

| 削除対象 | 行数 | 代替 |
|----------|------|------|
| `workflow-engine/modules/audit.md` | 463 | `rules/recipes/audit-gate.yml` + `flowrail run verify` |
| `workflow-engine/modules/inner-loop.md` | 219 | 3 sub-phase 展開 + `regate-router` recipe (triggers は `flowrail run step` が自動評価) |
| `workflow-engine/modules/autonomy.md` | 143 | `phases[].confirm` + rule set の `phase-confirm-*` primitive |
| `workflow-engine/modules/regate.md` | 29 | `rules/recipes/regate-*.yml` + `flowrail run step` (triggers 統合) |
| `workflow-engine/modules/resume.md` | 80 | `flowrail snapshot restore` + state.integrations |
| `workflow-engine/modules/phase-summary.md` | 100 | `rules/recipes/phase-summary-yaml.yml` + state.phases[].{concerns,directives,validation_record} |
| `workflow-engine/modules/context-budget.md` | 45 | `settings.default_snapshot_hint_after_phases` |
| **合計** | **1,079 行** | rule set + flowrail core |

> **計測時点**: commit `591c21c` (2026-04-05)。初期開発時点のスナップショットであり、以降 `modules/*.md` の軽微な変更に spec の数値を追従させない。再計測する場合は `wc -l claude/skills/workflow-engine/modules/*.md` で実施する。

**ネット削減**: ~1,079 行散文を削除、代わりに ~400 行の rule set YAML + flowrail core code に置き換え。散文の曖昧性がなくなり、`flowrail pipeline lint` で静的検証可能になる。

### SKILL.md 移行例

現行の `claude/skills/workflow-engine/SKILL.md`（114 行、commit `591c21c` 時点）は、flowrail 呼び出しループのみの ~30 行に縮小される。例:

```markdown
# workflow-engine (thin skill)

1. `flowrail run init --pipeline {pipeline-path} {flags}` で開始
2. ループ:
   - `flowrail run next` で次 phase の情報を取得
   - phase の実行指示に従い作業
   - `flowrail run verify` で checks を実行
   - LLM が validation を実施
   - `flowrail run step --validation-result pass|fail` で前進
   - 状態が `paused` / classifier question が返ってきたらユーザーに escalation
3. context budget が近づいたら `flowrail snapshot create --label mid-impl`
4. 次セッションでは `flowrail snapshot restore mid-impl` で再開
```

regate / audit / handover / inner-loop はすべて rule set が処理するため、SKILL.md からは対応する散文が全て消える。

## Flag System

pipeline.yml の `flags:` セクションで動的フラグを宣言する（現行 spec に無い新機能）。**この spec の Flag System セクションが単一の SSOT**。3 レイヤー構造の Layer 3 example や Technology Stack の記述はここを参照する。

### スキーマ (SSOT)

```yaml
flags:
  <flag_name>:                       # 例: --linear, --iterations
    type: bool | integer | string    # 必須
    default: <value>                 # 任意。型に応じたデフォルト値
    enables:                         # 任意。フラグ有効時に有効化される対象
      integrations: [<name>, ...]    # 指定 integration を enabled 状態にする
      phases: [<phase_id>, ...]      # 指定 phase を実行対象に含める (skip_unless の代替)
      params:
        <rule_set>:
          <param_name>: <value>      # 指定 rule set の param を override
    binds_to_param:                  # 任意。integer/string 型で値を rule set param に bind
      rule_set: <name>
      param: <param_name>
```

### 具体例

```yaml
flags:
  --linear:
    type: bool
    default: false
    enables:
      integrations: [linear-sync]
  --accept:
    type: bool
    default: false
    enables:
      phases: [accept-test]
  --doc:
    type: bool
    default: false
    enables:
      phases: [doc-audit]
  --iterations:
    type: integer
    default: 3
    binds_to_param:
      rule_set: review-parallel
      param: iteration_count
```

### フラグ効果 (2 通りの表現)

| 効果 | 書き方 |
|------|-------|
| integration を有効化 | `enables.integrations: [<name>, ...]` |
| phase を実行対象に含める | `enables.phases: [<phase_id>, ...]` |
| rule set param を override | `enables.params.<rule_set>.<param_name>: <value>` |
| rule set param に値を bind | `binds_to_param: { rule_set: <name>, param: <param_name> }` |

### `phases[].skip_unless` との関係

- `phases[].skip_unless: --accept` は**廃止**。`flags.--accept.enables.phases: [accept-test]` で代替する
- 移行期間中は両方サポートし、`flowrail pipeline lint` で `skip_unless` に warning を発し `flags.enables.phases` への移行を promote

### clap への実装戦略

flags は pipeline.yml を読まないと内容が分からない動的フラグ。clap の derive API (compile-time) だけでは扱えないため、**2-pass parse 戦略**を採る:

1. **第 1 pass**: `--pipeline <path>` のみを derive API で抽出 (他のフラグは全て `trailing_var_arg` で保留)
2. **pipeline.yml 読み込み**: 指定された pipeline.yml の `flags:` セクションを取得
3. **Builder API で動的 Arg 追加**: 第 1 pass で保留した残余引数を、pipeline.yml 宣言に基づいて Builder API の `Command::arg(...)` で動的に登録
4. **第 2 pass**: 動的登録済みの clap Command で再 parse

詳細は `src/cli.rs` の設計コメントに記載 (Phase 1 で実装)。

### 呼び出し例

```bash
flowrail run init --pipeline claude/skills/feature-dev/pipeline.yml --linear --accept --iterations 5
```

state.json に `flags: ["--linear", "--accept", "--iterations=5"]` が記録される。

## pipeline.yml Schema Changes

### 現行 v3 からの変更

**新設:**
- `kind: pipeline`（必須、rule set と区別）
- `imports:`（rule set への依存）
- `flags:`（動的フラグ宣言、SSOT は Flag System セクション）
- `uses:` pipeline トップレベル（pipeline-global な trigger や hook 宣言、compile-time に各 phase の triggers に normalize される）
- `pre_pipeline_start:`（LLM 側で実行する前処理スキル）
- `integrations[].pre_pipeline_start`（integration 固有の前処理）
- `integrations[].hooks.<event>` は**文字列または文字列配列** (単一 hook / 複数 hook 並列実行)

**廃止:**
- `pipeline:` トップレベルキー（`kind: pipeline` + `name: <id>` に置換）
- `modules:`（rule set に置き換え、後方互換として warning で無視）
- `regate:` ルート（rule set の `triggers` に統合）
- `phases[].uses: [inner-loop]`（inner-loop 戦略参照、sub-phase 展開に置き換え）
- `phases[].skip_unless: --<flag>`（`flags.--<flag>.enables.phases` に置換、移行期間中は warning 付きでサポート）
- `artifacts[].contract`（verification/validation は rule set 経由で宣言）

**維持:**
- `name`, `version`, `description`, `settings`, `phases`, `artifacts` の基本構造 (トップレベル `pipeline:` は `kind: pipeline` + `name:` に変更)

### Schema ファイル

- `schema/pipeline.schema.json` — Pipeline YAML
- `schema/rule-set.schema.json` — Rule Set YAML
- `schema/flowrail-state.schema.json` — state.json

## Technology Stack

### Rust Version

- **Edition**: `2024` (Rust 1.85 で安定化、十分成熟)
- **MSRV**: **`1.86.0`** — workspace 統一のため。ratatui 0.30 が 1.86 を要求するため Phase 3 のために先取り統一
- **推奨開発版 / CI toolchain**: **`1.94.1`** 以上 (CVE-2026-33055 / CVE-2026-33056 を回避、Cargo 同梱の `tar` crate 脆弱性修正)
- **`rust-toolchain.toml`**: 1.94.1 を固定 (ローカル開発・CI 用)
- **将来の bump 検討**: `notify` v9 stable (MSRV 1.88) 採用時に 1.88 へ再評価

### 言語: Rust

- 状態遷移を `enum` + `match` で網羅性保証
- シングルバイナリ配布 (agent CLI / TUI CLI の 2 バイナリ、Cargo workspace で分離)
- ~1ms 起動速度

### Workspace `Cargo.toml` (SSOT)

`[workspace.dependencies]` で全依存を一元管理する:

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.86"
authors = ["neko-neko"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/neko-neko/flowrail"

[workspace.dependencies]
# --- serialization / YAML ---
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0"
serde-saphyr = "=0.0.23"  # 0.0.x pre-1.0, completely pinned (SemVer 前)

# --- JSON Schema ---
jsonschema = { version = "=0.45.0", default-features = false, features = ["resolve-file"] }

# --- template ---
minijinja = { version = "2.19", default-features = false, features = ["builtins", "macros", "deserialization"] }

# --- error handling ---
miette = "7.6"  # plain, for flowrail-core
thiserror = "2.0.18"

# --- CLI (flowrail binary only) ---
clap = { version = "4.6", features = ["derive"] }

# --- filesystem watcher ---
notify = "8.2"  # v9 rc skipped until stable (v9 requires MSRV 1.88)

# --- utilities ---
uuid = { version = "1.23", features = ["v4", "v7", "v5", "serde"] }
regex = "1.12"
glob = "0.3"
toml = "0.9"

# --- TUI (flowrail-tui only, Phase 3) ---
ratatui = "=0.30.0"  # 0.x, pinned (monolithic → modular workspace 再構成)
crossterm = "=0.29.0"  # 0.x, pinned (ratatui backend)
```

### 依存 crate (各 crate 別)

**`flowrail-core`** (library, Phase 1):
```toml
[dependencies]
serde.workspace = true
serde_json.workspace = true
serde-saphyr.workspace = true
jsonschema.workspace = true
minijinja.workspace = true
miette.workspace = true        # plain, no fancy feature
thiserror.workspace = true
notify.workspace = true
uuid.workspace = true
regex.workspace = true
glob.workspace = true
```

**`flowrail`** (agent CLI binary, Phase 1):
```toml
[dependencies]
flowrail-core = { path = "../flowrail-core" }
clap.workspace = true
miette = { workspace = true, features = ["fancy-no-backtrace"] }  # fancy は top-level binary のみ
# ⚠ TUI 系依存 (ratatui, crossterm) は一切追加禁止
```

**`flowrail-tui`** (human CLI binary, Phase 3):
```toml
[dependencies]
flowrail-core = { path = "../flowrail-core" }
ratatui.workspace = true
crossterm.workspace = true
miette = { workspace = true, features = ["fancy-no-backtrace"] }
```

### 依存バージョンの由来と理由

| crate | 指定 | 最新版 | 理由 |
|-------|------|--------|------|
| `clap` | `"4.6"` | 4.6.0 (2026-03) | 2024 edition 移行済み、MSRV 1.85 |
| `serde` | `"1.0.228"` | 1.0.228 (2025-09) | 安定 |
| `serde-saphyr` | **`"=0.0.23"`** | 0.0.23 (2026-03) | **0.0.x pre-1.0、完全ピン必須**。`serde_yaml_bw` の maintainer 自身が推奨、唯一 panic-free + budget control + merge key/anchor 全対応 |
| `jsonschema` | **`"=0.45.0"`** + HTTPS 除外 | 0.45.0 (2026-03) | **0.x 完全ピン**、default の `resolve-http` + `tls-aws-lc-rs` は disable 必須 (supply chain 縮小) |
| `minijinja` | `"2.19"` | 2.19.0 (2026-04) | API 互換、`default-features = false` + 必要 features のみ |
| `miette` | `"7.6"` | 7.6.0 (2025-04、**1 年更新停滞**) | 代替候補 `annotate-snippets` を watch |
| `ratatui` | **`"=0.30.0"`** | 0.30.0 (2025-12) | **0.x 完全ピン**、monolithic → modular 再構成、MSRV 1.86 要求 |
| `crossterm` | **`"=0.29.0"`** | 0.29.0 (2025-04) | **0.x 完全ピン**、ratatui backend |
| `thiserror` | `"2.0.18"` | 2.0.18 (2026-01) | 2.0 系安定 |
| `notify` | `"8.2"` | 8.2.0 (2025-08、**v9.0.0-rc.2 は保留**) | v9 stable で MSRV 1.88 bump 予定、v8 で当面維持 |
| `uuid` | `"1.23"` | 1.23.0 (2026-03) | `v4`, `v7`, `v5` features 有効化 (deterministic 用に `v5`) |
| `regex` | `"1.12"` | 1.12.3 (2026-02) | デフォルト features で十分 |

### YAML Abstraction Layer (Phase 1 必須)

`src/yaml/mod.rs` を新設し、`serde-saphyr` への直接依存をこのモジュールのみに局限する。他のモジュール (`pipeline/loader.rs`, `ruleset/loader.rs`, `state/store.rs`) は必ずこの抽象層経由で YAML を扱う。

```rust
// src/yaml/mod.rs (Phase 1 で実装)
pub fn parse<T: serde::de::DeserializeOwned>(src: &str) -> Result<T, YamlError>;
pub fn parse_with_options<T: serde::de::DeserializeOwned>(src: &str, opts: YamlOptions) -> Result<T, YamlError>;
pub fn serialize<T: serde::Serialize>(value: &T) -> Result<String, YamlError>;

pub struct YamlOptions {
    pub duplicate_keys: DuplicateKeyPolicy,  // FirstWins | LastWins | Error
    pub strict_booleans: bool,
    pub budget: Option<Budget>,               // max_anchors, max_depth, max_events
}

pub enum DuplicateKeyPolicy { FirstWins, LastWins, Error }
pub struct YamlError { /* ... */ }
```

**契約**:
- 他モジュールは `yaml::parse::<T>(...)` / `yaml::serialize(...)` のみを使用
- `serde-saphyr` の型 (`Options`, `Budget`) は `yaml::*` 経由で公開
- 将来 `serde-saphyr` から別 crate (例: `serde_yaml_bw`, `serde_yaml_ng`, `serde_norway`) への差し替えは `src/yaml/mod.rs` のみの変更で完結
- `flowrail pipeline lint` は YAML subset (duplicate keys / merge keys / anchors / source-span) の contract テストを持つ

### YAML Parser Contract (保証すべき振る舞い)

`yaml::parse` 実装は以下を保証する:

| 機能 | 振る舞い |
|------|---------|
| Duplicate keys | `DuplicateKeyPolicy::Error` がデフォルト (spec では曖昧な duplicate key は lint エラー) |
| Merge keys (`<<: *alias`) | サポート (現行 feature-dev pipeline.yml で利用中) |
| Anchors / aliases | サポート |
| Source spans | エラー時に `line:column` を返す (miette との統合) |
| Normalization | `yaml::serialize` は canonical 出力 (キー順固定、インデント 2、末尾改行) |
| DoS 防御 | `Budget` で `max_anchors=200`, `max_depth=100`, `max_events=50000` がデフォルト |

### Git 操作

`git2` / `gix` は使用しない。`std::process::Command` で git CLI を直接呼び出す。C 依存を排除し、ユーザーの git 設定との完全一致を保証する。

### バイナリサイズ対策

成功基準 #9 (20MB 以下) を達成するため:

1. `jsonschema` の `default-features = false, features = ["resolve-file"]` で `reqwest` / `rustls` / `aws-lc-rs` / `tokio` 依存を外す
2. `miette` の `fancy` feature は使わず `fancy-no-backtrace` のみ
3. `cargo build --release` + `strip` symbols
4. LTO 有効化 (`Cargo.toml` の `[profile.release] lto = true`)
5. Phase 3 の `ratatui` 依存は feature flag 化 (`tui` feature、デフォルト off) を検討
6. CI でバイナリサイズ回帰検出 (Phase 2 完了時点で測定開始)

### Dependency Management Policy

**バージョン指定ポリシー:**

| crate 種別 | 指定方式 | 理由 |
|-----------|---------|------|
| **0.x crates** | `=X.Y.Z` **完全ピン** | SemVer 前、patch でも breaking 可能性 |
| **1.x crates** | `"X.Y"` caret 許容 | SemVer 準拠で安全 |

**完全ピン対象 (0.x)**: `serde-saphyr`, `jsonschema`, `ratatui`, `crossterm`
**caret 許容 (1.x+)**: `clap`, `serde`, `minijinja`, `miette`, `thiserror`, `uuid`, `regex`, `notify`

**Cargo.lock**: flowrail は binary project (library 公開ではない) のため `Cargo.lock` を**コミット対象**とする。再現可能なビルドを保証。

**更新ポリシー**:
- 0.x crates の更新は **必ず spec/CLAUDE.md と同時に commit** (changelog 確認必須)
- 1.x crates の更新は `cargo update` + `cargo audit` + テスト PASS で自動適用可
- 新規依存追加時は `cargo audit` と `cargo deny` を通す

**agent CLI (`flowrail`) の依存追加制限** (原則 8 "Separation by Audience" に基づく):

- **TUI/GUI ライブラリ追加禁止**: `ratatui`, `crossterm`, `tui-rs`, `iced`, `egui` 等
- **ネットワーク通信ライブラリ原則禁止**: `reqwest`, `hyper`, `tonic` 等。必要な場合は spec に justification を記載
- **非同期ランタイム原則禁止**: `tokio`, `async-std`, `smol` 等。同期実行を基本
- **unsafe code 原則禁止**: `#![forbid(unsafe_code)]` をクレート root に
- これらの制限は `flowrail-tui` (human CLI) には適用されない

### Known Risks (要監視)

| Risk | 影響 | 対策 |
|------|------|------|
| **`miette` 更新停滞** | 7.6.0 (2025-04-27) 以降リリースなし | 代替候補 `annotate-snippets` を watch、maintainer 活動を定期確認 |
| **`notify v9`** | rc.2 段階、v9 stable で MSRV 1.88 要求 | v8.2.0 で当面維持、v9 stable 時に再評価 + MSRV bump |
| **`serde-saphyr 0.0.x`** | SemVer 前、patch でも breaking 可 | `=0.0.23` 完全ピン、`cargo update` 前に changelog 確認 |
| **Rust 1.94.0 CVE** | Cargo 同梱 `tar` crate の CVE-2026-33055/33056 | **1.94.1+ を toolchain に固定** (修正済み) |
| **`jsonschema` default HTTPS** | default features に `resolve-http` + `tls-aws-lc-rs`、不要な TLS stack 取り込み | `default-features = false, features = ["resolve-file"]` 必須 |

## Crate 構造 (Cargo Workspace)

flowrail は Cargo workspace として構成される。**3 つの crate** に分離することで、agent CLI と human CLI を audience ごとに依存関係レベルで分離する (原則 8 参照)。

```
flowrail/                             # workspace root
├── Cargo.toml                        # [workspace] + [workspace.dependencies]
├── Cargo.lock                        # コミット対象
├── rust-toolchain.toml               # Rust 1.94.1 を固定
├── README.md
├── CLAUDE.md
├── .gitignore
├── docs/
│   ├── specs/
│   └── plans/
├── crates/
│   ├── flowrail-core/                # 📦 pure library (Phase 1)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── pipeline/
│   │       │   ├── mod.rs
│   │       │   ├── model.rs          # pipeline.yml の型定義
│   │       │   ├── loader.rs         # パース + schema 検証
│   │       │   └── graph.rs          # artifact 依存グラフ
│   │       ├── ruleset/
│   │       │   ├── mod.rs
│   │       │   ├── model.rs          # rule set の型定義
│   │       │   ├── loader.rs         # パース + schema 検証
│   │       │   ├── resolver.rs       # imports 解決 (max_depth による循環検出のみ)
│   │       │   ├── binder.rs         # parameter binding
│   │       │   ├── template.rs       # minijinja 統合
│   │       │   └── evaluator.rs      # checks / uses / triggers の評価
│   │       ├── state/
│   │       │   ├── mod.rs
│   │       │   ├── model.rs          # state.json の型定義
│   │       │   ├── store.rs          # atomic read/write
│   │       │   └── machine.rs        # phase/artifact state 遷移
│   │       ├── primitive/
│   │       │   ├── mod.rs
│   │       │   ├── file_exists.rs
│   │       │   ├── cmd_exit.rs
│   │       │   ├── regex_match.rs
│   │       │   └── git_status.rs
│   │       ├── engine/
│   │       │   ├── mod.rs
│   │       │   ├── next.rs           # flowrail run next の core logic
│   │       │   ├── verify.rs         # rule set 評価とチェック実行
│   │       │   └── step.rs           # flowrail run step: phase 進行 + triggers 自動評価
│   │       ├── snapshot/
│   │       │   ├── mod.rs
│   │       │   ├── store.rs          # .flowrail/snapshots/<label>.json の読み書き
│   │       │   └── lifecycle.rs      # create / restore / list / prune
│   │       ├── hook/
│   │       │   ├── mod.rs
│   │       │   ├── executor.rs       # env + stdin JSON
│   │       │   └── context.rs        # HookContext 構造体
│   │       ├── lint/
│   │       │   ├── mod.rs
│   │       │   ├── rules.rs          # セマンティック検証ルール
│   │       │   └── diagnostic.rs     # miette 統合
│   │       ├── fmt/
│   │       │   └── mod.rs            # YAML 正規化
│   │       ├── yaml/                 # serde-saphyr 抽象層 (直接呼び出し禁止)
│   │       │   └── mod.rs            # parse<T> / parse_with_options<T> / serialize<T>
│   │       ├── event/                # Phase 1 Infrastructure: JSONL event stream
│   │       │   ├── mod.rs
│   │       │   └── logger.rs
│   │       ├── deterministic/        # Phase 1 Infrastructure: FLOWRAIL_NOW/SEED
│   │       │   └── mod.rs
│   │       ├── test/                 # Phase 2: Testing Framework (5 primitives)
│   │       │   ├── mod.rs
│   │       │   ├── runner.rs
│   │       │   ├── recording.rs
│   │       │   ├── assertion.rs
│   │       │   └── diff.rs
│   │       └── error.rs              # thiserror + miette
│   │
│   ├── flowrail/                     # 🤖 agent 用 CLI binary (Phase 1)
│   │   ├── Cargo.toml                # 依存: flowrail-core + clap のみ (TUI 系一切なし)
│   │   └── src/
│   │       ├── main.rs               # entrypoint
│   │       ├── cli/                  # clap definitions
│   │       │   ├── mod.rs
│   │       │   └── subcommands.rs
│   │       ├── json_io.rs            # stdin/stdout JSON protocol
│   │       └── output/
│   │           ├── mod.rs
│   │           ├── json.rs
│   │           └── markdown.rs
│   │
│   └── flowrail-tui/                 # 👤 human 用 TUI binary (Phase 3)
│       ├── Cargo.toml                # 依存: flowrail-core + ratatui + crossterm
│       └── src/
│           ├── main.rs               # Phase 1 時点: placeholder
│           ├── app.rs                # Phase 3: main loop
│           ├── ui/                   # Phase 3: ratatui widgets
│           │   ├── mod.rs
│           │   ├── state_view.rs
│           │   ├── phase_progress.rs
│           │   └── log_viewer.rs
│           └── input.rs              # Phase 3: crossterm key events
│
├── catalog/                          # Standard Rule Sets (Phase 1 以降で充填)
│   ├── primitives/                   # ~15 primitive rule sets
│   └── recipes/                      # ~10 recipe rule sets
├── examples/                         # サンプル pipeline (Phase 1 以降)
│   ├── feature-dev.yml
│   ├── debug-flow.yml
│   └── triage.yml
└── schema/                           # JSON schemas
    ├── pipeline.schema.json
    ├── rule-set.schema.json
    └── flowrail-state.schema.json
```

### 依存関係フロー

```
flowrail-core (pure library)
     │
     ├──▶ flowrail (agent bin, minimal deps)
     │
     └──▶ flowrail-tui (human bin, UI deps isolated)
```

**重要**: `flowrail` と `flowrail-tui` の間に依存関係は**存在しない**。両者は独立して `flowrail-core` を参照する。

これにより:
- `cargo build -p flowrail` で agent バイナリのみビルド → ratatui/crossterm を一切取り込まない
- `cargo tree -p flowrail` で agent 用依存グラフを独立に audit 可能
- `cargo audit` / `cargo deny` を crate 単位で適用可能

## リポジトリ構成

flowrail は独立したリポジトリ `https://github.com/neko-neko/flowrail` として管理される (local path: `~/go/src/github.com/neko-neko/flowrail/`)。

元々は `~/.dotfiles/claude/skills/workflow-engine/` 内の workflow engine として発足したが、OSS 公開と自己完結化を見据えて 2026-04-05 に独立リポジトリへ分離された。

**`dotfiles` との関係**:
- dotfiles の `claude/skills/workflow-engine/` は本 spec の設計インスピレーション元 (upstream)
- flowrail は dotfiles から完全に独立したプロダクトとして発展する
- 将来、dotfiles の既存 pipeline.yml (feature-dev, debug-flow, triage 等) を flowrail の rule set として書き換える可能性はあるが、それは別プロジェクト

## Testability

flowrail は **Phase 1 で最小限の observability infrastructure** を提供し、**Phase 2 で rule set 作者向けの Testing Framework (5 primitives)** を追加する。両フェーズの境界を明確にし、Phase 1 の flowrail core を tiny に保つ。

### 責務分離（Phase 1 / Phase 2 / 外部ツール）

| 機能 | Phase 1 | Phase 2 | 外部ツール |
|------|---------|---------|-----------|
| Event Stream (`run.id` 付き JSONL) | ✓ | — | — |
| Deterministic Mode (`FLOWRAIL_NOW` / `FLOWRAIL_SEED`) | ✓ | — | — |
| Dry-Run Mode (`flowrail run <verb> --dry-run`) | ✓ | — | — |
| Recording Mode (`--record`) | — | ✓ | — |
| Recording Replay (`--replay-from`) | — | ✓ | — |
| Assertion Mode (`--assert-recording`) | — | ✓ | — |
| State Diff (`flowrail state diff`) | — | ✓ | — |
| Pipeline Test Runner (`flowrail pipeline test`) | — | ✓ | — |
| N 回実行 + consistency 計算 | — | — | ✓ (shell / Python ループで flowrail を呼び集計) |
| Golden file 管理 / diff visualization | — | — | ✓ (専用ツール or CI) |
| LLM バージョン tracking | — | — | ✓ (外部メタデータ管理) |

flowrail は N 回実行の概念を持たない。外部ツールがループで flowrail を呼び、event stream (`run.id` 付き) を集約して consistency score、分散、回帰を計算する。

---

### Phase 1 Infrastructure (全 subcommand の共通基盤)

#### Event Stream

全 `flowrail` 呼び出しは構造化 event を JSONL で発行。全 event に `run.id` (UUID) を付与し、外部ツールが N run を識別できるようにする。出力先は `FLOWRAIL_EVENTS_FILE=<path>` で指定。未指定時は stderr に fallback (human-readable フィルタ可能)。

共通フィールド:
- `run.id` (UUID v7、同一 run 内の全 event で共通)
- `timestamp` (ISO8601、`FLOWRAIL_NOW` で固定可能)
- `event` (種別)
- `payload` (種別ごとの構造化データ)

| イベント | 説明 | Phase |
|---------|-----|-------|
| `command.invoked` | CLI エントリ時 | 1 |
| `command.completed` | CLI 終了時 | 1 |
| `state.loaded` | state.json 読み込み | 1 |
| `state.mutated` | state.json 書き込み（diff は JSON Patch 形式） | 1 |
| `phase.transition` | phase の status 変化 | 1 |
| `ruleset.resolved` | rule set の imports 解決完了 | 1 |
| `ruleset.evaluated` | rule set の uses / checks 評価完了 | 1 |
| `verification.check` | primitive check の結果 | 1 |
| `validation.received` | LLM からの validation 結果 | 1 |
| `llm_response.received` | 実 LLM から受け取った応答 (validation / classifier) | 1 (記録のみ、Phase 2 で Recording と連携) |
| `trigger.fired` | rule set の trigger が発火し action (regate/pause/complete) が実行された | 1 |
| `classifier.invoked` | classifier action 実行 | 1 |
| `hook.fired` / `hook.completed` | hook ライフサイクル | 1 |
| `snapshot.created` / `snapshot.restored` | snapshot ライフサイクル | 1 |

#### Deterministic Mode

`--deterministic` フラグまたは `FLOWRAIL_DETERMINISTIC=1` で有効化:
- **時刻固定**: `FLOWRAIL_NOW=2026-04-05T10:00:00Z` (未指定なら現在時刻、有効化時はエラー)
- **UUID 決定化**: `FLOWRAIL_SEED=<seed>` で `run.id` 生成を seeded RNG 化 (v7 ではなく seeded v5 を使用)
- **JSON/YAML canonical 出力**: キー順固定、空白・改行正規化、byte-identical 保証
- **用途**: Phase 2 の Assertion Mode (strict) の前提、CI での regression detection

#### Dry-Run Mode

`flowrail run <verb> --dry-run`:
- checks を実行しない（予定のみ出力）
- hook を発火しない（予定のみ出力）
- state.json への書き込みなし
- 各 subcommand ごとに独立したフラグ (`flowrail run init --dry-run`, `flowrail run step --dry-run` 等)

### 観測すべき観点 (Phase 1 でカバーされる 6 項目 + Phase 2 で追加の 1 項目)

1. **ノイズ** (Phase 1) — LLM が不要な出力を出していないか → `output.emitted` イベントの `stdout_bytes`
2. **コンテキスト** (Phase 1) — LLM に何が渡されたか → `flowrail run next` の出力 + event stream
3. **判断** (Phase 1) — LLM がどういう validation / classifier 判断を下したか → `validation.received`, `llm_response.received` イベント
4. **進行** (Phase 1) — step したか、regate したか、止まったか → `phase.transition` イベント
5. **指摘** (Phase 1) — どんな findings が出てきたか → `verification.check` の失敗結果 + `trigger_history`
6. **Rule set 評価** (Phase 1) — どの rule set がどの値で評価されたか → `ruleset.evaluated` イベント
7. **揺らぎ** (Phase 2) — 同一入力に対する LLM 応答の差分 → baseline recording と current recording の比較

---

### Phase 2: Testing Framework (5 primitives)

Phase 2 で以下の 5 primitives を追加する。Phase 1 では **未実装**。原則 7 の「最小限の 5 primitives のみ」と整合する。

| # | Primitive | 概要 | CLI |
|---|-----------|------|-----|
| **1** | Recording Mode | 実 LLM 応答 + event stream + check 詳細 + 最終 state を JSONL に記録 | `flowrail run <verb> --record <path>` |
| **2** | Recording Replay | 記録済み JSONL から LLM 呼び出しなしで完全再現 | `flowrail run <verb> --replay-from <path>` (YAML scenario または recording.jsonl 両対応) |
| **3** | Assertion Mode | baseline recording と current recording を比較 | `flowrail run <verb> --assert-recording <baseline> --assert-level strict\|loose` |
| **4** | State Diff | 2 つの state.json を byte-level / semantic diff | `flowrail state diff <a> <b> [--ignore <fields>]` |
| **5** | Pipeline Test Runner | rule set の `tests:` セクションを一括実行 | `flowrail pipeline test [path...] [--filter <pattern>]` |

#### Phase 2 設計ルール (原則 7「最小限」の定量境界)

- **Assertion 表現力の上限**: `verdict` (`PASS`/`FAIL`) + `checks_passed` / `failed_checks` の set 比較のみ。カスタム matcher / 正規表現 / 値比較は提供しない
- **Strict mode**: Deterministic Mode 併用前提、byte-identical 比較
- **Loose mode**: event 順序 + check 結果 set + LLM 応答の最終 verdict が一致すればよい (raw text は比較しない、timestamps/durations/run.id は自動 ignore)
- **N 回実行・consistency 計算・可視化**: 外部ツールの責務
- **Golden file 自動更新 / diff visualization**: non-goal
- **LOC 見積**: Testing Framework subsystem 全体で ~600-1,000 LOC (Phase 1 の ~3,500-3,800 LOC に加算、Phase 2 完了時 ~4,400 LOC)

#### Phase 2 で追加される Rule Set Schema の `tests:` セクション

Phase 2 で rule set schema に `tests:` を追加する。Phase 1 では schema に定義されない。

```yaml
# rules/recipes/audit-gate.yml (Phase 2 対応)
kind: rule-set
name: audit-gate

tests:
  - name: "valid spec file passes"
    given:
      params:
        artifact_path: "fixtures/valid-spec.md"
        required_sections: [requirements, components]
      replay:
        validation-question:
          - { id: req-capture, result: pass }
    expect:
      verdict: PASS
      checks_passed: [check-file-exists, check-sections-present]

  - name: "missing sections fails"
    given:
      params:
        artifact_path: "fixtures/incomplete-spec.md"
        required_sections: [requirements, components]
    expect:
      verdict: FAIL
      failed_checks: [check-sections-present]
```

`flowrail pipeline test` が rule set を再帰的に collect し、各テストを replay + assertion で実行する。

#### N 回実行 + 揺らぎ測定の外部連携例 (Phase 2 完了後)

```bash
# 外部シェルスクリプト例: N=20 回 LLM を呼んで consistency 測定
for i in $(seq 1 20); do
  flowrail run step \
    --validation-result req-capture=pass \
    --record "runs/run-$i.jsonl" \
    --deterministic \
    --seed "$i"
done

# 外部ツールが run.id で集約し、consistency score を計算
python consistency.py runs/*.jsonl
```

flowrail 自身は N 回実行の概念を持たない。run.id 付きの event stream を出力するだけで、集約・統計・可視化は外部ツールの責務。

## 実装フェーズ

### Phase 1: `flowrail pipeline lint` + `flowrail pipeline fmt`（MVP）— **flowrail core ~3,500-3,800 LOC**

- Rust プロジェクト初期化（MSRV 1.85, edition 2024）
- pipeline.yml + rule set の型定義（serde）
- YAML abstraction layer (`src/yaml/mod.rs`) — serde-saphyr を wrap
- JSON Schema 検証（jsonschema crate、`default-features = false, features = ["resolve-file"]`）
- rule set の imports 解決（`max_depth` による再帰深度制限のみ、事前 cycle detection は行わない。layer metadata による逆依存検証は flowrail core の関心外 — Conventions & Best Practices 参照）
- Template 静的解析（regex ベースで `{{ ... }}` 参照抽出、minijinja machinery 非依存）
- セマンティック lint ルール
- YAML フォーマッタ
- **Phase 1 Infrastructure**:
  - Event Stream（全イベント型、`run.id` 付き JSONL、`FLOWRAIL_EVENTS_FILE`）
  - Deterministic Mode（`FLOWRAIL_NOW` / `FLOWRAIL_SEED` / JSON/YAML canonical 出力）
  - Dry-Run Mode（各 `flowrail run <verb> --dry-run`）
- 既存の feature-dev / debug-flow pipeline.yml + 新 rule set 試作で動作確認

### Phase 2: `flowrail run` + `flowrail state` + `flowrail snapshot` + Rule Set Evaluation + Testing Framework — **flowrail core 合計 ~4,400 LOC**

- state.json / snapshot ファイル の型定義・永続化（atomic write + compare-and-set、single-write transaction）
- `run init` / `next` / `verify` / `step`（triggers 自動評価、action 実行、pipeline-root `uses:` の global-trigger 展開）
- `state show` / `list` / `reset` / `prune`
- `snapshot create` (same-label error, `--force` で上書き) / `restore` / `list` / `prune`
- 4 core primitive check 実装
- Rule set evaluator（uses, checks, for-each, when, classifier, triggers, on_phase_complete、8 built-in directive、classifier の `on_unknown_response`/`on_timeout` エラー処理、`max_retries` の優先順位ルール）
- JSON / Markdown 出力
- Idempotency Contract の実装（各 subcommand の中断点からの再開挙動含む）
- Hook System（env + stdin JSON、15 分タイムアウト、fire-and-forget、state commit 後の post-action、`on_trigger_fired` / `on_snapshot_created` 発火）
- **Testing Framework (5 primitives、~600-1,000 LOC)**:
  - Recording Mode (`--record`)
  - Recording Replay (`--replay-from`、YAML scenario と recording.jsonl 両対応)
  - Assertion Mode (`--assert-recording`, `--assert-level strict|loose`)
  - `flowrail state diff` (byte-level / semantic diff)
  - `flowrail pipeline test` (rule set の `tests:` セクション実行)
- Rule Set Schema 拡張 (`tests:` セクション、Phase 2 で追加)
- SKILLS 再設計（workflow-engine SKILL.md の薄い化）
- Standard Rule Set Catalog 作成（~15 primitive + ~10 recipe）

### Phase 3: `flowrail-tui` (別 binary crate)

- 新しい workspace member `crates/flowrail-tui/` を実装 (Phase 1 では placeholder のみ)
- ratatui アプリケーション (MSRV 1.86 要求、crate は既に workspace 統一 1.86 のため影響なし)
- state.json watch + リアルタイム phase/artifact/trigger 表示
- crossterm による key event handling
- 依存は `flowrail-core` + `ratatui` + `crossterm` のみ。`flowrail` (agent CLI) とは依存関係を持たない (原則 8 参照)

### Phase 4: `flowrail pipeline init` + `flowrail help` + 安定化

- スキャフォールド生成（pipeline テンプレート）
- `flowrail help` の man page 風コンテンツ整備
- `.flowrail/config.toml` 対応（build/test/lint コマンド設定）
- エラーメッセージ磨き込み
- ドキュメント

## 成功基準

> **計測時点**: commit `591c21c` (2026-04-05)。行数・バイト数基準は初期開発時点のスナップショットで、以降保守しない。

1. `flowrail pipeline lint` が既存 + 新 pipeline.yml のスキーマ変更波及漏れを検出できる（0 false-negative）
2. `flowrail run` でパイプラインを駆動し、SKILLS のコンテキスト消費が削減される (commit `591c21c` 時点の実測 `workflow-engine/SKILL.md` 114 + `modules/*.md` 1,079 = **1,193 行 → ~30 行**)
3. phase 遷移・trigger 発火・snapshot が LLM の解釈に依存せず、state.json で決定論的に管理される (**検証条件**: `--deterministic` + replay で N 回実行した結果の state.json が byte-identical)
4. `workflow-engine/modules/*.md` の散文 **1,079 行** (commit `591c21c` 実測、audit 463 + autonomy 143 + context-budget 45 + inner-loop 219 + phase-summary 100 + regate 29 + resume 80) が削除され、rule set YAML + flowrail core code に置き換わる
5. **冪等性**: 全サブコマンドが Idempotency Contract の表通りに動作する (**検証条件**: 表の全 17 行について 2 回連続実行で state.json が byte-identical)
6. **拡張性**: 新しい rule set（例: カスタム audit gate、新 regate 戦略）を、flowrail のコード変更なしで追加できる (**検証条件**: `rules/recipes/<custom>.yml` を追加し `flowrail pipeline lint` が通れば OK)
7. **再現性**: feature-dev / debug-flow / triage / linear-sync hooks の全機能が新アーキテクチャで再現可能である ([Feasibility Mapping セクション参照](#feasibility-mapping-tbd-before-phase-a)、**Phase A 着手前に別セッションで完成予定**)
8. **テスト可能性 (Phase 2)**: 既存の feature-dev pipeline.yml の完全な実行を、LLM を呼び出さずに replay + deterministic mode で再現できる (byte-identical state.json、timestamps のみ ignore)
9. **バイナリサイズ**: Phase 2 完了時点で release ビルドが 20MB 以下。計測条件: `cargo build --release --target x86_64-unknown-linux-gnu` + strip symbols、`jsonschema` は `default-features = false, features = ["resolve-file"]`
10. **Rule set カタログ**: 標準 primitive (~15 個) と recipe (~10 個) が同梱され、`flowrail pipeline init --template feature-dev` で新パイプラインが生成される (Phase 4)
11. **Rule set 作者の品質保証 (Phase 2)**: rule set 作者が flowrail 標準機能のみで以下を実現できる:
    - a. 実 LLM を呼んだ 1 run の全応答 + event stream を recording file (`--record`) に保存 (JSONL、Event Stream 全種別 + `llm_response.received`)
    - b. recording file を replay (`--replay-from`) として後日完全再現 (strict mode で byte-identical)
    - c. baseline recording と current recording を比較 (`--assert-recording`) して回帰検出 (strict / loose mode)
    - d. rule set に `tests:` セクションを宣言し、`flowrail pipeline test` で一括検証 (assertion は verdict + checks_passed / failed_checks の set 比較のみ)
12. **外部ツールとの連携による揺らぎ検証 (Phase 2 完了後)**: 外部ツールが flowrail の event stream (`run.id` 付き) を集約して以下を実現できる:
    - a. N 回実行による LLM 応答の分散測定 (consistency score)
    - b. LLM バージョン変更の回帰検出 (baseline recording との差分)
    - c. flowrail 自身は N 回実行の概念を持たない (tiny 原則維持)
