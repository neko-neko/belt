# BELT-21: Engine Guards Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add verify-before-step and max_retries enforcement guards to belt-core Engine, preventing invalid state transitions deterministically at the CLI level.

**Architecture:** Two guards are added to `Engine::step()` before any state mutation. `RunState` tracks per-phase verify results via `phase_verify_passed: HashMap<String, bool>`. Gate-less phases auto-set `true` at init/step time. belt-agent surfaces guard violations as structured JSON.

**Tech Stack:** Rust, belt-core (engine/model/error), belt-agent (main.rs), serde_json, assert_cmd (E2E tests)

**Design Spec:** `docs/specs/2026-04-06-belt-engine-guards.md`

**Note on max_retries semantics:** `max_retries: N` means the phase allows at most N verify attempts. The guard check is `attempts > max_retries`. With `max_retries: 3`, attempts 1-3 are allowed; attempt 4+ triggers `MaxRetriesExceeded` at step time. `max_retries: 0` means unlimited.

**Note on design spec test #7 correction:** The spec listed "attempt 4 (all FAIL) -> MaxRetriesExceeded" but all-FAIL triggers `VerifyRequired` first (guard order). Corrected to: 3 FAIL + 1 PASS = 4 attempts, last PASS passes verify guard, then max_retries triggers.

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `crates/belt-core/src/error.rs` | Modify | Add `VerifyRequired`, `MaxRetriesExceeded` variants |
| `crates/belt-core/src/model.rs` | Modify | Add `phase_verify_passed` to `RunState` |
| `crates/belt-core/src/engine.rs` | Modify | Guard logic in `init`/`step`/`verify_verdict` |
| `crates/belt-agent/src/main.rs` | Modify | `cmd_step` error matching for escalation JSON |
| `crates/belt-core/tests/engine_test.rs` | Modify | 21 new engine-level test cases |
| `crates/belt-core/tests/fixtures/*.yml` | Create | 5 fixture files for multi-phase tests |
| `crates/belt-agent/tests/cli_test.rs` | Modify | 2 new E2E tests for JSON output |

---

### Task 1: Foundation — Error Variants and RunState Extension

**Files:**
- Modify: `crates/belt-core/src/error.rs:4-33`
- Modify: `crates/belt-core/src/model.rs:122-136`
- Modify: `crates/belt-core/src/engine.rs:47-61`

- [ ] **Step 1: Add error variants to `error.rs`**

Add two new variants after the `GateFailed` variant (after line 25):

```rust
    #[error("verify required for phase '{phase_id}': run verify before step")]
    #[diagnostic(code(belt::verify_required))]
    VerifyRequired { phase_id: String },

    #[error("max retries exceeded for phase '{phase_id}': {attempts}/{max_retries}")]
    #[diagnostic(code(belt::max_retries_exceeded))]
    MaxRetriesExceeded {
        phase_id: String,
        attempts: u32,
        max_retries: u32,
    },
```

- [ ] **Step 2: Add `phase_verify_passed` to `RunState` in `model.rs`**

Add after the `phase_attempts` field (after line 133):

```rust
    #[serde(default)]
    pub phase_verify_passed: HashMap<String, bool>,
```

- [ ] **Step 3: Update `Engine::init()` struct literal in `engine.rs`**

Add `phase_verify_passed: HashMap::new(),` to the `RunState` initialization in `init()` (after the `phase_attempts` line):

```rust
        let state = RunState {
            run_id,
            pipeline: pipeline.name,
            pipeline_file: pipeline_path.display().to_string(),
            version: pipeline.version,
            args: args.clone(),
            current_phase: active.id.clone(),
            completed_phases: Vec::new(),
            skipped_phases: Vec::new(),
            phase_attempts: HashMap::new(),
            phase_verify_passed: HashMap::new(),
            created_at: now.clone(),
            updated_at: now,
        };
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p belt-core -p belt-agent`
Expected: Compiles with no errors. No behavior change yet.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/src/error.rs crates/belt-core/src/model.rs crates/belt-core/src/engine.rs
git commit -m "feat(belt-core): add VerifyRequired/MaxRetriesExceeded errors and phase_verify_passed field"
```

---

### Task 2: Create Test Fixtures

**Files:**
- Create: `crates/belt-core/tests/fixtures/gate_pipeline.yml`
- Create: `crates/belt-core/tests/fixtures/gate_confirm_pipeline.yml`
- Create: `crates/belt-core/tests/fixtures/max_retries_pipeline.yml`
- Create: `crates/belt-core/tests/fixtures/when_gate_pipeline.yml`
- Create: `crates/belt-core/tests/fixtures/regate_pipeline.yml`

- [ ] **Step 1: Create `crates/belt-core/tests/fixtures/gate_pipeline.yml`**

```yaml
name: gate-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
  - id: test
    description: "Test"
    gate:
      - file_exists: "test.ok"
  - id: deploy
    description: "Deploy"
    gate:
      - file_exists: "deploy.ok"
