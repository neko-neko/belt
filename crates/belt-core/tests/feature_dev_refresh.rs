//! Integration tests for the refreshed feature-dev pipeline (8 phases).

use std::path::PathBuf;

use belt_core::{
    error::BeltError, expander::expand_pipeline, model::ArgType, parser::parse_pipeline,
};

fn feature_dev_pipeline_path() -> PathBuf {
    // `CARGO_MANIFEST_DIR` points at `crates/belt-core`; walk two levels up to
    // reach the workspace root, then join the pipeline path.
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path.push("examples/skills/feature-dev/pipeline.yml");
    path
}

#[test]
fn feature_dev_has_eight_phases() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;

    let expected: &[&str] = &[
        "design",
        "test-scenarios",
        "plan",
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
