#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unnecessary_get_then_check,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use belt_core::engine::Engine;
use belt_core::error::BeltError;
use std::collections::HashMap;
use tempfile::TempDir;

mod common;
use common::helpers::{fixture_path, write_yaml};

/// Helper: create a simple two-phase pipeline YAML.
fn two_phase_pipeline(dir: &TempDir) -> std::path::PathBuf {
    write_yaml(
        dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build the project"
  - id: test
    description: "Run tests"
"#,
    )
}

// ---------------------------------------------------------------------------
// Test 1: engine_init_creates_run_with_first_phase
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-init-creates-run-with-first-active-phase
#[test]
fn engine_init_creates_run_with_first_phase() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let args = HashMap::new();

    let state = engine
        .init(&pipeline_path, &args)
        .expect("init should succeed");

    assert_eq!(state.pipeline, "test");
    assert_eq!(state.current_phase, "build");
    assert!(state.completed_phases.is_empty());
    assert!(state.skipped_phases.is_empty());
    assert_eq!(state.version, 1);

    // state.json should exist on disk
    let state_path = belt_dir.join("runs").join(&state.run_id).join("state.json");
    assert!(state_path.exists(), "state.json should be persisted");

    // output_dir for first phase should exist
    let output_dir = belt_dir.join("runs").join(&state.run_id).join("build");
    assert!(
        output_dir.exists(),
        "output_dir for first phase should exist"
    );
}

// ---------------------------------------------------------------------------
// Test 2: engine_step_advances_to_next_phase
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-advances-through-active-phases-and-completes
#[test]
fn engine_step_advances_to_next_phase() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let args = HashMap::new();

    let mut state = engine.init(&pipeline_path, &args).expect("init");

    // Step from build -> test
    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step should succeed");
    assert_eq!(next.as_deref(), Some("test"));
    assert_eq!(state.current_phase, "test");
    assert_eq!(state.completed_phases, vec!["build"]);

    // output_dir for "test" phase should exist
    let output_dir = belt_dir.join("runs").join(&state.run_id).join("test");
    assert!(
        output_dir.exists(),
        "output_dir for test phase should exist"
    );

    // Step from test -> COMPLETED
    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step should succeed");
    assert!(next.is_none(), "pipeline should be complete");
    assert_eq!(state.current_phase, "COMPLETED");
    assert_eq!(state.completed_phases, vec!["build", "test"]);
}

// ---------------------------------------------------------------------------
// Test 3: engine_skips_phase_with_false_when
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-skips-phases-whose-when-is-false
#[test]
fn engine_skips_phase_with_false_when() {
    let dir = TempDir::new().expect("tempdir");

    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: conditional
version: 1
phases:
  - id: build
    description: "Build"
  - id: conditional
    description: "Conditional phase"
    when: "args.smoke"
  - id: final
    description: "Final phase"
"#,
    );

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);

    let mut args = HashMap::new();
    args.insert("smoke".to_string(), serde_json::Value::Bool(false));

    let mut state = engine.init(&pipeline_path, &args).expect("init");
    assert_eq!(state.current_phase, "build");

    // Step: should skip "conditional" and go directly to "final"
    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step should succeed");
    assert_eq!(next.as_deref(), Some("final"));
    assert_eq!(state.current_phase, "final");
    assert_eq!(state.completed_phases, vec!["build"]);
    assert_eq!(state.skipped_phases, vec!["conditional"]);
}

// ---------------------------------------------------------------------------
// Test 4: engine_verify_verdict_increments_attempts
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-verify-verdict-increments-attempts-and-records-outcome
#[test]
fn engine_verify_verdict_increments_attempts() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let args = HashMap::new();

    let mut state = engine.init(&pipeline_path, &args).expect("init");

    // First verdict: fail
    let result = engine
        .verify_verdict(&mut state, false)
        .expect("verify should succeed");
    assert!(!result);
    assert_eq!(state.phase_attempts.get("build").copied(), Some(1));

    // Second verdict: fail again
    let result = engine
        .verify_verdict(&mut state, false)
        .expect("verify should succeed");
    assert!(!result);
    assert_eq!(state.phase_attempts.get("build").copied(), Some(2));

    // Third verdict: pass
    let result = engine
        .verify_verdict(&mut state, true)
        .expect("verify should succeed");
    assert!(result);
    assert_eq!(state.phase_attempts.get("build").copied(), Some(3));
}

// ---------------------------------------------------------------------------
// Test 5: engine_load_state_round_trip
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-state-round-trips-through-save-load
#[test]
fn engine_load_state_round_trip() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let args = HashMap::new();

    let state = engine.init(&pipeline_path, &args).expect("init");
    let loaded = engine
        .load_state(&state.run_id)
        .expect("load_state should succeed");

    assert_eq!(loaded.run_id, state.run_id);
    assert_eq!(loaded.pipeline, state.pipeline);
    assert_eq!(loaded.pipeline_file, state.pipeline_file);
    assert_eq!(loaded.version, state.version);
    assert_eq!(loaded.current_phase, state.current_phase);
    assert_eq!(loaded.completed_phases, state.completed_phases);
    assert_eq!(loaded.skipped_phases, state.skipped_phases);
    assert_eq!(loaded.created_at, state.created_at);
    assert_eq!(loaded.updated_at, state.updated_at);
}

