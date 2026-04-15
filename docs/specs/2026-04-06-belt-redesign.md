# belt Redesign: LLM 向け超軽量ワークフローエンジン CLI

- **Date**: 2026-04-06
- **Status**: Approved
- **Linear**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)

## 1. Problem Statement

LLM にワークフロー全体を制御させるとコンテキストを大量に消費する。現行の SKILL.md ベースのワークフロー（feature-dev: ~579行 + done-criteria 8ファイル）は、パイプライン構造・遷移ロジック・ゲートチェックを全て LLM のコンテキストに載せる必要がある。

また、パイプライン定義が自然言語に埋め込まれているため、構造的正しさの静的検証ができない。フェーズ参照の typo、ゲート条件の不整合、循環参照などが実行時まで発見できない。

## 2. Solution

belt を「LLM が呼ぶ状態機械ツール」として再設計する。決定論的な制御面（フェーズ遷移、ゲートチェック、regate ループ、リトライ）を YAML に外出しし、LLM のコンテキスト消費を最小化する。

LLM は `belt next` で現フェーズの情報だけを受け取り、フェーズを実行し、`belt verify` でゲートチェックを走らせ、`belt step` で遷移する。パイプライン全体を保持する必要がない。

## 3. Design Principles

### 3.1 Simplicity First

YAML 仕様を最小に保つ。表現力より読みやすさ。テンプレートエンジンや Turing-complete な DSL は作らない。

### 3.2 Verification / Validation 分離

belt-core の Artifact Lifecycle 概念をそのまま YAML に反映する:

- **Verification (gate)**: belt が自動実行する決定論的チェック。「正しく作ったか」
- **Validation (validate)**: LLM が判断する inspection 基準。「正しいものを作ったか」

belt は verify を担い、validate は基準を返すだけ。判断は LLM 側。

### 3.3 Composable Building Blocks

最小単位の部品で組み立て、部品を合成してワークフローを作る:

| Level | 概念 | 例 |
|-------|------|-----|
| 0 | Gate | rust-build.yml, findings-resolved.yml |
| 1 | Sub-Pipeline | design-exploration.yml, review-cycle.yml |
| 2 | Pipeline | feature-dev.yml, debug-flow.yml |
| 3 | (将来) Composed Pipeline | full-release.yml |

### 3.4 GitHub Actions 風 `uses:` + `with:`

Gate にもフェーズにも同じ構文で外部定義を参照する。ローカルパスとリモート参照（将来）を統一的に扱う:

```yaml
# Gate 参照
gate:
  - uses: ./gates/rust-build.yml
    with:
      scope: "--workspace"

# Sub-Pipeline 参照
- id: spec-review
  uses: ./pipelines/review-cycle.yml
  with:
    skill: "/spec-review"
    perspectives: [requirements, design-judgment, feasibility, consistency]

# リモート参照 (v0.2)
- uses: neko-neko/belt-catalog/gates/rust-build@v1
```

リモート解決は `git clone --depth 1 --branch <ref>` → `~/.belt/cache/` にキャッシュ。HTTP ライブラリ不要。

### 3.5 Separation by Audience (原則 8)

サプライチェーンリスクの隔離のため、human CLI と agent CLI を物理的に分離する:

- `belt` (Human): lint / fmt — pipeline 作者向け authoring tool
- `belt-agent` (Agent): init / next / verify / step / status — LLM / CI/CD 向け runtime
- `belt-tui` (Human): TUI — 将来

`cargo build -p belt-agent` で agent runtime のみビルド。lint/fmt コードとその依存を一切取り込まない。

### 3.6 belt が担うもの / 担わないもの

**belt (決定論的制御層):**

- フェーズ遷移・状態管理
- gate 自動実行 (verification)
- validate 基準の提示 (validation は LLM が判断)
- regate 自動再検証
- confirm ゲーティング
- max_retries + escalation
- sub-pipeline 展開
- output_dir 管理
- lint (静的検証)
- 状態永続化

**LLM + Skills (非決定論的実行層):**

- フェーズの実際の実行
- validate 基準の判断
- サブエージェント dispatch (perspectives, iterations, swarm)
- Fix dispatch 戦略
- handover / session notes
- 個別 skill のチューニング

