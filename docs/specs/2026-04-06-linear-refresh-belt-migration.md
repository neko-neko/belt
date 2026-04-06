# linear-refresh belt 移植設計書

## 概要

既存の `/linear-refresh` スキル（linear-cleanup + linear-add の統合オーケストレーター）を belt パイプライン + スキルの2層構造に移植する。linear-cleanup と linear-add は linear-refresh 専用の sub-pipeline として移植する。

合わせて、既存の `skills/belt-agent/SKILL.md` と `skills/smoke-test/SKILL.md` にフロントマター追加・参照方法の改善を行う。

## 成果物

### 新規作成

| ファイル | 役割 | 言語 |
|---------|------|------|
| `pipelines/linear-refresh.yml` | トップレベルパイプライン（6フェーズ） | English |
| `pipelines/linear-cleanup.yml` | cleanup 分析 sub-pipeline | English |
| `pipelines/linear-add.yml` | add 分析 sub-pipeline | English |
| `skills/linear-refresh/SKILL.md` | フロー + ルール + HARD-GATE + Phase Map | English |
| `skills/linear-refresh/references/collected-context-schema.md` | CollectedContext スキーマ定義 | English |
| `skills/linear-refresh/references/external-source-exploration.md` | URL フィルタリング、要約予算、deferred signals、1hop/2hop ロジック | English |
| `skills/linear-refresh/references/ground-truth-audit.md` | 3質問と判定基準、修正ループの手順 | English |
| `skills/linear-refresh/references/execution-report.md` | 実行結果レポートフォーマット | English |

### 既存ファイル修正

| ファイル | 修正内容 |
|---------|---------|
| `skills/belt-agent/SKILL.md` | フロントマター追加、冗長な説明文削除 |
| `skills/smoke-test/SKILL.md` | フロントマター追加、`../belt-agent/SKILL.md` 参照を `/belt-agent` invoke に変更 |

## スコープ

### 含むもの

- 6フェーズパイプライン（collect → cleanup-analysis → add-analysis → audit → approve → execute）
- linear-cleanup / linear-add を sub-pipeline として `uses:` 参照
- Ground Truth 監査の修正ループ（`regate: [collect]` + `max_retries: 2`）
- `--force` による承認スキップ（`when: "!args.force"`）
- スキルの HARD-GATE（`/linear-cli` + `/slackcli` invoke 義務）
- リファレンスファイル分離（SKILL.md + references/ 4ファイル）
- 全スキルへのフロントマター適用

### 含まないもの

- linear-cleanup / linear-add のスタンドアロン版（linear-refresh 経由のみ）
- 監査手法（done-criteria、phase-auditor dispatch）
- belt-core の変更

## フロントマター設計

### `skills/belt-agent/SKILL.md`

```yaml
---
name: belt-agent
description: Belt Protocol for driving belt-agent CLI. Defines command loop, response interpretation, and safety constraints for LLM agents driving belt pipelines.
user-invocable: false
---
```

- `user-invocable: false` — ユーザーが直接 invoke する必要はない。パイプライン固有スキルが invoke する背景知識

### `skills/smoke-test/SKILL.md`

```yaml
---
name: smoke-test
description: Browser-based UI verification for code changes. Generates test scenarios from diffs and design docs, executes them via browser, and produces an evidence-backed report.
argument-hint: "[--diff-base <branch>] [--skip-vrt] [--skip-e2e]"
---
```

- デフォルト（user-invocable: true, disable-model-invocation: false）で、ユーザーも feature-dev pipeline も invoke 可能

### `skills/linear-refresh/SKILL.md`

```yaml
---
name: linear-refresh
description: Orchestrates linear-cleanup and linear-add in a single workflow. Collects tickets and external sources, analyzes for cleanup and add candidates, audits plan quality, then executes.
argument-hint: "[--force]"
---
```

## Pipeline 定義

### `pipelines/linear-refresh.yml`（トップレベル）

