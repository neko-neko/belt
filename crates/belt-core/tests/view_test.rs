#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::similar_names,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use belt_core::engine::Engine;
use belt_core::model::{Artifact, Invoker, RunState, RunStatus};
use belt_core::view::{PhaseMetadata, PhaseState, PipelineStatus, build_status_view};
use filetime::{FileTime, set_file_mtime};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn make_state(current: &str, completed: &[&str], skipped: &[&str]) -> RunState {
    RunState {
        run_id: "test-run".to_string(),
        pipeline: "test".to_string(),
        pipeline_file: "/tmp/pipeline.yml".to_string(),
        version: 1,
        branch: None,
        resolved_consumes: HashMap::new(),
        args: HashMap::new(),
        current_phase: current.to_string(),
        completed_phases: completed.iter().map(|s| (*s).to_string()).collect(),
        skipped_phases: skipped.iter().map(|s| (*s).to_string()).collect(),
        phase_attempts: HashMap::new(),
        phase_verify_passed: HashMap::new(),
        regate_passed: HashMap::new(),
        phase_start_times: HashMap::new(),
        status: RunStatus::default(),
        created_at: "2026-04-07T00:00:00Z".to_string(),
        updated_at: "2026-04-07T00:00:00Z".to_string(),
    }
}

fn phase_ids(ids: &[&str]) -> Vec<PhaseMetadata> {
    ids.iter()
        .map(|s| PhaseMetadata {
            id: (*s).to_string(),
            invoke: None,
            produces: Vec::new(),
            consumes: Vec::new(),
        })
        .collect()
}

/// scenario: belt-core-view-phase-state-classified-by-membership
#[test]
fn initial_state_first_current_rest_pending() {
    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build", "test", "deploy"]);
    let tmp = tempfile::tempdir().unwrap();

    let view = build_status_view(&state, &ids, tmp.path());

    assert_eq!(view.phases.len(), 3);
    assert_eq!(view.phases[0].id, "build");
    assert_eq!(view.phases[0].status, PhaseState::Current);
    assert_eq!(view.phases[1].id, "test");
    assert_eq!(view.phases[1].status, PhaseState::Pending);
    assert_eq!(view.phases[2].id, "deploy");
    assert_eq!(view.phases[2].status, PhaseState::Pending);
    assert_eq!(view.status, PipelineStatus::InProgress);
    assert_eq!(view.current_phase, Some("build".to_string()));
}

/// scenario: belt-core-view-phase-state-classified-by-membership
#[test]
fn partially_completed() {
    let state = make_state("deploy", &["build", "test"], &[]);
    let ids = phase_ids(&["build", "test", "deploy"]);
    let tmp = tempfile::tempdir().unwrap();

    let view = build_status_view(&state, &ids, tmp.path());

    assert_eq!(view.phases[0].status, PhaseState::Completed);
    assert_eq!(view.phases[1].status, PhaseState::Completed);
    assert_eq!(view.phases[2].status, PhaseState::Current);
    assert_eq!(view.status, PipelineStatus::InProgress);
    assert_eq!(view.current_phase, Some("deploy".to_string()));
}

/// scenario: belt-core-view-completed-sentinel-clears-current-phase
#[test]
fn fully_completed() {
    let state = make_state("COMPLETED", &["build", "test", "deploy"], &[]);
    let ids = phase_ids(&["build", "test", "deploy"]);
    let tmp = tempfile::tempdir().unwrap();

    let view = build_status_view(&state, &ids, tmp.path());

    assert_eq!(view.status, PipelineStatus::Completed);
    assert_eq!(view.current_phase, None);
    for phase in &view.phases {
        assert_eq!(
            phase.status,
            PhaseState::Completed,
            "phase {} should be Completed",
            phase.id
        );
    }
}

/// scenario: belt-core-view-phase-state-classified-by-membership
#[test]
fn skipped_phases() {
    let state = make_state("deploy", &["build"], &["test"]);
    let ids = phase_ids(&["build", "test", "deploy"]);
    let tmp = tempfile::tempdir().unwrap();

    let view = build_status_view(&state, &ids, tmp.path());

    assert_eq!(view.phases[0].status, PhaseState::Completed);
    assert_eq!(view.phases[1].status, PhaseState::Skipped);
    assert_eq!(view.phases[2].status, PhaseState::Current);
}