## 4. YAML Specification

### 4.1 Pipeline 定義

```yaml
name: string              # パイプライン名
description: string       # 説明
version: 1                # spec バージョン

args:                     # CLI 引数定義
  arg_name:
    type: bool | string | number
    default: value

phases:                   # 順序付きフェーズリスト
  - id: string            # 一意識別子
    # --- リーフフェーズ (uses なし) ---
    description: string   # フェーズの目的 (LLM に返す)
    # --- 合成フェーズ (uses あり) ---
    uses: string          # サブパイプライン参照
    with: map             # サブパイプラインへのパラメータ
    # --- 共通フィールド ---
    when: expression      # 実行条件 (args.xxx | !args.xxx)
    config: map           # LLM 向けメタデータ (belt はパススルー)
    artifacts: [string]   # 生成するユーザー向けファイルパス
    gate: [check]         # verification チェック (belt が自動実行)
    validate: [string]    # validation 基準 (LLM が判断)
    regate: [string]      # 再検証対象フェーズ ID
    confirm: bool         # ユーザー確認必須 (default: false)
    max_retries: number   # 最大リトライ回数 (default: 0)
```

### 4.2 フィールド詳細

#### `when:`

フェーズの実行条件。`args` の参照のみ。演算子は否定 (`!`) だけ:

```yaml
when: args.smoke      # args.smoke が truthy なら実行
when: !args.force     # args.force が falsy なら実行
```

複合条件（AND/OR）、比較演算子、式評価は一切サポートしない。

#### `config:`

LLM 向けの構造化メタデータ。belt は解釈せず JSON でそのまま返す:

```yaml
config:
  skill: "/code-review"
  perspectives: [simplify, quality, security, performance]
  executor: "feature-implementer"
  tdd: true
  impact_rule: "severity high+ requires explicit user decision"
```

#### `artifacts:`

フェーズが生成するユーザー向けファイル。belt は `belt next` の JSON に含めて返す。belt 内部パス（output_dir 管理のファイル）はここに書かない:

```yaml
artifacts:
  - "docs/specs/*.md"
  - "docs/plans/*-plan.md"
```

#### `gate:`

belt が自動実行する verification チェック。全て PASS で verdict = PASS:

```yaml
gate:
  - cmd: "cargo test --workspace"
  - file_exists: "docs/specs/*.md"
  - git_clean: true
  - has_output: true
  - uses: ./gates/rust-build.yml
    with:
      scope: "-p belt-core"
```

#### `validate:`

LLM が判断する validation 基準。belt は基準テキストを JSON で返すだけ。`validate:` があるフェーズは `belt step --confirm` 必須:

```yaml
validate:
  - "Investigation record has substantive content in all sections"
  - "Every design requirement maps to at least one task"
  - "Tests are not tautologies (assertions exercise actual implementation)"
```

#### `regate:`

現フェーズの gate が PASS した後、指定フェーズの gate を再実行する。regate 対象フェーズが `when:` で無効なら自動スキップ:

```yaml
regate: [execute, smoke-test]
```

#### `confirm:`

`true` の場合、`belt step` に `--confirm` フラグが必須。LLM が勝手に遷移できない。`validate:` があるフェーズも暗黙的に confirm 必須。

#### `max_retries:`

gate FAIL 時のリトライ回数。超過すると `belt step` が `max_retries_exceeded` を返す。LLM はエスカレーションするか PAUSE するか判断する。

### 4.3 Gate チェック型

| 型 | 構文 | 判定 |
|----|------|------|
| `cmd` | `cmd: "command"` | exit code 0 = PASS |
| `file_exists` | `file_exists: "glob"` | 1件以上マッチ = PASS |
| `git_clean` | `git_clean: true` | unstaged/untracked なし = PASS |
| `has_output` | `has_output: true` | フェーズの output_dir が非空 = PASS |
| `uses` | `uses: path` + `with:` | 参照先の全 checks PASS = PASS |

### 4.4 Gate 定義ファイル

```yaml
name: string
description: string
inputs:
  param_name:
    type: string | number | bool
    default: value
checks:
  - cmd: "cargo build ${scope}"
  - file_exists: "..."
  - git_clean: true
```

`${param_name}` で inputs のパラメータを参照。

