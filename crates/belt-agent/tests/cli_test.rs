#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use assert_cmd::Command;
use tempfile::TempDir;

fn write_yaml(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, content).expect("write");
    path
}

/// Helper: run belt-agent with args in a dir, return parsed JSON.
fn run_belt_agent(dir: &TempDir, args: &[&str]) -> serde_json::Value {
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(args)
        .current_dir(dir.path())
        .output()
        .unwrap_or_else(|e| panic!("belt-agent {args:?} failed: {e}"));
    assert!(
        output.status.success(),
        "belt-agent {:?} exit non-zero: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("invalid JSON from belt-agent {args:?}: {e}"))
}

#[test]
fn init_produces_valid_json() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build the project"
    gate:
      - cmd: "true"
"#,
    );

    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .arg("init")
        .arg("pipeline.yml")
        .current_dir(dir.path())
        .output()
        .expect("failed to run belt-agent init");

    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("non-utf8 stdout");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON output");

    assert!(v["run_id"].is_string(), "run_id should be a string");
    assert!(
        !v["run_id"].as_str().unwrap().is_empty(),
        "run_id should be non-empty"
    );
    assert_eq!(v["pipeline"], "test");
    assert_eq!(v["phase"]["id"], "build");
    assert_eq!(v["phase"]["description"], "Build the project");
    assert_eq!(v["confirm"], false);
}

#[test]
fn full_flow_init_next_verify_step() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: flow
version: 1
phases:
  - id: build
    description: "Build phase"
    gate:
      - cmd: "true"
  - id: done
    description: "Done phase"
"#,
    );

    // --- init ---
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .arg("init")
        .arg("pipeline.yml")
        .current_dir(dir.path())
        .output()
        .expect("init failed");
    assert!(
        output.status.success(),
        "init: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let init_json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("init: invalid JSON");
    let run_id = init_json["run_id"].as_str().expect("run_id missing");

    // --- verify (expect PASS) ---
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .arg("verify")
        .arg("--run")
        .arg(run_id)
        .current_dir(dir.path())
        .output()
        .expect("verify failed");
    assert!(
        output.status.success(),
        "verify: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let verify_json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("verify: invalid JSON");
    assert_eq!(verify_json["verdict"], "PASS");
    assert_eq!(verify_json["phase"], "build");
    assert!(verify_json["checks"].is_array());

    // --- step (expect advanced to "done") ---
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .arg("step")
        .arg("--run")
        .arg(run_id)
        .current_dir(dir.path())
        .output()
        .expect("step failed");
    assert!(
        output.status.success(),
        "step: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let step_json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("step: invalid JSON");
    assert_eq!(step_json["advanced"], true);
    assert_eq!(step_json["from"], "build");
    assert_eq!(step_json["to"], "done");
}

#[test]
fn step_without_confirm_on_confirm_phase() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: confirm-test
version: 1
phases:
  - id: review
    description: "Review phase"
    confirm: true
  - id: done
    description: "Done phase"
"#,
    );

    // --- init ---
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .arg("init")
        .arg("pipeline.yml")
        .current_dir(dir.path())
        .output()
        .expect("init failed");
    assert!(
        output.status.success(),
        "init: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let init_json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("init: invalid JSON");
    let run_id = init_json["run_id"].as_str().expect("run_id missing");

    // --- step WITHOUT --confirm → should block ---
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .arg("step")
        .arg("--run")
        .arg(run_id)
        .current_dir(dir.path())
        .output()
        .expect("step (no confirm) failed");
    assert!(
        output.status.success(),
        "step no-confirm: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let step_json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("step no-confirm: invalid JSON");
    assert_eq!(step_json["advanced"], false);
    assert_eq!(step_json["reason"], "confirmation_required");
    assert_eq!(step_json["phase"], "review");

    // --- step WITH --confirm → should advance ---
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .arg("step")
        .arg("--run")
        .arg(run_id)
        .arg("--confirm")
        .current_dir(dir.path())
        .output()
        .expect("step (confirm) failed");
    assert!(
        output.status.success(),
        "step confirm: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let step_json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("step confirm: invalid JSON");
    assert_eq!(step_json["advanced"], true);
    assert_eq!(step_json["from"], "review");
    assert_eq!(step_json["to"], "done");
}

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

#[test]
fn init_with_config_resolves_pipeline() {
    let dir = TempDir::new().unwrap();

    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: config-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
"#,
    );

    std::fs::write(dir.path().join("belt.toml"), r#"pipeline = "pipeline.yml""#).unwrap();

    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .arg("--config")
        .arg(dir.path().join("belt.toml").to_str().unwrap())
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("failed to run belt-agent init with --config");

    assert!(
        output.status.success(),
        "init with --config failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("invalid JSON output");
    assert_eq!(v["pipeline"], "config-test");
    assert_eq!(v["phase"]["id"], "build");
}

#[test]
fn init_config_and_positional_file_errors() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("belt.toml"), r#"pipeline = "pipeline.yml""#).unwrap();

    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .arg("--config")
        .arg(dir.path().join("belt.toml").to_str().unwrap())
        .arg("init")
        .arg("pipeline.yml")
        .current_dir(dir.path())
        .output()
        .expect("failed to run");

    assert!(
        !output.status.success(),
        "should fail when both --config and positional are provided"
    );
}

#[test]
fn init_config_nonexistent_file_errors() {
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .arg("--config")
        .arg("/nonexistent/belt.toml")
        .arg("init")
        .output()
        .expect("failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("file not found"),
        "stderr should mention file not found: {stderr}"
    );
}

#[test]
fn init_config_invalid_toml_errors() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("belt.toml"), "not valid [[[").unwrap();

    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .arg("--config")
        .arg(dir.path().join("belt.toml").to_str().unwrap())
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("config parse error") || stderr.contains("parse"),
        "stderr should mention config parse: {stderr}"
    );
}

