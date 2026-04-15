# Review Skills Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 4 review skills (`/code-review`, `/spec-review`, `/test-review`, `/implementation-review`) を同一構造に刷新。並列 multi-agent dispatch + N-way voting を廃止し、単一 reviewer subagent に集約。spec-review に grill-me dialogue を注入。

**Architecture:**
- pipeline.yml: `invoke.agents` を 1 要素に縮小、args は `codex` のみ、review → fix の 2 phase 構成を維持
- 既存 18 agent files を 4 つの consolidated reviewer files に merge（旧内容を章立てで転記）
- SKILL.md から N-way voting 関連を全削除、spec-review のみ grill-me dialogue protocol を追記

**Tech Stack:**
- Rust (belt-core integration test)
- YAML (pipeline.yml)
- Markdown (SKILL.md, agent definition files)

**Spec reference:** `docs/specs/2026-04-15-review-skills-refresh-design.md` (commit `e9169ff`)

---

## File Structure

### Create
- `crates/belt-core/tests/review_skills_refresh.rs` — Integration test locking pipeline.yml shape
- `.claude/agents/code-reviewer.md` — Consolidated 7-observation reviewer
- `.claude/agents/spec-reviewer.md` — Consolidated 5-observation reviewer
- `.claude/agents/test-reviewer.md` — Consolidated 3-observation reviewer + requirement map
- `.claude/agents/implementation-reviewer.md` — Consolidated 4-observation reviewer

### Modify
- `examples/skills/code-review/pipeline.yml`
- `examples/skills/code-review/SKILL.md`
- `examples/skills/spec-review/pipeline.yml`
- `examples/skills/spec-review/SKILL.md`
- `examples/skills/test-review/pipeline.yml`
- `examples/skills/test-review/SKILL.md`
- `examples/skills/implementation-review/pipeline.yml`
- `examples/skills/implementation-review/SKILL.md`
- `examples/skills/feature-dev/SKILL.md` (remove single Red Flag line)

### Delete
- `.claude/agents/code-review-quality.md`
- `.claude/agents/code-review-security.md`
- `.claude/agents/code-review-performance.md`
- `.claude/agents/code-review-test.md`
- `.claude/agents/code-review-ai-antipattern.md`
- `.claude/agents/code-review-impact.md`
- `.claude/agents/spec-review-requirements.md`
- `.claude/agents/spec-review-design-judgment.md`
- `.claude/agents/spec-review-feasibility.md`
- `.claude/agents/spec-review-consistency.md`
- `.claude/agents/spec-review-ui-design.md`
- `.claude/agents/test-review-coverage.md`
- `.claude/agents/test-review-quality.md`
- `.claude/agents/test-review-design-alignment.md`
- `.claude/agents/implementation-review-clarity.md`
- `.claude/agents/implementation-review-feasibility.md`
- `.claude/agents/implementation-review-consistency.md`
- `.claude/agents/implementation-review-ui-spec.md`

**No changes required to:**
- `examples/skills/feature-dev/pipeline.yml` — already passes only `codex` to `/code-review`, no `iterations`/`swarm`/`ui` references
- `crates/belt-core/tests/feature_dev_refresh.rs` — does not assert review skill args shape

---

## Verification Commands

Run these after the plan completes (Task 17):

```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt

# Rust
cargo fmt --package belt-core
cargo clippy --package belt-core -- -D warnings
cargo test -p belt-core

# belt lint on all 4 refreshed pipelines
cargo run --bin belt -- lint examples/skills/code-review/pipeline.yml
cargo run --bin belt -- lint examples/skills/spec-review/pipeline.yml
cargo run --bin belt -- lint examples/skills/test-review/pipeline.yml
cargo run --bin belt -- lint examples/skills/implementation-review/pipeline.yml
```

Expected: all commands exit 0, all tests pass.

---

## Task List

- [ ] Task 1: Add failing integration tests for refreshed review skills shape
- [ ] Task 2: Create consolidated `code-reviewer.md` (7 observations)
- [ ] Task 3: Create consolidated `spec-reviewer.md` (5 observations)
- [ ] Task 4: Create consolidated `test-reviewer.md` (3 observations + requirement map)
- [ ] Task 5: Create consolidated `implementation-reviewer.md` (4 observations)
- [ ] Task 6: Rewrite `code-review/pipeline.yml`
- [ ] Task 7: Rewrite `spec-review/pipeline.yml`
- [ ] Task 8: Rewrite `test-review/pipeline.yml`
- [ ] Task 9: Rewrite `implementation-review/pipeline.yml`
- [ ] Task 10: Rewrite `code-review/SKILL.md`
- [ ] Task 11: Rewrite `spec-review/SKILL.md` (grill-me dialogue)
- [ ] Task 12: Rewrite `test-review/SKILL.md`
- [ ] Task 13: Rewrite `implementation-review/SKILL.md`
- [ ] Task 14: Delete 18 legacy agent files
- [ ] Task 15: Remove obsolete Red Flag from `feature-dev/SKILL.md`
- [ ] Task 16: Run all verification commands
- [ ] Task 17: Final cleanup + summary commit

---

### Task 1: Add failing integration tests for refreshed review skills shape

**Files:**
- Create: `crates/belt-core/tests/review_skills_refresh.rs`

- [ ] **Step 1: Write the failing integration test**

Create `crates/belt-core/tests/review_skills_refresh.rs` with the following content:

