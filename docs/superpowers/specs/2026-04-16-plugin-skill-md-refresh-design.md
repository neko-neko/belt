---
title: Plugin SKILL.md Refresh — SSOT as pipeline.yml
date: 2026-04-16
status: approved
scope:
  - plugins/feature-dev/skills/feature-dev/SKILL.md
  - plugins/bug-fix/skills/bug-fix/SKILL.md
  - skills/belt-agent/SKILL.md
related:
  - docs/specs/2026-04-16-review-skills-subagent-boundary-design.md
  - MEMORY: project_skill_md_authoring_principle.md
---

# Plugin SKILL.md Refresh — SSOT as pipeline.yml

## 1. Overview

`pipeline.yml` を SSOT として確立し、SKILL.md を「pipeline.yml で表現できない
3 責務 (config key 解釈 / ドメイン制約 / references ポインタ)」に純化する。

背景:

- `feature-dev` / `bug-fix` SKILL.md は `pipeline.yml` と重複した情報 (Pipeline
  Overview 図、Args 表、Phase 番号付き Invocation Rules) を抱えており、pipeline
  側の変更に追随する度に SKILL.md も書き換えが必要になっている
- Phase 番号 (`Phase 1: design`, `Phase 2: test-scenarios`, ...) がハードコード
  されており、phase 追加・順序変更・条件付き phase (`when:`) の挿入で全体を
  書き換える運用負債になっている
- `belt-agent` SKILL.md に `> **Note (2026-04-16):** ...` や
  `As of 2026-04-16, ...` といった history 記述が混在しており、Anthropic の
  SKILL 著作ベストプラクティス "Avoid time-sensitive information" に反する

本 spec は、既存原則 `project_skill_md_authoring_principle.md` (Phase Map 禁止、
3 責務) に実装を追随させる refresh タスクとして、3 つの SKILL.md をリライトする。

## 2. Scope / Non-goals

**In-scope**:

- `plugins/feature-dev/skills/feature-dev/SKILL.md` 全面リライト
- `plugins/bug-fix/skills/bug-fix/SKILL.md` 全面リライト
- `skills/belt-agent/SKILL.md` 全体 audit + リライト
- MEMORY.md に今回の refresh を記録 (新規エントリ + index 更新)

**Non-goals**:

- pipeline.yml schema 変更 (approach C 不採用)
- supplement ファイル (`references/*-supplement.md`) 本体の書き換え
- criteria ファイル (`./criteria/*.md`) 変更
- belt-core / belt-agent CLI 変更
- legacy 外部参照 (agents / docs) への伝播修正 (別 spec 扱い)
- lint rule 追加による supplement map ↔ pipeline.yml 整合性 enforce (将来検討)

## 3. Approach Selection (記録用)

ユーザー承認を得た選択:

- **Q1 (SKILL.md scope)**: B (Balanced) — Pipeline Overview と Args 表は削除、
  Phase-Specific Invocation Rules は supplement reading 指示と phase-specific
  runtime note のみに絞り込む
- **Q2 (フォーマット)**: B1 — 役割別 2 セクション分離 (Supplement Loading 表 +
  Phase-specific Runtime Notes)
- **Q3 (belt-agent scope)**: C4 — belt-agent SKILL.md 全体の冗長 audit
  (Commands / Status Output / Config Keys を含む)

不採用案:

- A (Minimalist): supplement loading 指示まで削ると phase 実行時の invoke
  sequence が失われるため不採用
- C (Schema 拡張): pipeline.yml に `invoke.before_hints:` フィールドを新設する
  案は belt-core schema 変更が必要でスコープが膨らむため不採用
- C1 (belt-agent Line 173 のみ削除): Line 58 Note / Line 171 past tense が残る
  と同じ趣旨を異なる表現で語る矛盾が残るため不採用

## 4. feature-dev SKILL.md 新構造

