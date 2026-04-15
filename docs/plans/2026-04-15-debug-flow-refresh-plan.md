# Debug Flow Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refresh `/debug-flow` skill to feature-dev + review-skills parity. Prerequisite: implement `Artifact.when` support in belt-core (5 components) to enable conditional produce. Then rewrite debug-flow pipeline.yml, SKILL.md, criteria (6 files), and references (6 supplements) to feature-dev aesthetic; delete dead-letter references; add integration test; feature-dev argument-hint follow-up for parity.

**Architecture:**
- **belt-core (Phase A.1)**: Substantive `Artifact.when` implementation across model (struct field), expander (retention), view (status filtering), engine (ArtifactRef resolution), lint (undefined arg warning). Also resolves silent-drop bug on feature-dev `scenarios: when: "args.e2e"` as a side effect.
- **debug-flow (Phase B–F)**: pipeline.yml rewrite (8 phases, args `{e2e, codex}`, `skill:` invoke everywhere); SKILL.md rewrite (Phase-Specific Invocation Rules + Red Flags + References); criteria refactor (RCA-09 added, fix-plan-review thinned to 3 criteria, monkey-test/dogfood/integrate new, fix-plan unchanged); supplement pattern (6 references files new); dead-letter removal + audit-protocol.md reference fix.
- **Integration test (Phase G)**: New `debug_flow_refresh.rs` with type-level + runtime assertions; feature-dev retrofit follow-up.
- **feature-dev follow-up (Phase H)**: Add `argument-hint` to SKILL.md for parity.

**Tech Stack:**
- Rust 1.94.1, Cargo workspace
- `serde` / `serde-saphyr` (YAML parsing)
- `miette` / `thiserror` (error handling)
- `belt-core` + `belt-agent` CLIs
- YAML (pipeline definitions) + Markdown (SKILL.md, criteria, supplements)

**Spec reference:** `docs/specs/2026-04-15-debug-flow-refresh-design.md` (commit `a215560`)

---

## File Structure

### Create

**belt-core:**
- `crates/belt-core/tests/artifact_when_field.rs` — Integration tests for `Artifact.when` semantics (Tasks 1–5)
- `crates/belt-core/tests/debug_flow_refresh.rs` — Shape + runtime test for refreshed debug-flow pipeline (Task 24)

**debug-flow (`examples/skills/debug-flow/`):**
- `criteria/monkey-test.md`
- `criteria/dogfood.md`
- `criteria/integrate.md`
- `references/path-convention.md`
- `references/rca-supplement.md`
- `references/fix-plan-supplement.md`
- `references/monkey-test-supplement.md`
- `references/dogfood-supplement.md`
- `references/worktrunk-supplement.md`

### Modify

**belt-core:**
- `crates/belt-core/src/model.rs` — Add `when: Option<String>` to Artifact (Task 1)
- `crates/belt-core/src/expander.rs` (or `expander/mod.rs`) — Retain `when` through expansion (Task 2)
- `crates/belt-core/src/view.rs` — Filter `when=false` artifacts from status (Task 3)
- `crates/belt-core/src/engine.rs` (or `engine/mod.rs`) — Resolve ArtifactRef to None when source `when=false` (Task 4)
- `crates/belt-core/src/lint.rs` — Warning for undefined arg in `Artifact.when` (Task 5)
- `crates/belt-core/tests/feature_dev_refresh.rs` — Retrofit type-level when assertion (Task 6)

**debug-flow:**
- `examples/skills/debug-flow/criteria/rca.md` — Add RCA-09 (Task 8)
- `examples/skills/debug-flow/criteria/fix-plan-review.md` — Thin rewrite (3 criteria) (Task 9)
- `examples/skills/debug-flow/pipeline.yml` — Rewrite to 8 phases (Task 19)
- `examples/skills/debug-flow/SKILL.md` — Rewrite to feature-dev aesthetic (Task 20)

**Project-wide:**
- `examples/references/audit-protocol.md` — Remove `fix-dispatch-strategy.md` reference (Task 22)
- `examples/skills/feature-dev/SKILL.md` — Add `argument-hint` for parity (Task 25)

### Delete

- `examples/skills/debug-flow/references/evidence-plan-protocol.md` (Task 21)
- `examples/skills/debug-flow/references/fix-dispatch-strategy.md` (Task 21)
- (conditional, if zero external references) `examples/criteria/smoke-test.md` (Task 23)
- (conditional, if zero external references) `examples/criteria/test-review.md` (Task 23)

### Unchanged (intentional, documented in spec)

- `examples/skills/debug-flow/criteria/fix-plan.md` — No modifications required
- `examples/skills/debug-flow/belt.toml` — No modifications required
- `examples/criteria/execute.md`, `examples/criteria/code-review.md` — Shared with feature-dev

---

## Verification Commands

Run these after Task 26:

```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt

# Rust
cargo fmt --package belt-core
cargo clippy --workspace -- -D warnings
cargo test -p belt-core --test artifact_when_field
cargo test -p belt-core --test debug_flow_refresh
cargo test -p belt-core --test feature_dev_refresh
cargo test -p belt-core --test review_skills_refresh

# belt lint on debug-flow pipeline (must pass after Task 19)
cargo run --bin belt -- lint examples/skills/debug-flow/pipeline.yml

# Grep hygiene (must return zero hits for legacy terms in debug-flow)
grep -rn "iterations\|swarm\|ui\(:\| =\)" examples/skills/debug-flow/
grep -rn "consensus\|artifacts/reviews/" examples/skills/debug-flow/criteria/
grep -rn "fix-dispatch-strategy" examples/references/
```

Expected: all commands exit 0, all tests pass, grep returns zero legacy-term matches.

### Must-Verify Checklist (from spec Impact Analysis)

- [ ] `cargo run -p belt -- lint examples/skills/debug-flow/pipeline.yml` PASS
- [ ] `cargo test -p belt-core --test debug_flow_refresh` PASS
- [ ] `cargo test -p belt-core --test feature_dev_refresh` PASS (regression)
- [ ] `cargo clippy --workspace -- -D warnings` PASS
- [ ] `grep -rn "iterations\|swarm\|ui\(:\|=\|-\) " examples/skills/debug-flow/` zero hit
- [ ] `grep -rn "consensus\|artifacts/reviews/" examples/skills/debug-flow/criteria/` zero hit
- [ ] `grep -rn "fix-dispatch-strategy" examples/references/` zero hit
- [ ] `belt-agent init --args e2e=false` run → status で `rca_scenarios` が artifacts に **含まない** (conditional produce 動作確認)
- [ ] `belt-agent init --args e2e=true` run → `rca_scenarios` が含まれる

---

## Task List

### Phase A: Pre-work investigation (Task 7 — **MUST run first**)

- [ ] Task 7: Pre-work investigations (A.2–A.5 from spec: shared criteria references, fix-plan.md coverage, criteria N-way grep, agent existence). **A.5 agent existence is a hard gate — stop if missing**.

### Phase A: belt-core `Artifact.when` implementation (Tasks 1–6)

- [ ] Task 1: Add `when: Option<String>` field to Artifact struct (incl. expansion propagation assertion)
- [ ] Task 2: Regression guard — verify expander's Clone-based propagation retains `Artifact.when`
- [ ] Task 3: `view.rs` filters `when=false` artifacts from status JSON (`pub fn evaluate_when` + `active_produces`)
- [ ] Task 4: ArtifactRef resolves to `None` when source `when=false` (on expanded phases)
- [ ] Task 5: Lint warning for undefined arg reference in `Artifact.when` (via `LintDiagnostic + Severity::Warning`)
- [ ] Task 6: Retrofit type-level `when` assertion in `feature_dev_refresh.rs`

### Phase B: Criteria (Tasks 8–12)

- [ ] Task 8: Add RCA-09 to `criteria/rca.md`
- [ ] Task 9: Rewrite `criteria/fix-plan-review.md` (3 thin criteria)
- [ ] Task 10: Create `criteria/monkey-test.md`
- [ ] Task 11: Create `criteria/dogfood.md`
- [ ] Task 12: Create `criteria/integrate.md`

### Phase C: References & Supplements (Tasks 13–18)

- [ ] Task 13: Create `references/path-convention.md`
- [ ] Task 14: Create `references/rca-supplement.md`
- [ ] Task 15: Create `references/fix-plan-supplement.md`
- [ ] Task 16: Create `references/monkey-test-supplement.md`
- [ ] Task 17: Create `references/dogfood-supplement.md`
- [ ] Task 18: Create `references/worktrunk-supplement.md`

### Phase D–F: pipeline.yml / SKILL.md / Dead-letter cleanup (Tasks 19–23)

- [ ] Task 19: Rewrite `pipeline.yml` to target shape
- [ ] Task 20: Rewrite `SKILL.md` to feature-dev aesthetic
- [ ] Task 21: Delete dead-letter references (`evidence-plan-protocol.md`, `fix-dispatch-strategy.md`)
- [ ] Task 22: Remove `fix-dispatch-strategy.md` reference from `examples/references/audit-protocol.md`
- [ ] Task 23: (conditional) Delete shared `criteria/smoke-test.md` and `criteria/test-review.md` if zero external references

### Phase G: Integration test (Task 24)

- [ ] Task 24: Create `debug_flow_refresh.rs` (failing test → PASS after all fixtures)

### Phase H: feature-dev follow-up (Task 25)

- [ ] Task 25: Add `argument-hint: "[--e2e] [--codex]"` to `feature-dev/SKILL.md`

### Final verification (Task 26)

- [ ] Task 26: Run all verification commands + Must-Verify Checklist

---

## Risks & Pitfalls (Plan-time awareness)

From memory `project_review_skills_refresh_2026_04_15.md` — plan verbatim で clippy pedantic 違反混入の pitfalls。各 Rust task の step で意識すること:

1. **Unused import** (`Path` etc.) — `cargo clippy --workspace -- -D warnings` で検出、test task 末尾で確認
2. **items-after-statements** — `let` 文と `fn`/`const` の混在回避、先に declarations、後に statements
3. **use block の rustfmt collapse** — 手書き use ブロックの改行を意識、`cargo fmt` 後に確認
4. **Output Format 自己矛盾ルール** — criteria file 内の "must contain X / must NOT contain X" drift 回避
5. **新 observation の Review Checklist / Policy 欠落** — 新 criteria file で Checklist/Policy 節を必ず書く
6. **`severity` enum と policy text の drift** — `blocker / quality / warning` で statement 統一

並行セッション branch-race (memory `project_parallel_session_worktree_isolation.md`):
- subagent dispatch プロンプトに**絶対パスで worktree 指定**
- subagent 側で `git branch --show-current` で main branch 確認

### Deferred follow-up (not in scope)

Spec Impact Analysis "Side Effect Risks" #1 notes that `deny_unknown_fields` audit across belt-core structs is a follow-up audit task — not addressed in this plan. After this PR merges, open a follow-up issue to grep belt-core for `#[derive(Deserialize)]` structs lacking `#[serde(deny_unknown_fields)]` and enumerate remaining silent-drop risks. This plan closes only the `Artifact.when` specific case.

---

### Task 1: Add `when: Option<String>` field to Artifact struct

**Files:**
- Modify: `crates/belt-core/src/model.rs`
- Create: `crates/belt-core/tests/artifact_when_field.rs`

**Commit:** `feat(belt-core): add Artifact.when field`

- [ ] **Step 1: Read current Artifact struct**

Read `crates/belt-core/src/model.rs` around lines 176–189 to confirm current shape:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub description: Option<String>,
}
```

- [ ] **Step 2: Write the failing integration test**

Create `crates/belt-core/tests/artifact_when_field.rs` with the following content:

```rust
//! Integration tests for `Artifact.when` field support.
//!
//! Spec: `docs/specs/2026-04-15-debug-flow-refresh-design.md` ("Artifact.when Semantics").

use std::path::PathBuf;

use belt_core::parser::parse_pipeline;

fn fixture_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(name);
    path
}