```

- [ ] **Step 2: Create `crates/belt-core/tests/fixtures/gate_confirm_pipeline.yml`**

```yaml
name: gate-confirm-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
  - id: review
    description: "Review"
    confirm: true
  - id: deploy
    description: "Deploy"
    gate:
      - file_exists: "deploy.ok"
```

- [ ] **Step 3: Create `crates/belt-core/tests/fixtures/max_retries_pipeline.yml`**

```yaml
name: max-retries-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
    max_retries: 3
  - id: test
    description: "Test"
    gate:
      - file_exists: "test.ok"
    max_retries: 1
```

- [ ] **Step 4: Create `crates/belt-core/tests/fixtures/when_gate_pipeline.yml`**

```yaml
name: when-gate-test
version: 1
args:
  smoke:
    type: bool
    default: false
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
  - id: optional
    description: "Optional"
    gate:
      - file_exists: "optional.ok"
    when: "args.smoke"
  - id: final
    description: "Final"
    gate:
      - file_exists: "final.ok"
```

- [ ] **Step 5: Create `crates/belt-core/tests/fixtures/regate_pipeline.yml`**

```yaml
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
```

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/tests/fixtures/
git commit -m "test(belt-core): add YAML fixtures for engine guard tests"
```

---

### Task 3: Engine — verify_verdict Updates phase_verify_passed (TDD)

**Files:**
- Modify: `crates/belt-core/src/engine.rs:174-183`
- Modify: `crates/belt-core/tests/engine_test.rs`

- [ ] **Step 1: Write failing test**

Append to `engine_test.rs`:

```rust
#[test]
fn verify_verdict_sets_phase_verify_passed() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    assert!(
        state.phase_verify_passed.get("build").is_none()
            || state.phase_verify_passed.get("build") == Some(&true),
        "gate-less phase may be auto-true or absent"
    );

    // verify FAIL -> sets false
    engine.verify_verdict(&mut state, false).expect("verify");
    assert_eq!(state.phase_verify_passed.get("build"), Some(&false));

    // verify PASS -> sets true
    engine.verify_verdict(&mut state, true).expect("verify");
    assert_eq!(state.phase_verify_passed.get("build"), Some(&true));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p belt-core --test engine_test verify_verdict_sets_phase_verify_passed`
Expected: FAIL — `phase_verify_passed` is never updated by `verify_verdict`.

- [ ] **Step 3: Implement in `verify_verdict()`**

In `Engine::verify_verdict()`, add after `*count += 1;`:

```rust
        state
            .phase_verify_passed
            .insert(state.current_phase.clone(), passed);
```

Full method:

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
        state.updated_at = now_iso8601();
        self.save_state(state)?;
        Ok(passed)
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p belt-core --test engine_test verify_verdict_sets_phase_verify_passed`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/src/engine.rs crates/belt-core/tests/engine_test.rs
git commit -m "feat(belt-core): verify_verdict updates phase_verify_passed"
```

---

### Task 4: Engine — Gate-less Auto-true in init and step (TDD)

**Files:**
- Modify: `crates/belt-core/src/engine.rs:31-70` (init), `crates/belt-core/src/engine.rs:126-168` (step)
- Modify: `crates/belt-core/tests/engine_test.rs`