// ---------------------------------------------------------------------------
// Adversarial: load_state with non-existent run returns State error
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-latest-run-id-returns-most-recent-or-errors-when-empty
#[test]
fn engine_load_state_missing_run_returns_error() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);

    let result = engine.load_state("nonexistent-run-id");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), BeltError::State { .. }));
}

// ---------------------------------------------------------------------------
// Adversarial: latest_run_id with no runs returns State error
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-latest-run-id-returns-most-recent-or-errors-when-empty
#[test]
fn engine_latest_run_id_no_runs_returns_error() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);

    let result = engine.latest_run_id();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), BeltError::State { .. }));
}

// ---------------------------------------------------------------------------
// Adversarial: init with all phases having false `when:` returns error
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-init-error-paths
#[test]
fn engine_init_no_active_phases_returns_error() {
    let dir = TempDir::new().expect("tempdir");

    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: no-active
version: 1
phases:
  - id: only
    description: "Only phase"
    when: "args.enabled"
"#,
    );

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let args = HashMap::new(); // "enabled" not present -> false

    let result = engine.init(&pipeline_path, &args);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        BeltError::InvalidPipeline { .. }
    ));
}

// ---------------------------------------------------------------------------
// next_phase_info sets output_dir correctly
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-next-phase-info-exposes-output-dir
#[test]
fn engine_next_phase_info_sets_output_dir() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let args = HashMap::new();

    let state = engine.init(&pipeline_path, &args).expect("init");

    let phase = engine
        .next_phase_info(&state, &pipeline_path)
        .expect("next_phase_info should succeed");
    assert_eq!(phase.id, "build");
    assert!(phase.output_dir.is_some());

    let output_dir = phase.output_dir.expect("output_dir should be set");
    assert!(
        std::path::Path::new(&output_dir).exists(),
        "output_dir should exist on disk"
    );
}

// ---------------------------------------------------------------------------
// Test: init auto-sets verify for gate-less first phase
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-init-auto-sets-verify-for-gateless-first-phase
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

// ---------------------------------------------------------------------------
// Test: init does NOT auto-set verify for gate phase
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-init-auto-sets-verify-for-gateless-first-phase
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

// ---------------------------------------------------------------------------
// Test: phase_verify_passed round-trips through save/load
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-state-round-trips-through-save-load
#[test]
fn engine_phase_verify_passed_round_trip() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let args = HashMap::new();

    let mut state = engine.init(&pipeline_path, &args).expect("init");

    // Manually set a verify flag and persist
    state.phase_verify_passed.insert("build".to_string(), true);
    engine.save_state(&state).expect("save");

    let loaded = engine.load_state(&state.run_id).expect("load");
    assert_eq!(
        loaded.phase_verify_passed.get("build").copied(),
        Some(true),
        "phase_verify_passed should survive round-trip"
    );
}

// ---------------------------------------------------------------------------
// Test: VerifyRequired error variant
// ---------------------------------------------------------------------------
/// scenario: belt-core-error-display-verify-required-preserves-phase-id
#[test]
fn error_verify_required_message() {
    let phase_id = "build".to_string();
    let err = BeltError::VerifyRequired {
        phase_id: phase_id.clone(),
    };
    let msg = err.to_string();
    assert!(
        msg.contains(&format!("for phase '{phase_id}'")),
        "error message must preserve phase_id '{phase_id}' in quoted form: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Test: MaxRetriesExceeded error variant
// ---------------------------------------------------------------------------
/// scenario: belt-core-error-display-max-retries-preserves-phase-id-and-counter
#[test]
fn error_max_retries_exceeded_message() {
    let phase_id = "deploy".to_string();
    let attempts = 3u32;
    let max_retries = 3u32;
    let err = BeltError::MaxRetriesExceeded {
        phase_id: phase_id.clone(),
        attempts,
        max_retries,
    };
    let msg = err.to_string();
    assert!(
        msg.contains(&format!("for phase '{phase_id}'")),
        "error message must preserve phase_id '{phase_id}' in quoted form: {msg}"
    );
    assert!(
        msg.contains(&format!("{attempts}/{max_retries}")),
        "error message must preserve attempts/max_retries ratio '{attempts}/{max_retries}': {msg}"
    );
}

// ---------------------------------------------------------------------------
// latest_run_id returns the most recent run
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-latest-run-id-returns-most-recent-or-errors-when-empty
#[test]
fn engine_latest_run_id_returns_most_recent() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let args = HashMap::new();

    let state1 = engine.init(&pipeline_path, &args).expect("init 1");
    // Small delay to ensure UUIDv7 ordering
    std::thread::sleep(std::time::Duration::from_millis(2));
    let state2 = engine.init(&pipeline_path, &args).expect("init 2");

    let latest = engine.latest_run_id().expect("latest should succeed");
    // UUIDv7 is time-ordered; the second run should be latest
    assert_eq!(latest, state2.run_id);
    assert_ne!(latest, state1.run_id);
}

// ---------------------------------------------------------------------------
// Test: verify_verdict sets phase_verify_passed
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-verify-verdict-increments-attempts-and-records-outcome
#[test]
fn verify_verdict_sets_phase_verify_passed() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // verify FAIL -> sets false
    engine.verify_verdict(&mut state, false).expect("verify");
    assert_eq!(state.phase_verify_passed.get("build"), Some(&false));

    // verify PASS -> sets true
    engine.verify_verdict(&mut state, true).expect("verify");
    assert_eq!(state.phase_verify_passed.get("build"), Some(&true));
}