// ===========================================================================
// BELT-24: regate command tests
// ===========================================================================

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

    std::fs::write(dir.path().join("design.ok"), "").unwrap();
    std::fs::write(dir.path().join("build.ok"), "").unwrap();

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify", "--run", run_id]);
    run_belt_agent(&dir, &["step", "--run", run_id]);
    run_belt_agent(&dir, &["verify", "--run", run_id]);

    // Remove design.ok -> regate fails
    std::fs::remove_file(dir.path().join("design.ok")).unwrap();

    let regate = run_belt_agent(&dir, &["regate", "--run", run_id]);
    assert_eq!(regate["all_passed"], false);
    assert_eq!(regate["targets"]["design"]["passed"], false);
}

#[test]
fn regate_resolves_uri_in_target_gate() {
    // Two-phase pipeline: phase `design` produces a notes file via
    // `belt://current/` URI and gates on it; phase `build` regates on
    // `design`. After writing the note, completing `design`, and stepping
    // into `build`, regate must resolve the URI and PASS.
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: regate-uri
version: 1
phases:
  - id: design
    description: "Design"
    produces:
      - name: notes
        path: "belt://current/notes/phase-design.md"
    gate:
      - file_exists: "belt://current/notes/phase-design.md"
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
    regate: [design]
  - id: done
    description: "Done"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap().to_string();

    let notes_dir = dir
        .path()
        .join(".belt")
        .join("runs")
        .join(&run_id)
        .join("notes");
    std::fs::create_dir_all(&notes_dir).unwrap();
    std::fs::write(notes_dir.join("phase-design.md"), "").unwrap();

    run_belt_agent(&dir, &["verify", "--run", &run_id]);
    run_belt_agent(&dir, &["step", "--run", &run_id]);
    run_belt_agent(&dir, &["verify", "--run", &run_id]);

    let regate = run_belt_agent(&dir, &["regate", "--run", &run_id]);
    assert_eq!(
        regate["all_passed"], true,
        "regate must pass when the belt://current URI resolves to an existing file"
    );
    assert_eq!(regate["targets"]["design"]["passed"], true);
}

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
    assert!(
        step["regate_targets"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("design"))
    );
}

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

    // Advance to build without verify
    run_belt_agent(&dir, &["verify", "--run", run_id]);
    run_belt_agent(&dir, &["step", "--run", run_id]);

    // regate without verify on build -> error
    let regate = run_belt_agent(&dir, &["regate", "--run", run_id]);
    assert_eq!(regate["error"], "verify_not_passed");
}

