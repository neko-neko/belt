# BELT-24: Regate Auto-Execution — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `belt-agent regate` command that deterministically re-executes regate target gates, with step guard enforcement and verify→regate ordering.

**Architecture:** New `regate_passed: HashMap<String, bool>` in `RunState`. Engine gets `record_regate()` method; `verify_verdict()` clears regate state; `step()` adds regate guard between verify and max_retries guards. CLI gets `Regate` subcommand with `cmd_regate()` that orchestrates gate execution for targets.

**Tech Stack:** Rust std, serde, serde_json, clap, belt-core (engine, gate, expander, model, error), tempfile (tests), assert_cmd (CLI tests)

**Spec:** `docs/specs/2026-04-07-belt-regate-auto-execution.md`

**Breaking Change:** Existing test `regate_pipeline_verify_step_works` must be updated — regate guard will block `step()` on phases with regate targets unless `record_regate(true)` is called.

---

### Task 1: Foundation — RunState + Error Types

**Files:**
- Modify: `crates/belt-core/src/model.rs:136` (RunState)
- Modify: `crates/belt-core/src/error.rs:40` (BeltError)

- [ ] **Step 1: Add `regate_passed` to RunState**

In `crates/belt-core/src/model.rs`, add after line 135 (`phase_verify_passed`):

```rust
    #[serde(default)]
    pub regate_passed: HashMap<String, bool>,
```

- [ ] **Step 2: Add error variants to BeltError**

In `crates/belt-core/src/error.rs`, add after `MaxRetriesExceeded` (line 40):

```rust
    #[error("regate required for phase '{phase_id}': run regate before step")]
    #[diagnostic(code(belt::regate_required))]
    RegateRequired {
        phase_id: String,
        targets: Vec<String>,
    },

    #[error("regate failed for phase '{phase_id}': targets {targets:?} did not pass")]
    #[diagnostic(code(belt::regate_failed))]
    RegateFailed {
        phase_id: String,
        targets: Vec<String>,
    },
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p belt-core`

Expected: PASS. No tests yet — just foundation types.

- [ ] **Step 4: Commit**

```bash
git add crates/belt-core/src/model.rs crates/belt-core/src/error.rs
git commit -m "feat(belt-core): add regate_passed to RunState and regate error types (BELT-24)"
```

---

### Task 2: Engine State Management — record_regate + verify clears regate

**Files:**
- Modify: `crates/belt-core/src/engine.rs:209-221` (verify_verdict), new method
- Test: `crates/belt-core/tests/engine_test.rs`

- [ ] **Step 1: Write failing tests — regate state management (tests 1-4)**

Append to `crates/belt-core/tests/engine_test.rs`:

```rust
// ===========================================================================
// BELT-24: regate auto-execution
// ===========================================================================

/// Helper: create a pipeline with regate target.
/// design -> build(regate:[design]) -> test
fn regate_pipeline(dir: &TempDir) -> std::path::PathBuf {
    write_yaml(
        dir,
        "pipeline.yml",
        r#"
name: regate-test
version: 1
phases:
  - id: design
    description: "Design"
    gate:
      - file_exists: "design.ok"
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
    regate: [design]
  - id: test
    description: "Test"
    gate:
      - file_exists: "test.ok"
"#,
    )
}

/// Helper: advance to build phase (verify design, step to build).
fn advance_to_build(
    engine: &Engine,
    state: &mut belt_core::model::RunState,
    pipeline_path: &std::path::Path,
) {
    // design has gate, set verify PASS
    engine.verify_verdict(state, true).expect("verify design");
    engine.step(state, pipeline_path).expect("step to build");
    assert_eq!(state.current_phase, "build");
}

// ---------------------------------------------------------------------------
// Test 1: init does not set regate_passed
// ---------------------------------------------------------------------------
#[test]
fn regate_init_does_not_set_regate_passed() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    assert!(
        state.regate_passed.is_empty(),
        "init should not set regate_passed, got: {:?}",
        state.regate_passed
    );
}

// ---------------------------------------------------------------------------
// Test 2: record_regate stores result
// ---------------------------------------------------------------------------
#[test]
fn regate_record_stores_result() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);

    // verify build PASS
    engine.verify_verdict(&mut state, true).expect("verify build");

    // record regate
    engine.record_regate(&mut state, true).expect("record_regate");

    assert_eq!(
        state.regate_passed.get("build"),
        Some(&true),
        "regate_passed should store true for build phase"
    );
}

// ---------------------------------------------------------------------------
// Test 3: verify clears regate_passed
// ---------------------------------------------------------------------------
#[test]
fn regate_verify_clears_regate_passed() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);

    // verify PASS -> record regate PASS
    engine.verify_verdict(&mut state, true).expect("verify");
    engine.record_regate(&mut state, true).expect("record_regate");
    assert_eq!(state.regate_passed.get("build"), Some(&true));

    // re-verify -> regate_passed should be cleared
    engine.verify_verdict(&mut state, true).expect("re-verify");
    assert!(
        state.regate_passed.get("build").is_none(),
        "verify should clear regate_passed, got: {:?}",
        state.regate_passed
    );
}

// ---------------------------------------------------------------------------
// Test 4: regate_passed persists across save/load
// ---------------------------------------------------------------------------
#[test]
fn regate_passed_persists_across_save_load() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);

    engine.verify_verdict(&mut state, true).expect("verify");
    engine.record_regate(&mut state, true).expect("record_regate");

    // Reload from disk
    let loaded = engine.load_state(&state.run_id).expect("load");
    assert_eq!(
        loaded.regate_passed.get("build"),
        Some(&true),
        "regate_passed should survive save/load round-trip"
    );
}
```