```rust
//! Integration tests locking the refreshed review-skill pipelines
//! (/code-review, /spec-review, /test-review, /implementation-review).
//!
//! Shape contract:
//! - args = { codex: bool } only (no iterations, swarm, ui)
//! - phases = [review, fix], review.invoke.agents = [<skill>-reviewer]
//! - consolidated agent files exist; legacy per-observation files are removed.

use std::path::{Path, PathBuf};

use belt_core::{
    error::BeltError,
    model::ArgType,
    parser::parse_pipeline,
};

fn repo_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path
}

fn pipeline_path(skill: &str) -> PathBuf {
    repo_root().join(format!("examples/skills/{skill}/pipeline.yml"))
}

const REVIEW_SKILLS: &[(&str, &str, &str)] = &[
    ("code-review", "code-reviewer", "code-review"),
    ("spec-review", "spec-reviewer", "spec-review"),
    ("test-review", "test-reviewer", "test-review"),
    (
        "implementation-review",
        "implementation-reviewer",
        "implementation-review",
    ),
];

#[test]
fn review_skills_args_are_codex_only() -> Result<(), BeltError> {
    for (skill, _agent, _label) in REVIEW_SKILLS {
        let pipeline = parse_pipeline(&pipeline_path(skill))?;
        let mut names: Vec<&str> = pipeline.args.keys().map(String::as_str).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            vec!["codex"],
            "{skill} args must be exactly {{codex}}"
        );
        let codex = pipeline
            .args
            .get("codex")
            .ok_or_else(|| BeltError::InvalidPipeline {
                message: format!("{skill} codex arg missing"),
            })?;
        assert!(
            matches!(codex.arg_type, ArgType::Bool),
            "{skill} codex arg must be bool"
        );
        assert_eq!(
            codex.default.as_ref().and_then(serde_json::Value::as_bool),
            Some(false),
            "{skill} codex default must be false"
        );
    }
    Ok(())
}

#[test]
fn review_skills_have_two_phases_review_then_fix() -> Result<(), BeltError> {
    for (skill, _agent, _label) in REVIEW_SKILLS {
        let pipeline = parse_pipeline(&pipeline_path(skill))?;
        let ids: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["review", "fix"],
            "{skill} phases must be [review, fix]"
        );
    }
    Ok(())
}

#[test]
fn review_skills_invoke_single_consolidated_agent() -> Result<(), BeltError> {
    // belt-core's typed Invoker model may not expose `agents` directly; assert
    // by parsing the raw YAML.
    for (skill, agent, _label) in REVIEW_SKILLS {
        let yaml = std::fs::read_to_string(pipeline_path(skill))?;
        let doc: serde_json::Value =
            serde_saphyr::from_str(&yaml).map_err(|e| BeltError::YamlParse {
                message: e.to_string(),
                src: Some(yaml.clone()),
            })?;
        let review_phase = doc
            .get("phases")
            .and_then(serde_json::Value::as_array)
            .and_then(|phases| {
                phases
                    .iter()
                    .find(|p| p.get("id").and_then(serde_json::Value::as_str) == Some("review"))
            })
            .ok_or_else(|| BeltError::InvalidPipeline {
                message: format!("{skill} review phase missing"),
            })?;
        let agents = review_phase
            .get("invoke")
            .and_then(|i| i.get("agents"))
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| BeltError::InvalidPipeline {
                message: format!("{skill} review.invoke.agents missing"),
            })?;
        let agent_list: Vec<&str> = agents
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect();
        assert_eq!(
            agent_list,
            vec![*agent],
            "{skill} review.invoke.agents must be [{agent}]"
        );
    }
    Ok(())
}

#[test]
fn review_skills_consolidated_agent_files_exist() {
    let agents_dir = repo_root().join(".claude/agents");
    for (_skill, agent, _label) in REVIEW_SKILLS {
        let path = agents_dir.join(format!("{agent}.md"));
        assert!(
            path.exists(),
            "consolidated agent file must exist: {}",
            path.display()
        );
    }
}

#[test]
fn legacy_review_agent_files_are_removed() {
    let agents_dir = repo_root().join(".claude/agents");
    const LEGACY: &[&str] = &[
        "code-review-quality",
        "code-review-security",
        "code-review-performance",
        "code-review-test",
        "code-review-ai-antipattern",
        "code-review-impact",
        "spec-review-requirements",
        "spec-review-design-judgment",
        "spec-review-feasibility",
        "spec-review-consistency",
        "spec-review-ui-design",
        "test-review-coverage",
        "test-review-quality",
        "test-review-design-alignment",
        "implementation-review-clarity",
        "implementation-review-feasibility",
        "implementation-review-consistency",
        "implementation-review-ui-spec",
    ];
    for name in LEGACY {
        let path = agents_dir.join(format!("{name}.md"));
        assert!(
            !path.exists(),
            "legacy agent file must be deleted: {}",
            path.display()
        );
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p belt-core --test review_skills_refresh
```

Expected: All 5 tests FAIL. Specifically:
- `review_skills_args_are_codex_only` FAILS because current pipelines still declare `iterations`/`swarm`/`ui`
- `review_skills_have_two_phases_review_then_fix` may PASS (phases unchanged) — acceptable
- `review_skills_invoke_single_consolidated_agent` FAILS because `agents` is still a multi-element array
- `review_skills_consolidated_agent_files_exist` FAILS because `code-reviewer.md` etc. not yet created
- `legacy_review_agent_files_are_removed` FAILS because legacy files still present

- [ ] **Step 3: Commit the failing tests**

```bash
git add crates/belt-core/tests/review_skills_refresh.rs
git -c commit.gpgsign=false commit -m "test(belt-core): add failing tests for review skills refresh shape"
```

---

### Task 2: Create consolidated `code-reviewer.md` (7 observations)

**Files:**
- Create: `.claude/agents/code-reviewer.md`
- Reference (read-only, to source content for each observation): `.claude/agents/code-review-quality.md`, `code-review-security.md`, `code-review-performance.md`, `code-review-test.md`, `code-review-ai-antipattern.md`, `code-review-impact.md`

- [ ] **Step 1: Read all 6 legacy code-review agents**

```bash
# The subagent reads these to preserve content
cat .claude/agents/code-review-quality.md
cat .claude/agents/code-review-security.md
cat .claude/agents/code-review-performance.md
cat .claude/agents/code-review-test.md
cat .claude/agents/code-review-ai-antipattern.md
cat .claude/agents/code-review-impact.md
```

- [ ] **Step 2: Write the consolidated agent file**

Create `.claude/agents/code-reviewer.md` using this template. For each observation section, transcribe the **Scope / Review Checklist / Investigation Method / Boundary / Output Format** content from the matching legacy agent file. Keep the content in its original language (mix of English/Japanese is fine — match the legacy files). Remove per-file `Boundary` cross-references that no longer apply (since all observations are now in one agent).