### 4.5 Sub-Pipeline 定義ファイル

```yaml
name: string
description: string
version: 1
inputs:
  param_name:
    type: string | list
    required: bool
phases:
  - id: string
    # ... 通常のフェーズ定義
```

`${inputs.param_name}` で inputs のパラメータを参照。

### 4.6 Sub-Pipeline 展開

belt は `uses:` を**パース時にフラット展開**する。サブフェーズの ID は `<parent>/<child>` に namespace 化:

```
feature-dev.yml の phases:
  design (uses: design-exploration.yml)
  spec-review (uses: review-cycle.yml)
  plan
  ...

展開後:
  design/explore
  design/synthesize
  design/write-design
  spec-review/review
  spec-review/triage
  spec-review/fix
  plan
  ...
```

合成フェーズに `gate:` がある場合、サブパイプライン完了後に追加で実行される。`regate:`, `config:` も合成フェーズレベルで適用される。

## 5. CLI Interface

### 5.1 belt-agent (Agent Runtime)

全コマンド JSON 出力。`--run` 省略時は最新の run:

```
belt-agent init <pipeline.yml> [--<arg>=<value> ...]
belt-agent next   [--run <id>]
belt-agent verify [--run <id>]
belt-agent step   [--run <id>] [--confirm]
belt-agent status [--run <id>]
```

### 5.2 belt (Human CLI)

人向け出力 (color + miette diagnostics):

```
belt lint <pipeline.yml>
belt fmt  <pipeline.yml>
```

### 5.3 コマンド出力例

#### belt-agent init

```json
{
  "run_id": "01J...",
  "pipeline": "feature-dev",
  "phase": {
    "id": "design/explore",
    "description": "Dispatch parallel exploration agents...",
    "config": { "agents": ["code-explorer", "impact-analyzer", "code-architect"] },
    "output_dir": ".belt/runs/01J.../design/explore/"
  },
  "gate": [
    { "type": "has_output" }
  ],
  "validate": null,
  "confirm": false,
  "max_retries": 0,
  "attempt": 0,
  "args": { "codex": false, "doc": false, "smoke": false, "e2e": false }
}
```

#### belt-agent verify

```json
{
  "run_id": "01J...",
  "phase": "execute",
  "verdict": "FAIL",
  "checks": [
    { "type": "cmd", "cmd": "cargo build --workspace", "passed": true, "duration_ms": 1200 },
    { "type": "cmd", "cmd": "cargo test --workspace", "passed": false, "stderr": "test xyz failed" },
    { "type": "cmd", "cmd": "cargo clippy --workspace -- -D warnings", "passed": true }
  ],
  "regate": null,
  "validate": [
    "Every plan task has corresponding code changes",
    "Tests are not tautologies",
    "Implementation respects component boundaries",
    "Design -> plan -> implementation -> test traceability is maintained"
  ],
  "attempt": 1,
  "max_retries": 3
}
```

`regate` は現フェーズの gate が PASS した後にのみ実行。結果は:

```json
{
  "verdict": "PASS",
  "checks": [...],
  "regate": {
    "execute": "PASS",
    "smoke-test": "SKIP"
  },
  "validate": [...]
}
```

#### belt-agent step

成功:

```json
{
  "advanced": true,
  "from": "execute",
  "to": "code-review/review",
  "phase": { "id": "code-review/review", "description": "...", ... }
}
```

confirm / validate 未確認:

```json
{
  "advanced": false,
  "reason": "confirmation_required",
  "phase": "spec-review/triage"
}
```

max_retries 超過:

```json
{
  "advanced": false,
  "reason": "max_retries_exceeded",
  "phase": "execute",
  "attempt": 3,
  "max_retries": 3
}
```

パイプライン完了:

```json
{
  "advanced": true,
  "from": "integrate",
  "to": null,
  "completed": true
}
```

#### belt-agent status

```json
{
  "run_id": "01J...",
  "pipeline": "feature-dev",
  "args": { "codex": false, "smoke": true, "e2e": false },
  "current_phase": "code-review/triage",
  "completed_phases": ["design/explore", "design/synthesize", "design/write-design", "spec-review/review", "spec-review/triage", "spec-review/fix", "plan", "plan-review/review", "plan-review/triage", "plan-review/fix", "execute"],
  "skipped_phases": ["doc-audit"],
  "attempt": 0,
  "created_at": "2026-04-06T10:00:00Z",
  "updated_at": "2026-04-06T14:30:00Z"
}
```