/// scenario: belt-core-view-completed-sentinel-clears-current-phase
#[test]
fn mixed_skip_and_completed() {
    let state = make_state("COMPLETED", &["build", "deploy"], &["test", "lint"]);
    let ids = phase_ids(&["build", "test", "lint", "deploy"]);
    let tmp = tempfile::tempdir().unwrap();

    let view = build_status_view(&state, &ids, tmp.path());

    assert_eq!(view.phases[0].status, PhaseState::Completed);
    assert_eq!(view.phases[1].status, PhaseState::Skipped);
    assert_eq!(view.phases[2].status, PhaseState::Skipped);
    assert_eq!(view.phases[3].status, PhaseState::Completed);
    assert_eq!(view.status, PipelineStatus::Completed);
}

/// scenario: belt-core-view-progress-counts-sum-to-total
#[test]
fn progress_normal() {
    let state = make_state("phase4", &["phase1", "phase2"], &["phase3"]);
    let ids = phase_ids(&["phase1", "phase2", "phase3", "phase4", "phase5"]);
    let tmp = tempfile::tempdir().unwrap();

    let view = build_status_view(&state, &ids, tmp.path());

    assert_eq!(view.progress.completed, 2);
    assert_eq!(view.progress.skipped, 1);
    assert_eq!(view.progress.remaining, 2);
    assert_eq!(view.progress.total, 5);
}

/// scenario: belt-core-view-progress-counts-sum-to-total
#[test]
fn progress_all_complete() {
    let state = make_state("COMPLETED", &["a", "b", "c"], &[]);
    let ids = phase_ids(&["a", "b", "c"]);
    let tmp = tempfile::tempdir().unwrap();

    let view = build_status_view(&state, &ids, tmp.path());

    assert_eq!(view.progress.completed, 3);
    assert_eq!(view.progress.skipped, 0);
    assert_eq!(view.progress.remaining, 0);
    assert_eq!(view.progress.total, 3);
}

/// scenario: belt-core-view-progress-counts-sum-to-total
#[test]
fn progress_all_pending() {
    let state = make_state("a", &[], &[]);
    let ids = phase_ids(&["a", "b", "c"]);
    let tmp = tempfile::tempdir().unwrap();

    let view = build_status_view(&state, &ids, tmp.path());

    assert_eq!(view.progress.completed, 0);
    assert_eq!(view.progress.skipped, 0);
    assert_eq!(view.progress.remaining, 3);
    assert_eq!(view.progress.total, 3);
}

/// scenario: belt-core-view-progress-counts-sum-to-total
#[test]
fn single_phase_pipeline() {
    let state = make_state("only", &[], &[]);
    let ids = phase_ids(&["only"]);
    let tmp = tempfile::tempdir().unwrap();

    let view = build_status_view(&state, &ids, tmp.path());

    assert_eq!(view.phases.len(), 1);
    assert_eq!(view.phases[0].status, PhaseState::Current);
    assert_eq!(view.progress.total, 1);
    assert_eq!(view.progress.remaining, 1);
}

/// scenario: belt-core-view-metadata-fields-propagated-verbatim
#[test]
fn metadata_fields_propagated() {
    let mut state = make_state("build", &[], &[]);
    state.run_id = "run-abc-123".to_string();
    state.pipeline = "my-pipeline".to_string();
    state.pipeline_file = "/home/user/pipeline.yml".to_string();
    state.version = 3;
    state
        .args
        .insert("verbose".to_string(), serde_json::Value::Bool(true));
    state.created_at = "2026-04-07T10:00:00Z".to_string();
    state.updated_at = "2026-04-07T11:00:00Z".to_string();
    let ids = phase_ids(&["build"]);
    let tmp = tempfile::tempdir().unwrap();

    let view = build_status_view(&state, &ids, tmp.path());

    assert_eq!(view.run_id, "run-abc-123");
    assert_eq!(view.pipeline, "my-pipeline");
    assert_eq!(view.pipeline_file, "/home/user/pipeline.yml");
    assert_eq!(view.version, 3);
    assert_eq!(
        view.args.get("verbose"),
        Some(&serde_json::Value::Bool(true))
    );
    assert_eq!(view.created_at, "2026-04-07T10:00:00Z");
    assert_eq!(view.updated_at, "2026-04-07T11:00:00Z");
}

// --- Task 3: Verify / regate / attempt state tests ---