```markdown
---
name: code-reviewer
description: Multi-perspective code review covering quality, security, performance, testing, AI-antipattern, impact, and simplification. Reviews only the diff scope.
memory: project
effort: max
---

You are a consolidated code reviewer. In a single pass over the diff, produce findings across seven observations.

## Scope

Review ONLY the files and lines provided in the diff. Do not comment on unchanged code.

If the parent orchestrator supplied a design document path (e.g. `*-design.md`), read its Impact Analysis section before starting the Impact observation.

## Filtering (applies to all observations)

- 確信度 80% 未満の問題は報告しない。推測ベースの指摘は除外する
- 同一パターンの問題が複数箇所にある場合、1 件の finding にまとめ、件数と代表箇所を記載する
- スタイル好みや主観的な「こう書いた方がきれい」は報告しない。プロジェクト規約違反のみ報告する
- 観点間で同じ問題が見つかったら、最も本質的な観点 1 箇所のみに置く（self-dedup）

## Observation 1: Quality

<TRANSCRIBE FROM code-review-quality.md: Review Checklist, any specific rules. Omit the `Boundary` section.>

## Observation 2: Security

<TRANSCRIBE FROM code-review-security.md: Review Checklist, any specific rules. Omit the `Boundary` section.>

## Observation 3: Performance

<TRANSCRIBE FROM code-review-performance.md: Review Checklist, any specific rules. Omit the `Boundary` section.>

## Observation 4: Test

<TRANSCRIBE FROM code-review-test.md: Review Checklist, any specific rules. Omit the `Boundary` section.>

## Observation 5: AI-antipattern

<TRANSCRIBE FROM code-review-ai-antipattern.md: Review Checklist, any specific rules. Omit the `Boundary` section.>

## Observation 6: Impact

<TRANSCRIBE FROM code-review-impact.md: Review Checklist, any specific rules, Must-Verify Checklist handling. Omit the `Boundary` section.>

## Observation 7: Simplification

Review the diff for reuse opportunities, unnecessary complexity, and efficiency issues. This observation subsumes the `/simplify` skill's core checks:

- **Reuse** — 既存の関数・ユーティリティで置き換え可能な自作ロジックがないか
- **Quality** — 不必要な複雑さ、過剰な抽象、dead code
- **Efficiency** — 明らかに非効率な計算・重複処理・不要なオブジェクト生成

同一パターンの問題を他観点 (Quality / Performance) で既に報告済みなら、この観点では再度報告しない。

## Output Format

Write the aggregated findings to `.belt/runs/{run_id}/review/findings.json`:

```json
{
  "findings": [
    {
      "id": "<uuid>",
      "observation": "quality|security|performance|test|ai-antipattern|impact|simplification|codex",
      "severity": "high|medium|low",
      "file": "<path relative to repo root>",
      "line": <integer or null>,
      "description": "...",
      "suggestion": "...",
      "source": "agent|codex"
    }
  ]
}
```

- `observation` must be one of the 7 names above (or `codex` for Codex adversarial source, if invoked).
- Emit at most 20 findings total; if more exist, keep the highest-severity ones and note the truncation in a final `low` severity finding of observation `quality`.
- Do not emit empty findings arrays without at least a sentinel entry if nothing was found — omit the file entirely if and only if the run_id directory does not exist yet; otherwise write `{"findings": []}`.

## Guardrails

- Do not modify any source files. Read-only.
- Do not invoke further subagents.
- Stay within the diff scope. Do not comment on unchanged files.
```

Transcribe the legacy content into the placeholder blocks. Do not omit content.

- [ ] **Step 3: Run the Task 1 tests to confirm `code-reviewer.md` existence assertion passes**

```bash
cargo test -p belt-core --test review_skills_refresh review_skills_consolidated_agent_files_exist
```

Expected: still FAILS (spec-reviewer/test-reviewer/implementation-reviewer not yet present) but the first assertion for `code-reviewer.md` now passes internally. Continue.

- [ ] **Step 4: Commit**

```bash
git add .claude/agents/code-reviewer.md
git -c commit.gpgsign=false commit -m "feat(agents): add consolidated code-reviewer with 7 observations"
```

---

### Task 3: Create consolidated `spec-reviewer.md` (5 observations)

**Files:**
- Create: `.claude/agents/spec-reviewer.md`
- Reference (read-only): `.claude/agents/spec-review-requirements.md`, `spec-review-design-judgment.md`, `spec-review-feasibility.md`, `spec-review-consistency.md`, `spec-review-ui-design.md`

- [ ] **Step 1: Read all 5 legacy spec-review agents**

```bash
cat .claude/agents/spec-review-requirements.md
cat .claude/agents/spec-review-design-judgment.md
cat .claude/agents/spec-review-feasibility.md
cat .claude/agents/spec-review-consistency.md
cat .claude/agents/spec-review-ui-design.md
```

- [ ] **Step 2: Write the consolidated agent file**

Create `.claude/agents/spec-reviewer.md` using this template. Transcribe content from legacy files into each observation section as described.

```markdown
---
name: spec-reviewer
description: Multi-perspective spec review covering requirements, design judgment, feasibility, consistency, and UI design. Produces findings for grill-me dialogue and selection triage.
memory: project
effort: max
---

You are a consolidated spec reviewer. In a single pass, produce findings across five observations. UI observation is always included; if the spec has no UI-related content, emit zero UI findings.

## Scope

Review the target spec document. Use Grep and Read to investigate the codebase for implicit business rules, existing patterns, and constraints referenced by the spec.

## Filtering (applies to all observations)

- 確信度 80% 未満の問題は報告しない
- 同一問題は 1 件にまとめる
- 観点間で重複する指摘は最も本質的な観点 1 箇所のみに置く（self-dedup）

## Observation 1: Requirements

<TRANSCRIBE FROM spec-review-requirements.md: Review Checklist, Investigation Method. Omit `Boundary`.>

## Observation 2: Design judgment

<TRANSCRIBE FROM spec-review-design-judgment.md: Review Checklist, Investigation Method, any rules about alternatives and edge cases. Omit `Boundary`.>

## Observation 3: Feasibility

<TRANSCRIBE FROM spec-review-feasibility.md: Review Checklist, Investigation Method. Omit `Boundary`.>

## Observation 4: Consistency

<TRANSCRIBE FROM spec-review-consistency.md: Review Checklist, Investigation Method, impact-scope rules. Omit `Boundary`.>

## Observation 5: UI design

<TRANSCRIBE FROM spec-review-ui-design.md: Review Checklist, Investigation Method.> If the spec has no UI content, emit zero findings for this observation — do not fabricate issues.

## Output Format

Write findings to `.belt/runs/{run_id}/review/findings.json`:

```json
{
  "findings": [
    {
      "id": "<uuid>",
      "observation": "requirements|design-judgment|feasibility|consistency|ui-design|codex",
      "severity": "high|medium|low",
      "section": "<heading path, e.g. '## Background / ### Problem'>",
      "description": "...",
      "suggestion": "...",
      "source": "agent|codex"
    }
  ]
}
```

- `section` uses heading path instead of `file`/`line` (spec review is section-based).
- Emit at most 20 findings total; truncation notice same rule as code-reviewer.

## Guardrails

- Do not modify the spec. Read-only.
- Do not invoke further subagents.
```

