# Debug Flow Refresh (/debug-flow 8-phase modernization with feature-dev + review-skills parity)

**Linear**: BELT-TBD (to be created)
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)
**Status**: Draft (spec-review applied)
**Date**: 2026-04-15

## Summary

`/debug-flow` skill を feature-dev refresh (2026-04-15 `fa04895` merged) と review-skills refresh (2026-04-15 `0e41eec` merged) と同じ骨格に刷新する。multi-agent N-way voting pattern の痕跡 (`iterations`, `swarm`, `ui` args、`consensus` severity 依存、`artifacts/reviews/` path、sub-pipeline `with:` passthrough、Observation Collection N-way 前提) を完全除去し、`--e2e` / `--codex` の 2 arg のみに最小化する。

Phase 構造は 8 phase 維持するが、smoke-test / test-review を削除し、feature-dev と同じ monkey-test (scripted E2E) + dogfood (exploratory E2E) に置換する。全 sub-pipeline invoke を `skill:` 方式に揃え、supplement pattern (feature-dev 式) で phase 固有 override を 5 ファイル (rca / fix-plan / monkey-test / dogfood / worktrunk) に分離する。旧 references (`evidence-plan-protocol.md`, `fix-dispatch-strategy.md`) は dead letter として削除する。

本 refresh は **belt-core `Artifact.when` の substantive 実装を前提条件として包含する** (spec-review Grill-me #1 で確定)。feature-dev `scenarios: when: "args.e2e"` も同じ silent drop bug を抱えており、本 PR が両方を同時に解消する副次効果を持つ。

これにより、refresh 済みの `/code-review`, `/implementation-review`, `/monkey-test`, `/dogfood` との invoke contract が整合し、feature-dev との aesthetic / maintenance 負担が揃う。

## Background

### Problem

現行 `/debug-flow` は以下の drift を抱える:

1. **Sub-pipeline invoke の args contract 破損**: pipeline.yml は `fix-plan-review` / `code-review` / `test-review` phase で `with: { iterations, codex, ui, swarm }` を渡すが、refresh 済みの `/implementation-review` / `/code-review` / `/test-review` は **`codex` のみ** を受け付ける。`iterations / ui / swarm` は消滅済み → invoke 時点で壊れている
2. **multi-agent N-way voting の痕跡**: pipeline.yml args (`iterations / swarm / ui`)、criteria (`consensus` severity、`artifacts/reviews/` path、Observation Collection 節の N-way 前提文言) に残存
3. **criteria/fix-plan-review.md の旧 pattern 依存**:
   - `FIX-PLAN-REVIEW-01` が「3 perspectives (clarity, feasibility, consistency) の execution records」を検証 → 新 `/implementation-review` は **single agent が 4 observation** (clarity, feasibility, consistency, **ui-spec**) を扱う
   - `FIX-PLAN-REVIEW-02` が「severity: **consensus** findings resolved」を検証 → 新 pattern では severity は `blocker / quality / warning` のみで `consensus` は存在しない
   - `depends_on_artifacts: [artifacts/reviews/]` は旧 path、新 path は `.belt/runs/*/review/findings.json`
4. **`references/evidence-plan-protocol.md` の dead letter 化**:
   - `consensus findings count` を collect と指定 → N-way voting 痕跡
   - smoke-test activity entry 含有 → phase 削除で陳腐化
   - belt-agent に Evidence Plan inject hook が存在しない → 未実装
5. **`references/fix-dispatch-strategy.md` の drift**:
   - Dispatch Table に smoke-test / test-review phase (削除予定) 含有
   - code-review / test-review の fix は `feature-implementer` subagent 経由と規定 → 新 `/code-review` SKILL.md の "user direct selection + serial fix" と drift
   - `examples/references/audit-protocol.md:92` から参照されているため、削除時は audit-protocol.md の参照も同時対応が必要
6. **smoke-test phase の冗長**: feature-dev refresh で smoke-test は削除され monkey-test + dogfood に吸収されたが、debug-flow では独立 phase として残存
7. **test-review phase の冗長**: 同上。feature-dev refresh で削除済み
8. **`Artifact.when` silent drop bug (確定事実)**: `crates/belt-core/src/model.rs` の `Artifact` struct は `name / path / description` のみで `when` field を持たない。`deny_unknown_fields` 未指定のため YAML の `when:` は silently drop され、feature-dev `scenarios: when: "args.e2e"` も unconditional に produce 試行される。既存 test (`feature_dev_refresh.rs`) は型を bypass して生 YAML を `serde_json::Value` で parse しているため silent drop を捕捉していない

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
- **Subagent Ownership Boundary** (memory: `project_belt_agent_ownership_2026_04_15.md`): belt-specific reviewer agents は belt repo、汎用 agent は dotfiles 維持

### Subagent Dependencies

| Agent | Location | Ownership |
|---|---|---|
| `phase-auditor` | `~/.claude/agents/` (dotfiles) | 汎用 |
| `feature-implementer` | `~/.claude/agents/` (dotfiles) | 汎用 |
| `code-reviewer` | `belt/.claude/agents/` | belt-specific |
| `implementation-reviewer` | `belt/.claude/agents/` | belt-specific |

**Prerequisite**: dotfiles (`~/.claude/`) が設定済みであること。未配備環境では phase-auditor / feature-implementer invoke が失敗する。SKILL.md References で同様注記し、Pre-work で agent 存在確認を実施する。

## Goals

1. **Multi-agent N-way voting pattern を 4 軸で完全除去**: (a) pipeline.yml args (`iterations / swarm / ui`)、(b) criteria severity (`consensus` 排除、`blocker / quality / warning` のみ)、(c) references content (旧 protocol 削除)、(d) path 命名 (`artifacts/reviews/` → `.belt/runs/*/review/`)
2. 新 single-agent review skills (`/code-review`, `/implementation-review`) との invoke contract を整合 (`skill:` invoke に統一、args=`codex` のみ)
3. **feature-dev と同じ 8 phase 構造パターンを採用**。Invariants:
   - 全 phase `invoke.skill` で単一 skill invoke (sub-pipeline 不使用)
   - review 系 phase (`fix-plan-review`, `code-review`) は `args: {codex: "args.codex"}` passthrough
   - 全 phase `max_retries: 3`, `confirm: true`
   - `code-review` のみ `regate: [execute]`
   - supplement injection (SKILL.md "INVOKE 1 / INVOKE 2" pattern) は 5 phase (`rca`, `fix-plan`, `monkey-test`, `dogfood`, `integrate`)
   - `./criteria/` skill-local + `../../criteria/` shared (`execute`, `code-review`) の split
   
   Phase 1-3 は bug fix 固有 (`rca` / `fix-plan` / `fix-plan-review`)、Phase 4-8 は feature-dev と skill / args / regate topology 完全一致
4. supplement pattern 導入 — `references/*-supplement.md` を 5 ファイル新設 (`rca`, `fix-plan`, `monkey-test`, `dogfood`, `worktrunk`) + `path-convention.md`
5. dead letter references 削除 — `evidence-plan-protocol.md`, `fix-dispatch-strategy.md` + `audit-protocol.md` の参照修正
6. **criteria の役割分離 (thin check を operational に定義)** — content phase (`rca` / `fix-plan`) は独立監査を保持、meta phase (`fix-plan-review`) は thin check (後述の定義に準拠)
7. **`--e2e` 時に `rca_scenarios.yml` を RCA phase で conditional produce**、monkey-test / dogfood が consume。success criterion: `belt-agent status --run-id X` が `--e2e=false` の run で `rca_scenarios` を artifacts フィールドに **含まない** ことを確認

## Non-Goals

- **`/systematic-debugging`, `/writing-plans`, `/subagent-driven-development`, `/monkey-test`, `/dogfood`, `/worktrunk`, `/code-review`, `/implementation-review` の skill 本体修正** (invoke contract のみ揃える)
- **新 agent の作成** (既存 `phase-auditor` / `feature-implementer` / `code-reviewer` / `implementation-reviewer` で充足、Subagent Dependencies 参照)
- **Phase 数の増減** (8 phase 固定)
- **debug-flow 固有要求の削除** (RCA Symmetry Check, Excluded Hypothesis, Reproduction Test FAIL, Fix Strategy → tasks traceability は維持)
- **`--linear` 統合** (今回 scope 外)
- **`/spec-review` との連携追加** (debug-flow は design doc を持たない bug fix workflow のため不要)
- **独立 `/smoke-test` skill の廃止** (user が直接呼ぶ機能としては残す、debug-flow の phase としてのみ削除)
- **VRT 厳密比較 / flaky 検出の automatic detection** (feature-dev と同じく scope 外。必要なら user が post-integrate で `/smoke-test` 独立実行、または CI に委ねる)
- **Incremental `--e2e` promotion mechanism** (新 run で `--e2e=true` 始動する既存動作でカバー、Design 節参照)

## Design

### Core Principle

全 phase が 2 種の invocation pattern のいずれかに従う:

```
┌─────────────────────────────────────────────┐
│ Simple skill invoke                         │
│   INVOKE: Skill tool /<skill>               │
│   (fix-plan-review / execute / code-review) │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ Supplement-injected skill invoke            │
│   INVOKE 1: Read ./references/*-supplement  │
│   INVOKE 2: Skill tool /<skill>             │
│   (rca / fix-plan / monkey-test / dogfood / │
│    integrate)                               │
└─────────────────────────────────────────────┘
```

supplement は phase 固有 override を pipeline.yml / SKILL.md / skill 本体と独立したファイルに分離し、SSOT として保持する。

### Artifact.when Semantics (prerequisite for Goal 7)

belt-core に `Artifact.when: Option<String>` を substantive 実装する。仕様:

1. **Status JSON shape (when=false)**: `belt-agent status --run-id X` の `produces` / `artifacts` フィールドから **omit** する (phase-level `when` と同じ「無かった扱い」、`skipped` flag なし)
2. **Consumes resolution**: source artifact が `when=false` の時、`resolve_artifact()` は `None` を返す (エラーではなく not-resolved)
3. **Expression**: `args.<flag>` boolean reference のみ許可 (phase-level `when` と同じ syntax 制限)。複雑 expression は YAGNI
4. **Lint warning**: `Artifact.when` の式が `args` に定義されていない flag を参照 → warning

**Status JSON サンプル** (debug-flow `--e2e=false` run):
```json
{
  "current_phase": "rca",
  "artifacts": {
    "rca_report": {"exists": true, "resolved_path": "docs/plans/2026-04-20-login-bug-rca-report.md"}
  }
}
```
`rca_scenarios` は現れない (when=false により omit)。`--e2e=true` 時のみ `rca_scenarios` も artifacts に現れる。

### Test scenario production (bug fix vs feature-dev divergence)

Q2 採択 (rca phase が `rca_scenarios.yml` を produce) の rationale:

1. **bug fix には design 不在** — feature-dev の `design → test-scenarios → plan` は design doc を基に test-first 前倒し。debug-flow では RCA Report が design 相当で、test は RCA-05 Reproduction Test 1 本が core
2. **test の性格の違い** — feature-dev の scenarios は要件 acceptance test、debug-flow の scenarios は bug 再現 + regression 確認。ISTQB/ISO 25010 baseline は前者向けで overhead
3. **最小十分** — feature-dev `criteria/test-scenarios.md` 5 項目 (Priority Matrix / Must-Verify / Coverage / Regression / Business Rule) のうち debug 用途で必要なのは「reproduction coverage」のみ。RCA-09 + rca-supplement で充足
4. **phase 数維持** — test-scenarios phase を追加すると 9 phase になり feature-dev parity の趣旨から外れる

従って test-scenarios phase は独立せず、rca phase の supplement で responsibility を inline する。

### Incremental `--e2e` promotion

初回 `--e2e=false` で run 済み後に regression 確認が必要になった場合の扱い:

- `--e2e` toggle は **新 run で対応** (既存 run を incremental に promote する機能は scope 外)
- `belt-agent init --args e2e=true` で新 run 始動、rca phase は既存 commit 済み `rca_report` を gate で immediate PASS、`rca_scenarios.yml` の追加 produce のみ実行
- monkey-test / dogfood は通常通り実行

### UI なし bug fix の dogfood graceful degradation

`/dogfood` skill は UI 以外 (CLI output / API response / logs / DB state) も対象に探索可能。`dogfood-supplement.md` で以下を override 指示:

> Impact Scope に UI ファイルが含まれない bug fix の場合、探索は CLI output, API response, logs, DB state に切り替える。DOGFOOD-04 の `≥5 new issues` threshold は rationale paragraph (「Impact Scope に UI 含まず、CLI-only exploration の妥当性」) で充足可能。

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
| 7 | `dogfood` | `/dogfood` | `file_exists: docs/plans/*-dogfood-report/report.md` | `./criteria/dogfood.md` | — | `args.e2e` |
| 8 | `integrate` | `/worktrunk` | — | `./criteria/integrate.md` | — | — |

全 phase: `confirm: true`, `max_retries: 3`。

### Produces / Consumes

| Phase | produces (name, path, when) | consumes |
|---|---|---|
| rca | `rca_report` (`docs/plans/*-rca-report.md`, always), `rca_scenarios` (`docs/plans/*-rca-scenarios.yml`, **when `args.e2e`**) | — |
| fix-plan | `fix_plan_doc` (`docs/plans/*-fix-plan.md`, always) | `rca_report` |
| fix-plan-review | — | `fix_plan_doc` |
| execute | — | `rca_report`, `fix_plan_doc` |
| code-review | — | `rca_report`, `fix_plan_doc` |
| monkey-test | `monkey_test_report` (`docs/plans/*-monkey-test-report.md`), `monkey_test_results` (`docs/plans/*-monkey-test-results.json`) | `rca_report`, `rca_scenarios`, `fix_plan_doc` |
| dogfood | `dogfood_report` (`docs/plans/*-dogfood-report/report.md`, directory form) | `rca_report`, `rca_scenarios`, `fix_plan_doc`, `monkey_test_report`, `monkey_test_results` |
| integrate | — | `rca_report`, `fix_plan_doc` |

**dogfood path form**: feature-dev `criteria/dogfood.md` DOGFOOD-04 の screenshots/videos 要件に整合させるため、**ディレクトリ形式** (`docs/plans/<topic>-dogfood-report/{report.md, screenshots/, videos/}`) を採用。

**rca_scenarios glob 衝突 semantic**: `monkey-test-supplement.md` で「glob `docs/plans/*-rca-scenarios.yml` 衝突時は最新 mtime 優先」を明示。

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
        path: "docs/plans/*-dogfood-report/report.md"
    gate:
      - file_exists: "docs/plans/*-dogfood-report/report.md"
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
| `criteria/fix-plan-review.md` | skill-local | **thin 化**: 旧 -01 / -02 削除、旧 -04 削除 (重複)、新 -01 / -02 / -03 で **3 criteria** |
| `criteria/monkey-test.md` | skill-local | **新設**: feature-dev baseline + 固有項目 (reproduction scenario PASS 検証) |
| `criteria/dogfood.md` | skill-local | **新設**: feature-dev baseline + 固有項目 (Root Cause mechanism 再発検知 + directory form + CLI-only degradation) |
| `criteria/integrate.md` | skill-local | **新設**: inline validate (2 項目) から移行、feature-dev 同等 |
| `../../criteria/execute.md` | shared | 変更なし (feature-dev と共通) |
| `../../criteria/code-review.md` | shared | 変更なし (feature-dev と共通) |

#### thin check (meta phase criteria の operational definition)

Goal 6 を以下で operationalize する:

**thin check 定義**: meta phase (review / audit phase) の criteria は以下:
1. 最大 1 個の `blocker` criterion、**artifact 存在 or cross-artifact integrity** のみ (content 内容の re-audit はしない)
2. 残りは `quality` / `warning` severity
3. 呼び出し先 skill (`/implementation-review`) が自律的に triage/fix する content 内容は re-audit しない

#### `criteria/fix-plan-review.md` 最終形 (3 criteria)

**削除**: 旧 FIX-PLAN-REVIEW-01 (3 perspectives), 旧 -02 (consensus findings), 旧 -04 (task completion content audit — `criteria/fix-plan.md` FIX-PLAN-05 の content audit と重複、Pre-work で cover 確認)

**新 FIX-PLAN-REVIEW-01** (`quality`): review artifact (findings.json) exists + valid JSON
- verification: `.belt/runs/*/review/findings.json` の存在 + JSON parse 成功
- pass_condition: file exists and parses as valid JSON with `findings` array

**新 FIX-PLAN-REVIEW-02** (旧 -03 relabel, `blocker`): RCA Report と Fix Plan の整合性 (cross-artifact integrity → thin 定義 #1)
- verification: component 名、path、type の cross-reference check
- pass_condition: 矛盾ゼロ

**新 FIX-PLAN-REVIEW-03** (`quality`, DJ#4 対策): findings.json 内の severity=blocker の unresolved findings が 0 件
- verification: findings.json を parse、severity=blocker の各 finding について fix commit or resolution コメントが紐付いているか確認
- pass_condition: unresolved blocker findings が 0 件

#### `criteria/rca.md` RCA-09 追加 (単一 boolean pass_condition)

```
### RCA-09: Reproduction scenarios file exists when --e2e
- severity: blocker
- verify_type: automated
- verification:
  1. Read args.e2e from `belt-agent status --run-id <id>` JSON
  2. If args.e2e=false → PASS (vacuously satisfied)
  3. If args.e2e=true:
     a. Glob("docs/plans/*-rca-scenarios.yml")
     b. Verify file contains ≥1 Given/When/Then scenario
- pass_condition: args.e2e=false, OR (file exists with ≥1 G/W/T scenario)
- fail_diagnosis_hint: --e2e=true で missing なら RCA executor が rca-supplement 未読込。supplement injection 確認
- depends_on_artifacts: [docs/plans/*-rca-scenarios.yml]  # only relevant when args.e2e=true
- forward_check: monkey-test consumes rca_scenarios when args.e2e=true
```

`(conditional)` qualifier 削除、`skipped` terminology 削除、pass_condition を単一 boolean に。phase-auditor は `belt-agent status --run-id` で args を read-only 取得可能。

### References & Supplements

#### 削除 (2 ファイル + 参照修正 1 箇所)

- `references/evidence-plan-protocol.md`
- `references/fix-dispatch-strategy.md`
- `examples/references/audit-protocol.md:92` の `fix-dispatch-strategy.md` 参照を除去 or 他 ref に置換 (Phase F.5 で対応)

#### 新設 (6 ファイル, `references/`)

| File | 役割 |
|---|---|
| `path-convention.md` | `docs/plans/YYYY-MM-DD-<topic>-*.md` 命名 SSOT。(a) slug 文字集合 (a-z0-9-)・長さ制約 (3-48 chars)、(b) 同日同 topic 衝突時の `-N` suffix 回避、(c) 全 artifact path 対応表 (rca_report / rca_scenarios / fix_plan_doc / monkey_test_{report,results} / dogfood_report / `docs/plans/<topic>-dogfood-report/{report.md, screenshots/, videos/}`)、(d) `bugfix/<YYYY-MM-DD-topic>` branch name 対応 |
| `rca-supplement.md` | Phase 1 override — RCA Report 5 sections (+ Excluded Hypotheses) / Symmetry Check / Reproduction Test FAIL / `--e2e` 時 `rca_scenarios.yml` produce / parallel exploration (code-explorer / code-architect / impact-analyzer → orchestrator synthesis) |
| `fix-plan-supplement.md` | Phase 2 override — RCA Fix Strategy → task traceability / G/W/T test cases / verifiable completion conditions / task granularity (10 steps, 3 modules) / `rca_report` consume |
| `monkey-test-supplement.md` | Phase 6 override — scenarios source は `docs/plans/*-rca-scenarios.yml` / first scenario は RCA Reproduction Test PASS 確認 / regression scenarios 追加方針 / glob 衝突時は最新 mtime 優先 |
| `dogfood-supplement.md` | Phase 7 override — 探索範囲は fix Impact Scope + Symmetry pairs / Root Cause mechanism 再発の優先確認 / Output path override: `docs/plans/<topic>-dogfood-report/{report.md, screenshots/, videos/}` / UI なし bug fix での CLI-only graceful degradation |
| `worktrunk-supplement.md` | Phase 8 override — feature-dev parity (A/B choice: `wt merge` vs `gh pr create`)、bug fix 固有 context (fix commit naming、reproduction test pass confirmation) |

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
<6 supplement file への 1-line pointer>
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
| 8 integrate | INVOKE 1: Read `./references/worktrunk-supplement.md` → INVOKE 2: user mode prompt (A `wt merge` / B `gh pr create`) → `/worktrunk` |

#### `argument-hint` feature-dev parity follow-up

spec-review で指摘された通り、feature-dev SKILL.md には `argument-hint` field が存在しない。本 refresh で debug-flow に追加しつつ、**feature-dev にも follow-up commit で `argument-hint: "[--e2e] [--codex]"` 追加**を Migration Plan Commit #8 に含めることで parity を維持する。

#### Red Flags (8 項目)

- **Never skip Phase 1 (rca)**: root cause must precede fix. "Fix first" is anti-pattern
- **Never skip Phase 1 / 2 / 6 / 7 / 8 の supplement load**: debug-flow 固有 override が inject されず drift 発生
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

#### Impact Analysis (structured)

**Reverse Dependencies** (本 refresh が変更する物を参照している他箇所):
- `examples/references/audit-protocol.md:92` が `fix-dispatch-strategy.md` を参照 → Phase F.5 で修正
- `skills/belt-agent/SKILL.md:100` 周辺が `audit-protocol.md` を参照 → 影響軽微 (audit-protocol.md 自体は保持、内部修正のみ)
- `crates/belt-core/tests/review_skills_refresh.rs` — 本 refresh で新 `debug_flow_refresh.rs` を類似構造で新設、既存 test は touch せず
- feature-dev pipeline.yml `scenarios: when: "args.e2e"` — 本 refresh の belt-core `Artifact.when` 実装で semantics 初成立 (silent drop 解消)

**Shared State** (複数 skill が共有する成果物):
- `../../criteria/execute.md` — feature-dev と共通 (変更なし)
- `../../criteria/code-review.md` — feature-dev と共通 (変更なし)
- `examples/skills/feature-dev/criteria/{monkey-test,dogfood,integrate}.md` — debug-flow の新 criteria 作成時の baseline、feature-dev 側は touch せず

**Implicit Contracts** (明示されない依存):
- `docs/plans/*-*.md` glob が belt の phase-start mtime filter に依存 (memory 起源、SKILL.md Red Flag で表現)
- dotfiles (`~/.claude/agents/`) に phase-auditor / feature-implementer が配置済みであること (Subagent Dependencies で明示)
- `Artifact.when` 実装後、既存 feature-dev run が silent-drop 解消により挙動変化 (無影響確認は belt-core test で)

**Side Effect Risks**:
- serde `deny_unknown_fields` 未指定による unknown field silent-drop — `Artifact.when` 実装後も他 field で同問題残存の可能性 (別 task で監査)
- glob `*-rca-scenarios.yml` 複数 run 並行時の衝突 — `monkey-test-supplement.md` で最新 mtime 優先 semantic

#### Must-Verify Checklist

Plan 実装完了後、以下を順次 verify:

- [ ] `cargo run -p belt -- lint examples/skills/debug-flow/pipeline.yml` PASS
- [ ] `cargo test -p belt-core --test debug_flow_refresh` PASS
- [ ] `cargo test -p belt-core --test feature_dev_refresh` PASS (既存、regression 確認)
- [ ] `cargo clippy --workspace -- -D warnings` PASS
- [ ] `grep -rn "iterations\|swarm\|ui\(.\|=\|:\) " examples/skills/debug-flow/` で zero hit
- [ ] `grep -rn "consensus\|artifacts/reviews/" examples/skills/debug-flow/criteria/` で zero hit
- [ ] `examples/references/audit-protocol.md` に `fix-dispatch-strategy.md` への参照 zero hit
- [ ] `belt-agent init --args e2e=false` で debug-flow run、`status --run-id` で `rca_scenarios` が artifacts に **含まない** ことを確認 (conditional produce 動作確認)
- [ ] `belt-agent init --args e2e=true` で同 run、`rca_scenarios` が含まれることを確認

#### belt-core

- **`Artifact.when` field**: **確定未実装** (`crates/belt-core/src/model.rs` 検証済み、spec-review Feas#1 / Cons#3)。Migration Phase A.1 で修正 task 必須、5 components 実装:
  1. `Artifact { when: Option<String> }` 追加
  2. expander で when 保持
  3. view/engine の produce filtering (when=false → artifacts から omit)
  4. `ArtifactRef` 解決で source が when=false の時 None 返却
  5. lint warning (undefined arg 参照時)

- feature-dev `scenarios: when: "args.e2e"` も同じ silent drop bug を抱えており、本 PR で両方が同時解消される (**feature-dev 側の別 PR は不要**)

#### belt-core tests

- **新設**: `crates/belt-core/tests/debug_flow_refresh.rs` (先例: `review_skills_refresh.rs`)
  - 検証項目:
    - pipeline.yml の args が `{e2e, codex}` のみ (iterations / swarm / ui / smoke 不在)
    - 8 phase 名と順序
    - 各 phase の invoke skill 名 + args (`codex` のみ) — Goal 3 invariants 単位
    - 各 phase `max_retries: 3`, `confirm: true`
    - `code-review` のみ `regate: [execute]`
    - supplement injection 5 phase (rca, fix-plan, monkey-test, dogfood, integrate) が該当 supplement を reference
    - 各 phase の validate path が実在
    - supplement 6 files (path-convention + 5 supplements) が実在
    - criteria 6 files (skill-local) + 2 files (shared) が実在
    - 旧 files (`evidence-plan-protocol.md`, `fix-dispatch-strategy.md`) が不在
    - **型レベル assertion**: `pipeline.phases[0].produces[1].when == Some("args.e2e".to_string())` (silent drop 回帰検出)
    - `belt-agent init --args e2e=false` で `rca_scenarios` が produce list から除外されること (runtime assertion)
    - `belt-agent init --args e2e=true` で `rca_scenarios` が produce list に残存すること (runtime assertion)
    - SKILL.md に `## Phase-Specific Invocation Rules`, `## Red Flags`, `## References` 節が存在

- **follow-up**: `crates/belt-core/tests/feature_dev_refresh.rs` に同じ型レベル assertion を retrofit (separate commit、別 PR も可)

#### Shared criteria

- `../../criteria/smoke-test.md`, `../../criteria/test-review.md` は debug-flow からの参照が削除される。
- **Pre-work**: `Grep "criteria/smoke-test.md"` / `Grep "criteria/test-review.md"` で他 skill が参照していないか確認。zero hit なら delete、参照ありなら保持

#### feature-dev

- feature-dev は debug-flow と独立に動作するため **直接影響なし**
- ただし `scenarios: when: "args.e2e"` silent drop bug は本 PR で解消 (semantic が初めて正しく動作)
- `argument-hint` follow-up (Migration Commit #8) で feature-dev SKILL.md に `[--e2e] [--codex]` 追加、parity 維持

## Migration Plan

実装は別 plan doc (`docs/plans/2026-04-15-debug-flow-refresh-plan.md`) で `/writing-plans` により詳細化する。大枠:

### Phase A: Pre-work (調査 + 確定 task)

- **A.1 (確定 task)**: belt-core `Artifact.when` 5 components 実装 (Risk #1、必須前提)
  - `crates/belt-core/src/model.rs` の `pub struct Artifact` に `when: Option<String>` 追加
  - expander で when 保持
  - view/engine の produce filtering
  - `ArtifactRef` 解決ロジック
  - lint warning
  - 関連 unit test 追加
- **A.2**: `../../criteria/smoke-test.md` / `test-review.md` の被参照調査 (他 skill の参照有無)
- **A.3**: `fix-plan.md` FIX-PLAN-05 が旧 `fix-plan-review.md` FIX-PLAN-REVIEW-04 (task completion content audit) を完全に cover しているか確認
- **A.4**: criteria files (rca.md / fix-plan.md / fix-plan-review.md) 全面 grep 監査
  - `Observation Collection` 節の phase-auditor 仕様が新 single-agent 体系と整合しているか
  - `depends_on_artifacts: [artifacts/reviews/]` (旧 path) → `.belt/runs/*/review/` (新 path) への書き換え
  - `forward_check` 文言に N-way 前提 (`consensus`, `過半数` 等) があれば書き換え
- **A.5**: agent 存在確認 (`ls ~/.claude/agents/{phase-auditor,feature-implementer}.md`、`ls belt/.claude/agents/{code-reviewer,implementation-reviewer}.md`)

### Phase B: Criteria 整備

- B.1 `criteria/rca.md` に RCA-09 追加 (単一 boolean pass_condition)
- B.2 `criteria/fix-plan-review.md` 全面書き換え (3 criteria: 新 -01 / 新 -02 / 新 -03)
- B.3 `criteria/monkey-test.md` 新設 (feature-dev baseline + bug fix 固有)
- B.4 `criteria/dogfood.md` 新設 (feature-dev baseline + directory form + CLI-only degradation)
- B.5 `criteria/integrate.md` 新設 (feature-dev 同等)

### Phase C: References 整備

- C.1 `references/path-convention.md` 新設 (4 項目: slug / 衝突 / 対応表 / branch)
- C.2 `references/rca-supplement.md` 新設
- C.3 `references/fix-plan-supplement.md` 新設
- C.4 `references/monkey-test-supplement.md` 新設
- C.5 `references/dogfood-supplement.md` 新設 (+ CLI-only degradation)
- C.6 `references/worktrunk-supplement.md` 新設

### Phase D: pipeline.yml 全面書き直し

- D.1 pipeline.yml を target shape に rewrite (Artifact.when を使った conditional produce を含む)
- D.2 `belt lint examples/skills/debug-flow/pipeline.yml` で通過確認

### Phase E: SKILL.md 全面書き直し

- E.1 SKILL.md rewrite (Phase-Specific Invocation Rules / Red Flags / References 構成、argument-hint 追加)

### Phase F: Dead letter 削除 + 参照修正

- F.1 `references/evidence-plan-protocol.md` 削除
- F.2 `references/fix-dispatch-strategy.md` 削除
- F.3 (条件付き) `../../criteria/smoke-test.md` 削除
- F.4 (条件付き) `../../criteria/test-review.md` 削除
- F.5 `examples/references/audit-protocol.md` から `fix-dispatch-strategy.md` 参照を除去 or 他 ref に置換

### Phase G: Integration test

- G.1 `crates/belt-core/tests/debug_flow_refresh.rs` 新設 (型レベル + runtime assertion 含む)
- G.2 `cargo test -p belt-core --test debug_flow_refresh` 通過確認
- G.3 `cargo test -p belt-core --test feature_dev_refresh` 通過確認 (regression)
- G.4 `cargo clippy --workspace -- -D warnings` 通過確認
- G.5 `cargo fmt --package belt-core`

### Phase H: feature-dev follow-up

- H.1 feature-dev SKILL.md に `argument-hint: "[--e2e] [--codex]"` 追加 (parity 維持)
- H.2 (任意) `feature_dev_refresh.rs` に型レベル assertion retrofit (silent drop 回帰検出)

### Phase I: Dogfood (任意)

- 新 `/debug-flow` を実際の bug に対して自己 invoke し、8 phase のフローを手動検証

### Commit 粒度 (推奨)

| # | Commit |
|---|---|
| 1 | `feat(belt-core): add Artifact.when support` (**必須前提**、Phase A.1) |
| 2 | `refactor(debug-flow): thin fix-plan-review criteria, RCA-09 for scenarios, criteria cleanup` |
| 3 | `refactor(debug-flow): add monkey-test / dogfood / integrate criteria` |
| 4 | `refactor(debug-flow): introduce supplement pattern (6 files) and drop dead-letter references` |
| 5 | `refactor(debug-flow): rewrite pipeline.yml for feature-dev phase parity` |
| 6 | `refactor(debug-flow): rewrite SKILL.md for 8-phase monkey/dogfood flow` |
| 7 | `test(belt-core): add debug_flow_refresh integration test` |
| 8 | `docs(feature-dev): add argument-hint for parity with debug-flow` |
| 9 (任意) | `chore: drop unused shared smoke-test / test-review criteria` |
| 10 (任意) | `test(belt-core): retrofit type-level when assertions to feature_dev_refresh` |

## Risks

| # | Risk | Severity | Mitigation |
|---|---|---|---|
| 1 | belt-core `Artifact.when` 実装が想定より大きい (5 components spanning model / expander / view / engine / lint) | **high** | Phase A.1 を最優先の確定 task として先行実装。5 components を plan task で個別分割、既存 `review_skills_refresh.rs` type-level assertion pattern を参照 |
| 2 | feature-dev の `criteria/monkey-test.md` / `dogfood.md` 実内容を読んだ結果、debug 固有差分が想定より大きい | medium | Plan 執筆時に feature-dev content を Read → 差分を spec 再確認 |
| 3 | 並行セッション branch-race (memory: `project_parallel_session_worktree_isolation.md`) | medium | subagent dispatch で絶対パス指定、subagent 側で `git branch --show-current` 事前検証、`git status --porcelain` で clean 確認 |
| 4 | `../../criteria/smoke-test.md` / `test-review.md` が他 skill 参照で削除不可 | low | Pre-work A.2 で grep、不可なら debug-flow 参照のみ削除、ファイル自体は保持 |
| 5 | Plan verbatim で clippy pedantic 違反混入 (unused import / items-after-statements / use block fmt) | medium | memory `project_review_skills_refresh_2026_04_15.md` の 6 pitfalls を Plan Risk Areas に明記、`feature-implementer` subagent 指示で早期検出 |
| 6 | feature-dev silent drop が本 refresh で解消されることによる既存 run の挙動変化 | low | Phase G.3 で `feature_dev_refresh.rs` を regression 実行、semantics 変化が test expectation と整合 |
| 7 | `audit-protocol.md` の `fix-dispatch-strategy.md` 参照削除で他 skill に影響 | low | Phase F.5 で実施、Pre-work で audit-protocol.md の被参照調査 |

## Approval

この design で `/writing-plans` による実装計画作成へ進む。

---

**Spec-review 適用済み** (2026-04-15):
- Grill-me 11 件 (Req #1-#6, DJ #1-#3, #5-#6) resolve
- Selection 9 件 accept (Cons #1-#2, #4-#6, #8 + Feas #3, #5, #6 + Cons #7 統合)
- Reject 1 件 (Feas #2 は Grill-me #2 で cover 済み)
