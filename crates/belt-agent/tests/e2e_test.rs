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

/// Full chain happy path using the fixtures from Task 22
/// (`chain-producer.yml` → `chain-consumer.yml`).
///
/// Simulates an LLM driving the producer pipeline to COMPLETED (writing the
/// `phase-review.md` narrative along the way), then initialising the consumer
/// pipeline in the same workspace and reading the producer's note content
/// back through the `resolved_path` surfaced by `belt-agent next`. This is
/// the end-to-end wiring that the context-neutral narrative plan exists to
/// deliver: note body written in run A must flow through `belt://latest/...`
/// into run B without the consumer skill knowing the producer's run_id.
#[test]
fn e2e_chain_producer_to_consumer_happy_path() {
    let tmp = tempfile::tempdir().unwrap();

    // Copy fixtures into tmp so pipeline paths resolve relative to cwd.
    let producer_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("belt-core/tests/fixtures/chain-producer.yml");
    let consumer_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("belt-core/tests/fixtures/chain-consumer.yml");
    let producer = tmp.path().join("chain-producer.yml");
    let consumer = tmp.path().join("chain-consumer.yml");
    std::fs::copy(&producer_src, &producer).unwrap();
    std::fs::copy(&consumer_src, &consumer).unwrap();

    // 1. Init producer.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["init", "chain-producer.yml"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "producer init stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 2. Read producer run_id from the newly-created run directory.
    let run_dirs: Vec<_> = std::fs::read_dir(tmp.path().join(".belt/runs"))
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(run_dirs.len(), 1, "expected exactly one producer run");
    let producer_run = run_dirs[0].file_name().into_string().unwrap();

    // 3. Write the review note (simulating the LLM producing the narrative).
    //    The producer's `review` gate requires this file to exist.
    std::fs::write(
        tmp.path()
            .join(format!(".belt/runs/{producer_run}/notes/phase-review.md")),
        "Review narrative body",
    )
    .unwrap();

    // 4. Advance through `review` → `done` → COMPLETED.
    //    `review` gate passes because the file exists.
    //    `done` has no gate (auto-PASS on verify) but declares `confirm: true`
    //    (fixture invariant added in Task 22 so lint accepts the phase), so
    //    both `step` calls must pass `--confirm`.
    for i in 0..2 {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
            .args(["verify"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "verify[{i}] stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let verify_json: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(
            verify_json.get("verdict").and_then(|x| x.as_str()),
            Some("PASS"),
            "verify[{i}] verdict: {verify_json}"
        );

        let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
            .args(["step", "--confirm"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "step[{i}] stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // 5. Confirm producer state is COMPLETED.
    let state_json = std::fs::read_to_string(
        tmp.path()
            .join(format!(".belt/runs/{producer_run}/state.json")),
    )
    .unwrap();
    let v: Value = serde_json::from_str(&state_json).unwrap();
    assert_eq!(
        v.get("status").and_then(|x| x.as_str()),
        Some("completed"),
        "producer should be completed, state.json: {state_json}"
    );

    // 6. Init consumer in the same workspace.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["init", "chain-consumer.yml"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "consumer init stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 7. Run `next` on the consumer run. `phase.consumes[0].resolved_path`
    //    must point at the producer's `notes/phase-review.md`, and the file
    //    body must round-trip exactly what we wrote in step 3.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["next"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "next stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let next_json: Value = serde_json::from_slice(&out.stdout).unwrap();
    let consumes = next_json
        .get("phase")
        .and_then(|p| p.get("consumes"))
        .and_then(|x| x.as_array())
        .expect("phase.consumes array");
    assert_eq!(consumes.len(), 1, "expected one consumes entry");
    let entry = &consumes[0];
    let path = entry
        .get("resolved_path")
        .and_then(|x| x.as_str())
        .expect("resolved_path (or null)");
    assert!(
        path.contains(&producer_run),
        "resolved_path should target producer run '{producer_run}': {path}"
    );
    // `resolved_path` is emitted relative to the agent's working directory
    // (`belt_dir()` returns a relative `.belt` by convention) so we re-root
    // it against `tmp.path()` before reading.
    let path_buf = std::path::Path::new(path);
    let resolved_abs = if path_buf.is_absolute() {
        path_buf.to_path_buf()
    } else {
        tmp.path().join(path_buf)
    };
    let contents = std::fs::read_to_string(&resolved_abs)
        .unwrap_or_else(|e| panic!("read_to_string({}) failed: {e}", resolved_abs.display()));
    assert_eq!(contents, "Review narrative body");
}

/// `belt://latest/<pipeline>/...` resolution must be scoped to the current
/// git branch so narrative does not leak between branches that happen to
/// share a workspace. This test seeds TWO completed producer runs of the
/// same pipeline: one on `main` with an *earlier* UUIDv7 and one on
/// `develop` with a *later* UUIDv7. Lex-max on run_id alone would select
/// the develop run; but the branch filter (Task 14 + Task 17 wiring) must
/// steer the resolver back to the main run because the consumer is init'd
/// on branch `main`.
///
/// Task 11 discovered that `git init -b main` alone yields an ambiguous
/// unborn HEAD — `git rev-parse --abbrev-ref HEAD` either fails or
/// returns the literal `"HEAD"`. Seeding an `--allow-empty` commit with
/// inline identity mirrors the fix from `git::current_branch`'s own
/// tests and keeps the fixture independent of the runner's global git
/// config.
#[test]
fn e2e_branch_isolation_for_latest_uri() {
    let tmp = tempfile::tempdir().unwrap();

    // Producer run on branch=main (completed).
    let main_run = "01947aaa-0000-7000-8000-000000000000";
    let main_dir = tmp.path().join(".belt/runs").join(main_run);
    std::fs::create_dir_all(main_dir.join("notes")).unwrap();
    std::fs::write(main_dir.join("notes/phase-review.md"), "main body").unwrap();
    std::fs::write(
        main_dir.join("state.json"),
        format!(
            r#"{{
  "run_id": "{main_run}",
  "pipeline": "chain-producer",
  "pipeline_file": "/tmp/x.yml",
  "version": 1,
  "branch": "main",
  "args": {{}},
  "current_phase": "done",
  "completed_phases": ["review","done"],
  "skipped_phases": [],
  "status": "completed",
  "created_at": "2026-04-14T00:00:00Z",
  "updated_at": "2026-04-14T00:00:00Z"
}}"#
        ),
    )
    .unwrap();

    // Producer run on branch=develop with lexicographically LATER run_id,
    // should NOT be picked when current_branch == main.
    let dev_run = "01947bbb-0000-7000-8000-000000000000";
    let dev_dir = tmp.path().join(".belt/runs").join(dev_run);
    std::fs::create_dir_all(dev_dir.join("notes")).unwrap();
    std::fs::write(dev_dir.join("notes/phase-review.md"), "develop body").unwrap();
    std::fs::write(
        dev_dir.join("state.json"),
        format!(
            r#"{{
  "run_id": "{dev_run}",
  "pipeline": "chain-producer",
  "pipeline_file": "/tmp/x.yml",
  "version": 1,
  "branch": "develop",
  "args": {{}},
  "current_phase": "done",
  "completed_phases": ["review","done"],
  "skipped_phases": [],
  "status": "completed",
  "created_at": "2026-04-14T00:00:00Z",
  "updated_at": "2026-04-14T00:00:00Z"
}}"#
        ),
    )
    .unwrap();

    // Init a git repo on `main`.
    let init_st = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(tmp.path())
        .status()
        .unwrap();
    assert!(init_st.success());
    // Make an initial commit so HEAD actually points to a real commit on
    // branch 'main' (not ambiguous unborn branch). Without this,
    // `git rev-parse --abbrev-ref HEAD` returns "HEAD" and the branch
    // filter degrades to None, which would mask the test's intent.
    let commit_st = std::process::Command::new("git")
        .args([
            "-c",
            "user.email=belt@test.invalid",
            "-c",
            "user.name=belt",
            "commit",
            "--allow-empty",
            "-m",
            "init",
        ])
        .current_dir(tmp.path())
        .status()
        .unwrap();
    assert!(commit_st.success());

    // Copy consumer fixture.
    let consumer_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("belt-core/tests/fixtures/chain-consumer.yml");
    std::fs::copy(&consumer_src, tmp.path().join("chain-consumer.yml")).unwrap();

    // Init consumer — should pick main_run, not dev_run.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["init", "chain-consumer.yml"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "init stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Find the new run (not main_run, not dev_run).
    let consumer_run = std::fs::read_dir(tmp.path().join(".belt/runs"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().into_string().unwrap()))
        .find(|n| n != main_run && n != dev_run)
        .unwrap();
    let state = std::fs::read_to_string(
        tmp.path()
            .join(format!(".belt/runs/{consumer_run}/state.json")),
    )
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&state).unwrap();
    let rc = v
        .get("resolved_consumes")
        .and_then(|x| x.as_object())
        .unwrap();
    let path = rc.values().next().unwrap().as_str().unwrap();
    assert!(path.contains(main_run), "should pick main run, got: {path}");
}