- [ ] **Step 3: Commit**

```bash
git add .claude/agents/spec-reviewer.md
git -c commit.gpgsign=false commit -m "feat(agents): add consolidated spec-reviewer with 5 observations"
```

---

### Task 4: Create consolidated `test-reviewer.md` (3 observations + requirement map)

**Files:**
- Create: `.claude/agents/test-reviewer.md`
- Reference (read-only): `.claude/agents/test-review-coverage.md`, `test-review-quality.md`, `test-review-design-alignment.md`

- [ ] **Step 1: Read all 3 legacy test-review agents**

```bash
cat .claude/agents/test-review-coverage.md
cat .claude/agents/test-review-quality.md
cat .claude/agents/test-review-design-alignment.md
```

- [ ] **Step 2: Write the consolidated agent file**

Create `.claude/agents/test-reviewer.md`:

```markdown
---
name: test-reviewer
description: Multi-perspective test review covering coverage, quality, and design-alignment. Produces findings and a requirement map.
memory: project
effort: max
---

You are a consolidated test reviewer. In a single pass, produce findings across three observations plus an informational requirement map.

## Scope

Review the changed test files (diff scope). For the design-alignment observation, resolve the design spec path:

1. Check the output directory of the current run (if provided by the orchestrator) for `*-design.md`.
2. Else check `docs/plans/*-design.md` whose filename date prefix matches the plan date provided by the orchestrator.
3. If no design spec is found, proceed with reduced coverage and include a `low` severity finding under `design-alignment` noting the missing spec.

## Filtering (applies to all observations)

- 確信度 80% 未満の問題は報告しない
- 観点間で重複する指摘は最も本質的な観点 1 箇所のみに置く（self-dedup）

## Observation 1: Coverage

<TRANSCRIBE FROM test-review-coverage.md: Review Checklist, Investigation Method. Omit `Boundary`.>

## Observation 2: Quality

<TRANSCRIBE FROM test-review-quality.md: Review Checklist, Investigation Method, guidance on flaky tests and naming. Omit `Boundary`.>

## Observation 3: Design-alignment

<TRANSCRIBE FROM test-review-design-alignment.md: Review Checklist, Investigation Method, requirement-map generation rules. Omit `Boundary`.>

## Requirement Map

If a design spec was resolved, emit a `requirement_map` array alongside `findings` in the output file. Columns: number, requirement, source (section in design spec), test (file:line or `—`), gap (description or `—`). If no design spec, omit the `requirement_map` key (do not emit an empty array).

## Output Format

Write to `.belt/runs/{run_id}/review/findings.json`:

```json
{
  "findings": [
    {
      "id": "<uuid>",
      "observation": "coverage|quality|design-alignment|codex",
      "severity": "high|medium|low",
      "file": "<path>",
      "line": <integer or null>,
      "description": "...",
      "suggestion": "...",
      "source": "agent|codex"
    }
  ],
  "requirement_map": [
    {
      "number": 1,
      "requirement": "<from design spec>",
      "source": "<section heading>",
      "test": "<file:line or ->",
      "gap": "<description or ->"
    }
  ]
}
```

## Guardrails

- Do not modify test files. Read-only.
- Do not invoke further subagents.
```

- [ ] **Step 3: Commit**

```bash
git add .claude/agents/test-reviewer.md
git -c commit.gpgsign=false commit -m "feat(agents): add consolidated test-reviewer with 3 observations + requirement map"
```

---

### Task 5: Create consolidated `implementation-reviewer.md` (4 observations)

**Files:**
- Create: `.claude/agents/implementation-reviewer.md`
- Reference (read-only): `.claude/agents/implementation-review-clarity.md`, `implementation-review-feasibility.md`, `implementation-review-consistency.md`, `implementation-review-ui-spec.md`

- [ ] **Step 1: Read all 4 legacy implementation-review agents**

```bash
cat .claude/agents/implementation-review-clarity.md
cat .claude/agents/implementation-review-feasibility.md
cat .claude/agents/implementation-review-consistency.md
cat .claude/agents/implementation-review-ui-spec.md
```

- [ ] **Step 2: Write the consolidated agent file**

Create `.claude/agents/implementation-reviewer.md`:

```markdown
---
name: implementation-reviewer
description: Multi-perspective implementation-plan review covering clarity, feasibility, consistency, and UI-spec. Resolves the related design doc internally.
memory: project
effort: max
---

You are a consolidated implementation-plan reviewer. In a single pass, produce findings across four observations. UI-spec observation is always included; if the plan has no UI tasks, emit zero UI findings.

## Scope

Review the target plan document. Resolve the related design doc path before starting the Consistency observation:

1. Extract date prefix from plan filename (e.g. `2026-04-07` from `2026-04-07-foo-plan.md`).
2. Find matching `docs/plans/<prefix>*-design.md`.
3. Read the design doc to ground Consistency checks.
4. If missing, proceed with reduced coverage and include a `low` severity finding under `consistency` noting the gap.

## Filtering (applies to all observations)

- 確信度 80% 未満の問題は報告しない
- 観点間で重複する指摘は最も本質的な観点 1 箇所のみに置く（self-dedup）

## Observation 1: Clarity

<TRANSCRIBE FROM implementation-review-clarity.md: Review Checklist, Investigation Method. Omit `Boundary`.>

## Observation 2: Feasibility

<TRANSCRIBE FROM implementation-review-feasibility.md: Review Checklist, Investigation Method, task-granularity rules, TDD test-case coverage checks. Omit `Boundary`.>

## Observation 3: Consistency

<TRANSCRIBE FROM implementation-review-consistency.md: Review Checklist, Investigation Method, design-to-plan mapping. Omit `Boundary`.>

## Observation 4: UI-spec

<TRANSCRIBE FROM implementation-review-ui-spec.md: Review Checklist, Investigation Method.> If the plan has no UI tasks, emit zero findings for this observation.

## Output Format

Write to `.belt/runs/{run_id}/review/findings.json`:

```json
{
  "findings": [
    {
      "id": "<uuid>",
      "observation": "clarity|feasibility|consistency|ui-spec|codex",
      "severity": "high|medium|low",
      "section": "<heading path or task identifier e.g. 'Task 3 / Step 2'>",
      "description": "...",
      "suggestion": "...",
      "source": "agent|codex"
    }
  ]
}
```

## Guardrails

- Do not modify the plan. Read-only.
- Do not invoke further subagents.
```

- [ ] **Step 3: Run the agent-file-existence assertion to confirm all 4 new files pass**

```bash
cargo test -p belt-core --test review_skills_refresh review_skills_consolidated_agent_files_exist
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add .claude/agents/implementation-reviewer.md
git -c commit.gpgsign=false commit -m "feat(agents): add consolidated implementation-reviewer with 4 observations"
```

---

### Task 6: Rewrite `code-review/pipeline.yml`

**Files:**
- Modify: `examples/skills/code-review/pipeline.yml`

- [ ] **Step 1: Overwrite the file with the refreshed shape**

Replace the entire contents of `examples/skills/code-review/pipeline.yml` with:

```yaml
name: code-review
version: 1
args:
  codex: { type: bool, default: false }

phases:
  - id: review
    description: "Multi-perspective code review (7 observations)"
    invoke:
      agents:
        - code-reviewer
      args:
        codex: "args.codex"
    produces:
      - name: review_findings
        path: ".belt/runs/*/review/findings.json"
        description: "Deduplicated findings across 7 observations (quality, security, performance, test, ai-antipattern, impact, simplification)"
    confirm: true

  - id: fix
    description: "Fix accepted findings from the review phase"
    consumes:
      - review_findings
    gate:
      - has_output: true