fn write_fixture(name: &str, content: &str) -> PathBuf {
    let path = fixture_path(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
    path
}

#[test]
fn artifact_when_field_is_retained_on_parse() {
    let yaml = r#"
name: test-when-field
version: 1
args:
  e2e:
    type: bool
    default: false
phases:
  - id: phase1
    invoke:
      skill: /test-skill
    produces:
      - name: conditional_artifact
        path: "output/*.md"
        when: "args.e2e"
      - name: unconditional_artifact
        path: "other/*.md"
"#;
    let fixture = write_fixture("when_field.yml", yaml);
    let pipeline = parse_pipeline(&fixture).expect("parse must succeed");
    let produces = &pipeline.phases[0].produces;
    assert_eq!(produces.len(), 2, "both artifacts must be parsed");
    assert_eq!(
        produces[0].when,
        Some("args.e2e".to_string()),
        "Artifact.when must be retained on parse"
    );
    assert_eq!(
        produces[1].when,
        None,
        "unconditional Artifact.when must be None"
    );
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p belt-core --test artifact_when_field -- artifact_when_field_is_retained_on_parse`

Expected: FAIL with compile error `no field 'when' on type 'Artifact'`.

- [ ] **Step 4: Add `when: Option<String>` to Artifact struct**

Edit `crates/belt-core/src/model.rs`, replacing the `Artifact` struct:

```rust
/// A typed artifact produced by a phase. The `name` is a logical identifier
/// by which later phases reference the artifact via `consumes:`. The `path`
/// is the filesystem path the LLM is expected to produce (glob permitted for
/// runtime-determined filenames like `docs/plans/*-design.md`).
///
/// `when` is an optional expression (currently `args.<flag>` boolean reference
/// only) that, when false, causes the artifact to be omitted from the run's
/// produces list (see spec 2026-04-15-debug-flow-refresh-design.md "Artifact.when Semantics").
///
/// Glob resolution semantics are intentionally not specified here; they are
/// deferred to the Plan B examples migration implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub when: Option<String>,
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p belt-core --test artifact_when_field -- artifact_when_field_is_retained_on_parse`

Expected: PASS.

- [ ] **Step 6: Run clippy + fmt**

```bash
cargo fmt --package belt-core
cargo clippy --package belt-core -- -D warnings
```

Expected: both exit 0.

- [ ] **Step 7: Commit**

```bash
git add crates/belt-core/src/model.rs crates/belt-core/tests/artifact_when_field.rs crates/belt-core/tests/fixtures/when_field.yml
git commit -m "feat(belt-core): add Artifact.when field"
```

---

### Task 2: Regression guard — expander's Clone-based propagation retains `Artifact.when`

**Rationale:** The existing expander at `crates/belt-core/src/expander.rs` (lines ~95 / ~137) propagates produces via `phase.produces.clone()`. Because `Artifact` is `#[derive(Clone)]`, once Task 1 adds the `when` field, `Clone` auto-propagates it through expansion with no expander code change. This task adds a **regression guard test** (not a red-green TDD cycle) to lock the behavior, protecting against future refactors that might break the Clone propagation.

**Files:**
- Modify: `crates/belt-core/tests/artifact_when_field.rs`
- Create: `crates/belt-core/tests/fixtures/when_expander.yml`

**Commit:** `test(belt-core): regression guard for Artifact.when propagation through expander`

- [ ] **Step 1: Read current expander**

Read `crates/belt-core/src/expander.rs` and confirm: (a) `expand_pipeline(pipeline_path: &Path) -> BeltResult<Vec<ExpandedPhase>>` signature (single-path argument, returns `Vec<ExpandedPhase>` — **not** `ExpandedPipeline` struct), (b) produces propagation uses wholesale `.clone()`.

- [ ] **Step 2: Add regression guard test**

Append to `crates/belt-core/tests/artifact_when_field.rs`:

```rust
use belt_core::expander::expand_pipeline;

#[test]
fn expander_retains_artifact_when_field_via_clone() {
    let yaml = r#"
name: test-when-expander
version: 1
args:
  e2e:
    type: bool
    default: false
phases:
  - id: phase1
    invoke:
      skill: /test-skill
    produces:
      - name: conditional_artifact
        path: "output/*.md"
        when: "args.e2e"
"#;
    let fixture = write_fixture("when_expander.yml", yaml);
    let expanded = expand_pipeline(&fixture).expect("expansion must succeed");
    assert_eq!(
        expanded[0].produces[0].when,
        Some("args.e2e".to_string()),
        "expander must retain Artifact.when via Clone derive"
    );
}
```

- [ ] **Step 3: Run test — expect PASS immediately**

Run: `cargo test -p belt-core --test artifact_when_field -- expander_retains_artifact_when_field_via_clone`

Expected: **PASS** (Task 1's `Clone` derive auto-propagates the new field through expansion). If FAIL, investigate — it would indicate the expander diverged from Clone-based propagation and requires targeted fix.

- [ ] **Step 4: Run clippy + fmt**

```bash
cargo fmt --package belt-core
cargo clippy --package belt-core -- -D warnings
```

Expected: both exit 0.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/tests/artifact_when_field.rs crates/belt-core/tests/fixtures/when_expander.yml
git commit -m "test(belt-core): regression guard for Artifact.when propagation through expander"
```

---

### Task 3: `view.rs` filters `when=false` artifacts from status JSON

**Files:**
- Modify: `crates/belt-core/src/view.rs`
- Modify: `crates/belt-core/src/engine.rs` (or equivalent status-assembly call site)
- Modify: `crates/belt-core/tests/artifact_when_field.rs`
- Create: `crates/belt-core/tests/fixtures/when_view.yml`

**Commit:** `feat(belt-core): omit conditional artifacts from status when false`

- [ ] **Step 1: Read current `view.rs` + status-assembly call sites**

Read `crates/belt-core/src/view.rs` to locate `PhaseMetadata`, `build_status_view`, `resolve_produces` (or equivalent). Identify where `phase.produces` is iterated during status-JSON assembly.

Read `crates/belt-core/src/engine.rs` (or module) to locate the caller (likely `enriched_status` around line 368) that supplies `RunState.args: HashMap<String, serde_json::Value>` to view. This is the integration point — `args` must be threaded into `resolve_produces` so filtering happens before `ResolvedArtifact` construction.

- [ ] **Step 2: Add failing test to `artifact_when_field.rs`**

Append:

```rust
use std::collections::HashMap;

#[test]
fn view_filters_when_false_artifacts_from_status() {
    let yaml = r#"
name: test-when-view
version: 1
args:
  e2e:
    type: bool
    default: false
phases:
  - id: phase1
    invoke:
      skill: /test-skill
    produces:
      - name: conditional
        path: "out/*.md"
        when: "args.e2e"
      - name: unconditional
        path: "other/*.md"
"#;
    let fixture = write_fixture("when_view.yml", yaml);
    let expanded = belt_core::expander::expand_pipeline(&fixture).expect("expand");

    // args.e2e = false → conditional must be omitted
    let mut args_false: HashMap<String, serde_json::Value> = HashMap::new();
    args_false.insert("e2e".to_string(), serde_json::Value::Bool(false));
    let produces_false = belt_core::view::active_produces(&expanded[0], &args_false);
    let names_false: Vec<&str> = produces_false.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(
        names_false, vec!["unconditional"],
        "when=false artifacts must be filtered out"
    );

    // args.e2e = true → both present
    let mut args_true: HashMap<String, serde_json::Value> = HashMap::new();
    args_true.insert("e2e".to_string(), serde_json::Value::Bool(true));
    let produces_true = belt_core::view::active_produces(&expanded[0], &args_true);
    let names_true: Vec<&str> = produces_true.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names_true.len(), 2);
    assert!(names_true.contains(&"conditional"));
    assert!(names_true.contains(&"unconditional"));

    // empty args map → conditional omitted (undefined flag → false)
    let args_empty: HashMap<String, serde_json::Value> = HashMap::new();
    let produces_empty = belt_core::view::active_produces(&expanded[0], &args_empty);
    let names_empty: Vec<&str> = produces_empty.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names_empty, vec!["unconditional"]);
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p belt-core --test artifact_when_field -- view_filters_when_false_artifacts_from_status`

Expected: FAIL — `active_produces` function does not exist yet (compile error).

- [ ] **Step 4: Implement `evaluate_when` + `active_produces` in `view.rs`**

Add to `crates/belt-core/src/view.rs`:

```rust
use std::collections::HashMap;

/// Evaluates an `Artifact.when` expression (grammar: `args.<flag>` only).
/// Returns true when `when` is None. Returns false for unsupported expressions
/// or undefined arg references.
pub fn evaluate_when(
    when: Option<&str>,
    args: &HashMap<String, serde_json::Value>,
) -> bool {
    let Some(expr) = when else {
        return true;
    };
    let expr = expr.trim();
    let Some(arg_name) = expr.strip_prefix("args.") else {
        return false;
    };
    args.get(arg_name)
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Returns the artifacts in `phase.produces` whose `when` expression evaluates
/// to true (or has no `when`).
pub fn active_produces<'a>(
    phase: &'a crate::model::ExpandedPhase,
    args: &HashMap<String, serde_json::Value>,
) -> Vec<&'a crate::model::Artifact> {
    phase
        .produces
        .iter()
        .filter(|artifact| evaluate_when(artifact.when.as_deref(), args))
        .collect()
}
```

Ensure the `use` block includes any needed imports. If `ExpandedPhase` lives in a different module, adjust the import. If the existing `view.rs` `use` statements already bring in `HashMap` / `ExpandedPhase`, drop the local `use std::collections::HashMap;` at the top.

- [ ] **Step 5: Wire `active_produces` into status assembly**

In `view.rs`'s `resolve_produces` (or equivalent status-assembly function), change the signature to accept `args: &HashMap<String, serde_json::Value>` and call `active_produces(phase, args)` before iterating for `ResolvedArtifact` construction.

In `crates/belt-core/src/engine.rs` `enriched_status` (around line 368), pass `&run_state.args` through to the updated `build_status_view` / `resolve_produces` function.

Example shape (adjust to actual API):

```rust
// in view.rs
pub fn resolve_produces(
    produces: &[crate::model::Artifact],
    phase_start: i64,
    args: &HashMap<String, serde_json::Value>,
) -> Vec<ResolvedArtifact> {
    produces
        .iter()
        .filter(|a| evaluate_when(a.when.as_deref(), args))
        .map(|a| resolve_one(a, phase_start))
        .collect()
}
```

- [ ] **Step 6: Run test + regression**

```bash
cargo test -p belt-core --test artifact_when_field -- view_filters_when_false_artifacts_from_status
cargo test -p belt-core
```

Expected: all pass. If any existing test breaks due to the new `args` parameter, update call sites to thread args through.

- [ ] **Step 7: Run clippy + fmt**

```bash
cargo fmt --package belt-core
cargo clippy --workspace -- -D warnings
```

Expected: both exit 0.

- [ ] **Step 8: Commit**

```bash
git add crates/belt-core/src/view.rs crates/belt-core/src/engine.rs crates/belt-core/tests/artifact_when_field.rs crates/belt-core/tests/fixtures/when_view.yml
git commit -m "feat(belt-core): omit conditional artifacts from status when false"
```

---

### Task 4: ArtifactRef resolves to `None` when source `when=false`

**Files:**
- Modify: `crates/belt-core/src/engine.rs` (or `src/engine/mod.rs`)
- Modify: `crates/belt-core/tests/artifact_when_field.rs`
- Create: `crates/belt-core/tests/fixtures/when_resolve.yml`

**Commit:** `feat(belt-core): ArtifactRef resolves None for conditional source`

- [ ] **Step 1: Read existing consume resolution**

Read `crates/belt-core/src/engine.rs` (or module). Locate the current consume resolution logic used by `set_resolved_consumes` / `enriched_status` — this is where an `ArtifactRef::Named("conditional")` is matched against earlier phases' produces. Confirm there is no `when` handling today.

Note: `Engine` is a struct with methods (`pub struct Engine { belt_dir: PathBuf }` and its impl block). The existing pattern is to expose resolution as either (a) an `Engine` method, or (b) a free `pub fn` in `engine.rs` taking `&[ExpandedPhase]` / `&Vec<ExpandedPhase>`. Prefer **(b)** since this is a pure function over expanded phases + args, independent of `Engine` state.

- [ ] **Step 2: Add failing test to `artifact_when_field.rs`**

Append:

```rust
#[test]
fn artifact_ref_returns_none_when_source_conditional_false() {
    let yaml = r#"
name: test-when-resolve
version: 1
args:
  e2e:
    type: bool
    default: false
phases:
  - id: phase1
    invoke:
      skill: /test-skill
    produces:
      - name: conditional
        path: "out/*.md"
        when: "args.e2e"
  - id: phase2
    invoke:
      skill: /test-skill
    consumes:
      - conditional
"#;
    let fixture = write_fixture("when_resolve.yml", yaml);
    let expanded = belt_core::expander::expand_pipeline(&fixture).expect("expand");

    // args.e2e=false → resolve_artifact_ref must return None
    let mut args_false: HashMap<String, serde_json::Value> = HashMap::new();
    args_false.insert("e2e".to_string(), serde_json::Value::Bool(false));
    let resolved_false = belt_core::engine::resolve_artifact_ref(&expanded, "conditional", &args_false);
    assert!(
        resolved_false.is_none(),
        "ArtifactRef must resolve to None when source when=false"
    );

    // args.e2e=true → resolve_artifact_ref returns Some(...)
    let mut args_true: HashMap<String, serde_json::Value> = HashMap::new();
    args_true.insert("e2e".to_string(), serde_json::Value::Bool(true));
    let resolved_true = belt_core::engine::resolve_artifact_ref(&expanded, "conditional", &args_true);
    assert!(
        resolved_true.is_some(),
        "ArtifactRef must resolve to Some when source when=true"
    );
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p belt-core --test artifact_when_field -- artifact_ref_returns_none_when_source_conditional_false`

Expected: FAIL — function `resolve_artifact_ref` does not exist.

- [ ] **Step 4: Implement `resolve_artifact_ref` in `engine.rs`**

Add to `crates/belt-core/src/engine.rs`:

```rust
use std::collections::HashMap;

/// Resolves an artifact name to its producing artifact in the expanded pipeline.
/// Returns `None` if the name is unknown, or if the producing artifact has a
/// `when` expression evaluating to false under the given args.
///
/// Used by `belt-agent next` / `status` when assembling consumes resolution.
pub fn resolve_artifact_ref<'a>(
    phases: &'a [crate::model::ExpandedPhase],
    artifact_name: &str,
    args: &HashMap<String, serde_json::Value>,
) -> Option<&'a crate::model::Artifact> {
    for phase in phases {
        for artifact in &phase.produces {
            if artifact.name == artifact_name {
                if crate::view::evaluate_when(artifact.when.as_deref(), args) {
                    return Some(artifact);
                }
                return None;
            }
        }
    }
    None
}
```

Note: `crate::view::evaluate_when` is defined as `pub` in Task 3. No rename needed.

- [ ] **Step 5: Update call sites**

Locate existing consume resolution in `engine.rs` (specifically `set_resolved_consumes` / the helper that matches `consumes` artifact names against upstream phases' `produces`). Replace with `resolve_artifact_ref(&expanded_phases, name, &args)` so that a consume targeting a conditionally-skipped source yields `None` (propagating "not resolved" per spec Artifact.when Semantics #2).

- [ ] **Step 6: Run test to verify it passes + regression**

```bash
cargo test -p belt-core --test artifact_when_field -- artifact_ref_returns_none_when_source_conditional_false
cargo test -p belt-core
```

Expected: all pass.

- [ ] **Step 7: Run clippy + fmt**

```bash
cargo fmt --package belt-core
cargo clippy --workspace -- -D warnings
```

Expected: both exit 0.

- [ ] **Step 8: Commit**

```bash
git add crates/belt-core/src/engine.rs crates/belt-core/tests/artifact_when_field.rs crates/belt-core/tests/fixtures/when_resolve.yml
git commit -m "feat(belt-core): ArtifactRef resolves None for conditional source"
```

---

### Task 5: Lint warning for undefined arg reference in `Artifact.when`

**Files:**
- Modify: `crates/belt-core/src/lint.rs`
- Modify: `crates/belt-core/tests/artifact_when_field.rs`
- Create: `crates/belt-core/tests/fixtures/when_lint.yml`

**Commit:** `feat(belt-core): lint warning for undefined arg in Artifact.when`

- [ ] **Step 1: Read current lint API**

Read `crates/belt-core/src/lint.rs`. Confirm:
- `pub fn lint_pipeline(path: &Path) -> BeltResult<Vec<LintDiagnostic>>`
- `pub struct LintDiagnostic { pub severity: Severity, pub message: String }`
- `pub enum Severity { Error, Warning }`
- Existing checks use pattern: `diagnostics.push(LintDiagnostic { severity: Severity::Error, message: format!(...) });`

Locate the main `lint_pipeline` function body where to wire a new check.

- [ ] **Step 2: Add failing test to `artifact_when_field.rs`**

Append:

```rust
use belt_core::lint::{lint_pipeline, Severity};

