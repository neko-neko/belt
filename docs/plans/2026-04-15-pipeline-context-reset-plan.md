# Pipeline Context Reset Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `feature-dev` / `bug-fix` pipeline に phase-scoped narrative artifact を組み込み、user が任意の境界で `/clear` しても narrative から context を復元できる状態にする。

**Architecture:** 2026-04-14 の context-neutral narrative artifact spec で belt-core / belt-agent 側に既に実装済みの機構（`{run_id}` template、`.belt/runs/{run_id}/notes/` directory、`file_exists` gate、`ArtifactRef::Named`）を pipeline YAML 側で初めて active 利用する。変更は pipeline YAML + SKILL 層 convention のみで、belt-core / belt-agent source には触れない。

**Tech Stack:** Rust (belt-core integration tests), YAML (pipeline 定義), Markdown (criteria / SKILL.md / convention doc)

**Spec:** [`docs/specs/2026-04-15-pipeline-context-reset-design.md`](../specs/2026-04-15-pipeline-context-reset-design.md)

---

## File Structure

### NEW (1 file)
- `plugins/belt-agents/references/narrative-convention.md` — narrative note format SSOT (path template, frontmatter, 4 sections, examples)

### MODIFIED (18 files)

**Pipeline YAML (2):**
- `plugins/feature-dev/skills/feature-dev/pipeline.yml` — 6 phases (design/plan/execute/code-review/monkey-test/dogfood) に narrative produces/consumes/gate 追加
- `plugins/bug-fix/skills/bug-fix/pipeline.yml` — 6 phases (rca/fix-plan/execute/code-review/monkey-test/dogfood) に narrative produces/consumes/gate 追加

**SKILL.md (2):**
- `plugins/feature-dev/skills/feature-dev/SKILL.md` — Narrative Notes section + Red Flag + References
- `plugins/bug-fix/skills/bug-fix/SKILL.md` — 同上

**Skill-local criteria (8):**
- `plugins/feature-dev/skills/feature-dev/criteria/{design,plan,monkey-test,dogfood}.md`
- `plugins/bug-fix/skills/bug-fix/criteria/{rca,fix-plan,monkey-test,dogfood}.md`

**Per-plugin duplicated criteria (4 — 2 files × 2 plugins):**
- `plugins/feature-dev/skills/feature-dev/criteria/execute.md`
- `plugins/feature-dev/skills/feature-dev/criteria/code-review.md`
- `plugins/bug-fix/skills/bug-fix/criteria/execute.md`
- `plugins/bug-fix/skills/bug-fix/criteria/code-review.md`

**belt-core integration tests (2):**
- `crates/belt-core/tests/feature_dev_refresh.rs`
- `crates/belt-core/tests/bug_fix_refresh.rs`

**合計: 19 files（1 new + 18 modified）**

---

## Task 1: narrative-convention.md SSOT 作成

**Files:**
- Create: `plugins/belt-agents/references/narrative-convention.md`

`plugins/belt-agents/references/` に既存 reference（`audit-protocol.md`, `criteria-template.md` 等）と並べて配置する SSOT。

- [ ] **Step 1: Write narrative-convention.md**

