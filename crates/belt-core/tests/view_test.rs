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
