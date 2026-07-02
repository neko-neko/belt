//! Integration tests for the composed bug-fix pipeline (2026-07-02 pipeline
//! split): diagnose(sub) + pre-execute-handover(sub) + build(sub).
//!
//! Shape contract (spec docs/specs/2026-07-02-pipeline-split-design.md):
//! - args = { e2e: bool, codex: bool } only (legacy args stay removed)
//! - 3 top-level phases, all Invoker::Pipeline delegations:
//!   diagnose -> ../diagnose/pipeline.yml,
//!   pre-execute-handover -> ../handover/checkpoint.yml,
//!   build -> ../build/pipeline.yml
//! - diagnose/build receive with = { e2e: "args.e2e", codex: "args.codex" }
//! - expansion flattens to exactly 9 namespaced leaves
//! - stage-internal regate expands namespaced; verify leaves inherit
//!   when: "args.e2e"
//!
//! Stage-internal shape (phase order, docs/features artifact paths,
//! narrative notes, criteria files) is locked per stage in
//! `pipeline_split_refresh.rs`.

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
    "build/verify/monkey-test",
    "build/verify/dogfood",
    "build/integrate",
];

#[test]
fn bug_fix_composes_three_stages() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&bug_fix_pipeline_path())?;
    let got: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        got,
        vec!["diagnose", "pre-execute-handover", "build"],
        "top-level composition must be diagnose -> checkpoint -> build"
    );
    Ok(())
}

#[test]
fn stages_delegate_with_e2e_and_codex_passthrough() {
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
        let mut keys: Vec<&str> = with.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec!["codex", "e2e"],
            "phase '{phase_id}' must pass exactly {{codex, e2e}}"
        );
        assert_eq!(
            with.get("e2e").and_then(|v| v.as_str()),
            Some("args.e2e"),
            "phase '{phase_id}' e2e must be the bare full-string form"
        );
        assert_eq!(
            with.get("codex").and_then(|v| v.as_str()),
            Some("args.codex"),
            "phase '{phase_id}' codex must be the bare full-string form"
        );
    }
}

#[test]
fn checkpoint_delegates_with_no_args() {
    let pipeline = parse_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must parse");
    let phase = pipeline
        .phases
        .iter()
        .find(|p| p.id == "pre-execute-handover")
        .expect("pre-execute-handover phase must exist");
    match phase.invoke.as_ref() {
        Some(Invoker::Pipeline {
            pipeline: sub_path,
            with,
        }) => {
            assert_eq!(
                sub_path, "../handover/checkpoint.yml",
                "pre-execute-handover must delegate to ../handover/checkpoint.yml"
            );
            assert!(
                with.is_empty(),
                "pre-execute-handover delegation must not pass any `with` args"
            );
        }
        other => panic!("pre-execute-handover must use Invoker::Pipeline, got {other:?}"),
    }
}

#[test]
fn args_are_e2e_and_codex_only() {
    let pipeline = parse_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must parse");
    let mut keys: Vec<&str> = pipeline.args.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["codex", "e2e"]);

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
    for legacy in ["iterations", "swarm", "ui", "smoke"] {
        assert!(
            !pipeline.args.contains_key(legacy),
            "legacy arg '{legacy}' must be removed"
        );
    }
}

#[test]
fn bug_fix_expands_to_nine_namespaced_leaves() {
    let expanded = expand_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must expand");
    let ids: Vec<&str> = expanded.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, EXPECTED_LEAVES, "expanded leaf ids + order must match");
}

#[test]
fn expanded_regate_targets_are_namespaced() {
    let expanded = expand_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must expand");
    let code_review = expanded
        .iter()
        .find(|p| p.id == "build/code-review")
        .expect("build/code-review leaf must exist");
    assert_eq!(
        code_review.regate,
        vec!["build/execute".to_string()],
        "stage-internal regate must expand into the stage namespace"
    );
    // diagnose declares no regate; every other leaf must have none.
    for leaf in &expanded {
        if leaf.id != "build/code-review" {
            assert!(
                leaf.regate.is_empty(),
                "leaf '{}' must have empty regate, got {:?}",
                leaf.id,
                leaf.regate
            );
        }
    }
}

#[test]
fn expanded_verify_leaves_inherit_e2e_when() {
    let expanded = expand_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must expand");
    for id in ["build/verify/monkey-test", "build/verify/dogfood"] {
        let leaf = expanded
            .iter()
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("leaf '{id}' must exist"));
        assert_eq!(
            leaf.when.as_deref(),
            Some("args.e2e"),
            "leaf '{id}' must inherit when: args.e2e from build's verify phase"
        );
    }
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