// ---------------------------------------------------------------------------
// Test: step auto-sets verify for gate-less next phase
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-auto-verifies-gateless-next-phase-on-entry
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

// ---------------------------------------------------------------------------
// Test: step without verify returns VerifyRequired
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-requires-verify-before-advancing-gate-phase
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

// ---------------------------------------------------------------------------
// Test: step after verify PASS succeeds
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-advances-through-active-phases-and-completes
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

// ---------------------------------------------------------------------------
// Test: step after verify FAIL returns VerifyRequired
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-requires-verify-before-advancing-gate-phase
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

    engine
        .verify_verdict(&mut state, false)
        .expect("verify FAIL");
    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::VerifyRequired { phase_id }) if phase_id == "build"),
        "expected VerifyRequired after FAIL, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Test: step on gate-less phase succeeds without verify
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-auto-verifies-gateless-next-phase-on-entry
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

// ---------------------------------------------------------------------------
// Test: verify PASS does not carry to next gate phase
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-auto-verifies-gateless-next-phase-on-entry
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
    engine
        .verify_verdict(&mut state, true)
        .expect("verify build");
    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step to test");
    assert_eq!(next.as_deref(), Some("test"));

    // step without verifying test phase
    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::VerifyRequired { phase_id }) if phase_id == "test"),
        "expected VerifyRequired for unverified next phase, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Test: step exceeding max_retries returns error
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-blocks-when-max-retries-exceeded
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
    engine
        .verify_verdict(&mut state, true)
        .expect("verify PASS");

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

// ---------------------------------------------------------------------------
// Test: step within max_retries succeeds
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-blocks-when-max-retries-exceeded
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
    engine
        .verify_verdict(&mut state, true)
        .expect("verify PASS");

    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step should succeed");
    assert_eq!(next.as_deref(), Some("done"));
}

// ---------------------------------------------------------------------------
// Test: step with zero max_retries is unlimited
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-max-retries-zero-is-unlimited
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
    engine
        .verify_verdict(&mut state, true)
        .expect("verify PASS");

    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step should succeed");
    assert_eq!(next.as_deref(), Some("done"));
}

// ---------------------------------------------------------------------------
// Test: verify still works after max_retries exceeded
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-blocks-when-max-retries-exceeded
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
    engine
        .verify_verdict(&mut state, true)
        .expect("verify 3 PASS");

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

// ===========================================================================
// Task 7: Guard evaluation order and RunState persistence
// ===========================================================================

// ---------------------------------------------------------------------------
// Test: VerifyRequired fires before MaxRetriesExceeded when both apply
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-guard-order-verify-then-regate-then-max-retries
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

// ---------------------------------------------------------------------------
// Test: phase_verify_passed persists across load (via verify_verdict)
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-state-round-trips-through-save-load
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

// ===========================================================================
// Task 8: Fixture-based full lifecycle tests
// ===========================================================================

// ---------------------------------------------------------------------------
// Test: Full lifecycle through gate_pipeline (3 gate phases)
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-advances-through-active-phases-and-completes
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
    engine
        .verify_verdict(&mut state, true)
        .expect("verify build");
    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step build->test");
    assert_eq!(next.as_deref(), Some("test"));

    // Phase 2: test
    engine
        .verify_verdict(&mut state, true)
        .expect("verify test");
    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step test->deploy");
    assert_eq!(next.as_deref(), Some("deploy"));

    // Phase 3: deploy
    engine
        .verify_verdict(&mut state, true)
        .expect("verify deploy");
    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step deploy->COMPLETED");
    assert!(next.is_none());
    assert_eq!(state.current_phase, "COMPLETED");
    assert_eq!(state.completed_phases, vec!["build", "test", "deploy"]);
}

// ---------------------------------------------------------------------------
// Test: Lifecycle through gate + confirm mixed pipeline
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-init-auto-sets-verify-for-gateless-first-phase
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
    engine
        .verify_verdict(&mut state, true)
        .expect("verify build");
    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step build->review");
    assert_eq!(next.as_deref(), Some("review"));

    // Phase 2: review (confirm only, no gate -> auto-true)
    assert_eq!(
        state.phase_verify_passed.get("review"),
        Some(&true),
        "confirm-only phase should auto-set verify"
    );
    // step works without explicit verify
    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step review->deploy");
    assert_eq!(next.as_deref(), Some("deploy"));

    // Phase 3: deploy (has gate)
    assert!(
        state.phase_verify_passed.get("deploy").is_none(),
        "gate phase should not auto-set"
    );
    engine
        .verify_verdict(&mut state, true)
        .expect("verify deploy");
    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step deploy->COMPLETED");
    assert!(next.is_none());
    assert_eq!(state.current_phase, "COMPLETED");
}

