//! Integration tests for `belt_core::pipeline::model`.
//!
//! Test files are separate compilation units, so the `cfg_attr(test, allow(...))`
//! in `lib.rs` does NOT apply. Allow the panic-adjacent lints locally for tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use belt_core::pipeline::model::{Pipeline, PipelineKind};
use belt_core::yaml;

#[test]
fn minimal_pipeline_parses() {
    let yaml_text = r"
kind: pipeline
name: feature-dev
version: 4
imports: []
phases:
  - id: design
    confirm: after
";
    let pipeline: Pipeline = yaml::parse(yaml_text).expect("parse minimal pipeline");
    assert!(matches!(pipeline.kind, PipelineKind::Pipeline));
    assert_eq!(pipeline.name, "feature-dev");
    assert_eq!(pipeline.version, 4);
    assert_eq!(pipeline.phases.len(), 1);
    assert_eq!(pipeline.phases[0].id, "design");
}

#[test]
fn pipeline_with_artifacts_and_flags_parses() {
    let yaml_text = r#"
kind: pipeline
name: feature-dev
version: 4
imports:
  - rules/recipes/audit-gate.yml
flags:
  "--linear":
    type: bool
    default: false
artifacts:
  spec_file:
    type: file
    pattern: "docs/specs/*.md"
    produced_by: design
    consumed_by: [plan]
phases:
  - id: design
  - id: plan
"#;
    let pipeline: Pipeline = yaml::parse(yaml_text).expect("parse pipeline");
    assert_eq!(pipeline.imports.len(), 1);
    assert!(pipeline.flags.contains_key("--linear"));
    assert!(pipeline.artifacts.contains_key("spec_file"));
    let spec = &pipeline.artifacts["spec_file"];
    assert_eq!(spec.produced_by.as_deref(), Some("design"));
    assert_eq!(spec.consumed_by, vec!["plan"]);
}
