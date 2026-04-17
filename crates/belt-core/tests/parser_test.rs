#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use belt_core::error::BeltError;
use belt_core::parser::{parse_gate_definition, parse_pipeline};
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

/// scenario: belt-core-parser-parses-valid-pipeline-yaml
#[test]
fn parse_pipeline_from_file() {
    let mut f = NamedTempFile::new().expect("failed to create temp file");
    write!(
        f,
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "cargo build"
"#
    )
    .expect("failed to write temp file");

    let pipeline = parse_pipeline(f.path()).expect("parse_pipeline should succeed");
    assert_eq!(pipeline.name, "test");
    assert_eq!(pipeline.phases.len(), 1);
}

/// scenario: belt-core-parser-parses-valid-gate-definition-yaml
#[test]
fn parse_gate_def_from_file() {
    let mut f = NamedTempFile::new().expect("failed to create temp file");
    write!(
        f,
        r#"
name: rust-build
description: "Build checks"
inputs:
  scope:
    type: string
    default: "--workspace"
checks:
  - cmd: "cargo build ${{scope}}"
  - git_clean: true
"#
    )
    .expect("failed to write temp file");

    let gate_def = parse_gate_definition(f.path()).expect("parse_gate_definition should succeed");
    assert_eq!(gate_def.name, "rust-build");
    assert_eq!(gate_def.checks.len(), 2);
}

/// scenario: belt-core-parser-nonexistent-file-yields-file-not-found
#[test]
fn parse_nonexistent_file_returns_error() {
    let result = parse_pipeline(Path::new("/nonexistent/pipeline.yml"));
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        BeltError::FileNotFound { .. }
    ));
}

/// BELT-32 integration: a complete pipeline exercising Invoker, Artifact,
/// `ArtifactRef`, and `ValidationSource` together. Verifies parse + lint-clean
/// on a valid example and parses round-trip correctly.
///
/// scenario: belt-core-parser-parses-belt32-pipeline-with-invoker-artifact-validation-source
#[test]
fn belt32_full_pipeline_with_all_new_types() {
    use belt_core::lint::{Severity, lint_pipeline};
    use belt_core::model::{ArtifactRef, Invoker, Pipeline, ValidationSource};
    use belt_core::parser::parse_pipeline;
    use std::io::Write;
    use tempfile::TempDir;

    let dir = TempDir::new().expect("tempdir");

    // A criteria file for the validate file-ref.
    let criteria_path = dir.path().join("criteria.md");
    let mut f = std::fs::File::create(&criteria_path).expect("create criteria");
    f.write_all(b"# Criteria\n- C1: placeholder\n")
        .expect("write criteria");

    // A sub-pipeline for Invoker::Pipeline variant.
    let sub_path = dir.path().join("review.yml");
    let mut f = std::fs::File::create(&sub_path).expect("create sub");
    f.write_all(
        br#"
name: review
version: 1
phases:
  - id: vote
    description: "vote on findings"
"#,
    )
    .expect("write sub");

    // Main pipeline with all new types.
    let pipeline_path = dir.path().join("pipeline.yml");
    let mut f = std::fs::File::create(&pipeline_path).expect("create pipeline");
    f.write_all(
        br#"
name: belt32-full
version: 1
phases:
  - id: design
    description: "Design"
    invoke:
      skill: /brainstorming
      args:
        swarm: true
    produces:
      - name: design_doc
        path: "docs/plans/*-design.md"
        description: "Brainstormed design"
    validate:
      - file: ./criteria.md
      - "inline manual check"
    confirm: true

  - id: spec-review
    description: "Spec review sub-pipeline"
    invoke:
      pipeline: ./review.yml
    consumes:
      - design_doc
    produces:
      - name: review_findings
        path: "{output_dir}/findings.json"

  - id: execute
    description: "Execute"
    invoke:
      skill: /subagent-driven-development
    consumes:
      - design_doc
      - name: review_findings
        from: spec-review
"#,
    )
    .expect("write pipeline");

    // 1. Parse succeeds.
    let pipeline: Pipeline = parse_pipeline(&pipeline_path).expect("parse should succeed");
    assert_eq!(pipeline.phases.len(), 3);

    // 2. Design phase: Invoker::Skill, produces, mixed validate.
    let design = &pipeline.phases[0];
    assert_eq!(design.id, "design");
    assert!(matches!(
        design.invoke,
        Some(Invoker::Skill { ref skill, .. }) if skill == "/brainstorming"
    ));
    assert_eq!(design.produces.len(), 1);
    assert_eq!(design.produces[0].name, "design_doc");
    assert_eq!(design.validate.len(), 2);
    assert!(matches!(
        &design.validate[0],
        ValidationSource::File { file } if file == "./criteria.md"
    ));
    assert!(matches!(
        &design.validate[1],
        ValidationSource::Inline(s) if s == "inline manual check"
    ));
    assert!(design.confirm);

    // 3. Spec-review phase: Invoker::Pipeline, consumes, produces.
    let spec_review = &pipeline.phases[1];
    assert_eq!(spec_review.id, "spec-review");
    assert!(matches!(
        spec_review.invoke,
        Some(Invoker::Pipeline { ref pipeline, .. }) if pipeline == "./review.yml"
    ));
    assert_eq!(spec_review.consumes.len(), 1);
    assert!(matches!(
        &spec_review.consumes[0],
        ArtifactRef::Named(s) if s == "design_doc"
    ));
    assert_eq!(spec_review.produces.len(), 1);
    assert_eq!(spec_review.produces[0].name, "review_findings");

    // 4. Execute phase: Invoker::Skill, mixed consumes (Named + Qualified).
    let execute = &pipeline.phases[2];
    assert_eq!(execute.id, "execute");
    assert!(matches!(
        execute.invoke,
        Some(Invoker::Skill { ref skill, .. }) if skill == "/subagent-driven-development"
    ));
    assert_eq!(execute.consumes.len(), 2);
    assert!(matches!(
        &execute.consumes[0],
        ArtifactRef::Named(s) if s == "design_doc"
    ));
    assert!(matches!(
        &execute.consumes[1],
        ArtifactRef::Qualified { name, from }
            if name == "review_findings" && from == "spec-review"
    ));

    // 5. Lint: no errors expected.
    let diagnostics = lint_pipeline(&pipeline_path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "expected no lint errors, got: {errors:?}"
    );
}
