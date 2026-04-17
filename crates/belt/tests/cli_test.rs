#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use assert_cmd::Command;
use tempfile::TempDir;

fn write_yaml(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, content).expect("write");
    path
}

/// scenario: belt-lint-valid-pipeline-ok
#[test]
fn lint_valid_pipeline_exits_zero() {
    let dir = TempDir::new().unwrap();
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "cargo build"
"#,
    );

    Command::cargo_bin("belt")
        .unwrap()
        .arg("lint")
        .arg(path.to_str().unwrap())
        .assert()
        .success()
        .stderr(predicates::str::contains("ok"));
}

/// scenario: belt-lint-duplicate-phase-id-detected
#[test]
fn lint_invalid_pipeline_exits_one() {
    let dir = TempDir::new().unwrap();
    let path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
  - id: build
    description: "Duplicate"
"#,
    );

    Command::cargo_bin("belt")
        .unwrap()
        .arg("lint")
        .arg(path.to_str().unwrap())
        .assert()
        .code(1)
        .stderr(predicates::str::contains("duplicate"));
}

/// scenario: belt-lint-nonexistent-file-rejected
#[test]
fn lint_nonexistent_file_exits_one() {
    Command::cargo_bin("belt")
        .unwrap()
        .arg("lint")
        .arg("/nonexistent/pipeline.yml")
        .assert()
        .code(1);
}

/// scenario: belt-lint-config-resolves-pipeline-file
#[test]
fn lint_with_config_resolves_pipeline() {
    let dir = TempDir::new().unwrap();

    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "cargo build"
"#,
    );

    std::fs::write(dir.path().join("belt.toml"), r#"pipeline = "pipeline.yml""#).unwrap();

    Command::cargo_bin("belt")
        .unwrap()
        .arg("--config")
        .arg(dir.path().join("belt.toml").to_str().unwrap())
        .arg("lint")
        .assert()
        .success()
        .stderr(predicates::str::contains("ok"));
}

/// scenario: belt-lint-config-and-positional-mutually-exclusive
#[test]
fn lint_config_and_positional_file_errors() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("belt.toml"), r#"pipeline = "pipeline.yml""#).unwrap();

    Command::cargo_bin("belt")
        .unwrap()
        .arg("--config")
        .arg(dir.path().join("belt.toml").to_str().unwrap())
        .arg("lint")
        .arg("pipeline.yml")
        .assert()
        .code(1);
}