```

- [ ] **Step 2: Run belt lint**

```bash
cargo run --bin belt -- lint examples/skills/code-review/pipeline.yml
```

Expected: exit 0, no errors.

- [ ] **Step 3: Commit**

```bash
git add examples/skills/code-review/pipeline.yml
git -c commit.gpgsign=false commit -m "feat(code-review): consolidate pipeline to single reviewer agent"
```

---

### Task 7: Rewrite `spec-review/pipeline.yml`

**Files:**
- Modify: `examples/skills/spec-review/pipeline.yml`

- [ ] **Step 1: Overwrite the file**

Replace the entire contents of `examples/skills/spec-review/pipeline.yml` with:

```yaml
name: spec-review
version: 1
args:
  codex: { type: bool, default: false }

phases:
  - id: review
    description: "Multi-perspective spec review (5 observations) + grill-me dialogue"
    invoke:
      agents:
        - spec-reviewer
      args:
        codex: "args.codex"
    produces:
      - name: review_findings
        path: ".belt/runs/*/review/findings.json"
        description: "Deduplicated findings across 5 observations (requirements, design-judgment, feasibility, consistency, ui-design)"
    confirm: true

  - id: fix
    description: "Fix accepted findings from the review phase"
    consumes:
      - review_findings
    gate:
      - has_output: true
```

- [ ] **Step 2: Run belt lint**

```bash
cargo run --bin belt -- lint examples/skills/spec-review/pipeline.yml
```

Expected: exit 0, no errors.

- [ ] **Step 3: Commit**

```bash
git add examples/skills/spec-review/pipeline.yml
git -c commit.gpgsign=false commit -m "feat(spec-review): consolidate pipeline to single reviewer agent"
```

---

### Task 8: Rewrite `test-review/pipeline.yml`

**Files:**
- Modify: `examples/skills/test-review/pipeline.yml`

- [ ] **Step 1: Overwrite the file**

Replace the entire contents of `examples/skills/test-review/pipeline.yml` with:

```yaml
name: test-review
version: 1
args:
  codex: { type: bool, default: false }

phases:
  - id: review
    description: "Multi-perspective test review (3 observations) + requirement map"
    invoke:
      agents:
        - test-reviewer
      args:
        codex: "args.codex"
    produces:
      - name: review_findings
        path: ".belt/runs/*/review/findings.json"
        description: "Deduplicated findings (coverage, quality, design-alignment) + informational requirement_map"
    confirm: true

  - id: fix
    description: "Fix accepted findings from the review phase"
    consumes:
      - review_findings
    gate:
      - has_output: true
```

- [ ] **Step 2: Run belt lint**

```bash
cargo run --bin belt -- lint examples/skills/test-review/pipeline.yml
```

Expected: exit 0, no errors.

- [ ] **Step 3: Commit**

```bash
git add examples/skills/test-review/pipeline.yml
git -c commit.gpgsign=false commit -m "feat(test-review): consolidate pipeline to single reviewer agent"
```

---

### Task 9: Rewrite `implementation-review/pipeline.yml`

**Files:**
- Modify: `examples/skills/implementation-review/pipeline.yml`

- [ ] **Step 1: Overwrite the file**

Replace the entire contents of `examples/skills/implementation-review/pipeline.yml` with:

```yaml
name: implementation-review
version: 1
args:
  codex: { type: bool, default: false }

phases:
  - id: review
    description: "Multi-perspective plan review (4 observations)"
    invoke:
      agents:
        - implementation-reviewer
      args:
        codex: "args.codex"
    produces:
      - name: review_findings
        path: ".belt/runs/*/review/findings.json"
        description: "Deduplicated findings across 4 observations (clarity, feasibility, consistency, ui-spec)"
    confirm: true

  - id: fix
    description: "Fix accepted findings from the review phase"
    consumes:
      - review_findings
    gate:
      - has_output: true