// ---------------------------------------------------------------------------
// Test: Lifecycle with max_retries recovery (fail then pass within limit)
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-blocks-when-max-retries-exceeded
#[test]
fn lifecycle_max_retries_recovery() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline_path = fixture_path("max_retries_pipeline.yml");
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Phase 1: build (max_retries: 3)
    // FAIL twice, then PASS on attempt 3 (within limit)
    engine
        .verify_verdict(&mut state, false)
        .expect("verify FAIL 1");
    engine
        .verify_verdict(&mut state, false)
        .expect("verify FAIL 2");
    engine
        .verify_verdict(&mut state, true)
        .expect("verify PASS 3");

    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step build->test");
    assert_eq!(next.as_deref(), Some("test"));
    assert_eq!(state.phase_attempts.get("build").copied(), Some(3));

    // Phase 2: test (max_retries: 1)
    // PASS on first attempt
    engine
        .verify_verdict(&mut state, true)
        .expect("verify PASS 1");
    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step test->COMPLETED");
    assert!(next.is_none());
    assert_eq!(state.current_phase, "COMPLETED");
}

// ===========================================================================
// Task 9: Fixture-based compound condition and edge case tests
// ===========================================================================

// ---------------------------------------------------------------------------
// Test: Skipped phase does not pollute verify state
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-skips-phases-whose-when-is-false
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
    engine
        .verify_verdict(&mut state, true)
        .expect("verify build");
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

// ---------------------------------------------------------------------------
// Test: Regate pipeline verify-step works through all phases
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-regate-requires-record-before-step-and-resets-on-reverify
#[test]
fn regate_pipeline_verify_step_works() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline_path = fixture_path("regate_pipeline.yml");
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Phase 1: design
    engine
        .verify_verdict(&mut state, true)
        .expect("verify design");
    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step design->build");
    assert_eq!(next.as_deref(), Some("build"));

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

    // Phase 3: test
    engine
        .verify_verdict(&mut state, true)
        .expect("verify test");
    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step test->COMPLETED");
    assert!(next.is_none());

    // Verify state is intact
    assert_eq!(state.completed_phases, vec!["design", "build", "test"]);
    assert!(state.skipped_phases.is_empty());
}

// ---------------------------------------------------------------------------
// Test: max_retries: 1 triggers immediate escalation after 2 attempts
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-blocks-when-max-retries-exceeded
#[test]
fn max_retries_one_immediate_escalation() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline_path = fixture_path("max_retries_pipeline.yml");
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Advance to "test" phase (max_retries: 1)
    engine
        .verify_verdict(&mut state, true)
        .expect("verify build");
    engine
        .step(&mut state, &pipeline_path)
        .expect("step to test");
    assert_eq!(state.current_phase, "test");

    // 1 FAIL + 1 PASS = 2 attempts; 2 > 1 -> MaxRetriesExceeded
    engine
        .verify_verdict(&mut state, false)
        .expect("verify FAIL");
    engine
        .verify_verdict(&mut state, true)
        .expect("verify PASS");

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

// ---------------------------------------------------------------------------
// Test: Consecutive step without verify always rejected
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-requires-verify-before-advancing-gate-phase
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

// ---------------------------------------------------------------------------
// Test: After max_retries escalation, verify works but step remains rejected
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-blocks-when-max-retries-exceeded
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
        engine
            .verify_verdict(&mut state, false)
            .expect("verify FAIL");
    }
    engine
        .verify_verdict(&mut state, true)
        .expect("verify PASS");

    // step -> MaxRetriesExceeded
    assert!(matches!(
        engine.step(&mut state, &pipeline_path),
        Err(BeltError::MaxRetriesExceeded { .. })
    ));

    // verify still works (attempt 5)
    engine
        .verify_verdict(&mut state, true)
        .expect("verify should still work");
    assert_eq!(state.phase_attempts.get("build").copied(), Some(5));

    // step still rejected (5 > 3)
    assert!(matches!(
        engine.step(&mut state, &pipeline_path),
        Err(BeltError::MaxRetriesExceeded { .. })
    ));
}

// ===========================================================================
// BELT-23: pipeline_file path canonicalization
// ===========================================================================

// ---------------------------------------------------------------------------
// Test: init stores absolute path in state.pipeline_file
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-init-canonicalizes-pipeline-file-path
#[test]
fn init_stores_absolute_path() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    let stored = std::path::Path::new(&state.pipeline_file);
    assert!(
        stored.is_absolute(),
        "pipeline_file should be absolute, got: {}",
        state.pipeline_file
    );
}