// ===========================================================================
// BELT-24: lifecycle + edge case tests
// ===========================================================================

// Test 20: full lifecycle with regate
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

    // done: gateless -> step -> COMPLETED
    let step = run_belt_agent(&dir, &["step", "--run", run_id]);
    assert_eq!(step["completed"], true);
}

// Test 21: regate fail -> fix -> retry lifecycle
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

// Test 22: regate loop exhausts max_retries -> escalation
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

    // 3 verify cycles (exceeds max_retries: 2)
    for _ in 0..3 {
        run_belt_agent(&dir, &["verify", "--run", run_id]);
        run_belt_agent(&dir, &["regate", "--run", run_id]);
    }

    // step -> 3 attempts > max_retries 2 -> escalation
    let step = run_belt_agent(&dir, &["step", "--run", run_id]);
    assert_eq!(step["advanced"], false);
    assert_eq!(step["reason"], "max_retries_exceeded");
    assert_eq!(step["escalation"], true);
}

// Test 24: regate target not found (bypasses lint)
#[test]
fn regate_target_not_found_returns_error() {
    let dir = TempDir::new().unwrap();
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

    // init may fail if lint catches it — that's OK
    let init_output = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["init", "pipeline.yml"])
        .current_dir(dir.path())
        .output()
        .expect("init");

    if !init_output.status.success() {
        // lint caught it — acceptable behavior
        return;
    }

    let init_json: serde_json::Value = serde_json::from_slice(&init_output.stdout).unwrap();
    let run_id = init_json["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify", "--run", run_id]);

    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["regate", "--run", run_id])
        .current_dir(dir.path())
        .output()
        .expect("regate");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stderr.contains("not found") || stdout.contains("not found") || !output.status.success(),
        "expected error for nonexistent regate target"
    );
}

// Test 28: regate on completed pipeline
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

    // Gateless -> auto-verify -> step -> COMPLETED
    run_belt_agent(&dir, &["step", "--run", run_id]);

    // regate on completed pipeline
    let regate = run_belt_agent(&dir, &["regate", "--run", run_id]);
    assert!(
        regate.get("error").is_some(),
        "expected error for regate on completed pipeline, got: {regate}"
    );
}

// ===========================================================================
// BELT-29: enriched status view tests
// ===========================================================================

#[test]
fn status_after_init_returns_enriched_view() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: status-e2e
version: 1
phases:
  - id: build
    description: "Build"
  - id: test
    description: "Test"
  - id: deploy
    description: "Deploy"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().expect("run_id");

    let status = run_belt_agent(&dir, &["status", "--run", run_id]);

    assert_eq!(status["status"], "in_progress");
    assert_eq!(status["current_phase"], "build");
    assert_eq!(status["pipeline"], "status-e2e");
    assert_eq!(status["progress"]["completed"], 0);
    assert_eq!(status["progress"]["total"], 3);
    assert_eq!(status["progress"]["remaining"], 3);

    let phases = status["phases"].as_array().expect("phases array");
    assert_eq!(phases.len(), 3);
    assert_eq!(phases[0]["id"], "build");
    assert_eq!(phases[0]["status"], "current");
    assert_eq!(phases[1]["id"], "test");
    assert_eq!(phases[1]["status"], "pending");
    assert_eq!(phases[2]["id"], "deploy");
    assert_eq!(phases[2]["status"], "pending");
}

#[test]
fn status_after_verify_reflects_result() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: verify-status
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
    let run_id = init["run_id"].as_str().expect("run_id");

    run_belt_agent(&dir, &["verify", "--run", run_id]);
    let status = run_belt_agent(&dir, &["status", "--run", run_id]);

    let phases = status["phases"].as_array().expect("phases");
    assert_eq!(phases[0]["verify_passed"], true);
    assert_eq!(phases[0]["attempt"], 1);
}