/// scenario: belt-core-view-verify-passed-tri-state-reflects-record
#[test]
fn verify_not_run_is_none() {
    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());
    assert!(view.phases[0].verify_passed.is_none());
}

/// scenario: belt-core-view-verify-passed-tri-state-reflects-record
#[test]
fn verify_pass_reflected() {
    let mut state = make_state("build", &[], &[]);
    state.phase_verify_passed.insert("build".to_string(), true);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());
    assert_eq!(view.phases[0].verify_passed, Some(true));
}

/// scenario: belt-core-view-verify-passed-tri-state-reflects-record
#[test]
fn verify_fail_reflected() {
    let mut state = make_state("build", &[], &[]);
    state.phase_verify_passed.insert("build".to_string(), false);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());
    assert_eq!(view.phases[0].verify_passed, Some(false));
}

/// scenario: belt-core-view-regate-passed-tri-state-reflects-record
#[test]
fn regate_pass_reflected() {
    let mut state = make_state("build", &[], &[]);
    state.regate_passed.insert("build".to_string(), true);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());
    assert_eq!(view.phases[0].regate_passed, Some(true));
}

/// scenario: belt-core-view-regate-passed-tri-state-reflects-record
#[test]
fn regate_not_run_is_none() {
    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());
    assert!(view.phases[0].regate_passed.is_none());
}

/// scenario: belt-core-view-attempt-count-defaults-to-zero-and-reflects-record
#[test]
fn attempt_zero_when_not_run() {
    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());
    assert_eq!(view.phases[0].attempt, 0);
}

/// scenario: belt-core-view-attempt-count-defaults-to-zero-and-reflects-record
#[test]
fn attempt_count_reflected() {
    let mut state = make_state("build", &[], &[]);
    state.phase_attempts.insert("build".to_string(), 3);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());
    assert_eq!(view.phases[0].attempt, 3);
}

// --- Task 4: Output directory scanning tests ---

/// scenario: belt-core-view-outputs-lists-phase-directory-files-sorted-excluding-subdirs
#[test]
fn outputs_lists_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let phase_dir = dir.path().join("build");
    std::fs::create_dir_all(&phase_dir).expect("mkdir");
    std::fs::write(phase_dir.join("report.json"), "{}").expect("write");
    std::fs::write(phase_dir.join("a_summary.md"), "# ok").expect("write");

    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);
    let view = build_status_view(&state, &ids, dir.path());

    // Sorted alphabetically
    assert_eq!(view.phases[0].outputs, vec!["a_summary.md", "report.json"]);
}

/// scenario: belt-core-view-outputs-lists-phase-directory-files-sorted-excluding-subdirs
#[test]
fn outputs_empty_when_no_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);
    let view = build_status_view(&state, &ids, dir.path());
    assert!(view.phases[0].outputs.is_empty());
}

/// scenario: belt-core-view-outputs-lists-phase-directory-files-sorted-excluding-subdirs
#[test]
fn outputs_empty_when_dir_exists_but_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let phase_dir = dir.path().join("build");
    std::fs::create_dir_all(&phase_dir).expect("mkdir");

    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);
    let view = build_status_view(&state, &ids, dir.path());
    assert!(view.phases[0].outputs.is_empty());
}

/// scenario: belt-core-view-outputs-lists-phase-directory-files-sorted-excluding-subdirs
#[test]
fn outputs_excludes_subdirectories() {
    let dir = tempfile::tempdir().expect("tempdir");
    let phase_dir = dir.path().join("build");
    std::fs::create_dir_all(phase_dir.join("subdir")).expect("mkdir");
    std::fs::write(phase_dir.join("file.txt"), "data").expect("write");

    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);
    let view = build_status_view(&state, &ids, dir.path());
    assert_eq!(view.phases[0].outputs, vec!["file.txt"]);
}

/// scenario: belt-core-view-sub-pipeline-phase-id-sanitized-to-underscore-for-filesystem
#[test]
fn outputs_sub_pipeline_phase_uses_underscore_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let phase_dir = dir.path().join("review_triage");
    std::fs::create_dir_all(&phase_dir).expect("mkdir");
    std::fs::write(phase_dir.join("findings.md"), "# findings").expect("write");

    let state = make_state("review/triage", &[], &[]);
    let ids = phase_ids(&["review/triage"]);
    let view = build_status_view(&state, &ids, dir.path());
    assert_eq!(view.phases[0].outputs, vec!["findings.md"]);
}