#### belt lint

```
$ belt lint pipeline.yml
✓ YAML schema valid
✓ Phase IDs unique (10 phases)
✓ uses: references resolved (3 sub-pipelines, 2 gates)
✓ regate targets valid: execute, smoke-test
✓ args references valid: doc, smoke, e2e
✓ No circular sub-pipeline references
✓ with: parameters match inputs: definitions

pipeline.yml: 0 errors, 0 warnings
```

## 6. State Management

### 6.1 ディレクトリ構造

```
.belt/
├── runs/
│   └── <run-id>/
│       ├── state.json                    # run 状態
│       └── <phase-id>/                   # フェーズごとの output_dir
│           └── (LLM が書き込むファイル)
└── cache/                                # リモート uses: のキャッシュ (v0.2)
    └── <owner>/<repo>/<ref>/
```

### 6.2 state.json

`belt-agent init` で生成、`belt-agent verify` / `belt-agent step` で更新:

```json
{
  "run_id": "01J...",
  "pipeline": "feature-dev",
  "pipeline_file": "./pipeline.yml",
  "version": 1,
  "args": { "codex": false, "smoke": true, "e2e": false },
  "branch": "feature/belt-32-artifact",
  "status": "InProgress",
  "current_phase": "code-review/triage",
  "completed_phases": ["design/explore", "design/synthesize", "..."],
  "skipped_phases": ["doc-audit"],
  "phase_attempts": { "execute": 2 },
  "resolved_consumes": {
    "belt://latest/design.md": "/abs/path/.belt/runs/01J.../design/explore/design.md"
  },
  "last_verify": {
    "phase": "execute",
    "verdict": "PASS",
    "at": "2026-04-06T14:20:00Z"
  },
  "created_at": "2026-04-06T10:00:00Z",
  "updated_at": "2026-04-06T14:30:00Z"
}
```

BELT-spec 2026-04-14 (context-neutral narrative artifact) で追加された 3 フィールド:

- `branch: Option<String>` — init 時点の git ブランチ名。非 git リポジトリや detached HEAD では `None`。belt-agent の `git::current_branch` ラッパ経由で `Engine::init_with_branch` が記録する。`belt://workspace/{branch}/latest/` 解決に使用
- `resolved_consumes: HashMap<String, String>` — URI → 解決済み絶対パスのスナップショット。init 時に belt-agent resolver が pipeline の consumes 内の `ArtifactRef::External` 全エントリを解決し記録する。`belt-agent next` は consume 項目の `resolved_path` として再出力する
- `status: RunStatus` — `InProgress` | `Completed` | `Failed`。デフォルトは `InProgress`。`Engine::step` が last-phase advance 時に `Completed` へ遷移させる。`Failed` は将来用に予約

## 7. Crate Architecture

### 7.1 構成

```
belt/
├── crates/
│   ├── belt-core/     # 📦 pure library
│   ├── belt/          # 🛠 human CLI binary (lint / fmt)
│   └── belt-agent/    # 🤖 agent CLI binary (init / next / verify / step / status)
```

### 7.2 belt-core

pure library。CLI 依存なし。I/O は trait 抽象化:

- YAML パーサー（pipeline, gate, sub-pipeline）
- Sub-pipeline 展開（フラット化）
- State machine（フェーズ遷移、regate、max_retries）
- Gate executor（cmd, file_exists, git_clean, has_output）
- Lint validator（スキーマ、参照、循環検出）
- State persistence（state.json 読み書き）

### 7.3 belt (Human CLI)

pipeline 作者向け。人間に読みやすい出力:

- `belt lint` — 静的検証 (miette diagnostics)
- `belt fmt` — YAML フォーマット

belt-core + clap + miette[fancy]。runtime 実行ロジックは含まない。

### 7.4 belt-agent (Agent CLI)

LLM / CI/CD 向け。JSON 出力:

- `belt-agent init` / `next` / `verify` / `step` / `status`

belt-core + clap + miette[fancy]。lint/fmt コードは含まない。