#[test]
fn lint_warns_undefined_arg_in_artifact_when() {
    let yaml = r#"
name: test-when-lint
version: 1
args:
  e2e:
    type: bool
    default: false
phases:
  - id: phase1
    invoke:
      skill: /test-skill
    produces:
      - name: bad
        path: "out/*.md"
        when: "args.undefined_flag"
"#;
    let fixture = write_fixture("when_lint.yml", yaml);
    let diagnostics = lint_pipeline(&fixture).expect("lint");
    let undefined_warning = diagnostics
        .iter()
        .find(|d| d.severity == Severity::Warning && d.message.contains("undefined_flag"));
    assert!(
        undefined_warning.is_some(),
        "lint must emit Warning for undefined arg reference in Artifact.when, got diagnostics: {diagnostics:?}"
    );
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p belt-core --test artifact_when_field -- lint_warns_undefined_arg_in_artifact_when`

Expected: FAIL — no Warning diagnostic produced.

- [ ] **Step 4: Implement lint check**

In `lint.rs`, add a new check function (mirror the style of the existing `check_validate_file_refs` or similar):

```rust
/// Check: Artifact.when expressions must reference defined args.
fn check_artifact_when_references(
    pipeline: &crate::model::Pipeline,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let defined_args: std::collections::HashSet<&str> =
        pipeline.args.keys().map(String::as_str).collect();
    for phase in &pipeline.phases {
        for artifact in &phase.produces {
            let Some(expr) = artifact.when.as_deref() else {
                continue;
            };
            let expr = expr.trim();
            let Some(arg_name) = expr.strip_prefix("args.") else {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Warning,
                    message: format!(
                        "phase '{}' artifact '{}' has unsupported when expression '{}'; only `args.<flag>` is supported",
                        phase.id, artifact.name, expr
                    ),
                });
                continue;
            };
            if !defined_args.contains(arg_name) {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Warning,
                    message: format!(
                        "phase '{}' artifact '{}' references undefined arg '{}' in when clause",
                        phase.id, artifact.name, arg_name
                    ),
                });
            }
        }
    }
}
```

Wire this into `lint_pipeline`: in the main body after existing checks run, call `check_artifact_when_references(&pipeline, &mut diagnostics);`. If existing checks use a different signature (e.g., returning `Vec<LintDiagnostic>` and the caller extends), match that convention.

- [ ] **Step 5: Run test to verify it passes + regression**

```bash
cargo test -p belt-core --test artifact_when_field -- lint_warns_undefined_arg_in_artifact_when
cargo test -p belt-core
```

Expected: all pass.

- [ ] **Step 6: Run clippy + fmt**

```bash
cargo fmt --package belt-core
cargo clippy --workspace -- -D warnings
```

Expected: both exit 0.

- [ ] **Step 7: Commit**

```bash
git add crates/belt-core/src/lint.rs crates/belt-core/tests/artifact_when_field.rs crates/belt-core/tests/fixtures/when_lint.yml
git commit -m "feat(belt-core): lint warning for undefined arg in Artifact.when"
```

---

### Task 6: Retrofit type-level `when` assertion in `feature_dev_refresh.rs`

**Files:**
- Modify: `crates/belt-core/tests/feature_dev_refresh.rs`

**Commit:** `test(belt-core): retrofit type-level Artifact.when assertion for feature-dev`

- [ ] **Step 1: Read current `feature_dev_refresh.rs` scenarios assertion**

Read `crates/belt-core/tests/feature_dev_refresh.rs` around lines 81–130. Locate the test that currently asserts the `scenarios: when: "args.e2e"` text existence via `serde_json::Value`.

- [ ] **Step 2: Add type-level assertion**

First, read the existing helpers in `feature_dev_refresh.rs` (around lines 9–17) to confirm the actual helper name (likely `feature_dev_pipeline_path()` rather than `repo_root()`).

Add the following as a NEW standalone test at the end of the file (after all existing tests):

```rust
#[test]
fn feature_dev_scenarios_artifact_has_typed_when_field() {
    let pipeline_path = feature_dev_pipeline_path();
    let pipeline = belt_core::parser::parse_pipeline(&pipeline_path)
        .expect("feature-dev pipeline must parse");
    let test_scenarios_phase = pipeline
        .phases
        .iter()
        .find(|p| p.id == "test-scenarios")
        .expect("test-scenarios phase must exist");
    let scenarios_artifact = test_scenarios_phase
        .produces
        .iter()
        .find(|a| a.name == "scenarios")
        .expect("scenarios artifact must exist");
    assert_eq!(
        scenarios_artifact.when,
        Some("args.e2e".to_string()),
        "scenarios.when must parse as a typed field (regression test for silent-drop bug)"
    );
}
```

If the existing file uses a different helper name (e.g., `repo_root()` or a raw-path literal), replace `feature_dev_pipeline_path()` with the matching helper. Do NOT redefine a helper — use what exists.

- [ ] **Step 3: Run test**

```bash
cargo test -p belt-core --test feature_dev_refresh -- feature_dev_scenarios_artifact_has_typed_when_field
```

Expected: PASS (because Tasks 1–2 implemented `Artifact.when`).

Run full regression:

```bash
cargo test -p belt-core
```

Expected: all pass.

- [ ] **Step 4: Run clippy + fmt**

```bash
cargo fmt --package belt-core
cargo clippy --package belt-core -- -D warnings
```

Expected: both exit 0.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/tests/feature_dev_refresh.rs
git commit -m "test(belt-core): retrofit type-level Artifact.when assertion for feature-dev"
```

---

### Task 7: Pre-work investigations (A.2–A.5)

**Files:** None (investigation only, no commit)

**Purpose:** Verify spec assumptions before modifying debug-flow files.

- [ ] **Step 1: Investigate shared criteria external references (A.2)**

Run:

```bash
grep -rn "criteria/smoke-test.md" examples/ crates/ docs/ 2>/dev/null | grep -v debug-flow
grep -rn "criteria/test-review.md" examples/ crates/ docs/ 2>/dev/null | grep -v debug-flow
```

Record which skills (if any) reference `smoke-test.md` / `test-review.md` outside `debug-flow/`. This gates Task 23 (conditional deletion).

- [ ] **Step 2: Investigate fix-plan.md FIX-PLAN-05 coverage of old FIX-PLAN-REVIEW-04 (A.3)**

Read `examples/skills/debug-flow/criteria/fix-plan.md`, locate FIX-PLAN-05 (test cases in Given/When/Then format). Compare with the current FIX-PLAN-REVIEW-04 in `criteria/fix-plan-review.md` (task completion condition verifiability).

Confirm: does FIX-PLAN-05 already cover the "task completion conditions are verifiable" dimension that old FIX-PLAN-REVIEW-04 audits? If not, note the gap — Task 9 (thin fix-plan-review rewrite) must preserve the uncovered aspect.

- [ ] **Step 3: Criteria N-way residual grep (A.4)**