#[test]
fn status_after_step_shows_progression() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: step-status
version: 1
phases:
  - id: build
    description: "Build"
  - id: test
    description: "Test"
  - id: deploy
    description: "Deploy"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().expect("run_id");

    // build is gate-less -> auto verify_passed
    run_belt_agent(&dir, &["step", "--run", run_id]);
    let status = run_belt_agent(&dir, &["status", "--run", run_id]);

    assert_eq!(status["current_phase"], "test");
    assert_eq!(status["progress"]["completed"], 1);

    let phases = status["phases"].as_array().expect("phases");
    assert_eq!(phases[0]["status"], "completed");
    assert_eq!(phases[1]["status"], "current");
    assert_eq!(phases[2]["status"], "pending");
}

#[test]
fn status_after_completion_shows_null_current() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: complete-status
version: 1
phases:
  - id: only
    description: "Only phase"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().expect("run_id");

    run_belt_agent(&dir, &["step", "--run", run_id]);
    let status = run_belt_agent(&dir, &["status", "--run", run_id]);

    assert_eq!(status["status"], "completed");
    assert!(status["current_phase"].is_null());
    assert_eq!(status["progress"]["completed"], 1);
    assert_eq!(status["progress"]["remaining"], 0);
}

#[test]
fn status_run_flag_selects_correct_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: multi-run
version: 1
phases:
  - id: build
    description: "Build"
  - id: test
    description: "Test"
"#,
    );

    let init1 = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run1 = init1["run_id"].as_str().expect("run_id").to_string();

    run_belt_agent(&dir, &["step", "--run", &run1]);

    let init2 = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run2 = init2["run_id"].as_str().expect("run_id").to_string();

    let status1 = run_belt_agent(&dir, &["status", "--run", &run1]);
    assert_eq!(status1["run_id"], run1);
    assert_eq!(status1["current_phase"], "test");

    let status2 = run_belt_agent(&dir, &["status", "--run", &run2]);
    assert_eq!(status2["run_id"], run2);
    assert_eq!(status2["current_phase"], "build");
}

/// verify outputs `timed_out` field in JSON for passing cmd.
#[test]
fn verify_outputs_timed_out_field() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: timeout-test
version: 1
phases:
  - id: fast
    description: "Fast check"
    gate:
      - cmd: "true"
        timeout: 5
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    assert!(init["run_id"].is_string());

    let verify = run_belt_agent(&dir, &["verify"]);
    assert_eq!(verify["verdict"], "PASS");
    assert_eq!(verify["checks"][0]["timed_out"], false);
}

/// verify returns FAIL with `timed_out` = true for hanging cmd.
#[test]
fn verify_timeout_returns_fail() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: timeout-test
version: 1
phases:
  - id: hang
    description: "Hanging check"
    gate:
      - cmd: "sleep 60"
        timeout: 1
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    assert!(init["run_id"].is_string());

    let verify = run_belt_agent(&dir, &["verify"]);
    assert_eq!(verify["verdict"], "FAIL");
    assert_eq!(verify["checks"][0]["timed_out"], true);
    assert!(
        verify["checks"][0]["detail"]
            .as_str()
            .unwrap_or("")
            .contains("timed out"),
        "detail: {}",
        verify["checks"][0]["detail"]
    );
}

// ===========================================================================
// BELT-30: verify file persistence tests
// ===========================================================================

/// verify writes per-check result file.
#[test]
fn verify_writes_result_file() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: persist-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify"]);

    let verify_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("verify/build.json");
    assert!(verify_file.exists(), "verify file should exist");

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&verify_file).unwrap()).unwrap();
    assert_eq!(content["phase"], "build");
    assert_eq!(content["verdict"], "PASS");
    assert!(content["checks"].is_array());
    assert!(content["timestamp"].is_string());
}

/// verify FAIL writes result file with failure details.
#[test]
fn verify_fail_writes_result_file() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: persist-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "false"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify"]);

    let verify_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("verify/build.json");
    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&verify_file).unwrap()).unwrap();
    assert_eq!(content["verdict"], "FAIL");
    assert!(content["checks"][0]["detail"].is_string());
}

