# Debug Flow Refresh (/debug-flow 8-phase modernization with feature-dev + review-skills parity)

**Linear**: BELT-TBD (to be created)
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)
**Status**: Draft
**Date**: 2026-04-15

## Summary

`/debug-flow` skill を feature-dev refresh (2026-04-15 `fa04895` merged) と review-skills refresh (2026-04-15 `0e41eec` merged) と同じ骨格に刷新する。multi-agent N-way voting pattern の痕跡 (`iterations`, `swarm`, `ui` args、`consensus` severity 依存、sub-pipeline `with:` passthrough) を完全除去し、`--e2e` / `--codex` の 2 arg のみに最小化する。

Phase 構造は 8 phase 維持するが、smoke-test / test-review を削除し、feature-dev と同じ monkey-test (scripted E2E) + dogfood (exploratory E2E) に置換する。全 sub-pipeline invoke を `skill:` 方式に揃え、supplement pattern (feature-dev 式) で phase 固有 override を 4 ファイル (rca / fix-plan / monkey-test / dogfood) に分離する。旧 references (`evidence-plan-protocol.md`, `fix-dispatch-strategy.md`) は dead letter として削除する。

これにより、refresh 済みの `/code-review`, `/implementation-review`, `/monkey-test`, `/dogfood` との invoke contract が整合し、feature-dev との aesthetic / maintenance 負担が揃う。

## Background

### Problem

現行 `/debug-flow` は以下の drift を抱える:

1. **Sub-pipeline invoke の args contract 破損**: pipeline.yml は `fix-plan-review` / `code-review` / `test-review` phase で `with: { iterations, codex, ui, swarm }` を渡すが、refresh 済みの `/implementation-review` / `/code-review` / `/test-review` は **`codex` のみ** を受け付ける。`iterations / ui / swarm` は消滅済み → invoke 時点で壊れている
2. **multi-agent N-way voting の痕跡**: `args.iterations: { type: number, default: 3 }`, `args.swarm: bool`, `args.ui: bool` が pipeline.yml に残存。旧 pattern を継承したまま
3. **criteria/fix-plan-review.md の旧 pattern 依存**:
   - `FIX-PLAN-REVIEW-01` が「3 perspectives (clarity, feasibility, consistency) の execution records」を検証 → 新 `/implementation-review` は **single agent が 4 observation** (clarity, feasibility, consistency, **ui-spec**) を扱う
   - `FIX-PLAN-REVIEW-02` が「severity: **consensus** findings resolved」を検証 → 新 pattern では severity は `blocker / quality / warning` のみで `consensus` は存在しない
4. **`references/evidence-plan-protocol.md` の dead letter 化**:
   - `consensus findings count` を collect と指定 → N-way voting 痕跡
   - smoke-test activity entry 含有 → phase 削除で陳腐化
   - belt-agent に Evidence Plan inject hook が存在しない → 実装されていない可能性高
5. **`references/fix-dispatch-strategy.md` の drift**:
   - Dispatch Table に smoke-test / test-review phase (削除予定) 含有
   - code-review / test-review の fix は `feature-implementer` subagent 経由と規定 → 新 `/code-review` SKILL.md の "user direct selection + serial fix" と drift
6. **smoke-test phase の冗長**: feature-dev refresh で smoke-test は削除され monkey-test + dogfood に吸収されたが、debug-flow では独立 phase として残存
7. **test-review phase の冗長**: 同上。feature-dev refresh で削除済み

### Precedent

- **feature-dev refresh** (2026-04-15 `fa04895`): 10 phase → 8 phase、test-first 前倒し、args 7 → 2 (`e2e, codex`)、multi-agent review 廃止、`/monkey-test` + `/dogfood` 新設、supplement pattern 確立
- **review-skills refresh** (2026-04-15 `0e41eec`): 4 skill (code/spec/test/implementation-review) を single consolidated reviewer subagent に刷新、18 legacy agents 削除、args=codex のみ

`/debug-flow` は 2 先例のいずれも継承していないため、同じパターンで統合的に refresh する。

### Design Constraints (継承する憲法)

- **BELT-21**: CLI is deterministic, skill is protocol — belt-agent JSON は事実報告のみ、protocol 層は skill 責務
- **BELT-32**: Invoker + Artifact first-class — `invoke.skill` と `produces`/`consumes` 機構を活用
- **Context Neutrality (BELT-24)**: skill は multi-context / single-context で中立
- **Tiny by Constraint**: 冗長な args / agent を作らない
- **SKILL.md Authoring Principle**: pipeline.yml / belt-agent SKILL.md が表現可能なものを再掲しない