Run:

```bash
grep -n "Observation Collection\|depends_on_artifacts\|forward_check\|consensus\|artifacts/reviews" \
  examples/skills/debug-flow/criteria/*.md
```

Record:
- Any `depends_on_artifacts: [artifacts/reviews/]` → must be updated to `.belt/runs/*/review/` in Task 9 (new FIX-PLAN-REVIEW-01 / -02 / -03)
- Any `forward_check` containing `consensus` / `過半数` → note for rewrite
- `Observation Collection` sections — verify compatibility with new single-agent review (no N-way assumptions)

- [ ] **Step 4: Agent existence verification (A.5)**

Run:

```bash
ls ~/.claude/agents/phase-auditor.md ~/.claude/agents/feature-implementer.md
ls examples/skills/../.claude/agents/code-reviewer.md examples/skills/../.claude/agents/implementation-reviewer.md
# (Adjust paths if the test invocation point differs)
```

Use absolute paths:
```bash
ls /Users/nishikataseiichi/.claude/agents/phase-auditor.md
ls /Users/nishikataseiichi/.claude/agents/feature-implementer.md
ls /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/.claude/agents/code-reviewer.md
ls /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/.claude/agents/implementation-reviewer.md
```

Expected: all files exist. If any missing, stop and report — plan cannot proceed without these agents (spec Subagent Dependencies).

- [ ] **Step 5: Report findings**

Produce a brief investigation summary (inline, no file):
- A.2 result: `smoke-test.md` referenced by [list] / `test-review.md` referenced by [list]. Task 23 execution decision recorded.
- A.3 result: FIX-PLAN-05 coverage of old -04 = [full/partial/none]. Note gaps.
- A.4 result: [list of N-way residuals + path drift locations]. Task 9 scope confirmed.
- A.5 result: [all-exist / any-missing]. Proceed-or-block decision.

No commit.

---

### Task 8: Add RCA-09 to `criteria/rca.md`

**Files:**
- Modify: `examples/skills/debug-flow/criteria/rca.md`

**Commit:** `refactor(debug-flow): add RCA-09 for conditional rca_scenarios artifact`

- [ ] **Step 1: Read current `criteria/rca.md`**

Read `examples/skills/debug-flow/criteria/rca.md` to confirm: (a) existing RCA-01 through RCA-08 structure, (b) frontmatter format, (c) "Observation Collection" section format.

- [ ] **Step 2: Add RCA-09 before "Observation Collection" section**

Insert the following criterion after RCA-08:

```markdown
### RCA-09: Reproduction scenarios file exists when --e2e
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Read `args.e2e` from `belt-agent status --run-id <id>` JSON output
  2. If `args.e2e=false`, PASS (vacuously satisfied — scenarios not required for non-e2e runs)
  3. If `args.e2e=true`:
     a. Search for scenarios file using `Glob("docs/plans/*-rca-scenarios.yml")`
     b. Verify the file contains at least one scenario in Given/When/Then format
- **pass_condition**: `args.e2e=false`, OR (file exists with ≥1 Given/When/Then scenario)
- **fail_diagnosis_hint**: If `--e2e=true` and file is missing, the RCA executor did not load `rca-supplement.md`. Confirm supplement injection in SKILL.md Phase 1 invocation
- **depends_on_artifacts**: [docs/plans/*-rca-scenarios.yml]  # only relevant when args.e2e=true
- **forward_check**: monkey-test phase consumes `rca_scenarios` when `args.e2e=true`
```

- [ ] **Step 3: Verify markdown syntax**

```bash
head -50 examples/skills/debug-flow/criteria/rca.md
grep -c "^### RCA-" examples/skills/debug-flow/criteria/rca.md
```

Expected: `grep -c` returns 9 (RCA-01 through RCA-09). Markdown headings level consistent.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/debug-flow/criteria/rca.md
git commit -m "refactor(debug-flow): add RCA-09 for conditional rca_scenarios artifact"
```

---

### Task 9: Rewrite `criteria/fix-plan-review.md` (3 thin criteria)

**Files:**
- Modify: `examples/skills/debug-flow/criteria/fix-plan-review.md`

**Commit:** `refactor(debug-flow): thin fix-plan-review criteria to 3 (drop N-way voting residuals)`

- [ ] **Step 1: Read current `criteria/fix-plan-review.md`**

Read the file and confirm: 4 existing criteria (FIX-PLAN-REVIEW-01 through -04), frontmatter, `Observation Collection` section.

- [ ] **Step 2: Replace entire file with new content**

Overwrite `examples/skills/debug-flow/criteria/fix-plan-review.md` with the content below (frontmatter included — this supersedes any existing frontmatter):

```markdown
---
name: fix-plan-review
max_retries: 3
audit: required
---

## Criteria

### FIX-PLAN-REVIEW-01: Review artifact (findings.json) exists
- **severity**: quality
- **verify_type**: automated
- **verification**:
  1. Locate the review artifact file at `.belt/runs/*/review/findings.json`
  2. Verify the file exists
  3. Parse as JSON and confirm a `findings` array field is present