// ---------------------------------------------------------------------------
// Test: init canonicalizes dot segments (../ and ./)
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-init-canonicalizes-pipeline-file-path
#[test]
fn init_canonicalizes_dot_segments() {
    let dir = TempDir::new().expect("tempdir");
    let _pipeline_path = two_phase_pipeline(&dir);

    // Construct a path with redundant dot segments:
    // /tmp/xxx/pipeline.yml -> /tmp/xxx/./subdir/../pipeline.yml
    let subdir = dir.path().join("subdir");
    std::fs::create_dir(&subdir).expect("create subdir");
    let dotty_path = subdir.join("..").join("pipeline.yml");

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let state = engine.init(&dotty_path, &HashMap::new()).expect("init");

    assert!(
        !state.pipeline_file.contains(".."),
        "pipeline_file should not contain '..', got: {}",
        state.pipeline_file
    );
    assert!(
        !state.pipeline_file.contains("/./"),
        "pipeline_file should not contain '/./', got: {}",
        state.pipeline_file
    );
    assert!(
        std::path::Path::new(&state.pipeline_file).is_absolute(),
        "pipeline_file should be absolute, got: {}",
        state.pipeline_file
    );
}

// ---------------------------------------------------------------------------
// Test: init with absolute path stores same canonical path
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-init-canonicalizes-pipeline-file-path
#[test]
fn init_with_absolute_path_preserved() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);

    // pipeline_path from TempDir is already absolute
    assert!(pipeline_path.is_absolute());

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Canonicalize the input for comparison (resolves macOS /private/var symlinks)
    let expected = std::fs::canonicalize(&pipeline_path)
        .expect("canonicalize")
        .display()
        .to_string();
    assert_eq!(state.pipeline_file, expected);
}

// ---------------------------------------------------------------------------
// Test: init resolves symlink to real path
// ---------------------------------------------------------------------------
#[cfg(unix)]
/// scenario: belt-core-engine-init-canonicalizes-pipeline-file-path
#[test]
fn init_resolves_symlink() {
    let dir = TempDir::new().expect("tempdir");
    let real_path = two_phase_pipeline(&dir);

    // Create symlink: linked.yml -> pipeline.yml
    let link_path = dir.path().join("linked.yml");
    std::os::unix::fs::symlink(&real_path, &link_path).expect("create symlink");

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let state = engine.init(&link_path, &HashMap::new()).expect("init");

    // Should resolve to the real file, not the symlink
    let canonical_real = std::fs::canonicalize(&real_path)
        .expect("canonicalize")
        .display()
        .to_string();
    assert_eq!(
        state.pipeline_file, canonical_real,
        "symlink should resolve to real path"
    );
    assert!(
        !state.pipeline_file.contains("linked.yml"),
        "should not contain symlink name"
    );
}

// ---------------------------------------------------------------------------
// Test: state.pipeline_file is usable after save/load round-trip
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-state-round-trips-through-save-load
#[test]
fn state_pipeline_file_usable_after_reload() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Reload state from disk
    let loaded = engine.load_state(&state.run_id).expect("load");

    // Use the stored pipeline_file to call next_phase_info (proves path resolves)
    let restored_path = std::path::Path::new(&loaded.pipeline_file);
    let phase = engine
        .next_phase_info(&loaded, restored_path)
        .expect("next_phase_info should work with stored absolute path");
    assert_eq!(phase.id, "build");
}

// ---------------------------------------------------------------------------
// Test: nonexistent path returns parse_pipeline error, not canonicalize error
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-init-error-paths
#[test]
fn init_nonexistent_path_returns_parse_error() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);

    let bogus_path = dir.path().join("does_not_exist.yml");
    let result = engine.init(&bogus_path, &HashMap::new());

    assert!(result.is_err());
    // Should be a FileNotFound or YamlParse from parse_pipeline,
    // NOT an Io error from canonicalize
    let err = result.unwrap_err();
    assert!(
        !matches!(err, BeltError::Io(_)),
        "error should come from parse_pipeline, not canonicalize: {err}"
    );
}

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
    engine.verify_verdict(state, true).expect("verify design");
    engine.step(state, pipeline_path).expect("step to build");
    assert_eq!(state.current_phase, "build");
}

/// scenario: belt-core-engine-regate-requires-record-before-step-and-resets-on-reverify
#[test]
fn regate_init_does_not_set_regate_passed() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    assert!(
        state.regate_passed.is_empty(),
        "init should not set regate_passed"
    );
}

/// scenario: belt-core-engine-regate-requires-record-before-step-and-resets-on-reverify
#[test]
fn regate_record_stores_result() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);
    engine
        .verify_verdict(&mut state, true)
        .expect("verify build");
    engine
        .record_regate(&mut state, true)
        .expect("record_regate");
    assert_eq!(state.regate_passed.get("build"), Some(&true));
}

/// scenario: belt-core-engine-regate-requires-record-before-step-and-resets-on-reverify
#[test]
fn regate_verify_clears_regate_passed() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);
    engine.verify_verdict(&mut state, true).expect("verify");
    engine
        .record_regate(&mut state, true)
        .expect("record_regate");
    assert_eq!(state.regate_passed.get("build"), Some(&true));
    // re-verify clears regate
    engine.verify_verdict(&mut state, true).expect("re-verify");
    assert!(
        state.regate_passed.get("build").is_none(),
        "verify should clear regate_passed"
    );
}

