use assert_cmd::Command;
use tempfile::TempDir;

/// Test the full lifecycle with a pipeline that uses sub-pipelines.
#[test]
fn e2e_sub_pipeline_expansion() {
    let dir = TempDir::new().unwrap();

    // Sub-pipeline
    std::fs::create_dir_all(dir.path().join("pipelines")).unwrap();
    std::fs::write(
        dir.path().join("pipelines/review-cycle.yml"),
        r#"
name: review-cycle
version: 1
inputs:
  skill:
    type: string
    required: true
phases:
  - id: review
    description: "Dispatch review"
    gate:
      - has_output: true
  - id: triage
    description: "Triage findings"
    confirm: true
  - id: fix
    description: "Fix findings"
    max_retries: 3
"#,
    )
    .unwrap();

    // Main pipeline
    std::fs::write(
        dir.path().join("pipeline.yml"),
        r#"
name: e2e-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
  - id: code-review
    uses: ./pipelines/review-cycle.yml
    with:
      skill: "/code-review"
    regate:
      - build
"#,
    )
    .unwrap();

    // init
    let out = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["init", "./pipeline.yml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["phase"]["id"], "build");

    let run_id = json["run_id"].as_str().unwrap();

    // verify build (gate is `true`, should pass)
    let out = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["verify", "--run", run_id])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["verdict"], "PASS");

    // step → code-review/review (first sub-phase)
    let out = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["step", "--run", run_id])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["advanced"], true);
    assert_eq!(json["to"], "code-review/review");

    // next → should show code-review/review
    let out = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["next", "--run", run_id])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["phase"]["id"], "code-review/review");

    // status — verify completed_phases includes "build"
    let out = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["status", "--run", run_id])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["current_phase"], "code-review/review");
    assert!(
        json["completed_phases"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("build"))
    );
}