- [ ] **Step 2: Run tests — expect compile error (record_regate doesn't exist)**

Run: `cargo test -p belt-core --test engine_test regate_ -- --test-threads=1 2>&1 | head -20`

Expected: Compile error — `Engine` has no method `record_regate`.

- [ ] **Step 3: Implement `record_regate()` in Engine**

In `crates/belt-core/src/engine.rs`, add after `verify_verdict()` (after line 221):

```rust
    /// Record the result of regate gate checks for the current phase.
    pub fn record_regate(&self, state: &mut RunState, all_passed: bool) -> BeltResult<()> {
        state
            .regate_passed
            .insert(state.current_phase.clone(), all_passed);
        state.updated_at = now_iso8601();
        self.save_state(state)?;
        Ok(())
    }
```

- [ ] **Step 4: Modify `verify_verdict()` to clear regate state**

In `crates/belt-core/src/engine.rs`, in `verify_verdict()` (line 209-221), add one line after `phase_verify_passed.insert`:

```rust
    pub fn verify_verdict(&self, state: &mut RunState, passed: bool) -> BeltResult<bool> {
        let count = state
            .phase_attempts
            .entry(state.current_phase.clone())
            .or_insert(0);
        *count += 1;
        state
            .phase_verify_passed
            .insert(state.current_phase.clone(), passed);
        state.regate_passed.remove(&state.current_phase);
        state.updated_at = now_iso8601();
        self.save_state(state)?;
        Ok(passed)
    }
```

The added line is `state.regate_passed.remove(&state.current_phase);`.

- [ ] **Step 5: Run tests — expect PASS**

Run: `cargo test -p belt-core --test engine_test regate_ -- --test-threads=1`

Expected: All 4 new tests PASS. Existing tests unaffected (no step guard change yet).

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/engine.rs crates/belt-core/tests/engine_test.rs
git commit -m "feat(belt-core): add record_regate() and verify clears regate state (BELT-24)"
```

---

### Task 3: Engine Step Guard + Update Existing Test

**Files:**
- Modify: `crates/belt-core/src/engine.rs:148` (step guard insertion point)
- Modify: `crates/belt-core/tests/engine_test.rs:1074` (existing test update)
- Test: `crates/belt-core/tests/engine_test.rs` (new guard tests)

- [ ] **Step 1: Write failing tests — step regate guard (tests 5-10)**

Append to `crates/belt-core/tests/engine_test.rs`:

```rust
// ---------------------------------------------------------------------------
// Test 5: step requires regate when targets exist
// ---------------------------------------------------------------------------
#[test]
fn regate_step_requires_regate_when_targets_exist() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);

    // verify PASS but skip regate
    engine.verify_verdict(&mut state, true).expect("verify");

    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(
            &result,
            Err(BeltError::RegateRequired { phase_id, targets })
            if phase_id == "build" && targets == &vec!["design".to_string()]
        ),
        "expected RegateRequired, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 6: step blocked when regate failed
// ---------------------------------------------------------------------------
#[test]
fn regate_step_blocked_when_regate_failed() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);

    engine.verify_verdict(&mut state, true).expect("verify");
    engine.record_regate(&mut state, false).expect("regate FAIL");

    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(
            &result,
            Err(BeltError::RegateFailed { phase_id, targets })
            if phase_id == "build" && targets == &vec!["design".to_string()]
        ),
        "expected RegateFailed, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 7: step succeeds after verify and regate pass
// ---------------------------------------------------------------------------
#[test]
fn regate_step_succeeds_after_verify_and_regate_pass() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);

    engine.verify_verdict(&mut state, true).expect("verify");
    engine.record_regate(&mut state, true).expect("regate PASS");

    let next = engine.step(&mut state, &pipeline_path).expect("step");
    assert_eq!(next.as_deref(), Some("test"));
}

// ---------------------------------------------------------------------------
// Test 8: step succeeds without regate when no targets (regression guard)
// ---------------------------------------------------------------------------
#[test]
fn regate_step_succeeds_without_regate_when_no_targets() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // design phase has no regate targets — only verify needed
    engine.verify_verdict(&mut state, true).expect("verify");
    let next = engine.step(&mut state, &pipeline_path).expect("step");
    assert_eq!(next.as_deref(), Some("build"));
}

// ---------------------------------------------------------------------------
// Test 9: verify guard takes priority over regate guard
// ---------------------------------------------------------------------------
#[test]
fn regate_verify_guard_priority_over_regate_guard() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);

    // Neither verify nor regate done — should get VerifyRequired, not RegateRequired
    // Clear verify state to simulate fresh phase
    state.phase_verify_passed.remove("build");
    engine.save_state(&state).expect("save");

    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::VerifyRequired { .. })),
        "expected VerifyRequired (not RegateRequired), got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 10: regate guard takes priority over max_retries