```markdown
---
name: feature-dev
description: >-
  Runs a quality-gated feature-development pipeline spanning design, test
  strategy, spec review, implementation, and regression verification. Use when
  building a new feature that needs a structured design-to-integration flow.
  --e2e enables browser-based verification; --codex enables adversarial review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# feature-dev

Belt pipeline for quality-gated development. Pipeline structure, args, phase
order, and invocation mapping are defined in `pipeline.yml` and surfaced
dynamically by `belt-agent next` / `belt-agent status`. This SKILL.md covers
the supplement loading contract, phase-specific runtime notes, and red flags
that cannot be expressed in pipeline.yml.

## Supplement Loading

Before invoking a phase's skill, read the referenced supplement to inject
phase-specific overrides. Phases not listed (`test-scenarios` / `spec-review` /
`execute` / `code-review`) have no supplement; invoke their declared skill
directly.

| Phase | Supplement | Purpose |
|---|---|---|
| design | `./references/brainstorming-supplement.md` | parallel exploration (code-explorer / code-architect / impact-analyzer), implicit-rules extraction, required design sections, worktree creation order |
| plan | `./references/writing-plans-supplement.md` | path override, Must-Verify, scenarios cross-referencing |
| monkey-test | `./references/monkey-test-supplement.md` | context injection for replay |
| dogfood | `./references/dogfood-supplement.md` | overrides and prior-phase artifact hints |
| integrate | `./references/worktrunk-supplement.md` | A/B choice logic (wt merge / gh pr create) |

## Phase-specific Runtime Notes

- **spec-review**: grill-me dialogue for `requirements` / `design-judgment`
  findings; direct selection triage for the remaining observations.
- **execute**: orchestrator must reconstruct plan tasks into self-contained
  implementation specs before dispatching `belt-agents:feature-implementer`
  subagents. Do not forward broad research verbatim.
- **integrate**: prompt user for mode (A: `wt merge` / B: `gh pr create`)
  before executing `/worktrunk`.

## Narrative Notes

These phases produce a narrative note so context can be restored after `/clear`
(`.belt/runs/{run_id}/notes/phase-<id>.md`):

- `design` / `plan` / `execute` / `code-review`
- `monkey-test` (`--e2e`) / `dogfood` (`--e2e`)

Each note contains four sections (`## Decisions` / `## Concerns` /
`## Directives` / `## Observations`) and minimal frontmatter (`phase`,
`run_id`).

Full convention: [`plugins/belt-agents/references/narrative-convention.md`](plugins/belt-agents/references/narrative-convention.md)

`/clear` itself is the user's call — Claude Code runtime constraints prevent
automation. Use narrative notes as an option when context has grown large
after a heavy phase (for example, right after design, execute, or
code-review).

## Red Flags

- **Never skip supplement loading when listed above**: phase-specific overrides are lost and behavior drifts.
- **Never bypass the integrate A/B choice**: merge-vs-PR is always user-decided.
- **Never modify the consumed global skills**: all overrides go through `references/*-supplement.md`.
- **Never hand-edit files under `docs/features/<topic>/`**: they are phase-produced; manual edits break belt's phase-start mtime filter.
- **Never leave the narrative note's four sections blank**: the gate is `file_exists` only and empty sections still pass, but downstream consumers cannot restore context. Use at least `(none)` as a placeholder and always keep the heading.

## References

- `./references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `./references/brainstorming-supplement.md` — design phase overrides
- `./references/writing-plans-supplement.md` — plan phase overrides
- `./references/monkey-test-supplement.md` — monkey-test phase context injection
- `./references/dogfood-supplement.md` — dogfood phase overrides and hints
- `./references/worktrunk-supplement.md` — integrate phase A/B choice logic
- `plugins/belt-agents/references/narrative-convention.md` — narrative note schema (shared with bug-fix)
```

**削減量**: 123 行 → 約 70 行 (43% 減)。

**削除される既存セクション** (retain/delete 逐語列挙):

- `## Pipeline Overview` (line 16-23 相当) — 完全削除
- `## Args` 表 (line 24-29 相当) — 完全削除
- `## Phase-Specific Invocation Rules` 全体 (line 31-89 相当) — 削除。内容は
  Supplement Loading 表 + Phase-specific Runtime Notes に分解・縮約
- `### Phase 1: design` 〜 `### Phase 9: integrate` のサブ見出し — 削除
- Phase 番号 (`Phase 1:`, `Phase 2:`, ..., `Phase 9:`) の表記 — 完全削除

**保持される既存セクション** (逐語):

- frontmatter (変更なし)
- `# feature-dev` 見出し下の intro (再記述)
- `## Narrative Notes` (line 91-102 相当) — 保持
- `## Red Flags` (line 104-113 相当) — 一部縮約 (Phase 1 supplement 指示が
  Supplement Loading 表に統合されたため、Red Flags からは phase 個別名を削除し
  "supplement loading when listed above" 形式に変更)