### 7.5 依存管理

| 制約 | belt | belt-agent |
|------|------|-----------|
| TUI/GUI ライブラリ | 禁止 | 禁止 |
| HTTP / async runtime | 原則禁止 | 原則禁止 |
| unsafe code | forbid | forbid |
| lint/fmt コード | 含む | **含まない** |
| runtime コード | **含まない** | 含む |

## 8. Scope

### 8.1 MVP (v0.1)

- Pipeline YAML パース (全フィールド)
- Sub-pipeline 展開 (`uses:` on phases、パース時フラット化)
- belt-agent CLI: init, next, verify, step, status
- belt CLI: lint
- Gate: cmd, file_exists, git_clean, has_output
- ローカル `uses:` + `with:` (gates + sub-pipelines)
- `when:` / `!when:` (args 参照 + 否定)
- State 永続化 (.belt/runs/)
- output_dir 管理
- JSON 出力

### 8.2 v0.2

- リモート `uses:` (git fetch + cache)
- `belt fmt` (YAML フォーマット)
- セッション resume (handover 統合)
- gate 型追加 (regex_match, env_var 等)
- run 管理 (belt-agent run list / prune)

### 8.3 v0.3+

- YAML Universe (レジストリ / ディスカバリ)
- belt-tui (ratatui-based 監視 UI)
- Composed pipelines (Level 3)

BELT-spec 2026-04-14 は run をまたいだ narrative 参照のために `belt://` URI スキーム (selector: `latest/`, `workspace/{branch}/latest/`, `run/{run_id}/`) を導入した。現スコープはローカルの `.belt/runs/` のみだが、これは将来のクロスリポジトリ解決の土台となる。URI 解決は belt-agent の resolver が init 時に実行し、belt-core は pure を保つ (ファイルシステム走査を行わない)。完全な設計は `docs/specs/2026-04-14-belt-context-neutral-narrative-artifact.md` を参照。

## 9. Gap Analysis

feature-dev / debug-flow / linear-refresh の 3 ワークフローに対する意味的カバレッジ分析。全て本設計でカバーされることを確認済み。

### 9.1 カバー済みパターン

| パターン | YAML 表現 |
|---------|-----------|
| フェーズ列 | `phases:` 順序付きリスト |
| 条件付きフェーズ | `when: args.xxx` / `when: !args.xxx` |
| 自動ゲート | `gate:` (cmd, file_exists, git_clean, has_output, uses) |
| Inspection 基準 | `validate:` リスト |
| ユーザー確認 | `confirm: true` |
| レビューサイクル | `uses: review-cycle.yml` (review → triage → fix) |
| Re-gate ループ | `regate: [phase_ids]` |
| リトライ + エスカレーション | `max_retries: N` |
| サブワークフロー合成 | `uses:` on phases |
| サブエージェント設定 | `config:` (belt パススルー) |
| セッション永続化 | state.json + belt-agent status |

### 9.2 belt の外（LLM / Skills の責務）

| 項目 | 理由 |
|------|------|
| サブエージェント dispatch | ユースケースごとに perspectives/agents が異なる。個別最適化の範囲 |
| N-way 投票、Codex 並列 | `args` + `config` でパラメータを渡すが、実行は LLM |
| Fix dispatch 戦略 | SKILL.md に記述。belt は関知しない |
| phase-scoped narrative (Artifact via `belt://` URI) | belt (belt-core が `belt://` URI を Artifact に記録、belt-agent resolver が init 時に実体パス解決) |
| session-level narrative (active_tasks, recent_decisions) | SKILL.md protocol |
| handover / resume approval gate | SKILL.md protocol |
| Linear sync | SKILL.md protocol |

## 10. Context Reduction Estimate

feature-dev を例にした概算:

| | 現行 (SKILL.md) | belt 化後 |
|---|---|---|
| SKILL.md | ~579行 | ~130行 (protocol + rules) |
| done-criteria | 8ファイル (~300行) | `validate:` に集約 (YAML 内) |
| LLM が保持する情報 | 全体 (~900行) | **SKILL.md 130行 + belt next の JSON 1件** |
| パイプライン定義 | LLM コンテキスト内 | belt YAML (コンテキスト外) |

**推定コンテキスト削減: ~80%**