// ---------------------------------------------------------------------------
#[test]
fn regate_guard_priority_over_max_retries() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: regate-retries
version: 1
phases:
  - id: prep
    description: "Prep"
    gate:
      - file_exists: "prep.ok"
  - id: check
    description: "Check"
    gate:
      - file_exists: "check.ok"
    regate: [prep]
    max_retries: 1
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Advance to check phase
    engine.verify_verdict(&mut state, true).expect("verify prep");
    engine.step(&mut state, &pipeline_path).expect("step to check");

    // verify FAIL twice to exceed max_retries (1)
    engine.verify_verdict(&mut state, false).expect("v1");
    engine.verify_verdict(&mut state, true).expect("v2");
    // attempts=2 > max_retries=1, but regate not done

    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::RegateRequired { .. })),
        "expected RegateRequired (not MaxRetriesExceeded), got: {result:?}"
    );
}
```

- [ ] **Step 2: Run tests — expect compile error (RegateRequired/RegateFailed not matched)**

Run: `cargo test -p belt-core --test engine_test regate_step -- --test-threads=1 2>&1 | head -20`

Expected: Compile error on BeltError pattern match.

- [ ] **Step 3: Implement regate guard in `step()`**

In `crates/belt-core/src/engine.rs`, add after the verify-before-step guard (after line 147) and before max_retries guard (line 149):

```rust
        // Guard: regate-before-step
        if !current_phase_def.regate.is_empty() {
            match state.regate_passed.get(&state.current_phase) {
                Some(true) => {}
                Some(false) => {
                    return Err(BeltError::RegateFailed {
                        phase_id: state.current_phase.clone(),
                        targets: current_phase_def.regate.clone(),
                    });
                }
                None => {
                    return Err(BeltError::RegateRequired {
                        phase_id: state.current_phase.clone(),
                        targets: current_phase_def.regate.clone(),
                    });
                }
            }
        }
```

Note: `current_phase_def` is `&phases[current_idx]` which is already computed on line 150 of the original code. The regate guard must be inserted **after** the `current_phase_def` assignment. Check the actual line where `let current_phase_def = &phases[current_idx];` appears — the regate guard goes between that and the max_retries guard.

- [ ] **Step 4: Update existing test `regate_pipeline_verify_step_works`**

The existing test at line 1074 will now fail because the build phase requires regate. Update it to include `record_regate(true)`:

In `crates/belt-core/tests/engine_test.rs`, replace the build phase section of `regate_pipeline_verify_step_works`:

```rust
    // Phase 2: build (has regate: [design])
    engine
        .verify_verdict(&mut state, true)
        .expect("verify build");
    engine
        .record_regate(&mut state, true)
        .expect("regate build");
    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step build->test");
    assert_eq!(next.as_deref(), Some("test"));
```

- [ ] **Step 5: Run all tests**

Run: `cargo test -p belt-core -- --test-threads=1`

Expected: All tests PASS including existing tests and 6 new regate guard tests.

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/engine.rs crates/belt-core/tests/engine_test.rs
git commit -m "feat(belt-core): add regate-before-step guard in Engine::step() (BELT-24)

Guard order: verify -> regate -> max_retries.
Updated existing regate_pipeline_verify_step_works test."
```

---

### Task 4: Engine Verify-Regate Interaction Tests

**Files:**
- Test: `crates/belt-core/tests/engine_test.rs`

No new implementation — these tests exercise behavior already implemented in Tasks 2-3.

- [ ] **Step 1: Write tests 11-14**

Append to `crates/belt-core/tests/engine_test.rs`:

```rust
// ---------------------------------------------------------------------------
// Test 11: verify -> regate(PASS) -> re-verify -> regate cleared -> step blocked
// ---------------------------------------------------------------------------
#[test]
fn regate_verify_regate_reverify_resets_regate() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);

    // verify -> regate PASS
    engine.verify_verdict(&mut state, true).expect("verify");
    engine.record_regate(&mut state, true).expect("regate PASS");

    // re-verify clears regate
    engine.verify_verdict(&mut state, true).expect("re-verify");

    // step should be blocked — regate was cleared
    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::RegateRequired { .. })),
        "re-verify should reset regate, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 12: regate FAIL -> re-verify clears state for fresh retry
// ---------------------------------------------------------------------------
#[test]
fn regate_fail_then_reverify_clears_state() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);

    engine.verify_verdict(&mut state, true).expect("verify");
    engine.record_regate(&mut state, false).expect("regate FAIL");
    assert_eq!(state.regate_passed.get("build"), Some(&false));

    // re-verify clears the failed regate state
    engine.verify_verdict(&mut state, true).expect("re-verify");
    assert!(
        state.regate_passed.get("build").is_none(),
        "re-verify should clear failed regate state"
    );
}

// ---------------------------------------------------------------------------
// Test 13: multiple regate targets — partial fail -> all_passed false
// ---------------------------------------------------------------------------
#[test]
fn regate_multiple_targets_partial_fail() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: multi-regate
version: 1
phases:
  - id: alpha
    description: "Alpha"
    gate:
      - file_exists: "alpha.ok"
  - id: beta
    description: "Beta"
    gate:
      - file_exists: "beta.ok"
  - id: check
    description: "Check"
    gate:
      - file_exists: "check.ok"
    regate: [alpha, beta]
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Advance: alpha -> beta -> check
    engine.verify_verdict(&mut state, true).expect("v alpha");
    engine.step(&mut state, &pipeline_path).expect("step");
    engine.verify_verdict(&mut state, true).expect("v beta");
    engine.step(&mut state, &pipeline_path).expect("step");
    assert_eq!(state.current_phase, "check");

    engine.verify_verdict(&mut state, true).expect("v check");

    // Record partial fail (simulating alpha passed, beta failed -> overall false)
    engine.record_regate(&mut state, false).expect("regate partial fail");

    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::RegateFailed { .. })),
        "partial regate fail should block step: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 14: record_regate is idempotent
// ---------------------------------------------------------------------------
#[test]
fn regate_record_idempotent() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);

    engine.verify_verdict(&mut state, true).expect("verify");
    engine.record_regate(&mut state, true).expect("regate 1");
    engine.record_regate(&mut state, true).expect("regate 2");

    assert_eq!(state.regate_passed.get("build"), Some(&true));
    // step should work fine
    let next = engine.step(&mut state, &pipeline_path).expect("step");
    assert_eq!(next.as_deref(), Some("test"));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p belt-core --test engine_test regate_ -- --test-threads=1`