```markdown
# Narrative Note Convention

Phase-scoped narrative note の規約。`feature-dev` / `bug-fix` の narrative-producing phase から produce される。belt は content を parse しないため、本 convention は SKILL 層の責務。

## Purpose

User が `/clear` で session context をリセットした後、narrative note を読むことで各 phase の判断・懸念・指示・観察を復元できるようにする。domain artifact（`design.md` / `plan.md` / `rca-report.md` 等）が **何を作ったか** を記録するのに対し、narrative note は **なぜそう判断したか / 何が未解決か / 次 phase が守るべき前提は何か** を記録する。

## Path

```
.belt/runs/{run_id}/notes/phase-{phase_id}.md
```

- `{run_id}` は belt-core が template 展開する（Engine が init 時に `<run_dir>/notes/` directory を作成）
- `{phase_id}` は pipeline.yml の `phases[].id` と同一（hyphen 保持: `monkey-test` → `phase-monkey-test.md`）
- Domain artifact (`docs/features/*`, `docs/plans/*`) とは別 directory

## File Schema

```markdown
---
phase: <phase_id>
run_id: <run_id>
---

## Decisions

<この phase で確定した設計判断・方針>

## Concerns

<未解決の懸念・リスク・下流で注意すべき事項>

## Directives

<次以降の phase への指示・前提条件>

## Observations

<事実記録・探索で判明した事項・テスト結果など>
```

## Rules

1. **Frontmatter は必須 2 field のみ**: `phase`, `run_id`。belt は parse しないが、下流 consumer (skill / LLM) が出自を追跡できるようにする。`run_id` は `belt-agent step` / `belt-agent status` 出力の `run_id` 値を LLM が書き写す
2. **4 section は全て必須**: `## Decisions` / `## Concerns` / `## Directives` / `## Observations`。空でも heading のみ残すこと（下流 consumer が section 欠落で混乱しないため）
3. **Section 順序は固定**: Decisions → Concerns → Directives → Observations
4. **各 section は簡潔に**: `/clear` 後の LLM が判断を再構成できる最低限の情報を含む。Domain artifact に書いた内容の複写は避ける（path 参照で十分）
5. **Code block / link は自由**: Markdown 規約内であれば自由。ただし冗長な再説明は避ける

## Section 別記入指針

### Decisions
- 何を決めたか。代替案を捨てた理由
- 下流 phase が「なぜその選択か」を問う時に答えになる情報
- 例: "NoSQL 候補を捨てて PostgreSQL 採用。理由: 既存 schema migration infra を流用できるため"

### Concerns
- 未解決の risk / 仮定 / 検証不足事項
- 下流が注意すべき時限爆弾
- 例: "monkey-test で E2E 未検証。dogfood 時に手動確認が必須"

### Directives
- 次以降の phase が守るべき制約・前提
- 例: "plan phase で task granularity を 30 分以下に保つこと（design で合意）"

### Observations
- 探索で判明した事実（特に domain artifact に書ききれないもの）
- 将来の調査で有用な context
- 例: "既存 `FooService` は actually Bar interface を実装していない（lint warn を確認）"

## Example: feature-dev design phase

```markdown
---
phase: design
run_id: 01947abc-1234-7890-def0-123456789abc
---

## Decisions

- Context reset mechanism は既存 belt-core narrative 機構 (2026-04-14 spec) を再利用する。belt-core への新規追加はしない
- narrative note は 6 phases のみで produce（軽量 phase は除外、user 合意済み）

## Concerns

- `/clear` は user 手動操作に依存。SKILL.md に "reset するタイミングの目安" を記述しない限り、note が参照されない可能性あり

## Directives

- plan phase: 実装タスクは 30 分以下の粒度に保つこと
- execute phase: narrative の Decisions を commit message に引用しないこと（noise になる）

## Observations

- narrative-convention.md は `plugins/belt-agents/references/` に既存 reference と並列配置
- plugin 移行で criteria は各 plugin に per-plugin 化済み（parity test で drift 検出）
```
```

- [ ] **Step 2: Verify file created**

Run: `ls plugins/belt-agents/references/narrative-convention.md`
Expected: file が存在すること

- [ ] **Step 3: Commit**

```bash
git add plugins/belt-agents/references/narrative-convention.md
git commit -m "$(cat <<'EOF'
docs(skills): add narrative-convention SSOT for phase notes

Introduce shared convention for phase-scoped narrative notes produced by
feature-dev and bug-fix pipelines. Records path template, minimal
frontmatter (phase, run_id), 4 required sections (Decisions / Concerns /
Directives / Observations), and per-section guidance.

belt remains content-neutral; this convention is a SKILL-layer contract
referenced by both pipelines' SKILL.md and criteria files.
EOF
)"
```

---

## Task 2: feature-dev pipeline narrative (TDD)

**Files:**
- Modify: `crates/belt-core/tests/feature_dev_refresh.rs` (末尾に新規テスト 6 個追加)
- Modify: `plugins/feature-dev/skills/feature-dev/pipeline.yml` (6 phases に narrative 追加)

- [ ] **Step 1: Update imports and add failing tests to `feature_dev_refresh.rs`**

まず既存 top-of-file の `use belt_core::{...}` block を以下に置き換える（`Artifact, ArtifactRef, GateCheck, Phase` を `model::` import に追加）:

```rust
use belt_core::{
    error::BeltError,
    expander::expand_pipeline,
    model::{ArgType, Artifact, ArtifactRef, GateCheck, Phase},
    parser::parse_pipeline,
};
```

次に file 末尾に以下を追加:

```rust
// --- narrative artifact shape (context reset) ---

/// feature-dev の narrative-producing phase リスト。
/// (phase_id, artifact_name, path)
const FEATURE_DEV_NARRATIVE_PHASES: &[(&str, &str, &str)] = &[
    ("design", "design_notes", ".belt/runs/{run_id}/notes/phase-design.md"),
    ("plan", "plan_notes", ".belt/runs/{run_id}/notes/phase-plan.md"),
    ("execute", "execute_notes", ".belt/runs/{run_id}/notes/phase-execute.md"),
    ("code-review", "code_review_notes", ".belt/runs/{run_id}/notes/phase-code-review.md"),
    ("monkey-test", "monkey_test_notes", ".belt/runs/{run_id}/notes/phase-monkey-test.md"),
    ("dogfood", "dogfood_notes", ".belt/runs/{run_id}/notes/phase-dogfood.md"),
];

fn find_phase<'a>(pipeline: &'a belt_core::model::Pipeline, id: &str) -> &'a Phase {
    pipeline
        .phases
        .iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("phase '{id}' must exist"))
}

fn find_produce<'a>(phase: &'a Phase, name: &str) -> &'a Artifact {
    phase
        .produces
        .iter()
        .find(|a| a.name == name)
        .unwrap_or_else(|| panic!("phase '{}' must produce '{name}'", phase.id))
}

fn has_file_exists_gate(phase: &Phase, path: &str) -> bool {
    phase.gate.iter().any(|g| matches!(g, GateCheck::FileExists { file_exists } if file_exists == path))
}

fn has_named_consume(phase: &Phase, name: &str) -> bool {
    phase
        .consumes
        .iter()
        .any(|r| matches!(r, ArtifactRef::Named(n) if n == name))
}

#[test]
fn feature_dev_narrative_phases_produce_notes() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;
    for (phase_id, artifact_name, path) in FEATURE_DEV_NARRATIVE_PHASES {
        let phase = find_phase(&pipeline, phase_id);
        let note = find_produce(phase, artifact_name);
        assert_eq!(
            note.path, *path,
            "phase '{phase_id}' note path mismatch"
        );
    }
    Ok(())
}

#[test]
fn feature_dev_narrative_phases_gate_notes() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;
    for (phase_id, _, path) in FEATURE_DEV_NARRATIVE_PHASES {
        let phase = find_phase(&pipeline, phase_id);
        assert!(
            has_file_exists_gate(phase, path),
            "phase '{phase_id}' must gate on file_exists: '{path}'"
        );
    }
    Ok(())
}

#[test]
fn feature_dev_narrative_accumulating_consumes() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;

    // accumulating: 各 narrative phase は それ以前の narrative phases の notes を全て consume。
    let expected_consumes: &[(&str, &[&str])] = &[
        ("design", &[]),
        ("plan", &["design_notes"]),
        ("execute", &["design_notes", "plan_notes"]),
        ("code-review", &["design_notes", "plan_notes", "execute_notes"]),
        (
            "monkey-test",
            &["design_notes", "plan_notes", "execute_notes", "code_review_notes"],
        ),
        (
            "dogfood",
            &[
                "design_notes",
                "plan_notes",
                "execute_notes",
                "code_review_notes",
                "monkey_test_notes",
            ],
        ),
    ];

    for (phase_id, names) in expected_consumes {
        let phase = find_phase(&pipeline, phase_id);
        for name in *names {
            assert!(
                has_named_consume(phase, name),
                "phase '{phase_id}' must consume '{name}'"
            );
        }
    }
    Ok(())
}

#[test]
fn feature_dev_non_narrative_phases_have_no_notes() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;
    for phase_id in ["test-scenarios", "spec-review", "integrate"] {
        let phase = find_phase(&pipeline, phase_id);
        for artifact in &phase.produces {
            assert!(
                !artifact.path.starts_with(".belt/runs/"),
                "phase '{phase_id}' must not produce belt notes, got '{}'",
                artifact.path
            );
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Run tests to verify FAIL**

Run: `cargo test -p belt-core --test feature_dev_refresh`
Expected: `feature_dev_narrative_phases_produce_notes` / `feature_dev_narrative_phases_gate_notes` / `feature_dev_narrative_accumulating_consumes` が FAIL（既存 pipeline.yml に narrative が無いため）。`feature_dev_non_narrative_phases_have_no_notes` は PASS。

- [ ] **Step 3: Update `plugins/feature-dev/skills/feature-dev/pipeline.yml`**

以下のように 6 phases を更新。diff は `+` 付き行が追加。

**design phase** (既存 L16-28 を置き換え):

```yaml
  - id: design
    description: "Generate design document via interactive brainstorming"
    invoke:
      skill: /brainstorming
    produces:
      - name: design_doc
        path: "docs/features/*/design.md"
        description: "Design document with explored context and test perspectives"
      - name: design_notes
        path: ".belt/runs/{run_id}/notes/phase-design.md"
        description: "Phase narrative: decisions, concerns, directives, observations"
    gate:
      - file_exists: "docs/features/*/design.md"
      - file_exists: ".belt/runs/{run_id}/notes/phase-design.md"
    validate: ./criteria/design.md
    confirm: true
    max_retries: 3