// --- Task 5: YAML drift tests ---

/// scenario: belt-core-view-yaml-drift-preserves-orphan-completed-and-skipped-phases
#[test]
fn yaml_drift_phase_added() {
    let state = make_state("test", &["build"], &[]);
    let ids = phase_ids(&["build", "lint", "test"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases[0].status, PhaseState::Completed); // build
    assert_eq!(view.phases[1].status, PhaseState::Pending); // lint (new)
    assert_eq!(view.phases[2].status, PhaseState::Current); // test
}

/// scenario: belt-core-view-yaml-drift-preserves-orphan-completed-and-skipped-phases
#[test]
fn yaml_drift_phase_removed_completed() {
    let state = make_state("test", &["old-phase", "build"], &[]);
    let ids = phase_ids(&["build", "test"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases[0].status, PhaseState::Completed);
    assert_eq!(view.phases[1].status, PhaseState::Current);
    assert_eq!(view.phases.len(), 3);
    assert_eq!(view.phases[2].id, "old-phase");
    assert_eq!(view.phases[2].status, PhaseState::Completed);
}

/// scenario: belt-core-view-yaml-drift-preserves-orphan-completed-and-skipped-phases
#[test]
fn yaml_drift_phase_removed_skipped() {
    let state = make_state("test", &["build"], &["old-phase"]);
    let ids = phase_ids(&["build", "test"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases.len(), 3);
    assert_eq!(view.phases[2].id, "old-phase");
    assert_eq!(view.phases[2].status, PhaseState::Skipped);
}

// --- Task 7: Engine enriched_status integration tests ---

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// scenario: belt-core-view-engine-enriched-status-integrates-state-metadata-and-outputs
#[test]
fn engine_enriched_status_after_init() {
    let dir = tempfile::tempdir().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline = fixture_path("status_pipeline.yml");

    let args = HashMap::new();
    let state = engine.init(&pipeline, &args).expect("init");

    let view = engine
        .enriched_status(&state.run_id)
        .expect("enriched_status");

    assert_eq!(view.status, PipelineStatus::InProgress);
    assert_eq!(view.current_phase.as_deref(), Some("build"));
    assert_eq!(view.phases.len(), 3);
    assert_eq!(view.phases[0].id, "build");
    assert_eq!(view.phases[0].status, PhaseState::Current);
    assert_eq!(view.phases[1].id, "review");
    assert_eq!(view.phases[1].status, PhaseState::Pending);
    assert_eq!(view.phases[2].id, "deploy");
    assert_eq!(view.phases[2].status, PhaseState::Pending);
    assert_eq!(view.progress.total, 3);
    assert_eq!(view.progress.remaining, 3);
}

/// scenario: belt-core-view-engine-enriched-status-integrates-state-metadata-and-outputs
#[test]
fn engine_enriched_status_with_skipped_phase() {
    let dir = tempfile::tempdir().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline = fixture_path("status_pipeline.yml");

    let mut args = HashMap::new();
    args.insert("skip_review".to_string(), serde_json::Value::Bool(true));
    let mut state = engine.init(&pipeline, &args).expect("init");

    // verify and step past build
    engine.verify_verdict(&mut state, true).expect("verify");
    engine.step(&mut state, &pipeline).expect("step");

    let view = engine
        .enriched_status(&state.run_id)
        .expect("enriched_status");

    assert_eq!(view.phases[0].status, PhaseState::Completed); // build
    assert_eq!(view.phases[1].status, PhaseState::Skipped); // review
    assert_eq!(view.phases[2].status, PhaseState::Current); // deploy
    assert_eq!(view.progress.completed, 1);
    assert_eq!(view.progress.skipped, 1);
    assert_eq!(view.progress.remaining, 1);
}

/// scenario: belt-core-view-engine-enriched-status-integrates-state-metadata-and-outputs
#[test]
fn engine_enriched_status_pipeline_not_found() {
    let dir = tempfile::tempdir().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline = fixture_path("status_pipeline.yml");

    let state = engine.init(&pipeline, &HashMap::new()).expect("init");

    // Tamper with state to point to a non-existent pipeline
    let mut loaded = engine.load_state(&state.run_id).expect("load");
    loaded.pipeline_file = "/nonexistent/pipeline.yml".to_string();
    engine.save_state(&loaded).expect("save");

    let result = engine.enriched_status(&state.run_id);
    assert!(result.is_err());
}

// --- BELT-30: verify_checks / regate_checks in status ---

/// status includes `verify_checks` when verify file exists.
/// scenario: belt-core-view-verify-checks-read-from-verify-json-with-graceful-degradation
#[test]
fn status_includes_verify_checks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let run_dir = dir.path().join("run1");
    std::fs::create_dir_all(run_dir.join("verify")).unwrap();
    std::fs::write(
        run_dir.join("verify/build.json"),
        r#"{"phase":"build","verdict":"PASS","checks":[{"check_type":"cmd","passed":true,"detail":null,"duration_ms":100,"timed_out":false}],"attempt":1,"timestamp":"2026-04-07T12:00:00Z"}"#,
    )
    .unwrap();

    let state = make_state("build", &[], &[]);
    let view = build_status_view(&state, &phase_ids(&["build"]), &run_dir);

    assert!(view.phases[0].verify_checks.is_some());
    let checks = view.phases[0].verify_checks.as_ref().unwrap();
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].check_type, "cmd");
    assert!(checks[0].passed);
}

