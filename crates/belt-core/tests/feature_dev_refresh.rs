//! Integration tests for the composed feature-dev pipeline (2026-07-05
//! sonnet-lean rewrite): design(sub) + pre-execute-handover(sub) + build(sub).
//!
//! Shape contract (spec docs/specs/2026-07-05-sonnet-lean-pipeline-design.md):
//! - args = { e2e: bool, codex: bool } only
//! - 3 top-level phases, all Invoker::Pipeline delegations:
//!   design -> ../design/pipeline.yml,
//!   pre-execute-handover -> ../handover/checkpoint.yml,
//!   build -> ../build/pipeline.yml
//! - design/build receive with = { e2e: "args.e2e", codex: "args.codex" }
//!   (bare full-string form — the only form the expander substitutes)
//! - expansion flattens to exactly 7 namespaced leaves
//! - no leaf declares regate (sonnet-lean removed regate)
//! - build/e2e is the only leaf carrying when: "args.e2e"

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

const EXPECTED_LEAVES: &[&str] = &[
    "design/intake",
    "design/design",
    "pre-execute-handover/checkpoint",
    "build/execute",
    "build/code-review",
    "build/e2e",
    "build/integrate",
];

#[test]
fn feature_dev_composes_three_stages() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;
    let got: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        got,
        vec!["design", "pre-execute-handover", "build"],
        "top-level composition must be design -> checkpoint -> build"
    );
    Ok(())
}

#[test]
fn stages_delegate_with_e2e_and_codex_passthrough() {
    let pipeline =
        parse_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must parse");
    for (phase_id, expected_sub) in [
        ("design", "../design/pipeline.yml"),
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
    let pipeline =
        parse_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must parse");
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
fn top_level_args_are_e2e_and_codex_only() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;

    let mut names: Vec<&str> = pipeline.args.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec!["codex", "e2e"],
        "args must be exactly {{codex, e2e}}"
    );

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
fn feature_dev_expands_to_seven_namespaced_leaves() {
    let expanded =
        expand_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must expand");
    let ids: Vec<&str> = expanded.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, EXPECTED_LEAVES, "expanded leaf ids + order must match");
}

#[test]
fn no_leaf_declares_regate() {
    let expanded =
        expand_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must expand");
    for leaf in &expanded {
        assert!(
            leaf.regate.is_empty(),
            "leaf '{}' must have empty regate (sonnet-lean removed regate), got {:?}",
            leaf.id,
            leaf.regate
        );
    }
}

#[test]
fn e2e_leaf_carries_when_and_others_do_not() {
    let expanded =
        expand_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must expand");
    let e2e = expanded
        .iter()
        .find(|p| p.id == "build/e2e")
        .expect("build/e2e leaf must exist");
    assert_eq!(
        e2e.when.as_deref(),
        Some("args.e2e"),
        "build/e2e must carry when: args.e2e"
    );
    for id in ["build/execute", "build/code-review", "build/integrate"] {
        let leaf = expanded
            .iter()
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("leaf '{id}' must exist"));
        assert_eq!(leaf.when, None, "leaf '{id}' must not carry a when");
    }
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