- **pass_condition**: File exists AND parses as valid JSON AND contains a `findings` array
- **fail_diagnosis_hint**: `/implementation-review` invocation interrupted or artifact path drift. Re-invoke the skill from the fix-plan-review phase
- **depends_on_artifacts**: [.belt/runs/*/review/findings.json]

### FIX-PLAN-REVIEW-02: Fix plan and RCA Report are consistent
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Cross-reference the RCA Report's Fix Strategy list with the fix plan document's task list
  2. Verify component names, file paths, and data types used in the fix plan document match the definitions in the RCA Report
  3. Verify that task completion conditions in the fix plan document do not deviate from Fix Strategy items
  4. Verify interfaces defined in the RCA Report (function signatures, API endpoints, etc.) are correctly referenced in the fix plan document
- **pass_condition**: Zero mismatches in component names / paths / types, zero deviations, zero reference inconsistencies
- **fail_diagnosis_hint**: Compare inconsistent entries side-by-side. If a review fix updated only one document, trace cause via `git log --oneline -- docs/plans/`
- **depends_on_artifacts**: [docs/plans/*-rca-report.md, docs/plans/*-fix-plan.md]

### FIX-PLAN-REVIEW-03: No unresolved blocker findings in review artifact
- **severity**: quality
- **verify_type**: automated
- **verification**:
  1. Parse `.belt/runs/*/review/findings.json`
  2. Filter findings where `severity == "blocker"`
  3. For each blocker finding, verify either (a) a resolution comment / fix commit is referenced in the fix plan, OR (b) the finding has been explicitly rejected by user triage
- **pass_condition**: Zero unresolved blocker findings
- **fail_diagnosis_hint**: User triage (accept/reject for each finding) is incomplete, or fix commits have not landed. Re-run the `/implementation-review` fix phase with accepted blocker findings
- **depends_on_artifacts**: [.belt/runs/*/review/findings.json, docs/plans/*-fix-plan.md]

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output. Record
quality/warning-level findings even for criteria that PASS. Observations
accumulate in the pipeline's audit trail.
```

**Thin check rationale (reference):** meta phase criteria limit themselves to (1) artifact existence / (2) cross-artifact integrity / (3) user-triage completion signals. Content audit of the review findings themselves is `/implementation-review` skill's responsibility.

- [ ] **Step 3: Verify**

```bash
grep -c "^### FIX-PLAN-REVIEW-" examples/skills/debug-flow/criteria/fix-plan-review.md
grep -n "consensus\|artifacts/reviews/" examples/skills/debug-flow/criteria/fix-plan-review.md
```

Expected: `grep -c` returns 3. Second grep returns zero matches.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/debug-flow/criteria/fix-plan-review.md
git commit -m "refactor(debug-flow): thin fix-plan-review criteria to 3 (drop N-way voting residuals)"
```

---

### Task 10: Create `criteria/monkey-test.md`

**Files:**
- Create: `examples/skills/debug-flow/criteria/monkey-test.md`

**Commit:** `refactor(debug-flow): add monkey-test criteria (bug-fix-specific baseline)`

- [ ] **Step 1: Read feature-dev's `criteria/monkey-test.md` as baseline**

Read `examples/skills/feature-dev/criteria/monkey-test.md` to understand the existing 4–6 criteria for scenarios replay, report format, coverage.

- [ ] **Step 2: Write `criteria/monkey-test.md` (new file)**

Content:

```markdown
---
name: monkey-test
max_retries: 3
audit: required
---

## Criteria

### MONKEY-TEST-01: Monkey test report file exists
- **severity**: blocker
- **verify_type**: automated
- **verification**: `Glob("docs/plans/*-monkey-test-report.md")`
- **pass_condition**: At least one match
- **fail_diagnosis_hint**: `/monkey-test` invocation did not produce the report. Confirm Phase 6 supplement was loaded and scenarios source `docs/plans/*-rca-scenarios.yml` was resolvable
- **depends_on_artifacts**: [docs/plans/*-monkey-test-report.md]

### MONKEY-TEST-02: Monkey test results JSON exists
- **severity**: blocker
- **verify_type**: automated
- **verification**: `Glob("docs/plans/*-monkey-test-results.json")` and parse as valid JSON
- **pass_condition**: File exists AND JSON parses successfully
- **depends_on_artifacts**: [docs/plans/*-monkey-test-results.json]

### MONKEY-TEST-03: Reproduction scenario replay is PASS
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. The first scenario in `rca-scenarios.yml` corresponds to the RCA Reproduction Test
  2. In `monkey-test-results.json`, confirm this scenario's result is PASS (previously FAIL per RCA-05)
- **pass_condition**: First scenario PASSes post-fix
- **fail_diagnosis_hint**: If PASS not achieved, the fix did not resolve the root cause. Re-examine Fix Strategy and `execute` phase output. If the scenario itself is malformed, correct the Given/When/Then in `rca-scenarios.yml` and re-run monkey-test
- **depends_on_artifacts**: [docs/plans/*-monkey-test-results.json, docs/plans/*-rca-scenarios.yml]

### MONKEY-TEST-04: All scenarios executed (no skip without rationale)
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Count scenarios in `rca-scenarios.yml`
  2. Count results in `monkey-test-results.json`
  3. Skipped scenarios must each have a rationale line in the report
- **pass_condition**: Scenario count matches results count, OR every skipped scenario has a documented rationale
- **depends_on_artifacts**: [docs/plans/*-rca-scenarios.yml, docs/plans/*-monkey-test-results.json, docs/plans/*-monkey-test-report.md]

### MONKEY-TEST-05: Report is committed to git
- **severity**: blocker
- **verify_type**: automated
- **verification**: Run `git status --porcelain -- docs/plans/*-monkey-test-*.{md,json}`; confirm zero output lines
- **pass_condition**: `git status --porcelain` returns empty for monkey-test artifacts

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output. Record
quality/warning-level findings even for criteria that PASS.
```

- [ ] **Step 3: Verify**

```bash
grep -c "^### MONKEY-TEST-" examples/skills/debug-flow/criteria/monkey-test.md
```

Expected: 5.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/debug-flow/criteria/monkey-test.md
git commit -m "refactor(debug-flow): add monkey-test criteria (bug-fix-specific baseline)"
```

---

### Task 11: Create `criteria/dogfood.md`

**Files:**
- Create: `examples/skills/debug-flow/criteria/dogfood.md`

**Commit:** `refactor(debug-flow): add dogfood criteria with directory form + CLI-only degradation`

- [ ] **Step 1: Read feature-dev's `criteria/dogfood.md` as baseline**

Read `examples/skills/feature-dev/criteria/dogfood.md`. Confirm the DOGFOOD-04 "evidence paragraph with screenshots/videos" requirement.

- [ ] **Step 2: Write `criteria/dogfood.md` (new file)**

```markdown
---
name: dogfood
max_retries: 3
audit: required
---

## Criteria

### DOGFOOD-01: Dogfood report file exists (directory form)
- **severity**: blocker
- **verify_type**: automated
- **verification**: `Glob("docs/plans/*-dogfood-report/report.md")`
- **pass_condition**: At least one match (directory `docs/plans/<topic>-dogfood-report/` must contain `report.md`)
- **fail_diagnosis_hint**: `/dogfood` default output is `./dogfood-output/`; `dogfood-supplement.md` must override to `docs/plans/<topic>-dogfood-report/`. Confirm Phase 7 supplement was loaded
- **depends_on_artifacts**: [docs/plans/*-dogfood-report/report.md]

### DOGFOOD-02: Root Cause mechanism does not re-emerge in exploration
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the Root Cause section of `*-rca-report.md`, extract the specific mechanism description
  2. In dogfood report, confirm an explicit statement that the mechanism was re-verified (e.g., "after fix, the XXX condition no longer triggers")
  3. If the exploration uncovered the same mechanism re-manifesting in a different code path, flag FAIL
- **pass_condition**: Re-verification statement present AND no re-emergence of the mechanism
- **fail_diagnosis_hint**: Fix is incomplete or has asymmetric coverage. Re-examine RCA Symmetry Check output and Fix Strategy

### DOGFOOD-03: Fix scope exploration documented
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. The dogfood report explicitly lists the Fix Impact Scope (derived from `fix_plan_doc`)
  2. Exploration covered Impact Scope areas + Symmetry pairs (from RCA-08)
- **pass_condition**: Impact Scope listed AND exploration coverage matches

### DOGFOOD-04: Evidence (screenshots / videos OR CLI-only rationale)
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Check `docs/plans/<topic>-dogfood-report/screenshots/` or `.../videos/` for at least one evidence file
  2. If Impact Scope contains zero UI files (CLI / API / backend-only fix), accept a rationale paragraph in `report.md` explaining CLI-only exploration scope (per spec "UI なし bug fix の dogfood graceful degradation")
- **pass_condition**: ≥1 evidence file under screenshots/ or videos/, OR rationale paragraph present for CLI-only fixes
- **fail_diagnosis_hint**: For UI-touching fixes, `/dogfood` should emit screenshots by default. For CLI-only fixes, ensure the rationale paragraph was added per supplement instructions
- **depends_on_artifacts**: [docs/plans/*-dogfood-report/]

### DOGFOOD-05: Report is committed to git
- **severity**: blocker
- **verify_type**: automated
- **verification**: `git status --porcelain -- docs/plans/*-dogfood-report/` returns empty
- **pass_condition**: zero uncommitted changes under the dogfood report directory

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
```

- [ ] **Step 3: Verify**

```bash
grep -c "^### DOGFOOD-" examples/skills/debug-flow/criteria/dogfood.md
```

Expected: 5.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/debug-flow/criteria/dogfood.md
git commit -m "refactor(debug-flow): add dogfood criteria with directory form + CLI-only degradation"
```

---

### Task 12: Create `criteria/integrate.md`

**Files:**
- Create: `examples/skills/debug-flow/criteria/integrate.md`

**Commit:** `refactor(debug-flow): add integrate criteria`

- [ ] **Step 1: Read feature-dev's `criteria/integrate.md` as baseline**

Read `examples/skills/feature-dev/criteria/integrate.md` for the baseline structure.

- [ ] **Step 2: Write `criteria/integrate.md` (new file)**

```markdown
---
name: integrate
max_retries: 3
audit: required
---

## Criteria

### INTEGRATE-01: Integration method was chosen and executed
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. The SKILL.md Phase 8 A/B prompt was presented to the user
  2. Either `wt merge` (option A) or `gh pr create` (option B) was executed
  3. Execution logs (or git state) reflect the chosen method
- **pass_condition**: One of the two methods was executed per user choice

### INTEGRATE-02: All pre-merge checks pass
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. `cargo test` (if Rust changes) exit 0
  2. `cargo clippy --workspace -- -D warnings` exit 0
  3. `cargo fmt --check` exit 0 for modified packages
  4. belt lint for any modified pipeline.yml files exit 0
- **pass_condition**: All applicable checks exit 0

### INTEGRATE-03: Reproduction test PASSes on integrated branch
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. After `wt merge` to main / after PR creation, re-run the test identified in RCA-05 Reproduction Test
  2. Confirm it now PASSes (previously FAILed per RCA-05)
- **pass_condition**: Reproduction test PASSes on the integrated branch
- **fail_diagnosis_hint**: Merge introduced a regression, or reproduction test expectations drifted. Review integration diff

## Observation Collection

The phase-auditor MUST include `observations[]` in its verdict output.
```

- [ ] **Step 3: Verify**

```bash
grep -c "^### INTEGRATE-" examples/skills/debug-flow/criteria/integrate.md
```

Expected: 3.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/debug-flow/criteria/integrate.md
git commit -m "refactor(debug-flow): add integrate criteria"
```

---

### Task 13: Create `references/path-convention.md`

**Files:**
- Create: `examples/skills/debug-flow/references/path-convention.md`

**Commit:** `refactor(debug-flow): add path-convention supplement`

- [ ] **Step 1: Read feature-dev's `references/path-convention.md` as baseline**

Read `examples/skills/feature-dev/references/path-convention.md`.

- [ ] **Step 2: Write `references/path-convention.md`**

```markdown
# Debug Flow Path Convention

**Purpose:** SSOT for file naming and directory layout under `docs/plans/` for a debug-flow run.

## Base path

All debug-flow run outputs live under:

```
docs/plans/YYYY-MM-DD-<topic>-*
```

## Topic slug rules

- Characters: `[a-z0-9-]` (lowercase, digits, hyphens only)
- Length: 3–48 characters
- Separator: hyphen `-`
- Collision handling: on same-day topic collision, append numeric suffix `-N` (e.g., `2026-04-20-login-bug-2`)

## Branch name convention

- Format: `bugfix/<YYYY-MM-DD-topic>`
- Same topic slug as used in the paths
- Branch must be a worktree-linked branch per `worktrunk` conventions

## Artifact path table

| Artifact (logical name) | Path |
|---|---|
| `rca_report` | `docs/plans/YYYY-MM-DD-<topic>-rca-report.md` |
| `rca_scenarios` (when `--e2e`) | `docs/plans/YYYY-MM-DD-<topic>-rca-scenarios.yml` |
| `fix_plan_doc` | `docs/plans/YYYY-MM-DD-<topic>-fix-plan.md` |
| `monkey_test_report` | `docs/plans/YYYY-MM-DD-<topic>-monkey-test-report.md` |
| `monkey_test_results` | `docs/plans/YYYY-MM-DD-<topic>-monkey-test-results.json` |
| `dogfood_report` | `docs/plans/YYYY-MM-DD-<topic>-dogfood-report/report.md` (directory form) |
| `dogfood_screenshots` | `docs/plans/YYYY-MM-DD-<topic>-dogfood-report/screenshots/` |
| `dogfood_videos` | `docs/plans/YYYY-MM-DD-<topic>-dogfood-report/videos/` |

## Glob resolution

belt-agent glob resolution for `docs/plans/*-<suffix>` patterns returns files matching the glob; on ambiguity (multiple matches), the most recently modified file wins (mtime DESC). `monkey-test-supplement.md` documents this for scenarios resolution.
```

- [ ] **Step 3: Verify**

```bash
ls examples/skills/debug-flow/references/path-convention.md
head -5 examples/skills/debug-flow/references/path-convention.md
```

Expected: file exists, starts with `# Debug Flow Path Convention`.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/debug-flow/references/path-convention.md
git commit -m "refactor(debug-flow): add path-convention supplement"
```

---

### Task 14: Create `references/rca-supplement.md`

**Files:**
- Create: `examples/skills/debug-flow/references/rca-supplement.md`

**Commit:** `refactor(debug-flow): add rca-supplement`

- [ ] **Step 1: Read current `/systematic-debugging` SKILL.md (briefly)**

Confirm the skill exists and accepts no args. The supplement overrides output path and requires specific RCA Report structure.

- [ ] **Step 2: Write `references/rca-supplement.md`**

```markdown
# RCA Supplement (Phase 1 override for `/systematic-debugging`)

**Invoked by:** `examples/skills/debug-flow/SKILL.md` Phase 1 (INVOKE 1 = Read this file; INVOKE 2 = `/systematic-debugging`).

## Output path override

Write the RCA Report to:

```
docs/plans/YYYY-MM-DD-<topic>-rca-report.md
```

Path convention: see `./path-convention.md`.

## Required RCA Report sections

The report MUST contain these five top-level sections (`##` level):

1. `## Symptom` — User-observable symptom, reproduction steps, error messages
2. `## Investigation Record` — Four subsections:
   1. `### Code Flow Trace` — call chains (file path + function name pairs)
   2. `### Architecture Context` — relevant patterns, conventions, implicit rules
   3. `### Impact Scope` — affected files / modules (paths must exist per RCA-03)
   4. `### Symmetry Check` — whether change target has paired paths (required per RCA-08)
3. `## Root Cause` — file:line location + mechanism explanation (per RCA-06)
4. `## Reproduction Test` — test file path + assertion; test MUST currently FAIL (per RCA-05)
5. `## Fix Strategy` — ordered list of remediation steps

An additional section `## Excluded Hypotheses` (or equivalent within Investigation Record) MUST record at least one alternative root cause, its verification method, and rejection reason (per RCA-04).

## Parallel exploration order

Orchestrator dispatches exploration subagents in parallel, then synthesizes:

1. `code-explorer` — entry-point tracing and data flow
2. `code-architect` — architecture patterns and implicit contracts
3. `impact-analyzer` — reverse dependencies and shared state

**After** subagent results return, the orchestrator reconstructs the root cause **itself** (do NOT forward broad research verbatim into Reproduction Test / Fix Strategy). See debug-flow SKILL.md Red Flag "Never delegate root cause synthesis to subagents."

## Reproduction test requirement

Write a failing test that captures the bug mechanism (RCA-05 blocker). The test must:
- Be placed in an appropriate test directory for the project (see `tests/` / `spec/` conventions)
- Currently FAIL when run (before fix)
- Transition to PASS only after `execute` phase applies the fix

## `--e2e` additional output

When `args.e2e=true`, additionally produce:

```
docs/plans/YYYY-MM-DD-<topic>-rca-scenarios.yml
```

Content: Given/When/Then YAML with at least one scenario. The first scenario MUST correspond to the RCA Reproduction Test (see `monkey-test-supplement.md`).

Example format:

```yaml
scenarios:
  - name: "Reproduce login 500 error"
    given: "The user has an expired session cookie"
    when: "The user navigates to /dashboard"
    then: "The server returns 302 redirect to /login (not 500)"
  - name: "Regression: valid session still works"
    given: "The user has a fresh session cookie"
    when: "The user navigates to /dashboard"
    then: "The dashboard page renders successfully"
```
```

- [ ] **Step 3: Verify + commit**

```bash
ls examples/skills/debug-flow/references/rca-supplement.md
head -5 examples/skills/debug-flow/references/rca-supplement.md
git add examples/skills/debug-flow/references/rca-supplement.md
git commit -m "refactor(debug-flow): add rca-supplement"
```

---

### Task 15: Create `references/fix-plan-supplement.md`

**Files:**
- Create: `examples/skills/debug-flow/references/fix-plan-supplement.md`

**Commit:** `refactor(debug-flow): add fix-plan-supplement`

- [ ] **Step 1: Write `references/fix-plan-supplement.md`**

```markdown
# Fix Plan Supplement (Phase 2 override for `/writing-plans`)

**Invoked by:** `examples/skills/debug-flow/SKILL.md` Phase 2 (INVOKE 1 = Read this file; INVOKE 2 = `/writing-plans`).

## Output path override

Write the fix plan to:

```
docs/plans/YYYY-MM-DD-<topic>-fix-plan.md
```

Path convention: see `./path-convention.md`.

## Consumed inputs

- `docs/plans/YYYY-MM-DD-<topic>-rca-report.md` (produced by Phase 1)

Read this file in full before authoring the fix plan. Extract the `## Fix Strategy` section as the driver of task decomposition.

## Mandatory traceability

Every task in the fix plan MUST map to at least one Fix Strategy item in the RCA Report (blocker per FIX-PLAN-02). Include a task-to-Fix Strategy mapping table at the top of the fix plan document. Example:

| Task # | Fix Strategy ID |
|---|---|
| 1 | FS-1 |
| 2 | FS-2 |
| 3 | FS-2 |

## Task granularity

- Each task MUST have ≤10 steps (per FIX-PLAN-03)
- Each task MUST span <3 modules (per FIX-PLAN-03)
- If a task exceeds either limit, split it

## Given/When/Then test cases

Every task MUST include at least one test case in Given/When/Then format (per FIX-PLAN-05). Example:

```markdown
**Test case:**
- Given: user has expired session
- When: GET /dashboard
- Then: 302 redirect to /login
```

Then clauses MUST contain verifiable expected values (numeric thresholds, pattern-matchable assertions, boolean state).

## Verifiable completion conditions

Every task MUST have a completion condition expressible as:
- File existence check (e.g., "file X exists at path Y"), OR
- Command output numeric comparison (e.g., "exit code == 0"), OR
- Pattern match (e.g., "grep returns ≥1 line"), OR
- Boolean state assertion (e.g., "feature flag is enabled")

Reject subjective terms (「適切に」「十分に」「correct」).

## RCA artifacts reference

- Include a reference line to the consumed RCA Report at the top of the fix plan (e.g., "Based on: `docs/plans/YYYY-MM-DD-<topic>-rca-report.md`")
- If `--e2e` and `rca-scenarios.yml` exists, reference it so monkey-test phase can extend the scenarios list with fix-specific Given/When/Then entries
```

- [ ] **Step 2: Verify + commit**

```bash
ls examples/skills/debug-flow/references/fix-plan-supplement.md
git add examples/skills/debug-flow/references/fix-plan-supplement.md
git commit -m "refactor(debug-flow): add fix-plan-supplement"
```

---

### Task 16: Create `references/monkey-test-supplement.md`

**Files:**
- Create: `examples/skills/debug-flow/references/monkey-test-supplement.md`

**Commit:** `refactor(debug-flow): add monkey-test-supplement`

- [ ] **Step 1: Write `references/monkey-test-supplement.md`**

```markdown
# Monkey Test Supplement (Phase 6 override for `/monkey-test`)

**Invoked by:** `examples/skills/debug-flow/SKILL.md` Phase 6 (INVOKE 1 = Read this file; INVOKE 2 = `/monkey-test`). Only runs when `args.e2e=true`.

## Scenarios source

`/monkey-test` defaults to `docs/features/*/scenarios.yml` (feature-dev). In debug-flow, override to:

```
docs/plans/*-rca-scenarios.yml
```

## Glob collision handling

If multiple files match the `docs/plans/*-rca-scenarios.yml` glob (concurrent runs / multiple bugs same day), select the **most recently modified** (mtime DESC).

## First scenario requirement

The first scenario in `rca-scenarios.yml` MUST correspond to the RCA Reproduction Test (from Phase 1). After fix, this scenario is expected to PASS (previously FAIL per RCA-05). `criteria/monkey-test.md` MONKEY-TEST-03 verifies this transition.

## Regression scenarios

Supplement subsequent scenarios cover:
- Symmetry pair validation (per RCA-08): if the RCA identified paired paths, add scenarios exercising them
- Impact Scope regression: scenarios exercising adjacent functionality that shares code paths with the fix

## Output paths

Produce:

```
docs/plans/YYYY-MM-DD-<topic>-monkey-test-report.md
docs/plans/YYYY-MM-DD-<topic>-monkey-test-results.json
```

Report structure: scenario list, per-scenario result (PASS/FAIL/SKIP with rationale), summary count, flaky detection notes (optional).
```

- [ ] **Step 2: Verify + commit**

```bash
ls examples/skills/debug-flow/references/monkey-test-supplement.md
git add examples/skills/debug-flow/references/monkey-test-supplement.md
git commit -m "refactor(debug-flow): add monkey-test-supplement"
```

---

### Task 17: Create `references/dogfood-supplement.md`

**Files:**
- Create: `examples/skills/debug-flow/references/dogfood-supplement.md`

**Commit:** `refactor(debug-flow): add dogfood-supplement with CLI-only degradation`

- [ ] **Step 1: Write `references/dogfood-supplement.md`**

```markdown
# Dogfood Supplement (Phase 7 override for `/dogfood`)

**Invoked by:** `examples/skills/debug-flow/SKILL.md` Phase 7 (INVOKE 1 = Read this file; INVOKE 2 = `/dogfood`). Only runs when `args.e2e=true`.

## Output path override

`/dogfood` default output is `./dogfood-output/`. Override to:

```
docs/plans/YYYY-MM-DD-<topic>-dogfood-report/
  ├── report.md
  ├── screenshots/
  └── videos/
```

Path convention: see `./path-convention.md`.

## Exploration scope

Prioritize:

1. **Impact Scope areas** from `fix_plan_doc` — specifically the files and modules modified in the fix
2. **Symmetry pairs** from RCA-08 — paired paths that may exhibit the same mechanism
3. **Root Cause mechanism re-emergence** — verify the specific mechanism described in the RCA `## Root Cause` section does not recur in adjacent code paths

## Priority: Root Cause mechanism re-verification

Include in the report an explicit statement of the form:

> After the fix, the <mechanism description from RCA Root Cause> condition no longer triggers. Verified by: <specific exploration path / inputs>.

This satisfies `criteria/dogfood.md` DOGFOOD-02.

## CLI-only graceful degradation (UI-free bug fix)

When the Impact Scope contains **zero UI files** (CLI / API / backend-only fix):

1. Substitute visual exploration with:
   - CLI output capture (stdout / stderr)
   - API response inspection (JSON / headers)
   - Log file inspection
   - DB state queries
2. DOGFOOD-04 evidence requirement is satisfied by a **rationale paragraph** in `report.md`:

   > Impact Scope contains no UI files (<list affected paths>). Exploration is CLI-only; evidence is captured as CLI output / API response / log excerpts in this report.

3. Still produce the `screenshots/` and `videos/` directories (empty is acceptable) to keep the artifact structure consistent with DOGFOOD-01.

## Minimum report content

- Fix Impact Scope listing (copy from `fix_plan_doc`)
- Exploration coverage map (which Impact Scope items + Symmetry pairs were explored)
- Root Cause mechanism re-verification statement
- Evidence index (screenshots/videos listing, OR CLI-only rationale paragraph)
- Issue summary (new issues discovered, if any, with severity)
```

- [ ] **Step 2: Verify + commit**

```bash
ls examples/skills/debug-flow/references/dogfood-supplement.md
git add examples/skills/debug-flow/references/dogfood-supplement.md
git commit -m "refactor(debug-flow): add dogfood-supplement with CLI-only degradation"
```

---

### Task 18: Create `references/worktrunk-supplement.md`

**Files:**
- Create: `examples/skills/debug-flow/references/worktrunk-supplement.md`

**Commit:** `refactor(debug-flow): add worktrunk-supplement (feature-dev parity)`

- [ ] **Step 1: Read feature-dev's `worktrunk-supplement.md` as baseline**

Read `examples/skills/feature-dev/references/worktrunk-supplement.md` for the A/B choice pattern.

- [ ] **Step 2: Write `references/worktrunk-supplement.md`**

```markdown
# Worktrunk Supplement (Phase 8 override for `/worktrunk`)

**Invoked by:** `examples/skills/debug-flow/SKILL.md` Phase 8 (INVOKE 1 = Read this file; INVOKE 2 = prompt user for mode; INVOKE 3 = `/worktrunk`).

## A/B choice prompt

After all review / monkey-test / dogfood phases pass, prompt the user:

> Integration mode:
>   A. `wt merge` — Merge bugfix branch to main locally (worktree-first workflow)
>   B. `gh pr create` — Open a PR on GitHub for remote review
>
> Which? (A / B)

Default: no default; always require explicit user choice. This is a debug-flow Red Flag ("Never bypass the Phase 8 A/B choice").

## Branch naming convention

Debug-flow branches follow: `bugfix/YYYY-MM-DD-<topic>` (see `./path-convention.md`).

## Pre-merge checks

Before invoking `/worktrunk`:

1. `cargo test` (or project-appropriate test command) exit 0
2. `cargo clippy --workspace -- -D warnings` exit 0 (if Rust project)
3. `cargo fmt --check` exit 0 for modified packages
4. belt lint exit 0 for any modified pipeline.yml files
5. Reproduction test (from RCA-05) PASSes on this branch (INTEGRATE-03 blocker)

If any check fails, abort Phase 8 and report to user — do NOT merge.

## Post-merge verification

After `wt merge`:
- Re-run reproduction test on main branch to confirm PASS
- Confirm `git log` shows the fix commits merged

After `gh pr create`:
- Confirm PR URL is reachable
- No further action in debug-flow — user follows up externally

## Commit message convention for the fix

Fix commits (from `execute` phase) should follow:

```
fix(<scope>): <short description of bug fix>
```

Where `<scope>` is derived from the RCA Impact Scope (primary module). Example: `fix(auth): redirect expired session cookies to /login instead of 500`.
```

- [ ] **Step 2: Verify + commit**

```bash
ls examples/skills/debug-flow/references/worktrunk-supplement.md
git add examples/skills/debug-flow/references/worktrunk-supplement.md
git commit -m "refactor(debug-flow): add worktrunk-supplement (feature-dev parity)"
```

---

### Task 19: Rewrite `pipeline.yml` to target shape

**Files:**
- Modify: `examples/skills/debug-flow/pipeline.yml`

**Commit:** `refactor(debug-flow): rewrite pipeline.yml to 8-phase feature-dev parity shape`

- [ ] **Step 1: Backup + verify current**

```bash
cat examples/skills/debug-flow/pipeline.yml
```

Note current content for reference only.

- [ ] **Step 2: Replace entire file content**

Write the exact content below to `examples/skills/debug-flow/pipeline.yml` (overwrite):

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

- [ ] **Step 3: Verify belt lint passes**

```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
cargo run --bin belt -- lint examples/skills/debug-flow/pipeline.yml
```

Expected: exit 0, no errors. If errors, inspect and fix — do NOT `--no-verify` commit.

- [ ] **Step 4: Verify grep hygiene (canonical patterns)**

```bash
grep -nE 'iterations|swarm|--smoke' examples/skills/debug-flow/pipeline.yml
grep -nE '(^|[[:space:]])(ui)[=:]' examples/skills/debug-flow/pipeline.yml
```

Expected: zero matches from both.

- [ ] **Step 5: Commit**

```bash
git add examples/skills/debug-flow/pipeline.yml
git commit -m "refactor(debug-flow): rewrite pipeline.yml to 8-phase feature-dev parity shape"
```

---

### Task 20: Rewrite `SKILL.md` to feature-dev aesthetic

**Files:**
- Modify: `examples/skills/debug-flow/SKILL.md`

**Commit:** `refactor(debug-flow): rewrite SKILL.md for 8-phase monkey/dogfood flow`

- [ ] **Step 1: Read current `SKILL.md`**

Review structure and confirm sections to delete: "Dispatch Rules", "Coordinator Discipline", "Evidence Plan", any validate / phase-auditor section.

- [ ] **Step 2: Replace entire SKILL.md body**

Write the following content (overwrite):

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

| Arg | Type | Default | Description |
|---|---|---|---|
| e2e | bool | false | Enable monkey-test and dogfood phases |
| codex | bool | false | Enable Codex parallel review in review phases |

## Phase-Specific Invocation Rules

### Phase 1: rca

- **INVOKE 1**: Read `./references/rca-supplement.md` into context.
- **INVOKE 2**: Skill tool `/systematic-debugging`.
- The supplement enforces RCA Report 5 sections, Symmetry Check, Reproduction Test FAIL, parallel exploration order, and (when `--e2e`) `rca-scenarios.yml` produce.

### Phase 2: fix-plan

- **INVOKE 1**: Read `./references/fix-plan-supplement.md`.
- **INVOKE 2**: Skill tool `/writing-plans`.
- The supplement enforces RCA Fix Strategy → task traceability, Given/When/Then test cases, verifiable completion conditions, and task granularity.

### Phase 3: fix-plan-review

- **INVOKE**: Skill tool `/implementation-review` with `codex` passed through.
- No supplement required; the skill is self-contained.

### Phase 4: execute

- **INVOKE**: Skill tool `/subagent-driven-development`.
- Orchestrator must reconstruct fix plan tasks into self-contained implementation specs before dispatching `feature-implementer` subagents. Do not forward RCA / Fix Plan excerpts verbatim.

### Phase 5: code-review

- **INVOKE**: Skill tool `/code-review` with `codex` passed through.
- On fix commits, Phase 4 validate is re-verified per belt regate semantics. `max_retries: 3` limits the review-fix loop.

### Phase 6: monkey-test (when `--e2e`)

- **INVOKE 1**: Read `./references/monkey-test-supplement.md`.
- **INVOKE 2**: Skill tool `/monkey-test`.
- The supplement points scenarios source at `docs/plans/*-rca-scenarios.yml`, requires the first scenario to verify the RCA Reproduction Test now PASSes, and documents glob collision resolution.

### Phase 7: dogfood (when `--e2e`)

- **INVOKE 1**: Read `./references/dogfood-supplement.md`.
- **INVOKE 2**: Skill tool `/dogfood`.
- The supplement focuses exploration on fix Impact Scope + Symmetry pairs, flags Root Cause mechanism re-emergence, and provides CLI-only graceful degradation for UI-free fixes.

### Phase 8: integrate

- **INVOKE 1**: Read `./references/worktrunk-supplement.md`.
- **INVOKE 2**: Prompt user for mode (A: `wt merge` / B: `gh pr create`).
- **INVOKE 3**: Execute via `/worktrunk` per user's choice.

## Red Flags

- **Never skip Phase 1 (rca)**: root cause must precede fix. "Fix first" is anti-pattern.
- **Never skip Phase 1 / 2 / 6 / 7 / 8 の supplement load**: debug-flow 固有 override が inject されず drift 発生.
- **Never delegate root cause synthesis to subagents**: parallel exploration results は orchestrator が再構築.
- **Never proceed without a failing reproduction test**: RCA-05 blocker.
- **Never filter or omit review findings**: `/code-review`, `/implementation-review` の triage は user 責務.
- **Never bypass the Phase 8 A/B choice**: merge-vs-PR は user 決定.
- **Never hand-edit files under `docs/plans/<topic>-*`**: phase-produced; manual edits break belt の phase-start mtime filter.
- **Never modify the consumed global skills**: override は `references/*-supplement.md` 経由のみ.

## References

- `./references/path-convention.md` — `docs/plans/YYYY-MM-DD-<topic>-*` 命名 SSOT
- `./references/rca-supplement.md` — Phase 1 override
- `./references/fix-plan-supplement.md` — Phase 2 override
- `./references/monkey-test-supplement.md` — Phase 6 override
- `./references/dogfood-supplement.md` — Phase 7 override and CLI-only degradation
- `./references/worktrunk-supplement.md` — Phase 8 A/B choice logic
```

- [ ] **Step 3: Verify markdown syntax + grep hygiene**

```bash
head -20 examples/skills/debug-flow/SKILL.md
grep -c "^### Phase" examples/skills/debug-flow/SKILL.md
grep -n "iterations\|swarm\|--ui\|--smoke" examples/skills/debug-flow/SKILL.md
```

Expected: 8 phase sections, zero legacy-arg matches.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/debug-flow/SKILL.md
git commit -m "refactor(debug-flow): rewrite SKILL.md for 8-phase monkey/dogfood flow"
```

---

### Task 21: Delete dead-letter references

**Files:**
- Delete: `examples/skills/debug-flow/references/evidence-plan-protocol.md`
- Delete: `examples/skills/debug-flow/references/fix-dispatch-strategy.md`

**Commit:** `refactor(debug-flow): drop dead-letter references (evidence-plan-protocol, fix-dispatch-strategy)`

- [ ] **Step 1: Verify files exist before deletion**

```bash
ls examples/skills/debug-flow/references/evidence-plan-protocol.md
ls examples/skills/debug-flow/references/fix-dispatch-strategy.md
```

Expected: both exist.

- [ ] **Step 2: Delete**

```bash
git rm examples/skills/debug-flow/references/evidence-plan-protocol.md
git rm examples/skills/debug-flow/references/fix-dispatch-strategy.md
```

- [ ] **Step 3: Verify deletion + hygiene**

```bash
ls examples/skills/debug-flow/references/
grep -rn "evidence-plan-protocol\|fix-dispatch-strategy" examples/skills/debug-flow/
```

Expected: listing shows 6 supplements (`path-convention`, `rca-supplement`, `fix-plan-supplement`, `monkey-test-supplement`, `dogfood-supplement`, `worktrunk-supplement`). Second grep returns zero matches inside `examples/skills/debug-flow/`.

- [ ] **Step 4: Commit**

```bash
git commit -m "refactor(debug-flow): drop dead-letter references (evidence-plan-protocol, fix-dispatch-strategy)"
```

---

### Task 22: Remove `fix-dispatch-strategy.md` reference from `examples/references/audit-protocol.md`

**Files:**
- Modify: `examples/references/audit-protocol.md`

**Commit:** `refactor(references): drop fix-dispatch-strategy reference from audit-protocol`

- [ ] **Step 1: Locate the offending reference**

```bash
grep -n "fix-dispatch-strategy" examples/references/audit-protocol.md
```

Expected: one or more line numbers returned. Spec identified line 92 specifically.

- [ ] **Step 2: Read the context around the reference**

Read `examples/references/audit-protocol.md` around the identified line to understand the surrounding sentence / bullet.

- [ ] **Step 3: Remove or replace the reference**

Strategies (choose based on context):
- If the reference is a bullet inside a list → remove the entire bullet
- If the reference is a parenthetical aside → remove only the parenthetical
- If the reference anchors a section → remove the sentence or replace with a neutral alternative (e.g., "see individual skill SKILL.md Fix sections")

Apply the edit. Verify the surrounding prose still reads coherently.

- [ ] **Step 4: Verify**

```bash
grep -rn "fix-dispatch-strategy" examples/references/
grep -rn "fix-dispatch-strategy" . --include="*.md" --exclude-dir=.git 2>/dev/null
```

Expected: zero matches in `examples/references/`; zero matches anywhere in the tree (docs/plans/ plan files may mention it as "deleted" — that's OK).

- [ ] **Step 5: Commit**

```bash
git add examples/references/audit-protocol.md
git commit -m "refactor(references): drop fix-dispatch-strategy reference from audit-protocol"
```

---

### Task 23: (conditional) Delete shared `criteria/smoke-test.md` and `criteria/test-review.md` if zero external references

**Files:**
- (Conditional) Delete: `examples/criteria/smoke-test.md`
- (Conditional) Delete: `examples/criteria/test-review.md`

**Commit (if executed):** `chore: drop unused shared smoke-test / test-review criteria`

- [ ] **Step 1: Confirm Task 7 Pre-work A.2 result**

Check the Task 7 investigation notes. If `smoke-test.md` / `test-review.md` had external references from other skills, **skip this task**.

- [ ] **Step 2 (if zero external references): Delete and verify**

```bash
git rm examples/criteria/smoke-test.md
git rm examples/criteria/test-review.md

# Verify nothing else references them
grep -rn "criteria/smoke-test\.md\|criteria/test-review\.md" examples/ 2>/dev/null | grep -v debug-flow
```

Expected: zero output from grep.

- [ ] **Step 3 (if deleted): Commit**

```bash
git commit -m "chore: drop unused shared smoke-test / test-review criteria"
```

If external references existed (Task 7 A.2 reported non-zero), skip this task entirely and note in the final summary.

---

### Task 24: Create `debug_flow_refresh.rs` integration test

**Files:**
- Create: `crates/belt-core/tests/debug_flow_refresh.rs`

**Commit:** `test(belt-core): add debug_flow_refresh integration test`

- [ ] **Step 1: Read existing `review_skills_refresh.rs` for structural template**

Read `crates/belt-core/tests/review_skills_refresh.rs`. Note helpers (`repo_root`, `pipeline_path`, etc.) and assertion patterns.

- [ ] **Step 2: Write `debug_flow_refresh.rs`**

Create `crates/belt-core/tests/debug_flow_refresh.rs` with:

```rust
//! Integration tests for the refreshed /debug-flow pipeline.
//!
//! Shape contract (spec docs/specs/2026-04-15-debug-flow-refresh-design.md):
//! - args = { e2e: bool, codex: bool } only (iterations / swarm / ui / smoke removed)
//! - 8 phases: rca → fix-plan → fix-plan-review → execute → code-review →
//!             monkey-test → dogfood → integrate
//! - All phases use skill: invoke (no pipeline:)
//! - Review phases (fix-plan-review, code-review) pass codex
//! - code-review has regate: [execute]; no other phase has regate
//! - Supplement injection for 5 phases (rca, fix-plan, monkey-test, dogfood, integrate)
//! - criteria skill-local (6 files) + shared (execute.md, code-review.md)
//! - rca_scenarios.when = "args.e2e" (type-level, not just YAML text)
//! - Dead letter references removed.

use std::collections::HashMap;
use std::path::PathBuf;

use belt_core::{
    expander::expand_pipeline,
    model::{ArgType, Invoker, Pipeline},
    parser::parse_pipeline,
};

fn repo_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path
}

fn debug_flow_dir() -> PathBuf {
    repo_root().join("examples/skills/debug-flow")
}

fn debug_flow_pipeline_path() -> PathBuf {
    debug_flow_dir().join("pipeline.yml")
}

fn debug_flow_pipeline() -> Pipeline {
    parse_pipeline(&debug_flow_pipeline_path()).expect("debug-flow pipeline.yml must parse")
}

const EXPECTED_PHASES: &[&str] = &[
    "rca",
    "fix-plan",
    "fix-plan-review",
    "execute",
    "code-review",
    "monkey-test",
    "dogfood",
    "integrate",
];

#[test]
fn args_are_e2e_and_codex_only() {
    let pipeline = debug_flow_pipeline();
    let mut keys: Vec<&str> = pipeline.args.keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(keys, vec!["codex", "e2e"]);

    for (name, def) in &pipeline.args {
        assert_eq!(def.arg_type, ArgType::Bool, "arg '{name}' must be bool");
    }
}

#[test]
fn no_legacy_args() {
    let pipeline = debug_flow_pipeline();
    for legacy in ["iterations", "swarm", "ui", "smoke"] {
        assert!(
            !pipeline.args.contains_key(legacy),
            "legacy arg '{legacy}' must be removed"
        );
    }
}

#[test]
fn phase_count_and_order() {
    let pipeline = debug_flow_pipeline();
    let actual: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(actual, EXPECTED_PHASES);
}

#[test]
fn all_phases_use_skill_invoke() {
    let pipeline = debug_flow_pipeline();
    for phase in &pipeline.phases {
        let invoker = phase
            .invoke
            .as_ref()
            .unwrap_or_else(|| panic!("phase '{}' must have invoke", phase.id));
        match invoker {
            Invoker::Skill { skill, .. } => {
                assert!(
                    skill.starts_with('/'),
                    "phase '{}' skill must start with '/', got '{skill}'",
                    phase.id
                );
            }
            _ => panic!(
                "phase '{}' must use Invoker::Skill variant, got {invoker:?}",
                phase.id
            ),
        }
    }
}

#[test]
fn review_phases_pass_codex_only() {
    let pipeline = debug_flow_pipeline();
    for phase in &pipeline.phases {
        if !matches!(phase.id.as_str(), "fix-plan-review" | "code-review") {
            continue;
        }
        let Some(Invoker::Skill { args, .. }) = phase.invoke.as_ref() else {
            panic!("phase '{}' must use Invoker::Skill variant", phase.id);
        };
        let mut keys: Vec<&str> = args.keys().map(String::as_str).collect();
        keys.sort();
        assert_eq!(keys, vec!["codex"], "phase '{}' must pass only codex", phase.id);
        assert_eq!(
            args.get("codex").and_then(|v| v.as_str()),
            Some("args.codex"),
            "phase '{}' codex must passthrough from args",
            phase.id
        );
    }
}

#[test]
fn only_code_review_has_regate() {
    let pipeline = debug_flow_pipeline();
    for phase in &pipeline.phases {
        if phase.id == "code-review" {
            assert_eq!(
                phase.regate,
                vec!["execute".to_string()],
                "code-review must have regate == [\"execute\"]"
            );
        } else {
            assert!(
                phase.regate.is_empty(),
                "phase '{}' must have empty regate, got {:?}",
                phase.id,
                phase.regate
            );
        }
    }
}

#[test]
fn rca_scenarios_when_is_typed() {
    let pipeline = debug_flow_pipeline();
    let rca = pipeline
        .phases
        .iter()
        .find(|p| p.id == "rca")
        .expect("rca phase must exist");
    let scenarios = rca
        .produces
        .iter()
        .find(|a| a.name == "rca_scenarios")
        .expect("rca_scenarios artifact must exist");
    assert_eq!(
        scenarios.when,
        Some("args.e2e".to_string()),
        "rca_scenarios.when must parse as a typed field (not silent-dropped)"
    );
}

#[test]
fn rca_scenarios_filtered_when_e2e_false() {
    let expanded = expand_pipeline(&debug_flow_pipeline_path()).expect("expansion must succeed");
    let mut args_false: HashMap<String, serde_json::Value> = HashMap::new();
    args_false.insert("e2e".to_string(), serde_json::Value::Bool(false));
    let active = belt_core::view::active_produces(&expanded[0], &args_false);
    let names: Vec<&str> = active.iter().map(|a| a.name.as_str()).collect();
    assert!(
        !names.contains(&"rca_scenarios"),
        "rca_scenarios must be omitted when args.e2e=false, got: {names:?}"
    );
    assert!(
        names.contains(&"rca_report"),
        "rca_report must always be present, got: {names:?}"
    );
}

#[test]
fn rca_scenarios_present_when_e2e_true() {
    let expanded = expand_pipeline(&debug_flow_pipeline_path()).expect("expansion must succeed");
    let mut args_true: HashMap<String, serde_json::Value> = HashMap::new();
    args_true.insert("e2e".to_string(), serde_json::Value::Bool(true));
    let active = belt_core::view::active_produces(&expanded[0], &args_true);
    let names: Vec<&str> = active.iter().map(|a| a.name.as_str()).collect();
    assert!(names.contains(&"rca_scenarios"));
    assert!(names.contains(&"rca_report"));
}

#[test]
fn all_phases_have_max_retries_3_and_confirm_true() {
    let pipeline = debug_flow_pipeline();
    for phase in &pipeline.phases {
        assert_eq!(phase.max_retries, 3, "phase '{}' max_retries must be 3", phase.id);
        assert!(phase.confirm, "phase '{}' confirm must be true", phase.id);
    }
}

#[test]
fn supplement_files_exist() {
    let refs_dir = debug_flow_dir().join("references");
    for name in [
        "path-convention.md",
        "rca-supplement.md",
        "fix-plan-supplement.md",
        "monkey-test-supplement.md",
        "dogfood-supplement.md",
        "worktrunk-supplement.md",
    ] {
        assert!(
            refs_dir.join(name).exists(),
            "supplement file '{name}' must exist"
        );
    }
}

#[test]
fn dead_letter_references_removed() {
    let refs_dir = debug_flow_dir().join("references");
    for name in ["evidence-plan-protocol.md", "fix-dispatch-strategy.md"] {
        assert!(
            !refs_dir.join(name).exists(),
            "dead-letter reference '{name}' must be removed"
        );
    }
}

#[test]
fn criteria_files_exist() {
    let criteria_dir = debug_flow_dir().join("criteria");
    for name in [
        "rca.md",
        "fix-plan.md",
        "fix-plan-review.md",
        "monkey-test.md",
        "dogfood.md",
        "integrate.md",
    ] {
        assert!(
            criteria_dir.join(name).exists(),
            "criteria file '{name}' must exist"
        );
    }

    // Shared criteria: pipeline.yml uses `../../criteria/` relative to
    // `examples/skills/debug-flow/`, resolving to `examples/criteria/`.
    let shared = repo_root().join("examples/criteria");
    for name in ["execute.md", "code-review.md"] {
        assert!(
            shared.join(name).exists(),
            "shared criteria '{name}' must exist at examples/criteria/"
        );
    }
}

#[test]
fn skill_md_has_expected_sections() {
    let skill_md = debug_flow_dir().join("SKILL.md");
    let content = std::fs::read_to_string(&skill_md).expect("SKILL.md must exist");
    for section in [
        "## Phase-Specific Invocation Rules",
        "## Red Flags",
        "## References",
        "argument-hint:",
    ] {
        assert!(
            content.contains(section),
            "SKILL.md must contain '{section}'"
        );
    }
}

#[test]
fn skill_md_declares_supplement_injection_per_phase() {
    let skill_md = debug_flow_dir().join("SKILL.md");
    let content = std::fs::read_to_string(&skill_md).expect("SKILL.md must exist");
    // Phases 1 (rca), 2 (fix-plan), 6 (monkey-test), 7 (dogfood), 8 (integrate)
    // must each reference a specific supplement via INVOKE 1 in SKILL.md.
    for supplement in [
        "rca-supplement.md",
        "fix-plan-supplement.md",
        "monkey-test-supplement.md",
        "dogfood-supplement.md",
        "worktrunk-supplement.md",
    ] {
        assert!(
            content.contains(supplement),
            "SKILL.md must reference supplement '{supplement}'"
        );
    }
}
```

If any type-path mismatch appears at compile time (e.g., `Invoker` located in a submodule), adjust the `use` block by reading `crates/belt-core/src/model.rs` and mirroring the `model::*` re-exports used in existing `feature_dev_refresh.rs` / `review_skills_refresh.rs`.

- [ ] **Step 3: Run test + regression**

```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
cargo test -p belt-core --test debug_flow_refresh
cargo test -p belt-core
```

Expected: all pass. If any assertion fails, the spec / fixture drift — investigate and fix (prefer fixing the implementation / content to match spec, not weakening the test).

- [ ] **Step 4: Run clippy + fmt**

```bash
cargo fmt --package belt-core
cargo clippy --workspace -- -D warnings
```

Expected: both exit 0. Common pitfalls from memory `project_review_skills_refresh_2026_04_15.md`:
- `unused import` (e.g., `Path` if no longer used)
- `items-after-statements`
- `use` block fmt changes

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/tests/debug_flow_refresh.rs
git commit -m "test(belt-core): add debug_flow_refresh integration test"
```

---

### Task 25: Add `argument-hint` to `feature-dev/SKILL.md`

**Files:**
- Modify: `examples/skills/feature-dev/SKILL.md`

**Commit:** `docs(feature-dev): add argument-hint for parity with debug-flow`

- [ ] **Step 1: Read current `feature-dev/SKILL.md` frontmatter**

Read `examples/skills/feature-dev/SKILL.md` lines 1–10. Confirm current frontmatter has `name`, `description`, `user-invocable` but no `argument-hint`.

- [ ] **Step 2: Add `argument-hint`**

Insert `argument-hint: "[--e2e] [--codex]"` into the frontmatter (directly below `user-invocable: true`):

```yaml
---
name: feature-dev
description: >-
  Quality-gated development pipeline (8 phases). Design → test scenarios → plan →
  execute → code review → monkey test (E2E scripted) → dogfood (E2E exploratory) →
  integrate. Web UI testing phases are conditional on --e2e.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---
```

- [ ] **Step 3: Verify**

```bash
head -10 examples/skills/feature-dev/SKILL.md
grep "argument-hint" examples/skills/feature-dev/SKILL.md
```

Expected: frontmatter intact, `argument-hint` line present.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/feature-dev/SKILL.md
git commit -m "docs(feature-dev): add argument-hint for parity with debug-flow"
```

---

### Task 26: Final verification

**Files:** None (verification only, no commit)

- [ ] **Step 1: Run full verification suite**

```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt

cargo fmt --package belt-core
cargo clippy --workspace -- -D warnings
cargo test -p belt-core --test artifact_when_field
cargo test -p belt-core --test debug_flow_refresh
cargo test -p belt-core --test feature_dev_refresh
cargo test -p belt-core --test review_skills_refresh
cargo test -p belt-core

cargo run --bin belt -- lint examples/skills/debug-flow/pipeline.yml
```

Expected: all exit 0.

- [ ] **Step 2: Grep hygiene checks (canonical patterns)**

```bash
grep -rnE 'iterations|swarm|--smoke' examples/skills/debug-flow/
grep -rnE '(^|[[:space:]])(ui)[=:]' examples/skills/debug-flow/
grep -rn "consensus\|artifacts/reviews/" examples/skills/debug-flow/criteria/
grep -rn "fix-dispatch-strategy" examples/references/
grep -rn "evidence-plan-protocol" examples/skills/debug-flow/
```

Expected: zero matches for all five.

- [ ] **Step 3: Runtime end-to-end verification (Must-Verify Checklist items #8-#9)**

Run debug-flow pipeline with `--e2e=false`, confirm `rca_scenarios` omitted from status JSON:

```bash
cd /tmp
rm -rf belt-verify-e2e-false
mkdir -p belt-verify-e2e-false && cd belt-verify-e2e-false
# Use absolute path to the belt repo pipeline.yml
PIPELINE=/Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/debug-flow/pipeline.yml
cargo run --manifest-path /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/Cargo.toml --bin belt-agent -- init --pipeline "$PIPELINE" --args e2e=false
RUN_ID=$(cargo run --manifest-path /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/Cargo.toml --bin belt-agent -- latest-run-id)
cargo run --manifest-path /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/Cargo.toml --bin belt-agent -- status --run-id "$RUN_ID" | tee status-e2e-false.json
grep -q "rca_scenarios" status-e2e-false.json && echo "FAIL: rca_scenarios must be omitted when --e2e=false" || echo "PASS"
```

Expected: "PASS" line. If `rca_scenarios` appears in the status JSON, Artifact.when filtering (Tasks 3–4) is broken.

Repeat with `--e2e=true`, confirm `rca_scenarios` present:

```bash
cd /tmp && rm -rf belt-verify-e2e-true && mkdir belt-verify-e2e-true && cd belt-verify-e2e-true
cargo run --manifest-path /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/Cargo.toml --bin belt-agent -- init --pipeline "$PIPELINE" --args e2e=true
RUN_ID=$(cargo run --manifest-path /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/Cargo.toml --bin belt-agent -- latest-run-id)
cargo run --manifest-path /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/Cargo.toml --bin belt-agent -- status --run-id "$RUN_ID" | tee status-e2e-true.json
grep -q "rca_scenarios" status-e2e-true.json && echo "PASS" || echo "FAIL: rca_scenarios must be present when --e2e=true"
```

Expected: "PASS" line.

(Adjust the `belt-agent` subcommand names if they differ — use `belt-agent --help` to confirm. Minimum: `init --pipeline`, some way to retrieve latest run id, `status --run-id`.)

- [ ] **Step 4: Must-Verify Checklist from spec**

Re-check the Must-Verify Checklist in this plan's header section. Confirm each of 9 items checked.

- [ ] **Step 4: (optional) Dogfood run**

If time permits, manually invoke `/debug-flow` on a real small bug and walk through the 8 phases. Not required for plan completion but valuable for sanity check.

- [ ] **Step 5: Summary**

Produce a final summary report (inline, no file):
- All tasks completed
- Verification command outputs
- Known deviations (if any, with rationale)
- Follow-ups remaining (e.g., Task 23 conditional skip, Phase I dogfood pending)