/// status returns `verify_checks` = None when no verify file.
/// scenario: belt-core-view-verify-checks-read-from-verify-json-with-graceful-degradation
#[test]
fn status_verify_checks_none_when_no_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let run_dir = dir.path().join("run1");
    std::fs::create_dir_all(&run_dir).unwrap();

    let state = make_state("build", &[], &[]);
    let view = build_status_view(&state, &phase_ids(&["build"]), &run_dir);

    assert!(view.phases[0].verify_checks.is_none());
}

/// status returns `verify_checks` = None on corrupt file (graceful degradation).
/// scenario: belt-core-view-verify-checks-read-from-verify-json-with-graceful-degradation
#[test]
fn status_verify_checks_none_on_corrupt_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let run_dir = dir.path().join("run1");
    std::fs::create_dir_all(run_dir.join("verify")).unwrap();
    std::fs::write(run_dir.join("verify/build.json"), "not json{{{").unwrap();

    let state = make_state("build", &[], &[]);
    let view = build_status_view(&state, &phase_ids(&["build"]), &run_dir);

    assert!(view.phases[0].verify_checks.is_none());
}

/// status reads verify file for sub-pipeline phase with sanitized ID.
/// scenario: belt-core-view-sub-pipeline-phase-id-sanitized-to-underscore-for-filesystem
#[test]
fn status_verify_checks_sub_pipeline_phase() {
    let dir = tempfile::tempdir().expect("tempdir");
    let run_dir = dir.path().join("run1");
    std::fs::create_dir_all(run_dir.join("verify")).unwrap();
    std::fs::write(
        run_dir.join("verify/review_triage.json"),
        r#"{"phase":"review/triage","verdict":"FAIL","checks":[{"check_type":"cmd","passed":false,"detail":"exit 1:","duration_ms":50,"timed_out":false}],"attempt":1,"timestamp":"2026-04-07T12:00:00Z"}"#,
    )
    .unwrap();

    let state = make_state("review/triage", &[], &[]);
    let view = build_status_view(&state, &phase_ids(&["review/triage"]), &run_dir);

    assert!(view.phases[0].verify_checks.is_some());
    assert!(!view.phases[0].verify_checks.as_ref().unwrap()[0].passed);
}

/// status includes `regate_checks` when regate file exists.
/// scenario: belt-core-view-regate-checks-read-from-regate-json-with-graceful-degradation
#[test]
fn status_includes_regate_checks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let run_dir = dir.path().join("run1");
    std::fs::create_dir_all(run_dir.join("regate")).unwrap();
    std::fs::write(
        run_dir.join("regate/audit.json"),
        r#"{"phase":"audit","targets":{"collect":{"passed":true,"checks":[]}},"all_passed":true,"timestamp":"2026-04-07T12:00:00Z"}"#,
    )
    .unwrap();

    let state = make_state("audit", &[], &[]);
    let view = build_status_view(&state, &phase_ids(&["audit"]), &run_dir);

    assert!(view.phases[0].regate_checks.is_some());
    let targets = view.phases[0].regate_checks.as_ref().unwrap();
    assert!(targets["collect"]["passed"].as_bool().unwrap());
}

