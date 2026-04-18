//! Integration tests for the refreshed feature-dev pipeline (10 phases).

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use std::path::PathBuf;

use belt_core::{
    error::BeltError, expander::expand_pipeline, model::ArgType, parser::parse_pipeline,
};

mod common;
use common::helpers::repo_root;
use common::narrative::{
    assert_narrative_accumulating_consumes, assert_narrative_gate_paths,
    assert_narrative_produce_paths, assert_non_narrative_phases_have_no_notes,
};

fn feature_dev_pipeline_path() -> PathBuf {
    repo_root().join("plugins/belt/skills/feature-dev/pipeline.yml")
}

#[test]
fn feature_dev_has_ten_phases() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;

    let expected: &[&str] = &[
        "design",
        "test-scenarios",
        "spec-review",
        "plan",
        "pre-execute-handover",
        "execute",
        "code-review",
        "monkey-test",
        "dogfood",
        "integrate",
    ];

    let got: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(got, expected, "phase IDs must match spec order");
    Ok(())
}

#[test]
fn feature_dev_expands_cleanly() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;
    // Refresh deletes all `uses:`/`invoke.pipeline:` references; the expanded
    // phases must equal the top-level phases 1:1.
    let expanded = expand_pipeline(&feature_dev_pipeline_path())?;
    assert_eq!(expanded.len(), pipeline.phases.len());
    Ok(())
}

#[test]
fn monkey_test_and_dogfood_are_conditional_on_e2e() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;

    let monkey = pipeline
        .phases
        .iter()
        .find(|p| p.id == "monkey-test")
        .ok_or_else(|| BeltError::InvalidPipeline {
            message: "monkey-test phase missing".to_string(),
        })?;
    let dogfood = pipeline
        .phases
        .iter()
        .find(|p| p.id == "dogfood")
        .ok_or_else(|| BeltError::InvalidPipeline {
            message: "dogfood phase missing".to_string(),
        })?;

    assert_eq!(
        monkey.when.as_deref(),
        Some("args.e2e"),
        "monkey-test must be gated by args.e2e"
    );
    assert_eq!(
        dogfood.when.as_deref(),
        Some("args.e2e"),
        "dogfood must be gated by args.e2e"
    );
    Ok(())
}

#[test]
fn scenarios_produce_is_conditional_on_e2e() -> Result<(), BeltError> {
    // The typed `Artifact` struct in belt-core does not (yet) surface a
    // per-artifact `when` field, so we assert the pipeline YAML declares
    // `when: "args.e2e"` on the `scenarios` produce by parsing the source
    // as an untyped `serde_json::Value` tree. This keeps the test aligned
    // with the pipeline contract without mutating belt-core's model.
    let yaml = std::fs::read_to_string(feature_dev_pipeline_path())?;
    let doc: serde_json::Value =
        serde_saphyr::from_str(&yaml).map_err(|e| BeltError::YamlParse {
            message: e.to_string(),
            src: Some(yaml.clone()),
        })?;

    let phases = doc
        .get("phases")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| BeltError::InvalidPipeline {
            message: "phases array missing".to_string(),
        })?;

    let test_scenarios = phases
        .iter()
        .find(|p| p.get("id").and_then(serde_json::Value::as_str) == Some("test-scenarios"))
        .ok_or_else(|| BeltError::InvalidPipeline {
            message: "test-scenarios phase missing".to_string(),
        })?;

    let produces = test_scenarios
        .get("produces")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| BeltError::InvalidPipeline {
            message: "test-scenarios.produces array missing".to_string(),
        })?;

    let scenarios_artifact = produces
        .iter()
        .find(|a| a.get("name").and_then(serde_json::Value::as_str) == Some("scenarios"))
        .ok_or_else(|| BeltError::InvalidPipeline {
            message: "scenarios artifact missing".to_string(),
        })?;

    assert_eq!(
        scenarios_artifact
            .get("when")
            .and_then(serde_json::Value::as_str),
        Some("args.e2e"),
        "scenarios.yml produce must be gated by args.e2e"
    );
    Ok(())
}

