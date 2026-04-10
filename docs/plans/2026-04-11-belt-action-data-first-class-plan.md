# belt-core v2: Invoker and Artifact Implementation Plan (Phase 1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add first-class typed primitives `Invoker`, `Artifact`, `ArtifactRef`, and `ValidationSource` to `belt-core`, extend `Phase` with `invoke`, `produces`, `consumes` fields and extended `validate`, and add corresponding lint rules, expander handling, and view enrichment. All changes are additive; existing examples and `artifacts:` / `uses:` / `config.skill` patterns continue to parse unchanged.

**Architecture:** TDD throughout. Each new type is a small serde-driven addition, implemented as an untagged enum (parallel to the existing `GateCheck` precedent). The approach is strictly additive within `belt-core` — no external crate API changes, no new CLI commands, no new dependencies. Backwards compatibility is preserved so that `examples/skills/` does not need to be touched in this plan.

**Tech Stack:** Rust 2024, `serde 1.0.228`, `serde-saphyr =0.0.23` (pinned), `miette 7.6`, `tempfile` (tests). MSRV 1.86.0, CI toolchain 1.94.1.

**Linear:** [BELT-32](https://linear.app/neko-neko/issue/BELT-32)
**Spec:** `docs/specs/2026-04-11-belt-action-data-first-class.md`
**Parent:** [BELT-20](https://linear.app/neko-neko/issue/BELT-20)

---

## Scope

### In scope (this plan — "Phase 1: belt-core additive")

- `belt-core::model` — new types: `ValidationSource`, `Artifact`, `ArtifactRef`, `Invoker`
- `belt-core::model::Phase` — new fields: `invoke`, `produces`, `consumes`; extended field type: `validate: Vec<ValidationSource>`
- `belt-core::model::ExpandedPhase` — mirror new fields so engine/view can read them
- `belt-core::parser` — verify new fields deserialize correctly (serde-driven, mostly automatic)
- `belt-core::expander` — propagate new fields through sub-pipeline expansion; handle `Invoker::Pipeline` as an alternate phase-level sub-pipeline reference
- `belt-core::lint` — four new lint rules: invoker shape, consumes resolution, produces uniqueness, validate file existence
- `belt-core::view::PhaseView` — expose `invoke`, `produces`, `consumes` data in enriched status view
- Tests: model unit tests, parser integration, expander integration, lint rule integration, view tests

### Out of scope (deferred to Plan B)

- Migration of `examples/skills/` (feature-dev, debug-flow, smoke-test, spec-review, code-review, test-review, implementation-review)
- Deletion of `examples/skills/audit-gate/`
- `skills/belt-agent/SKILL.md` protocol update
- Removal of legacy `Phase.artifacts` / `Phase.uses` fields (kept for backwards compatibility)
- Glob resolution semantics for `Artifact.path` (Artifact struct carries the path as-is; resolution logic is a Plan B concern)
- `confirm:`, `retry:`, `when:`, `regate:` typing (forward-compat schema only; no behavior change, covered by spec DD-6)
- `belt` / `belt-agent` CLI binary changes — both pick up new fields automatically via `belt-core`

### Key design decisions locked by the spec

- `Invoker` variant ordering (serde-saphyr untagged enum disambiguation): `Skill` → `Agent` → `Agents` → `Pipeline`. All four variants have unique discriminating required field names (`skill`, `agent`, `agents`, `pipeline`), so ordering is defensive rather than strictly necessary.
- `ArtifactRef::Named` (string) before `ArtifactRef::Qualified` (struct `{ name, from }`) in untagged enum.
- `ValidationSource::Inline` (string) before `ValidationSource::File` (struct `{ file: ... }`) in untagged enum.
- `Phase.artifacts: Vec<String>` is **kept** as legacy (not touched by this plan; removed in Plan B after examples migration).
- `Phase.uses: Option<String>` is **kept** as legacy; `Invoker::Pipeline` is a new alternate form that the expander handles alongside the existing `uses:` path.

---

## File Structure

### Modified files (belt-core)

- `crates/belt-core/src/model.rs` — Add new types; extend `Phase` and `ExpandedPhase`.
- `crates/belt-core/src/expander.rs` — Propagate new fields through sub-pipeline expansion; handle `Invoker::Pipeline`.
- `crates/belt-core/src/lint.rs` — Add four lint rules.
- `crates/belt-core/src/view.rs` — Extend `PhaseView` with invoker, produces, consumes data.

### Modified files (tests)

- `crates/belt-core/tests/model_test.rs` — Add tests for new types and Phase field changes.
- `crates/belt-core/tests/parser_test.rs` — Integration tests for parser handling new fields.
- `crates/belt-core/tests/expander_test.rs` — Expansion with new fields.
- `crates/belt-core/tests/lint_test.rs` — Four new lint rule tests.
- `crates/belt-core/tests/view_test.rs` — PhaseView with new fields.

### Files NOT modified (by design)

- `crates/belt-core/src/parser.rs` — Serde handles new fields automatically; no parser code changes needed. (Verify via tests.)
- `crates/belt-core/src/engine.rs` — Engine only constructs `RunState` and delegates to view for status output; no direct field handling needed.
- `crates/belt-core/src/gate.rs` — Unrelated.
- `crates/belt-core/src/config.rs` — Unrelated.
- `crates/belt-core/src/error.rs` — No new error variants required (lint diagnostics use existing `LintDiagnostic`).
- `crates/belt-core/src/lib.rs` — No module changes (types are accessible via `belt_core::model::...`).
- `crates/belt/src/main.rs` — Lint rules surface automatically.
- `crates/belt-agent/src/main.rs` — JSON output via view module automatically includes new fields.
- `examples/skills/**` — Migration is Plan B.
- `skills/belt-agent/SKILL.md` — Update is Plan B.

---

## Task 1: Add `ValidationSource` untagged enum and change `Phase.validate` type

**Files:**
- Modify: `crates/belt-core/src/model.rs`
- Modify: `crates/belt-core/tests/model_test.rs`

- [ ] **Step 1: Write failing test for inline variant (backwards compat)**

Add to `crates/belt-core/tests/model_test.rs` (at the end of the file):

```rust
use belt_core::model::ValidationSource;

/// Backwards compat: `validate: ["string"]` must still parse to `Inline`.
#[test]
fn parse_validate_inline_backwards_compat() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: "p"
    validate:
      - "inline criterion"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].validate.len(), 1);
    match &pipeline.phases[0].validate[0] {
        ValidationSource::Inline(s) => assert_eq!(s, "inline criterion"),
        other => panic!("expected Inline, got {other:?}"),
    }
}

/// New: `validate: [{ file: "./path" }]` parses to `File`.
#[test]
fn parse_validate_file_reference() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: "p"
    validate:
      - file: "./criteria/p.md"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].validate.len(), 1);
    match &pipeline.phases[0].validate[0] {
        ValidationSource::File { file } => assert_eq!(file, "./criteria/p.md"),
        other => panic!("expected File, got {other:?}"),
    }
}

/// Mixed inline and file references in one validate list.
#[test]
fn parse_validate_mixed_inline_and_file() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: "p"
    validate:
      - "inline one"
      - file: "./criteria/p.md"
      - "inline two"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].validate.len(), 3);
    assert!(matches!(
        &pipeline.phases[0].validate[0],
        ValidationSource::Inline(s) if s == "inline one"
    ));
    assert!(matches!(
        &pipeline.phases[0].validate[1],
        ValidationSource::File { file } if file == "./criteria/p.md"
    ));
    assert!(matches!(
        &pipeline.phases[0].validate[2],
        ValidationSource::Inline(s) if s == "inline two"
    ));
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p belt-core --test model_test parse_validate_ 2>&1
```

Expected: FAIL with compilation error `cannot find type ValidationSource in module belt_core::model`.

- [ ] **Step 3: Add `ValidationSource` to `model.rs`**

In `crates/belt-core/src/model.rs`, add after the `GateCheck` enum (around line 87 — look for `pub enum GateCheck`):

```rust
/// A single validation criterion. Either an inline string that the
/// orchestrator evaluates directly, or a reference to a markdown file whose
/// contents are the criteria. The file form replaces the audit-gate
/// sub-pipeline pattern used in BELT-20 MVP examples.
///
/// Ordering is significant for serde-saphyr untagged enum deserialization:
/// `Inline` (scalar string) is checked before `File` (mapping with a `file`
/// key), matching the GateCheck precedent where more specific struct
/// variants come after scalar variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValidationSource {
    Inline(String),
    File { file: String },
}
```

And change the `validate` field in `Phase` (around line 54):

```rust
// Before:
// pub validate: Vec<String>,

// After:
#[serde(default)]
pub validate: Vec<ValidationSource>,
```

- [ ] **Step 4: Fix any test/code that constructs `Phase` directly with old `validate: Vec<String>`**

Run:
```bash
cargo build -p belt-core 2>&1
```

Expected: compilation errors in places that construct `Phase { validate: vec!["..."], ... }`. For each error, convert inline strings to `ValidationSource::Inline(s.to_string())`.

Known locations to check (search via Grep):
```bash
# Run Grep for Phase literal constructors:
# pattern: "Phase \{"
# Expected: possibly in parser/expander tests or model tests
```

Also check `crates/belt-core/src/expander.rs` — the `leaf_phase()` and `expand_sub_pipeline()` functions construct `ExpandedPhase`, which has its own `validate: Vec<String>` field. Update `ExpandedPhase` too:

In `model.rs`, find `ExpandedPhase` (around line 147) and change:
```rust
// Before: pub validate: Vec<String>,
// After:
#[serde(default)]
pub validate: Vec<ValidationSource>,
```

Re-run `cargo build -p belt-core 2>&1` and fix any remaining errors.

- [ ] **Step 5: Run new tests to verify they pass**

```bash
cargo test -p belt-core --test model_test parse_validate_ 2>&1
```

Expected: 3 tests pass.

- [ ] **Step 6: Run full belt-core test suite to verify no regressions**

```bash
cargo test -p belt-core 2>&1
```

Expected: All tests pass. Pre-existing `parse_phase_all_fields` test at `crates/belt-core/tests/model_test.rs:33` contains `validate: ["file_exists dist/app.tar.gz"]` which will now deserialize to `ValidationSource::Inline` — the existing assertion `assert_eq!(phase.validate.len(), 1)` still passes.

- [ ] **Step 7: Run clippy and fmt**

```bash
cargo clippy -p belt-core -- -D warnings 2>&1 && \
cargo fmt --package belt-core -- --check 2>&1
```

Expected: no warnings, no format diff. If fmt diff exists, run `cargo fmt --package belt-core` and commit.

- [ ] **Step 8: Commit**

```bash
git add crates/belt-core/src/model.rs crates/belt-core/tests/model_test.rs
git commit -m "feat(belt-core): add ValidationSource enum for validate field (BELT-32)"
```

---

## Task 2: Add `Artifact` struct

**Files:**
- Modify: `crates/belt-core/src/model.rs`
- Modify: `crates/belt-core/tests/model_test.rs`

- [ ] **Step 1: Write failing test**

Add to `crates/belt-core/tests/model_test.rs`:

```rust
use belt_core::model::Artifact;

/// Parse a phase with one produces artifact (all fields populated).
#[test]
fn parse_phase_produces_single_artifact() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: design
    description: "Create design"
    produces:
      - name: design_doc
        path: "docs/plans/*-design.md"
        description: "Brainstormed design document"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].produces.len(), 1);
    let a = &pipeline.phases[0].produces[0];
    assert_eq!(a.name, "design_doc");
    assert_eq!(a.path, "docs/plans/*-design.md");
    assert_eq!(a.description.as_deref(), Some("Brainstormed design document"));
}

/// Parse a phase with produces artifact where description is omitted.
#[test]
fn parse_phase_produces_without_description() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: design
    description: "Create design"
    produces:
      - name: design_doc
        path: "docs/plans/*-design.md"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].produces.len(), 1);
    assert!(pipeline.phases[0].produces[0].description.is_none());
}

/// Phase with no produces field defaults to empty vec.
#[test]
fn parse_phase_produces_default_empty() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: design
    description: "Create design"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert!(pipeline.phases[0].produces.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p belt-core --test model_test parse_phase_produces_ 2>&1
```

Expected: FAIL with compilation error `no field 'produces' on type 'Phase'` or `cannot find type 'Artifact'`.

- [ ] **Step 3: Add `Artifact` struct to `model.rs`**

In `crates/belt-core/src/model.rs`, add after `ValidationSource` (from Task 1):

```rust
/// A typed artifact produced by a phase. The `name` is a logical identifier
/// by which later phases reference the artifact via `consumes:`. The `path`
/// is the filesystem path the LLM is expected to produce (glob permitted for
/// runtime-determined filenames like `docs/plans/*-design.md`).
///
/// Glob resolution semantics are intentionally not specified here; they are
/// deferred to the Plan B examples migration implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub description: Option<String>,
}
```

And add the `produces` field to `Phase` (around where `artifacts:` currently lives, before `gate:`):

```rust
// In Phase struct, add after the existing `artifacts` field:
#[serde(default)]
pub produces: Vec<Artifact>,
```

Also add `produces` to `ExpandedPhase` (so the engine can read it after expansion):

```rust
// In ExpandedPhase struct, add after `artifacts`:
#[serde(default)]
pub produces: Vec<Artifact>,
```

- [ ] **Step 4: Update `expander.rs` to propagate `produces`**

In `crates/belt-core/src/expander.rs`, find `expand_sub_pipeline` (around line 38) and `leaf_phase` (around line 83). In both functions, the `ExpandedPhase` literal currently omits the new field — add it:

In `leaf_phase`:
```rust
Ok(ExpandedPhase {
    id: phase.id.clone(),
    description,
    config: phase.config.clone(),
    artifacts: phase.artifacts.clone(),
    produces: phase.produces.clone(),  // NEW
    gate: phase.gate.clone(),
    validate: phase.validate.clone(),
    regate: phase.regate.clone(),
    confirm: phase.confirm,
    max_retries: phase.max_retries,
    when: phase.when.clone(),
    output_dir: None,
})
```

In `expand_sub_pipeline` (inside the `phases.push(ExpandedPhase { ... })` block around line 66):
```rust
phases.push(ExpandedPhase {
    id: namespaced_id,
    description: sub_phase.description.clone().unwrap_or_default(),
    config: merged_config,
    artifacts: sub_phase.artifacts.clone(),
    produces: sub_phase.produces.clone(),  // NEW
    gate,
    validate,
    regate,
    confirm: sub_phase.confirm,
    max_retries: sub_phase.max_retries,
    when,
    output_dir: None,
});
```

- [ ] **Step 5: Run new tests to verify they pass**

```bash
cargo test -p belt-core --test model_test parse_phase_produces_ 2>&1
```

Expected: 3 tests pass.

- [ ] **Step 6: Run full belt-core test suite**

```bash
cargo test -p belt-core 2>&1
```

Expected: All tests pass.

- [ ] **Step 7: Run clippy and fmt**

```bash
cargo clippy -p belt-core -- -D warnings 2>&1 && \
cargo fmt --package belt-core -- --check 2>&1
```

Expected: no warnings, no format diff.

- [ ] **Step 8: Commit**

```bash
git add crates/belt-core/src/model.rs crates/belt-core/src/expander.rs crates/belt-core/tests/model_test.rs
git commit -m "feat(belt-core): add Artifact struct and Phase.produces field (BELT-32)"
```

---

## Task 3: Add `ArtifactRef` untagged enum and `Phase.consumes` field

**Files:**
- Modify: `crates/belt-core/src/model.rs`
- Modify: `crates/belt-core/src/expander.rs`
- Modify: `crates/belt-core/tests/model_test.rs`

- [ ] **Step 1: Write failing test**

Add to `crates/belt-core/tests/model_test.rs`:

```rust
use belt_core::model::ArtifactRef;

/// Parse consumes as a list of short (Named) references.
#[test]
fn parse_phase_consumes_named() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: plan
    description: "Plan"
    consumes:
      - design_doc
      - requirements
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].consumes.len(), 2);
    assert!(matches!(
        &pipeline.phases[0].consumes[0],
        ArtifactRef::Named(s) if s == "design_doc"
    ));
    assert!(matches!(
        &pipeline.phases[0].consumes[1],
        ArtifactRef::Named(s) if s == "requirements"
    ));
}

/// Parse consumes as a list of qualified references.
#[test]
fn parse_phase_consumes_qualified() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: plan
    description: "Plan"
    consumes:
      - name: design_doc
        from: design
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].consumes.len(), 1);
    match &pipeline.phases[0].consumes[0] {
        ArtifactRef::Qualified { name, from } => {
            assert_eq!(name, "design_doc");
            assert_eq!(from, "design");
        }
        other => panic!("expected Qualified, got {other:?}"),
    }
}

/// Parse consumes as a mixed list of short and qualified references.
#[test]
fn parse_phase_consumes_mixed() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: execute
    description: "Execute"
    consumes:
      - plan_doc
      - name: design_doc
        from: design
      - test_cases
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].consumes.len(), 3);
    assert!(matches!(
        &pipeline.phases[0].consumes[0],
        ArtifactRef::Named(s) if s == "plan_doc"
    ));
    assert!(matches!(
        &pipeline.phases[0].consumes[1],
        ArtifactRef::Qualified { name, from } if name == "design_doc" && from == "design"
    ));
    assert!(matches!(
        &pipeline.phases[0].consumes[2],
        ArtifactRef::Named(s) if s == "test_cases"
    ));
}

/// Phase with no consumes field defaults to empty vec.
#[test]
fn parse_phase_consumes_default_empty() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: "p"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert!(pipeline.phases[0].consumes.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p belt-core --test model_test parse_phase_consumes_ 2>&1
```

Expected: FAIL with compilation errors.

- [ ] **Step 3: Add `ArtifactRef` to `model.rs`**

In `crates/belt-core/src/model.rs`, add after `Artifact` (from Task 2):

```rust
/// A reference to an artifact produced by an earlier phase. `Named` is the
/// short form — lint resolves it to the most recent earlier phase that
/// produced that name. `Qualified` disambiguates when multiple earlier phases
/// produce the same name.
///
/// Ordering: `Named` (scalar string) is checked before `Qualified` (struct
/// mapping) for serde-saphyr untagged enum disambiguation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArtifactRef {
    Named(String),
    Qualified { name: String, from: String },
}
```

Add the `consumes` field to `Phase`:

```rust
// In Phase struct, after `produces`:
#[serde(default)]
pub consumes: Vec<ArtifactRef>,
```

Add the same field to `ExpandedPhase`:

```rust
// In ExpandedPhase struct, after `produces`:
#[serde(default)]
pub consumes: Vec<ArtifactRef>,
```

- [ ] **Step 4: Update `expander.rs` to propagate `consumes`**

In both `leaf_phase` and `expand_sub_pipeline` in `crates/belt-core/src/expander.rs`, add `consumes: ... .clone()` to the `ExpandedPhase` literal alongside the `produces` field added in Task 2.

`leaf_phase`:
```rust
produces: phase.produces.clone(),
consumes: phase.consumes.clone(),  // NEW
```

`expand_sub_pipeline`:
```rust
produces: sub_phase.produces.clone(),
consumes: sub_phase.consumes.clone(),  // NEW
```

- [ ] **Step 5: Run new tests to verify they pass**

```bash
cargo test -p belt-core --test model_test parse_phase_consumes_ 2>&1
```

Expected: 4 tests pass.

- [ ] **Step 6: Run full belt-core test suite**

```bash
cargo test -p belt-core 2>&1
```

Expected: All tests pass.

- [ ] **Step 7: Run clippy and fmt**

```bash
cargo clippy -p belt-core -- -D warnings 2>&1 && \
cargo fmt --package belt-core -- --check 2>&1
```

Expected: no warnings.

- [ ] **Step 8: Commit**

```bash
git add crates/belt-core/src/model.rs crates/belt-core/src/expander.rs crates/belt-core/tests/model_test.rs
git commit -m "feat(belt-core): add ArtifactRef enum and Phase.consumes field (BELT-32)"
```

---

## Task 4: Add `Invoker` untagged enum (all 4 variants) and `Phase.invoke` field

**Files:**
- Modify: `crates/belt-core/src/model.rs`
- Modify: `crates/belt-core/src/expander.rs`
- Modify: `crates/belt-core/tests/model_test.rs`

- [ ] **Step 1: Write failing test — all 4 Invoker variants**

Add to `crates/belt-core/tests/model_test.rs`:

```rust
use belt_core::model::Invoker;

/// Parse a phase with `invoke: { skill: "/foo" }`.
#[test]
fn parse_invoke_skill_variant() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: design
    description: "Design"
    invoke:
      skill: /brainstorming
      args:
        swarm: true
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    match pipeline.phases[0].invoke.as_ref().expect("invoke present") {
        Invoker::Skill { skill, args } => {
            assert_eq!(skill, "/brainstorming");
            assert_eq!(
                args.get("swarm").and_then(serde_json::Value::as_bool),
                Some(true)
            );
        }
        other => panic!("expected Skill, got {other:?}"),
    }
}

/// Parse a phase with `invoke: { agent: "phase-auditor" }`.
#[test]
fn parse_invoke_agent_variant() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: audit
    description: "Audit"
    invoke:
      agent: phase-auditor
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    match pipeline.phases[0].invoke.as_ref().expect("invoke present") {
        Invoker::Agent { agent, args } => {
            assert_eq!(agent, "phase-auditor");
            assert!(args.is_empty());
        }
        other => panic!("expected Agent, got {other:?}"),
    }
}

/// Parse a phase with `invoke: { agents: [a, b], iterations: 3 }`.
#[test]
fn parse_invoke_agents_variant() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: review
    description: "Review"
    invoke:
      agents:
        - spec-review-requirements
        - spec-review-consistency
      iterations: 3
      args:
        codex: true
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    match pipeline.phases[0].invoke.as_ref().expect("invoke present") {
        Invoker::Agents { agents, iterations, args } => {
            assert_eq!(agents.len(), 2);
            assert_eq!(agents[0], "spec-review-requirements");
            assert_eq!(*iterations, 3);
            assert_eq!(
                args.get("codex").and_then(serde_json::Value::as_bool),
                Some(true)
            );
        }
        other => panic!("expected Agents, got {other:?}"),
    }
}

/// Parse a phase with `invoke: { pipeline: "./sub.yml" }`.
#[test]
fn parse_invoke_pipeline_variant() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: spec-review
    description: "Spec review sub-pipeline"
    invoke:
      pipeline: ../spec-review/pipeline.yml
      with:
        iterations: 2
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    match pipeline.phases[0].invoke.as_ref().expect("invoke present") {
        Invoker::Pipeline { pipeline: p, with } => {
            assert_eq!(p, "../spec-review/pipeline.yml");
            assert_eq!(
                with.get("iterations").and_then(serde_json::Value::as_u64),
                Some(2)
            );
        }
        other => panic!("expected Pipeline, got {other:?}"),
    }
}

/// Phase without `invoke` field: `invoke` is None.
#[test]
fn parse_phase_invoke_default_none() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: "p"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert!(pipeline.phases[0].invoke.is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p belt-core --test model_test parse_invoke_ 2>&1
```

Expected: FAIL with compilation errors.

- [ ] **Step 3: Add `Invoker` enum to `model.rs`**

In `crates/belt-core/src/model.rs`, add after `ArtifactRef` (from Task 3):

```rust
/// Typed invocation target for a phase. Parallel to the existing `GateCheck`
/// untagged enum: belt-core models the invocation shape but the LLM
/// orchestrator is responsible for actually dispatching the skill, agent, or
/// sub-pipeline at runtime.
///
/// Variant ordering for serde-saphyr untagged enum disambiguation:
/// `Skill` (field: `skill`) → `Agent` (field: `agent`) → `Agents` (field:
/// `agents`) → `Pipeline` (field: `pipeline`). Each variant has a unique
/// required discriminating field, so ordering is defensive rather than
/// strictly necessary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Invoker {
    Skill {
        skill: String,
        #[serde(default)]
        args: HashMap<String, serde_json::Value>,
    },
    Agent {
        agent: String,
        #[serde(default)]
        args: HashMap<String, serde_json::Value>,
    },
    Agents {
        agents: Vec<String>,
        #[serde(default)]
        iterations: u32,
        #[serde(default)]
        args: HashMap<String, serde_json::Value>,
    },
    Pipeline {
        pipeline: String,
        #[serde(default)]
        with: HashMap<String, serde_json::Value>,
    },
}
```

Add the `invoke` field to `Phase` (place it before `config` so it appears near the top of the phase definition in serialized form):

```rust
// In Phase struct, after `when`:
#[serde(default)]
pub invoke: Option<Invoker>,
```

Add the same field to `ExpandedPhase`:

```rust
// In ExpandedPhase struct, after `when`:
#[serde(default)]
pub invoke: Option<Invoker>,
```

- [ ] **Step 4: Update `expander.rs` to propagate `invoke`**

In both `leaf_phase` and `expand_sub_pipeline` in `crates/belt-core/src/expander.rs`, add `invoke: ... .clone()` to the `ExpandedPhase` literal.

`leaf_phase`:
```rust
invoke: phase.invoke.clone(),  // NEW
```

`expand_sub_pipeline`: For the last sub-phase, we also want to inherit the parent's `invoke` if the sub-phase has none. But this is subtle — it should probably NOT inherit, because sub-pipelines declare their own invocations. For safety, the sub-phase's own `invoke` is preserved and the parent's `invoke` is **not** cascaded:

```rust
invoke: sub_phase.invoke.clone(),  // NEW, not inherited from parent
```

- [ ] **Step 5: Run new tests to verify they pass**

```bash
cargo test -p belt-core --test model_test parse_invoke_ 2>&1
```

Expected: 5 tests pass.

- [ ] **Step 6: Adversarial test — ensure Invoker ordering correctly disambiguates**

Add an adversarial test that tries to construct a phase where the YAML could plausibly match multiple variants, to verify the ordering holds:

```rust
/// Adversarial: a phase invoke that has both `skill` and `agent` (should
/// prefer the first matching variant — Skill). This is malformed YAML but
/// should either pick Skill deterministically or produce a parser error.
#[test]
fn parse_invoke_variant_order_is_deterministic() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: "p"
    invoke:
      skill: /foo
      agent: bar
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    // With Skill variant declared first, serde-saphyr should match Skill.
    // `agent` becomes an extra key and is either ignored or included in args.
    match pipeline.phases[0].invoke.as_ref().expect("invoke present") {
        Invoker::Skill { skill, .. } => assert_eq!(skill, "/foo"),
        other => panic!(
            "expected Skill variant (declared first), got {other:?}; \
             variant ordering may not be deterministic"
        ),
    }
}
```

Run:
```bash
cargo test -p belt-core --test model_test parse_invoke_variant_order_is_deterministic 2>&1
```

Expected: PASS. If it fails, re-order `Invoker` variants until Skill comes first and is matched.

- [ ] **Step 7: Run full belt-core test suite and clippy**

```bash
cargo test -p belt-core 2>&1 && \
cargo clippy -p belt-core -- -D warnings 2>&1 && \
cargo fmt --package belt-core -- --check 2>&1
```

Expected: All tests pass, no clippy warnings, no format diff.

- [ ] **Step 8: Commit**

```bash
git add crates/belt-core/src/model.rs crates/belt-core/src/expander.rs crates/belt-core/tests/model_test.rs
git commit -m "feat(belt-core): add Invoker enum with 4 variants and Phase.invoke (BELT-32)"
```

---

## Task 5: Expander — handle `Invoker::Pipeline` as alternate sub-pipeline reference

The `Invoker::Pipeline` variant references a sub-pipeline file via `invoke: { pipeline: "./sub.yml" }`. For backwards compatibility, the existing `uses: "./sub.yml"` (phase-level) still works. This task teaches the expander to treat `Invoker::Pipeline` as a **second** way to trigger sub-pipeline expansion.

**Files:**
- Modify: `crates/belt-core/src/expander.rs`
- Modify: `crates/belt-core/tests/expander_test.rs`

- [ ] **Step 1: Write failing test**

Add to `crates/belt-core/tests/expander_test.rs`:

```rust
/// A phase using `invoke: { pipeline: "./sub.yml" }` expands into the
/// sub-pipeline's phases with namespaced IDs, identically to using
/// `uses: "./sub.yml"` at the phase level.
#[test]
fn expand_invoke_pipeline_variant() {
    use belt_core::expander::expand_pipeline;
    use tempfile::TempDir;
    use std::io::Write;

    let dir = TempDir::new().expect("tempdir");

    // Write sub-pipeline.
    let sub_path = dir.path().join("sub.yml");
    let mut f = std::fs::File::create(&sub_path).expect("create sub");
    f.write_all(
        br#"
name: sub
version: 1
phases:
  - id: work
    description: "sub work"
  - id: audit
    description: "sub audit"
"#,
    )
    .expect("write sub");

    // Write top-level pipeline using invoke: { pipeline: ... }.
    let top_path = dir.path().join("pipeline.yml");
    let mut f = std::fs::File::create(&top_path).expect("create top");
    f.write_all(
        br#"
name: top
version: 1
phases:
  - id: review
    invoke:
      pipeline: ./sub.yml
"#,
    )
    .expect("write top");

    let expanded = expand_pipeline(&top_path).expect("expand should succeed");

    // Expect 2 phases, namespaced `review/work` and `review/audit`.
    assert_eq!(expanded.len(), 2);
    assert_eq!(expanded[0].id, "review/work");
    assert_eq!(expanded[1].id, "review/audit");
}

/// Both `uses:` and `invoke: { pipeline: ... }` produce identical expansion.
#[test]
fn expand_uses_and_invoke_pipeline_equivalent() {
    use belt_core::expander::expand_pipeline;
    use tempfile::TempDir;
    use std::io::Write;

    let dir = TempDir::new().expect("tempdir");

    let sub_path = dir.path().join("sub.yml");
    std::fs::File::create(&sub_path)
        .expect("create sub")
        .write_all(
            br#"
name: sub
version: 1
phases:
  - id: run
    description: "sub run"
"#,
        )
        .expect("write sub");

    // Pipeline A: uses:
    let a_path = dir.path().join("a.yml");
    std::fs::File::create(&a_path)
        .expect("create a")
        .write_all(
            br#"
name: a
version: 1
phases:
  - id: x
    uses: ./sub.yml
"#,
        )
        .expect("write a");

    // Pipeline B: invoke: { pipeline: ... }
    let b_path = dir.path().join("b.yml");
    std::fs::File::create(&b_path)
        .expect("create b")
        .write_all(
            br#"
name: b
version: 1
phases:
  - id: x
    invoke:
      pipeline: ./sub.yml
"#,
        )
        .expect("write b");

    let a_exp = expand_pipeline(&a_path).expect("expand a");
    let b_exp = expand_pipeline(&b_path).expect("expand b");

    assert_eq!(a_exp.len(), b_exp.len());
    assert_eq!(a_exp[0].id, b_exp[0].id);
    assert_eq!(a_exp[0].id, "x/run");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p belt-core --test expander_test expand_invoke_pipeline 2>&1
```

Expected: FAIL — the first test fails because `invoke: { pipeline: ... }` is currently ignored by the expander.

- [ ] **Step 3: Update `expand_pipeline` to detect `Invoker::Pipeline`**

In `crates/belt-core/src/expander.rs`, find `expand_pipeline` (around line 20). Currently it branches on `phase.uses`. Add a parallel branch for `phase.invoke` when it is `Some(Invoker::Pipeline { pipeline, .. })`:

```rust
pub fn expand_pipeline(pipeline_path: &Path) -> BeltResult<Vec<ExpandedPhase>> {
    let pipeline = parse_pipeline(pipeline_path)?;
    let base_dir = pipeline_path.parent().unwrap_or_else(|| Path::new("."));

    let mut expanded = Vec::new();
    for phase in &pipeline.phases {
        // Resolve the sub-pipeline path from either phase.uses or
        // phase.invoke: { pipeline: ... }. `uses:` takes precedence if both
        // are present (legacy behavior during transition).
        let sub_path_opt: Option<String> = if let Some(uses) = &phase.uses {
            Some(uses.clone())
        } else if let Some(crate::model::Invoker::Pipeline { pipeline, .. }) = &phase.invoke {
            Some(pipeline.clone())
        } else {
            None
        };

        if let Some(sub_rel) = sub_path_opt {
            let sub_path = base_dir.join(&sub_rel);
            let sub = parse_sub_pipeline(&sub_path)?;
            let sub_phases = expand_sub_pipeline(&phase.id, phase, &sub);
            expanded.extend(sub_phases);
        } else {
            expanded.push(leaf_phase(phase)?);
        }
    }
    Ok(expanded)
}
```

- [ ] **Step 4: Update `leaf_phase` to reject `Invoker::Pipeline` reaching it**

A leaf phase should never have `Invoker::Pipeline` — that's an internal invariant. Add a defensive check in `leaf_phase`:

```rust
fn leaf_phase(phase: &Phase) -> BeltResult<ExpandedPhase> {
    // Sanity: if phase has invoke: { pipeline: ... }, it should have been
    // handled by the sub-pipeline branch in expand_pipeline. Hitting this
    // case is a bug.
    debug_assert!(
        !matches!(phase.invoke, Some(crate::model::Invoker::Pipeline { .. })),
        "leaf_phase called with Invoker::Pipeline — expander branch logic is wrong"
    );

    let description = phase
        .description
        .clone()
        .ok_or_else(|| BeltError::InvalidPipeline {
            message: format!("leaf phase '{}' must have a description", phase.id),
        })?;
    // ... (rest of existing implementation)
}
```

- [ ] **Step 5: Run new tests to verify they pass**

```bash
cargo test -p belt-core --test expander_test expand_invoke_pipeline 2>&1 && \
cargo test -p belt-core --test expander_test expand_uses_and_invoke_pipeline_equivalent 2>&1
```

Expected: both tests pass.

- [ ] **Step 6: Run full belt-core test suite**

```bash
cargo test -p belt-core 2>&1
```

Expected: All tests pass, including the 11 existing expander tests.

- [ ] **Step 7: Run clippy and fmt**

```bash
cargo clippy -p belt-core -- -D warnings 2>&1 && \
cargo fmt --package belt-core -- --check 2>&1
```

Expected: no warnings.

- [ ] **Step 8: Commit**

```bash
git add crates/belt-core/src/expander.rs crates/belt-core/tests/expander_test.rs
git commit -m "feat(belt-core): expander handles Invoker::Pipeline as alt sub-pipeline ref (BELT-32)"
```

---

## Task 6: Lint rule — `Invoker::Pipeline` path existence

**Files:**
- Modify: `crates/belt-core/src/lint.rs`
- Modify: `crates/belt-core/tests/lint_test.rs`

- [ ] **Step 1: Write failing test**

Add to `crates/belt-core/tests/lint_test.rs`:

```rust
#[test]
fn lint_detects_missing_invoke_pipeline_file() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad-invoke-pipeline
version: 1
phases:
  - id: sub
    description: "sub"
    invoke:
      pipeline: ./nonexistent.yml
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("nonexistent.yml")
                && (d.message.contains("invoke") || d.message.contains("pipeline"))),
        "expected diagnostic mentioning 'nonexistent.yml' and invoke/pipeline, got: {errors:?}"
    );
}

#[test]
fn lint_accepts_valid_invoke_pipeline_path() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Write a valid sub-pipeline file first.
    write_yaml(
        &dir,
        "sub.yml",
        r#"
name: sub
version: 1
phases:
  - id: run
    description: "sub run"
"#,
    );

    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: good-invoke-pipeline
version: 1
phases:
  - id: s
    invoke:
      pipeline: ./sub.yml
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "expected no errors for valid invoke pipeline path, got: {errors:?}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p belt-core --test lint_test lint_detects_missing_invoke_pipeline lint_accepts_valid_invoke_pipeline 2>&1
```

Expected: FAIL on `lint_detects_missing_invoke_pipeline_file` (no such lint rule yet).

- [ ] **Step 3: Add the lint rule**

In `crates/belt-core/src/lint.rs`, after the existing `// Check: gate uses: references exist` block (around line 109), add:

```rust
    // Check: invoke.pipeline references exist
    for phase in &pipeline.phases {
        if let Some(crate::model::Invoker::Pipeline { pipeline: sub_path, .. }) = &phase.invoke {
            let resolved = base_dir.join(sub_path);
            if !resolved.exists() {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Error,
                    message: format!(
                        "phase '{}': invoke pipeline '{}' not found",
                        phase.id, sub_path
                    ),
                });
            }
        }
    }
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p belt-core --test lint_test lint_detects_missing_invoke_pipeline lint_accepts_valid_invoke_pipeline 2>&1
```

Expected: both tests pass.

- [ ] **Step 5: Run full test suite and clippy**

```bash
cargo test -p belt-core 2>&1 && \
cargo clippy -p belt-core -- -D warnings 2>&1 && \
cargo fmt --package belt-core -- --check 2>&1
```

Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/lint.rs crates/belt-core/tests/lint_test.rs
git commit -m "feat(belt-core): lint rule for Invoker::Pipeline path existence (BELT-32)"
```

---

## Task 7: Lint rule — `Invoker::Skill` slash format verification

**Files:**
- Modify: `crates/belt-core/src/lint.rs`
- Modify: `crates/belt-core/tests/lint_test.rs`

- [ ] **Step 1: Write failing test**

Add to `crates/belt-core/tests/lint_test.rs`:

```rust
#[test]
fn lint_detects_invoke_skill_without_leading_slash() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad-invoke-skill
version: 1
phases:
  - id: p
    description: "p"
    invoke:
      skill: brainstorming
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("brainstorming") && d.message.contains("slash")),
        "expected diagnostic about leading slash, got: {errors:?}"
    );
}

#[test]
fn lint_accepts_invoke_skill_with_leading_slash() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: good-invoke-skill
version: 1
phases:
  - id: p
    description: "p"
    invoke:
      skill: /brainstorming
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

#[test]
fn lint_detects_invoke_skill_empty() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: empty-invoke-skill
version: 1
phases:
  - id: p
    description: "p"
    invoke:
      skill: ""
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.iter().any(|d| d.message.contains("empty")),
        "expected diagnostic about empty skill, got: {errors:?}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p belt-core --test lint_test lint_detects_invoke_skill lint_accepts_invoke_skill 2>&1
```

Expected: FAIL on the "bad" cases.

- [ ] **Step 3: Add the lint rule**

In `crates/belt-core/src/lint.rs`, after the invoke pipeline path check from Task 6, add:

```rust
    // Check: invoke.skill must start with '/' and be non-empty
    for phase in &pipeline.phases {
        if let Some(crate::model::Invoker::Skill { skill, .. }) = &phase.invoke {
            if skill.is_empty() {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Error,
                    message: format!("phase '{}': invoke skill is empty", phase.id),
                });
            } else if !skill.starts_with('/') {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Error,
                    message: format!(
                        "phase '{}': invoke skill '{}' must start with a leading slash (e.g. '/{}')",
                        phase.id, skill, skill
                    ),
                });
            }
        }
    }
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p belt-core --test lint_test lint_detects_invoke_skill lint_accepts_invoke_skill 2>&1
```

Expected: 3 tests pass.

- [ ] **Step 5: Full test suite + clippy**

```bash
cargo test -p belt-core 2>&1 && \
cargo clippy -p belt-core -- -D warnings 2>&1 && \
cargo fmt --package belt-core -- --check 2>&1
```

Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/lint.rs crates/belt-core/tests/lint_test.rs
git commit -m "feat(belt-core): lint rule for Invoker::Skill slash format (BELT-32)"
```

---

## Task 8: Lint rule — `produces:` uniqueness and `consumes:` resolution

**Files:**
- Modify: `crates/belt-core/src/lint.rs`
- Modify: `crates/belt-core/tests/lint_test.rs`

- [ ] **Step 1: Write failing tests**

Add to `crates/belt-core/tests/lint_test.rs`:

```rust
#[test]
fn lint_detects_duplicate_produces_name_in_one_phase() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: dup-produces
version: 1
phases:
  - id: p
    description: "p"
    produces:
      - name: doc
        path: "a.md"
      - name: doc
        path: "b.md"
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("duplicate") && d.message.contains("doc")),
        "expected duplicate produces name error, got: {errors:?}"
    );
}

#[test]
fn lint_detects_unresolved_consumes_named() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: unresolved-consumes
version: 1
phases:
  - id: first
    description: "first"
    produces:
      - name: design_doc
        path: "design.md"
  - id: second
    description: "second"
    consumes:
      - phantom_artifact
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("phantom_artifact") && d.message.contains("consumes")),
        "expected unresolved consumes error, got: {errors:?}"
    );
}

#[test]
fn lint_detects_unresolved_consumes_qualified_unknown_phase() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: unresolved-qualified
version: 1
phases:
  - id: first
    description: "first"
    produces:
      - name: doc
        path: "doc.md"
  - id: second
    description: "second"
    consumes:
      - name: doc
        from: nonexistent
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("nonexistent") && d.message.contains("consumes")),
        "expected unresolved qualified consumes error, got: {errors:?}"
    );
}

#[test]
fn lint_accepts_consumes_resolved_to_earlier_phase() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: good-consumes
version: 1
phases:
  - id: design
    description: "d"
    produces:
      - name: design_doc
        path: "design.md"
  - id: plan
    description: "p"
    consumes:
      - design_doc
  - id: review
    description: "r"
    consumes:
      - name: design_doc
        from: design
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

#[test]
fn lint_detects_consumes_from_later_phase() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad-forward-consumes
version: 1
phases:
  - id: early
    description: "e"
    consumes:
      - late_doc
  - id: late
    description: "l"
    produces:
      - name: late_doc
        path: "late.md"
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("late_doc")),
        "expected error about consuming later phase's output, got: {errors:?}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p belt-core --test lint_test lint_detects_duplicate_produces lint_detects_unresolved_consumes lint_accepts_consumes lint_detects_consumes_from_later 2>&1
```

Expected: FAIL on all "bad" cases.

- [ ] **Step 3: Add the lint rules**

In `crates/belt-core/src/lint.rs`, after the invoke.skill check from Task 7, add:

```rust
    // Check: produces names are unique within a phase
    for phase in &pipeline.phases {
        let mut seen_names: HashSet<&str> = HashSet::new();
        for artifact in &phase.produces {
            if !seen_names.insert(artifact.name.as_str()) {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Error,
                    message: format!(
                        "phase '{}': duplicate produces name '{}'",
                        phase.id, artifact.name
                    ),
                });
            }
        }
    }

    // Check: consumes references resolve to an earlier phase's produces
    // Build an index of (phase_index, name) → phase_id for all produces.
    let mut produces_index: std::collections::HashMap<String, Vec<(usize, String)>> =
        std::collections::HashMap::new();
    for (i, phase) in pipeline.phases.iter().enumerate() {
        for artifact in &phase.produces {
            produces_index
                .entry(artifact.name.clone())
                .or_default()
                .push((i, phase.id.clone()));
        }
    }

    for (i, phase) in pipeline.phases.iter().enumerate() {
        for consumed in &phase.consumes {
            match consumed {
                crate::model::ArtifactRef::Named(name) => {
                    let found = produces_index
                        .get(name)
                        .map(|locs| locs.iter().any(|(j, _)| *j < i))
                        .unwrap_or(false);
                    if !found {
                        diagnostics.push(LintDiagnostic {
                            severity: Severity::Error,
                            message: format!(
                                "phase '{}': consumes '{}' not produced by any earlier phase",
                                phase.id, name
                            ),
                        });
                    }
                }
                crate::model::ArtifactRef::Qualified { name, from } => {
                    let found = produces_index
                        .get(name)
                        .map(|locs| {
                            locs.iter()
                                .any(|(j, phase_id)| phase_id == from && *j < i)
                        })
                        .unwrap_or(false);
                    if !found {
                        diagnostics.push(LintDiagnostic {
                            severity: Severity::Error,
                            message: format!(
                                "phase '{}': consumes {{name: '{}', from: '{}'}} not found in earlier phases",
                                phase.id, name, from
                            ),
                        });
                    }
                }
            }
        }
    }
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p belt-core --test lint_test lint_detects_duplicate_produces lint_detects_unresolved_consumes lint_accepts_consumes lint_detects_consumes_from_later 2>&1
```

Expected: 5 tests pass.

- [ ] **Step 5: Full test suite + clippy**

```bash
cargo test -p belt-core 2>&1 && \
cargo clippy -p belt-core -- -D warnings 2>&1 && \
cargo fmt --package belt-core -- --check 2>&1
```

Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/lint.rs crates/belt-core/tests/lint_test.rs
git commit -m "feat(belt-core): lint rules for produces uniqueness and consumes resolution (BELT-32)"
```

---

## Task 9: Lint rule — `validate: { file: ... }` file existence

**Files:**
- Modify: `crates/belt-core/src/lint.rs`
- Modify: `crates/belt-core/tests/lint_test.rs`

- [ ] **Step 1: Write failing test**

Add to `crates/belt-core/tests/lint_test.rs`:

```rust
#[test]
fn lint_detects_validate_file_missing() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad-validate-file
version: 1
phases:
  - id: p
    description: "p"
    validate:
      - file: ./nonexistent-criteria.md
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("nonexistent-criteria.md") && d.message.contains("validate")),
        "expected validate file missing error, got: {errors:?}"
    );
}

#[test]
fn lint_accepts_validate_file_present() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Write a criteria file.
    write_yaml(
        &dir,
        "criteria.md",
        "# Criteria\n\n- C1: placeholder\n",
    );

    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: good-validate-file
version: 1
phases:
  - id: p
    description: "p"
    validate:
      - file: ./criteria.md
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

#[test]
fn lint_validate_inline_strings_unaffected() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: inline-validate
version: 1
phases:
  - id: p
    description: "p"
    validate:
      - "inline criterion 1"
      - "inline criterion 2"
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p belt-core --test lint_test lint_detects_validate_file lint_accepts_validate_file lint_validate_inline 2>&1
```

Expected: FAIL on `lint_detects_validate_file_missing`.

- [ ] **Step 3: Add the lint rule**

In `crates/belt-core/src/lint.rs`, after the consumes resolution check from Task 8, add:

```rust
    // Check: validate file references exist
    for phase in &pipeline.phases {
        for v in &phase.validate {
            if let crate::model::ValidationSource::File { file } = v {
                let resolved = base_dir.join(file);
                if !resolved.exists() {
                    diagnostics.push(LintDiagnostic {
                        severity: Severity::Error,
                        message: format!(
                            "phase '{}': validate file '{}' not found",
                            phase.id, file
                        ),
                    });
                }
            }
        }
    }
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p belt-core --test lint_test lint_detects_validate_file lint_accepts_validate_file lint_validate_inline 2>&1
```

Expected: 3 tests pass.

- [ ] **Step 5: Full test suite + clippy**

```bash
cargo test -p belt-core 2>&1 && \
cargo clippy -p belt-core -- -D warnings 2>&1 && \
cargo fmt --package belt-core -- --check 2>&1
```

Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/lint.rs crates/belt-core/tests/lint_test.rs
git commit -m "feat(belt-core): lint rule for validate file reference existence (BELT-32)"
```

---

## Task 10: Extend `view::PhaseView` with `invoke`, `produces`, `consumes`

**Files:**
- Modify: `crates/belt-core/src/view.rs`
- Modify: `crates/belt-core/tests/view_test.rs`

- [ ] **Step 1: Write failing test**

Add to `crates/belt-core/tests/view_test.rs`:

```rust
use belt_core::model::{Artifact, ArtifactRef, Invoker};
use belt_core::view::{PhaseView, build_status_view};

/// PhaseView serializes `invoke` as a nested JSON object when present.
#[test]
fn phase_view_serializes_invoke_skill() {
    // This test constructs a PhaseView via build_status_view and verifies
    // JSON output contains the invoke field.
    //
    // Setup: fake RunState + phase ids + empty run dir.
    let state = belt_core::model::RunState {
        run_id: "01961234-0000-7000-8000-000000000000".to_string(),
        pipeline: "test".to_string(),
        pipeline_file: "pipeline.yml".to_string(),
        version: 1,
        args: std::collections::HashMap::new(),
        current_phase: "design".to_string(),
        completed_phases: vec![],
        skipped_phases: vec![],
        phase_attempts: std::collections::HashMap::new(),
        phase_verify_passed: std::collections::HashMap::new(),
        regate_passed: std::collections::HashMap::new(),
        created_at: "2026-04-11T00:00:00Z".to_string(),
        updated_at: "2026-04-11T00:00:00Z".to_string(),
    };

    // For this test, we need a version of build_status_view that accepts
    // typed phase data (including invoke/produces/consumes). The current
    // signature only takes phase_ids: &[String]. We extend it to accept a
    // richer type — see Step 3.
    //
    // Using a temporary directory for run_dir:
    let dir = tempfile::TempDir::new().expect("tempdir");

    // New build_status_view signature (proposed in Step 3):
    // fn build_status_view(
    //     state: &RunState,
    //     phases: &[PhaseMetadata],
    //     run_dir: &Path,
    // ) -> StatusView;
    //
    // Where PhaseMetadata carries id + invoke + produces + consumes.

    use belt_core::view::PhaseMetadata;
    let phases = vec![PhaseMetadata {
        id: "design".to_string(),
        invoke: Some(Invoker::Skill {
            skill: "/brainstorming".to_string(),
            args: std::collections::HashMap::new(),
        }),
        produces: vec![Artifact {
            name: "design_doc".to_string(),
            path: "docs/plans/*-design.md".to_string(),
            description: Some("design".to_string()),
        }],
        consumes: vec![],
    }];

    let view = build_status_view(&state, &phases, dir.path());

    assert_eq!(view.phases.len(), 1);
    assert_eq!(view.phases[0].id, "design");
    assert!(view.phases[0].invoke.is_some(), "expected invoke in PhaseView");
    assert_eq!(view.phases[0].produces.len(), 1);
    assert_eq!(view.phases[0].produces[0].name, "design_doc");
    assert!(view.phases[0].consumes.is_empty());

    // JSON round-trip check.
    let json = serde_json::to_string(&view).expect("serialize");
    assert!(json.contains("\"invoke\""));
    assert!(json.contains("\"skill\":\"/brainstorming\""));
    assert!(json.contains("\"produces\""));
    assert!(json.contains("\"design_doc\""));
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p belt-core --test view_test phase_view_serializes_invoke 2>&1
```

Expected: FAIL with compilation errors (`PhaseMetadata` doesn't exist, `PhaseView.invoke` doesn't exist, `PhaseView.produces` doesn't exist).

- [ ] **Step 3: Add `PhaseMetadata` and extend `PhaseView` in `view.rs`**

In `crates/belt-core/src/view.rs`, add a new `PhaseMetadata` struct at the top (after imports):

```rust
use crate::model::{Artifact, ArtifactRef, Invoker};

// ... existing use lines ...

/// Subset of ExpandedPhase fields used by `build_status_view` to enrich the
/// per-phase view. Callers (engine) pass this alongside RunState to produce
/// a StatusView with typed invoke/produces/consumes data.
#[derive(Debug, Clone)]
pub struct PhaseMetadata {
    pub id: String,
    pub invoke: Option<Invoker>,
    pub produces: Vec<Artifact>,
    pub consumes: Vec<ArtifactRef>,
}
```

Extend `PhaseView` (around line 43):

```rust
#[derive(Debug, Serialize)]
pub struct PhaseView {
    pub id: String,
    pub status: PhaseState,
    pub verify_passed: Option<bool>,
    pub regate_passed: Option<bool>,
    pub attempt: u32,
    pub outputs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify_checks: Option<Vec<GateResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regate_checks: Option<serde_json::Value>,
    // NEW: typed invocation and artifact data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoke: Option<Invoker>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub produces: Vec<Artifact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consumes: Vec<ArtifactRef>,
}
```

- [ ] **Step 4: Update `build_status_view` signature to accept `PhaseMetadata`**

In `crates/belt-core/src/view.rs`, change the signature of `build_status_view` (around line 119):

```rust
/// Build an enriched status view from `RunState` + phase metadata + run directory.
#[must_use]
pub fn build_status_view(
    state: &RunState,
    phases_meta: &[PhaseMetadata],
    run_dir: &Path,
) -> StatusView {
    let is_completed = state.current_phase == COMPLETED_SENTINEL;

    let mut phases: Vec<PhaseView> = phases_meta
        .iter()
        .map(|meta| {
            let status = if is_completed && state.completed_phases.contains(&meta.id) {
                PhaseState::Completed
            } else {
                determine_phase_state(&meta.id, state)
            };
            PhaseView {
                id: meta.id.clone(),
                status,
                verify_passed: state.phase_verify_passed.get(&meta.id).copied(),
                regate_passed: state.regate_passed.get(&meta.id).copied(),
                attempt: state.phase_attempts.get(&meta.id).copied().unwrap_or(0),
                outputs: scan_phase_outputs(run_dir, &meta.id),
                verify_checks: read_verify_checks(run_dir, &meta.id),
                regate_checks: read_regate_checks(run_dir, &meta.id),
                invoke: meta.invoke.clone(),
                produces: meta.produces.clone(),
                consumes: meta.consumes.clone(),
            }
        })
        .collect();

    // Append orphan phases (in state but removed from YAML).
    let yaml_ids: HashSet<&String> = phases_meta.iter().map(|m| &m.id).collect();
    for id in &state.completed_phases {
        if !yaml_ids.contains(id) {
            phases.push(PhaseView {
                id: id.clone(),
                status: PhaseState::Completed,
                verify_passed: state.phase_verify_passed.get(id).copied(),
                regate_passed: state.regate_passed.get(id).copied(),
                attempt: state.phase_attempts.get(id).copied().unwrap_or(0),
                outputs: scan_phase_outputs(run_dir, id),
                verify_checks: read_verify_checks(run_dir, id),
                regate_checks: read_regate_checks(run_dir, id),
                invoke: None,
                produces: Vec::new(),
                consumes: Vec::new(),
            });
        }
    }
    for id in &state.skipped_phases {
        if !yaml_ids.contains(id) && !state.completed_phases.contains(id) {
            phases.push(PhaseView {
                id: id.clone(),
                status: PhaseState::Skipped,
                verify_passed: None,
                regate_passed: None,
                attempt: 0,
                outputs: Vec::new(),
                verify_checks: None,
                regate_checks: None,
                invoke: None,
                produces: Vec::new(),
                consumes: Vec::new(),
            });
        }
    }

    // ... (rest of the function, progress computation, StatusView construction, unchanged)
    let completed = phases
        .iter()
        .filter(|p| p.status == PhaseState::Completed)
        .count();
    let skipped = phases
        .iter()
        .filter(|p| p.status == PhaseState::Skipped)
        .count();
    let total = phases.len();
    let remaining = total - completed - skipped;

    StatusView {
        run_id: state.run_id.clone(),
        pipeline: state.pipeline.clone(),
        pipeline_file: state.pipeline_file.clone(),
        version: state.version,
        args: state.args.clone(),
        status: if is_completed {
            PipelineStatus::Completed
        } else {
            PipelineStatus::InProgress
        },
        current_phase: if is_completed {
            None
        } else {
            Some(state.current_phase.clone())
        },
        progress: Progress {
            completed,
            skipped,
            remaining,
            total,
        },
        phases,
        created_at: state.created_at.clone(),
        updated_at: state.updated_at.clone(),
    }
}
```

- [ ] **Step 5: Fix callers of `build_status_view`**

The signature change from `phase_ids: &[String]` to `phases_meta: &[PhaseMetadata]` breaks the engine. Run:

```bash
cargo build -p belt-core 2>&1
```

Find and update call sites. The engine likely calls this from `engine.rs`. Update it to construct `Vec<PhaseMetadata>` from the expanded phases:

In `crates/belt-core/src/engine.rs`, find calls to `build_status_view(state, &phase_ids, ...)` and replace with:

```rust
let phases_meta: Vec<belt_core::view::PhaseMetadata> = expanded
    .iter()
    .map(|ep| belt_core::view::PhaseMetadata {
        id: ep.id.clone(),
        invoke: ep.invoke.clone(),
        produces: ep.produces.clone(),
        consumes: ep.consumes.clone(),
    })
    .collect();

let view = build_status_view(state, &phases_meta, run_dir);
```

(Exact path depends on engine internal names. Use `cargo build` errors as a guide.)

- [ ] **Step 6: Run the new test**

```bash
cargo test -p belt-core --test view_test phase_view_serializes_invoke 2>&1
```

Expected: test passes.

- [ ] **Step 7: Run full test suite + clippy + fmt**

```bash
cargo test -p belt-core 2>&1 && \
cargo test -p belt-agent 2>&1 && \
cargo clippy -p belt-core -- -D warnings 2>&1 && \
cargo clippy -p belt-agent -- -D warnings 2>&1 && \
cargo fmt --package belt-core -- --check 2>&1 && \
cargo fmt --package belt-agent -- --check 2>&1
```

Expected: all green. If view_test has other tests that used the old `build_status_view(state, &phase_ids, ...)` signature, update them to construct `PhaseMetadata` with empty `invoke`, `produces`, `consumes`.

- [ ] **Step 8: Commit**

```bash
git add crates/belt-core/src/view.rs crates/belt-core/src/engine.rs crates/belt-core/tests/view_test.rs
git commit -m "feat(belt-core): extend PhaseView with invoke/produces/consumes (BELT-32)"
```

---

## Task 11: Integration test — full pipeline round-trip with new types

**Files:**
- Modify: `crates/belt-core/tests/parser_test.rs`

- [ ] **Step 1: Write a comprehensive integration test**

Add to `crates/belt-core/tests/parser_test.rs`:

```rust
/// BELT-32 integration: a complete pipeline exercising Invoker, Artifact,
/// ArtifactRef, and ValidationSource together. Verifies parse + lint-clean
/// on a valid example and parses round-trip correctly.
#[test]
fn belt32_full_pipeline_with_all_new_types() {
    use belt_core::lint::{Severity, lint_pipeline};
    use belt_core::model::{Artifact, ArtifactRef, Invoker, Pipeline, ValidationSource};
    use belt_core::parser::parse_pipeline;
    use std::io::Write;
    use tempfile::TempDir;

    let dir = TempDir::new().expect("tempdir");

    // A criteria file for the validate file-ref.
    let criteria_path = dir.path().join("criteria.md");
    let mut f = std::fs::File::create(&criteria_path).expect("create criteria");
    f.write_all(b"# Criteria\n- C1: placeholder\n")
        .expect("write criteria");

    // A sub-pipeline for Invoker::Pipeline variant.
    let sub_path = dir.path().join("review.yml");
    let mut f = std::fs::File::create(&sub_path).expect("create sub");
    f.write_all(
        br#"
name: review
version: 1
phases:
  - id: vote
    description: "vote on findings"
"#,
    )
    .expect("write sub");

    // Main pipeline with all new types.
    let pipeline_path = dir.path().join("pipeline.yml");
    let mut f = std::fs::File::create(&pipeline_path).expect("create pipeline");
    f.write_all(
        br#"
name: belt32-full
version: 1
phases:
  - id: design
    description: "Design"
    invoke:
      skill: /brainstorming
      args:
        swarm: true
    produces:
      - name: design_doc
        path: "docs/plans/*-design.md"
        description: "Brainstormed design"
    validate:
      - file: ./criteria.md
      - "inline manual check"
    confirm: true

  - id: spec-review
    description: "Spec review sub-pipeline"
    invoke:
      pipeline: ./review.yml
    consumes:
      - design_doc
    produces:
      - name: review_findings
        path: "{output_dir}/findings.json"

  - id: execute
    description: "Execute"
    invoke:
      skill: /subagent-driven-development
    consumes:
      - design_doc
      - name: review_findings
        from: spec-review
"#,
    )
    .expect("write pipeline");

    // 1. Parse succeeds.
    let pipeline: Pipeline = parse_pipeline(&pipeline_path).expect("parse should succeed");
    assert_eq!(pipeline.phases.len(), 3);

    // 2. Design phase: Invoker::Skill, produces, mixed validate.
    let design = &pipeline.phases[0];
    assert_eq!(design.id, "design");
    assert!(matches!(
        design.invoke,
        Some(Invoker::Skill { ref skill, .. }) if skill == "/brainstorming"
    ));
    assert_eq!(design.produces.len(), 1);
    assert_eq!(design.produces[0].name, "design_doc");
    assert_eq!(design.validate.len(), 2);
    assert!(matches!(
        &design.validate[0],
        ValidationSource::File { file } if file == "./criteria.md"
    ));
    assert!(matches!(
        &design.validate[1],
        ValidationSource::Inline(s) if s == "inline manual check"
    ));
    assert!(design.confirm);

    // 3. Spec-review phase: Invoker::Pipeline, consumes, produces.
    let spec_review = &pipeline.phases[1];
    assert_eq!(spec_review.id, "spec-review");
    assert!(matches!(
        spec_review.invoke,
        Some(Invoker::Pipeline { ref pipeline, .. }) if pipeline == "./review.yml"
    ));
    assert_eq!(spec_review.consumes.len(), 1);
    assert!(matches!(
        &spec_review.consumes[0],
        ArtifactRef::Named(s) if s == "design_doc"
    ));
    assert_eq!(spec_review.produces.len(), 1);
    assert_eq!(spec_review.produces[0].name, "review_findings");

    // 4. Execute phase: Invoker::Skill, mixed consumes (Named + Qualified).
    let execute = &pipeline.phases[2];
    assert_eq!(execute.id, "execute");
    assert!(matches!(
        execute.invoke,
        Some(Invoker::Skill { ref skill, .. }) if skill == "/subagent-driven-development"
    ));
    assert_eq!(execute.consumes.len(), 2);
    assert!(matches!(
        &execute.consumes[0],
        ArtifactRef::Named(s) if s == "design_doc"
    ));
    assert!(matches!(
        &execute.consumes[1],
        ArtifactRef::Qualified { name, from }
            if name == "review_findings" && from == "spec-review"
    ));

    // 5. Lint: no errors expected.
    let diagnostics = lint_pipeline(&pipeline_path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "expected no lint errors, got: {errors:?}"
    );
}
```

- [ ] **Step 2: Run the integration test**

```bash
cargo test -p belt-core --test parser_test belt32_full_pipeline 2>&1
```

Expected: test passes. If it fails due to parse error, inspect the YAML and correct formatting. If it fails due to lint errors, inspect the diagnostics and fix either the lint rule or the pipeline YAML.

- [ ] **Step 3: Run full test suite one more time**

```bash
cargo test --workspace 2>&1
```

Expected: all tests pass across belt-core, belt, and belt-agent.

- [ ] **Step 4: Full clippy and fmt across workspace**

```bash
cargo clippy --workspace -- -D warnings 2>&1 && \
cargo fmt --all -- --check 2>&1
```

Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/tests/parser_test.rs
git commit -m "test(belt-core): BELT-32 integration test for full pipeline with new types"
```

---

## Task 12: Backwards compatibility smoke test — existing examples still lint clean

**Files:**
- No new files; runs lint against existing examples.

- [ ] **Step 1: Lint every existing example pipeline**

Run:

```bash
for p in examples/skills/*/pipeline.yml; do
  echo "=== Linting $p ==="
  cargo run -p belt -- lint --config "$(dirname $p)/belt.toml" 2>&1
done
```

Expected: every example lints clean (no errors). If any example has errors, it is a regression — investigate and fix by either adjusting the new lint rule or the model backwards compatibility.

**Candidates for review:**
- `examples/skills/feature-dev/pipeline.yml` — uses `config.skill`, `uses:`, `artifacts:` (legacy field preservation)
- `examples/skills/debug-flow/pipeline.yml` — same patterns
- `examples/skills/smoke-test/pipeline.yml` — uses `config.skill` + `config.reference`
- `examples/skills/spec-review/pipeline.yml` — uses `config.agents`
- `examples/skills/code-review/pipeline.yml` — uses `config.agents`
- `examples/skills/test-review/pipeline.yml` — uses `config.agents`
- `examples/skills/implementation-review/pipeline.yml` — uses `config.agents`
- `examples/skills/audit-gate/pipeline.yml` — uses `config.audit: required`

- [ ] **Step 2: Run a full `belt-agent init` / `next` / `status` loop on one legacy example**

```bash
cd /tmp
mkdir -p belt32-smoke-test && cd belt32-smoke-test
cargo run -p belt-agent -- --config /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/smoke-test/belt.toml init 2>&1
cargo run -p belt-agent -- --config /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/smoke-test/belt.toml next 2>&1
cargo run -p belt-agent -- --config /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/smoke-test/belt.toml status 2>&1
```

Expected:
- `init` creates a new run successfully.
- `next` returns the first phase (env-setup) with its legacy `config.skill` / `config.reference` fields intact.
- `status` returns a full StatusView JSON including the new `invoke`, `produces`, `consumes` fields (which will be empty / null for legacy examples).

If any command fails with a panic or error referencing the new types, the backwards compatibility is broken — investigate.

Clean up:
```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
rm -rf /tmp/belt32-smoke-test
```

- [ ] **Step 3: Commit the verification note (no code change)**

If everything passes, there is nothing to commit. If any small fixes were needed (e.g., a lint rule adjustment), commit those:

```bash
# Only if changes were made:
git add -u
git commit -m "fix(belt-core): BELT-32 backwards compat smoke test adjustments"
```

---

## Self-Review

After completing all 12 tasks, verify against this checklist.

### 1. Spec coverage

Walk through `docs/specs/2026-04-11-belt-action-data-first-class.md` section by section. For each spec requirement, identify which task implements it.

| Spec Section | Implementing Task(s) |
|--------------|----------------------|
| DD-1: Invoker enum | Task 4 |
| DD-2: Artifact / produces / consumes | Tasks 2, 3 |
| DD-3: ValidationSource (validate file ref) | Task 1 |
| DD-4: `uses:` absorbed into `Invoker::Pipeline` | Task 5 (additive; full absorption is Plan B) |
| DD-5: `config` remains opaque | (implicit: no task removes config) |
| DD-6: Forward-compat for thin fields | (implicit: no task touches confirm/max_retries/when/regate) |
| Lint: invoker shape | Tasks 6, 7 |
| Lint: consumes resolution | Task 8 |
| Lint: produces uniqueness | Task 8 |
| Lint: validate file existence | Task 9 |
| Engine/view JSON enrichment | Task 10 |
| Backwards compatibility | Task 12 |
| Examples migration | **Plan B** (explicitly out of scope) |
| audit-gate deletion | **Plan B** |
| belt-agent/SKILL.md update | **Plan B** |

All spec requirements in Phase 1 scope have an implementing task.

### 2. Placeholder scan

Search this plan for red flags:

- "TBD", "TODO", "implement later", "fill in details" — none present.
- "Add appropriate error handling" / "add validation" — none present.
- "Write tests for the above" without test code — none present (every test has full code).
- "Similar to Task N" without repeating code — none present.
- Code blocks showing how for every implementation step — verified.

### 3. Type consistency

| Type/Method | Tasks that reference it | Consistent? |
|-------------|--------------------------|-------------|
| `ValidationSource::Inline(String)` | Tasks 1, 9, 11 | ✓ |
| `ValidationSource::File { file: String }` | Tasks 1, 9, 11 | ✓ (field name `file` used consistently) |
| `Artifact { name, path, description }` | Tasks 2, 8, 10, 11 | ✓ |
| `ArtifactRef::Named(String)` | Tasks 3, 8, 11 | ✓ |
| `ArtifactRef::Qualified { name, from }` | Tasks 3, 8, 11 | ✓ (field names `name`, `from` used consistently) |
| `Invoker::Skill { skill, args }` | Tasks 4, 7, 10, 11 | ✓ |
| `Invoker::Agent { agent, args }` | Tasks 4, 11 | ✓ |
| `Invoker::Agents { agents, iterations, args }` | Task 4 | ✓ |
| `Invoker::Pipeline { pipeline, with }` | Tasks 4, 5, 6, 11 | ✓ |
| `Phase.invoke: Option<Invoker>` | Tasks 4, 5, 6, 7, 10, 11 | ✓ |
| `Phase.produces: Vec<Artifact>` | Tasks 2, 8, 10, 11 | ✓ |
| `Phase.consumes: Vec<ArtifactRef>` | Tasks 3, 8, 10, 11 | ✓ |
| `Phase.validate: Vec<ValidationSource>` | Tasks 1, 9, 11 | ✓ |
| `PhaseMetadata` struct | Task 10 | ✓ (introduced, fields match usage) |
| `build_status_view(state, phases_meta, run_dir)` | Task 10 (new signature); old callers fixed in same task | ✓ |

No inconsistencies.

### 4. Task ordering soundness

- Task 1 (ValidationSource + Phase.validate type change) — can be done first; does not depend on other new types.
- Task 2 (Artifact + Phase.produces) — does not depend on Task 1.
- Task 3 (ArtifactRef + Phase.consumes) — depends on Task 2 (no, actually only on `Artifact` type existing; but the test references `produces` from Task 2 for setup, so depends on Task 2 logically).
- Task 4 (Invoker + Phase.invoke) — independent of Tasks 1-3.
- Task 5 (expander Invoker::Pipeline) — depends on Task 4.
- Task 6 (lint invoker.pipeline path) — depends on Task 4.
- Task 7 (lint invoker.skill slash) — depends on Task 4.
- Task 8 (lint produces/consumes) — depends on Tasks 2, 3.
- Task 9 (lint validate file) — depends on Task 1.
- Task 10 (view PhaseView extension) — depends on Tasks 2, 3, 4.
- Task 11 (integration test) — depends on all prior tasks.
- Task 12 (backwards compat smoke) — depends on all prior tasks.

Order: 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 is valid.

Optimization: tasks 1, 2, 3, 4 can be parallelized in a worktree-per-task setup (subagent-driven-development supports this), but sequential execution in a single worktree is also fine.

---

## Summary

12 tasks, each with 5-8 bite-sized steps, covering:
- 4 new model types (`ValidationSource`, `Artifact`, `ArtifactRef`, `Invoker`)
- 3 new `Phase` / `ExpandedPhase` fields (`invoke`, `produces`, `consumes`) plus one field type extension (`validate`)
- 4 new lint rules
- Expander extension for `Invoker::Pipeline`
- View module extension for enriched status output
- Integration test and backwards-compatibility smoke test

Expected total: ~1000 LOC of code changes across `belt-core` (matching the spec's impact analysis). All changes are additive; legacy fields (`artifacts`, phase-level `uses`, `config.skill`) continue to work unchanged.

**After this plan completes:** Plan B (examples migration) becomes unblocked. Plan B scope:
- Migrate all 7 example skills to use `invoke:`, `produces:`, `consumes:`, `validate: file`
- Delete `examples/skills/audit-gate/`
- Update `skills/belt-agent/SKILL.md` protocol documentation
- Eventually remove legacy `artifacts:` and phase-level `uses:` fields (Phase 3 of migration strategy)