```

**plan phase** (既存 L65-80 を置き換え):

```yaml
  - id: plan
    description: "Generate implementation plan from design and test strategy"
    invoke:
      skill: /writing-plans
    consumes:
      - design_doc
      - test_strategy
      - design_notes
    produces:
      - name: plan_doc
        path: "docs/features/*/plan.md"
        description: "Task-level implementation plan (TDD)"
      - name: plan_notes
        path: ".belt/runs/{run_id}/notes/phase-plan.md"
        description: "Phase narrative"
    gate:
      - file_exists: "docs/features/*/plan.md"
      - file_exists: ".belt/runs/{run_id}/notes/phase-plan.md"
    validate: ./criteria/plan.md
    confirm: true
    max_retries: 3
```

**execute phase** (既存 L82-91 を置き換え):

```yaml
  - id: execute
    description: "Execute implementation plan via TDD subagents"
    invoke:
      skill: /subagent-driven-development
    consumes:
      - design_doc
      - plan_doc
      - design_notes
      - plan_notes
    produces:
      - name: execute_notes
        path: ".belt/runs/{run_id}/notes/phase-execute.md"
        description: "Phase narrative"
    gate:
      - file_exists: ".belt/runs/{run_id}/notes/phase-execute.md"
    validate: ./criteria/execute.md
    confirm: true
    max_retries: 3
```

**code-review phase** (既存 L93-105 を置き換え):

```yaml
  - id: code-review
    description: "Multi-perspective code review"
    invoke:
      skill: /code-review:code-review
      args:
        codex: "args.codex"
    consumes:
      - design_doc
      - plan_doc
      - design_notes
      - plan_notes
      - execute_notes
    produces:
      - name: code_review_notes
        path: ".belt/runs/{run_id}/notes/phase-code-review.md"
        description: "Phase narrative"
    gate:
      - file_exists: ".belt/runs/{run_id}/notes/phase-code-review.md"
    validate: ./criteria/code-review.md
    regate: [execute]
    confirm: true
    max_retries: 3
```

**monkey-test phase** (既存 L107-126 を置き換え):

```yaml
  - id: monkey-test
    description: "Replay pre-defined scenarios via agent-browser"
    when: "args.e2e"
    invoke:
      skill: /monkey-test:monkey-test
    consumes:
      - design_doc
      - test_strategy
      - scenarios
      - plan_doc
      - design_notes
      - plan_notes
      - execute_notes
      - code_review_notes
    produces:
      - name: monkey_test_report
        path: "docs/features/*/monkey-test-report.md"
      - name: monkey_test_results
        path: "docs/features/*/monkey-test-results.json"
      - name: monkey_test_notes
        path: ".belt/runs/{run_id}/notes/phase-monkey-test.md"
        description: "Phase narrative"
    gate:
      - file_exists: "docs/features/*/monkey-test-report.md"
      - file_exists: ".belt/runs/{run_id}/notes/phase-monkey-test.md"
    validate: ./criteria/monkey-test.md
    confirm: true
    max_retries: 3
```

**dogfood phase** (既存 L128-147 を置き換え):

```yaml
  - id: dogfood
    description: "Exploratory testing via agent-browser with feature context"
    when: "args.e2e"
    invoke:
      skill: /dogfood
    consumes:
      - design_doc
      - test_strategy
      - scenarios
      - monkey_test_report
      - monkey_test_results
      - plan_doc
      - design_notes
      - plan_notes
      - execute_notes
      - code_review_notes
      - monkey_test_notes
    produces:
      - name: dogfood_report
        path: "docs/features/*/dogfood-report/report.md"
      - name: dogfood_notes
        path: ".belt/runs/{run_id}/notes/phase-dogfood.md"
        description: "Phase narrative"
    gate:
      - file_exists: "docs/features/*/dogfood-report/report.md"
      - file_exists: ".belt/runs/{run_id}/notes/phase-dogfood.md"
    validate: ./criteria/dogfood.md
    confirm: true
    max_retries: 3
```

他の phase（test-scenarios / spec-review / integrate）は **変更なし**。

- [ ] **Step 4: Run tests to verify PASS**

Run: `cargo test -p belt-core --test feature_dev_refresh`
Expected: 全 test が PASS（既存 + 新規 4 個）

- [ ] **Step 5: Run belt lint to verify pipeline is valid**

Run: `cargo run -p belt -- lint plugins/feature-dev/skills/feature-dev/pipeline.yml`
Expected: lint PASS、unprotected produces warning 0 件（narrative produces が全て gate で保護されているため）

- [ ] **Step 6: Apply fmt and run clippy**

Run: `cargo fmt -p belt-core` (formats belt-core in place; stages changes if any)
Run: `cargo clippy -p belt-core -- -D warnings`
Expected: clippy PASS with zero warnings. fmt applies silently if adjustments needed

- [ ] **Step 7: Commit**

```bash
git add crates/belt-core/tests/feature_dev_refresh.rs plugins/feature-dev/skills/feature-dev/pipeline.yml
git commit -m "$(cat <<'EOF'
feat(feature-dev): add narrative notes to 6 phases for context reset