Expected: All 14 regate tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/belt-core/tests/engine_test.rs
git commit -m "test(belt-core): add verify-regate interaction tests (BELT-24)

Tests 11-14: reverify resets regate, fail clears state,
multi-target partial fail, idempotent record."
```

---

### Task 5: Test Fixtures for CLI Tests

**Files:**
- Create: `crates/belt-core/tests/fixtures/regate_max_retries_pipeline.yml`
- Create: `crates/belt-core/tests/fixtures/regate_gateless_pipeline.yml`
- Create: `crates/belt-core/tests/fixtures/regate_when_skip_pipeline.yml`

- [ ] **Step 1: Create regate_max_retries_pipeline.yml**

```yaml
name: regate-max-retries
version: 1
phases:
  - id: collect
    description: "Collect data"
    gate:
      - file_exists: "collect.ok"
  - id: audit
    description: "Audit quality"
    gate:
      - file_exists: "audit.ok"
    regate: [collect]
    max_retries: 2
  - id: done
    description: "Done"
```

- [ ] **Step 2: Create regate_gateless_pipeline.yml**

```yaml
name: regate-gateless
version: 1
phases:
  - id: prep
    description: "Preparation"
    gate:
      - file_exists: "prep.ok"
  - id: check
    description: "Gateless check with regate"
    regate: [prep]
  - id: done
    description: "Done"
```

- [ ] **Step 3: Create regate_when_skip_pipeline.yml**

```yaml
name: regate-when-skip
version: 1
args:
  run_optional: { type: bool, default: false }
phases:
  - id: optional
    description: "Optional phase"
    when: "args.run_optional"
    gate:
      - file_exists: "optional.ok"
  - id: main
    description: "Main phase"
    gate:
      - file_exists: "main.ok"
    regate: [optional]
  - id: done
    description: "Done"
```

- [ ] **Step 4: Create regate_multi_target_pipeline.yml**

```yaml
name: regate-multi-target
version: 1
phases:
  - id: design
    description: "Design"
    gate:
      - file_exists: "design.ok"
  - id: impl
    description: "Implement"
    gate:
      - file_exists: "impl.ok"
  - id: review
    description: "Review"
    gate:
      - file_exists: "review.ok"
    regate: [design, impl]
  - id: done
    description: "Done"
```

- [ ] **Step 5: Verify fixtures parse correctly**

Run: `cargo run -p belt -- lint crates/belt-core/tests/fixtures/regate_max_retries_pipeline.yml && cargo run -p belt -- lint crates/belt-core/tests/fixtures/regate_gateless_pipeline.yml && cargo run -p belt -- lint crates/belt-core/tests/fixtures/regate_when_skip_pipeline.yml && cargo run -p belt -- lint crates/belt-core/tests/fixtures/regate_multi_target_pipeline.yml`

Expected: All lint PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/tests/fixtures/regate_max_retries_pipeline.yml \
        crates/belt-core/tests/fixtures/regate_gateless_pipeline.yml \
        crates/belt-core/tests/fixtures/regate_when_skip_pipeline.yml \
        crates/belt-core/tests/fixtures/regate_multi_target_pipeline.yml
git commit -m "test(belt-core): add regate test fixtures (BELT-24)"
```

---

### Task 6: CLI — Regate Subcommand + cmd_regate

**Files:**
- Modify: `crates/belt-agent/src/main.rs` (Command enum, main(), new cmd_regate)
- Test: `crates/belt-agent/tests/cli_test.rs`

- [ ] **Step 1: Write failing CLI tests (tests 15-18)**

Append to `crates/belt-agent/tests/cli_test.rs`:

```rust
// ===========================================================================
// BELT-24: regate command tests
// ===========================================================================

/// Helper: run belt-agent with args in a dir, return parsed JSON.
fn run_belt_agent(dir: &TempDir, args: &[&str]) -> serde_json::Value {
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(args)
        .current_dir(dir.path())
        .output()
        .unwrap_or_else(|e| panic!("belt-agent {:?} failed: {e}", args));
    assert!(
        output.status.success(),
        "belt-agent {:?} exit non-zero: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("invalid JSON from belt-agent {:?}: {e}", args))
}

// ---------------------------------------------------------------------------
// Test 15: regate command runs target gates (PASS)
// ---------------------------------------------------------------------------
#[test]
fn regate_command_runs_target_gates() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: regate-cli
version: 1
phases:
  - id: design
    description: "Design"
    gate:
      - file_exists: "design.ok"
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
    regate: [design]
  - id: done
    description: "Done"
"#,
    );

    // Create gate files
    std::fs::write(dir.path().join("design.ok"), "").unwrap();
    std::fs::write(dir.path().join("build.ok"), "").unwrap();

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    // verify design -> step to build
    run_belt_agent(&dir, &["verify", "--run", run_id]);
    run_belt_agent(&dir, &["step", "--run", run_id]);

    // verify build
    run_belt_agent(&dir, &["verify", "--run", run_id]);

    // regate — design.ok exists, should pass
    let regate = run_belt_agent(&dir, &["regate", "--run", run_id]);
    assert_eq!(regate["all_passed"], true);
    assert_eq!(regate["targets"]["design"]["passed"], true);
}

// ---------------------------------------------------------------------------
// Test 16: regate command — target gate fails
// ---------------------------------------------------------------------------
#[test]
fn regate_command_target_gate_fails() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: regate-fail
version: 1
phases:
  - id: design
    description: "Design"
    gate:
      - file_exists: "design.ok"
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
    regate: [design]
  - id: done
    description: "Done"
"#,
    );

    // Create gate files for init/verify progression
    std::fs::write(dir.path().join("design.ok"), "").unwrap();
    std::fs::write(dir.path().join("build.ok"), "").unwrap();

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify", "--run", run_id]);
    run_belt_agent(&dir, &["step", "--run", run_id]);
    run_belt_agent(&dir, &["verify", "--run", run_id]);

    // Remove design.ok BEFORE regate -> target gate fails
    std::fs::remove_file(dir.path().join("design.ok")).unwrap();

    let regate = run_belt_agent(&dir, &["regate", "--run", run_id]);
    assert_eq!(regate["all_passed"], false);
    assert_eq!(regate["targets"]["design"]["passed"], false);
}

// ---------------------------------------------------------------------------
// Test 17: regate no targets returns empty
// ---------------------------------------------------------------------------
#[test]
fn regate_no_targets_returns_empty() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: no-regate
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
  - id: done
    description: "Done"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify", "--run", run_id]);

    let regate = run_belt_agent(&dir, &["regate", "--run", run_id]);
    assert_eq!(regate["all_passed"], true);
    assert!(regate["targets"].as_object().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// Test 18: regate before verify returns error
// ---------------------------------------------------------------------------
#[test]
fn regate_before_verify_returns_error() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: regate-pre-verify
version: 1
phases:
  - id: design
    description: "Design"
    gate:
      - file_exists: "design.ok"
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
    regate: [design]
"#,
    );

    std::fs::write(dir.path().join("design.ok"), "").unwrap();

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    // Advance to build, then try regate WITHOUT verify
    run_belt_agent(&dir, &["verify", "--run", run_id]);
    run_belt_agent(&dir, &["step", "--run", run_id]);

    // regate without verify -> error
    let regate = run_belt_agent(&dir, &["regate", "--run", run_id]);
    assert_eq!(regate["error"], "verify_not_passed");
}
```