```yaml
name: linear-refresh
version: 1
args:
  force: { type: bool, default: false }

phases:
  - id: collect
    description: "Fetch all tickets and explore external sources (1-hop + 2-hop)."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/collected-context.json"

  - id: cleanup-analysis
    description: "Analyze tickets for structural issues."
    uses: ./pipelines/linear-cleanup.yml

  - id: add-analysis
    description: "Detect new ticket candidates from external sources."
    uses: ./pipelines/linear-add.yml

  - id: audit
    description: "Ground Truth audit — verify CollectedContext completeness and Plan quality."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/refresh-plan.json"
    validate:
      - "Every In Progress ticket's latest context is reflected in the plan"
      - "No untracked references remain for high-priority tickets"
      - "Deferred signals from external sources are addressed in the plan"
    regate: [collect]
    max_retries: 2

  - id: approve
    description: "Present unified plan for user approval."
    when: "!args.force"
    config:
      skill: "/linear-refresh"
    confirm: true

  - id: execute
    description: "Execute cleanup changes, then add changes."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/refresh-result.json"
```

### `pipelines/linear-cleanup.yml`（sub-pipeline）

```yaml
name: linear-cleanup
version: 1
description: "Analyze CollectedContext for structural issues in existing tickets."
phases:
  - id: analyze
    description: "Detect parent-child, blocking, duplicate, and status issues."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/plan-a.json"
```

### `pipelines/linear-add.yml`（sub-pipeline）

```yaml
name: linear-add
version: 1
description: "Detect new ticket candidates from CollectedContext, excluding Plan A items."
phases:
  - id: analyze
    description: "Identify create/link/skip candidates from external sources."
    config:
      skill: "/linear-refresh"
    gate:
      - file_exists: ".belt/plan-b.json"
```

### 設計判断

- **collect**: CollectedContext を `.belt/collected-context.json` として永続化。gate で存在確認
- **cleanup-analysis / add-analysis**: sub-pipeline として分離。それぞれ Plan A / Plan B を生成
- **audit**: validate で Ground Truth 品質を LLM 判断。`regate: [collect]` で追加探索時に collect の gate を再検証。`max_retries: 2` で最大2回の修正ループ
- **approve**: `when: "!args.force"` で `--force` 時にスキップ
- **execute**: 結果レポートを `.belt/refresh-result.json` として永続化

## Skill 構成

### ファイル構造

```
skills/linear-refresh/
├── SKILL.md
└── references/
    ├── collected-context-schema.md
    ├── external-source-exploration.md
    ├── ground-truth-audit.md
    └── execution-report.md
```

### SKILL.md の構造

```
---
name: linear-refresh
description: Orchestrates linear-cleanup and linear-add in a single workflow. Collects tickets and external sources, analyzes for cleanup and add candidates, audits plan quality, then executes.
argument-hint: "[--force]"
---

# Linear Refresh

Orchestrates linear-cleanup and linear-add in a single workflow. Collects tickets
and external sources once, analyzes for cleanup and add candidates, audits plan
quality, then executes approved changes.

This skill is used with the `pipelines/linear-refresh.yml` belt pipeline.
Invoke /belt-agent for protocol details.

## HARD-GATE

<HARD-GATE>
Before starting the collect phase, invoke /linear-cli and /slackcli skills
to load their context. These skills provide the CLI usage patterns required
for ticket retrieval and external source exploration.
</HARD-GATE>

## Output

- `.belt/collected-context.json` — all tickets + external sources
- `.belt/plan-a.json` — cleanup change candidates
- `.belt/plan-b.json` — add detection candidates
- `.belt/refresh-plan.json` — unified plan (cleanup + add)
- `.belt/refresh-result.json` — execution results

## Phase Map

| Phase | What to do | Reference |
|-------|-----------|-----------|
| collect | Fetch tickets, explore external sources | collected-context-schema.md, external-source-exploration.md |
| cleanup-analysis/analyze | Analyze for structural issues | Read /linear-cleanup skill guidelines |
| add-analysis/analyze | Detect new ticket candidates | Read /linear-add skill guidelines |
| audit | Ground Truth audit, generate unified plan | ground-truth-audit.md |
| approve | Present plan, wait for user approval | — |
| execute | Run cleanup then add changes | execution-report.md |

## Phase: collect
  - Invoke /linear-cli and /slackcli (HARD-GATE)
  - Team selection (1 team → auto, multiple → ask user, 0 → error)
  - Step 0-1: Fetch all tickets via linear CLI
  - Step 0-2: Fetch details for active tickets (Agent parallel, 10-ticket batches)
  - Step 0-3: 1-hop external source exploration (Agent parallel)
  - Step 0-3b: 2-hop recursive expansion for high-priority tickets (Agent parallel)
  - Step 0-4: Generate CollectedContext JSON
  - See references for URL filtering, summary budgets, and schema

## Phase: cleanup-analysis/analyze
  - Read /linear-cleanup SKILL.md analysis guidelines
  - Analyze CollectedContext for: parent-child, blocking, related, status, duplicates
  - Output Plan A (.belt/plan-a.json)

## Phase: add-analysis/analyze
  - Read /linear-add SKILL.md detection criteria
  - Exclude items already in Plan A
  - Output Plan B (.belt/plan-b.json)

## Phase: audit
  - Read ground-truth-audit.md
  - Run 3 audit questions (Q1-Q3) for each In Progress ticket
  - If issues found: fix CollectedContext, regenerate Plan A/B
  - Generate unified plan (.belt/refresh-plan.json)

## Phase: approve
  - Present unified plan (Cleanup + Add sections)
  - Skipped when --force is set

## Phase: execute
  - Cleanup first: parent-child → parallel (blocking, related, status, context) → duplicates
  - Add second: create → link
  - Error handling: skip individual failures, continue execution
  - Generate result report (.belt/refresh-result.json)

## Red Flags

**Never:**
- Execute changes without an approved plan (unless --force)
- Delete or archive tickets (cleanup closes duplicates only)
- Rewrite ticket descriptions (context additions use comments/attachments)
- Explore beyond 2 hops (infinite expansion prevention)

**Always:**
- Invoke /linear-cli and /slackcli before collect
- Respect summary budgets (200/400/800 chars by priority)
- Record deferred signals from external sources
- Report all execution failures in result JSON
```