## Goals

1. Multi-agent N-way voting pattern (iterations, swarm, ui, consensus severity) を完全除去
2. 新 single-agent review skills (`/code-review`, `/implementation-review`) との invoke contract を整合 (`skill:` invoke に統一、args=`codex` のみ)
3. feature-dev と同一の 8 phase 骨格に揃える (`rca → fix-plan → fix-plan-review → execute → code-review → monkey-test → dogfood → integrate`)
4. supplement pattern (feature-dev 式) 導入 — `references/*-supplement.md` を 4 ファイル新設 (rca / fix-plan / monkey-test / dogfood)
5. dead letter references 削除 — `evidence-plan-protocol.md`, `fix-dispatch-strategy.md`
6. criteria の役割分離 — content phase (rca / fix-plan) は独立監査を保持、meta phase (fix-plan-review) は thin check に薄化
7. `--e2e` 時に `rca_scenarios.yml` を RCA phase で conditional produce、monkey-test / dogfood が consume

## Non-Goals

- **`/systematic-debugging`, `/writing-plans`, `/subagent-driven-development`, `/monkey-test`, `/dogfood`, `/worktrunk`, `/code-review`, `/implementation-review` の skill 本体修正** (invoke contract のみ揃える)
- **新 agent の作成** (既存 `phase-auditor` / `feature-implementer` / `code-reviewer` / `implementation-reviewer` で充足)
- **Phase 数の増減** (8 phase 固定)
- **debug-flow 固有要求の削除** (RCA Symmetry Check, Excluded Hypothesis, Reproduction Test FAIL, Fix Strategy → tasks traceability は維持)
- **`--linear` 統合** (今回 scope 外。現行 `argument-hint` に存在しないので削除方向で整理)
- **`/spec-review` との連携追加** (debug-flow は design doc を持たない bug fix workflow のため不要)
- **独立 `/smoke-test` skill の廃止** (user が直接呼ぶ機能としては残す、debug-flow の phase としてのみ削除)

## Design

### Core Principle

全 phase が 2 種の invocation pattern のいずれかに従う:

```
┌─────────────────────────────────────────────┐
│ Simple skill invoke                         │
│   INVOKE: Skill tool /<skill>               │
│   (fix-plan-review / execute / code-review  │
│    / integrate)                             │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ Supplement-injected skill invoke            │
│   INVOKE 1: Read ./references/*-supplement  │
│   INVOKE 2: Skill tool /<skill>             │
│   (rca / fix-plan / monkey-test / dogfood)  │
└─────────────────────────────────────────────┘
```

supplement は phase 固有 override を pipeline.yml / SKILL.md / skill 本体と独立したファイルに分離し、SSOT として保持する。

### Args

```yaml
args:
  e2e:
    type: bool
    default: false
    description: "Enable E2E testing phases (monkey-test, dogfood)"
  codex:
    type: bool
    default: false
    description: "Enable Codex parallel review in review phases"
```

**廃止**: `iterations`, `swarm`, `ui`, `smoke`

### Phase Layout (8 phases)

| # | Phase | Skill | Gate | Validate | Regate | When |
|---|---|---|---|---|---|---|
| 1 | `rca` | `/systematic-debugging` | `file_exists: docs/plans/*-rca-report.md` | `./criteria/rca.md` | — | — |
| 2 | `fix-plan` | `/writing-plans` | `file_exists: docs/plans/*-fix-plan.md` | `./criteria/fix-plan.md` | — | — |
| 3 | `fix-plan-review` | `/implementation-review` (args: codex) | — | `./criteria/fix-plan-review.md` | — | — |
| 4 | `execute` | `/subagent-driven-development` | — | `../../criteria/execute.md` | — | — |
| 5 | `code-review` | `/code-review` (args: codex) | — | `../../criteria/code-review.md` | `[execute]` | — |
| 6 | `monkey-test` | `/monkey-test` | `file_exists: docs/plans/*-monkey-test-report.md` | `./criteria/monkey-test.md` | — | `args.e2e` |
| 7 | `dogfood` | `/dogfood` | `file_exists: docs/plans/*-dogfood-report.md` | `./criteria/dogfood.md` | — | `args.e2e` |
| 8 | `integrate` | `/worktrunk` | — | `./criteria/integrate.md` | — | — |