- [ ] **Step 2: Run tests — expect failure (regate subcommand doesn't exist)**

Run: `cargo test -p belt-agent --test cli_test regate_ 2>&1 | head -20`

Expected: Compile error or runtime error — `regate` subcommand not recognized.

- [ ] **Step 3: Add Regate subcommand to Command enum**

In `crates/belt-agent/src/main.rs`, add after `Verify` (line 44) and before `Step`:

```rust
    /// Run regate checks for current phase targets
    Regate {
        #[arg(long)]
        run: Option<String>,
    },
```

- [ ] **Step 4: Add import for `expand_pipeline`**

In `crates/belt-agent/src/main.rs`, add to imports (after line 4):

```rust
use belt_core::expander::expand_pipeline;
```

- [ ] **Step 5: Add command dispatch in main()**

In `crates/belt-agent/src/main.rs`, in the match block (after line 119, `Command::Verify`):

```rust
        Command::Regate { run } => cmd_regate(&engine, run.as_ref())?,
```

- [ ] **Step 6: Implement cmd_regate()**

Add at the end of `crates/belt-agent/src/main.rs` (before `cmd_status`):

```rust
fn cmd_regate(engine: &Engine, run: Option<&String>) -> miette::Result<()> {
    let run_id = resolve_run(engine, run)?;
    let mut state = engine
        .load_state(&run_id)
        .map_err(|e| miette::miette!("{e}"))?;

    if state.current_phase == "COMPLETED" {
        let out = json!({
            "error": "pipeline_completed",
            "message": "pipeline is already completed"
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
        );
        return Ok(());
    }

    // Pre-check: verify must have passed
    if state.phase_verify_passed.get(&state.current_phase) != Some(&true) {
        let out = json!({
            "error": "verify_not_passed",
            "phase": state.current_phase,
            "message": "verify must pass before regate"
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
        );
        return Ok(());
    }

    let pipeline_path_str = state.pipeline_file.clone();
    let pipeline_path = Path::new(&pipeline_path_str);
    let phase = engine
        .next_phase_info(&state, pipeline_path)
        .map_err(|e| miette::miette!("{e}"))?;

    // No regate targets
    if phase.regate.is_empty() {
        let out = json!({
            "run_id": state.run_id,
            "phase": phase.id,
            "targets": {},
            "all_passed": true
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
        );
        return Ok(());
    }

    // Get all expanded phases for target lookup
    let all_phases =
        expand_pipeline(pipeline_path).map_err(|e| miette::miette!("{e}"))?;
    let work_dir = std::env::current_dir().map_err(|e| miette::miette!("{e}"))?;
    let belt = belt_dir();

    let mut targets = serde_json::Map::new();
    let mut all_passed_flag = true;

    for target_id in &phase.regate {
        // Skip regate for skipped phases (auto-passed)
        if state.skipped_phases.contains(target_id) {
            targets.insert(
                target_id.clone(),
                json!({ "passed": true, "skipped": true, "checks": [] }),
            );
            continue;
        }

        let target_phase = all_phases
            .iter()
            .find(|p| &p.id == target_id)
            .ok_or_else(|| {
                miette::miette!("regate target '{}' not found in pipeline", target_id)
            })?;

        let run_dir = belt.join("runs").join(&state.run_id);
        let output_dir = run_dir.join(target_id.replace('/', "_"));
        let results = execute_gates(&target_phase.gate, &work_dir, &output_dir);
        let passed = all_passed(&results);
        if !passed {
            all_passed_flag = false;
        }
        targets.insert(
            target_id.clone(),
            json!({ "passed": passed, "checks": results }),
        );
    }

    engine
        .record_regate(&mut state, all_passed_flag)
        .map_err(|e| miette::miette!("{e}"))?;

    let out = json!({
        "run_id": state.run_id,
        "phase": phase.id,
        "targets": targets,
        "all_passed": all_passed_flag
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
    );
    Ok(())
}
```

- [ ] **Step 7: Run CLI tests**

Run: `cargo test -p belt-agent --test cli_test regate_ -- --test-threads=1`

Expected: All 4 new CLI tests PASS.

- [ ] **Step 8: Run all tests to check for regressions**

Run: `cargo test --workspace -- --test-threads=1`

Expected: All tests PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/belt-agent/src/main.rs crates/belt-agent/tests/cli_test.rs
git commit -m "feat(belt-agent): add regate subcommand (BELT-24)

belt-agent regate --run <id> executes regate target gates,
persists results, returns structured JSON.
Handles: verify pre-check, skipped targets, target not found."
```

---

### Task 7: CLI — Step Error Handling for Regate + Test

**Files:**
- Modify: `crates/belt-agent/src/main.rs` (cmd_step error handling)
- Test: `crates/belt-agent/tests/cli_test.rs`

- [ ] **Step 1: Write failing test — step JSON regate_not_executed (test 19)**

Append to `crates/belt-agent/tests/cli_test.rs`:

```rust
// ---------------------------------------------------------------------------
// Test 19: step returns regate_not_executed JSON
// ---------------------------------------------------------------------------
#[test]
fn step_json_regate_not_executed() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: regate-step-block
version: 1
phases:
  - id: design
    description: "Design"
    gate:
      - file_exists: "design.ok"
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
    regate: [design]
  - id: done
    description: "Done"
"#,
    );

    std::fs::write(dir.path().join("design.ok"), "").unwrap();
    std::fs::write(dir.path().join("build.ok"), "").unwrap();

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    // Advance to build
    run_belt_agent(&dir, &["verify", "--run", run_id]);
    run_belt_agent(&dir, &["step", "--run", run_id]);

    // Verify build but skip regate
    run_belt_agent(&dir, &["verify", "--run", run_id]);

    // step should return regate_not_executed
    let step = run_belt_agent(&dir, &["step", "--run", run_id]);
    assert_eq!(step["advanced"], false);
    assert_eq!(step["reason"], "regate_not_executed");
    assert_eq!(step["phase"], "build");
    assert!(step["regate_targets"].as_array().unwrap().contains(&json!("design")));
}
```

- [ ] **Step 2: Run test — expect failure (error falls through to generic handler)**

Run: `cargo test -p belt-agent --test cli_test step_json_regate -- --exact`

Expected: Test FAIL — step returns a generic error instead of structured JSON because `RegateRequired` is not handled in `cmd_step`.

- [ ] **Step 3: Add regate error handling to cmd_step**

In `crates/belt-agent/src/main.rs`, in `cmd_step()`, add two new arms before the catch-all `Err(e)` (before line 327):

```rust
        Err(BeltError::RegateRequired {
            phase_id,
            targets,
        }) => {
            let out = json!({
                "advanced": false,
                "reason": "regate_not_executed",
                "phase": phase_id,
                "regate_targets": targets,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
            );
        }
        Err(BeltError::RegateFailed {
            phase_id,
            targets,
        }) => {
            let out = json!({
                "advanced": false,
                "reason": "regate_failed",
                "phase": phase_id,
                "regate_targets": targets,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
            );
        }
```

- [ ] **Step 4: Run test**

Run: `cargo test -p belt-agent --test cli_test step_json_regate -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-agent/src/main.rs crates/belt-agent/tests/cli_test.rs
git commit -m "feat(belt-agent): handle regate errors in step JSON output (BELT-24)"
```

---

### Task 8: Full Lifecycle + Edge Case Tests

**Files:**
- Test: `crates/belt-agent/tests/cli_test.rs` (lifecycle tests)
- Test: `crates/belt-core/tests/engine_test.rs` (edge cases)

- [ ] **Step 1: Write full lifecycle test (test 20)**

Append to `crates/belt-agent/tests/cli_test.rs`:

```rust
// ---------------------------------------------------------------------------
// Test 20: regate pipeline full lifecycle
// ---------------------------------------------------------------------------
#[test]
fn regate_pipeline_full_lifecycle() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: lifecycle
version: 1
phases:
  - id: design
    description: "Design"
    gate:
      - file_exists: "design.ok"
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
    regate: [design]
  - id: test
    description: "Test"
    gate:
      - file_exists: "test.ok"
  - id: done
    description: "Done"
"#,
    );

    std::fs::write(dir.path().join("design.ok"), "").unwrap();
    std::fs::write(dir.path().join("build.ok"), "").unwrap();
    std::fs::write(dir.path().join("test.ok"), "").unwrap();

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    // design: verify -> step
    run_belt_agent(&dir, &["verify", "--run", run_id]);
    let step = run_belt_agent(&dir, &["step", "--run", run_id]);
    assert_eq!(step["to"], "build");

    // build: verify -> regate -> step
    run_belt_agent(&dir, &["verify", "--run", run_id]);
    let regate = run_belt_agent(&dir, &["regate", "--run", run_id]);
    assert_eq!(regate["all_passed"], true);
    let step = run_belt_agent(&dir, &["step", "--run", run_id]);
    assert_eq!(step["to"], "test");

    // test: verify -> step (no regate)
    run_belt_agent(&dir, &["verify", "--run", run_id]);
    let step = run_belt_agent(&dir, &["step", "--run", run_id]);
    assert_eq!(step["to"], "done");

    // done: gateless -> step
    let step = run_belt_agent(&dir, &["step", "--run", run_id]);
    assert_eq!(step["completed"], true);
}

