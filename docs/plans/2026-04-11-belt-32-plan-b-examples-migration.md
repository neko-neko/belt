# BELT-32 Plan B: Examples Migration and Legacy Cutover — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate all seven example skills in `examples/skills/` to the Plan A types (`Invoker`, `Artifact`, `ArtifactRef`, `ValidationSource`), delete the `audit-gate` sub-pipeline scaffolding, remove legacy fields (`Phase.artifacts`, phase-level `Phase.uses`), and update `skills/belt-agent/SKILL.md` to the new protocol.

**Architecture:** TDD and subagent-driven-development per Plan A. Two `belt-core` implementation tasks (phase-start mtime filter, validate scalar shorthand) ship first, then six migration tasks run on leaf skills (smoke-test, four review skills) and two on orchestrator skills (feature-dev, debug-flow). The legacy cutover is a single atomic sub-task that deletes `examples/skills/audit-gate/` and removes the legacy fields from `belt-core::model::Phase` in one commit, gated by an automated grep check that confirms no example still references the legacy shapes. The final sub-task rewrites `skills/belt-agent/SKILL.md` and runs the end-to-end smoke test.

**Tech Stack:** Rust 2024, `serde 1.0.228`, `serde-saphyr =0.0.23` (pinned), `miette 7.6`, `tempfile` for tests, `chrono 0.4` (for `DateTime<Utc>` in `phase_start_times`). MSRV 1.86.0, CI toolchain 1.94.1.

**Linear:** [BELT-32](https://linear.app/neko-neko/issue/BELT-32)
**Parent spec**: `docs/specs/2026-04-11-belt-action-data-first-class.md`
**Plan B spec**: `docs/specs/2026-04-11-belt-32-plan-b-examples-migration.md`
**Plan A (completed)**: `docs/plans/2026-04-11-belt-action-data-first-class-plan.md`
**Parent issue**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)

---

## Scope

### In scope (this plan)

- `belt-core::engine` — add `RunState.phase_start_times: HashMap<PhaseId, DateTime<Utc>>`; write it in `step()` on phase transition.
- `belt-core::view` — use `phase_start_times` when resolving glob paths in `Artifact.path`.
- `belt-core::model` — add `deserialize_with` for `Phase.validate` to support scalar shorthand.
- `belt-core::model` — remove `Phase.artifacts: Vec<String>` and `Phase.uses: Option<String>` (phase-level); also remove the same fields from `ExpandedPhase`.
- `belt-core::expander` — simplify by removing phase-level `uses:` handling (only `Invoker::Pipeline` path remains).
- `belt-core::lint` — remove lint rules that referenced legacy fields; add lint rule that rejects empty phases (no `invoke:`, `gate:`, `validate:`, or `confirm: true`).
- `examples/criteria/` — create shared canonical done-criteria directory.
- `examples/references/audit-protocol.md` — new location for the shared audit protocol document.
- `examples/skills/smoke-test/` — `pipeline.yml` migration, `criteria/` not required (existing validate is inline).
- `examples/skills/spec-review/`, `code-review/`, `test-review/`, `implementation-review/` — `pipeline.yml` migration.
- `examples/skills/feature-dev/` — `pipeline.yml` migration, `SKILL.md` dispatch-rule table shrinks, `criteria/` directory with skill-specific files.
- `examples/skills/debug-flow/` — `pipeline.yml` migration, `SKILL.md` dispatch-rule table shrinks, `criteria/` directory.
- `examples/skills/audit-gate/` — **deleted**.
- `skills/belt-agent/SKILL.md` — add "Reading phase.invoke", "Artifact graph in status", "Validate file semantics" sections; remove `config.skill` from "Well-known Config Keys"; add `max_retries` row.

### Out of scope (deferred by Plan B spec)

- BELT-28 (`on_escalation`).
- Agent/Agents unification.
- `Skill.reference` promotion out of `args:`.
- `confirm:` / `when:` / `regate:` type expansion.
- Remote `uses:` (YAML Universe).
- New CLI commands.
- Deprecation warnings for legacy fields.

### Key design decisions locked by the Plan B spec

