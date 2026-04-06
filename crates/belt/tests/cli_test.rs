use assert_cmd::Command;
use tempfile::TempDir;

fn write_yaml(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, content).expect("write");
    path
}

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

#[test]
fn lint_nonexistent_file_exits_one() {
    Command::cargo_bin("belt")
        .unwrap()
        .arg("lint")
        .arg("/nonexistent/pipeline.yml")
        .assert()
        .code(1);
}