// ---------------------------------------------------------------------------
// Test 21: regate fail -> fix -> retry lifecycle
// ---------------------------------------------------------------------------
#[test]
fn regate_fail_retry_lifecycle() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: retry
version: 1
phases:
  - id: design
    description: "Design"
    gate:
      - file_exists: "design.ok"
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
    regate: [design]
  - id: done
    description: "Done"
"#,
    );

    std::fs::write(dir.path().join("build.ok"), "").unwrap();

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    // design: create file, verify, step
    std::fs::write(dir.path().join("design.ok"), "").unwrap();
    run_belt_agent(&dir, &["verify", "--run", run_id]);
    run_belt_agent(&dir, &["step", "--run", run_id]);

    // build: verify PASS
    run_belt_agent(&dir, &["verify", "--run", run_id]);

    // Remove design.ok -> regate FAIL
    std::fs::remove_file(dir.path().join("design.ok")).unwrap();
    let regate = run_belt_agent(&dir, &["regate", "--run", run_id]);
    assert_eq!(regate["all_passed"], false);

    // step blocked
    let step = run_belt_agent(&dir, &["step", "--run", run_id]);
    assert_eq!(step["reason"], "regate_failed");

    // Fix: restore design.ok, re-verify, re-regate
    std::fs::write(dir.path().join("design.ok"), "").unwrap();
    run_belt_agent(&dir, &["verify", "--run", run_id]);
    let regate = run_belt_agent(&dir, &["regate", "--run", run_id]);
    assert_eq!(regate["all_passed"], true);

    // step succeeds
    let step = run_belt_agent(&dir, &["step", "--run", run_id]);
    assert_eq!(step["advanced"], true);
}
```

- [ ] **Step 2: Write test 22 — regate with max_retries escalation**

Append to `crates/belt-agent/tests/cli_test.rs`:

```rust
// ---------------------------------------------------------------------------
// Test 22: regate loop exhausts max_retries -> escalation
// ---------------------------------------------------------------------------
#[test]
fn regate_with_max_retries_escalation() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: escalation
version: 1
phases:
  - id: collect
    description: "Collect"
    gate:
      - file_exists: "collect.ok"
  - id: audit
    description: "Audit"
    gate:
      - file_exists: "audit.ok"
    regate: [collect]
    max_retries: 2
  - id: done
    description: "Done"