- [ ] **Step 1: Write failing test — init auto-sets true for gate-less first phase (#11)**

```rust
#[test]
fn init_auto_sets_verify_for_gateless_phase() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    assert_eq!(
        state.phase_verify_passed.get("build"),
        Some(&true),
        "gate-less first phase should auto-set verify true"
    );
}
```

- [ ] **Step 2: Write test — init does NOT auto-set for gate phase**

```rust
#[test]
fn init_does_not_auto_set_verify_for_gate_phase() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    assert!(
        state.phase_verify_passed.get("build").is_none(),
        "gate phase should NOT auto-set verify"
    );
}
```

- [ ] **Step 3: Run tests to check initial state**

Run: `cargo test -p belt-core --test engine_test init_auto_sets_verify init_does_not_auto_set`
Expected: `init_auto_sets_verify_for_gateless_phase` FAILS. `init_does_not_auto_set_verify_for_gate_phase` PASSES.

- [ ] **Step 4: Implement auto-true in `init()`**

In `Engine::init()`, replace `phase_verify_passed: HashMap::new(),` with:

```rust
            phase_verify_passed: if active.gate.is_empty() {
                HashMap::from([(active.id.clone(), true)])
            } else {
                HashMap::new()
            },
```

- [ ] **Step 5: Run tests to verify**

Run: `cargo test -p belt-core --test engine_test init_auto_sets_verify init_does_not_auto_set`
Expected: Both PASS.

- [ ] **Step 6: Write failing test — step auto-sets true for gate-less next phase (#12)**

```rust
#[test]
fn step_auto_sets_verify_for_gateless_next_phase() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Manually set verify for current phase (guard not yet added)
    state.phase_verify_passed.insert("build".to_string(), true);
    engine.save_state(&state).expect("save");

    let next = engine.step(&mut state, &pipeline_path).expect("step");
    assert_eq!(next.as_deref(), Some("done"));
    assert_eq!(
        state.phase_verify_passed.get("done"),
        Some(&true),
        "gate-less next phase should auto-set verify true"
    );
}
```

- [ ] **Step 7: Run test to verify it fails**

Run: `cargo test -p belt-core --test engine_test step_auto_sets_verify_for_gateless_next_phase`
Expected: FAIL — `phase_verify_passed["done"]` not set.

- [ ] **Step 8: Implement auto-true in `step()`**

In `Engine::step()`, replace the next-phase search loop and match block. The loop must track gate emptiness, and the `Some` branch must auto-set:

Replace from `// Find next active phase` (line ~140) through the end of `match`:

```rust
        // Find next active phase, skipping those whose `when:` is false
        let mut next_phase = None;
        let mut next_gate_empty = false;
        for phase in &phases[current_idx + 1..] {
            if eval_when(phase.when.as_ref(), &state.args) {
                next_phase = Some(phase.id.clone());
                next_gate_empty = phase.gate.is_empty();
                break;
            }
            state.skipped_phases.push(phase.id.clone());
        }

        match &next_phase {
            Some(next_id) => {
                state.current_phase.clone_from(next_id);
                // Create output_dir for next phase
                let run_dir = self.belt_dir.join("runs").join(&state.run_id);
                let output_dir = run_dir.join(next_id.replace('/', "_"));
                std::fs::create_dir_all(&output_dir)?;
                // Auto-set verify for gate-less next phase
                if next_gate_empty {
                    state.phase_verify_passed.insert(next_id.clone(), true);
                }
            }
            None => {
                // Pipeline complete
                state.current_phase = "COMPLETED".to_string();
            }
        }
```

- [ ] **Step 9: Run test and full suite**

Run: `cargo test -p belt-core --test engine_test`
Expected: All PASS (existing tests use gate-less pipelines, auto-true covers them).

- [ ] **Step 10: Commit**

```bash
git add crates/belt-core/src/engine.rs crates/belt-core/tests/engine_test.rs
git commit -m "feat(belt-core): auto-set phase_verify_passed for gate-less phases in init/step"
```

---

### Task 5: Engine — verify-before-step Guard (TDD)

**Files:**
- Modify: `crates/belt-core/src/engine.rs:126-140`
- Modify: `crates/belt-core/tests/engine_test.rs`

- [ ] **Step 1: Write failing tests #1, #3, #5**

```rust
#[test]
fn step_without_verify_returns_verify_required() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::VerifyRequired { phase_id }) if phase_id == "build"),
        "expected VerifyRequired, got: {result:?}"
    );
}

#[test]
fn step_after_verify_fail_returns_verify_required() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    engine.verify_verdict(&mut state, false).expect("verify FAIL");
    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::VerifyRequired { phase_id }) if phase_id == "build"),
        "expected VerifyRequired after FAIL, got: {result:?}"
    );
}

#[test]
fn verify_pass_does_not_carry_to_next_gate_phase() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
  - id: test
    description: "Test"
    gate:
      - file_exists: "test.ok"
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // verify PASS build, step to test
    engine.verify_verdict(&mut state, true).expect("verify build");
    let next = engine.step(&mut state, &pipeline_path).expect("step to test");
    assert_eq!(next.as_deref(), Some("test"));

    // step without verifying test phase
    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::VerifyRequired { phase_id }) if phase_id == "test"),
        "expected VerifyRequired for unverified next phase, got: {result:?}"
    );
}
```

- [ ] **Step 2: Write tests #2, #4 (expected to pass already)**

```rust
#[test]
fn step_after_verify_pass_succeeds() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    engine.verify_verdict(&mut state, true).expect("verify");
    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step should succeed after PASS");
    assert_eq!(next.as_deref(), Some("done"));
}

#[test]
fn step_on_gateless_phase_succeeds_without_verify() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("gate-less step should succeed without verify");
    assert_eq!(next.as_deref(), Some("test"));
}
```

- [ ] **Step 3: Run tests — expect #1, #3, #5 to fail**

Run: `cargo test -p belt-core --test engine_test step_without_verify step_after_verify_fail verify_pass_does_not_carry step_after_verify_pass step_on_gateless`
Expected: #1, #3, #5 FAIL (no guard). #2, #4 PASS.

- [ ] **Step 4: Implement verify-before-step guard**

In `Engine::step()`, add immediately after finding `current_idx` (before `// Mark current as completed`):

```rust
        // Guard: verify-before-step
        if state.phase_verify_passed.get(&state.current_phase) != Some(&true) {
            return Err(BeltError::VerifyRequired {
                phase_id: state.current_phase.clone(),
            });
        }
```

- [ ] **Step 5: Run all tests**

Run: `cargo test -p belt-core --test engine_test`
Expected: All PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/engine.rs crates/belt-core/tests/engine_test.rs
git commit -m "feat(belt-core): add verify-before-step guard to Engine::step"
```

---

### Task 6: Engine — max_retries Guard (TDD)

**Files:**
- Modify: `crates/belt-core/src/engine.rs:126-145`
- Modify: `crates/belt-core/tests/engine_test.rs`

- [ ] **Step 1: Write failing test — exceeding max_retries returns error (#7)**

```rust
#[test]
fn step_exceeding_max_retries_returns_error() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
    max_retries: 3
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // 3 FAIL + 1 PASS = 4 attempts; 4 > 3 triggers MaxRetriesExceeded
    engine.verify_verdict(&mut state, false).expect("verify");
    engine.verify_verdict(&mut state, false).expect("verify");
    engine.verify_verdict(&mut state, false).expect("verify");
    engine.verify_verdict(&mut state, true).expect("verify PASS");

    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(
            &result,
            Err(BeltError::MaxRetriesExceeded { phase_id, attempts, max_retries })
            if phase_id == "build" && *attempts == 4 && *max_retries == 3
        ),
        "expected MaxRetriesExceeded, got: {result:?}"
    );
}
```

- [ ] **Step 2: Write test — within max_retries succeeds (#6)**

```rust
#[test]
fn step_within_max_retries_succeeds() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
    max_retries: 3
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // 2 FAIL + 1 PASS = 3 attempts; 3 > 3 is false -> OK
    engine.verify_verdict(&mut state, false).expect("verify");
    engine.verify_verdict(&mut state, false).expect("verify");
    engine.verify_verdict(&mut state, true).expect("verify PASS");

    let next = engine.step(&mut state, &pipeline_path).expect("step should succeed");
    assert_eq!(next.as_deref(), Some("done"));
}
```

- [ ] **Step 3: Write test — max_retries 0 is unlimited (#8)**

```rust
#[test]
fn step_with_zero_max_retries_is_unlimited() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // 10 FAILs + 1 PASS, max_retries: 0 (default) = unlimited
    for _ in 0..10 {
        engine.verify_verdict(&mut state, false).expect("verify");
    }
    engine.verify_verdict(&mut state, true).expect("verify PASS");

    let next = engine.step(&mut state, &pipeline_path).expect("step should succeed");
    assert_eq!(next.as_deref(), Some("done"));
}
```

- [ ] **Step 4: Write test — verify works after max_retries exceeded (#9)**

```rust
#[test]
fn verify_still_works_after_max_retries_exceeded() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
    max_retries: 2
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // 3 attempts on max_retries: 2 -> exceeded
    engine.verify_verdict(&mut state, false).expect("verify 1");
    engine.verify_verdict(&mut state, false).expect("verify 2");
    engine.verify_verdict(&mut state, true).expect("verify 3 PASS");

    // step blocked
    assert!(matches!(
        engine.step(&mut state, &pipeline_path),
        Err(BeltError::MaxRetriesExceeded { .. })
    ));

    // verify still works (attempt 4)
    let result = engine.verify_verdict(&mut state, true);
    assert!(result.is_ok());
    assert_eq!(state.phase_attempts.get("build").copied(), Some(4));
}
```

- [ ] **Step 5: Run tests — expect #7 to fail**

Run: `cargo test -p belt-core --test engine_test step_exceeding_max step_within_max step_with_zero_max verify_still_works_after_max`
Expected: `step_exceeding_max_retries_returns_error` FAILS (no guard). Others PASS.

- [ ] **Step 6: Implement max_retries guard**

In `Engine::step()`, add after the verify-before-step guard (after the `VerifyRequired` block):

```rust
        // Guard: max_retries
        let current_phase_def = &phases[current_idx];
        if current_phase_def.max_retries > 0 {
            let attempts = state
                .phase_attempts
                .get(&state.current_phase)
                .copied()
                .unwrap_or(0);
            if attempts > current_phase_def.max_retries {
                return Err(BeltError::MaxRetriesExceeded {
                    phase_id: state.current_phase.clone(),
                    attempts,
                    max_retries: current_phase_def.max_retries,
                });
            }
        }