- **DD-1** Glob resolution is phase-start mtime filter. Timestamp is set once per phase entry in `step()`. Retries within a phase do not update it. `regate` is an in-place check and does not modify any phase's `phase_start_times` (spec DD-1).
- **DD-2** `validate:` scalar shorthand: a YAML scalar starting with `./` or `/` deserializes to `vec![ValidationSource::File]`; any other scalar to `vec![ValidationSource::Inline]`; list form is unchanged. The heuristic applies only to scalars, not to list items (spec DD-2).
- **DD-3** `max_retries` semantics are documented only; no new mechanism. Every `verify_verdict` call increments `phase_attempts[current_phase]`; `regate` does not touch the counter; earlier phases' counters are never modified (spec DD-3).
- **DD-4** Legacy field removal is immediate cutover in sub-task 11 (this plan's Task 11). No deprecation warnings (spec DD-4).
- **DD-5** `examples/criteria/` holds canonical shared done-criteria; `{skill}/criteria/` holds skill-specific ones; `examples/references/audit-protocol.md` is the shared audit protocol location. For overlap between `audit-gate/done-criteria/*.md` and `feature-dev/references/done-criteria/*.md`, use the **feature-dev version** as the shared canonical because it has more specific paths (e.g., `docs/plans/*-plan.md` rather than `docs/plans/`) and has been field-tested (spec DD-5).
- **DD-6** Twelve sub-tasks with the dependency graph in the spec (spec DD-6).
- **DD-7** `ArtifactRef::Qualified` remains struct-only (spec DD-7).
- **DD-8** A phase without `invoke:` is legal if it has `gate:`, `validate:`, or `confirm: true`. An "empty" phase (all four missing) is a lint error (spec DD-8).

---

## File Structure

### Modified files (belt-core)

- `crates/belt-core/src/model.rs` — Remove legacy `Phase.artifacts` and `Phase.uses`; remove same from `ExpandedPhase`. Add `deserialize_with` attribute to `Phase.validate` and `ExpandedPhase.validate`. Keep new types unchanged.
- `crates/belt-core/src/engine.rs` — Add `phase_start_times` field to `RunState`; write it in `step()` on transition; make it accessible via `enriched_status`.
- `crates/belt-core/src/view.rs` — Accept `phase_start_times` in `build_status_view()`; use it to filter globs in `Artifact.path` resolution. Return concrete resolved paths in the status JSON.
- `crates/belt-core/src/expander.rs` — Remove handling of phase-level `uses:` (Plan A's `Invoker::Pipeline` already handles sub-pipelines via `invoke:`). Keep `GateCheck::Uses` handling intact (gate-level `uses:` is preserved).
- `crates/belt-core/src/lint.rs` — Remove `check_gate_uses_exist`'s phase-level `uses:` branch (gate-level branch stays). Add `check_empty_phase` rule. Remove any rules that only applied to legacy fields.

### Modified files (belt-core tests)

- `crates/belt-core/tests/model_test.rs` — Add tests for `validate:` scalar shorthand (scalar → File, scalar → Inline, list unchanged). Remove tests for legacy `Phase.artifacts` and `Phase.uses`.
- `crates/belt-core/tests/engine_test.rs` — Add tests for `phase_start_times` lifecycle (set on entry, unchanged on retry, not touched by regate). Update tests that depended on legacy fields.
- `crates/belt-core/tests/view_test.rs` — Add tests for glob resolution via `phase_start_times` (single match, multiple → newest, equal mtime → alphabetical, zero matches → missing).
- `crates/belt-core/tests/expander_test.rs` — Remove legacy `uses:` test fixtures; keep `Invoker::Pipeline` tests.
- `crates/belt-core/tests/lint_test.rs` — Add `check_empty_phase` tests (empty → error, only-gate → ok, only-validate → ok, only-confirm → ok). Remove legacy-field-targeted tests.
- `crates/belt-core/tests/parser_test.rs` — Update any integration tests that used legacy shapes.

### Created files (examples)

- `examples/criteria/execute.md` — copy of `feature-dev/references/done-criteria/execute.md`.
- `examples/criteria/code-review.md` — copy of `feature-dev/references/done-criteria/code-review.md`.
- `examples/criteria/smoke-test.md` — copy of `feature-dev/references/done-criteria/smoke-test.md`.
- `examples/criteria/test-review.md` — copy of `feature-dev/references/done-criteria/test-review.md`.
- `examples/criteria/_schema.md` — copy of `audit-gate/done-criteria/_schema.md`.
- `examples/references/audit-protocol.md` — copy of `audit-gate/references/audit-protocol.md`.
- `examples/skills/feature-dev/criteria/design.md` — move from `feature-dev/references/done-criteria/design.md`.
- `examples/skills/feature-dev/criteria/plan.md` — move from same directory.
- `examples/skills/feature-dev/criteria/plan-review.md` — move.
- `examples/skills/feature-dev/criteria/spec-review.md` — move.
- `examples/skills/feature-dev/criteria/doc-audit.md` — move.
- `examples/skills/debug-flow/criteria/rca.md` — move from `debug-flow/references/done-criteria/rca.md`.
- `examples/skills/debug-flow/criteria/fix-plan.md` — move.
- `examples/skills/debug-flow/criteria/fix-plan-review.md` — move.

### Modified files (examples)

- `examples/skills/smoke-test/pipeline.yml` — migrate to `invoke:` + `produces:` + scalar-shorthand `validate:`.
- `examples/skills/spec-review/pipeline.yml` — same.
- `examples/skills/code-review/pipeline.yml` — same.
- `examples/skills/test-review/pipeline.yml` — same.
- `examples/skills/implementation-review/pipeline.yml` — same.
- `examples/skills/feature-dev/pipeline.yml` — same + audit-phase collapse + `invoke: { pipeline: ... }` replacement for phase-level `uses:`.
- `examples/skills/feature-dev/SKILL.md` — shrink dispatch-rule table.
- `examples/skills/debug-flow/pipeline.yml` — same as feature-dev.
- `examples/skills/debug-flow/SKILL.md` — shrink dispatch-rule table.
- `skills/belt-agent/SKILL.md` — add three new sections; rewrite `Decision Rules` table; remove `config.skill` from `Well-known Config Keys`; add `max_retries` row.

### Deleted files (Task 11)

- `examples/skills/audit-gate/` — entire directory including `pipeline.yml`, `done-criteria/`, `references/`.
- `examples/skills/feature-dev/references/done-criteria/` — after contents have been moved to either `examples/criteria/` or `examples/skills/feature-dev/criteria/`.
- `examples/skills/debug-flow/references/done-criteria/` — similarly.

### Files NOT modified

- `crates/belt/src/main.rs` — the `belt lint` binary picks up rule changes automatically via `belt-core::lint`.
- `crates/belt-agent/src/main.rs` — JSON output flows through `belt-core::view`, which picks up `phase_start_times` automatically.
- `Cargo.toml`, workspace lints — no dependency changes required. `chrono` is already a transitive dependency via `serde_yaml` paths, but we may need to add it to `belt-core`'s direct dependencies; verify in Task 2.

---

## Task 1: Consolidate done-criteria into `examples/criteria/` and `examples/references/`

**Files:**
- Create: `examples/criteria/execute.md`
- Create: `examples/criteria/code-review.md`
- Create: `examples/criteria/smoke-test.md`
- Create: `examples/criteria/test-review.md`
- Create: `examples/criteria/_schema.md`
- Create: `examples/references/audit-protocol.md`
- Create: `examples/skills/feature-dev/criteria/design.md` (moved from `references/done-criteria/`)
- Create: `examples/skills/feature-dev/criteria/plan.md` (moved)
- Create: `examples/skills/feature-dev/criteria/plan-review.md` (moved)
- Create: `examples/skills/feature-dev/criteria/spec-review.md` (moved)
- Create: `examples/skills/feature-dev/criteria/doc-audit.md` (moved)
- Create: `examples/skills/debug-flow/criteria/rca.md` (moved)
- Create: `examples/skills/debug-flow/criteria/fix-plan.md` (moved)
- Create: `examples/skills/debug-flow/criteria/fix-plan-review.md` (moved)

Note: this task does **not** delete the source files under `references/done-criteria/` yet. Those are removed in Task 11 after the YAML migrations have switched to the new paths.

- [ ] **Step 1: Inspect the overlap and pick the canonical copies**

Run:

```bash
diff -q examples/skills/audit-gate/done-criteria/execute.md examples/skills/feature-dev/references/done-criteria/execute.md
diff -q examples/skills/audit-gate/done-criteria/code-review.md examples/skills/feature-dev/references/done-criteria/code-review.md
diff -q examples/skills/audit-gate/done-criteria/smoke-test.md examples/skills/feature-dev/references/done-criteria/smoke-test.md
diff -q examples/skills/audit-gate/done-criteria/test-review.md examples/skills/feature-dev/references/done-criteria/test-review.md
```

Expected: all four pairs differ. The canonical choice per Plan B spec DD-5 is the feature-dev version (more specific paths, field-tested). This step confirms the expectation and documents any surprise.

If any pair is byte-identical, use that content. If they differ, proceed with the feature-dev version.

- [ ] **Step 2: Create the shared `examples/criteria/` directory by copying feature-dev's canonical done-criteria**

Run:

```bash
mkdir -p examples/criteria
cp examples/skills/feature-dev/references/done-criteria/execute.md examples/criteria/execute.md
cp examples/skills/feature-dev/references/done-criteria/code-review.md examples/criteria/code-review.md
cp examples/skills/feature-dev/references/done-criteria/smoke-test.md examples/criteria/smoke-test.md
cp examples/skills/feature-dev/references/done-criteria/test-review.md examples/criteria/test-review.md
cp examples/skills/audit-gate/done-criteria/_schema.md examples/criteria/_schema.md
```

Verify:

```bash
ls examples/criteria/
```

Expected output:
```
_schema.md
code-review.md
execute.md
smoke-test.md
test-review.md
```

- [ ] **Step 3: Create the shared `examples/references/` directory by copying audit-gate's audit-protocol**

Run:

```bash
mkdir -p examples/references
cp examples/skills/audit-gate/references/audit-protocol.md examples/references/audit-protocol.md
```

Verify:

```bash
ls examples/references/
```

Expected output:
```
audit-protocol.md
```

- [ ] **Step 4: Create `examples/skills/feature-dev/criteria/` and move feature-dev-specific done-criteria**

Run:

```bash
mkdir -p examples/skills/feature-dev/criteria
cp examples/skills/feature-dev/references/done-criteria/design.md examples/skills/feature-dev/criteria/design.md
cp examples/skills/feature-dev/references/done-criteria/plan.md examples/skills/feature-dev/criteria/plan.md
cp examples/skills/feature-dev/references/done-criteria/plan-review.md examples/skills/feature-dev/criteria/plan-review.md
cp examples/skills/feature-dev/references/done-criteria/spec-review.md examples/skills/feature-dev/criteria/spec-review.md
cp examples/skills/feature-dev/references/done-criteria/doc-audit.md examples/skills/feature-dev/criteria/doc-audit.md
```

Verify:

```bash
ls examples/skills/feature-dev/criteria/
```

Expected output:
```
design.md
doc-audit.md
plan-review.md
plan.md
spec-review.md
```

Note: `execute.md`, `code-review.md`, `smoke-test.md`, `test-review.md` are **not** in this list — they are in the shared `examples/criteria/` directory. `feature-dev/pipeline.yml` will reference the shared ones with `../../criteria/{name}.md`.

- [ ] **Step 5: Create `examples/skills/debug-flow/criteria/` and move debug-flow-specific done-criteria**

Run:

```bash
mkdir -p examples/skills/debug-flow/criteria
cp examples/skills/debug-flow/references/done-criteria/rca.md examples/skills/debug-flow/criteria/rca.md
cp examples/skills/debug-flow/references/done-criteria/fix-plan.md examples/skills/debug-flow/criteria/fix-plan.md
cp examples/skills/debug-flow/references/done-criteria/fix-plan-review.md examples/skills/debug-flow/criteria/fix-plan-review.md
```

Verify:

```bash
ls examples/skills/debug-flow/criteria/
```

Expected output:
```
fix-plan-review.md
fix-plan.md
rca.md
```

- [ ] **Step 6: Verify all canonical and specific criteria files are in place**

Run:

```bash
echo "--- shared criteria ---"
ls examples/criteria/
echo "--- shared references ---"
ls examples/references/
echo "--- feature-dev specific ---"
ls examples/skills/feature-dev/criteria/
echo "--- debug-flow specific ---"
ls examples/skills/debug-flow/criteria/
```

Expected: each directory lists exactly the files above. No file is in two places where both will survive; the overlap files (execute, code-review, smoke-test, test-review) exist only in `examples/criteria/` at this point — the source copies under `feature-dev/references/done-criteria/` and `debug-flow/references/done-criteria/` are left in place until Task 11.

- [ ] **Step 7: Commit**

```bash
git add examples/criteria/ examples/references/ examples/skills/feature-dev/criteria/ examples/skills/debug-flow/criteria/
git commit -m "$(cat <<'EOF'
docs(examples): consolidate done-criteria into shared + skill-specific dirs

Introduces examples/criteria/ for shared canonical done-criteria
(execute, code-review, smoke-test, test-review, _schema) and
examples/references/audit-protocol.md for the shared audit protocol.
Moves feature-dev and debug-flow specific done-criteria into
{skill}/criteria/. Uses feature-dev's versions of the shared files as
canonical because they have more specific artifact paths and have been
field-tested. Source copies under references/done-criteria/ are not
yet deleted; they are removed in task 11 (legacy cutover) after all
pipeline.yml migrations reference the new paths.

BELT-32 Plan B task 1.
EOF
)"
```

---

## Task 2: Implement phase-start mtime filter in `belt-core`

**Files:**
- Modify: `crates/belt-core/src/engine.rs`
- Modify: `crates/belt-core/src/model.rs` (for `RunState.phase_start_times`)
- Modify: `crates/belt-core/src/view.rs`
- Modify: `crates/belt-core/tests/engine_test.rs`
- Modify: `crates/belt-core/tests/view_test.rs`
- Modify: `Cargo.toml` workspace dependencies (if `chrono` is not already a direct dep of `belt-core`)

- [ ] **Step 1: Verify `chrono` availability for `DateTime<Utc>`**

Run:

```bash
cargo tree -p belt-core | grep chrono
```

Expected: either `chrono` appears as a transitive dependency, or nothing is printed. In either case, we add it as a direct dependency in the next step.

Inspect `crates/belt-core/Cargo.toml`:

```bash
grep -n 'chrono' crates/belt-core/Cargo.toml
```

If no line appears, `chrono` is not a direct dependency of `belt-core` and must be added.

- [ ] **Step 2: Add `chrono` to workspace and `belt-core` dependencies**

Open `Cargo.toml` at the workspace root. Find the `[workspace.dependencies]` table and add:

```toml
chrono = { version = "0.4", default-features = false, features = ["std", "serde", "clock"] }
```

Place it in alphabetical order with the other entries.

Then open `crates/belt-core/Cargo.toml` and add to `[dependencies]`:

```toml
chrono = { workspace = true }
```

Also in alphabetical order. Run:

```bash
cargo check -p belt-core
```

Expected: compiles successfully; no new warnings.

- [ ] **Step 3: Write failing test for `phase_start_times` lifecycle**

Add to `crates/belt-core/tests/engine_test.rs` (at the end of the file):

```rust
use chrono::{DateTime, Utc};

/// phase_start_times is set when step() first enters a phase.
/// It is not touched by retries within the same phase.
/// regate does not modify any phase's phase_start_times.
#[test]
fn phase_start_times_is_set_on_entry_not_updated_on_retry() {
    let temp = tempfile::tempdir().unwrap();
    let belt_dir = temp.path().join(".belt");
    let pipeline_path = temp.path().join("pipeline.yml");
    std::fs::write(
        &pipeline_path,
        r#"
name: t
version: 1
phases:
  - id: first
    description: first
    gate:
      - cmd: "true"
  - id: second
    description: second
    gate:
      - cmd: "true"
    max_retries: 3
"#,
    )
    .unwrap();

    let engine = belt_core::engine::Engine::new(belt_dir.clone()).unwrap();
    let run_id = engine
        .init(&pipeline_path, std::collections::HashMap::new())
        .unwrap();
    let mut state = engine.load_state(&run_id).unwrap();

    // First phase entry time recorded via init.
    let first_entry = state
        .phase_start_times
        .get("first")
        .copied()
        .expect("first phase should have a start time set on init");

    // Simulate a verify PASS and step to next phase.
    engine.verify_verdict(&mut state, true).unwrap();
    engine.step(&mut state, &pipeline_path).unwrap();

    // second should now have a start time; first should be unchanged.
    let second_entry = state
        .phase_start_times
        .get("second")
        .copied()
        .expect("second phase should have a start time after step()");
    let first_after = state.phase_start_times.get("first").copied().unwrap();
    assert_eq!(
        first_entry, first_after,
        "first phase start time must not change after leaving it"
    );
    assert!(
        second_entry >= first_after,
        "second phase start time must be at or after first's"
    );

    // Retry on second: verify FAIL, verify PASS, each should preserve second's start time.
    engine.verify_verdict(&mut state, false).unwrap();
    let second_after_fail = state.phase_start_times.get("second").copied().unwrap();
    assert_eq!(
        second_entry, second_after_fail,
        "retry (verify FAIL) must not update phase_start_times"
    );

    engine.verify_verdict(&mut state, true).unwrap();
    let second_after_pass = state.phase_start_times.get("second").copied().unwrap();
    assert_eq!(
        second_entry, second_after_pass,
        "retry (verify PASS) must not update phase_start_times"
    );
}

/// phase_start_times uses UTC and serialises round-trip via state.json.
#[test]
fn phase_start_times_round_trips_through_state_json() {
    let temp = tempfile::tempdir().unwrap();
    let belt_dir = temp.path().join(".belt");
    let pipeline_path = temp.path().join("pipeline.yml");
    std::fs::write(
        &pipeline_path,
        r#"
name: t
version: 1
phases:
  - id: only
    description: only phase
"#,
    )
    .unwrap();

    let engine = belt_core::engine::Engine::new(belt_dir.clone()).unwrap();
    let run_id = engine
        .init(&pipeline_path, std::collections::HashMap::new())
        .unwrap();
    let state = engine.load_state(&run_id).unwrap();

    let written = state.phase_start_times.get("only").copied().unwrap();

    // Reload from disk.
    let reloaded = engine.load_state(&run_id).unwrap();
    let read_back: DateTime<Utc> = reloaded
        .phase_start_times
        .get("only")
        .copied()
        .expect("phase_start_times must persist");
    assert_eq!(written.to_rfc3339(), read_back.to_rfc3339());
}
```

- [ ] **Step 4: Run the new tests to confirm they fail**

```bash
cargo test -p belt-core --test engine_test phase_start_times 2>&1
```

Expected: compilation error — `RunState` has no field `phase_start_times`.

- [ ] **Step 5: Add `phase_start_times` to `RunState`**

Open `crates/belt-core/src/model.rs`. Find the `RunState` struct. Add a new field with `#[serde(default)]` so pre-existing state.json files deserialize cleanly:

```rust
// Add to the imports at the top of model.rs:
use chrono::{DateTime, Utc};
```

Then inside `RunState`, add the field (place it just after `updated_at` or any other timestamp-ish field):

```rust
#[serde(default)]
pub phase_start_times: std::collections::HashMap<String, DateTime<Utc>>,
```

Also update the constructor / `Default` impl if any. If `RunState` has a custom `::new()` or similar helper, initialise `phase_start_times: HashMap::new()`.

- [ ] **Step 6: Write `phase_start_times[phase_id] = now_utc()` on init and step**

Open `crates/belt-core/src/engine.rs`. In `Engine::init()`, immediately before saving the new `RunState`, insert the timestamp for the starting phase:

```rust
// Inside init(), after constructing the RunState:
let now = chrono::Utc::now();
state.phase_start_times.insert(state.current_phase.clone(), now);
```

In `Engine::step()`, after the current phase has been marked completed and the next phase id has been determined, insert the timestamp for the next phase (only if next_phase is Some):

```rust
// Inside step(), after state.current_phase.clone_from(next_id) and std::fs::create_dir_all(&output_dir)?;
// but before the phase_verify_passed auto-set for gate-less phases:
let now = chrono::Utc::now();
state.phase_start_times.insert(next_id.clone(), now);
```

- [ ] **Step 7: Run the lifecycle test to confirm it passes**

```bash
cargo test -p belt-core --test engine_test phase_start_times 2>&1
```

Expected: both tests pass.

- [ ] **Step 8: Write failing test for glob resolution in `build_status_view`**

Add to `crates/belt-core/tests/view_test.rs` (at the end of the file):

```rust
use chrono::{DateTime, Utc};

/// Glob resolution picks the newest matching file after the phase start time.
#[test]
fn glob_resolution_picks_newest_after_phase_start() {
    let temp = tempfile::tempdir().unwrap();
    let run_dir = temp.path().join("run");
    std::fs::create_dir_all(&run_dir).unwrap();

    // Create two files. The older one must be ignored because it was
    // created before the phase started.
    let older = temp.path().join("docs-plans-2026-01-01-old-design.md");
    let newer = temp.path().join("docs-plans-2026-04-11-new-design.md");
    std::fs::write(&older, "older").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    let phase_start: DateTime<Utc> = Utc::now();
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&newer, "newer").unwrap();

    let glob_pattern = format!("{}/docs-plans-*-design.md", temp.path().display());

    // Build a tiny PhaseMetadata with a produces entry using the glob.
    let metadata = vec![belt_core::view::PhaseMetadata {
        id: "design".to_string(),
        invoke: None,
        produces: vec![belt_core::model::Artifact {
            name: "design_doc".to_string(),
            path: glob_pattern.clone(),
            description: None,
        }],
        consumes: vec![],
    }];

    let mut phase_start_times = std::collections::HashMap::new();
    phase_start_times.insert("design".to_string(), phase_start);

    let state = belt_core::model::RunState {
        run_id: "test-run".to_string(),
        pipeline_file: "/tmp/pipeline.yml".to_string(),
        current_phase: "design".to_string(),
        completed_phases: vec!["design".to_string()],
        skipped_phases: vec![],
        args: std::collections::HashMap::new(),
        phase_attempts: std::collections::HashMap::new(),
        phase_verify_passed: Default::default(),
        regate_passed: Default::default(),
        phase_start_times,
        created_at: "".to_string(),
        updated_at: "".to_string(),
    };

    let view = belt_core::view::build_status_view(&state, &metadata, &run_dir);
    let design = view
        .phases
        .iter()
        .find(|p| p.id == "design")
        .expect("design phase in view");

    // Find the resolved produces entry. The resolved path must be the newer file.
    let resolved_produces = design
        .produces
        .as_ref()
        .expect("produces present on design phase");
    let design_doc = resolved_produces
        .iter()
        .find(|a| a.name == "design_doc")
        .expect("design_doc artifact");
    assert!(design_doc.exists, "design_doc must resolve");
    assert_eq!(
        design_doc.resolved_path.as_deref(),
        Some(newer.to_str().unwrap()),
        "must pick the newer file (older was created before phase_start)"
    );
}

/// If no files match the glob after the filter, existence is false.
#[test]
fn glob_resolution_zero_matches_reports_missing() {
    let temp = tempfile::tempdir().unwrap();
    let run_dir = temp.path().join("run");
    std::fs::create_dir_all(&run_dir).unwrap();

    let phase_start: DateTime<Utc> = Utc::now();
    // Create a file BEFORE phase_start — it must be filtered out.
    std::thread::sleep(std::time::Duration::from_millis(20));
    let stale = temp.path().join("stale.md");
    std::fs::write(&stale, "stale").unwrap();

    // Advance phase_start to just now, after the stale file creation.
    let phase_start = Utc::now();
    std::thread::sleep(std::time::Duration::from_millis(20));

    let glob_pattern = format!("{}/*.md", temp.path().display());

    let metadata = vec![belt_core::view::PhaseMetadata {
        id: "p".to_string(),
        invoke: None,
        produces: vec![belt_core::model::Artifact {
            name: "missing_doc".to_string(),
            path: glob_pattern,
            description: None,
        }],
        consumes: vec![],
    }];

    let mut phase_start_times = std::collections::HashMap::new();
    phase_start_times.insert("p".to_string(), phase_start);

    let state = belt_core::model::RunState {
        run_id: "test-run".to_string(),
        pipeline_file: "/tmp/pipeline.yml".to_string(),
        current_phase: "p".to_string(),
        completed_phases: vec!["p".to_string()],
        skipped_phases: vec![],
        args: std::collections::HashMap::new(),
        phase_attempts: std::collections::HashMap::new(),
        phase_verify_passed: Default::default(),
        regate_passed: Default::default(),
        phase_start_times,
        created_at: "".to_string(),
        updated_at: "".to_string(),
    };

    let view = belt_core::view::build_status_view(&state, &metadata, &run_dir);
    let p = view
        .phases
        .iter()
        .find(|ph| ph.id == "p")
        .expect("p phase in view");
    let produces = p.produces.as_ref().expect("produces list");
    let missing = produces.iter().find(|a| a.name == "missing_doc").unwrap();
    assert!(!missing.exists, "no matches after phase_start → exists=false");
    assert!(missing.resolved_path.is_none());
}

/// Equal mtimes break ties via ascending filename.
#[test]
fn glob_resolution_equal_mtime_alphabetical_tiebreaker() {
    let temp = tempfile::tempdir().unwrap();
    let run_dir = temp.path().join("run");
    std::fs::create_dir_all(&run_dir).unwrap();

    let phase_start: DateTime<Utc> = Utc::now();
    std::thread::sleep(std::time::Duration::from_millis(20));

    // Two files created in one burst so their mtimes are likely identical.
    let b = temp.path().join("b.md");
    let a = temp.path().join("a.md");
    std::fs::write(&b, "b").unwrap();
    std::fs::write(&a, "a").unwrap();

    // Force identical mtimes using filetime so the tiebreaker is exercised.
    let same = filetime::FileTime::now();
    filetime::set_file_mtime(&a, same).unwrap();
    filetime::set_file_mtime(&b, same).unwrap();

    let glob_pattern = format!("{}/*.md", temp.path().display());

    let metadata = vec![belt_core::view::PhaseMetadata {
        id: "p".to_string(),
        invoke: None,
        produces: vec![belt_core::model::Artifact {
            name: "doc".to_string(),
            path: glob_pattern,
            description: None,
        }],
        consumes: vec![],
    }];

    let mut phase_start_times = std::collections::HashMap::new();
    phase_start_times.insert("p".to_string(), phase_start);

    let state = belt_core::model::RunState {
        run_id: "test-run".to_string(),
        pipeline_file: "/tmp/pipeline.yml".to_string(),
        current_phase: "p".to_string(),
        completed_phases: vec!["p".to_string()],
        skipped_phases: vec![],
        args: std::collections::HashMap::new(),
        phase_attempts: std::collections::HashMap::new(),
        phase_verify_passed: Default::default(),
        regate_passed: Default::default(),
        phase_start_times,
        created_at: "".to_string(),
        updated_at: "".to_string(),
    };

    let view = belt_core::view::build_status_view(&state, &metadata, &run_dir);
    let produces = view
        .phases
        .iter()
        .find(|p| p.id == "p")
        .unwrap()
        .produces
        .as_ref()
        .unwrap();
    let doc = produces.iter().find(|x| x.name == "doc").unwrap();
    assert_eq!(
        doc.resolved_path.as_deref(),
        Some(a.to_str().unwrap()),
        "equal mtimes → alphabetical 'a.md' wins over 'b.md'"
    );
}

/// Concrete (non-glob) path uses std::fs::metadata directly.
#[test]
fn concrete_path_skips_filter() {
    let temp = tempfile::tempdir().unwrap();
    let run_dir = temp.path().join("run");
    std::fs::create_dir_all(&run_dir).unwrap();

    let concrete = temp.path().join("smoke-test-report.md");
    std::fs::write(&concrete, "report").unwrap();

    // phase_start is AFTER the file creation — this test proves that
    // concrete paths bypass the mtime filter entirely.
    std::thread::sleep(std::time::Duration::from_millis(20));
    let phase_start: DateTime<Utc> = Utc::now();

    let metadata = vec![belt_core::view::PhaseMetadata {
        id: "smoke".to_string(),
        invoke: None,
        produces: vec![belt_core::model::Artifact {
            name: "report".to_string(),
            path: concrete.to_str().unwrap().to_string(),
            description: None,
        }],
        consumes: vec![],
    }];

    let mut phase_start_times = std::collections::HashMap::new();
    phase_start_times.insert("smoke".to_string(), phase_start);

    let state = belt_core::model::RunState {
        run_id: "test-run".to_string(),
        pipeline_file: "/tmp/pipeline.yml".to_string(),
        current_phase: "smoke".to_string(),
        completed_phases: vec!["smoke".to_string()],
        skipped_phases: vec![],
        args: std::collections::HashMap::new(),
        phase_attempts: std::collections::HashMap::new(),
        phase_verify_passed: Default::default(),
        regate_passed: Default::default(),
        phase_start_times,
        created_at: "".to_string(),
        updated_at: "".to_string(),
    };

    let view = belt_core::view::build_status_view(&state, &metadata, &run_dir);
    let report = view
        .phases
        .iter()
        .find(|p| p.id == "smoke")
        .unwrap()
        .produces
        .as_ref()
        .unwrap()
        .iter()
        .find(|a| a.name == "report")
        .unwrap();
    assert!(report.exists, "concrete path must exist regardless of mtime");
    assert_eq!(
        report.resolved_path.as_deref(),
        Some(concrete.to_str().unwrap())
    );
}
```

- [ ] **Step 9: Add `filetime` test dependency if not already present**

Check `crates/belt-core/Cargo.toml` for a `[dev-dependencies]` section with `filetime`:

```bash
grep -A5 'dev-dependencies' crates/belt-core/Cargo.toml
```

If `filetime` is not listed, add it:

```toml
[dev-dependencies]
# ... existing entries ...
filetime = "0.2"
```

Run `cargo check -p belt-core --tests` to confirm.

- [ ] **Step 10: Run the new view tests to confirm they fail**

```bash
cargo test -p belt-core --test view_test glob_resolution 2>&1
cargo test -p belt-core --test view_test concrete_path_skips_filter 2>&1
```

Expected: compile errors because `PhaseView.produces` is not yet an enriched `Vec<ResolvedArtifact>` with `exists` and `resolved_path` fields.

- [ ] **Step 11: Extend the view types with a resolved artifact representation**

Open `crates/belt-core/src/view.rs`. Near the existing `PhaseView` definition, add:

```rust
/// An artifact produced by a phase with runtime-resolved filesystem state.
/// This extends `model::Artifact` with the concrete resolved path (glob
/// filtering applied) and an existence flag that the orchestrator uses
/// to populate the artifact graph in the status output.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedArtifact {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_path: Option<String>,
}
```

In `PhaseView`, change the `produces` field type from `Option<Vec<model::Artifact>>` (or whatever Plan A defined) to `Option<Vec<ResolvedArtifact>>`. Also update any serde `skip_serializing_if` so that the field is still omitted for phases without produces.

Run `cargo check -p belt-core --tests` and fix any immediate type errors (expect errors in `build_status_view` where the old `Artifact` vec was returned).

- [ ] **Step 12: Implement glob resolution in `build_status_view`**

Open `crates/belt-core/src/view.rs`. Find `build_status_view()`. It accepts `phase_meta: &[PhaseMetadata]` and a `run_dir: &Path`. Extend it so that when building each `PhaseView`, the `produces` field is constructed by resolving each `model::Artifact` into a `ResolvedArtifact` using the phase's `phase_start_times` entry.

Insert a helper above `build_status_view`:

```rust
/// Resolve a single `model::Artifact` to its runtime state.
/// - If the path contains no glob metacharacters, use `std::fs::metadata` directly.
/// - Otherwise, enumerate matches via `glob::glob()`, filter by phase-start mtime,
///   and pick the newest. On ties, pick the lexicographically smallest filename.
/// - Returns `exists: false` and `resolved_path: None` if no file matches.
fn resolve_artifact(
    artifact: &crate::model::Artifact,
    phase_start: Option<chrono::DateTime<chrono::Utc>>,
) -> ResolvedArtifact {
    use chrono::{DateTime, Utc};
    use std::time::SystemTime;

    let is_glob = artifact.path.contains('*')
        || artifact.path.contains('?')
        || artifact.path.contains('[');

    let (exists, resolved_path) = if is_glob {
        let entries = match glob::glob(&artifact.path) {
            Ok(iter) => iter,
            Err(_) => {
                return ResolvedArtifact {
                    name: artifact.name.clone(),
                    path: artifact.path.clone(),
                    description: artifact.description.clone(),
                    exists: false,
                    resolved_path: None,
                };
            }
        };
        let mut candidates: Vec<(SystemTime, std::path::PathBuf)> = Vec::new();
        for entry in entries.flatten() {
            if let Ok(meta) = std::fs::metadata(&entry) {
                if let Ok(mtime) = meta.modified() {
                    // Filter by phase_start if provided.
                    if let Some(start) = phase_start {
                        let mtime_dt = DateTime::<Utc>::from(mtime);
                        if mtime_dt < start {
                            continue;
                        }
                    }
                    candidates.push((mtime, entry));
                }
            }
        }
        if candidates.is_empty() {
            (false, None)
        } else {
            // Newest mtime first; on ties, ascending filename.
            candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
            // But descending by mtime + ascending filename on tie means the
            // winner is the one with the largest mtime. For ties, we need
            // ascending filename, so flip the comparator explicitly:
            candidates.sort_by(|a, b| match b.0.cmp(&a.0) {
                std::cmp::Ordering::Equal => a.1.cmp(&b.1),
                non_equal => non_equal,
            });
            let (_, path) = candidates.into_iter().next().unwrap();
            (true, Some(path.to_string_lossy().to_string()))
        }
    } else {
        let exists = std::fs::metadata(&artifact.path).is_ok();
        let resolved = if exists {
            Some(artifact.path.clone())
        } else {
            None
        };
        (exists, resolved)
    };

    ResolvedArtifact {
        name: artifact.name.clone(),
        path: artifact.path.clone(),
        description: artifact.description.clone(),
        exists,
        resolved_path,
    }
}
```

Then in `build_status_view`, for each phase, collect `produces: Option<Vec<ResolvedArtifact>>` by calling `resolve_artifact` over `phase_meta.produces`, passing `state.phase_start_times.get(&phase.id).copied()` as the second argument.

- [ ] **Step 13: Update `build_status_view` signature to accept `&RunState`**

The helper needs `phase_start_times`, which is inside `RunState`. Confirm the existing `build_status_view` signature already takes `&RunState`; if so, no signature change is required. Otherwise, add `state: &RunState` as the first parameter and update all call sites in `engine::enriched_status`.

Run:

```bash
cargo check -p belt-core --tests
```

Expected: compiles cleanly.

- [ ] **Step 14: Run the view tests to confirm they pass**

```bash
cargo test -p belt-core --test view_test glob_resolution 2>&1
cargo test -p belt-core --test view_test concrete_path_skips_filter 2>&1
```

Expected: all four tests pass.

- [ ] **Step 15: Run the full `belt-core` test suite to catch regressions**

```bash
cargo test -p belt-core 2>&1
```

Expected: all tests pass. No existing test should break; the new `ResolvedArtifact` type replaces `Artifact` in the view output, but the JSON shape only **adds** `exists` and `resolved_path` fields. Plan A's existing view tests may need to be updated if they asserted on the exact `produces` vec type — if so, update the assertions to match the new type (read the test, update the type reference, re-run).

- [ ] **Step 16: Run clippy and fmt**

```bash
cargo clippy -p belt-core -- -D warnings 2>&1
cargo fmt --package belt-core -- --check 2>&1
```

Expected: clean. If `cargo fmt --check` reports a diff, run `cargo fmt --package belt-core` and include the formatting fixes in the commit.

- [ ] **Step 17: Commit**

```bash
git add Cargo.toml crates/belt-core/Cargo.toml crates/belt-core/src/engine.rs crates/belt-core/src/model.rs crates/belt-core/src/view.rs crates/belt-core/tests/engine_test.rs crates/belt-core/tests/view_test.rs
git commit -m "$(cat <<'EOF'
feat(belt-core): phase-start mtime filter for Artifact glob resolution

Adds RunState.phase_start_times: HashMap<PhaseId, DateTime<Utc>> written
on phase entry in Engine::init and Engine::step. Extends view with
ResolvedArtifact (name, path, description, exists, resolved_path) and
a resolve_artifact helper that:
- uses std::fs::metadata for concrete paths
- enumerates glob matches, filters by mtime >= phase_start, picks newest
  with ascending-filename tiebreaker

This is the first consumer of phase_start_times and replaces the Plan A
placeholder Vec<model::Artifact> view with runtime-resolved output.

BELT-32 Plan B task 2.
EOF
)"
```

---

## Task 3: Implement `validate:` scalar shorthand deserializer in `belt-core::model`

**Files:**
- Modify: `crates/belt-core/src/model.rs`
- Modify: `crates/belt-core/tests/model_test.rs`

- [ ] **Step 1: Write failing tests for the scalar shorthand behavior**

Add to `crates/belt-core/tests/model_test.rs`:

```rust
/// Top-level scalar starting with `./` is treated as a file reference.
#[test]
fn parse_validate_scalar_shorthand_relative_file() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: p
    validate: ./criteria/p.md
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].validate.len(), 1);
    match &pipeline.phases[0].validate[0] {
        ValidationSource::File { file } => assert_eq!(file, "./criteria/p.md"),
        other => panic!("expected File, got {other:?}"),
    }
}

/// Top-level scalar starting with `/` is treated as a file reference.
#[test]
fn parse_validate_scalar_shorthand_absolute_file() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: p
    validate: /abs/path/criteria.md
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    match &pipeline.phases[0].validate[0] {
        ValidationSource::File { file } => assert_eq!(file, "/abs/path/criteria.md"),
        other => panic!("expected File, got {other:?}"),
    }
}

/// Top-level scalar without path prefix is treated as inline criterion.
#[test]
fn parse_validate_scalar_shorthand_inline() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: p
    validate: "All checks pass"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    match &pipeline.phases[0].validate[0] {
        ValidationSource::Inline(s) => assert_eq!(s, "All checks pass"),
        other => panic!("expected Inline, got {other:?}"),
    }
}

/// Top-level scalar with a dot-prefix that is NOT a relative path ("." alone, "..foo", etc)
/// must NOT be promoted to File — the prefix match is strict: `./` or `/`.
#[test]
fn parse_validate_scalar_shorthand_non_path_dot_prefix() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: p
    validate: ".hidden criterion text"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    match &pipeline.phases[0].validate[0] {
        ValidationSource::Inline(s) => assert_eq!(s, ".hidden criterion text"),
        other => panic!("expected Inline, got {other:?}"),
    }
}

/// List form is unchanged: bare strings are Inline, even if they start with ./
#[test]
fn parse_validate_list_bare_string_stays_inline_even_with_dot_prefix() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: p
    validate:
      - "./should-be-inline-because-in-list"
      - file: "./actual-file.md"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].validate.len(), 2);
    match &pipeline.phases[0].validate[0] {
        ValidationSource::Inline(s) => assert_eq!(s, "./should-be-inline-because-in-list"),
        other => panic!("expected Inline, got {other:?}"),
    }
    match &pipeline.phases[0].validate[1] {
        ValidationSource::File { file } => assert_eq!(file, "./actual-file.md"),
        other => panic!("expected File, got {other:?}"),
    }
}

/// Empty list is accepted (existing behavior, no change).
#[test]
fn parse_validate_empty_list_still_parses() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: p
    validate: []
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("should parse");
    assert_eq!(pipeline.phases[0].validate.len(), 0);
}
```

- [ ] **Step 2: Run the new tests to confirm they fail (scalar form parse error)**

```bash
cargo test -p belt-core --test model_test parse_validate_scalar 2>&1
cargo test -p belt-core --test model_test parse_validate_list_bare 2>&1
cargo test -p belt-core --test model_test parse_validate_empty 2>&1
```

Expected: the scalar-form tests fail because serde cannot accept a scalar into a `Vec<ValidationSource>`. The list tests (bare and empty) should pass already.

- [ ] **Step 3: Add a custom deserializer for `Phase.validate`**

Open `crates/belt-core/src/model.rs`. Just below the `ValidationSource` enum definition, add the deserializer module:

```rust
mod validate_de {
    use super::ValidationSource;
    use serde::de::{Deserializer, SeqAccess, Visitor};
    use std::fmt;

    /// Custom deserializer for `Phase.validate`.
    ///
    /// Accepts either:
    /// - A YAML scalar (string): if the scalar starts with `./` or `/`, it
    ///   is wrapped as `vec![ValidationSource::File { file }]`; otherwise as
    ///   `vec![ValidationSource::Inline(s)]`.
    /// - A YAML sequence: delegates to the stock `Vec<ValidationSource>`
    ///   deserializer which in turn uses the untagged enum discrimination.
    ///
    /// This is the "scalar shorthand" described in Plan B spec DD-2.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<ValidationSource>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = Vec<ValidationSource>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a string or a sequence of validation sources")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(classify_scalar(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(classify_scalar(value))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut out = Vec::new();
                while let Some(item) = seq.next_element::<ValidationSource>()? {
                    out.push(item);
                }
                Ok(out)
            }
        }

        deserializer.deserialize_any(V)
    }

    /// Scalar classification rule: `./` or `/` prefix → File; else Inline.
    /// The prefix match is strict: only these two prefixes qualify.
    fn classify_scalar(s: String) -> Vec<ValidationSource> {
        if s.starts_with("./") || s.starts_with('/') {
            vec![ValidationSource::File { file: s }]
        } else {
            vec![ValidationSource::Inline(s)]
        }
    }
}
```

- [ ] **Step 4: Attach the deserializer to `Phase.validate` and `ExpandedPhase.validate`**

In `crates/belt-core/src/model.rs`, find `Phase.validate`:

```rust
#[serde(default)]
pub validate: Vec<ValidationSource>,
```

Change it to:

```rust
#[serde(default, deserialize_with = "validate_de::deserialize")]
pub validate: Vec<ValidationSource>,
```

And do the same for `ExpandedPhase.validate`.

- [ ] **Step 5: Run the new tests to confirm they pass**

```bash
cargo test -p belt-core --test model_test parse_validate_scalar 2>&1
cargo test -p belt-core --test model_test parse_validate_list_bare 2>&1
cargo test -p belt-core --test model_test parse_validate_empty 2>&1
```

Expected: all six tests pass.

- [ ] **Step 6: Run the full belt-core test suite**

```bash
cargo test -p belt-core 2>&1
```

Expected: all tests pass. Plan A's existing `parse_validate_inline_backwards_compat`, `parse_validate_file_reference`, and `parse_validate_mixed_inline_and_file` tests should continue to pass because they use the list form, which the new deserializer handles via `visit_seq`.

- [ ] **Step 7: Run clippy and fmt**

```bash
cargo clippy -p belt-core -- -D warnings 2>&1
cargo fmt --package belt-core -- --check 2>&1
```

Expected: clean.

- [ ] **Step 8: Commit**

```bash
git add crates/belt-core/src/model.rs crates/belt-core/tests/model_test.rs
git commit -m "$(cat <<'EOF'
feat(belt-core): add validate scalar shorthand deserializer

Phase.validate now accepts a top-level YAML scalar in addition to a
list. Scalars starting with `./` or `/` are classified as
ValidationSource::File; all other scalars as ValidationSource::Inline.
List form is unchanged; bare strings inside a list remain Inline even
if they start with ./, per spec DD-2.

Enables the common single-criterion-file pattern in migrated examples
(e.g., `validate: ./criteria/design.md`) without requiring the
two-line list+struct form.

BELT-32 Plan B task 3.
EOF
)"
```

---

## Task 4: Migrate `examples/skills/smoke-test/pipeline.yml`

**Files:**
- Modify: `examples/skills/smoke-test/pipeline.yml`

**Migration principle:** `config.skill` → `invoke.skill`; `config.reference` → `invoke.skill.args.reference`; `artifacts:` → `produces:`; `validate:` list stays as-is.

- [ ] **Step 1: Read the current pipeline for reference**

```bash
cat examples/skills/smoke-test/pipeline.yml
```

Expected output: matches the pre-migration shape documented in `docs/specs/2026-04-11-belt-32-plan-b-examples-migration.md` background section (four phases: `env-setup`, `adhoc-test`, `vrt-check`, `e2e-detection`).

- [ ] **Step 2: Rewrite the pipeline in the new format**

Overwrite `examples/skills/smoke-test/pipeline.yml` with exactly:

```yaml
name: smoke-test
version: 1
args:
  diff_base:    { type: string, default: "HEAD~1" }
  design:       { type: string, default: "" }
  server:       { type: string, default: "" }
  port:         { type: number, default: 0 }
  skip_vrt:     { type: bool, default: false }
  skip_e2e:     { type: bool, default: false }
  adhoc_only:   { type: bool, default: false }
  full_e2e:     { type: bool, default: false }
  perspectives: { type: string, default: "" }

phases:
  - id: env-setup
    description: "Start dev server and verify it is accessible."
    invoke:
      skill: /smoke-test
      args:
        reference: ./references/env-setup-procedure.md
    gate:
      - cmd: "curl -sf http://localhost:${args.port:-3000}/ > /dev/null"

  - id: adhoc-test
    description: "Generate and execute ad-hoc smoke test scenarios via browser."
    invoke:
      skill: /smoke-test
      args:
        reference: ./references/adhoc-test-procedure.md
    produces:
      - name: smoke_test_report
        path: smoke-test-report.md
        description: "Ad-hoc smoke test report with adversarial probes"
      - name: smoke_test_screenshots
        path: "smoke-*.png"
        description: "Screenshots captured during smoke test scenarios"
    gate:
      - file_exists: "smoke-test-report.md"
      - file_exists: "smoke-*.png"
    validate:
      - "At least one adversarial probe executed and documented in report"
      - "Test scenarios cover changes from diff (not just generic checks)"
    confirm: true

  - id: vrt-check
    description: "Run VRT diff check if VRT tooling is detected."
    when: "!args.skip_vrt"
    invoke:
      skill: /smoke-test
      args:
        reference: ./references/vrt-check-procedure.md

  - id: e2e-detection
    description: "Run E2E test suite with flaky detection (2-pass execution)."
    when: "!args.skip_e2e"
    invoke:
      skill: /smoke-test
      args:
        reference: ./references/e2e-detection-procedure.md
```

Notes on specific migration choices:
- `config: { skill, reference }` becomes `invoke: { skill: /smoke-test, args: { reference: ./... } }`. The leading `/` is required by `Invoker::Skill`'s slash-format lint rule (Plan A Task 7).
- `reference:` paths now begin with `./` so that the path is unambiguously relative to the pipeline.yml directory. This replaces the previous implicit "relative to skill directory" convention.
- `artifacts: ["smoke-test-report.md"]` (Plan A's legacy path) is replaced by two `produces:` entries, one for the report and one for the screenshots. The PNG glob is kept because smoke-test screenshots historically use runtime-chosen filenames.
- `validate:` stays in the list form because there are two inline criteria.

- [ ] **Step 3: Lint the migrated pipeline**

```bash
cargo run -p belt -- lint examples/skills/smoke-test/pipeline.yml
```

Expected: no errors. The Plan A `check_invoke_skill_format` rule accepts the leading-slash form; `check_artifact_flow` accepts the two unique produces names.

- [ ] **Step 4: Initialise a run to verify belt-agent accepts the file**

```bash
mkdir -p /tmp/belt-plan-b-task4
cd /tmp/belt-plan-b-task4
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/smoke-test/* .
cargo run -p belt-agent -- init pipeline.yml --arg skip_vrt=true --arg skip_e2e=true
```

Expected: `belt-agent` returns a JSON object containing `run_id`. No parse error.

Clean up:

```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
rm -rf /tmp/belt-plan-b-task4
```

- [ ] **Step 5: Commit**

```bash
git add examples/skills/smoke-test/pipeline.yml
git commit -m "$(cat <<'EOF'
refactor(example): migrate smoke-test to Invoker/produces format

Replaces config.skill + config.reference with invoke.skill + nested
args.reference. Replaces the single artifacts: entry with two
produces: entries (report + screenshots) so the artifact graph surfaces
both outputs in belt-agent status.

BELT-32 Plan B task 4.
EOF
)"
```

---

## Task 5: Migrate `examples/skills/spec-review/pipeline.yml`

**Files:**
- Modify: `examples/skills/spec-review/pipeline.yml`

**Migration principle:** `config.agents` → `invoke.agents`; `config.ui_agent` / `codex` / `iterations` / `swarm` stay inside `invoke.args` because they are qualifiers on how the Agents invocation runs, not on the identity of the agents.

- [ ] **Step 1: Rewrite the pipeline**

Overwrite `examples/skills/spec-review/pipeline.yml` with exactly:

```yaml
name: spec-review
version: 1
args:
  iterations: { type: number, default: 3 }
  codex: { type: bool, default: false }
  ui: { type: bool, default: false }
  swarm: { type: bool, default: false }

phases:
  - id: review
    description: "4-perspective spec review"
    invoke:
      agents:
        - spec-review-requirements
        - spec-review-design-judgment
        - spec-review-feasibility
        - spec-review-consistency
      iterations: "args.iterations"
      args:
        ui_agent: spec-review-ui-design
        codex: "args.codex"
        swarm: "args.swarm"
    produces:
      - name: review_findings
        path: ".belt/runs/*/review/findings.json"
        description: "Deduplicated N-way voted findings across requirements, design, feasibility, and consistency perspectives"
    confirm: true

  - id: fix
    description: "Fix accepted findings from the review phase"
    consumes:
      - review_findings
    gate:
      - has_output: true
```

Notes:
- `invoke.agents:` list is the first-class field; `iterations` is hoisted up because it is an intrinsic property of how the Agents invocation runs (parallel to GateCheck's own intrinsic fields).
- `ui_agent`, `codex`, `swarm` stay inside `args:` because they are scenario qualifiers rather than identity.
- `produces:` for `review_findings` points at the output directory convention (`.belt/runs/*/review/findings.json`). The glob is resolved at runtime via phase-start mtime filter. The description documents the voting semantics.
- The `fix` phase gains `consumes: [review_findings]` so the artifact graph is explicit.

- [ ] **Step 2: Lint**

```bash
cargo run -p belt -- lint examples/skills/spec-review/pipeline.yml
```

Expected: no errors. The Plan A `check_artifact_flow` rule resolves `review_findings` in the `fix` phase against the earlier `review` phase's produces.

- [ ] **Step 3: Smoke-init via belt-agent**

```bash
mkdir -p /tmp/belt-plan-b-task5
cd /tmp/belt-plan-b-task5
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/spec-review/* .
cargo run -p belt-agent -- init pipeline.yml
cargo run -p belt-agent -- next
```

Expected: `init` returns a `run_id`, and `next` returns the `review` phase with its `invoke` fully populated.

Clean up:

```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
rm -rf /tmp/belt-plan-b-task5
```

- [ ] **Step 4: Commit**

```bash
git add examples/skills/spec-review/pipeline.yml
git commit -m "$(cat <<'EOF'
refactor(example): migrate spec-review to Invoker::Agents format

config.agents becomes invoke.agents; iterations is hoisted to invoke.
ui_agent, codex, swarm stay inside invoke.args as run-time qualifiers.
Adds produces: review_findings so the fix phase consumes: it explicitly.

BELT-32 Plan B task 5.
EOF
)"
```

---

## Task 6: Migrate `examples/skills/code-review/pipeline.yml`

**Files:**
- Modify: `examples/skills/code-review/pipeline.yml`

- [ ] **Step 1: Rewrite the pipeline**

Overwrite `examples/skills/code-review/pipeline.yml` with exactly:

```yaml
name: code-review
version: 1
args:
  iterations: { type: number, default: 3 }
  codex: { type: bool, default: false }
  swarm: { type: bool, default: false }

phases:
  - id: review
    description: "7-perspective code review"
    invoke:
      agents:
        - code-review-quality
        - code-review-security
        - code-review-performance
        - code-review-test
        - code-review-ai-antipattern
        - code-review-impact
      iterations: "args.iterations"
      args:
        skills:
          - /simplify
        codex: "args.codex"
        swarm: "args.swarm"
    produces:
      - name: review_findings
        path: ".belt/runs/*/review/findings.json"
        description: "Deduplicated N-way voted findings across quality, security, performance, test, AI-antipattern, and impact perspectives"
    confirm: true

  - id: fix
    description: "Fix accepted findings from the review phase"
    consumes:
      - review_findings
    gate:
      - has_output: true
```

Notes:
- `config.skills: ["/simplify"]` migrates into `invoke.args.skills:` because `skills` is not part of the `Invoker::Agents` struct — it is an optional companion skill list that lives as a run-time qualifier.
- The agent list is six entries (the old pipeline documents 7 perspectives but actually lists 6 agents; `code-review-quality` acts as the seventh slot historically). If that needs to change, it is a separate concern outside this migration.

- [ ] **Step 2: Lint**

```bash
cargo run -p belt -- lint examples/skills/code-review/pipeline.yml
```

Expected: no errors.

- [ ] **Step 3: Smoke-init via belt-agent**

```bash
mkdir -p /tmp/belt-plan-b-task6
cd /tmp/belt-plan-b-task6
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/code-review/* .
cargo run -p belt-agent -- init pipeline.yml
cargo run -p belt-agent -- next
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
rm -rf /tmp/belt-plan-b-task6
```

Expected: `next` returns the `review` phase with six agents in `invoke.agents`.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/code-review/pipeline.yml
git commit -m "$(cat <<'EOF'
refactor(example): migrate code-review to Invoker::Agents format

Six code-review agents move to invoke.agents; iterations is hoisted.
skills: [/simplify], codex, and swarm stay inside invoke.args as
run-time qualifiers. Adds produces/consumes for review_findings.

BELT-32 Plan B task 6.
EOF
)"
```

---

## Task 7: Migrate `examples/skills/test-review/pipeline.yml`

**Files:**
- Modify: `examples/skills/test-review/pipeline.yml`

- [ ] **Step 1: Rewrite the pipeline**

Overwrite `examples/skills/test-review/pipeline.yml` with exactly:

```yaml
name: test-review
version: 1
args:
  iterations: { type: number, default: 3 }
  codex: { type: bool, default: false }
  swarm: { type: bool, default: false }

phases:
  - id: review
    description: "3-perspective test review"
    invoke:
      agents:
        - test-review-coverage
        - test-review-quality
        - test-review-design-alignment
      iterations: "args.iterations"
      args:
        codex: "args.codex"
        swarm: "args.swarm"
    produces:
      - name: review_findings
        path: ".belt/runs/*/review/findings.json"
        description: "Deduplicated N-way voted findings across coverage, quality, and design-alignment perspectives"
    confirm: true

  - id: fix
    description: "Fix accepted findings from the review phase"
    consumes:
      - review_findings
    gate:
      - has_output: true
```

- [ ] **Step 2: Lint**

```bash
cargo run -p belt -- lint examples/skills/test-review/pipeline.yml
```

Expected: no errors.

- [ ] **Step 3: Smoke-init via belt-agent**

```bash
mkdir -p /tmp/belt-plan-b-task7
cd /tmp/belt-plan-b-task7
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/test-review/* .
cargo run -p belt-agent -- init pipeline.yml
cargo run -p belt-agent -- next
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
rm -rf /tmp/belt-plan-b-task7
```

Expected: `next` returns the `review` phase with three agents.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/test-review/pipeline.yml
git commit -m "$(cat <<'EOF'
refactor(example): migrate test-review to Invoker::Agents format

BELT-32 Plan B task 7.
EOF
)"
```

---

## Task 8: Migrate `examples/skills/implementation-review/pipeline.yml`

**Files:**
- Modify: `examples/skills/implementation-review/pipeline.yml`

- [ ] **Step 1: Rewrite the pipeline**

Overwrite `examples/skills/implementation-review/pipeline.yml` with exactly:

```yaml
name: implementation-review
version: 1
args:
  iterations: { type: number, default: 3 }
  codex: { type: bool, default: false }
  ui: { type: bool, default: false }
  swarm: { type: bool, default: false }

phases:
  - id: review
    description: "3-perspective plan review"
    invoke:
      agents:
        - implementation-review-clarity
        - implementation-review-feasibility
        - implementation-review-consistency
      iterations: "args.iterations"
      args:
        ui_agent: implementation-review-ui-spec
        codex: "args.codex"
        swarm: "args.swarm"
    produces:
      - name: review_findings
        path: ".belt/runs/*/review/findings.json"
        description: "Deduplicated N-way voted findings across clarity, feasibility, and consistency perspectives"
    confirm: true

  - id: fix
    description: "Fix accepted findings from the review phase"
    consumes:
      - review_findings
    gate:
      - has_output: true
```

- [ ] **Step 2: Lint**

```bash
cargo run -p belt -- lint examples/skills/implementation-review/pipeline.yml
```

Expected: no errors.

- [ ] **Step 3: Smoke-init via belt-agent**

```bash
mkdir -p /tmp/belt-plan-b-task8
cd /tmp/belt-plan-b-task8
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/implementation-review/* .
cargo run -p belt-agent -- init pipeline.yml
cargo run -p belt-agent -- next
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
rm -rf /tmp/belt-plan-b-task8
```

Expected: `next` returns the `review` phase.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/implementation-review/pipeline.yml
git commit -m "$(cat <<'EOF'
refactor(example): migrate implementation-review to Invoker::Agents format

BELT-32 Plan B task 8.
EOF
)"
```

---

## Task 9: Migrate `examples/skills/feature-dev/`

**Files:**
- Modify: `examples/skills/feature-dev/pipeline.yml`
- Modify: `examples/skills/feature-dev/SKILL.md`

**Migration principle:** collapse every `{phase}-audit` pair into a single phase with `validate: ./criteria/{name}.md` or `../../criteria/{name}.md`. Replace `uses: ../sub/pipeline.yml` with `invoke: { pipeline: ../sub/pipeline.yml }`. The 19-phase pipeline collapses to 10 phases.

- [ ] **Step 1: Rewrite `pipeline.yml`**

Overwrite `examples/skills/feature-dev/pipeline.yml` with exactly:

```yaml
name: feature-dev
description: "Quality-gated development orchestrator"
version: 1
args:
  e2e: { type: bool, default: false }
  smoke: { type: bool, default: false }
  doc: { type: bool, default: false }
  codex: { type: bool, default: false }
  ui: { type: bool, default: false }
  iterations: { type: number, default: 3 }
  swarm: { type: bool, default: false }

phases:
  # ─── Design ───
  - id: design
    description: "Create design spec via brainstorming"
    invoke:
      skill: /brainstorming
      args:
        swarm: "args.swarm"
    produces:
      - name: design_doc
        path: "docs/plans/*-design.md"
        description: "Brainstormed design with requirements, impact scope, test perspectives"
    gate:
      - file_exists: "docs/plans/*-design.md"
    validate: ./criteria/design.md
    confirm: true
    max_retries: 3

  # ─── Spec Review ───
  - id: spec-review
    description: "Multi-perspective spec review via spec-review sub-pipeline"
    invoke:
      pipeline: ../spec-review/pipeline.yml
    consumes:
      - design_doc
    validate: ./criteria/spec-review.md
    confirm: true
    max_retries: 3

  # ─── Plan ───
  - id: plan
    description: "Create implementation plan and test cases"
    invoke:
      skill: /writing-plans
    consumes:
      - design_doc
    produces:
      - name: plan_doc
        path: "docs/plans/*-plan.md"
        description: "Implementation plan with task breakdown and TDD steps"
      - name: test_cases
        path: "docs/plans/*-test-cases.md"
        description: "Test case catalogue enumerated from the design"
    gate:
      - file_exists: "docs/plans/*-plan.md"
      - file_exists: "docs/plans/*-test-cases.md"
    validate: ./criteria/plan.md
    confirm: true
    max_retries: 3

  # ─── Plan Review ───
  - id: plan-review
    description: "Plan review via implementation-review sub-pipeline"
    invoke:
      pipeline: ../implementation-review/pipeline.yml
    consumes:
      - plan_doc
      - test_cases
    validate: ./criteria/plan-review.md
    confirm: true
    max_retries: 3

  # ─── Execute ───
  - id: execute
    description: "TDD implementation following the plan"
    invoke:
      skill: /subagent-driven-development
    consumes:
      - design_doc
      - plan_doc
      - test_cases
    validate: ../../criteria/execute.md
    confirm: true
    max_retries: 3

  # ─── Doc Audit (conditional) ───
  - id: doc-audit
    description: "4-layer document audit"
    when: "args.doc"
    invoke:
      skill: /doc-audit
    validate: ./criteria/doc-audit.md
    confirm: true
    max_retries: 3

  # ─── Smoke Test (conditional) ───
  - id: smoke-test
    description: "Local smoke test with browser verification"
    when: "args.smoke"
    invoke:
      skill: /smoke-test
    produces:
      - name: smoke_test_report
        path: "smoke-test-report.md"
        description: "Ad-hoc smoke test report with adversarial probes"
    gate:
      - file_exists: "smoke-test-report.md"
    validate: ../../criteria/smoke-test.md
    confirm: true
    max_retries: 3

  # ─── Code Review ───
  - id: code-review
    description: "Multi-perspective code review via code-review sub-pipeline"
    invoke:
      pipeline: ../code-review/pipeline.yml
    consumes:
      - design_doc
      - plan_doc
    validate: ../../criteria/code-review.md
    regate: [execute, smoke-test, doc-audit]
    confirm: true
    max_retries: 3

  # ─── Test Review (conditional) ───
  - id: test-review
    description: "Multi-perspective test review via test-review sub-pipeline"
    when: "args.e2e"
    invoke:
      pipeline: ../test-review/pipeline.yml
    consumes:
      - plan_doc
      - test_cases
    validate: ../../criteria/test-review.md
    regate: [execute]
    confirm: true
    max_retries: 3

  # ─── Integrate ───
  - id: integrate
    description: "Merge, PR, or branch management"
    invoke:
      skill: /worktrunk
    validate:
      - "Integration method chosen and executed"
      - "All pre-merge checks pass"
    confirm: true
```

Notes on specific migration choices:
- Every `{phase}-audit` pair collapses. For example, `design` + `design-audit` becomes one `design` phase with `validate: ./criteria/design.md`. Nine audit phases disappear.
- `feature-dev` specific criteria (`design`, `plan`, `plan-review`, `spec-review`, `doc-audit`) reference the local `./criteria/{name}.md` (files created in Task 1 under `examples/skills/feature-dev/criteria/`).
- Shared canonical criteria (`execute`, `code-review`, `smoke-test`, `test-review`) reference `../../criteria/{name}.md` (i.e., `examples/criteria/{name}.md`).
- Sub-pipeline references (`spec-review`, `plan-review`, `code-review`, `test-review`) migrate from `uses:` to `invoke: { pipeline: ... }`.
- `produces:` and `consumes:` are declared explicitly to expose the artifact graph: design produces design_doc; plan consumes design_doc and produces plan_doc + test_cases; execute consumes all three; review phases consume the relevant subset.
- `regate:` targets are preserved exactly as in the pre-migration pipeline; `code-review` regates to `[execute, smoke-test, doc-audit]`, `test-review` regates to `[execute]`.
- `integrate` does not collapse because it has no audit pair and has two inline validate criteria that belong together — the list form stays.
- Phase count: 10 phases (down from 19).

- [ ] **Step 2: Rewrite `SKILL.md` dispatch rule table**

Open `examples/skills/feature-dev/SKILL.md` and replace its "Dispatch Rules" section. The rewritten section (located where the old table lives) becomes:

```markdown
## Dispatch Rules

| invoke variant | Orchestrator action |
|---|---|
| `skill:` | Invoke the Claude Code slash-command skill named in `invoke.skill`, passing `invoke.args` as arguments. |
| `pipeline:` | Initialise a nested `belt-agent` run on the referenced sub-pipeline (`invoke.pipeline`) with `invoke.with` as args. |

Phase validation is driven by `validate:` on each phase. When `validate` is
a file reference (e.g., `./criteria/design.md`), read the file and judge
each criterion defined inside it before calling `belt-agent step --confirm`.
When `validate` is a list of inline strings, judge each string directly.
Both forms use the same `phase-auditor` subagent by convention; see
`../../references/audit-protocol.md`.
```

(Replace the entire previous Dispatch Rules section, not just the table.)

- [ ] **Step 3: Lint**

```bash
cargo run -p belt -- lint examples/skills/feature-dev/pipeline.yml
```

Expected: no errors. The `check_artifact_flow` rule verifies every `consumes:` resolves to an earlier `produces:`. Shared criteria paths (`../../criteria/execute.md` etc.) are validated by the Plan A validate-file-existence rule.

If lint errors appear regarding `check_artifact_flow`, the likely cause is a `consumes:` name that no earlier phase produces. Cross-reference against the `produces:` declarations above.

- [ ] **Step 4: Initialise and walk through the pipeline**

```bash
mkdir -p /tmp/belt-plan-b-task9
cd /tmp/belt-plan-b-task9
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/feature-dev/* .
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/criteria ../criteria
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/spec-review ../spec-review
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/code-review ../code-review
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/test-review ../test-review
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/implementation-review ../implementation-review
cargo run -p belt-agent -- init pipeline.yml --arg smoke=false --arg e2e=false --arg doc=false
cargo run -p belt-agent -- next
```

Expected: `init` succeeds, `next` returns the `design` phase with `invoke.skill == "/brainstorming"` and `validate` containing a `File` entry pointing at `./criteria/design.md`.

Clean up:

```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
rm -rf /tmp/belt-plan-b-task9
```

- [ ] **Step 5: Commit**

```bash
git add examples/skills/feature-dev/pipeline.yml examples/skills/feature-dev/SKILL.md
git commit -m "$(cat <<'EOF'
refactor(example): migrate feature-dev to Invoker/Artifact/validate-file

Collapses every {phase}-audit pair into a single phase with
validate: ./criteria/{name}.md (skill-specific) or ../../criteria/{name}.md
(shared canonical). Replaces phase-level uses: with invoke.pipeline.
Declares produces/consumes to expose the artifact graph from design
through execute, review, and integrate phases. The pipeline drops
from 19 phases to 10. SKILL.md dispatch rule table shrinks to two
rows (skill variant + pipeline variant).

BELT-32 Plan B task 9.
EOF
)"
```

---

## Task 10: Migrate `examples/skills/debug-flow/`

**Files:**
- Modify: `examples/skills/debug-flow/pipeline.yml`
- Modify: `examples/skills/debug-flow/SKILL.md`

- [ ] **Step 1: Rewrite `pipeline.yml`**

Overwrite `examples/skills/debug-flow/pipeline.yml` with exactly:

```yaml
name: debug-flow
description: "Quality-gated debugging orchestrator"
version: 1
args:
  e2e: { type: bool, default: false }
  smoke: { type: bool, default: false }
  codex: { type: bool, default: false }
  ui: { type: bool, default: false }
  iterations: { type: number, default: 3 }
  swarm: { type: bool, default: false }

phases:
  # ─── Root Cause Analysis ───
  - id: rca
    description: "Investigate root cause via parallel exploration"
    invoke:
      skill: /systematic-debugging
      args:
        swarm: "args.swarm"
    produces:
      - name: rca_report
        path: "docs/plans/*-rca-report.md"
        description: "Root cause analysis report identifying the underlying defect and contributing conditions"
    gate:
      - file_exists: "docs/plans/*-rca-report.md"
    validate: ./criteria/rca.md
    confirm: true
    max_retries: 3

  # ─── Fix Plan ───
  - id: fix-plan
    description: "Create fix plan from RCA report"
    invoke:
      skill: /writing-plans
    consumes:
      - rca_report
    produces:
      - name: fix_plan_doc
        path: "docs/plans/*-fix-plan.md"
        description: "Fix plan describing the remediation strategy and task breakdown"
    gate:
      - file_exists: "docs/plans/*-fix-plan.md"
    validate: ./criteria/fix-plan.md
    confirm: true
    max_retries: 3

  # ─── Fix Plan Review ───
  - id: fix-plan-review
    description: "Plan review via implementation-review sub-pipeline"
    invoke:
      pipeline: ../implementation-review/pipeline.yml
    consumes:
      - fix_plan_doc
    validate: ./criteria/fix-plan-review.md
    confirm: true
    max_retries: 3

  # ─── Execute ───
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

  # ─── Smoke Test (conditional) ───
  - id: smoke-test
    description: "Local smoke test with browser verification"
    when: "args.smoke"
    invoke:
      skill: /smoke-test
    produces:
      - name: smoke_test_report
        path: "smoke-test-report.md"
        description: "Ad-hoc smoke test report with adversarial probes"
    gate:
      - file_exists: "smoke-test-report.md"
    validate: ../../criteria/smoke-test.md
    confirm: true
    max_retries: 3

  # ─── Code Review ───
  - id: code-review
    description: "Multi-perspective code review via code-review sub-pipeline"
    invoke:
      pipeline: ../code-review/pipeline.yml
    consumes:
      - fix_plan_doc
    validate: ../../criteria/code-review.md
    regate: [execute, smoke-test]
    confirm: true
    max_retries: 3

  # ─── Test Review (conditional) ───
  - id: test-review
    description: "Multi-perspective test review via test-review sub-pipeline"
    when: "args.e2e"
    invoke:
      pipeline: ../test-review/pipeline.yml
    consumes:
      - fix_plan_doc
    validate: ../../criteria/test-review.md
    regate: [execute]
    confirm: true
    max_retries: 3

  # ─── Integrate ───
  - id: integrate
    description: "Merge, PR, or branch management"
    invoke:
      skill: /worktrunk
    validate:
      - "Integration method chosen and executed"
      - "All pre-merge checks pass"
    confirm: true
```

Notes:
- Phase count: 8 (down from 16).
- `rca`, `fix-plan`, `fix-plan-review` use skill-specific criteria from `./criteria/`.
- `execute`, `smoke-test`, `code-review`, `test-review` use shared canonical criteria from `../../criteria/`.
- `debug-flow` does **not** use `spec-review` as a sub-pipeline (see Plan B spec sub-task 10 dependency list).
- Regate targets are preserved: `code-review` regates to `[execute, smoke-test]`, `test-review` regates to `[execute]`.

- [ ] **Step 2: Rewrite `SKILL.md` dispatch rule table**

Open `examples/skills/debug-flow/SKILL.md` and replace its "Dispatch Rules" section with the same two-row table pattern used in Task 9 Step 2 (feature-dev), adjusted for the debug-flow wording as needed. Both skills use the same dispatch rule set.

- [ ] **Step 3: Lint**

```bash
cargo run -p belt -- lint examples/skills/debug-flow/pipeline.yml
```

Expected: no errors.

- [ ] **Step 4: Smoke-init**

```bash
mkdir -p /tmp/belt-plan-b-task10
cd /tmp/belt-plan-b-task10
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/debug-flow/* .
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/criteria ../criteria
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/implementation-review ../implementation-review
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/code-review ../code-review
cp -r /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/test-review ../test-review
cargo run -p belt-agent -- init pipeline.yml --arg smoke=false --arg e2e=false
cargo run -p belt-agent -- next
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
rm -rf /tmp/belt-plan-b-task10
```

Expected: `next` returns the `rca` phase.

- [ ] **Step 5: Commit**

```bash
git add examples/skills/debug-flow/pipeline.yml examples/skills/debug-flow/SKILL.md
git commit -m "$(cat <<'EOF'
refactor(example): migrate debug-flow to Invoker/Artifact/validate-file

Collapses audit phases, migrates sub-pipeline refs to invoke.pipeline,
declares produces/consumes for rca_report and fix_plan_doc. Pipeline
drops from 16 phases to 8. SKILL.md dispatch rule table shrinks to
two rows matching feature-dev's.

BELT-32 Plan B task 10.
EOF
)"
```

---

## Task 11: Delete `audit-gate` and remove legacy fields from `belt-core::model::Phase`

**Files:**
- Delete: `examples/skills/audit-gate/` (entire directory)
- Delete: `examples/skills/feature-dev/references/done-criteria/` (entire sub-directory; the directory itself and its parent `references/` directory survive because they still contain `evidence-plan-protocol.md` and `fix-dispatch-strategy.md`)
- Delete: `examples/skills/debug-flow/references/done-criteria/` (same pattern)
- Modify: `crates/belt-core/src/model.rs`
- Modify: `crates/belt-core/src/expander.rs`
- Modify: `crates/belt-core/src/lint.rs`
- Modify: `crates/belt-core/tests/model_test.rs`
- Modify: `crates/belt-core/tests/expander_test.rs`
- Modify: `crates/belt-core/tests/lint_test.rs`
- Modify: `crates/belt-core/tests/parser_test.rs`

- [ ] **Step 1: Pre-check: confirm no example still references legacy shapes**

Run:

```bash
grep -rE "^[[:space:]]+artifacts:" examples/skills 2>&1 || true
grep -rE "config:" examples/skills --include "pipeline.yml" | grep -E "(skill|criteria|agents|reference|audit):" 2>&1 || true
```

Expected: zero matches for `artifacts:`. The second grep should also return zero lines, confirming that no `config.*` dispatch key remains in any migrated pipeline.

Run the phase-level `uses:` check (must skip gate-level `uses:`):

```bash
for f in examples/skills/*/pipeline.yml; do
    awk '
        /^  - id:/ { in_phase=1; indent_len=0; next }
        in_phase && /^  [a-zA-Z]/ && !/^  -/ {
            if ($1 == "uses:") { print FILENAME ":" NR ": phase-level uses still present: " $0 }
            in_phase=0
        }
    ' "$f"
done
```

Expected: no output (zero phase-level `uses:` lines).

If any of these checks fail, **stop** — return to the relevant task (4 through 10) and fix the migration before continuing.

- [ ] **Step 2: Delete `examples/skills/audit-gate/`**

```bash
rm -rf examples/skills/audit-gate
```

Verify:

```bash
ls examples/skills/
```

Expected: seven directories (`code-review`, `debug-flow`, `feature-dev`, `implementation-review`, `smoke-test`, `spec-review`, `test-review`). `audit-gate` is absent.

- [ ] **Step 3: Delete `examples/skills/feature-dev/references/done-criteria/`**

```bash
rm -rf examples/skills/feature-dev/references/done-criteria
```

Verify:

```bash
ls examples/skills/feature-dev/references/
```

Expected: two files (`evidence-plan-protocol.md`, `fix-dispatch-strategy.md`).

- [ ] **Step 4: Delete `examples/skills/debug-flow/references/done-criteria/`**

```bash
rm -rf examples/skills/debug-flow/references/done-criteria
```

Verify:

```bash
ls examples/skills/debug-flow/references/
```

Expected: two files (`evidence-plan-protocol.md`, `fix-dispatch-strategy.md`).

- [ ] **Step 5: Remove `Phase.artifacts` and `Phase.uses` from `model.rs`**

Open `crates/belt-core/src/model.rs`. Find `struct Phase`. Delete the `artifacts:` line and the phase-level `uses:` line. Example before/after (the surrounding fields may vary; the two lines to delete are the ones matching these patterns):

Before:
```rust
pub struct Phase {
    pub id: String,
    // ...
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uses: Option<String>,
    // ...
}
```

After:
```rust
pub struct Phase {
    pub id: String,
    // ...
    // `artifacts` and phase-level `uses` removed in BELT-32 Plan B;
    // use `produces: Vec<Artifact>` and `invoke: Invoker::Pipeline` instead.
    // ...
}
```

Do the same for `ExpandedPhase`.

- [ ] **Step 6: Simplify `expander.rs` by removing the legacy `uses:` branch**

Open `crates/belt-core/src/expander.rs`. Find the `expand_phase` or `leaf_phase` function. Locate the branch that handles `phase.uses.is_some()` separately from `phase.invoke = Some(Invoker::Pipeline { ... })`. Remove the legacy branch — only the `Invoker::Pipeline` path remains.

Concretely, any `match phase.uses` block or `if let Some(uses) = &phase.uses` block should be removed, and the expander should rely solely on inspecting `phase.invoke`.

Run `cargo check -p belt-core` to catch any reference to the removed field. Fix call sites by switching to `Invoker::Pipeline`.

- [ ] **Step 7: Remove legacy-field references from `lint.rs` and add empty-phase rule**

Open `crates/belt-core/src/lint.rs`. Find any rule that references `phase.artifacts` or `phase.uses` and either delete the rule or narrow it so it operates on `phase.produces` / `phase.invoke` equivalents.

Add the new empty-phase rule at the bottom of the file (or near the other `check_*` helpers):

```rust
/// Reject phases that have no action, no verification, and no interaction.
/// A phase must do at least one of: invoke something, run a gate, declare
/// a validate criterion, or require confirmation. Completely empty phases
/// are almost always authoring mistakes (spec DD-8).
fn check_empty_phase(phase: &crate::model::ExpandedPhase, diagnostics: &mut Vec<LintDiagnostic>) {
    let has_action = phase.invoke.is_some();
    let has_verification = !phase.gate.is_empty() || !phase.validate.is_empty();
    let has_interaction = phase.confirm;
    if !has_action && !has_verification && !has_interaction {
        diagnostics.push(LintDiagnostic::EmptyPhase {
            phase_id: phase.id.clone(),
        });
    }
}
```

Add the `EmptyPhase` variant to `LintDiagnostic`:

```rust
// Inside enum LintDiagnostic { ... }
EmptyPhase { phase_id: String },
```

And extend the `Display` impl (or `message()` helper, if Plan A uses that pattern) to return a human-readable string:

```rust
LintDiagnostic::EmptyPhase { phase_id } => format!(
    "phase '{phase_id}' has neither invoke, gate, validate, nor confirm — add at least one"
),
```

Wire `check_empty_phase` into the top-level `lint_pipeline` function by calling it for each phase.

- [ ] **Step 8: Remove tests for legacy fields; add tests for empty-phase lint**

Open `crates/belt-core/tests/model_test.rs`. Search for tests that reference `artifacts:` or phase-level `uses:` (as opposed to gate-level `uses:` in `GateCheck::Uses`) and delete them or update them to use `produces:` / `invoke: Invoker::Pipeline`.

Open `crates/belt-core/tests/expander_test.rs`. Similarly remove legacy `uses:` tests; keep `Invoker::Pipeline` tests intact.

Open `crates/belt-core/tests/lint_test.rs`. Add tests:

```rust
#[test]
fn lint_rejects_completely_empty_phase() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: empty
    description: "has nothing"
"#;
    let diagnostics = lint_pipeline_from_str(yaml).expect("parse");
    assert!(
        diagnostics.iter().any(|d| matches!(d, belt_core::lint::LintDiagnostic::EmptyPhase { phase_id } if phase_id == "empty")),
        "expected EmptyPhase for 'empty', got {diagnostics:?}"
    );
}

#[test]
fn lint_accepts_phase_with_only_gate() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: p
    gate:
      - cmd: "true"
"#;
    let diagnostics = lint_pipeline_from_str(yaml).expect("parse");
    assert!(
        !diagnostics.iter().any(|d| matches!(d, belt_core::lint::LintDiagnostic::EmptyPhase { .. })),
        "phase with only gate must pass empty-phase lint: {diagnostics:?}"
    );
}

#[test]
fn lint_accepts_phase_with_only_validate() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: p
    validate:
      - "some criterion"
"#;
    let diagnostics = lint_pipeline_from_str(yaml).expect("parse");
    assert!(
        !diagnostics.iter().any(|d| matches!(d, belt_core::lint::LintDiagnostic::EmptyPhase { .. })),
        "phase with only validate must pass empty-phase lint: {diagnostics:?}"
    );
}