/// scenario: belt-core-engine-state-round-trips-through-save-load
#[test]
fn regate_passed_persists_across_save_load() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);
    engine.verify_verdict(&mut state, true).expect("verify");
    engine
        .record_regate(&mut state, true)
        .expect("record_regate");
    let loaded = engine.load_state(&state.run_id).expect("load");
    assert_eq!(loaded.regate_passed.get("build"), Some(&true));
}

/// scenario: belt-core-engine-regate-requires-record-before-step-and-resets-on-reverify
#[test]
fn regate_step_requires_regate_when_targets_exist() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);
    engine.verify_verdict(&mut state, true).expect("verify");
    // Skip regate — step should fail
    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::RegateRequired { phase_id, targets })
            if phase_id == "build" && targets == &vec!["design".to_string()]),
        "expected RegateRequired, got: {result:?}"
    );
}

/// scenario: belt-core-engine-regate-requires-record-before-step-and-resets-on-reverify
#[test]
fn regate_step_blocked_when_regate_failed() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);
    engine.verify_verdict(&mut state, true).expect("verify");
    engine
        .record_regate(&mut state, false)
        .expect("regate FAIL");
    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::RegateFailed { phase_id, targets })
            if phase_id == "build" && targets == &vec!["design".to_string()]),
        "expected RegateFailed, got: {result:?}"
    );
}

/// scenario: belt-core-engine-regate-requires-record-before-step-and-resets-on-reverify
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

/// scenario: belt-core-engine-regate-requires-record-before-step-and-resets-on-reverify
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

/// scenario: belt-core-engine-guard-order-verify-then-regate-then-max-retries
#[test]
fn regate_verify_guard_priority_over_regate_guard() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);
    // Clear verify state
    state.phase_verify_passed.remove("build");
    engine.save_state(&state).expect("save");
    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::VerifyRequired { .. })),
        "expected VerifyRequired (not RegateRequired), got: {result:?}"
    );
}

/// scenario: belt-core-engine-guard-order-verify-then-regate-then-max-retries
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
    engine
        .verify_verdict(&mut state, true)
        .expect("verify prep");
    engine
        .step(&mut state, &pipeline_path)
        .expect("step to check");
    // Exceed max_retries but don't run regate
    engine.verify_verdict(&mut state, false).expect("v1");
    engine.verify_verdict(&mut state, true).expect("v2");
    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::RegateRequired { .. })),
        "expected RegateRequired (not MaxRetriesExceeded), got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 11: verify -> regate(PASS) -> re-verify -> regate cleared -> step blocked
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-regate-requires-record-before-step-and-resets-on-reverify
#[test]
fn regate_verify_regate_reverify_resets_regate() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);

    engine.verify_verdict(&mut state, true).expect("verify");
    engine.record_regate(&mut state, true).expect("regate PASS");
    // re-verify clears regate
    engine.verify_verdict(&mut state, true).expect("re-verify");
    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::RegateRequired { .. })),
        "re-verify should reset regate, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 12: regate FAIL -> re-verify clears state for fresh retry
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-regate-requires-record-before-step-and-resets-on-reverify
#[test]
fn regate_fail_then_reverify_clears_state() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = regate_pipeline(&dir);
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    advance_to_build(&engine, &mut state, &pipeline_path);

    engine.verify_verdict(&mut state, true).expect("verify");
    engine
        .record_regate(&mut state, false)
        .expect("regate FAIL");
    assert_eq!(state.regate_passed.get("build"), Some(&false));
    // re-verify clears failed regate
    engine.verify_verdict(&mut state, true).expect("re-verify");
    assert!(state.regate_passed.get("build").is_none());
}

// ---------------------------------------------------------------------------
// Test 13: multiple regate targets — partial fail -> all_passed false
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-regate-requires-record-before-step-and-resets-on-reverify
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
    engine
        .record_regate(&mut state, false)
        .expect("regate partial fail");

    let result = engine.step(&mut state, &pipeline_path);
    assert!(
        matches!(&result, Err(BeltError::RegateFailed { .. })),
        "partial regate fail should block step: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 14: record_regate is idempotent
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-regate-requires-record-before-step-and-resets-on-reverify
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
    let next = engine.step(&mut state, &pipeline_path).expect("step");
    assert_eq!(next.as_deref(), Some("test"));
}

// ---------------------------------------------------------------------------
// Test 23: gateless phase with regate targets
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-regate-handles-empty-or-skipped-or-gateless-targets
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
    engine
        .verify_verdict(&mut state, true)
        .expect("verify prep");
    engine
        .step(&mut state, &pipeline_path)
        .expect("step to check");
    assert_eq!(state.current_phase, "check");

    // check is gateless -> auto-verify. But regate still required.
    assert_eq!(state.phase_verify_passed.get("check"), Some(&true));

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
/// scenario: belt-core-engine-regate-handles-empty-or-skipped-or-gateless-targets
#[test]
fn regate_target_skipped_phase_auto_passed() {
    let dir = TempDir::new().expect("tempdir");
    // Structure: start -> optional(when:false) -> main(regate:[optional]) -> done
    // optional is skipped during step from start, so it lands in skipped_phases.
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: skip-regate
version: 1
args:
  run_optional: { type: bool, default: false }
phases:
  - id: start
    description: "Start"
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

    // init starts at "start" (first active phase)
    let mut state = engine.init(&pipeline_path, &HashMap::new()).expect("init");
    assert_eq!(state.current_phase, "start");

    // step from start -> optional is skipped (when: false) -> lands on main
    let next = engine.step(&mut state, &pipeline_path).expect("step");
    assert_eq!(next.as_deref(), Some("main"));
    assert!(state.skipped_phases.contains(&"optional".to_string()));

    engine
        .verify_verdict(&mut state, true)
        .expect("verify main");
    // Simulate cmd_regate auto-passing for skipped target
    engine.record_regate(&mut state, true).expect("regate");

    let next = engine.step(&mut state, &pipeline_path).expect("step");
    assert_eq!(next.as_deref(), Some("done"));
}