#[test]
fn e2e_consumer_init_fails_when_no_completed_producer() {
    let tmp = tempfile::tempdir().unwrap();

    // Only an in-progress producer.
    let run = "01947a00-0000-7000-8000-000000000000";
    let dir = tmp.path().join(".belt/runs").join(run);
    std::fs::create_dir_all(dir.join("notes")).unwrap();
    std::fs::write(dir.join("notes/phase-review.md"), "x").unwrap();
    std::fs::write(
        dir.join("state.json"),
        format!(
            r#"{{
  "run_id": "{run}",
  "pipeline": "chain-producer",
  "pipeline_file": "/tmp/x.yml",
  "version": 1,
  "branch": null,
  "args": {{}},
  "current_phase": "review",
  "completed_phases": [],
  "skipped_phases": [],
  "status": "in_progress",
  "created_at": "2026-04-14T00:00:00Z",
  "updated_at": "2026-04-14T00:00:00Z"
}}"#
        ),
    )
    .unwrap();

    let consumer_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("belt-core/tests/fixtures/chain-consumer.yml");
    std::fs::copy(&consumer_src, tmp.path().join("chain-consumer.yml")).unwrap();

    let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["init", "chain-consumer.yml"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(!out.status.success(), "init should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no COMPLETED run"), "stderr: {stderr}");
    // Adversarial probe: also assert the pipeline name appears so we know the
    // failure came from the resolver (not a YAML parse error etc).
    assert!(
        stderr.contains("chain-producer"),
        "expected resolver error about chain-producer; stderr: {stderr}"
    );

    // Atomicity probe (BELT-33): a failed init must NOT leave an orphan
    // consumer run directory behind. The only run that may exist is the
    // pre-seeded in_progress producer; any other directory indicates
    // init materialised state before resolver validation succeeded.
    let stray: Vec<_> = std::fs::read_dir(tmp.path().join(".belt/runs"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().into_string().unwrap()))
        .filter(|n| n != run)
        .collect();
    assert!(
        stray.is_empty(),
        "orphan consumer run left behind after failed init: {stray:?}"
    );
}