/// status returns `regate_checks` = None when no regate file.
/// scenario: belt-core-view-regate-checks-read-from-regate-json-with-graceful-degradation
#[test]
fn status_regate_checks_none_when_no_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let run_dir = dir.path().join("run1");
    std::fs::create_dir_all(&run_dir).unwrap();

    let state = make_state("build", &[], &[]);
    let view = build_status_view(&state, &phase_ids(&["build"]), &run_dir);

    assert!(view.phases[0].regate_checks.is_none());
}

/// status returns `regate_checks` = None on corrupt file.
/// scenario: belt-core-view-regate-checks-read-from-regate-json-with-graceful-degradation
#[test]
fn status_regate_checks_none_on_corrupt_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let run_dir = dir.path().join("run1");
    std::fs::create_dir_all(run_dir.join("regate")).unwrap();
    std::fs::write(run_dir.join("regate/build.json"), "corrupt!!!").unwrap();

    let state = make_state("build", &[], &[]);
    let view = build_status_view(&state, &phase_ids(&["build"]), &run_dir);

    assert!(view.phases[0].regate_checks.is_none());
}

/// scenario: belt-core-view-engine-enriched-status-integrates-state-metadata-and-outputs
#[test]
fn engine_enriched_status_output_files_visible() {
    let dir = tempfile::tempdir().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline = fixture_path("status_pipeline.yml");

    let state = engine.init(&pipeline, &HashMap::new()).expect("init");

    // Write a file into the build phase output directory
    let output_dir = belt_dir.join("runs").join(&state.run_id).join("build");
    std::fs::create_dir_all(&output_dir).expect("mkdir");
    std::fs::write(output_dir.join("artifact.tar.gz"), b"data").expect("write");

    let view = engine
        .enriched_status(&state.run_id)
        .expect("enriched_status");

    assert_eq!(view.phases[0].outputs, vec!["artifact.tar.gz"]);
}

// --- BELT-32: PhaseView invoke / produces / consumes serialization ---

/// `PhaseView` serializes `invoke` as a nested JSON object when present.
/// scenario: belt-core-view-phase-view-serializes-invoke-and-produces-as-structured-json
#[test]
fn phase_view_serializes_invoke_skill() {
    let state = RunState {
        run_id: "01961234-0000-7000-8000-000000000000".to_string(),
        pipeline: "test".to_string(),
        pipeline_file: "pipeline.yml".to_string(),
        version: 1,
        branch: None,
        resolved_consumes: HashMap::new(),
        args: HashMap::new(),
        current_phase: "design".to_string(),
        completed_phases: vec![],
        skipped_phases: vec![],
        phase_attempts: HashMap::new(),
        phase_verify_passed: HashMap::new(),
        regate_passed: HashMap::new(),
        phase_start_times: HashMap::new(),
        status: RunStatus::default(),
        created_at: "2026-04-11T00:00:00Z".to_string(),
        updated_at: "2026-04-11T00:00:00Z".to_string(),
    };

    let dir = tempfile::TempDir::new().expect("tempdir");

    let phases = vec![PhaseMetadata {
        id: "design".to_string(),
        invoke: Some(Invoker::Skill {
            skill: "/brainstorming".to_string(),
            args: HashMap::new(),
        }),
        produces: vec![Artifact {
            name: "design_doc".to_string(),
            path: "docs/plans/*-design.md".to_string(),
            description: Some("design".to_string()),
            when: None,
        }],
        consumes: vec![],
    }];

    let view = build_status_view(&state, &phases, dir.path());

    assert_eq!(view.phases.len(), 1);
    assert_eq!(view.phases[0].id, "design");
    assert!(
        view.phases[0].invoke.is_some(),
        "expected invoke in PhaseView"
    );
    let produces = view.phases[0]
        .produces
        .as_ref()
        .expect("produces resolved on design phase");
    assert_eq!(produces.len(), 1);
    assert_eq!(produces[0].name, "design_doc");
    assert!(view.phases[0].consumes.is_empty());

    // JSON round-trip check.
    let json = serde_json::to_string(&view).expect("serialize");
    assert!(json.contains("\"invoke\""));
    assert!(json.contains("\"skill\":\"/brainstorming\""));
    assert!(json.contains("\"produces\""));
    assert!(json.contains("\"design_doc\""));
}

// --- BELT-32 Plan B Task 2: glob resolution in build_status_view ---