// ---------------------------------------------------------------------------
// Test 26: regate target with empty gate -> treated as passed
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-regate-handles-empty-or-skipped-or-gateless-targets
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
    engine
        .step(&mut state, &pipeline_path)
        .expect("step to check");
    assert_eq!(state.current_phase, "check");

    engine
        .verify_verdict(&mut state, true)
        .expect("verify check");
    // empty gate target -> all_passed(&[]) = true
    engine.record_regate(&mut state, true).expect("regate");

    let next = engine.step(&mut state, &pipeline_path).expect("step");
    assert_eq!(next.as_deref(), Some("done"));
}

// ---------------------------------------------------------------------------
// BELT-32 Plan B Task 2: phase_start_times lifecycle
// ---------------------------------------------------------------------------

/// `phase_start_times` is set when `step()` first enters a phase.
/// It is not touched by retries within the same phase.
/// regate does not modify any phase's `phase_start_times`.
/// scenario: belt-core-engine-phase-start-times-set-on-entry-preserved-on-retry-and-persisted
#[test]
fn phase_start_times_is_set_on_entry_not_updated_on_retry() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
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
    );

    let engine = Engine::new(&belt_dir);
    let mut state = engine
        .init(&pipeline_path, &HashMap::new())
        .expect("init should succeed");

    // First phase entry time recorded via init.
    let first_entry = state
        .phase_start_times
        .get("first")
        .copied()
        .expect("first phase should have a start time set on init");

    // Simulate a verify PASS and step to next phase.
    engine
        .verify_verdict(&mut state, true)
        .expect("verify first");
    engine
        .step(&mut state, &pipeline_path)
        .expect("step to second");

    // second should now have a start time; first should be unchanged.
    let second_entry = state
        .phase_start_times
        .get("second")
        .copied()
        .expect("second phase should have a start time after step()");
    let first_after = state
        .phase_start_times
        .get("first")
        .copied()
        .expect("first phase time must remain");
    assert_eq!(
        first_entry, first_after,
        "first phase start time must not change after leaving it"
    );
    assert!(
        second_entry >= first_after,
        "second phase start time must be at or after first's"
    );

    // Retry on second: verify FAIL, verify PASS, each should preserve second's start time.
    engine
        .verify_verdict(&mut state, false)
        .expect("verify fail");
    let second_after_fail = state
        .phase_start_times
        .get("second")
        .copied()
        .expect("second phase time retained on fail");
    assert_eq!(
        second_entry, second_after_fail,
        "retry (verify FAIL) must not update phase_start_times"
    );

    engine
        .verify_verdict(&mut state, true)
        .expect("verify pass");
    let second_after_pass = state
        .phase_start_times
        .get("second")
        .copied()
        .expect("second phase time retained on pass");
    assert_eq!(
        second_entry, second_after_pass,
        "retry (verify PASS) must not update phase_start_times"
    );
}

/// `phase_start_times` uses UTC and serialises round-trip via state.json.
/// scenario: belt-core-engine-phase-start-times-set-on-entry-preserved-on-retry-and-persisted
#[test]
fn phase_start_times_round_trips_through_state_json() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r"
name: t
version: 1
phases:
  - id: only
    description: only phase
",
    );

    let engine = Engine::new(&belt_dir);
    let state = engine
        .init(&pipeline_path, &HashMap::new())
        .expect("init should succeed");

    let written = state
        .phase_start_times
        .get("only")
        .copied()
        .expect("first phase must have start time");

    // Reload from disk.
    let reloaded = engine.load_state(&state.run_id).expect("load_state");
    let read_back: chrono::DateTime<chrono::Utc> = reloaded
        .phase_start_times
        .get("only")
        .copied()
        .expect("phase_start_times must persist");
    assert_eq!(written.to_rfc3339(), read_back.to_rfc3339());
}

// ---------------------------------------------------------------------------
// init_creates_notes_directory
//
// Ensures the run-scoped notes directory (`<run_dir>/notes/`) is created on
// `Engine::init`, so downstream tasks can write narrative Markdown artifacts
// without worrying about directory-creation ordering.
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-init-creates-run-with-first-active-phase
#[test]
fn init_creates_notes_directory() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let pipeline_path = write_yaml(
        &dir,
        "p.yml",
        r#"name: p
version: 1
phases:
  - id: one
    description: "only phase"
"#,
    );

    let engine = Engine::new(&belt_dir);
    let state = engine
        .init(&pipeline_path, &HashMap::new())
        .expect("init should succeed");

    let notes = belt_dir.join("runs").join(&state.run_id).join("notes");
    assert!(notes.is_dir(), "notes dir not created: {}", notes.display());
}