### リファレンスファイルの内容

#### `references/collected-context-schema.md`

CollectedContext の JSON スキーマ定義:

```json
{
  "team_id": "string",
  "tickets": [
    {
      "id": "string",
      "title": "string",
      "status": "string",
      "priority": "string",
      "labels": ["string"],
      "project": "string | null",
      "parentId": "string | null",
      "assignee": "string | null",
      "completedAt": "string | null",
      "archivedAt": "string | null",
      "attachments": [{ "url": "string", "title": "string" }],
      "description_urls": ["string"],
      "relations": {
        "relatedTo": ["string"],
        "blocks": ["string"],
        "blockedBy": ["string"]
      }
    }
  ],
  "external_sources": [
    {
      "url": "string",
      "ticket_id": "string",
      "hop": 1,
      "accessible": true,
      "summary": "string",
      "referenced_urls": ["string"],
      "latest_activity_ts": "string (ISO 8601)",
      "deferred_signals": ["string"],
      "source_type": "slack_thread | github_issue | github_pr | github_comment | document"
    }
  ]
}
```

#### `references/external-source-exploration.md`

- URL フィルタリング基準テーブル（探索する / メタデータのみ / スキップ）
- 要約予算テーブル（Backlog: 200字, Todo: 400字, In Progress+High: 800字 + raw excerpts）
- deferred signals パターン一覧
- 1ホップ探索の実行手順（Agent parallel, 戻り値フィールド）
- 2ホップ再帰展開の条件（In Progress + High/Urgent + 72h 以内）と追跡する/しない URL 種別
- 無限展開防止（3ホップ目以降は禁止）

#### `references/ground-truth-audit.md`

3つの監査質問:
- Q1: 実装者向け ground truth — Plan にコンテキストが反映されているか
- Q2: 最近の活動 — deferred signals が Plan に反映されているか
- Q3: 未追跡参照 — referenced_urls に漏れがないか

判定と修正ループ:
- 不備あり + 追加探索必要 → 該当 URL のみ単発探索 → CollectedContext 補完 → Plan 再生成
- 不備なし → 統合 Plan 生成へ

#### `references/execution-report.md`

実行結果レポートの JSON フォーマット:
- Cleanup 結果（成功/失敗件数、失敗詳細）
- Add 結果（create/link 件数、失敗詳細）
- 変更されたチケット一覧
- エラーハンドリング（API エラー、レート制限、循環参照、削除済みチケット）

## 既存スキルの修正