```

- [ ] **Step 2: Run belt lint**

```bash
cargo run --bin belt -- lint examples/skills/implementation-review/pipeline.yml
```

Expected: exit 0, no errors.

- [ ] **Step 3: Run the Task 1 pipeline-shape tests**

```bash
cargo test -p belt-core --test review_skills_refresh review_skills_args_are_codex_only
cargo test -p belt-core --test review_skills_refresh review_skills_have_two_phases_review_then_fix
cargo test -p belt-core --test review_skills_refresh review_skills_invoke_single_consolidated_agent
```

Expected: all three PASS.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/implementation-review/pipeline.yml
git -c commit.gpgsign=false commit -m "feat(implementation-review): consolidate pipeline to single reviewer agent"
```

---

### Task 10: Rewrite `code-review/SKILL.md`

**Files:**
- Modify: `examples/skills/code-review/SKILL.md`

- [ ] **Step 1: Overwrite the file**

Replace the entire contents with:

```markdown
---
name: code-review
description: >-
  Multi-perspective code review via a single consolidated reviewer subagent.
  7 observations: quality, security, performance, test, ai-antipattern, impact,
  simplification. Diff-scoped. Optional Codex adversarial pass.
argument-hint: "[--codex]"
---

# Code Review

Multi-perspective code review with direct selection triage.

Dispatching and `invoke:` semantics follow `skills/belt-agent/SKILL.md`. This document covers only code-review-specific concerns (scope detection, impact context, triage, verify).

## Scope Detection

Determine diff scope before dispatching the reviewer agent:

1. If branch differs from `main` → `git diff main...HEAD`
2. Else if staged changes exist → `git diff --staged`
3. Else → report "no diff detected" and exit without dispatching

Pass the diff summary (file list + line counts) as context to the reviewer agent.

## Impact Observation Context

If a design doc exists in the current run's output directory (filename matches `*-design.md`), pass the Impact Analysis section content as additional context to the reviewer agent. The Impact observation consumes it.

## Triage

Categories: `quality`, `security`, `performance`, `test`, `ai-antipattern`, `impact`, `simplification`, `codex`.

All findings are presented as a numbered list sorted by severity descending. User selects which to fix by number. No dialogue phase.

## Verify (after fix)

1. `git diff` — confirm changes match approved findings scope
2. Auto-detect and run project linter:

| Indicator file | Linter command |
|---|---|
| `Cargo.toml` | `cargo clippy -- -D warnings` |
| `package.json` (has lint script) | `npm run lint` |
| `pyproject.toml` / `setup.cfg` | `ruff check .` or `flake8` |
| `go.mod` | `go vet ./...` |
| `Makefile` (has lint target) | `make lint` |

3. Auto-detect and run project tests:

| Indicator file | Test command |
|---|---|
| `Cargo.toml` | `cargo test` |
| `package.json` (has test script) | `npm test` |
| `pyproject.toml` | `pytest` |
| `go.mod` | `go test ./...` |
| `Makefile` (has test target) | `make test` |

4. If linter or tests fail → report honestly, do not suppress

## Red Flags

**Never:**
- Modify code without user approval of findings
- Change files outside the diff scope
- Omit or filter findings before presenting to user
- Suppress or hide test/linter failures

**Always:**
- Announce the reviewer agent being dispatched (and Codex, if `--codex`)
- Run linter and tests after applying fixes
- Apply fixes serially to avoid merge conflicts in the same file
```

- [ ] **Step 2: Commit**

```bash
git add examples/skills/code-review/SKILL.md
git -c commit.gpgsign=false commit -m "docs(code-review): rewrite SKILL.md for single-agent flow"
```

---

### Task 11: Rewrite `spec-review/SKILL.md` (grill-me dialogue)

**Files:**
- Modify: `examples/skills/spec-review/SKILL.md`

- [ ] **Step 1: Overwrite the file**

Replace the entire contents with:

```markdown
---
name: spec-review
description: >-
  Multi-perspective spec review via a single consolidated reviewer subagent.
  5 observations: requirements, design-judgment, feasibility, consistency,
  ui-design. Review → grill-me dialogue → selection → fix.
argument-hint: "[--codex]"
---

# Spec Review

Multi-perspective spec review with grill-me dialogue on design-critical findings and direct selection on the rest.

Dispatching and `invoke:` semantics follow `skills/belt-agent/SKILL.md`. This document covers only spec-review-specific concerns (triage, grill-me dialogue, selection, verify).

## Triage

After the reviewer agent returns findings, the orchestrator partitions them:

- **Grill-me group**: findings where `observation` ∈ {`requirements`, `design-judgment`} AND `severity` ∈ {`high`, `medium`}
- **Selection group**: everything else (feasibility, consistency, ui-design, low-severity, codex)

Process grill-me group first, then selection group.

## Grill-me Dialogue (Grill-me group)

Principles (borrowed from `/grill-me`):
- **One question at a time** — present a single finding; do not batch
- **Orchestrator provides a recommended answer** for every question
- **Codebase-answerable questions are not asked** — use Read/Grep to resolve them, update the suggestion, and move on
- **Rounds are unlimited** — iterate until the user explicitly accepts, rejects, or says "enough / move on"
- **Decision-tree order** — if finding A's decision affects finding B's proposal, resolve A first

Loop (pseudo):
```
order = topologically_sort(grill_group, by decision dependency)
for finding in order:
    while not resolved:
        if finding is answerable by codebase inspection:
            explore with Read/Grep
            revise finding.suggestion
            continue
        present finding + recommended_answer to user
        response = await user
        if response in {"accept", "OK", "approved"}:
            finding.resolution = "accept"; break
        if response in {"reject", "skip"}:
            finding.resolution = "reject"; break
        if response in {"enough", "move on"}:
            finding.resolution = "accept_current"; break  # accept revised state
        revise finding.suggestion based on response
```

After the loop, every grill-group finding has `resolution ∈ {accept, reject, accept_current}`.

## Selection Group

Present the selection-group findings as a numbered list sorted by severity descending. User picks by number which to fix.

## Verify (after fix)

1. `git diff` — confirm only target spec files changed
2. Markdown syntax check — no broken links, headers, or list formatting
3. Cross-reference consistency — internal document links still resolve

## Red Flags

**Never:**
- Modify spec without user approval of findings
- Change files outside the review target
- Omit or filter findings before presenting to user
- Ask a user question that could be answered by inspecting the codebase
- Present multiple grill-group findings simultaneously

**Always:**
- Announce the reviewer agent being dispatched (and Codex, if `--codex`)
- Provide a recommended answer with every grill-me question
- Explore the codebase before asking user questions
- Honor the user's "enough / move on" signal without pushback
- Get explicit user approval before applying any fixes
- Run verification checks after fixes are applied
```