全 phase: `confirm: true`, `max_retries: 3`。

### Produces / Consumes

| Phase | produces | consumes |
|---|---|---|
| rca | `rca_report` (always), `rca_scenarios` (when `args.e2e`) | — |
| fix-plan | `fix_plan_doc` | `rca_report` |
| fix-plan-review | — | `fix_plan_doc` |
| execute | — | `rca_report`, `fix_plan_doc` |
| code-review | — | `rca_report`, `fix_plan_doc` |
| monkey-test | `monkey_test_report`, `monkey_test_results` | `rca_report`, `rca_scenarios`, `fix_plan_doc` |
| dogfood | `dogfood_report` | `rca_report`, `rca_scenarios`, `fix_plan_doc`, `monkey_test_report`, `monkey_test_results` |
| integrate | — | `rca_report`, `fix_plan_doc` |

### pipeline.yml 全体 (target shape)

```yaml
name: debug-flow
version: 1
description: "Quality-gated debugging pipeline (8 phases)"

args:
  e2e:
    type: bool
    default: false
    description: "Enable E2E testing phases (monkey-test, dogfood)"
  codex:
    type: bool
    default: false
    description: "Enable Codex parallel review in review phases"

phases:
  - id: rca
    description: "Investigate root cause via parallel exploration"
    invoke:
      skill: /systematic-debugging
    produces:
      - name: rca_report
        path: "docs/plans/*-rca-report.md"
        description: "Root cause analysis report (Symptom / Investigation Record / Root Cause / Reproduction Test / Fix Strategy)"
      - name: rca_scenarios
        path: "docs/plans/*-rca-scenarios.yml"
        description: "Reproduction scenarios in Given/When/Then YAML for monkey-test replay"
        when: "args.e2e"
    gate:
      - file_exists: "docs/plans/*-rca-report.md"
    validate: ./criteria/rca.md
    confirm: true
    max_retries: 3

  - id: fix-plan
    description: "Create fix plan from RCA report"
    invoke:
      skill: /writing-plans
    consumes:
      - rca_report
    produces:
      - name: fix_plan_doc
        path: "docs/plans/*-fix-plan.md"
        description: "Fix plan with RCA Fix Strategy → task mapping"
    gate:
      - file_exists: "docs/plans/*-fix-plan.md"
    validate: ./criteria/fix-plan.md
    confirm: true
    max_retries: 3

  - id: fix-plan-review
    description: "Plan review via implementation-review"
    invoke:
      skill: /implementation-review
      args:
        codex: "args.codex"
    consumes:
      - fix_plan_doc
    validate: ./criteria/fix-plan-review.md
    confirm: true
    max_retries: 3

  - id: execute
    description: "TDD implementation following the fix plan"
    invoke:
      skill: /subagent-driven-development
    consumes:
      - rca_report
      - fix_plan_doc
    validate: ../../criteria/execute.md
    confirm: true
    max_retries: 3

  - id: code-review
    description: "Multi-perspective code review"
    invoke:
      skill: /code-review
      args:
        codex: "args.codex"
    consumes:
      - rca_report
      - fix_plan_doc
    validate: ../../criteria/code-review.md
    regate: [execute]
    confirm: true
    max_retries: 3

  - id: monkey-test
    description: "Replay reproduction scenarios via agent-browser"
    when: "args.e2e"
    invoke:
      skill: /monkey-test
    consumes:
      - rca_report
      - rca_scenarios
      - fix_plan_doc
    produces:
      - name: monkey_test_report
        path: "docs/plans/*-monkey-test-report.md"
      - name: monkey_test_results
        path: "docs/plans/*-monkey-test-results.json"
    gate:
      - file_exists: "docs/plans/*-monkey-test-report.md"
    validate: ./criteria/monkey-test.md
    confirm: true
    max_retries: 3

  - id: dogfood
    description: "Exploratory regression testing around fix scope"
    when: "args.e2e"
    invoke:
      skill: /dogfood
    consumes:
      - rca_report
      - rca_scenarios
      - fix_plan_doc
      - monkey_test_report
      - monkey_test_results
    produces:
      - name: dogfood_report
        path: "docs/plans/*-dogfood-report.md"
    gate:
      - file_exists: "docs/plans/*-dogfood-report.md"
    validate: ./criteria/dogfood.md
    confirm: true
    max_retries: 3

  - id: integrate
    description: "Integrate fix (user chooses: wt merge or gh pr create)"
    invoke:
      skill: /worktrunk
    consumes:
      - rca_report
      - fix_plan_doc
    validate: ./criteria/integrate.md
    confirm: true
    max_retries: 3
```