"#,
    );

    std::fs::write(dir.path().join("collect.ok"), "").unwrap();
    std::fs::write(dir.path().join("audit.ok"), "").unwrap();

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    // Advance to audit
    run_belt_agent(&dir, &["verify", "--run", run_id]);
    run_belt_agent(&dir, &["step", "--run", run_id]);

    // Simulate 3 verify cycles (exceeding max_retries: 2)
    // Cycle 1: verify PASS -> regate FAIL (collect.ok removed)
    run_belt_agent(&dir, &["verify", "--run", run_id]);
    std::fs::remove_file(dir.path().join("collect.ok")).unwrap();
    let regate = run_belt_agent(&dir, &["regate", "--run", run_id]);
    assert_eq!(regate["all_passed"], false);

    // Cycle 2: re-verify (attempt 2) -> regate FAIL
    std::fs::write(dir.path().join("collect.ok"), "").unwrap(); // restore for verify
    run_belt_agent(&dir, &["verify", "--run", run_id]);
    std::fs::remove_file(dir.path().join("collect.ok")).unwrap();
    let regate = run_belt_agent(&dir, &["regate", "--run", run_id]);
    assert_eq!(regate["all_passed"], false);

    // Cycle 3: re-verify (attempt 3) -> regate PASS this time
    std::fs::write(dir.path().join("collect.ok"), "").unwrap();
    run_belt_agent(&dir, &["verify", "--run", run_id]);
    let regate = run_belt_agent(&dir, &["regate", "--run", run_id]);
    assert_eq!(regate["all_passed"], true);

    // step -> 3 attempts > max_retries 2 -> escalation
    let step = run_belt_agent(&dir, &["step", "--run", run_id]);
    assert_eq!(step["advanced"], false);
    assert_eq!(step["reason"], "max_retries_exceeded");
    assert_eq!(step["escalation"], true);
}
```

- [ ] **Step 3: Write edge case tests (tests 23, 25, 26, 28)**

Append to `crates/belt-core/tests/engine_test.rs`:

```rust
// ---------------------------------------------------------------------------
// Test 23: gateless phase with regate targets
// ---------------------------------------------------------------------------
#[test]
fn regate_gateless_phase_with_regate_targets() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: gateless-regate
version: 1
phases:
  - id: prep
    description: "Prep"
    gate:
      - file_exists: "prep.ok"
  - id: check
    description: "Gateless with regate"
    regate: [prep]
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Advance to check phase
    engine.verify_verdict(&mut state, true).expect("verify prep");
    engine.step(&mut state, &pipeline_path).expect("step to check");
    assert_eq!(state.current_phase, "check");

    // check is gateless -> auto-verify. But regate still required.
    assert_eq!(state.phase_verify_passed.get("check"), Some(&true));

    // step should be blocked — regate not done
    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::RegateRequired { .. })),
        "gateless phase with regate should still require regate: {result:?}"
    );

    // record regate -> step succeeds
    engine.record_regate(&mut state, true).expect("regate");
    let next = engine.step(&mut state, &pipeline_path).expect("step");
    assert_eq!(next.as_deref(), Some("done"));
}

// ---------------------------------------------------------------------------
// Test 25: regate target skipped phase — auto-passed
// ---------------------------------------------------------------------------
#[test]
fn regate_target_skipped_phase_auto_passed() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: skip-regate
version: 1
args:
  run_optional: { type: bool, default: false }