/// verify overwrites result file on retry.
#[test]
fn verify_overwrites_on_retry() {
    let dir = TempDir::new().unwrap();
    let marker = dir.path().join("marker.txt");
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: persist-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "marker.txt"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    // First verify: FAIL
    run_belt_agent(&dir, &["verify"]);
    let verify_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("verify/build.json");
    let c1: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&verify_file).unwrap()).unwrap();
    assert_eq!(c1["verdict"], "FAIL");

    // Fix: create marker
    std::fs::write(&marker, "ok").unwrap();

    // Second verify: PASS
    run_belt_agent(&dir, &["verify"]);
    let c2: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&verify_file).unwrap()).unwrap();
    assert_eq!(c2["verdict"], "PASS");
    assert!(
        c2["attempt"].as_u64().unwrap() > c1["attempt"].as_u64().unwrap(),
        "attempt should increment"
    );
}

/// verify file includes timestamp field.
#[test]
fn verify_file_has_timestamp() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: ts-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify"]);

    let verify_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("verify/build.json");
    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&verify_file).unwrap()).unwrap();
    let ts = content["timestamp"].as_str().unwrap();
    assert!(ts.contains('T') && ts.ends_with('Z'), "bad timestamp: {ts}");
}

/// verify result file includes `timed_out` field from BELT-31.
#[test]
fn verify_result_file_includes_timed_out() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: timeout-persist
version: 1
phases:
  - id: hang
    description: "Hang"
    gate:
      - cmd: "sleep 60"
        timeout: 1
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify"]);

    let verify_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("verify/hang.json");
    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&verify_file).unwrap()).unwrap();
    assert_eq!(content["checks"][0]["timed_out"], true);
}

/// regate writes result file with targets.
#[test]
fn regate_writes_result_file() {
    let dir = TempDir::new().unwrap();
    let design_marker = dir.path().join("design.ok");
    std::fs::write(&design_marker, "ok").unwrap();

    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: regate-persist
version: 1
phases:
  - id: design
    description: "Design"
    gate:
      - file_exists: "design.ok"
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
    regate: [design]
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    // Advance past design
    run_belt_agent(&dir, &["verify"]);
    run_belt_agent(&dir, &["step"]);

    // Now at build: verify then regate
    run_belt_agent(&dir, &["verify"]);
    run_belt_agent(&dir, &["regate"]);

    let regate_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("regate/build.json");
    assert!(regate_file.exists(), "regate file should exist");

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&regate_file).unwrap()).unwrap();
    assert_eq!(content["phase"], "build");
    assert!(content["targets"].is_object());
    assert!(content["timestamp"].is_string());
}

/// regate with no targets writes result file.
#[test]
fn regate_no_targets_writes_file() {
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
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify"]);
    run_belt_agent(&dir, &["regate"]);

    let regate_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("regate/build.json");
    assert!(
        regate_file.exists(),
        "regate file should exist even with no targets"
    );

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&regate_file).unwrap()).unwrap();
    assert_eq!(content["all_passed"], true);
}

/// regate failure writes result file.
#[test]
fn regate_fail_writes_result_file() {
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
      - cmd: "true"
    regate: [design]
"#,
    );

    // Create marker for init, then remove before regate
    let design_marker = dir.path().join("design.ok");
    std::fs::write(&design_marker, "ok").unwrap();

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify"]);
    run_belt_agent(&dir, &["step"]);

    // At build: verify passes
    run_belt_agent(&dir, &["verify"]);

    // Remove design marker so regate fails
    std::fs::remove_file(&design_marker).unwrap();
    run_belt_agent(&dir, &["regate"]);

    let regate_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("regate/build.json");
    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&regate_file).unwrap()).unwrap();
    assert_eq!(content["all_passed"], false);
}

// ===========================================================================
// BELT-30: status with verify_checks / regate_checks
// ===========================================================================

/// status shows `verify_checks` after verify.
#[test]
fn status_shows_verify_checks_after_verify() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: status-checks
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
"#,
    );

    run_belt_agent(&dir, &["init", "pipeline.yml"]);
    run_belt_agent(&dir, &["verify"]);
    let status = run_belt_agent(&dir, &["status"]);

    let checks = &status["phases"][0]["verify_checks"];
    assert!(
        checks.is_array(),
        "verify_checks should be array, got: {checks}"
    );
    assert_eq!(checks[0]["check_type"], "cmd");
    assert_eq!(checks[0]["passed"], true);
}