```

- [ ] **Step 7: Run all tests**

Run: `cargo test -p belt-core --test engine_test`
Expected: All PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/belt-core/src/engine.rs crates/belt-core/tests/engine_test.rs
git commit -m "feat(belt-core): add max_retries guard to Engine::step"
```

---

### Task 7: Guard Evaluation Order and RunState Persistence Tests

**Files:**
- Modify: `crates/belt-core/tests/engine_test.rs`

- [ ] **Step 1: Write test — guard evaluation order (#10)**

```rust
#[test]
fn guard_order_verify_required_before_max_retries() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
    max_retries: 3
  - id: done
    description: "Done"
"#,
    );
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // 4 FAIL attempts: verify_passed = false, attempts = 4 > max_retries = 3
    // Both guards would trigger, but VerifyRequired must fire first
    for _ in 0..4 {
        engine.verify_verdict(&mut state, false).expect("verify");
    }

    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::VerifyRequired { .. })),
        "VerifyRequired should fire before MaxRetriesExceeded, got: {result:?}"
    );
}
```

- [ ] **Step 2: Write test — RunState persistence round-trip (#13)**

```rust
#[test]
fn phase_verify_passed_persists_across_load() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    engine.verify_verdict(&mut state, true).expect("verify");

    // Reload state from disk
    let loaded = engine.load_state(&state.run_id).expect("load");
    assert_eq!(
        loaded.phase_verify_passed.get("build"),
        Some(&true),
        "phase_verify_passed should survive save/load round-trip"
    );
}
```

- [ ] **Step 3: Run tests — expect all to pass**

Run: `cargo test -p belt-core --test engine_test guard_order phase_verify_passed_persists`
Expected: Both PASS (guards implemented in Tasks 5-6, persistence via serde).

- [ ] **Step 4: Commit**

```bash
git add crates/belt-core/tests/engine_test.rs
git commit -m "test(belt-core): add guard evaluation order and persistence tests"
```

---

### Task 8: Fixture-based Full Lifecycle Tests

**Files:**
- Modify: `crates/belt-core/tests/engine_test.rs`

- [ ] **Step 1: Add fixture helper**

Add at the top of `engine_test.rs` (after existing imports/helpers):

```rust
fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}
```

- [ ] **Step 2: Write test — full lifecycle with gate pipeline (#16)**

```rust
#[test]
fn lifecycle_gate_pipeline_init_verify_step_to_completion() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline_path = fixture_path("gate_pipeline.yml");
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    assert_eq!(state.current_phase, "build");
    assert!(state.phase_verify_passed.get("build").is_none());

    // Phase 1: build
    engine.verify_verdict(&mut state, true).expect("verify build");
    let next = engine.step(&mut state, &pipeline_path).expect("step build->test");
    assert_eq!(next.as_deref(), Some("test"));

    // Phase 2: test
    engine.verify_verdict(&mut state, true).expect("verify test");
    let next = engine.step(&mut state, &pipeline_path).expect("step test->deploy");
    assert_eq!(next.as_deref(), Some("deploy"));

    // Phase 3: deploy
    engine.verify_verdict(&mut state, true).expect("verify deploy");
    let next = engine.step(&mut state, &pipeline_path).expect("step deploy->COMPLETED");
    assert!(next.is_none());
    assert_eq!(state.current_phase, "COMPLETED");
    assert_eq!(state.completed_phases, vec!["build", "test", "deploy"]);
}
```

- [ ] **Step 3: Write test — gate + confirm mixed lifecycle (#17)**

```rust
#[test]
fn lifecycle_gate_confirm_mixed_pipeline() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline_path = fixture_path("gate_confirm_pipeline.yml");
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Phase 1: build (has gate)
    assert_eq!(state.current_phase, "build");
    assert!(state.phase_verify_passed.get("build").is_none());
    engine.verify_verdict(&mut state, true).expect("verify build");
    let next = engine.step(&mut state, &pipeline_path).expect("step build->review");
    assert_eq!(next.as_deref(), Some("review"));

    // Phase 2: review (confirm only, no gate -> auto-true)
    assert_eq!(
        state.phase_verify_passed.get("review"),
        Some(&true),
        "confirm-only phase should auto-set verify"
    );
    // step works without explicit verify
    let next = engine.step(&mut state, &pipeline_path).expect("step review->deploy");
    assert_eq!(next.as_deref(), Some("deploy"));

    // Phase 3: deploy (has gate)
    assert!(
        state.phase_verify_passed.get("deploy").is_none(),
        "gate phase should not auto-set"
    );
    engine.verify_verdict(&mut state, true).expect("verify deploy");
    let next = engine.step(&mut state, &pipeline_path).expect("step deploy->COMPLETED");
    assert!(next.is_none());
    assert_eq!(state.current_phase, "COMPLETED");
}
```

- [ ] **Step 4: Write test — max_retries recovery flow (#18)**

```rust
#[test]
fn lifecycle_max_retries_recovery() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline_path = fixture_path("max_retries_pipeline.yml");
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Phase 1: build (max_retries: 3)
    // FAIL twice, then PASS on attempt 3 (within limit)
    engine.verify_verdict(&mut state, false).expect("verify FAIL 1");
    engine.verify_verdict(&mut state, false).expect("verify FAIL 2");
    engine.verify_verdict(&mut state, true).expect("verify PASS 3");

    let next = engine.step(&mut state, &pipeline_path).expect("step build->test");
    assert_eq!(next.as_deref(), Some("test"));
    assert_eq!(state.phase_attempts.get("build").copied(), Some(3));

    // Phase 2: test (max_retries: 1)
    // PASS on first attempt
    engine.verify_verdict(&mut state, true).expect("verify PASS 1");
    let next = engine.step(&mut state, &pipeline_path).expect("step test->COMPLETED");
    assert!(next.is_none());
    assert_eq!(state.current_phase, "COMPLETED");
}
```

- [ ] **Step 5: Run fixture tests**

Run: `cargo test -p belt-core --test engine_test lifecycle_`
Expected: All 3 PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/tests/engine_test.rs
git commit -m "test(belt-core): add fixture-based full lifecycle tests for engine guards"
```

---

### Task 9: Fixture-based Compound Condition and Edge Case Tests

**Files:**
- Modify: `crates/belt-core/tests/engine_test.rs`

- [ ] **Step 1: Write test — when: skipped phase does not pollute verify state (#19)**

```rust
#[test]
fn when_skipped_phase_does_not_pollute_verify_state() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline_path = fixture_path("when_gate_pipeline.yml");

    let mut args = HashMap::new();
    args.insert("smoke".to_string(), serde_json::Value::Bool(false));
    let mut state = engine.init(&pipeline_path, &args).expect("init");

    // build phase (has gate)
    engine.verify_verdict(&mut state, true).expect("verify build");
    let next = engine.step(&mut state, &pipeline_path).expect("step");
    // "optional" is skipped (when: args.smoke = false), goes to "final"
    assert_eq!(next.as_deref(), Some("final"));
    assert_eq!(state.skipped_phases, vec!["optional"]);

    // "optional" should NOT be in phase_verify_passed
    assert!(
        state.phase_verify_passed.get("optional").is_none(),
        "skipped phase should not have verify state"
    );

    // "final" has gate, requires verify
    let result = engine.step(&mut state, &pipeline_path);
    assert!(matches!(result, Err(BeltError::VerifyRequired { .. })));
}
```

- [ ] **Step 2: Write test — regate pipeline verify/step works (#20)**

```rust
#[test]
fn regate_pipeline_verify_step_works() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline_path = fixture_path("regate_pipeline.yml");
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Phase 1: design
    engine.verify_verdict(&mut state, true).expect("verify design");
    let next = engine.step(&mut state, &pipeline_path).expect("step design->build");
    assert_eq!(next.as_deref(), Some("build"));

    // Phase 2: build (has regate: [design])
    engine.verify_verdict(&mut state, true).expect("verify build");
    let next = engine.step(&mut state, &pipeline_path).expect("step build->test");
    assert_eq!(next.as_deref(), Some("test"));

    // Phase 3: test
    engine.verify_verdict(&mut state, true).expect("verify test");
    let next = engine.step(&mut state, &pipeline_path).expect("step test->COMPLETED");
    assert!(next.is_none());

    // Verify state is intact
    assert_eq!(state.completed_phases, vec!["design", "build", "test"]);
    assert!(state.skipped_phases.is_empty());
}
```

- [ ] **Step 3: Write test — max_retries: 1 immediate escalation (#21)**

```rust
#[test]
fn max_retries_one_immediate_escalation() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline_path = fixture_path("max_retries_pipeline.yml");
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Advance to "test" phase (max_retries: 1)
    engine.verify_verdict(&mut state, true).expect("verify build");
    engine.step(&mut state, &pipeline_path).expect("step to test");
    assert_eq!(state.current_phase, "test");

    // 1 FAIL + 1 PASS = 2 attempts; 2 > 1 -> MaxRetriesExceeded
    engine.verify_verdict(&mut state, false).expect("verify FAIL");
    engine.verify_verdict(&mut state, true).expect("verify PASS");

    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(
            &result,
            Err(BeltError::MaxRetriesExceeded { phase_id, attempts, max_retries })
            if phase_id == "test" && *attempts == 2 && *max_retries == 1
        ),
        "expected immediate escalation with max_retries: 1, got: {result:?}"
    );
}
```

- [ ] **Step 4: Write test — consecutive step without verify (#22)**

```rust
#[test]
fn consecutive_step_without_verify_always_rejected() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline_path = fixture_path("gate_pipeline.yml");
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Try step 3 times without verify
    for _ in 0..3 {
        let result = engine.step(&mut state, &pipeline_path);
        assert!(
            matches!(&result, Err(BeltError::VerifyRequired { phase_id }) if phase_id == "build"),
            "each step without verify should return VerifyRequired"
        );
    }

    // State should not have changed
    assert_eq!(state.current_phase, "build");
    assert!(state.completed_phases.is_empty());
}
```

- [ ] **Step 5: Write test — after escalation, verify works but step stays rejected (#23)**

```rust
#[test]
fn after_escalation_verify_works_but_step_rejected() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline_path = fixture_path("max_retries_pipeline.yml");
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // build phase: max_retries: 3
    // 3 FAIL + 1 PASS = 4 attempts -> escalation
    for _ in 0..3 {
        engine.verify_verdict(&mut state, false).expect("verify FAIL");
    }
    engine.verify_verdict(&mut state, true).expect("verify PASS");

    // step -> MaxRetriesExceeded
    assert!(matches!(
        engine.step(&mut state, &pipeline_path),
        Err(BeltError::MaxRetriesExceeded { .. })
    ));

    // verify still works (attempt 5)
    engine.verify_verdict(&mut state, true).expect("verify should still work");
    assert_eq!(state.phase_attempts.get("build").copied(), Some(5));

    // step still rejected (5 > 3)
    assert!(matches!(
        engine.step(&mut state, &pipeline_path),
        Err(BeltError::MaxRetriesExceeded { .. })
    ));
}
```

- [ ] **Step 6: Run all fixture tests**

Run: `cargo test -p belt-core --test engine_test`
Expected: All PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/belt-core/tests/engine_test.rs
git commit -m "test(belt-core): add fixture-based compound condition and edge case tests"
```