Integrate phase-scoped narrative artifact into feature-dev pipeline:
- design / plan / execute / code-review / monkey-test (e2e) / dogfood (e2e)
- Each narrative phase produces .belt/runs/{run_id}/notes/phase-<id>.md
- file_exists gate enforces deterministic note submission
- Accumulating consume: each phase reads all prior narrative notes

Non-narrative phases (test-scenarios / spec-review / integrate) unchanged.

Refs: docs/specs/2026-04-15-pipeline-context-reset-design.md
EOF
)"
```

---

## Task 3: bug-fix pipeline narrative (TDD)

**Files:**
- Modify: `crates/belt-core/tests/bug_fix_refresh.rs`
- Modify: `plugins/bug-fix/skills/bug-fix/pipeline.yml`

- [ ] **Step 1: Update imports and add failing tests to `bug_fix_refresh.rs`**

まず既存 top-of-file の `use belt_core::{...}` block を以下に置き換える（`Artifact, ArtifactRef, GateCheck, Phase` を既存 `model::` import に追加。`Invoker` は既存テストで使用中なので保持）:

```rust
use belt_core::{
    expander::expand_pipeline,
    model::{ArgType, Artifact, ArtifactRef, GateCheck, Invoker, Phase, Pipeline},
    parser::parse_pipeline,
};
```

次に file 末尾に以下を追加:

```rust
// --- narrative artifact shape (context reset) ---

const BUG_FIX_NARRATIVE_PHASES: &[(&str, &str, &str)] = &[
    ("rca", "rca_notes", ".belt/runs/{run_id}/notes/phase-rca.md"),
    ("fix-plan", "fix_plan_notes", ".belt/runs/{run_id}/notes/phase-fix-plan.md"),
    ("execute", "execute_notes", ".belt/runs/{run_id}/notes/phase-execute.md"),
    ("code-review", "code_review_notes", ".belt/runs/{run_id}/notes/phase-code-review.md"),
    ("monkey-test", "monkey_test_notes", ".belt/runs/{run_id}/notes/phase-monkey-test.md"),
    ("dogfood", "dogfood_notes", ".belt/runs/{run_id}/notes/phase-dogfood.md"),
];

fn find_phase<'a>(pipeline: &'a Pipeline, id: &str) -> &'a Phase {
    pipeline
        .phases
        .iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("phase '{id}' must exist"))
}

fn find_produce<'a>(phase: &'a Phase, name: &str) -> &'a Artifact {
    phase
        .produces
        .iter()
        .find(|a| a.name == name)
        .unwrap_or_else(|| panic!("phase '{}' must produce '{name}'", phase.id))
}

fn has_file_exists_gate(phase: &Phase, path: &str) -> bool {
    phase.gate.iter().any(|g| matches!(g, GateCheck::FileExists { file_exists } if file_exists == path))
}

fn has_named_consume(phase: &Phase, name: &str) -> bool {
    phase
        .consumes
        .iter()
        .any(|r| matches!(r, ArtifactRef::Named(n) if n == name))
}

#[test]
fn bug_fix_narrative_phases_produce_notes() {
    let pipeline = bug_fix_pipeline();
    for (phase_id, artifact_name, path) in BUG_FIX_NARRATIVE_PHASES {
        let phase = find_phase(&pipeline, phase_id);
        let note = find_produce(phase, artifact_name);
        assert_eq!(note.path, *path, "phase '{phase_id}' note path mismatch");
    }
}

#[test]
fn bug_fix_narrative_phases_gate_notes() {
    let pipeline = bug_fix_pipeline();
    for (phase_id, _, path) in BUG_FIX_NARRATIVE_PHASES {
        let phase = find_phase(&pipeline, phase_id);
        assert!(
            has_file_exists_gate(phase, path),
            "phase '{phase_id}' must gate on file_exists: '{path}'"
        );
    }
}

#[test]
fn bug_fix_narrative_accumulating_consumes() {
    let pipeline = bug_fix_pipeline();

    let expected_consumes: &[(&str, &[&str])] = &[
        ("rca", &[]),
        ("fix-plan", &["rca_notes"]),
        ("execute", &["rca_notes", "fix_plan_notes"]),
        (
            "code-review",
            &["rca_notes", "fix_plan_notes", "execute_notes"],
        ),
        (
            "monkey-test",
            &[
                "rca_notes",
                "fix_plan_notes",
                "execute_notes",
                "code_review_notes",
            ],
        ),
        (
            "dogfood",
            &[
                "rca_notes",
                "fix_plan_notes",
                "execute_notes",
                "code_review_notes",
                "monkey_test_notes",
            ],
        ),
    ];

    for (phase_id, names) in expected_consumes {
        let phase = find_phase(&pipeline, phase_id);
        for name in *names {
            assert!(
                has_named_consume(phase, name),
                "phase '{phase_id}' must consume '{name}'"
            );
        }
    }
}

#[test]
fn bug_fix_non_narrative_phases_have_no_notes() {
    let pipeline = bug_fix_pipeline();
    for phase_id in ["fix-plan-review", "integrate"] {
        let phase = find_phase(&pipeline, phase_id);
        for artifact in &phase.produces {
            assert!(
                !artifact.path.starts_with(".belt/runs/"),
                "phase '{phase_id}' must not produce belt notes, got '{}'",
                artifact.path
            );
        }
    }
}
```

- [ ] **Step 2: Run tests to verify FAIL**

Run: `cargo test -p belt-core --test bug_fix_refresh`
Expected: 新規 3 test が FAIL、`bug_fix_non_narrative_phases_have_no_notes` は PASS

- [ ] **Step 3: Update `plugins/bug-fix/skills/bug-fix/pipeline.yml`**

**rca phase** (既存 L16-32 を置き換え):

```yaml
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
      - name: rca_notes
        path: ".belt/runs/{run_id}/notes/phase-rca.md"
        description: "Phase narrative: decisions, concerns, directives, observations"
    gate:
      - file_exists: "docs/plans/*-rca-report.md"
      - file_exists: ".belt/runs/{run_id}/notes/phase-rca.md"
    validate: ./criteria/rca.md
    confirm: true
    max_retries: 3
