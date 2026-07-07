//! Integration tests for the composed feature-dev pipeline (2026-07-07
//! four-stage rewrite): design(sub) + plan(sub) + pre-execute-handover(sub)
//! + build(sub) + qa(sub) + integrate(leaf).
//!
//! Shape contract (spec docs/specs/2026-07-07-four-stage-agentic-pipeline-design.md):
//! - args = { codex: bool } only (e2e removed — QA is mandatory, D2)
//! - 6 top-level phases: design/plan/build delegate with { codex },
//!   pre-execute-handover and qa delegate with empty `with`,
//!   integrate is an inline leaf (Invoker::Skill /worktrunk)
//! - expansion flattens to exactly 8 namespaced leaves
//! - no leaf declares regate; no leaf carries a phase-level when
//! - confirm leaves are exactly design/design, plan/plan,
//!   pre-execute-handover/checkpoint, integrate (D4)
//! - the integrate leaf is byte-equivalent (as serde_json::Value) to the
//!   bug-fix integrate leaf (D14 inline duplication + identity lock)

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use std::path::PathBuf;

use belt_core::{
    error::BeltError,
    expander::expand_pipeline,
    model::{ArgType, Invoker},
    parser::parse_pipeline,
};

mod common;
use common::helpers::repo_root;

fn feature_dev_pipeline_path() -> PathBuf {
    repo_root().join("plugins/belt/skills/feature-dev/pipeline.yml")
}

fn bug_fix_pipeline_path() -> PathBuf {
    repo_root().join("plugins/belt/skills/bug-fix/pipeline.yml")
}

const EXPECTED_LEAVES: &[&str] = &[
    "design/intake",
    "design/design",
    "plan/plan",
    "pre-execute-handover/checkpoint",
    "build/execute",
    "build/code-review",
    "qa/qa",
    "integrate",
];

const CONFIRM_LEAVES: &[&str] = &[
    "design/design",
    "plan/plan",
    "pre-execute-handover/checkpoint",
    "integrate",
];

#[test]
fn feature_dev_composes_six_stages() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;
    let got: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        got,
        vec![
            "design",
            "plan",
            "pre-execute-handover",
            "build",
            "qa",
            "integrate"
        ],
        "top-level composition must be design -> plan -> checkpoint -> build -> qa -> integrate"
    );
    Ok(())
}

#[test]
fn stages_delegate_with_codex_passthrough() {
    let pipeline =
        parse_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must parse");
    for (phase_id, expected_sub) in [
        ("design", "../design/pipeline.yml"),
        ("plan", "../plan/pipeline.yml"),
        ("build", "../build/pipeline.yml"),
    ] {
        let phase = pipeline
            .phases
            .iter()
            .find(|p| p.id == phase_id)
            .unwrap_or_else(|| panic!("phase '{phase_id}' must exist"));
        let Some(Invoker::Pipeline {
            pipeline: sub_path,
            with,
        }) = phase.invoke.as_ref()
        else {
            panic!("phase '{phase_id}' must use Invoker::Pipeline");
        };
        assert_eq!(sub_path, expected_sub, "phase '{phase_id}' sub path");
        let keys: Vec<&str> = with.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            vec!["codex"],
            "phase '{phase_id}' must pass exactly {{codex}}"
        );
        assert_eq!(
            with.get("codex").and_then(|v| v.as_str()),
            Some("args.codex"),
            "phase '{phase_id}' codex must be the bare full-string form"
        );
    }
}

#[test]
fn checkpoint_and_qa_delegate_with_no_args() {
    let pipeline =
        parse_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must parse");
    for (phase_id, expected_sub) in [
        ("pre-execute-handover", "../handover/checkpoint.yml"),
        ("qa", "../qa/pipeline.yml"),
    ] {
        let phase = pipeline
            .phases
            .iter()
            .find(|p| p.id == phase_id)
            .unwrap_or_else(|| panic!("phase '{phase_id}' must exist"));
        match phase.invoke.as_ref() {
            Some(Invoker::Pipeline {
                pipeline: sub_path,
                with,
            }) => {
                assert_eq!(sub_path, expected_sub, "phase '{phase_id}' sub path");
                assert!(
                    with.is_empty(),
                    "phase '{phase_id}' delegation must not pass any `with` args"
                );
            }
            other => panic!("phase '{phase_id}' must use Invoker::Pipeline, got {other:?}"),
        }
    }
}

#[test]
fn top_level_args_are_codex_only() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;

    let names: Vec<&str> = pipeline.args.keys().map(String::as_str).collect();
    assert_eq!(names, vec!["codex"], "args must be exactly {{codex}}");

    for (name, def) in &pipeline.args {
        assert!(
            matches!(def.arg_type, ArgType::Bool),
            "arg '{name}' must be typed bool"
        );
        assert_eq!(
            def.default.as_ref().and_then(serde_json::Value::as_bool),
            Some(false),
            "arg '{name}' default must be false"
        );
    }
    Ok(())
}

#[test]
fn feature_dev_expands_to_eight_namespaced_leaves() {
    let expanded =
        expand_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must expand");
    let ids: Vec<&str> = expanded.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, EXPECTED_LEAVES, "expanded leaf ids + order must match");
}

#[test]
fn no_leaf_declares_regate_or_when() {
    let expanded =
        expand_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must expand");
    for leaf in &expanded {
        assert!(
            leaf.regate.is_empty(),
            "leaf '{}' must have empty regate, got {:?}",
            leaf.id,
            leaf.regate
        );
        assert_eq!(
            leaf.when, None,
            "leaf '{}' must not carry a phase-level when (e2e opt-in removed)",
            leaf.id
        );
    }
}

#[test]
fn confirm_leaves_match_the_four_touchpoints() {
    let expanded =
        expand_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must expand");
    let confirmed: Vec<&str> = expanded
        .iter()
        .filter(|p| p.confirm)
        .map(|p| p.id.as_str())
        .collect();
    assert_eq!(
        confirmed, CONFIRM_LEAVES,
        "confirm leaves must be exactly the four human touchpoints (D4)"
    );
}

#[test]
fn integrate_leaf_identical_across_orchestrators() {
    let feature = parse_pipeline(&feature_dev_pipeline_path()).expect("feature-dev must parse");
    let bug = parse_pipeline(&bug_fix_pipeline_path()).expect("bug-fix must parse");
    let f_integrate = feature
        .phases
        .iter()
        .find(|p| p.id == "integrate")
        .expect("feature-dev integrate leaf must exist");
    let b_integrate = bug
        .phases
        .iter()
        .find(|p| p.id == "integrate")
        .expect("bug-fix integrate leaf must exist");
    let f_val = serde_json::to_value(f_integrate).expect("serialize feature-dev integrate");
    let b_val = serde_json::to_value(b_integrate).expect("serialize bug-fix integrate");
    assert_eq!(
        f_val, b_val,
        "integrate leaf must be identical across feature-dev and bug-fix (D14)"
    );
}

#[test]
fn feature_dev_pipeline_has_no_run_id_template() {
    let yaml = std::fs::read_to_string(feature_dev_pipeline_path())
        .expect("feature-dev pipeline.yml must be readable");
    assert!(
        !yaml.contains("{run_id}"),
        "pipeline must not contain {{run_id}} template anywhere"
    );
    assert!(
        !yaml.contains(".belt/runs/"),
        "pipeline must not contain .belt/runs/ literal anywhere"
    );
}