### `skills/belt-agent/SKILL.md`

修正前:
```markdown
# Belt Protocol

Generic protocol for driving the belt-agent CLI. This skill defines how LLM agents
interact with belt's deterministic state machine — the command loop, response
interpretation, and safety constraints.

Pipeline-specific skills reference this protocol for consistent belt-agent usage.
This skill is not invoked directly by users.
```

修正後:
```markdown
---
name: belt-agent
description: Belt Protocol for driving belt-agent CLI. Defines command loop, response interpretation, and safety constraints for LLM agents driving belt pipelines.
user-invocable: false
---

# Belt Protocol

Generic protocol for driving the belt-agent CLI. Defines how LLM agents interact
with belt's deterministic state machine — the command loop, response interpretation,
and safety constraints.
```

- フロントマター追加（`name`, `description`, `user-invocable: false`）
- 冗長な説明文（"Pipeline-specific skills reference..."、"This skill is not invoked directly by users."）削除。フロントマターの `user-invocable: false` と `description` が同等の情報を持つ

### `skills/smoke-test/SKILL.md`

修正前:
```markdown
# Smoke Test

Browser-based UI verification for code changes. Generates test scenarios from diffs
and design docs, executes them via browser interaction, and produces an evidence-backed
report.

This skill is used with the `pipelines/smoke-test.yml` belt pipeline. It follows the
[Belt Protocol](../belt-agent/SKILL.md) for pipeline driving.
```

修正後:
```markdown
---
name: smoke-test
description: Browser-based UI verification for code changes. Generates test scenarios from diffs and design docs, executes them via browser, and produces an evidence-backed report.
argument-hint: "[--diff-base <branch>] [--skip-vrt] [--skip-e2e]"
---

# Smoke Test

Browser-based UI verification for code changes. Generates test scenarios from diffs
and design docs, executes them via browser interaction, and produces an evidence-backed
report.

This skill is used with the `pipelines/smoke-test.yml` belt pipeline.
Invoke /belt-agent for protocol details.
```

- フロントマター追加（`name`, `description`, `argument-hint`）
- ファイルパス参照 `[Belt Protocol](../belt-agent/SKILL.md)` を `Invoke /belt-agent for protocol details.` に変更

## 設計判断の根拠

### なぜ linear-cleanup / linear-add をスタンドアロンにしないか

linear-refresh の Phase 0 (Collect) が外部ソース探索を一括で行い、その CollectedContext を cleanup と add が共有する。スタンドアロン版は独自の Collect が必要になり、設計が大きくなる。今回は linear-refresh 経由のみに絞り、スタンドアロン版は必要になった時に追加する。

### なぜ audit に regate + max_retries を使うか

Ground Truth 監査は「CollectedContext を補完して Plan を再生成する」修正ループを持つ。belt は前方遷移のみだが、`regate: [collect]` で collect の gate を再検証し、`max_retries: 2` で最大2回ループさせることで擬似的な後方修正を実現する。belt-core の変更なしで対応できる。

### なぜ全スキルにフロントマターを付けるか

Claude Code のスキルシステムはフロントマターで invocation control、description（Claude の自動選択に使用）、argument-hint（自動補完）を制御する。フロントマターなしではスキルの発見と invocation が最適化されない。特に `user-invocable: false`（belt-agent）と `argument-hint`（smoke-test, linear-refresh）は UX に直結する。

### なぜ /belt-agent をファイルパスではなくスキル invoke で参照するか

`../belt-agent/SKILL.md` のファイルパス参照はスキルの配置場所に依存し、プロジェクト間で壊れやすい。`/belt-agent` として invoke すれば Claude Code のスキル解決メカニズムが配置場所を抽象化し、personal / project / plugin どこに置いても動作する。

## 関連

- [BELT-20](https://linear.app/neko-neko/issue/BELT-20) — belt 再設計 MVP
- [BELT-21](https://linear.app/neko-neko/issue/BELT-21) — belt-core verify-before-step 強制ガード
- `skills/belt-agent/SKILL.md` — Belt Protocol 汎用スキル
- `docs/specs/2026-04-06-belt-agent-skill.md` — Belt Protocol 設計書
- `docs/specs/2026-04-06-smoke-test-belt-migration.md` — smoke-test 移植設計書