/// status shows `regate_checks` after regate.
#[test]
fn status_shows_regate_checks_after_regate() {
    let dir = TempDir::new().unwrap();
    let design_marker = dir.path().join("design.ok");
    std::fs::write(&design_marker, "ok").unwrap();

    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: status-regate
version: 1
phases:
  - id: design
    description: "Design"
    gate:
      - file_exists: "design.ok"
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
    regate: [design]
"#,
    );

    run_belt_agent(&dir, &["init", "pipeline.yml"]);
    run_belt_agent(&dir, &["verify"]);
    run_belt_agent(&dir, &["step"]);
    run_belt_agent(&dir, &["verify"]);
    run_belt_agent(&dir, &["regate"]);

    let status = run_belt_agent(&dir, &["status"]);

    let regate = &status["phases"][1]["regate_checks"];
    assert!(
        regate.is_object(),
        "regate_checks should be object, got: {regate}"
    );
    assert!(regate["design"]["passed"].as_bool().unwrap());
}

/// verify writes result file with sanitized phase ID for sub-pipeline phases.
#[test]
fn verify_sub_pipeline_phase_sanitized() {
    let dir = TempDir::new().unwrap();

    // Sub-pipeline
    std::fs::create_dir_all(dir.path().join("pipelines")).unwrap();
    std::fs::write(
        dir.path().join("pipelines/review.yml"),
        r#"
name: review
version: 1
phases:
  - id: check
    description: "Run checks"
    gate:
      - cmd: "true"
"#,
    )
    .unwrap();

    write_yaml(
        &dir,
        "pipeline.yml",
        r"
name: sub-test
version: 1
phases:
  - id: review
    invoke:
      pipeline: ./pipelines/review.yml
",
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify"]);

    // Sub-pipeline phase "review/check" should produce "review_check.json"
    let verify_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("verify/review_check.json");
    assert!(
        verify_file.exists(),
        "verify file should use sanitized name: review_check.json"
    );

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&verify_file).unwrap()).unwrap();
    assert_eq!(content["phase"], "review/check");
    assert_eq!(content["verdict"], "PASS");
}

/// verify still succeeds when file write fails (non-fatal).
#[test]
fn verify_file_write_failure_non_fatal() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: write-fail
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    // Make runs dir read-only to prevent verify file write
    let runs_dir = dir.path().join(".belt/runs").join(run_id);
    std::fs::set_permissions(&runs_dir, std::fs::Permissions::from_mode(0o555)).unwrap();

    // verify should still succeed (file write is non-fatal)
    let result = run_belt_agent(&dir, &["verify"]);
    assert_eq!(result["verdict"], "PASS");

    // Restore permissions for cleanup
    std::fs::set_permissions(&runs_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// End-to-end walk through the migrated feature-dev pipeline using the
/// real plugins/belt/skills/feature-dev tree. This test is not meant to simulate
/// LLM behavior; it only drives belt-agent through init → next to prove that
/// the new-format pipeline boots and surfaces the first phase correctly.
#[test]
fn feature_dev_migrated_pipeline_boots() {
    use std::path::PathBuf;

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let workspace = PathBuf::from(&manifest_dir).join("..").join("..");
    let pipeline = workspace
        .join("plugins")
        .join("belt")
        .join("skills")
        .join("feature-dev")
        .join("pipeline.yml");
    assert!(pipeline.exists(), "feature-dev pipeline must exist");

    // Use a scratch cwd so the default `.belt/` is created there and we don't
    // touch the developer's state.
    let scratch = tempfile::tempdir().expect("tempdir");

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
        ])
        .current_dir(scratch.path())
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
        .args(["next"])
        .current_dir(scratch.path())
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

#[test]
fn init_with_inherits_from_missing_errors() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("p.yml"),
        r#"name: p
version: 1
phases:
  - id: only
    description: "only"
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["init", "p.yml", "--inherits-from", "01947deadbeef"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("run not found"), "stderr: {stderr}");
}