#[test]
fn lint_accepts_phase_with_only_confirm() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: p
    description: p
    confirm: true
"#;
    let diagnostics = lint_pipeline_from_str(yaml).expect("parse");
    assert!(
        !diagnostics.iter().any(|d| matches!(d, belt_core::lint::LintDiagnostic::EmptyPhase { .. })),
        "phase with only confirm must pass empty-phase lint: {diagnostics:?}"
    );
}
```

`lint_pipeline_from_str` is a helper used elsewhere in Plan A's `lint_test.rs` (search for an existing usage to confirm its signature). If it does not exist, construct the `Pipeline` via `serde_saphyr::from_str` and call `lint_pipeline(&pipeline, Path::new("/dev/null"))` directly.

- [ ] **Step 9: Update `parser_test.rs` for removed fields**

Open `crates/belt-core/tests/parser_test.rs`. Find any test that constructs a pipeline with `artifacts:` or phase-level `uses:` (not gate-level). Update them to the new shapes or remove them if they were purely testing the legacy behavior.

The Plan A integration test `belt32_full_pipeline_with_all_new_types` (around line ending of that file) should already use the new types and remain valid. Verify by reading the test body and confirming no `artifacts:` or phase-level `uses:` YAML fragments are present.

- [ ] **Step 10: Build, test, lint, fmt**

```bash
cargo build -p belt-core 2>&1
cargo test -p belt-core 2>&1
cargo clippy -p belt-core -- -D warnings 2>&1
cargo fmt --package belt-core -- --check 2>&1
```

Expected: all four succeed with no errors and no warnings. Many tests may have been modified in steps 8 and 9; this step is the integration check.

- [ ] **Step 11: Lint every migrated example to confirm they still pass with the legacy-removed parser**

```bash
for f in examples/skills/*/pipeline.yml; do
    echo "--- $f ---"
    cargo run -p belt -- lint "$f"
done
```

Expected: every file lints clean. If any file fails with "unknown field: artifacts" or similar, the migration for that file was incomplete — fix it, commit a small fixup, then retry this step.

- [ ] **Step 12: Commit**

```bash
git add crates/belt-core/src/model.rs crates/belt-core/src/expander.rs crates/belt-core/src/lint.rs crates/belt-core/tests/ examples/skills/audit-gate examples/skills/feature-dev/references/done-criteria examples/skills/debug-flow/references/done-criteria 2>/dev/null
# git add of deleted directories uses `git rm -r` internally via git's add interface
git add -A crates/belt-core/ examples/skills/
git commit -m "$(cat <<'EOF'
refactor(belt-core): remove legacy Phase.artifacts and phase-level uses

Cuts over from the Plan A additive state to the Plan B target shape:
- Phase.artifacts (Vec<String>) removed; use produces: (Vec<Artifact>)
- Phase.uses (Option<String> at phase level) removed; use invoke.pipeline
- Gate-level uses: in GateCheck::Uses preserved (different concept)
- expander simplified: only Invoker::Pipeline path remains
- new lint rule EmptyPhase rejects phases with no invoke, gate, validate,
  or confirm (spec DD-8)
- examples/skills/audit-gate/ deleted
- examples/skills/feature-dev/references/done-criteria/ deleted
  (contents moved to examples/criteria/ and feature-dev/criteria/ in task 1)
- examples/skills/debug-flow/references/done-criteria/ deleted similarly

This is the atomic cutover described in Plan B spec DD-4. Every migrated
example (tasks 4-10) now uses only the new shapes; legacy parsing is
removed. Revert past this commit to restore legacy support if needed.

BELT-32 Plan B task 11.
EOF
)"
```

---

## Task 12: Update `skills/belt-agent/SKILL.md` and run end-to-end smoke test

**Files:**
- Modify: `skills/belt-agent/SKILL.md`
- Modify (possibly new): `crates/belt-agent/tests/cli_test.rs` (E2E smoke section)

- [ ] **Step 1: Rewrite `skills/belt-agent/SKILL.md`**

Overwrite `skills/belt-agent/SKILL.md` with exactly:

````markdown
---
name: belt-agent
description: Belt Protocol for driving belt-agent CLI. Defines command loop, response interpretation, invoke/artifact/validate semantics, and safety constraints for LLM agents driving belt pipelines.
user-invocable: false
---

# Belt Protocol

Protocol for LLM agents driving `belt-agent` CLI — a deterministic state machine for pipeline execution.

## Commands

```bash
# Start a new run
belt-agent init <pipeline.yml>
belt-agent init <pipeline.yml> --arg smoke=true --arg team=BELT

# Get current phase info (or completion signal)
belt-agent next
belt-agent next --run <run_id>

# Run gate checks for current phase
belt-agent verify
belt-agent verify --run <run_id>

# Run regate checks for target phases
belt-agent regate
belt-agent regate --run <run_id>

# Advance to next phase
belt-agent step
belt-agent step --confirm
belt-agent step --run <run_id>

# Inspect full run state (enriched view)
belt-agent status
belt-agent status --run <run_id>
```

`--run <id>` is optional on all commands; omit to use the latest run.

## Workflow

```
init → next → read phase.invoke → execute per variant →
verify (if gates) → regate (if targets) → step → next → ... → completed
```

## Reading `phase.invoke`

Every phase returned by `next` may carry an `invoke` field with one of four variants. Read the variant and take the matching action.

| Variant | Shape | Orchestrator action |
|---------|-------|---------------------|
| `skill` | `{ skill: "/name", args: { ... } }` | Invoke the Claude Code slash-command skill named in `invoke.skill`, passing `invoke.args` as parameters. |
| `agent` | `{ agent: "name", args: { ... } }` | Dispatch a single subagent via the `Agent` tool with `subagent_type: <name>`. Pass `invoke.args` as context. |
| `agents` | `{ agents: ["a", "b", ...], iterations: N, args: { ... } }` | Dispatch each named subagent in parallel. If `iterations > 1`, run N rounds for voting. `invoke.args` carries run-time qualifiers (`ui_agent`, `codex`, `swarm`, etc.). |
| `pipeline` | `{ pipeline: "./path.yml", with: { ... } }` | Initialise a nested `belt-agent` run on the referenced sub-pipeline. Pass `with` as args. Treat the nested run as a black-box until it reports `completed`. |

If `invoke` is absent, the phase is a "pure checkpoint" with only `gate:`, `validate:`, or `confirm:`. Proceed directly to the verify/step loop.

## Artifact graph in `status`

`belt-agent status` returns each phase's `produces` and `consumes` as part of the enriched view.

`produces` is a list of resolved artifacts. Each entry has:

```json
{
  "name": "design_doc",
  "path": "docs/plans/*-design.md",
  "description": "Brainstormed design...",
  "exists": true,
  "resolved_path": "docs/plans/2026-04-11-feature-x-design.md"
}
```

`belt-core` resolves glob paths using the phase-start mtime filter: the matching file with the newest mtime (greater than or equal to the phase's entry timestamp) is chosen, ties broken lexicographically. For concrete paths, `exists` is a direct `std::fs::metadata` check. Use `exists: false` and `resolved_path: null` as a signal that the declared artifact is missing.

`consumes` is a list of artifact references. Each entry is either:

- A string (short form): the artifact name, resolved by lint against the most recent earlier phase that produced that name.
- An object: `{ "name": "design_doc", "from": "design" }` — explicit disambiguation when multiple earlier phases produce the same name.

Use the status output's artifact graph when you need to locate the concrete path of a prior phase's output during the current phase.

## Validate file semantics

Phases may use either:

- `validate: ./criteria/name.md` (scalar file reference) — read the file at that path (relative to the pipeline.yml directory) and judge the criteria defined inside it.
- `validate: /abs/path.md` — same, absolute path.
- `validate: ["criterion one", "criterion two"]` (list of inline strings) — judge each string directly.
- `validate: [{ file: "./x.md" }, "inline"]` (mixed list) — combine.

When a validate entry is a file reference, the orchestrator MUST read the file before running `step --confirm`. The file contains the actual criteria; the scalar/struct in `pipeline.yml` is just the pointer. See `examples/references/audit-protocol.md` for the expected criteria file format.

## Decision Rules

| Situation | Action |
|-----------|--------|
| Phase has no `gate` | Skip `verify`. Go directly to `step`. |
| `verify` returns FAIL | Read `checks` array. Fix failing items. Re-run `verify`. Each verify invocation counts toward `max_retries`. |
| Phase has `regate` targets | After `verify` PASS, run `regate`. On FAIL, fix target phases and re-run `verify` then `regate`. |
| Phase has no `regate` targets | Skip `regate`. Go directly to `step`. |
| Phase has `validate` criteria | Verify each criterion yourself (read file if file-ref; read list if inline), then `step --confirm`. |
| `phase_attempts[phase] > max_retries` | `step` fails with `max_retries_exceeded`. Escalate per pipeline's `on_escalation` policy (BELT-28). |

Every call to `verify` increments the current phase's attempts counter regardless of verdict. `regate` is an in-place re-verification of earlier phases' gates; it does not modify any phase's attempts counter. Earlier phases' counters are never touched by operations at the current phase.

## Step Troubleshooting

When `step` returns `advanced: false`, read the `reason` field:

| `reason` | Action |
|----------|--------|
| `confirmation_required` | Phase has `validate` or `confirm`. Verify criteria, then `step --confirm`. |
| `verify_required` | Run `verify` first. |
| `regate_not_executed` | Run `regate` first. |
| `regate_failed` | Fix regate target phases. Re-run `verify` then `regate`. |
| `max_retries_exceeded` | Escalate. Pipeline author defines recovery via `on_escalation`. |

## Status Output

`status` returns an enriched view assembled from run state, pipeline YAML, and output directories:

```json
{
  "status": "in_progress",
  "current_phase": "review",
  "progress": { "completed": 2, "skipped": 0, "remaining": 2, "total": 4 },
  "phases": [
    {
      "id": "build",
      "status": "completed",
      "verify_passed": true,
      "attempt": 1,
      "invoke": { "skill": "/brainstorming", "args": { "swarm": false } },
      "produces": [
        { "name": "design_doc", "path": "docs/plans/*-design.md", "exists": true, "resolved_path": "docs/plans/2026-04-11-feature-x-design.md" }
      ],
      "consumes": [],
      "outputs": ["report.json"]
    },
    {
      "id": "review",
      "status": "current",
      "verify_passed": false,
      "attempt": 2,
      "invoke": { "pipeline": "../spec-review/pipeline.yml", "with": {} },
      "produces": [],
      "consumes": ["design_doc"],
      "outputs": []
    }
  ]
}
```

Use `status` for context recovery or progress checks.

## Well-known Config Keys

`config` is an opaque map that belt passes through without interpretation. It is retained for phase-specific flags that are orthogonal to invocation identity.

Phase-level invocation identity moved to the typed `invoke:` field; do not use `config.skill`, `config.agents`, `config.criteria`, `config.audit`, or `config.reference` — these have been replaced by `invoke:`, `produces:`, `consumes:`, and file-reference `validate:` respectively.

Remaining `config` keys are skill-specific flags (e.g., `codex: true`, `ui: true`, pipeline-specific arguments). Unknown keys MAY be ignored.

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
````

(Note: the triple-backtick in the code fence of the final `### Status Output` example needs to be preserved. The section above uses four backticks at the outermost level so the embedded triple-backticks render correctly.)

- [ ] **Step 2: Add an E2E smoke test for the migrated feature-dev**

Open `crates/belt-agent/tests/cli_test.rs`. Add at the end of the file:

```rust
/// End-to-end walk through the migrated feature-dev pipeline using the
/// real examples/skills/feature-dev tree. This test is not meant to simulate
/// LLM behavior; it only drives belt-agent through init → next to prove that
/// the new-format pipeline boots and surfaces the first phase correctly.
#[test]
fn feature_dev_migrated_pipeline_boots() {
    use std::path::PathBuf;

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let workspace = PathBuf::from(&manifest_dir).join("..").join("..");
    let pipeline = workspace
        .join("examples")
        .join("skills")
        .join("feature-dev")
        .join("pipeline.yml");
    assert!(pipeline.exists(), "feature-dev pipeline must exist");

    // Use a scratch belt_dir so we don't touch the developer's state.
    let scratch = tempfile::tempdir().expect("tempdir");
    let belt_dir = scratch.path().join(".belt");

    // init
    let init_out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args([
            "init",
            pipeline.to_str().unwrap(),
            "--arg",
            "smoke=false",
            "--arg",
            "e2e=false",
            "--arg",
            "doc=false",
            "--belt-dir",
            belt_dir.to_str().unwrap(),
        ])
        .output()
        .expect("belt-agent init");
    assert!(
        init_out.status.success(),
        "init failed: stdout={} stderr={}",
        String::from_utf8_lossy(&init_out.stdout),
        String::from_utf8_lossy(&init_out.stderr)
    );
    let init_json: serde_json::Value =
        serde_json::from_slice(&init_out.stdout).expect("init stdout is JSON");
    assert!(init_json.get("run_id").is_some(), "init returns run_id");

    // next — must return the first active phase.
    let next_out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args([
            "next",
            "--belt-dir",
            belt_dir.to_str().unwrap(),
        ])
        .output()
        .expect("belt-agent next");
    assert!(
        next_out.status.success(),
        "next failed: stdout={} stderr={}",
        String::from_utf8_lossy(&next_out.stdout),
        String::from_utf8_lossy(&next_out.stderr)
    );
    let next_json: serde_json::Value =
        serde_json::from_slice(&next_out.stdout).expect("next stdout is JSON");
    assert_eq!(
        next_json["phase"]["id"].as_str(),
        Some("design"),
        "first phase should be 'design'"
    );
    // The phase should carry the new invoke shape.
    let invoke = &next_json["phase"]["invoke"];
    assert!(invoke.is_object(), "invoke must be present");
    assert_eq!(invoke["skill"].as_str(), Some("/brainstorming"));
}
```

Note: the `--belt-dir` flag may need to be added to `belt-agent` if it does not already exist (check `crates/belt-agent/src/main.rs`'s clap definition). If it is already present, use it. If not, add it as part of this task with a doc comment "scratch state directory override; used by tests".

If `--belt-dir` does not exist, the simpler test strategy is to set the current working directory of the child process to the tempdir and let `belt-agent` pick up the default `.belt/` there:

```rust
.current_dir(scratch.path())
```

and drop the `--belt-dir` arg. Whichever approach is cleaner, use it consistently.

- [ ] **Step 3: Run the full test suite**

```bash
cargo test -p belt-core 2>&1
cargo test -p belt-agent 2>&1
```

Expected: all tests pass. The new `feature_dev_migrated_pipeline_boots` test is the key addition for this task.

- [ ] **Step 4: Run `belt lint` over every migrated example as a final sweep**

```bash
for f in examples/skills/*/pipeline.yml; do
    echo "--- $f ---"
    cargo run -p belt -- lint "$f"