/// Glob resolution picks the newest matching file after the phase start time.
/// scenario: belt-core-view-glob-produces-path-resolves-to-newest-file-after-phase-start-with-alphabetical-tiebreak
#[test]
fn glob_resolution_picks_newest_after_phase_start() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_dir = temp.path().join("run");
    std::fs::create_dir_all(&run_dir).expect("mkdir run_dir");

    let older = temp.path().join("docs-plans-2026-01-01-old-design.md");
    let newer = temp.path().join("docs-plans-2026-04-11-new-design.md");
    std::fs::write(&older, "older").expect("write older");
    std::fs::write(&newer, "newer").expect("write newer");
    let base = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap();
    set_file_mtime(&older, FileTime::from_unix_time(base - 4, 0)).unwrap();
    set_file_mtime(&newer, FileTime::from_unix_time(base + 4, 0)).unwrap();
    let phase_start: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

    let glob_pattern = format!("{}/docs-plans-*-design.md", temp.path().display());

    let metadata = vec![belt_core::view::PhaseMetadata {
        id: "design".to_string(),
        invoke: None,
        produces: vec![belt_core::model::Artifact {
            name: "design_doc".to_string(),
            path: glob_pattern.clone(),
            description: None,
            when: None,
        }],
        consumes: vec![],
    }];

    let mut phase_start_times = HashMap::new();
    phase_start_times.insert("design".to_string(), phase_start);

    let state = RunState {
        run_id: "test-run".to_string(),
        pipeline: "test".to_string(),
        pipeline_file: "/tmp/pipeline.yml".to_string(),
        version: 1,
        branch: None,
        resolved_consumes: HashMap::new(),
        args: HashMap::new(),
        current_phase: "design".to_string(),
        completed_phases: vec!["design".to_string()],
        skipped_phases: vec![],
        phase_attempts: HashMap::new(),
        phase_verify_passed: HashMap::new(),
        regate_passed: HashMap::new(),
        phase_start_times,
        status: RunStatus::default(),
        created_at: String::new(),
        updated_at: String::new(),
    };

    let view = build_status_view(&state, &metadata, &run_dir);
    let design = view
        .phases
        .iter()
        .find(|p| p.id == "design")
        .expect("design phase in view");

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
/// scenario: belt-core-view-glob-produces-path-resolves-to-newest-file-after-phase-start-with-alphabetical-tiebreak
#[test]
fn glob_resolution_zero_matches_reports_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_dir = temp.path().join("run");
    std::fs::create_dir_all(&run_dir).expect("mkdir run_dir");

    // Create a stale file BEFORE phase_start so it gets filtered out.
    let stale = temp.path().join("stale.md");
    std::fs::write(&stale, "stale").expect("write stale");
    let base = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap();
    set_file_mtime(&stale, FileTime::from_unix_time(base - 4, 0)).unwrap();
    let phase_start: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

    let glob_pattern = format!("{}/*.md", temp.path().display());

    let metadata = vec![belt_core::view::PhaseMetadata {
        id: "p".to_string(),
        invoke: None,
        produces: vec![belt_core::model::Artifact {
            name: "missing_doc".to_string(),
            path: glob_pattern,
            description: None,
            when: None,
        }],
        consumes: vec![],
    }];

    let mut phase_start_times = HashMap::new();
    phase_start_times.insert("p".to_string(), phase_start);

    let state = RunState {
        run_id: "test-run".to_string(),
        pipeline: "test".to_string(),
        pipeline_file: "/tmp/pipeline.yml".to_string(),
        version: 1,
        branch: None,
        resolved_consumes: HashMap::new(),
        args: HashMap::new(),
        current_phase: "p".to_string(),
        completed_phases: vec!["p".to_string()],
        skipped_phases: vec![],
        phase_attempts: HashMap::new(),
        phase_verify_passed: HashMap::new(),
        regate_passed: HashMap::new(),
        phase_start_times,
        status: RunStatus::default(),
        created_at: String::new(),
        updated_at: String::new(),
    };

    let view = build_status_view(&state, &metadata, &run_dir);
    let p = view
        .phases
        .iter()
        .find(|ph| ph.id == "p")
        .expect("p phase in view");
    let produces = p.produces.as_ref().expect("produces list");
    let missing = produces.iter().find(|a| a.name == "missing_doc").unwrap();
    assert!(
        !missing.exists,
        "no matches after phase_start => exists=false"
    );
    assert!(missing.resolved_path.is_none());
}