phases:
  - id: optional
    description: "Optional"
    when: "args.run_optional"
    gate:
      - file_exists: "optional.ok"
  - id: main
    description: "Main"
    gate:
      - file_exists: "main.ok"
    regate: [optional]
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);

    // init with run_optional=false -> optional phase skipped
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    // First active phase is "main" (optional was skipped)
    assert_eq!(state.current_phase, "main");
    assert!(state.skipped_phases.contains(&"optional".to_string()));

    // verify main
    engine.verify_verdict(&mut state, true).expect("verify main");

    // record regate with true (simulating cmd_regate auto-passing skipped target)
    engine.record_regate(&mut state, true).expect("regate");

    // step should succeed
    let next = engine.step(&mut state, &pipeline_path).expect("step");
    assert_eq!(next.as_deref(), Some("done"));
}
```

- [ ] **Step 4: Write edge case test 26 — regate target with empty gate**

Append to `crates/belt-core/tests/engine_test.rs`:

```rust
// ---------------------------------------------------------------------------
// Test 26: regate target with empty gate -> treated as passed
// ---------------------------------------------------------------------------
#[test]
fn regate_target_with_empty_gate() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: empty-gate-regate
version: 1
phases:
  - id: prep
    description: "Prep (no gate)"
  - id: check
    description: "Check"
    gate:
      - file_exists: "check.ok"
    regate: [prep]
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // prep is gateless -> auto-verify -> step to check
    engine.step(&mut state, &pipeline_path).expect("step to check");
    assert_eq!(state.current_phase, "check");

    engine.verify_verdict(&mut state, true).expect("verify check");
    // record regate as true (cmd_regate would compute all_passed(&[]) = true for empty gate)
    engine.record_regate(&mut state, true).expect("regate");

    let next = engine.step(&mut state, &pipeline_path).expect("step");
    assert_eq!(next.as_deref(), Some("done"));
}
```

- [ ] **Step 5: Write CLI test 24 — regate target not found**

Append to `crates/belt-agent/tests/cli_test.rs`:

```rust
// ---------------------------------------------------------------------------
// Test 24: regate target not found in pipeline
// ---------------------------------------------------------------------------
#[test]
fn regate_target_not_found_returns_error() {
    let dir = TempDir::new().unwrap();
    // Pipeline where regate references a non-existent phase
    // (lint would catch this, but test runtime behavior if lint is bypassed)
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad-regate
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
    regate: [nonexistent]
  - id: done
    description: "Done"
"#,
    );

    let init_output = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["init", "pipeline.yml"])
        .current_dir(dir.path())
        .output()
        .expect("init");
    // init may fail due to lint or succeed — check if we can reach regate
    if !init_output.status.success() {
        // lint caught the bad regate target — acceptable
        return;
    }
    let init_json: serde_json::Value = serde_json::from_slice(&init_output.stdout).unwrap();
    let run_id = init_json["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify", "--run", run_id]);

    // regate should error — target not found
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["regate", "--run", run_id])
        .current_dir(dir.path())
        .output()
        .expect("regate");
    // Expect non-zero exit (miette error) or error JSON
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stderr.contains("not found") || stdout.contains("not found") || !output.status.success(),
        "expected 'not found' error for nonexistent regate target, got stdout={stdout}, stderr={stderr}"
    );
}

// ---------------------------------------------------------------------------
// Test 28: regate on completed pipeline
// ---------------------------------------------------------------------------
#[test]
fn regate_on_completed_pipeline_returns_error() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: completed
version: 1
phases:
  - id: only
    description: "Only phase"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    // Gateless phase -> auto-verify -> step -> COMPLETED
    run_belt_agent(&dir, &["step", "--run", run_id]);

    // regate on completed pipeline
    let regate = run_belt_agent(&dir, &["regate", "--run", run_id]);
    assert!(
        regate.get("error").is_some(),
        "expected error for regate on completed pipeline, got: {regate}"
    );
}
```

- [ ] **Step 6: Run all tests**

Run: `cargo test --workspace -- --test-threads=1`

Expected: All tests PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/belt-core/tests/engine_test.rs crates/belt-agent/tests/cli_test.rs
git commit -m "test: add full lifecycle and edge case tests for regate (BELT-24)

- Full lifecycle: verify -> regate -> step through all phases
- Fail/retry lifecycle: regate FAIL -> fix -> re-verify -> regate -> step
- Gateless phase with regate targets
- Skipped regate target auto-passed"
```

---

### Task 9: Lint, Format, Final Verification

**Files:**
- All modified crates

- [ ] **Step 1: Run formatter**

Run: `cargo fmt --package belt-core --package belt-agent`

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`

Expected: No warnings.

- [ ] **Step 3: Run full test suite**

Run: `cargo test --workspace -- --test-threads=1`

Expected: All tests PASS.

- [ ] **Step 4: Verify test count increase**

Run: `cargo test --workspace -- --test-threads=1 2>&1 | grep "test result"`

Expected: Total test count should be approximately 97+ (87 existing + ~10 new engine tests + ~7 new CLI tests).

- [ ] **Step 5: Commit if any formatting changes**

```bash
git add -A
git commit -m "style: fmt and clippy fixes for regate (BELT-24)"
```

(Skip if no changes.)

- [ ] **Step 6: Update BELT-24 Linear ticket status**

Run: `linear issue update BELT-24 --state "Done"`

---

### Deferred: Test 27 — regate self-reference

Spec test 27 (`regate_self_reference_detected`) is a lint improvement, not a runtime feature. Current lint validates regate target existence but not self-reference. Self-referencing regate is semantically redundant (verify already ran the phase's own gates). This should be addressed as a future lint enhancement, not in this implementation.