/// Regression for BELT-33: a resolver failure during `init` must not
/// leave an orphan `.belt/runs/<id>/` behind. Running `init` twice —
/// once while the producer is missing (fail), once after the producer
/// completes (succeed) — must end up with exactly two run directories:
/// the completed producer plus the successful consumer. A pre-fix
/// cmd_init accumulates a half-initialised consumer run from the first
/// failed call, breaking this invariant.
#[test]
fn e2e_init_succeeds_after_resolver_failure() {
    let tmp = tempfile::tempdir().unwrap();

    // Copy the producer / consumer fixtures into the tempdir so pipeline
    // paths resolve relative to cwd.
    let producer_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("belt-core/tests/fixtures/chain-producer.yml");
    let consumer_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("belt-core/tests/fixtures/chain-consumer.yml");
    std::fs::copy(&producer_src, tmp.path().join("chain-producer.yml")).unwrap();
    std::fs::copy(&consumer_src, tmp.path().join("chain-consumer.yml")).unwrap();

    // 1. Consumer init with no producer — must fail.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["init", "chain-consumer.yml"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "first consumer init should fail (no producer)"
    );

    // 2. `.belt/runs/` must be empty — BELT-33 atomicity.
    let runs_dir = tmp.path().join(".belt/runs");
    let after_fail: Vec<_> = if runs_dir.is_dir() {
        std::fs::read_dir(&runs_dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.file_name().into_string().unwrap()))
            .collect()
    } else {
        Vec::new()
    };
    assert!(
        after_fail.is_empty(),
        "no orphan run should remain after failed init: {after_fail:?}"
    );

    // 3. Run the producer pipeline to completion.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["init", "chain-producer.yml"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "producer init should succeed, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let run_dirs: Vec<_> = std::fs::read_dir(tmp.path().join(".belt/runs"))
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(run_dirs.len(), 1, "exactly one producer run expected");
    let producer_run = run_dirs[0].file_name().into_string().unwrap();
    std::fs::write(
        tmp.path()
            .join(format!(".belt/runs/{producer_run}/notes/phase-review.md")),
        "body",
    )
    .unwrap();
    for _ in 0..2 {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
            .args(["verify"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
            .args(["step", "--confirm"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(out.status.success());
    }

    // 4. Consumer init — must now succeed because the producer is COMPLETED.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["init", "chain-consumer.yml"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "second consumer init should succeed, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 5. Exactly two runs — producer + consumer. No orphan from step 1.
    let final_runs: Vec<_> = std::fs::read_dir(tmp.path().join(".belt/runs"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().into_string().unwrap()))
        .collect();
    assert_eq!(
        final_runs.len(),
        2,
        "expected exactly producer + consumer, got: {final_runs:?}"
    );
}

/// BELT-35 E2E probe: when the only producer has a corrupt state.json,
/// a consumer init fails loudly (non-zero exit, stderr mentions parse
/// failure) AND leaves no orphan consumer run behind. The orphan
/// assertion cross-verifies BELT-33 atomicity under a different
/// resolver-error path (StateParse, not NoCompletedRun).
#[test]
fn e2e_init_fails_when_producer_state_json_is_corrupt() {
    let tmp = tempfile::tempdir().unwrap();

    // Producer run directory with a truncated state.json.
    let run = "01947cor-0000-7000-8000-000000000000";
    let dir = tmp.path().join(".belt/runs").join(run);
    std::fs::create_dir_all(dir.join("notes")).unwrap();
    std::fs::write(dir.join("notes/phase-review.md"), "x").unwrap();
    std::fs::write(dir.join("state.json"), r#"{"run_id": "trun"#).unwrap();

    // Consumer fixture references chain-producer via belt://latest/...
    let consumer_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("belt-core/tests/fixtures/chain-consumer.yml");
    std::fs::copy(&consumer_src, tmp.path().join("chain-consumer.yml")).unwrap();

    let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["init", "chain-consumer.yml"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        !out.status.success(),
        "init should fail on corrupt state.json"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("state.json parse error")
            || stderr.contains("json")
            || stderr.contains("parse"),
        "stderr should mention state.json parse failure: {stderr}"
    );

    // Atomicity cross-check (BELT-33): the failed init must not leave an
    // orphan consumer run. Only the pre-seeded corrupt producer should
    // remain.
    let survivors: Vec<_> = std::fs::read_dir(tmp.path().join(".belt/runs"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().into_string().unwrap()))
        .filter(|n| n != run)
        .collect();
    assert!(
        survivors.is_empty(),
        "orphan consumer run left behind: {survivors:?}"
    );
}
