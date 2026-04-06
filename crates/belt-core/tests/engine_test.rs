use belt_core::engine::Engine;
use belt_core::error::BeltError;
use std::collections::HashMap;
use std::io::Write;
use tempfile::TempDir;

/// Helper: write a file inside the given directory and return its path.
fn write_yaml(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let mut f = std::fs::File::create(&path).expect("failed to create file");
    f.write_all(content.as_bytes())
        .expect("failed to write file");
    path
}

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
// Test: init sets phase_verify_passed for gate-less first phase
// ---------------------------------------------------------------------------
#[test]
fn engine_init_sets_phase_verify_passed_empty() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let args = HashMap::new();

    let state = engine.init(&pipeline_path, &args).expect("init");
    // two_phase_pipeline has gate-less phases, so first phase gets auto-true
    assert_eq!(
        state.phase_verify_passed.get("build"),
        Some(&true),
        "gate-less first phase should auto-set verify true"
    );
}

// ---------------------------------------------------------------------------
// Test: init auto-sets verify for gate-less first phase
// ---------------------------------------------------------------------------
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
#[test]
fn error_verify_required_message() {
    let err = BeltError::VerifyRequired {
        phase_id: "build".to_string(),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("verify required for phase 'build'"),
        "error message should mention phase_id: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Test: MaxRetriesExceeded error variant
// ---------------------------------------------------------------------------
#[test]
fn error_max_retries_exceeded_message() {
    let err = BeltError::MaxRetriesExceeded {
        phase_id: "deploy".to_string(),
        attempts: 3,
        max_retries: 3,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("max retries exceeded for phase 'deploy'"),
        "error message should mention phase_id: {msg}"
    );
    assert!(
        msg.contains("3/3"),
        "error message should show attempts/max_retries: {msg}"
    );
}

// ---------------------------------------------------------------------------
// latest_run_id returns the most recent run
// ---------------------------------------------------------------------------
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
