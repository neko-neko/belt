use belt_core::lint::{Severity, lint_pipeline};
use std::io::Write;
use tempfile::TempDir;

/// Helper: write a file inside the given directory and return its path.
fn write_yaml(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let mut f = std::fs::File::create(&path).expect("failed to create file");
    f.write_all(content.as_bytes())
        .expect("failed to write file");
    path
}

#[test]
fn lint_detects_duplicate_phase_ids() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: dup
version: 1
phases:
  - id: build
    description: "Build 1"
  - id: build
    description: "Build 2"
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint_pipeline should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.iter().any(|d| d.message.contains("duplicate")),
        "expected a diagnostic containing 'duplicate', got: {errors:?}",
    );
}

#[test]
fn lint_detects_invalid_regate_target() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad-regate
version: 1
phases:
  - id: build
    description: "Build the project"
    regate:
      - nonexistent
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint_pipeline should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.iter().any(|d| d.message.contains("nonexistent")),
        "expected a diagnostic mentioning 'nonexistent', got: {errors:?}",
    );
}

#[test]
fn lint_detects_undefined_arg_in_when() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad-when
version: 1
args:
  smoke:
    type: bool
    default: false
phases:
  - id: build
    description: "Build the project"
    when: "args.nonexistent"
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint_pipeline should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.iter().any(|d| d.message.contains("nonexistent")),
        "expected a diagnostic mentioning 'nonexistent', got: {errors:?}",
    );
}

#[test]
fn clean_pipeline_passes_lint() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: clean
version: 1
args:
  smoke:
    type: bool
    default: false
phases:
  - id: build
    description: "Build the project"
    gate:
      - cmd: "cargo build"
  - id: test
    description: "Run tests"
    when: "args.smoke"
    regate:
      - build
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint_pipeline should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "expected no Error diagnostics, got: {errors:?}",
    );
}

#[test]
fn lint_detects_missing_invoke_pipeline_file() {
    let dir = TempDir::new().expect("failed to create tempdir");
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad-invoke-pipeline
version: 1
phases:
  - id: sub
    description: "sub"
    invoke:
      pipeline: ./nonexistent.yml
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.iter().any(|d| d.message.contains("nonexistent.yml")
            && (d.message.contains("invoke") || d.message.contains("pipeline"))),
        "expected diagnostic mentioning 'nonexistent.yml' and invoke/pipeline, got: {errors:?}"
    );
}

#[test]
fn lint_accepts_valid_invoke_pipeline_path() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Write a valid sub-pipeline file first.
    write_yaml(
        &dir,
        "sub.yml",
        r#"
name: sub
version: 1
phases:
  - id: run
    description: "sub run"
"#,
    );

    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: good-invoke-pipeline
version: 1
phases:
  - id: s
    invoke:
      pipeline: ./sub.yml
"#,
    );

    let diagnostics = lint_pipeline(&path).expect("lint should succeed");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "expected no errors for valid invoke pipeline path, got: {errors:?}"
    );
}