```

**fix-plan phase** (既存 L34-48 を置き換え):

```yaml
  - id: fix-plan
    description: "Create fix plan from RCA report"
    invoke:
      skill: /writing-plans
    consumes:
      - rca_report
      - rca_notes
    produces:
      - name: fix_plan_doc
        path: "docs/plans/*-fix-plan.md"
        description: "Fix plan with RCA Fix Strategy → task mapping"
      - name: fix_plan_notes
        path: ".belt/runs/{run_id}/notes/phase-fix-plan.md"
        description: "Phase narrative"
    gate:
      - file_exists: "docs/plans/*-fix-plan.md"
      - file_exists: ".belt/runs/{run_id}/notes/phase-fix-plan.md"
    validate: ./criteria/fix-plan.md
    confirm: true
    max_retries: 3
```

**execute phase** (既存 L62-71 を置き換え):

```yaml
  - id: execute
    description: "TDD implementation following the fix plan"
    invoke:
      skill: /subagent-driven-development
    consumes:
      - rca_report
      - fix_plan_doc
      - rca_notes
      - fix_plan_notes
    produces:
      - name: execute_notes
        path: ".belt/runs/{run_id}/notes/phase-execute.md"
        description: "Phase narrative"
    gate:
      - file_exists: ".belt/runs/{run_id}/notes/phase-execute.md"
    validate: ./criteria/execute.md
    confirm: true
    max_retries: 3
```

**code-review phase** (既存 L73-85 を置き換え):

```yaml
  - id: code-review
    description: "Multi-perspective code review"
    invoke:
      skill: /code-review:code-review
      args:
        codex: "args.codex"
    consumes:
      - rca_report
      - fix_plan_doc
      - rca_notes
      - fix_plan_notes
      - execute_notes
    produces:
      - name: code_review_notes
        path: ".belt/runs/{run_id}/notes/phase-code-review.md"
        description: "Phase narrative"
    gate:
      - file_exists: ".belt/runs/{run_id}/notes/phase-code-review.md"
    validate: ./criteria/code-review.md
    regate: [execute]
    confirm: true
    max_retries: 3
```

**monkey-test phase** (既存 L87-105 を置き換え):

```yaml
  - id: monkey-test
    description: "Replay reproduction scenarios via agent-browser"
    when: "args.e2e"
    invoke:
      skill: /monkey-test:monkey-test
    consumes:
      - rca_report
      - rca_scenarios
      - fix_plan_doc
      - rca_notes
      - fix_plan_notes
      - execute_notes
      - code_review_notes
    produces:
      - name: monkey_test_report
        path: "docs/plans/*-monkey-test-report.md"
      - name: monkey_test_results
        path: "docs/plans/*-monkey-test-results.json"
      - name: monkey_test_notes
        path: ".belt/runs/{run_id}/notes/phase-monkey-test.md"
        description: "Phase narrative"
    gate:
      - file_exists: "docs/plans/*-monkey-test-report.md"
      - file_exists: ".belt/runs/{run_id}/notes/phase-monkey-test.md"
    validate: ./criteria/monkey-test.md
    confirm: true
    max_retries: 3
```

**dogfood phase** (既存 L107-125 を置き換え):

```yaml
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
      - rca_notes
      - fix_plan_notes
      - execute_notes
      - code_review_notes
      - monkey_test_notes
    produces:
      - name: dogfood_report
        path: "docs/plans/*-dogfood-report/report.md"
      - name: dogfood_notes
        path: ".belt/runs/{run_id}/notes/phase-dogfood.md"
        description: "Phase narrative"
    gate:
      - file_exists: "docs/plans/*-dogfood-report/report.md"
      - file_exists: ".belt/runs/{run_id}/notes/phase-dogfood.md"
    validate: ./criteria/dogfood.md
    confirm: true
    max_retries: 3
```

他の phase（fix-plan-review / integrate）は **変更なし**。

- [ ] **Step 4: Run tests to verify PASS**

Run: `cargo test -p belt-core --test bug_fix_refresh`
Expected: 全 test PASS（既存 + 新規 4 個）

- [ ] **Step 5: Run belt lint**

Run: `cargo run -p belt -- lint plugins/bug-fix/skills/bug-fix/pipeline.yml`
Expected: lint PASS、unprotected produces warning 0 件

- [ ] **Step 6: Apply fmt and run clippy**

Run: `cargo fmt -p belt-core`
Run: `cargo clippy -p belt-core -- -D warnings`
Expected: clippy PASS with zero warnings

- [ ] **Step 7: Commit**

```bash
git add crates/belt-core/tests/bug_fix_refresh.rs plugins/bug-fix/skills/bug-fix/pipeline.yml
git commit -m "$(cat <<'EOF'
feat(bug-fix): add narrative notes to 6 phases for context reset

Integrate phase-scoped narrative artifact into bug-fix pipeline:
- rca / fix-plan / execute / code-review / monkey-test (e2e) / dogfood (e2e)
- Each narrative phase produces .belt/runs/{run_id}/notes/phase-<id>.md
- file_exists gate enforces deterministic note submission
- Accumulating consume: each phase reads all prior narrative notes

Non-narrative phases (fix-plan-review / integrate) unchanged.

Refs: docs/specs/2026-04-15-pipeline-context-reset-design.md
EOF
)"
```

---

## Task 4: execute / code-review criteria narrative section（per-plugin 各 2 files）

**Files:**
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/execute.md`
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/code-review.md`
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/execute.md`
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/code-review.md`

plugin 移行で execute.md / code-review.md は per-plugin 化されている。同一の narrative 節を 4 file に追加する。`shared_criteria_parity.rs` が内容 drift を検出するため、**全 4 file で追加内容を完全に一致させること**。

- [ ] **Step 1: Append narrative section to both plugins' `criteria/execute.md`**

更新対象: `plugins/feature-dev/skills/feature-dev/criteria/execute.md` と `plugins/bug-fix/skills/bug-fix/criteria/execute.md` の両方に同一内容を追加。

既存 `EXECUTE-09` 項目の直後、`## Observation Collection` の直前に挿入:

