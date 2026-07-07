//! Integration tests for the composed bug-fix pipeline (2026-07-07
//! four-stage rewrite): diagnose(sub) + pre-execute-handover(sub) +
//! build(sub) + qa(sub) + integrate(leaf).
//!
//! Shape contract (spec docs/specs/2026-07-07-four-stage-agentic-pipeline-design.md):
//! - args = { codex: bool } only (e2e removed — QA is mandatory, D2)
//! - 5 top-level phases: diagnose/build delegate with { codex },
//!   pre-execute-handover and qa delegate with empty `with`,
//!   integrate is an inline leaf (Invoker::Skill /worktrunk)
//! - expansion flattens to exactly 8 namespaced leaves
//! - no leaf declares regate; no leaf carries a phase-level when
//! - confirm leaves are exactly diagnose/fix-plan-review,
//!   pre-execute-handover/checkpoint, integrate (D4: one diagnosis
//!   approval point)
//! - integrate identity with feature-dev is locked in
//!   feature_dev_refresh.rs::integrate_leaf_identical_across_orchestrators

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

fn bug_fix_pipeline_path() -> PathBuf {
    repo_root().join("plugins/belt/skills/bug-fix/pipeline.yml")
}

const EXPECTED_LEAVES: &[&str] = &[
    "diagnose/rca",
    "diagnose/fix-plan",
    "diagnose/fix-plan-review",
    "pre-execute-handover/checkpoint",
    "build/execute",
    "build/code-review",
    "qa/qa",
    "integrate",
];

const CONFIRM_LEAVES: &[&str] = &[
    "diagnose/fix-plan-review",
    "pre-execute-handover/checkpoint",
    "integrate",
];

#[test]
fn bug_fix_composes_five_stages() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&bug_fix_pipeline_path())?;
    let got: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        got,
        vec![
            "diagnose",
            "pre-execute-handover",
            "build",
            "qa",
            "integrate"
        ],
        "top-level composition must be diagnose -> checkpoint -> build -> qa -> integrate"
    );
    Ok(())
}

#[test]
fn stages_delegate_with_codex_passthrough() {
    let pipeline = parse_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must parse");
    for (phase_id, expected_sub) in [
        ("diagnose", "../diagnose/pipeline.yml"),
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
    let pipeline = parse_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must parse");
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
fn args_are_codex_only() {
    let pipeline = parse_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must parse");
    let keys: Vec<&str> = pipeline.args.keys().map(String::as_str).collect();
    assert_eq!(keys, vec!["codex"]);

    for (name, def) in &pipeline.args {
        assert!(
            matches!(def.arg_type, ArgType::Bool),
            "arg '{name}' must be bool"
        );
        assert_eq!(
            def.default.as_ref().and_then(serde_json::Value::as_bool),
            Some(false),
            "arg '{name}' default must be false"
        );
    }
}

#[test]
fn no_legacy_args() {
    let pipeline = parse_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must parse");
    for legacy in ["iterations", "swarm", "ui", "smoke", "e2e"] {
        assert!(
            !pipeline.args.contains_key(legacy),
            "legacy arg '{legacy}' must be removed"
        );
    }
}

#[test]
fn bug_fix_expands_to_eight_namespaced_leaves() {
    let expanded = expand_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must expand");
    let ids: Vec<&str> = expanded.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, EXPECTED_LEAVES, "expanded leaf ids + order must match");
}

#[test]
fn no_leaf_declares_regate_or_when() {
    let expanded = expand_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must expand");
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
fn confirm_leaves_match_the_three_touchpoints() {
    let expanded = expand_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must expand");
    let confirmed: Vec<&str> = expanded
        .iter()
        .filter(|p| p.confirm)
        .map(|p| p.id.as_str())
        .collect();
    assert_eq!(
        confirmed, CONFIRM_LEAVES,
        "confirm leaves must be exactly the three human touchpoints (D4)"
    );
}

#[test]
fn bug_fix_pipeline_has_no_run_id_template() {
    let yaml = std::fs::read_to_string(bug_fix_pipeline_path())
        .expect("bug-fix pipeline.yml must be readable");
    assert!(
        !yaml.contains("{run_id}"),
        "pipeline must not contain {{run_id}} template anywhere"
    );
    assert!(
        !yaml.contains(".belt/runs/"),
        "pipeline must not contain .belt/runs/ literal anywhere"
    );
}