#[test]
fn code_review_regates_execute_only() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;

    let code_review = pipeline
        .phases
        .iter()
        .find(|p| p.id == "code-review")
        .ok_or_else(|| BeltError::InvalidPipeline {
            message: "code-review phase missing".to_string(),
        })?;

    assert_eq!(
        code_review.regate,
        vec!["execute".to_string()],
        "code-review.regate must target only [execute] (no smoke-test/doc-audit)"
    );
    assert_eq!(
        code_review.max_retries, 3,
        "code-review.max_retries must be 3"
    );
    Ok(())
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

    let e2e = pipeline
        .args
        .get("e2e")
        .ok_or_else(|| BeltError::InvalidPipeline {
            message: "e2e arg missing".to_string(),
        })?;
    assert!(
        matches!(e2e.arg_type, ArgType::Bool),
        "e2e arg must be typed bool"
    );
    assert_eq!(
        e2e.default.as_ref().and_then(serde_json::Value::as_bool),
        Some(false),
        "e2e arg default must be false"
    );

    let codex = pipeline
        .args
        .get("codex")
        .ok_or_else(|| BeltError::InvalidPipeline {
            message: "codex arg missing".to_string(),
        })?;
    assert!(
        matches!(codex.arg_type, ArgType::Bool),
        "codex arg must be typed bool"
    );
    assert_eq!(
        codex.default.as_ref().and_then(serde_json::Value::as_bool),
        Some(false),
        "codex arg default must be false"
    );
    Ok(())
}

#[test]
fn feature_dev_scenarios_artifact_has_typed_when_field() {
    let pipeline_path = feature_dev_pipeline_path();
    let pipeline =
        belt_core::parser::parse_pipeline(&pipeline_path).expect("feature-dev pipeline must parse");
    let test_scenarios_phase = pipeline
        .phases
        .iter()
        .find(|p| p.id == "test-scenarios")
        .expect("test-scenarios phase must exist");
    let scenarios_artifact = test_scenarios_phase
        .produces
        .iter()
        .find(|a| a.name == "scenarios")
        .expect("scenarios artifact must exist");
    assert_eq!(
        scenarios_artifact.when,
        Some("args.e2e".to_string()),
        "scenarios.when must parse as a typed field (regression test for silent-drop bug)"
    );
}

// --- narrative artifact shape (context reset) ---

// feature-dev narrative-producing phases.
// Tuple fields: (`phase_id`, `artifact_name`, path).
const FEATURE_DEV_NARRATIVE_PHASES: &[(&str, &str, &str)] = &[
    (
        "design",
        "design_notes",
        ".belt/runs/{run_id}/notes/phase-design.md",
    ),
    (
        "plan",
        "plan_notes",
        ".belt/runs/{run_id}/notes/phase-plan.md",
    ),
    (
        "execute",
        "execute_notes",
        ".belt/runs/{run_id}/notes/phase-execute.md",
    ),
    (
        "code-review",
        "code_review_notes",
        ".belt/runs/{run_id}/notes/phase-code-review.md",
    ),
    (
        "monkey-test",
        "monkey_test_notes",
        ".belt/runs/{run_id}/notes/phase-monkey-test.md",
    ),
    (
        "dogfood",
        "dogfood_notes",
        ".belt/runs/{run_id}/notes/phase-dogfood.md",
    ),
];

#[test]
fn feature_dev_narrative_phases_produce_notes() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;
    assert_narrative_produce_paths(&pipeline, FEATURE_DEV_NARRATIVE_PHASES);
    Ok(())
}

#[test]
fn feature_dev_narrative_phases_gate_notes() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;
    assert_narrative_gate_paths(&pipeline, FEATURE_DEV_NARRATIVE_PHASES);
    Ok(())
}

#[test]
fn feature_dev_narrative_accumulating_consumes() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;

    // accumulating: 各 narrative phase は それ以前の narrative phases の notes を全て consume。
    let expected_consumes: &[(&str, &[&str])] = &[
        ("design", &[]),
        ("plan", &["design_notes"]),
        ("execute", &["design_notes", "plan_notes"]),
        (
            "code-review",
            &["design_notes", "plan_notes", "execute_notes"],
        ),
        (
            "monkey-test",
            &[
                "design_notes",
                "plan_notes",
                "execute_notes",
                "code_review_notes",
            ],
        ),
        (
            "dogfood",
            &[
                "design_notes",
                "plan_notes",
                "execute_notes",
                "code_review_notes",
                "monkey_test_notes",
            ],
        ),
    ];

    assert_narrative_accumulating_consumes(&pipeline, expected_consumes);
    Ok(())
}

#[test]
fn feature_dev_non_narrative_phases_have_no_notes() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;
    assert_non_narrative_phases_have_no_notes(
        &pipeline,
        &["test-scenarios", "spec-review", "integrate"],
    );
    Ok(())
}