```markdown
### EXECUTE-10: Narrative note captures phase decisions and directives
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Verify file exists at `.belt/runs/<run_id>/notes/phase-execute.md` (gate already enforces existence, re-confirm)
  2. Verify frontmatter contains both `phase: execute` and `run_id: <run_id>` fields
  3. Verify 4 required sections exist in order: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  4. Verify each section is either populated or retains its heading (empty sections may carry `(none)` placeholder but heading must be present)
  5. Verify Decisions / Directives are specific enough for a `/clear`-ed LLM to reconstruct the phase outcome (not vague generalities)
- **pass_condition**: Steps 1-4 all pass; Step 5 narrative is concrete (references task IDs / file paths / decisions made, not abstract statements)
- **fail_diagnosis_hint**: If heading missing, add empty heading. If frontmatter missing, copy `run_id` from `belt-agent step` / `belt-agent status` JSON output. If content is vague, rewrite to cite concrete artifacts (task IDs, file paths, decision triggers). See `plugins/belt-agents/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [.belt/runs/*/notes/phase-execute.md]
```

- [ ] **Step 2: Append narrative section to both plugins' `criteria/code-review.md`**

更新対象: `plugins/feature-dev/skills/feature-dev/criteria/code-review.md` と `plugins/bug-fix/skills/bug-fix/criteria/code-review.md` の両方に同一内容を追加。まず既存 file を Read して最後の `CODE-REVIEW-<X>` 番号を確認する。`<N>` は `<X> + 1`。

末尾の適切な位置（既存の最後の項目の直後、`## Observation Collection` の直前）に挿入:

```markdown
### CODE-REVIEW-<N>: Narrative note captures review findings and directives
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Verify file exists at `.belt/runs/<run_id>/notes/phase-code-review.md`
  2. Verify frontmatter contains `phase: code-review` and `run_id: <run_id>`
  3. Verify 4 required sections exist: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  4. Verify Decisions records which review findings were accepted / rejected and why
  5. Verify Directives flags carry-over concerns for downstream phases (e.g. regression tests to run in monkey-test)
- **pass_condition**: Steps 1-5 all pass; narrative records specific review outcomes not abstract "code reviewed"
- **fail_diagnosis_hint**: If Decisions lacks accept/reject rationale, re-read review findings and enumerate. If Directives empty, consider whether monkey-test / dogfood needs specific regression coverage. See `plugins/belt-agents/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [.belt/runs/*/notes/phase-code-review.md]
```

**Note**: `<N>` は既存 criteria の最後の番号 + 1。実装時に既存 file を確認して adjust すること。

- [ ] **Step 3: Commit**

```bash
git add plugins/feature-dev/skills/feature-dev/criteria/execute.md plugins/feature-dev/skills/feature-dev/criteria/code-review.md plugins/bug-fix/skills/bug-fix/criteria/execute.md plugins/bug-fix/skills/bug-fix/criteria/code-review.md
git commit -m "$(cat <<'EOF'
docs(criteria): add narrative note checks to execute/code-review criteria

Both feature-dev and bug-fix plugins now produce narrative notes at
execute and code-review phases. Add identical audit criteria to both
plugins' copies (per-plugin duplication, parity test enforced):
- file existence at .belt/runs/<run_id>/notes/phase-<id>.md
- required frontmatter (phase, run_id)
- 4 required sections (Decisions/Concerns/Directives/Observations)
- content is concrete enough for /clear-ed LLM to reconstruct outcome

Refs plugins/belt-agents/references/narrative-convention.md for schema.
EOF
)"
```

---

## Task 5: feature-dev skill criteria narrative section (4 files)

**Files:**
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/design.md`
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/plan.md`
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/monkey-test.md`
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/dogfood.md`

各 criteria file の末尾に、該当 phase の narrative section 1 節を追加。template は一貫させる。

**共通手順**: 各 Step の開始時に対象 file を Read し、既存 criterion ID の最大値 `<X>` を確認する。`<N>` は `<X> + 1`。例えば design.md は既存が `DESIGN-01..06` なので `<N> = 07`（この例は Step 1 で記述済）。

- [ ] **Step 1: Append to `plugins/feature-dev/skills/feature-dev/criteria/design.md`**

file 末尾に追加:

```markdown
- **DESIGN-07**: Narrative note at `.belt/runs/<run_id>/notes/phase-design.md` exists,
  contains frontmatter (`phase: design`, `run_id: <run_id>`), and all 4 required
  sections (`## Decisions`, `## Concerns`, `## Directives`, `## Observations`).
  Decisions section records chosen approach and rejected alternatives with rationale.
  Directives section records constraints for plan / execute phases. Empty sections
  may carry `(none)` placeholder but heading must be preserved.
  See `plugins/belt-agents/references/narrative-convention.md`.
```

- [ ] **Step 2: Append to `plugins/feature-dev/skills/feature-dev/criteria/plan.md`**

既存 file の最後の criterion 直後に同形式で追加（`PLAN-<N>` の番号は実装時に既存最大値 + 1 に adjust）:

```markdown
- **PLAN-<N>**: Narrative note at `.belt/runs/<run_id>/notes/phase-plan.md` exists,
  contains frontmatter (`phase: plan`, `run_id: <run_id>`), and all 4 required
  sections. Decisions records task decomposition rationale and granularity choices.
  Directives records constraints for execute phase (e.g. commit granularity rules,
  test-first enforcement). See `plugins/belt-agents/references/narrative-convention.md`.
```

- [ ] **Step 3: Append to `plugins/feature-dev/skills/feature-dev/criteria/monkey-test.md`**

同様に末尾に追加:

```markdown
- **MONKEY-TEST-<N>**: Narrative note at `.belt/runs/<run_id>/notes/phase-monkey-test.md`
  exists with required frontmatter and 4 sections. Observations records scenario
  replay results (pass/fail per scenario). Concerns flags scenarios that revealed
  unexpected behavior worth dogfood follow-up. Directives carries forward regression
  hotspots. See `plugins/belt-agents/references/narrative-convention.md`.