// ---------------------------------------------------------------------------
// step_marks_run_completed_when_no_next_phase
//
// Ensures that when `Engine::step` advances past the last phase, the run's
// `status` transitions from `InProgress` to `Completed`. This is required by
// Task 14's resolver, which filters runs by `status == "completed"` for
// `belt://latest/...` URI resolution.
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-step-advances-through-active-phases-and-completes
#[test]
fn step_marks_run_completed_when_no_next_phase() {
    use belt_core::model::RunStatus;

    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"name: solo
version: 1
phases:
  - id: only
    description: "only phase"
"#,
    );

    let engine = Engine::new(&belt_dir);
    let mut state = engine
        .init(&pipeline_path, &HashMap::new())
        .expect("init should succeed");

    // Simulate verify pass so `step` is not blocked by the verify-before-step
    // guard.
    state.phase_verify_passed.insert("only".into(), true);

    // Sanity: run starts in InProgress.
    assert_eq!(state.status, RunStatus::InProgress);

    let next = engine
        .step(&mut state, &pipeline_path)
        .expect("step should succeed");

    assert!(next.is_none(), "no next phase after the only phase");
    assert_eq!(
        state.status,
        RunStatus::Completed,
        "status must transition to Completed when pipeline finishes"
    );

    // Adversarial probe: the transition must survive serde round-trip so
    // downstream `belt://latest/...` resolvers can filter completed runs by
    // re-loading state.json from disk.
    let reloaded = engine.load_state(&state.run_id).expect("reload");
    assert_eq!(
        reloaded.status,
        RunStatus::Completed,
        "status=Completed must persist to disk via save_state"
    );
}

// ---------------------------------------------------------------------------
// init_records_branch_when_provided
//
// `Engine::init_with_branch` accepts a caller-supplied branch name and stores
// it verbatim in `RunState.branch`. Belt-core remains pure: the caller
// (belt-agent) is responsible for detecting the branch via git; core trusts
// whatever `Option<String>` it receives. This test pins the happy path.
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-init-records-optional-branch-verbatim
#[test]
fn init_records_branch_when_provided() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: t
version: 1
phases:
  - id: only
    description: "only phase"
"#,
    );

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);

    let state = engine
        .init_with_branch(&pipeline_path, &HashMap::new(), Some("develop".to_string()))
        .expect("init_with_branch should succeed");

    assert_eq!(
        state.branch,
        Some("develop".to_string()),
        "branch parameter must be recorded verbatim in RunState"
    );
}

// ---------------------------------------------------------------------------
// init_legacy_records_no_branch
//
// The legacy 2-arg `Engine::init` is preserved for backward compatibility and
// delegates to `init_with_branch(.., None)`. Existing callers (and this test)
// observe `branch: None` in the resulting state, matching the pre-Task-12
// behaviour guarded by Tasks 8/9/10.
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-init-records-optional-branch-verbatim
#[test]
fn init_legacy_records_no_branch() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: t
version: 1
phases:
  - id: only
    description: "only phase"
"#,
    );

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);

    let state = engine
        .init(&pipeline_path, &HashMap::new())
        .expect("init should succeed");

    assert_eq!(
        state.branch, None,
        "legacy init must leave branch unset for backward compatibility"
    );
}

// ---------------------------------------------------------------------------
// next_phase_info_emits_declared_uri_verbatim
//
// After Phase D cleanup, `next_phase_info` no longer rewrites `{run_id}`
// template tokens. It emits `produces[*].path` and string gate fields verbatim
// as declared in the pipeline YAML. For `belt://current/` URIs, resolution is
// deferred to the gate executor (belt-agent's Resolver).
// ---------------------------------------------------------------------------
/// scenario: belt-core-engine-emits-declared-uri-in-next-phase-info
#[test]
fn next_phase_info_emits_declared_uri_verbatim() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"name: t
version: 1
phases:
  - id: p
    description: "phase"
    produces:
      - name: notes
        path: "belt://current/notes/phase-p.md"
    gate:
      - file_exists: "belt://current/notes/phase-p.md"
"#,
    );

    let engine = Engine::new(&belt_dir);
    let state = engine
        .init(&pipeline_path, &HashMap::new())
        .expect("init should succeed");
    let phase = engine
        .next_phase_info(&state, &pipeline_path)
        .expect("next_phase_info should succeed");

    assert_eq!(
        phase.produces[0].path, "belt://current/notes/phase-p.md",
        "produces[*].path must be emitted verbatim (no substitution)"
    );
    match &phase.gate[0] {
        belt_core::model::GateCheck::FileExists { file_exists } => {
            assert_eq!(
                file_exists, "belt://current/notes/phase-p.md",
                "file_exists gate must be emitted verbatim (no substitution)"
            );
        }
        other => panic!("expected FileExists gate, got {other:?}"),
    }
}