### Criteria Files

#### 配置方針

| Criteria | 配置 | 変更 |
|---|---|---|
| `criteria/rca.md` | skill-local | **修正**: RCA-09 (`rca_scenarios` exists when `--e2e`) 追加 |
| `criteria/fix-plan.md` | skill-local | 変更なし |
| `criteria/fix-plan-review.md` | skill-local | **thin 化**: -01 / -02 削除、新 -01 (artifact 存在) 追加、-03 / -04 保持 |
| `criteria/monkey-test.md` | skill-local | **新設**: feature-dev baseline + 固有項目 (reproduction scenario PASS 検証) |
| `criteria/dogfood.md` | skill-local | **新設**: feature-dev baseline + 固有項目 (Root Cause mechanism 再発検知) |
| `criteria/integrate.md` | skill-local | **新設**: inline validate (2 項目) から移行、feature-dev 同等 |
| `../../criteria/execute.md` | shared | 変更なし (feature-dev と共通) |
| `../../criteria/code-review.md` | shared | 変更なし (feature-dev と共通) |

#### criteria/fix-plan-review.md thin 化詳細

**削除**:
- `FIX-PLAN-REVIEW-01` (3 perspectives): `/implementation-review` は single agent で 4 observation を扱うため、"3 perspectives の execution records" 検証は意味をなさない
- `FIX-PLAN-REVIEW-02` (consensus findings resolved): 新 pattern では severity は `blocker / quality / warning` のみ、`consensus` は存在しない

**保持**:
- `FIX-PLAN-REVIEW-03` (RCA Report と Fix Plan の整合性, blocker): 2 artifact 間の cross-reference 検証は独立監査として有効
- `FIX-PLAN-REVIEW-04` (各 task の completion condition 検証性, blocker): content quality の独立監査として有効

**新 FIX-PLAN-REVIEW-01** (置換):

```
### FIX-PLAN-REVIEW-01: Review artifact (findings.json) exists
- severity: quality
- verify_type: automated
- verification:
  Read the review result file (`.belt/runs/*/review/findings.json` or equivalent)
  and confirm it exists with valid JSON structure.
- pass_condition: findings.json file exists and parses as valid JSON with a `findings` array
- fail_diagnosis_hint: /implementation-review invoke が中断または artifact path drift。
  再 invoke で回復可能。
- depends_on_artifacts: [.belt/runs/*/review/findings.json]
```

結果: **3 criteria** (新 -01 thin + 保持 -03 / -04)。triage / fix 詳細は `/implementation-review` SKILL.md の責務 (memory: `feedback_belt_cli_vs_skill_responsibility.md` — belt CLI JSON は事実報告のみ、プロトコル教育は skill 側)。

#### criteria/rca.md RCA-09 追加

```
### RCA-09: Reproduction scenarios file exists when --e2e
- severity: blocker (conditional)
- verify_type: automated
- verification:
  1. If args.e2e is false, mark as skipped
  2. If args.e2e is true:
     a. Glob("docs/plans/*-rca-scenarios.yml")
     b. Verify file contains ≥1 Given/When/Then scenario
- pass_condition: args.e2e=false (skipped), OR file exists with ≥1 G/W/T scenario
- fail_diagnosis_hint: --e2e=true で missing なら RCA executor が supplement を未読込。
  supplement injection 確認。