```

- [ ] **Step 4: Append to `plugins/feature-dev/skills/feature-dev/criteria/dogfood.md`**

```markdown
- **DOGFOOD-<N>**: Narrative note at `.belt/runs/<run_id>/notes/phase-dogfood.md`
  exists with required frontmatter and 4 sections. Observations records exploratory
  findings beyond scripted scenarios. Concerns flags unresolved risks for integrate
  phase. Directives carries forward any must-verify items discovered during
  exploration. See `plugins/belt-agents/references/narrative-convention.md`.
```

- [ ] **Step 5: Verify criteria files parse as markdown**

Run: `ls plugins/feature-dev/skills/feature-dev/criteria/*.md | xargs -I{} cat {} > /dev/null`
Expected: エラーなし（file が readable）

- [ ] **Step 6: Commit**

```bash
git add plugins/feature-dev/skills/feature-dev/criteria/design.md plugins/feature-dev/skills/feature-dev/criteria/plan.md plugins/feature-dev/skills/feature-dev/criteria/monkey-test.md plugins/feature-dev/skills/feature-dev/criteria/dogfood.md
git commit -m "$(cat <<'EOF'
docs(feature-dev): add narrative note criteria to 4 phase done-criteria

design, plan, monkey-test, dogfood done-criteria now verify narrative note
existence, frontmatter, and 4 required sections per narrative-convention.md.
Per-phase guidance clarifies which decisions/concerns/directives each phase
should record.
EOF
)"
```

---

## Task 6: bug-fix skill criteria narrative section (4 files)

**Files:**
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/rca.md`
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/fix-plan.md`
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/monkey-test.md`
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/dogfood.md`

**共通手順**: 各 Step の開始時に対象 file を Read し、既存 criterion ID の最大値を確認して `<N>` を決める（= 既存最大 + 1）。

- [ ] **Step 1: Append to `plugins/bug-fix/skills/bug-fix/criteria/rca.md`**

```markdown
- **RCA-<N>**: Narrative note at `.belt/runs/<run_id>/notes/phase-rca.md` exists,
  contains frontmatter (`phase: rca`, `run_id: <run_id>`), and all 4 required
  sections. Decisions records chosen root cause hypothesis and rejected candidates.
  Concerns flags ambiguity in the reproduction window / environment coupling.
  Directives records Fix Strategy constraints for fix-plan phase.
  See `plugins/belt-agents/references/narrative-convention.md`.
```

- [ ] **Step 2: Append to `plugins/bug-fix/skills/bug-fix/criteria/fix-plan.md`**

```markdown
- **FIX-PLAN-<N>**: Narrative note at `.belt/runs/<run_id>/notes/phase-fix-plan.md`
  exists with required frontmatter and 4 sections. Decisions records task
  decomposition from RCA Fix Strategy. Directives records test-first requirements
  and regression scope for execute phase.
  See `plugins/belt-agents/references/narrative-convention.md`.
```

- [ ] **Step 3: Append to `plugins/bug-fix/skills/bug-fix/criteria/monkey-test.md`**

```markdown
- **MONKEY-TEST-<N>**: Narrative note at `.belt/runs/<run_id>/notes/phase-monkey-test.md`
  exists with required frontmatter and 4 sections. Observations records reproduction
  scenario results (was the RCA reproduction test now PASS?). Directives carries
  forward dogfood exploration targets.
  See `plugins/belt-agents/references/narrative-convention.md`.
```

- [ ] **Step 4: Append to `plugins/bug-fix/skills/bug-fix/criteria/dogfood.md`**

```markdown
- **DOGFOOD-<N>**: Narrative note at `.belt/runs/<run_id>/notes/phase-dogfood.md`
  exists with required frontmatter and 4 sections. Observations records
  Symmetry-Pair probe results and Impact Scope coverage. Concerns flags Root Cause
  mechanism re-emergence signals. Directives carries forward regression guards for
  integrate.
  See `plugins/belt-agents/references/narrative-convention.md`.
```

- [ ] **Step 5: Commit**

```bash
git add plugins/bug-fix/skills/bug-fix/criteria/rca.md plugins/bug-fix/skills/bug-fix/criteria/fix-plan.md plugins/bug-fix/skills/bug-fix/criteria/monkey-test.md plugins/bug-fix/skills/bug-fix/criteria/dogfood.md
git commit -m "$(cat <<'EOF'
docs(bug-fix): add narrative note criteria to 4 phase done-criteria

rca, fix-plan, monkey-test, dogfood done-criteria now verify narrative note
existence, frontmatter, and 4 required sections per narrative-convention.md.
Per-phase guidance aligns with bug-fix domain (RCA hypotheses, reproduction
outcomes, Symmetry-Pair probes).
EOF
)"
```

---

## Task 7: feature-dev SKILL.md update

**Files:**
- Modify: `plugins/feature-dev/skills/feature-dev/SKILL.md`

- [ ] **Step 1: Add Narrative Notes section**

既存 `## Red Flags` セクションの直前に挿入:

```markdown
## Narrative Notes

以下 6 phase は `/clear` 後の context 復元のため narrative note を produce する (`.belt/runs/{run_id}/notes/phase-<id>.md`):

- **design** / **plan** / **execute** / **code-review**
- **monkey-test** (`--e2e`) / **dogfood** (`--e2e`)

各 note は 4 section (`## Decisions` / `## Concerns` / `## Directives` / `## Observations`) と minimal frontmatter (`phase`, `run_id`) を含む。

規約詳細: [`plugins/belt-agents/references/narrative-convention.md`](plugins/belt-agents/references/narrative-convention.md)

`/clear` 自体は user 判断（Claude Code runtime 制約で自動化不可）。重い phase 完了直後（例: design / execute / code-review 後）に context が膨れた場合の選択肢として narrative を活用できる。
```

- [ ] **Step 2: Add Red Flag**

既存 `## Red Flags` リストに以下 1 行を追加（末尾が自然）:

```markdown
- **Never leave narrative note 4 sections blank**: gate は file_exists のみで空 section も通過するが、下流 consume で context 復元不能になる。最低限 `(none)` placeholder を置き、heading は必ず保持。
```

- [ ] **Step 3: Update References section**

既存 `## References` リスト末尾に追加:

```markdown
- `plugins/belt-agents/references/narrative-convention.md` — narrative note schema (shared with bug-fix)
```

- [ ] **Step 4: Verify SKILL.md still parses**

Run: `cat plugins/feature-dev/skills/feature-dev/SKILL.md | head -5`
Expected: frontmatter（`---\nname: feature-dev...`）が正しく始まっていること

- [ ] **Step 5: Commit**

```bash
git add plugins/feature-dev/skills/feature-dev/SKILL.md
git commit -m "$(cat <<'EOF'
docs(feature-dev): document narrative notes and /clear guidance in SKILL.md

Add Narrative Notes section that points at narrative-convention.md SSOT
and clarifies /clear is a user decision (not automated). Add red flag
against empty-section notes. Link narrative-convention.md in References.
EOF
)"
```

---

## Task 8: bug-fix SKILL.md update

**Files:**
- Modify: `plugins/bug-fix/skills/bug-fix/SKILL.md`

- [ ] **Step 1: Add Narrative Notes section**

既存 `## Red Flags` セクションの直前に挿入:

```markdown
## Narrative Notes

以下 6 phase は `/clear` 後の context 復元のため narrative note を produce する (`.belt/runs/{run_id}/notes/phase-<id>.md`):

- **rca** / **fix-plan** / **execute** / **code-review**
- **monkey-test** (`--e2e`) / **dogfood** (`--e2e`)

各 note は 4 section (`## Decisions` / `## Concerns` / `## Directives` / `## Observations`) と minimal frontmatter (`phase`, `run_id`) を含む。

規約詳細: [`plugins/belt-agents/references/narrative-convention.md`](plugins/belt-agents/references/narrative-convention.md)

`/clear` 自体は user 判断（Claude Code runtime 制約で自動化不可）。重い phase 完了直後（例: rca / execute 後）に context が膨れた場合の選択肢として narrative を活用できる。
```

- [ ] **Step 2: Add Red Flag**

既存 `## Red Flags` リストに以下 1 行を追加:

```markdown
- **Never leave narrative note 4 sections blank**: gate は file_exists のみで空 section も通過するが、下流 consume で context 復元不能になる。最低限 `(none)` placeholder を置き、heading は必ず保持。
```

- [ ] **Step 3: Update References section**

既存 References リスト末尾に追加:

```markdown
- `plugins/belt-agents/references/narrative-convention.md` — narrative note schema (shared with feature-dev)
```

- [ ] **Step 4: Commit**

```bash
git add plugins/bug-fix/skills/bug-fix/SKILL.md
git commit -m "$(cat <<'EOF'
docs(bug-fix): document narrative notes and /clear guidance in SKILL.md

Add Narrative Notes section that points at narrative-convention.md SSOT
and clarifies /clear is a user decision. Add red flag against empty-section
notes. Link narrative-convention.md in References.
EOF
)"
```

---

## Task 9: Final verification

**Files:** なし（verification のみ）

- [ ] **Step 1: Run full belt-core tests**

Run: `cargo test -p belt-core`
Expected: 全 test PASS（既存 + 新規 7 テスト: feature-dev 3 + bug-fix 3 + non-narrative-check 1 が bug-fix 側にある）

- [ ] **Step 2: Run workspace clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: warning 0 件 (CI 基準)

- [ ] **Step 3: Run fmt check**

Run: `cargo fmt --check`
Expected: diff 0 件

- [ ] **Step 4: Lint both pipelines**

Run: `cargo run -p belt -- lint plugins/feature-dev/skills/feature-dev/pipeline.yml`
Run: `cargo run -p belt -- lint plugins/bug-fix/skills/bug-fix/pipeline.yml`
Expected: 両 pipeline で lint PASS、unprotected produces warning 0 件

- [ ] **Step 5: Verify git log shape**

Run: `git log --oneline -10`
Expected: 本計画の 8 commits が上位に並ぶ (narrative-convention / feature-dev pipeline / bug-fix pipeline / shared criteria / feature-dev criteria / bug-fix criteria / feature-dev SKILL / bug-fix SKILL)

- [ ] **Step 6: Adversarial probe — phase skip sanity**

belt-agent を実際に走らせて narrative gate が期待通り機能することを確認（手動確認なのでスキップ可、ただし推奨）:

```bash
# e2e=false のケース: monkey-test / dogfood がスキップされ narrative note の file_exists 評価が発動しないこと
belt-agent init plugins/feature-dev/skills/feature-dev/pipeline.yml --arg e2e=false
belt-agent next  # monkey-test / dogfood が当然 skip されることを JSON output で確認
```

Expected: `skipped_phases` に monkey-test / dogfood が含まれ、当該 narrative gate は評価対象に入らない

- [ ] **Step 7: Summary**

タスク完了。19 files 変更完了（1 new + 18 modified）、8 commits 作成済み。spec 記載の Open Questions は実装時の判断で以下のように決着:
- (a) `narrative-convention.md` は `plugins/belt-agents/references/` に配置（G1: cross-cutting reference plugin）
- (b) per-plugin 化済みの execute.md / code-review.md に同一 narrative 節を追加（Task 4）。`shared_criteria_parity.rs` で drift 検出
- (c) criteria audit は section 存在 + 内容の concreteness（placeholder / 抽象化の拒否）まで検証（Task 4 参照）

---

## Summary: Commit Sequence

1. `docs(skills): add narrative-convention SSOT for phase notes` (Task 1)
2. `feat(feature-dev): add narrative notes to 6 phases for context reset` (Task 2)
3. `feat(bug-fix): add narrative notes to 6 phases for context reset` (Task 3)
4. `docs(criteria): add narrative note checks to execute/code-review criteria` (Task 4)
5. `docs(feature-dev): add narrative note criteria to 4 phase done-criteria` (Task 5)
6. `docs(bug-fix): add narrative note criteria to 4 phase done-criteria` (Task 6)
7. `docs(feature-dev): document narrative notes and /clear guidance in SKILL.md` (Task 7)
8. `docs(bug-fix): document narrative notes and /clear guidance in SKILL.md` (Task 8)

Final verification (Task 9) produces no commits.

**Total: 8 commits, 19 files (1 new + 18 modified), ~260-330 lines changed.**
