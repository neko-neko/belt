use belt_core::error::BeltError;
use belt_core::parser::{parse_gate_definition, parse_pipeline};
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

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

#[test]
fn parse_nonexistent_file_returns_error() {
    let result = parse_pipeline(Path::new("/nonexistent/pipeline.yml"));
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        BeltError::FileNotFound { .. }
    ));
}