- `## References` (line 115-123 相当) — 保持、記述の細部 (phase 番号言及) を
  phase id 表記に修正

## 5. bug-fix SKILL.md 新構造

```markdown
---
name: bug-fix
description: >-
  Runs a quality-gated debugging pipeline with root-cause analysis, fix planning,
  code review, and regression verification. Use when a bug needs structured
  diagnosis and verified repair. --e2e adds browser-based regression tests;
  --codex enables adversarial review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# bug-fix

Belt pipeline for quality-gated debugging. Pipeline structure, args, phase
order, and invocation mapping are defined in `pipeline.yml` and surfaced
dynamically by `belt-agent next` / `belt-agent status`. This SKILL.md covers
the supplement loading contract, phase-specific runtime notes, and red flags.

## Supplement Loading

Before invoking a phase's skill, read the referenced supplement to inject
phase-specific overrides. Phases not listed (`fix-plan-review` / `execute` /
`code-review`) have no supplement; invoke their declared skill directly.

| Phase | Supplement | Purpose |
|---|---|---|
| rca | `./references/rca-supplement.md` | RCA Report 5 sections, Symmetry Check, Reproduction Test FAIL, parallel exploration order, `rca-scenarios.yml` produce (when `--e2e`) |
| fix-plan | `./references/fix-plan-supplement.md` | RCA Fix Strategy → task traceability, Given/When/Then test cases, verifiable completion conditions, task granularity |
| monkey-test | `./references/monkey-test-supplement.md` | scenarios source = `docs/plans/*-rca-scenarios.yml`, first scenario verifies Reproduction Test now PASSes, glob collision resolution |
| dogfood | `./references/dogfood-supplement.md` | Impact Scope + Symmetry exploration, Root Cause re-emergence flag, CLI-only graceful degradation |
| integrate | `./references/worktrunk-supplement.md` | A/B choice logic (wt merge / gh pr create) |

## Phase-specific Runtime Notes

- **fix-plan-review**: `/spec-review:spec-review` is reused for fix-plan
  review. The grill-me prompt under the `design-judgment` observation does
  not fire by default (design decisions are already settled in rca /
  fix-plan). If it fires, treat it as a signal that upstream phases need
  to be revisited.
- **execute**: orchestrator must reconstruct fix plan tasks into
  self-contained implementation specs before dispatching
  `belt-agents:feature-implementer` subagents. Do not forward RCA / Fix
  Plan excerpts verbatim.
- **integrate**: prompt user for mode (A: `wt merge` / B: `gh pr create`)
  before executing `/worktrunk`.

## Narrative Notes

These phases produce a narrative note so context can be restored after `/clear`
(`.belt/runs/{run_id}/notes/phase-<id>.md`):

- `rca` / `fix-plan` / `execute` / `code-review`
- `monkey-test` (`--e2e`) / `dogfood` (`--e2e`)

Each note contains four sections (`## Decisions` / `## Concerns` /
`## Directives` / `## Observations`) and minimal frontmatter.

Full convention: [`plugins/belt-agents/references/narrative-convention.md`](plugins/belt-agents/references/narrative-convention.md)

## Red Flags

- **Never skip rca**: root cause must precede fix. "Fix first" is anti-pattern.
- **Never skip supplement loading when listed above**: without bug-fix specific overrides, behavior drifts.
- **Never delegate root cause synthesis to subagents**: the orchestrator must reconstruct parallel exploration results.
- **Never proceed without a failing reproduction test**: RCA blocker.
- **Never filter or omit review findings**: triage of `/code-review:code-review` and `/spec-review:spec-review` output is the user's responsibility.
- **Never bypass the integrate A/B choice**: merge-vs-PR is always user-decided.
- **Never hand-edit files under `docs/plans/<topic>-*`**: they are phase-produced; manual edits break belt's phase-start mtime filter.
- **Never modify the consumed global skills**: overrides go through `references/*-supplement.md` only.
- **Never leave the narrative note's four sections blank**: gate is `file_exists` only; empty sections pass but break downstream consumers.

## References

- `./references/path-convention.md` — `docs/plans/YYYY-MM-DD-<topic>-*` naming (SSOT)
- `./references/rca-supplement.md` — rca phase override
- `./references/fix-plan-supplement.md` — fix-plan phase override
- `./references/monkey-test-supplement.md` — monkey-test phase override
- `./references/dogfood-supplement.md` — dogfood phase override and CLI-only degradation
- `./references/worktrunk-supplement.md` — integrate phase A/B choice logic
- `plugins/belt-agents/references/narrative-convention.md` — narrative note schema (shared with feature-dev)
```

**削減量**: 116 行 → 約 70 行 (40% 減)。

**削除される既存セクション** (retain/delete 逐語列挙):

- `## Pipeline Overview` (line 16-22 相当) — 完全削除
- `## Args` 表 (line 24-29 相当) — 完全削除
- `## Phase-Specific Invocation Rules` 全体 (line 31-80 相当) — 削除。内容は
  Supplement Loading 表 + Phase-specific Runtime Notes に分解・縮約
- `### Phase 1: rca` 〜 `### Phase 8: integrate` のサブ見出し — 削除
- Phase 番号 (`Phase 1:`, ..., `Phase 8:`) の表記 — 完全削除

**保持される既存セクション** (逐語):

- frontmatter (変更なし)
- `# bug-fix` 見出し下の intro (再記述)
- `## Narrative Notes` (line 82-93 相当) — 保持、phase 番号言及は phase id
  に修正
- `## Red Flags` (line 95-105 相当) — 保持、"Phase 1 / 2 / 6 / 7 / 8" など
  phase 番号言及を削除し "supplement loading when listed above" 形式に
- `## References` (line 107-115 相当) — 保持、細部 (phase 番号言及) を phase
  id 表記に修正

## 6. belt-agent SKILL.md 新構造 (全体 audit)

Audit 観点:

- **"Concise is key"** (Anthropic best practice): 例示と reference の冗長を
  削減
- **"Avoid time-sensitive information"**: history / past-tense migration 記述を
  全削除
- **現行仕様の reference 純化**: "何ができるか" に集中し、"何ができたか" を語らない

```markdown
---
name: belt-agent
description: Belt Protocol for driving belt-agent CLI. Defines command loop, response interpretation, invoke/artifact/validate semantics, and safety constraints for LLM agents driving belt pipelines.
user-invocable: false
---

# Belt Protocol

Protocol for LLM agents driving `belt-agent` CLI — a deterministic state
machine for pipeline execution.

## Commands

```bash
belt-agent init   <pipeline.yml> [--arg key=value ...]  # Start a new run
belt-agent next   [--run <id>]                          # Get current phase info (or completion signal)
belt-agent verify [--run <id>]                          # Run gate checks for current phase
belt-agent regate [--run <id>]                          # Run regate checks for target phases
belt-agent step   [--confirm] [--run <id>]              # Advance to next phase
belt-agent status [--run <id>]                          # Inspect full run state (enriched)
```

`--run <id>` is optional on all commands; omit to use the latest run.

## Workflow

```
init → next → read phase.invoke → execute per variant →
verify (if gates) → regate (if targets) → step → next → ... → completed
```

## Reading `phase.invoke`

Every phase returned by `next` may carry an `invoke` field with one of two
variants. Read the variant and take the matching action.

| Variant | Shape | Orchestrator action |
|---|---|---|
| `skill` | `{ skill: "/name", args: { ... } }` | Invoke the Claude Code slash-command skill named in `invoke.skill`, passing `invoke.args` as parameters. |
| `pipeline` | `{ pipeline: "./path.yml", with: { ... } }` | Initialise a nested `belt-agent` run on the referenced sub-pipeline. Pass `with` as args. Treat the nested run as a black-box until it reports `completed`. |

**`pipeline` invoke — `with` template resolution.** When a `with` entry's
value is a string of the form `"args.X"` (literal prefix `args.` followed by
a single arg identifier — no nested dotted paths), resolve it against the
parent run's `args` before calling `belt-agent init --arg X=<value>`. Literal
values (bool, number, non-template string) are passed through verbatim. If
`args.X` is absent in the parent, omit the `--arg` instead of passing `null`;
the sub-pipeline's declared default applies.

If `invoke` is absent, the phase is a "pure checkpoint" with only `gate:`,
`validate:`, or `confirm:`. Proceed directly to the verify/step loop.

## Artifact Graph in `status`

`belt-agent status` returns each phase's `produces` and `consumes` as part of
the enriched view.

`produces` entries are resolved artifacts:

```json
{
  "name": "design_doc",
  "path": "docs/plans/*-design.md",
  "description": "Brainstormed design...",
  "exists": true,
  "resolved_path": "docs/plans/2026-04-11-feature-x-design.md"
}
```

`belt-core` resolves glob paths using the phase-start mtime filter: the
matching file with the newest mtime (>= phase entry timestamp) wins, ties
broken lexicographically. For concrete paths, `exists` is a direct
`std::fs::metadata` check. The `resolved_path` field is omitted from JSON
when unresolved.

`consumes` entries are artifact references — either a string (resolved by
lint against the most recent earlier phase producing that name) or
`{ "name": "...", "from": "..." }` for explicit disambiguation.

**`next` and `init` emit declared artifacts, not resolved.** The `produces`
array in `next`/`init` carries raw `{ name, path, description }` entries from
pipeline.yml — without `exists` or `resolved_path`. Filesystem resolution
only happens in `status`. Call `belt-agent status` whenever you need the
concrete path of a prior phase's output.

## Validate File Semantics

Phases may use either:

- `validate: ./criteria/name.md` (scalar file reference, relative to pipeline.yml directory)
- `validate: /abs/path.md` (absolute path)
- `validate: ["criterion one", "criterion two"]` (inline list)
- `validate: [{ file: "./x.md" }, "inline"]` (mixed)

When a validate entry is a file reference, the orchestrator MUST read the
file before `step --confirm`. The file contains the actual criteria; the
scalar in pipeline.yml is just the pointer. See
`plugins/belt-agents/references/audit-protocol.md` for the expected
criteria file format.

## Decision Rules

| Situation | Action |
|---|---|
| Phase has no `gate` | Skip `verify`. Go directly to `step`. |
| `verify` returns FAIL | Read `checks` array. Fix failing items. Re-run `verify`. Each verify invocation counts toward `max_retries`. |
| Phase has `regate` targets | After `verify` PASS, run `regate`. On FAIL, fix target phases and re-run `verify` then `regate`. |
| Phase has no `regate` targets | Skip `regate`. Go directly to `step`. |
| Phase has `validate` criteria | Verify each criterion yourself (file-ref: read file; inline: judge strings), then `step --confirm`. |
| `phase_attempts[phase] > max_retries` | `step` fails with `max_retries_exceeded`. Escalate per pipeline's `on_escalation` policy. |

Every call to `verify` increments the current phase's attempts counter
regardless of verdict. `regate` is an in-place re-verification of earlier
phases' gates; it does not modify any phase's attempts counter.

## Step Troubleshooting

When `step` returns `advanced: false`, read the `reason` field:

| `reason` | Action |
|---|---|
| `confirmation_required` | Phase has `validate` or `confirm`. Verify criteria, then `step --confirm`. |
| `verify_required` | Run `verify` first. |
| `regate_not_executed` | Run `regate` first. |
| `regate_failed` | Fix regate target phases. Re-run `verify` then `regate`. |
| `max_retries_exceeded` | Escalate. Pipeline author defines recovery via `on_escalation`. |

## Status Output

`status` returns an enriched view assembled from run state, pipeline YAML,
and output directories:

```json
{
  "status": "in_progress",
  "current_phase": "review",
  "progress": { "completed": 2, "skipped": 0, "remaining": 2, "total": 4 },
  "phases": [
    {
      "id": "build",
      "status": "completed",
      "invoke": { "skill": "/brainstorming" },
      "produces": [{ "name": "design_doc", "exists": true, "resolved_path": "docs/plans/2026-04-11-feature-x-design.md" }],
      "consumes": [],
      "outputs": ["report.json"]
    },
    {
      "id": "review",
      "status": "current",
      "invoke": { "pipeline": "../spec-review/pipeline.yml", "with": {} },
      "consumes": ["design_doc"]
    }
  ]
}
```

`produces`, `consumes`, and `invoke` are omitted when empty/absent. Treat
absence as equivalent to an empty array (or `null` for `invoke`). Use
`status` for context recovery or progress checks.

## Well-known Config Keys

`config` is an opaque map that belt passes through without interpretation.
Use it for phase-specific flags orthogonal to invocation identity (e.g.,
`codex: true`, `ui: true`, or pipeline-specific arguments). Unknown keys
MAY be ignored.

Phase-level invocation identity belongs in the typed `invoke:` field. Agent
dispatch and iteration loops are skill-layer concerns; `pipeline.yml`
references only `invoke.skill` or `invoke.pipeline`.

## HARD-GATE

<HARD-GATE>
When validate criteria exist for a phase (inline or file-reference), you
MUST NOT run `belt-agent step --confirm` without verifying each criterion.

For inline `validate: ["..."]` criteria, judge each string directly.

For file-reference `validate: ./criteria/name.md` or `validate: /abs/path.md`,
you MUST Read the referenced file first, then judge each criterion defined
inside that file. The file is the authoritative source; the scalar in
pipeline.yml is just the pointer.

The --confirm flag is the LLM's declaration that criteria have been verified.
Passing it without verification is a protocol violation.
</HARD-GATE>
```

**削減量**: 192 行 → 約 150 行 (22% 減)。

**削除される既存記述** (retain/delete 逐語列挙):

- **Line 13-38 の Commands セクション**: 6 個の個別 code block (`# Start a new
  run` から `# Inspect full run state (enriched view)` まで) を、1 つの統合
  code block に集約
- **Line 58 の Note block 全体**:
  ```
  > **Note (2026-04-16):** The `agent` and `agents` variants were removed.
  > Agent dispatch is now a skill-layer concern: a parent skill uses the
  > Task tool internally (or wraps `context: fork` + `agent:` in a child
  > skill) to launch subagents, and `pipeline.yml` references only
  > `invoke.skill`. See
  > `docs/specs/2026-04-16-review-skills-subagent-boundary-design.md`.
  ```
  完全削除
- **Line 171** の `Phase-level invocation identity moved to the typed invoke:
  field; do not use config.skill, config.agents, config.criteria, config.audit,
  or config.reference — these have been replaced by invoke:, produces:,
  consumes:, and file-reference validate: respectively.` — 完全削除
- **Line 173** の `As of 2026-04-16, invoke.agents and invoke.iterations are
  permanently removed from the Invoker schema (partial revert of BELT-32).
  Agent dispatch and iteration loops are skill-layer concerns; pipeline.yml
  now references only invoke.skill or invoke.pipeline.` — 完全削除、相当情報は
  current tense で Well-known Config Keys セクションに再表現
- **Line 131-160 の Status Output JSON 詳細**: 3 phase full-detail example を、
  2 phase (build = completed / review = current) に圧縮。`verify_passed`,
  `attempt`, `args` フィールドの例示を削除 (現行仕様で必須ではない)

**保持される既存記述** (逐語):

- frontmatter (変更なし)
- `# Belt Protocol` 見出し
- Workflow 図 (line 44-47 相当)
- Reading `phase.invoke` の variant 表と `with` template resolution 説明 (Note
  削除のみ)
- Artifact Graph 説明全体 (line 64-89 相当)
- Validate File Semantics (line 91-100 相当)
- Decision Rules 表 (line 103-113 相当)
- Step Troubleshooting 表 (line 115-125 相当)
- HARD-GATE block (line 177-192 相当)

## 7. データフロー / 情報責務の明確化

```
┌──────────────┐          ┌──────────────┐
│ pipeline.yml │ ──SSOT→  │ belt-agent   │
│              │          │ next/status  │
└──────┬───────┘          └──────┬───────┘
       │                         │
       │ (phase order, invoke,   │ (dynamic JSON:
       │  args, gate, regate,    │  phase info, artifact
       │  validate, produces,    │  resolution, progress)
       │  consumes, when,        │
       │  max_retries)           │
       │                         │
       │                         ↓
┌──────┴─────────────────────────────────┐
│ SKILL.md (pipeline-specific)            │
│                                         │
│  - Supplement Loading (phase id → path) │
│  - Phase-specific Runtime Notes         │
│  - Narrative Notes convention           │
│  - Red Flags                            │
│  - References pointer                   │
└─────────────────────────────────────────┘
```

情報の所在を単一化する:

- Phase 順序・invoke・gate・validate: **pipeline.yml のみ**
- Supplement loading map: **SKILL.md のみ** (pipeline-specific)
- belt-agent プロトコル解釈: **belt-agent SKILL.md のみ**
- Narrative note schema: **`plugins/belt-agents/references/narrative-convention.md` のみ**

## 8. エラーハンドリング / エッジケース

- **Phase 追加・順序変更時**: pipeline.yml 単独変更で済む。SKILL.md の
  supplement map は phase id ベースのため、supplement 追加が必要な場合のみ
  SKILL.md を修正。
- **Supplement と pipeline.yml の乖離**: supplement map にある phase id が
  pipeline.yml に存在しない場合の検出は lint で enforce すると望ましいが、
  本スコープ外 (別 spec)。人間レビューと `cargo test` の既存 lock test で
  検出。
- **既存 orchestrator / agent への影響**: `.claude/agents/` と `docs/specs/`
  内の SKILL.md 参照を grep し、破壊的変更がないか確認。破壊的変更があれば
  同一 plan 内で修正。
- **Plan verbatim bug 再発防止** (MEMORY 既記載の
  `feedback_subagent_prompt_verbatim_spec.md`): 後続の writing-plans 段階で、
  本 spec の retain/delete 逐語列挙セクションを controller prompt に verbatim
  転記する指示を明記。paraphrase 禁止。

## 9. テスト方針

**構造検証** (grep-based lock):

各 SKILL.md に以下のパターンが **出現しないこと** を確認:

```
# feature-dev / bug-fix SKILL.md
- Phase 1:   Phase 2:   Phase 3:   Phase 4:   Phase 5:   Phase 6:
- Phase 7:   Phase 8:   Phase 9:
- Pipeline Overview
- ## Args   (heading as section header)

# belt-agent SKILL.md
- As of 2026-   As of 2025-
- were removed   have been replaced   moved to the typed
- > **Note (2026-
```

Supplement map の phase id が pipeline.yml の phase id と一致することを手動
確認。具体的に: `grep -E '^\| (design|plan|...)' SKILL.md` と
`grep -E '^  - id:' pipeline.yml` の出力を突合。

**動作検証**:

- `cargo test --workspace` で既存の lock test (`review_skills_refresh.rs` 等)
  を通す
- feature-dev / bug-fix pipeline を smoke run (init → next → step で 1-2 phase
  だけ進める) して regression なきこと確認
- `belt-agent next` / `belt-agent status` の JSON 出力が従来と不変なこと確認
  (JSON 構造変更なし; SKILL.md 変更は belt-agent の挙動に影響しない)

**ドキュメント一貫性**:

- MEMORY.md の `project_skill_md_authoring_principle.md` と今回の SKILL.md が
  矛盾しないことを確認
- `project_review_skills_2026_04_16_boundary.md` との整合 (同日付の boundary
  refactor との関係)。特に belt-agent SKILL.md line 58 の削除対象 Note は
  boundary refactor を指していたため、削除しても情報損失はない (現行仕様で既に
  本文が語り直している)

## 10. Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| 外部 agent / subagent 設定が旧 SKILL.md セクション名 (`Phase 1:` など) に依存 | Low | 事前 grep で `.claude/agents/` と `docs/` を確認、破壊的参照は同 plan で修正 |
| Supplement map と pipeline.yml phase id の drift | Medium | 人間レビューで確認、将来 lint rule 化を検討 (別 spec) |
| belt-agent SKILL.md 削減で必要情報を失う | Low | Commands / Decision Rules / Step Troubleshooting / HARD-GATE は保持、削減は例示冗長と history のみ |
| Plan 段階で SKILL.md 削減対象が paraphrase される (MEMORY 記載の副次バグ) | Medium | spec に retain/delete を逐語列挙、plan controller prompt も verbatim |

## 11. Implementation Sequence (writing-plans で詳細化)

1. feature-dev SKILL.md を Section 4 の内容で上書き
2. bug-fix SKILL.md を Section 5 の内容で上書き
3. belt-agent SKILL.md を Section 6 の内容で上書き
4. Grep 検証実施 (Section 9 構造検証)
5. Smoke pipeline run (`cargo test --workspace` 含む)
6. 外部参照 grep 確認 (`.claude/agents/`, `docs/specs/` 内の `Phase N:` 参照)
7. MEMORY.md に今回の refresh を記録するエントリ追加
   (`project_skill_md_plugin_refresh_2026_04_16.md`)
8. MEMORY.md index 更新

各ステップの成果物と検証は writing-plans で TDD ベースにブレイクダウンする。

## 12. Related Documents

- MEMORY: `project_skill_md_authoring_principle.md` — 本 spec が実装追随する
  原則
- MEMORY: `project_review_skills_2026_04_16_boundary.md` — 同日付の boundary
  refactor (belt-agent SKILL.md line 58 が参照していた変更)
- MEMORY: `feedback_subagent_prompt_verbatim_spec.md` — plan verbatim の原則
- `docs/specs/2026-04-16-review-skills-subagent-boundary-design.md` — history
  note が指す既存 spec (削除した際の情報退避先として有効)
