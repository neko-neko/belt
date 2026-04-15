use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

/// Shared chain fixture setup used by context-neutral narrative tests.
///
/// Writes a COMPLETED producer run at `tmp/.belt/runs/<producer_run>/` and a
/// consumer pipeline YAML (`tmp/consumer.yml`) that references the producer
/// via a `belt://run/<id>/...` External URI. Returns the producer run_id so
/// tests can reference it in assertions.
fn setup_chain(tmp: &std::path::Path) -> String {
    let producer_run = "01947a0a-0000-7000-8000-000000000000";
    let producer_dir = tmp.join(".belt/runs").join(producer_run);
    std::fs::create_dir_all(producer_dir.join("notes")).unwrap();
    std::fs::write(producer_dir.join("notes/phase-review.md"), "body").unwrap();
    std::fs::write(
        producer_dir.join("state.json"),
        format!(
            r#"{{
  "run_id": "{producer_run}",
  "pipeline": "feature-dev",
  "pipeline_file": "/tmp/x.yml",
  "version": 1,
  "branch": null,
  "args": {{}},
  "current_phase": "done",
  "completed_phases": ["review", "done"],
  "skipped_phases": [],
  "status": "completed",
  "created_at": "2026-04-14T00:00:00Z",
  "updated_at": "2026-04-14T00:00:00Z"
}}"#
        ),
    )
    .unwrap();

    std::fs::write(
        tmp.join("consumer.yml"),
        r#"name: debug-flow
version: 1
phases:
  - id: rca
    description: "rca"
    consumes:
      - name: prior_review
        uri: "belt://run/01947a0a-0000-7000-8000-000000000000/notes/phase-review.md"
"#,
    )
    .unwrap();

    producer_run.to_string()
}

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
    invoke:
      pipeline: ./pipelines/review-cycle.yml
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

    // status — verify enriched view shows build as completed
    let out = Command::cargo_bin("belt-agent")
        .unwrap()
        .args(["status", "--run", run_id])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["current_phase"], "code-review/review");
    let phases = json["phases"].as_array().expect("phases array");
    let build_phase = phases
        .iter()
        .find(|p| p["id"] == "build")
        .expect("build phase");
    assert_eq!(build_phase["status"], "completed");
}

/// `init` must walk each phase's `consumes:` list, resolve every External
/// `belt://` URI through the resolver, and persist the `uri -> abs_path`
/// mapping into `state.resolved_consumes`. A pre-existing COMPLETED run
/// supplies the resolution target; we then assert that the new consumer
/// run's state.json contains the resolved entry under the original URI key.
#[test]
fn init_resolves_external_uris_and_writes_resolved_consumes() {
    let tmp = tempfile::tempdir().unwrap();
    let producer_run = setup_chain(tmp.path());

    // Init.
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["init", "consumer.yml"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Inspect state.json.
    let runs: Vec<_> = std::fs::read_dir(tmp.path().join(".belt/runs"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().into_string().unwrap()))
        .filter(|n| n != &producer_run)
        .collect();
    assert_eq!(runs.len(), 1, "expected one new run");
    let new_state = std::fs::read_to_string(
        tmp.path()
            .join(".belt/runs")
            .join(&runs[0])
            .join("state.json"),
    )
    .unwrap();
    let v: Value = serde_json::from_str(&new_state).unwrap();
    let rc = v
        .get("resolved_consumes")
        .and_then(|x| x.as_object())
        .unwrap();
    let expected_key = "belt://run/01947a0a-0000-7000-8000-000000000000/notes/phase-review.md";
    assert!(rc.contains_key(expected_key), "rc keys: {:?}", rc.keys());
    let path = rc.get(expected_key).unwrap().as_str().unwrap();
    assert!(
        path.ends_with("notes/phase-review.md"),
        "resolved path: {path}"
    );
}

/// `belt-agent next` must include `uri` and `resolved_path` for each
/// `ArtifactRef::External` entry in the current phase's `consumes`. Skills
/// invoking `next` read `resolved_path` to know which file to load for
/// narrative context. Named and Qualified refs retain their existing JSON
/// shape so downstream consumers are not broken.
#[test]
fn next_json_output_includes_uri_and_resolved_path() {
    let tmp = tempfile::tempdir().unwrap();
    let _producer_run = setup_chain(tmp.path());

    // Init consumer.
    let init_out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["init", "consumer.yml"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        init_out.status.success(),
        "init stderr: {}",
        String::from_utf8_lossy(&init_out.stderr)
    );

    // Run `next` and parse JSON.
    let next_out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["next"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        next_out.status.success(),
        "next stderr: {}",
        String::from_utf8_lossy(&next_out.stderr)
    );
    let json: Value = serde_json::from_slice(&next_out.stdout).unwrap();
    let consumes = json
        .get("phase")
        .and_then(|p| p.get("consumes"))
        .and_then(|x| x.as_array())
        .expect("phase.consumes array");
    assert_eq!(consumes.len(), 1);
    let entry = &consumes[0];
    assert_eq!(
        entry.get("name").and_then(|x| x.as_str()),
        Some("prior_review")
    );
    let uri_val = entry.get("uri").expect("entry missing uri");
    assert_eq!(
        uri_val.as_str(),
        Some("belt://run/01947a0a-0000-7000-8000-000000000000/notes/phase-review.md"),
    );
    let resolved = entry
        .get("resolved_path")
        .and_then(|x| x.as_str())
        .expect("entry missing resolved_path (or null)");
    assert!(
        resolved.ends_with("notes/phase-review.md"),
        "resolved: {resolved}"
    );
}
