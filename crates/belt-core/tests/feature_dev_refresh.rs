//! Integration tests for the refreshed feature-dev pipeline (8 phases).

use std::path::PathBuf;

use belt_core::{error::BeltError, expander::expand_pipeline, parser::parse_pipeline};

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
