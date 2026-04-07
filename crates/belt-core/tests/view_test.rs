use belt_core::engine::Engine;
use belt_core::model::RunState;
use belt_core::view::{PhaseState, PipelineStatus, build_status_view};
use std::collections::HashMap;

fn make_state(current: &str, completed: &[&str], skipped: &[&str]) -> RunState {
    RunState {
        run_id: "test-run".to_string(),
        pipeline: "test".to_string(),
        pipeline_file: "/tmp/pipeline.yml".to_string(),
        version: 1,
        args: HashMap::new(),
        current_phase: current.to_string(),
        completed_phases: completed.iter().map(|s| (*s).to_string()).collect(),
        skipped_phases: skipped.iter().map(|s| (*s).to_string()).collect(),
        phase_attempts: HashMap::new(),
        phase_verify_passed: HashMap::new(),
        regate_passed: HashMap::new(),
        created_at: "2026-04-07T00:00:00Z".to_string(),
        updated_at: "2026-04-07T00:00:00Z".to_string(),
    }
}

fn phase_ids(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|s| (*s).to_string()).collect()
}

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

#[test]
fn verify_not_run_is_none() {
    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());
    assert!(view.phases[0].verify_passed.is_none());
}

#[test]
fn verify_pass_reflected() {
    let mut state = make_state("build", &[], &[]);
    state.phase_verify_passed.insert("build".to_string(), true);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());
    assert_eq!(view.phases[0].verify_passed, Some(true));
}

#[test]
fn verify_fail_reflected() {
    let mut state = make_state("build", &[], &[]);
    state.phase_verify_passed.insert("build".to_string(), false);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());
    assert_eq!(view.phases[0].verify_passed, Some(false));
}

#[test]
fn regate_pass_reflected() {
    let mut state = make_state("build", &[], &[]);
    state.regate_passed.insert("build".to_string(), true);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());
    assert_eq!(view.phases[0].regate_passed, Some(true));
}

#[test]
fn regate_not_run_is_none() {
    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());
    assert!(view.phases[0].regate_passed.is_none());
}

#[test]
fn attempt_zero_when_not_run() {
    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");
    let view = build_status_view(&state, &ids, dir.path());
    assert_eq!(view.phases[0].attempt, 0);
}

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

#[test]
fn outputs_empty_when_no_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);
    let view = build_status_view(&state, &ids, dir.path());
    assert!(view.phases[0].outputs.is_empty());
}

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
