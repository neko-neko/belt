use assert_cmd::Command;
use tempfile::TempDir;

fn write_yaml(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, content).expect("write");
    path
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