- depends_on_artifacts: [docs/plans/*-rca-scenarios.yml]
- forward_check: monkey-test phase consumes rca_scenarios
```

### References & Supplements

#### 削除 (2 ファイル)

- `references/evidence-plan-protocol.md` — N-way voting 痕跡 (`consensus findings count`)、smoke-test 陳腐化、belt-agent 未実装の可能性
- `references/fix-dispatch-strategy.md` — 削除 phase 言及、`feature-implementer` dispatch が新 `/code-review` の "user direct selection" と drift

#### 新設 (5 ファイル, `references/`)

| File | 役割 |
|---|---|
| `path-convention.md` | `docs/plans/YYYY-MM-DD-<topic>-*.md` 命名 SSOT、各 phase produce path の対応表 |
| `rca-supplement.md` | Phase 1 override — RCA Report 5 sections (+ Excluded Hypotheses) / Symmetry Check / Reproduction Test FAIL / `--e2e` 時 `rca_scenarios.yml` produce / parallel exploration (code-explorer / code-architect / impact-analyzer → orchestrator synthesis) |
| `fix-plan-supplement.md` | Phase 2 override — RCA Fix Strategy → task traceability / Given/When/Then test cases / verifiable completion conditions / task granularity (10 steps, 3 modules) / `rca_report` consume |
| `monkey-test-supplement.md` | Phase 6 override — scenarios source は `docs/plans/*-rca-scenarios.yml` (feature-dev の `docs/features/*/scenarios.yml` と異なる) / first scenario は RCA Reproduction Test PASS 確認 / regression scenarios 追加方針 |
| `dogfood-supplement.md` | Phase 7 override — 探索範囲は fix Impact Scope + Symmetry pairs / Root Cause mechanism 再発の優先確認 / feature-dev dogfood-supplement 相当は skill 本体に任せ、bug fix 固有 context のみ記述 |

**relative path 参照は取らない** (skill 間 coupling 回避、review-skills refresh で確立した skill-local 原則を継承)。重複は許容し、将来必要あれば shared に昇格。

### SKILL.md Structure

新 SKILL.md 全体:

```markdown
---
name: debug-flow
description: >-
  Quality-gated debugging pipeline (8 phases). rca → fix-plan → plan-review →
  execute → code-review → monkey-test (E2E scripted) → dogfood (E2E exploratory)
  → integrate. --e2e enables monkey-test and dogfood.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# debug-flow

Belt pipeline for quality-gated debugging. 8 phases driven by belt-agent.

## Args
<Markdown table: e2e, codex>

## Phase-Specific Invocation Rules
<Phase 1〜8 を subsection で>

## Red Flags
<never do / always do>

## References
<5 supplement file への 1-line pointer>
```

#### Phase-Specific Invocation Rules

| Phase | Invocation |
|---|---|
| 1 rca | INVOKE 1: Read `./references/rca-supplement.md` → INVOKE 2: `/systematic-debugging` |
| 2 fix-plan | INVOKE 1: Read `./references/fix-plan-supplement.md` → INVOKE 2: `/writing-plans` |
| 3 fix-plan-review | INVOKE: `/implementation-review` with `codex` passed through (supplement 不要) |
| 4 execute | INVOKE: `/subagent-driven-development` + orchestrator による task 再構築注記 |
| 5 code-review | INVOKE: `/code-review` with `codex` passed through + regate 注記 (max_retries: 3) |
| 6 monkey-test (when e2e) | INVOKE 1: Read `./references/monkey-test-supplement.md` → INVOKE 2: `/monkey-test` |
| 7 dogfood (when e2e) | INVOKE 1: Read `./references/dogfood-supplement.md` → INVOKE 2: `/dogfood` |
| 8 integrate | INVOKE: user mode prompt (A `wt merge` / B `gh pr create`) → `/worktrunk` |

#### Red Flags (8 項目)

- **Never skip Phase 1 (rca)**: root cause must precede fix. "Fix first" is anti-pattern
- **Never skip Phase 1 / 2 / 6 / 7 の supplement load**: debug-flow 固有 override が inject されず drift 発生
- **Never delegate root cause synthesis to subagents**: parallel exploration results は orchestrator が再構築
- **Never proceed without a failing reproduction test**: RCA-05 blocker
- **Never filter or omit review findings**: `/code-review`, `/implementation-review` の triage は user 責務
- **Never bypass the Phase 8 A/B choice**: merge-vs-PR は user 決定
- **Never hand-edit files under `docs/plans/<topic>-*.md`**: phase-produced、手編集で belt の phase-start mtime filter が壊れる
- **Never modify the consumed global skills**: override は `references/*-supplement.md` 経由のみ

#### 旧 SKILL.md からの削除節

| 旧節 | 削除理由 |
|---|---|
| **Dispatch Rules** | `skills/belt-agent/SKILL.md` に統合済み、重複不要 |
| **Coordinator Discipline** | Red Flags の "Never delegate root cause synthesis" に吸収 |
| **Evidence Plan** | evidence-plan-protocol.md 削除と整合 |
| **Validate / phase-auditor** 節 | belt-agent SKILL.md の汎用規則に委譲、feature-dev と同様 |

#### description / argument-hint

- **description**: 新「Quality-gated debugging pipeline (8 phases). rca → ... → integrate. --e2e enables monkey-test and dogfood.」
- **argument-hint**: `[--e2e] [--codex]` (現行 6 個から 2 個に縮小)

### Impact on Other Skills / Code

#### belt-core

- **`Artifact.when` field**: memory `project_belt_core_model_shapes_2026_04_14.md` に「Artifact.when 未実装」と記録あり。Pre-work で `Grep "when"` を belt-core model / expander / engine で実行し、実装有無を確認する。
  - **未実装の場合**: debug-flow pipeline.yml の `rca_scenarios: when: "args.e2e"` を belt-core がパースできず error。Plan 最初に belt-core `Artifact.when` 実装 task を追加する (feature-dev pipeline.yml も `scenarios: when: "args.e2e"` を既に使っているので、同一修正で両方が救済される)
  - **実装済みの場合**: そのまま使用

#### belt-core tests

- **新設**: `crates/belt-core/tests/debug_flow_refresh.rs` (先例: `review_skills_refresh.rs`)
  - 検証項目:
    - pipeline.yml の args が `{e2e, codex}` のみ (iterations / swarm / ui / smoke 不在)
    - 8 phase 名と順序
    - 各 phase の invoke skill 名 + args (`codex` のみ)
    - 各 phase の validate path が実在
    - supplement 5 files が実在
    - criteria 6 files (skill-local) + 2 files (shared) が実在
    - 旧 files (`evidence-plan-protocol.md`, `fix-dispatch-strategy.md`) が不在
    - `--e2e` expanded run で `rca_scenarios` conditional produce が適用される
    - SKILL.md に `## Phase-Specific Invocation Rules`, `## Red Flags`, `## References` 節が存在

#### Shared criteria

- `../../criteria/smoke-test.md`, `../../criteria/test-review.md` は debug-flow からの参照が削除される。
- **Pre-work**: `Grep "criteria/smoke-test.md"` / `Grep "criteria/test-review.md"` で他 skill が参照していないか確認。zero hit なら delete、参照ありなら保持 (file は残す、debug-flow からだけリンク切り)

#### feature-dev

- feature-dev は debug-flow と無関係に動作するため直接影響なし。ただし:
  - feature-dev pipeline.yml の `scenarios: when: "args.e2e"` と debug-flow pipeline.yml の `rca_scenarios: when: "args.e2e"` は **同一 belt-core 機能 (Artifact.when)** に依存する。feature-dev 側が既存 deploy で動作している場合は実装済みと推定される (Pre-work で確認)

## Migration Plan

実装は別 plan doc (`docs/plans/2026-04-15-debug-flow-refresh-plan.md`) で `/writing-plans` により詳細化する。大枠:

### Phase A: Pre-work (調査)

- A.1 belt-core `Artifact.when` 実装確認 → 未実装なら belt-core 修正 task を Phase B 前に追加
- A.2 `../../criteria/smoke-test.md` / `test-review.md` の被参照調査 (他 skill の参照有無)

### Phase B: Criteria 整備

- B.1 `criteria/rca.md` に RCA-09 追加 (conditional `--e2e`)
- B.2 `criteria/fix-plan-review.md` thin 化 (-01 / -02 削除、新 -01 追加、-03 / -04 保持)
- B.3 `criteria/monkey-test.md` 新設 (feature-dev baseline + bug fix 固有)
- B.4 `criteria/dogfood.md` 新設 (feature-dev baseline + bug fix 固有)
- B.5 `criteria/integrate.md` 新設 (feature-dev 同等)

### Phase C: References 整備

- C.1 `references/path-convention.md` 新設
- C.2 `references/rca-supplement.md` 新設
- C.3 `references/fix-plan-supplement.md` 新設
- C.4 `references/monkey-test-supplement.md` 新設
- C.5 `references/dogfood-supplement.md` 新設

### Phase D: pipeline.yml 全面書き直し

- D.1 pipeline.yml を target shape に rewrite
- D.2 `belt lint examples/skills/debug-flow/pipeline.yml` で通過確認

### Phase E: SKILL.md 全面書き直し

- E.1 SKILL.md rewrite (Phase-Specific Invocation Rules / Red Flags / References 構成)

### Phase F: Dead letter 削除

- F.1 `references/evidence-plan-protocol.md` 削除
- F.2 `references/fix-dispatch-strategy.md` 削除
- F.3 (条件付き) `../../criteria/smoke-test.md` 削除
- F.4 (条件付き) `../../criteria/test-review.md` 削除

### Phase G: Integration test

- G.1 `crates/belt-core/tests/debug_flow_refresh.rs` 新設
- G.2 `cargo test -p belt-core --test debug_flow_refresh` 通過確認
- G.3 `cargo clippy --package belt-core -- -D warnings` 通過確認
- G.4 `cargo fmt --package belt-core`

### Phase H: Dogfood (任意)

- 新 `/debug-flow` を実際の bug に対して自己 invoke し、8 phase のフローを手動検証 (feature-dev refresh / review-skills refresh でも未実施、時間許容で)

### Commit 粒度 (推奨)

| # | Commit |
|---|---|
| 1 (pre-work) | `feat(belt-core): add Artifact.when support` (未実装の場合のみ先行) |
| 2 | `refactor(debug-flow): thin fix-plan-review criteria and add RCA-09 for scenarios` |
| 3 | `refactor(debug-flow): add monkey-test / dogfood / integrate criteria` |
| 4 | `refactor(debug-flow): introduce supplement pattern and drop dead-letter references` |
| 5 | `refactor(debug-flow): rewrite pipeline.yml for feature-dev phase parity` |
| 6 | `refactor(debug-flow): rewrite SKILL.md for 8-phase monkey/dogfood flow` |
| 7 | `test(belt-core): add debug_flow_refresh integration test` |
| 8 (任意) | `chore: drop unused shared smoke-test / test-review criteria` |

## Risks

| # | Risk | Severity | Mitigation |
|---|---|---|---|
| 1 | belt-core `Artifact.when` 未実装で pipeline parse 失敗 | **high** | Pre-work A.1 で確認。未実装なら Phase B 前に belt-core 修正 task を優先。feature-dev pipeline.yml も同機能に依存するため同一修正で両方救済 |
| 2 | feature-dev の `criteria/monkey-test.md` / `dogfood.md` 実内容を読んだ結果、debug 固有差分が想定より大きい | medium | Plan 執筆時に feature-dev content を Read → 差分を spec 再確認 |
| 3 | dogfood produce path (`.md` 単一 vs ディレクトリ) の決定 | low | 本 spec で `.md` 単一採用 (debug-flow の `docs/plans/` convention 整合)。feature-dev とは意図的に divergence |
| 4 | 並行セッション branch-race (memory: `project_parallel_session_worktree_isolation.md`) | medium | subagent dispatch で絶対パス指定、subagent 側で `git branch --show-current` 事前検証、`git status --porcelain` で clean 確認 |
| 5 | `../../criteria/smoke-test.md` / `test-review.md` が他 skill 参照で削除不可 | low | Pre-work A.2 で grep、不可なら debug-flow 参照のみ削除、ファイル自体は保持 |
| 6 | single agent prompt の長大化で review 品質低下 (review-skills refresh Risk 1 と同様) | medium | 本 refresh は既存 single-agent pattern を継承するのみで新 agent 作成なし、該当リスクは適用外 |
| 7 | Plan verbatim で clippy pedantic 違反混入 (unused import / items-after-statements / use block fmt) | medium | memory `project_review_skills_refresh_2026_04_15.md` の 6 pitfalls を Plan Risk Areas に明記、`feature-implementer` subagent 指示で早期検出 |

## Open Questions

- **Observation count の将来拡張**: 新 supplement 追加時、SKILL.md References 節への追記運用で良いか (現状 OK の想定)
- **`/systematic-debugging` skill 本体への更新要求**: rca-supplement で override するだけで十分か、skill 本体に bug fix 用途の分岐を追加すべきか (Non-goal として一旦後者は保留)
- **integrate phase の `criteria/integrate.md` 記述範囲**: feature-dev と完全同一で良いか、bug fix 固有の post-merge 確認 (e.g., reproduction test が main で PASS するか) を追加すべきか (Plan 時に feature-dev content を Read して決定)
- **monkey-test の first scenario 自動挿入**: RCA Reproduction Test を scenarios.yml の最初のエントリとして自動挿入する仕組みは supplement 記述で十分か、`/monkey-test` skill 本体の責務として扱うべきか (Non-goal の一環として supplement 記述で表現)

## Approval

この design で `/writing-plans` による実装計画作成へ進む。