/// Equal mtimes break ties via ascending filename.
/// scenario: belt-core-view-glob-produces-path-resolves-to-newest-file-after-phase-start-with-alphabetical-tiebreak
#[test]
fn glob_resolution_equal_mtime_alphabetical_tiebreaker() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_dir = temp.path().join("run");
    std::fs::create_dir_all(&run_dir).expect("mkdir run_dir");

    let phase_start: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

    let b = temp.path().join("b.md");
    let a = temp.path().join("a.md");
    std::fs::write(&b, "b").expect("write b");
    std::fs::write(&a, "a").expect("write a");

    let base = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap();
    let same = FileTime::from_unix_time(base + 2, 0);
    set_file_mtime(&a, same).expect("set a mtime");
    set_file_mtime(&b, same).expect("set b mtime");

    let glob_pattern = format!("{}/*.md", temp.path().display());

    let metadata = vec![belt_core::view::PhaseMetadata {
        id: "p".to_string(),
        invoke: None,
        produces: vec![belt_core::model::Artifact {
            name: "doc".to_string(),
            path: glob_pattern,
            description: None,
            when: None,
        }],
        consumes: vec![],
    }];

    let mut phase_start_times = HashMap::new();
    phase_start_times.insert("p".to_string(), phase_start);

    let state = RunState {
        run_id: "test-run".to_string(),
        pipeline: "test".to_string(),
        pipeline_file: "/tmp/pipeline.yml".to_string(),
        version: 1,
        branch: None,
        resolved_consumes: HashMap::new(),
        args: HashMap::new(),
        current_phase: "p".to_string(),
        completed_phases: vec!["p".to_string()],
        skipped_phases: vec![],
        phase_attempts: HashMap::new(),
        phase_verify_passed: HashMap::new(),
        regate_passed: HashMap::new(),
        phase_start_times,
        status: RunStatus::default(),
        created_at: String::new(),
        updated_at: String::new(),
    };

    let view = build_status_view(&state, &metadata, &run_dir);
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
        "equal mtimes => alphabetical 'a.md' wins over 'b.md'"
    );
}

/// Concrete (non-glob) path uses `std::fs::metadata` directly.
/// scenario: belt-core-view-glob-produces-path-resolves-to-newest-file-after-phase-start-with-alphabetical-tiebreak
#[test]
fn concrete_path_skips_filter() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_dir = temp.path().join("run");
    std::fs::create_dir_all(&run_dir).expect("mkdir run_dir");

    let concrete = temp.path().join("smoke-test-report.md");
    std::fs::write(&concrete, "report").expect("write concrete");

    // phase_start is AFTER file creation -- concrete paths bypass the mtime filter.
    let base = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap();
    set_file_mtime(&concrete, FileTime::from_unix_time(base - 4, 0)).unwrap();
    let phase_start: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

    let metadata = vec![belt_core::view::PhaseMetadata {
        id: "smoke".to_string(),
        invoke: None,
        produces: vec![belt_core::model::Artifact {
            name: "report".to_string(),
            path: concrete.to_str().unwrap().to_string(),
            description: None,
            when: None,
        }],
        consumes: vec![],
    }];

    let mut phase_start_times = HashMap::new();
    phase_start_times.insert("smoke".to_string(), phase_start);

    let state = RunState {
        run_id: "test-run".to_string(),
        pipeline: "test".to_string(),
        pipeline_file: "/tmp/pipeline.yml".to_string(),
        version: 1,
        branch: None,
        resolved_consumes: HashMap::new(),
        args: HashMap::new(),
        current_phase: "smoke".to_string(),
        completed_phases: vec!["smoke".to_string()],
        skipped_phases: vec![],
        phase_attempts: HashMap::new(),
        phase_verify_passed: HashMap::new(),
        regate_passed: HashMap::new(),
        phase_start_times,
        status: RunStatus::default(),
        created_at: String::new(),
        updated_at: String::new(),
    };

    let view = build_status_view(&state, &metadata, &run_dir);
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
    assert!(
        report.exists,
        "concrete path must exist regardless of mtime"
    );
    assert_eq!(
        report.resolved_path.as_deref(),
        Some(concrete.to_str().unwrap())
    );
}