done
```

Expected: every file lints clean.

- [ ] **Step 5: Run full workspace clippy and fmt**

```bash
cargo clippy --workspace -- -D warnings 2>&1
cargo fmt --check 2>&1
```

Expected: clean across the whole workspace.

- [ ] **Step 6: Commit**

```bash
git add skills/belt-agent/SKILL.md crates/belt-agent/tests/cli_test.rs crates/belt-agent/src/main.rs 2>/dev/null
git commit -m "$(cat <<'EOF'
docs(skill): update belt-agent SKILL.md for Invoker/Artifact/validate-file

Adds three new protocol sections:
- Reading phase.invoke (four variants with orchestrator actions)
- Artifact graph in status (produces/consumes shape + glob resolution rules)
- Validate file semantics (scalar shorthand + HARD-GATE extension)

Rewrites Decision Rules to document max_retries semantics under the
collapsed work+validate model: every verify invocation counts; regate
is in-place and does not touch the counter; earlier phases never mutate.
Removes config.skill from Well-known Config Keys.

Adds an E2E smoke test that initialises the migrated feature-dev pipeline
and confirms the first phase is returned with the new invoke shape.

BELT-32 Plan B task 12.
EOF
)"
```

---

## Success Criteria (plan-wide)

After Task 12 completes, all of these must be true:

- `cargo test -p belt-core` passes. Expected ~230 tests (Plan A baseline 199 + Plan B ~30 new).
- `cargo test -p belt-agent` passes. Expected ~45 tests (baseline 39 + 1 E2E smoke + incidental additions).
- `cargo clippy --workspace -- -D warnings` clean.
- `cargo fmt --check` clean.
- Every `examples/skills/*/pipeline.yml` passes `cargo run -p belt -- lint`.
- `examples/skills/audit-gate/` does not exist.
- `grep -rE "config:" examples/skills --include 'pipeline.yml' | grep -E "(skill|criteria|agents|reference|audit):"` returns zero lines.
- `grep -rE "^[[:space:]]+artifacts:" examples/skills --include 'pipeline.yml'` returns zero lines.
- `feature-dev/pipeline.yml` phase count is 10.
- `debug-flow/pipeline.yml` phase count is 8.
- `feature-dev/SKILL.md` dispatch rule table is two rows (skill + pipeline).
- `debug-flow/SKILL.md` dispatch rule table is two rows.
- `skills/belt-agent/SKILL.md` contains sections titled "Reading `phase.invoke`", "Artifact graph in `status`", and "Validate file semantics".
- E2E smoke test `feature_dev_migrated_pipeline_boots` passes.

## Rollback

The Plan A commit `4608fc4` is the additive-complete checkpoint. To restore legacy support:

```bash
git revert --no-commit <plan-b-task-11-commit>..HEAD
git commit -m "revert: BELT-32 Plan B (restore legacy shapes)"
```

All Plan B tasks 1 through 10 are idempotent with respect to rollback (they only add files and edit YAML; reverting them restores the pre-migration state). Task 11 is the destructive cutover; reverting it restores the legacy fields. Task 12 is a SKILL.md edit that is safe to revert independently.