---

### Task 10: belt-agent JSON Error Handling

**Files:**
- Modify: `crates/belt-agent/src/main.rs:1-6` (imports), `crates/belt-agent/src/main.rs:224-272` (cmd_step)
- Modify: `crates/belt-agent/tests/cli_test.rs`

- [ ] **Step 1: Write E2E test — step without verify returns verify_required JSON (#14)**

Append to `crates/belt-agent/tests/cli_test.rs`:

```rust
#[test]
fn step_without_verify_returns_verify_required_json() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: guard-test
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

    // init
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["init", "pipeline.yml"])
        .current_dir(dir.path())
        .output()
        .expect("init failed");
    assert!(output.status.success());
    let init_json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let run_id = init_json["run_id"].as_str().unwrap();

    // step WITHOUT verify -> verify_required
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["step", "--run", run_id])
        .current_dir(dir.path())
        .output()
        .expect("step failed");
    assert!(
        output.status.success(),
        "step should succeed (exit 0) even for guard errors: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["advanced"], false);
    assert_eq!(json["reason"], "verify_required");
    assert_eq!(json["phase"], "build");
}
```

- [ ] **Step 2: Write E2E test — max_retries exceeded returns escalation JSON (#15)**

```rust
#[test]
fn step_after_max_retries_returns_escalation_json() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: escalation-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "false"
    max_retries: 2
  - id: done
    description: "Done"
"#,
    );

    // init
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["init", "pipeline.yml"])
        .current_dir(dir.path())
        .output()
        .expect("init failed");
    assert!(output.status.success());
    let init_json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let run_id = init_json["run_id"].as_str().unwrap();

    // verify 3 times (gate: cmd: "false" -> all FAIL, but verify_verdict records attempts)
    // Then manually verify PASS to get past VerifyRequired guard
    for _ in 0..2 {
        let output = Command::cargo_bin("belt-agent")
            .unwrap()
            .args(["verify", "--run", run_id])
            .current_dir(dir.path())
            .output()
            .expect("verify failed");
        assert!(output.status.success());
    }
    // 3rd verify with a passing gate to get PASS verdict
    // Change gate to pass: create a dummy approach
    // Actually, verify runs the actual gate checks. cmd: "false" always fails.
    // We need a pipeline where the gate can transition from FAIL to PASS.
    // Use file_exists instead:
    let _ = dir; // drop, restart with file_exists gate

    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: escalation-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "build.ok"
    max_retries: 2
  - id: done
    description: "Done"
"#,
    );

    // init
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["init", "pipeline.yml"])
        .current_dir(dir.path())
        .output()
        .expect("init failed");
    assert!(output.status.success());
    let init_json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let run_id = init_json["run_id"].as_str().unwrap();

    // verify twice (file doesn't exist -> FAIL)
    for _ in 0..2 {
        let output = Command::cargo_bin("belt-agent")
            .unwrap()
            .args(["verify", "--run", run_id])
            .current_dir(dir.path())
            .output()
            .expect("verify failed");
        assert!(output.status.success());
        let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(json["verdict"], "FAIL");
    }

    // Create the file so 3rd verify passes
    std::fs::write(dir.path().join("build.ok"), "").unwrap();
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["verify", "--run", run_id])
        .current_dir(dir.path())
        .output()
        .expect("verify failed");
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["verdict"], "PASS");

    // step -> 3 attempts > max_retries 2 -> escalation
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["step", "--run", run_id])
        .current_dir(dir.path())
        .output()
        .expect("step failed");
    assert!(
        output.status.success(),
        "step should succeed (exit 0): {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["advanced"], false);
    assert_eq!(json["reason"], "max_retries_exceeded");
    assert_eq!(json["phase"], "build");
    assert_eq!(json["attempts"], 3);
    assert_eq!(json["max_retries"], 2);
    assert_eq!(json["escalation"], true);
}
```

- [ ] **Step 3: Run E2E tests to verify they fail**

Run: `cargo test -p belt-agent --test cli_test step_without_verify_returns_verify_required_json step_after_max_retries_returns_escalation_json`
Expected: FAIL — `cmd_step` doesn't handle the new error variants yet.

- [ ] **Step 4: Add `BeltError` import to `main.rs`**

Add to the imports at the top of `crates/belt-agent/src/main.rs`:

```rust
use belt_core::error::BeltError;
```

- [ ] **Step 5: Modify `cmd_step` to handle guard errors**

Replace the step call and result handling in `cmd_step` (the section after the confirm check). Replace from `let from = state.current_phase.clone();` through the end of the function:

```rust
    let from = state.current_phase.clone();
    match engine.step(&mut state, pipeline_path) {
        Ok(next) => {
            let out = match next {
                Some(to) => json!({
                    "advanced": true,
                    "from": from,
                    "to": to,
                }),
                None => json!({
                    "advanced": true,
                    "from": from,
                    "to": null,
                    "completed": true,
                }),
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
            );
        }
        Err(BeltError::VerifyRequired { phase_id }) => {
            let out = json!({
                "advanced": false,
                "reason": "verify_required",
                "phase": phase_id,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
            );
        }
        Err(BeltError::MaxRetriesExceeded {
            phase_id,
            attempts,
            max_retries,
        }) => {
            let out = json!({
                "advanced": false,
                "reason": "max_retries_exceeded",
                "phase": phase_id,
                "attempts": attempts,
                "max_retries": max_retries,
                "escalation": true,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
            );
        }
        Err(e) => return Err(miette::miette!("{e}")),
    }
    Ok(())
```

- [ ] **Step 6: Run E2E tests**

Run: `cargo test -p belt-agent --test cli_test step_without_verify_returns_verify_required_json step_after_max_retries_returns_escalation_json`
Expected: Both PASS.

- [ ] **Step 7: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests PASS. Verify existing E2E tests (`full_flow_init_next_verify_step`, `step_without_confirm_on_confirm_phase`, `e2e_sub_pipeline_expansion`) still pass.

- [ ] **Step 8: Run clippy and fmt**

Run: `cargo fmt --package belt-core --package belt-agent && cargo clippy --package belt-core --package belt-agent -- -D warnings`
Expected: No errors or warnings.

- [ ] **Step 9: Commit**

```bash
git add crates/belt-agent/src/main.rs crates/belt-agent/tests/cli_test.rs
git commit -m "feat(belt-agent): surface VerifyRequired/MaxRetriesExceeded as structured JSON"
```