- [ ] **Step 2: Commit**

```bash
git add examples/skills/spec-review/SKILL.md
git -c commit.gpgsign=false commit -m "docs(spec-review): rewrite SKILL.md with grill-me dialogue protocol"
```

---

### Task 12: Rewrite `test-review/SKILL.md`

**Files:**
- Modify: `examples/skills/test-review/SKILL.md`

- [ ] **Step 1: Overwrite the file**

Replace the entire contents with:

```markdown
---
name: test-review
description: >-
  Multi-perspective test review via a single consolidated reviewer subagent.
  3 observations: coverage, quality, design-alignment. Produces findings and
  requirement map. Optional Codex adversarial pass.
argument-hint: "[--codex]"
---

# Test Review

Multi-perspective test review with requirement mapping and direct selection triage.

Dispatching and `invoke:` semantics follow `skills/belt-agent/SKILL.md`. This document covers only test-review-specific concerns (requirement map handling, triage, fix strategy, verify).

## Design Spec Resolution

The reviewer agent resolves the design spec path internally (see `test-reviewer` agent). The orchestrator does not pre-resolve.

## Requirement Map

The reviewer agent emits a `requirement_map` array in `findings.json` alongside `findings`. Columns: number, requirement, source (design-spec section), test (file:line or `—`), gap (description or `—`).

The requirement map is **informational only** — not subject to selection. Present it as a table in the review report. Gap entries inform coverage findings in the numbered list.

## Triage

Categories: `coverage`, `quality`, `design-alignment`, `codex`.

All findings are presented as a numbered list sorted by severity descending. User selects which to fix by number. No dialogue phase.

### Fix Strategy by Observation

| Observation | Fix action |
|---|---|
| `coverage` | Add new test cases for uncovered paths |
| `quality` | Improve existing test structure, naming, assertions |
| `design-alignment` | Add requirement-based tests from requirement-map gaps |

## Verify (after fix)

1. `git diff` — confirm changes are test files only
2. Auto-detect and run project tests:

| Indicator file | Test command |
|---|---|
| `Cargo.toml` | `cargo test` |
| `package.json` (has test script) | `npm test` |
| `pyproject.toml` | `pytest` |
| `go.mod` | `go test ./...` |
| `Makefile` (has test target) | `make test` |

3. No linter step (test-only review)
4. If tests fail → report honestly, do not suppress

## Red Flags

**Never:**
- Modify production code (only test files)
- Change files outside the diff scope
- Omit or filter findings before presenting to user
- Classify test failures as acceptable without investigation

**Always:**
- Announce the reviewer agent being dispatched (and Codex, if `--codex`)
- Include the requirement map in the report even if no gaps found
- Run the full test suite after applying fixes
```

- [ ] **Step 2: Commit**

```bash
git add examples/skills/test-review/SKILL.md
git -c commit.gpgsign=false commit -m "docs(test-review): rewrite SKILL.md for single-agent flow"
```

---

### Task 13: Rewrite `implementation-review/SKILL.md`

**Files:**
- Modify: `examples/skills/implementation-review/SKILL.md`

- [ ] **Step 1: Overwrite the file**

Replace the entire contents with:

```markdown
---
name: implementation-review
description: >-
  Multi-perspective implementation-plan review via a single consolidated
  reviewer subagent. 4 observations: clarity, feasibility, consistency,
  ui-spec. Direct selection triage. Optional Codex adversarial pass.
argument-hint: "[--codex]"
---

# Implementation Review

Multi-perspective plan review with direct selection triage.

Dispatching and `invoke:` semantics follow `skills/belt-agent/SKILL.md`. This document covers only implementation-review-specific concerns (related design doc resolution, triage, verify).

## Related Design Doc Resolution

The reviewer agent resolves the related design doc internally (see `implementation-reviewer` agent). The orchestrator does not pre-resolve.

## Triage

Categories: `clarity`, `feasibility`, `consistency`, `ui-spec`, `codex`.

All findings are presented as a numbered list sorted by severity descending. User selects which to fix by number. No dialogue phase.

## Verify (after fix)

1. `git diff` — confirm only target plan files changed
2. Markdown syntax check — no broken links, headers, or list formatting
3. Cross-reference consistency — internal document links still resolve
4. Design doc alignment — modified sections still reference correct design decisions

## Red Flags

**Never:**
- Modify plan without user approval of findings
- Change files outside the review target
- Omit or filter findings before presenting to user

**Always:**
- Announce the reviewer agent being dispatched (and Codex, if `--codex`)
- Get explicit user approval before applying any fixes
- Run verification checks after fixes are applied
```

- [ ] **Step 2: Commit**

```bash
git add examples/skills/implementation-review/SKILL.md
git -c commit.gpgsign=false commit -m "docs(implementation-review): rewrite SKILL.md for single-agent flow"
```

---

### Task 14: Delete 18 legacy agent files

**Files:**
- Delete: 18 files listed in "File Structure → Delete" at the top of this plan

- [ ] **Step 1: Remove all legacy agent files**

```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
rm .claude/agents/code-review-quality.md
rm .claude/agents/code-review-security.md
rm .claude/agents/code-review-performance.md
rm .claude/agents/code-review-test.md
rm .claude/agents/code-review-ai-antipattern.md
rm .claude/agents/code-review-impact.md
rm .claude/agents/spec-review-requirements.md
rm .claude/agents/spec-review-design-judgment.md
rm .claude/agents/spec-review-feasibility.md
rm .claude/agents/spec-review-consistency.md
rm .claude/agents/spec-review-ui-design.md
rm .claude/agents/test-review-coverage.md
rm .claude/agents/test-review-quality.md
rm .claude/agents/test-review-design-alignment.md
rm .claude/agents/implementation-review-clarity.md
rm .claude/agents/implementation-review-feasibility.md
rm .claude/agents/implementation-review-consistency.md
rm .claude/agents/implementation-review-ui-spec.md
```

- [ ] **Step 2: Run the legacy-removal assertion**

```bash
cargo test -p belt-core --test review_skills_refresh legacy_review_agent_files_are_removed
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add -A .claude/agents/
git -c commit.gpgsign=false commit -m "chore(agents): remove 18 legacy per-observation review agents"
```

---

### Task 15: Remove obsolete Red Flag from `feature-dev/SKILL.md`

**Files:**
- Modify: `examples/skills/feature-dev/SKILL.md:76`

- [ ] **Step 1: Verify current line content**

```bash
grep -n "Never pass --iterations" examples/skills/feature-dev/SKILL.md
```

Expected output:
```
76:- **Never pass --iterations to /code-review**: single-pass review by design.
```

- [ ] **Step 2: Remove that line**

Using Edit tool:
- `old_string`: `- **Never pass --iterations to /code-review**: single-pass review by design.\n`
- `new_string`: `` (empty)

If the line is followed by a blank line that should also be collapsed, verify by reading lines 74-80 first and remove just the single line without breaking surrounding list structure. Preserve surrounding bullets.

- [ ] **Step 3: Verify removal**

```bash
grep -n "iterations" examples/skills/feature-dev/SKILL.md
```

Expected: no output (no matches).

- [ ] **Step 4: Commit**

```bash
git add examples/skills/feature-dev/SKILL.md
git -c commit.gpgsign=false commit -m "docs(feature-dev): drop obsolete Red Flag about --iterations"
```

---

### Task 16: Run all verification commands

**Files:** none (read-only verification)

- [ ] **Step 1: Rust formatting**

```bash
cargo fmt --package belt-core
```

Expected: no output, no file changes. (If changes appear, `git add` and re-run tests.)

- [ ] **Step 2: Rust clippy**

```bash
cargo clippy --package belt-core -- -D warnings
```

Expected: exit 0.

- [ ] **Step 3: Full belt-core test suite**

```bash
cargo test -p belt-core
```

Expected: all tests pass, including all 5 tests in `review_skills_refresh.rs` and all existing tests in `feature_dev_refresh.rs` and elsewhere.

- [ ] **Step 4: belt lint on all 4 refreshed pipelines**

```bash
cargo run --bin belt -- lint examples/skills/code-review/pipeline.yml
cargo run --bin belt -- lint examples/skills/spec-review/pipeline.yml
cargo run --bin belt -- lint examples/skills/test-review/pipeline.yml
cargo run --bin belt -- lint examples/skills/implementation-review/pipeline.yml
```

Expected: each command exits 0.

- [ ] **Step 5: belt lint on feature-dev pipeline (regression check)**

```bash
cargo run --bin belt -- lint examples/skills/feature-dev/pipeline.yml
```

Expected: exit 0 (should still pass since feature-dev pipeline.yml did not change).

- [ ] **Step 6: If any step above fails**

Stop and report the failure. Do not proceed to Task 17 until all checks pass.

---

### Task 17: Final cleanup + summary commit

**Files:** none (housekeeping)

- [ ] **Step 1: Confirm working tree is clean**

```bash
git status
```

Expected: `nothing to commit, working tree clean` (all per-task commits already made).

- [ ] **Step 2: Review commit log of this plan**

```bash
git log --oneline main~25..HEAD  # adjust range to cover all plan commits
```

Expected: logical sequence of commits matching the task list.

- [ ] **Step 3: No additional commit needed**

If working tree is clean and all tests pass, implementation is complete. Report success to the user.

If any stray files remain (e.g. editor backup files), clean them up before finishing.

---

## Self-Review (plan author, pre-handoff)

**Spec coverage check:**
- spec §2.1 `/code-review` refresh → Tasks 2, 6, 10
- spec §2.2 `/spec-review` refresh + grill-me → Tasks 3, 7, 11
- spec §2.3 `/test-review` refresh → Tasks 4, 8, 12
- spec §2.4 `/implementation-review` refresh → Tasks 5, 9, 13
- spec §3 Agent File Migration (create 4 + delete 18) → Tasks 2-5, 14
- spec §5 Impact on feature-dev skill → Task 15
- spec §5 Impact on belt-core tests → Task 1 verifies shape via new test; `feature_dev_refresh.rs` unchanged (as spec notes, no iterations/swarm assertions exist there)
- spec §2 Common Args (codex only) → enforced by Task 1 test, applied in Tasks 6-9
- spec §2 Common Pipeline Structure → enforced by Task 1 test, applied in Tasks 6-9
- spec §Grill-me Dialogue Protocol → Task 11 SKILL.md rewrite
- spec Risks section → mitigations are implicit in the task ordering (Task 1 failing tests before Tasks 2-9 lock the shape; Task 16 verification gates Task 17)

All spec requirements mapped to tasks.

**Placeholder scan:**
- Each Task N-M content block contains full YAML/Rust/Markdown, not "TBD"
- `<TRANSCRIBE FROM ...>` markers in Tasks 2-5 are content-directives pointing to read-specific files — not placeholders; the subagent resolves them by reading the referenced legacy agent file. This is necessary because the legacy content totals ~1250 lines across 18 files and including it verbatim in this plan would double its size without adding clarity
- No "add appropriate error handling" or similar hand-waving remains

**Type consistency:**
- Test function names in Task 1 (`review_skills_args_are_codex_only`, `review_skills_have_two_phases_review_then_fix`, `review_skills_invoke_single_consolidated_agent`, `review_skills_consolidated_agent_files_exist`, `legacy_review_agent_files_are_removed`) are referenced verbatim in Tasks 5 step 3, 9 step 3, and 14 step 2. Verified.
- Agent file names (`code-reviewer.md`, `spec-reviewer.md`, `test-reviewer.md`, `implementation-reviewer.md`) are consistent across Tasks 2-5, 6-9 (pipeline.yml `invoke.agents` references), and Task 1 test constants.
- Observation count/names in YAML `description` fields in Tasks 6-9 match the Observation sections in Tasks 2-5 (7/5/3/4) respectively.
- Legacy file list in Task 14 step 1 matches the list in the File Structure → Delete section at the top of this plan, and matches the `LEGACY` array in Task 1 step 1.
