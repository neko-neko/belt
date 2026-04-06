use belt_core::error::BeltError;
use belt_core::expander::expand_pipeline;
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

/// `uses:` phases are expanded to namespaced IDs, and leaf phases are preserved.
#[test]
fn expand_uses_phase_to_namespaced_ids() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Sub-pipeline with 2 phases
    write_yaml(
        &dir,
        "spec-review.yml",
        r#"
name: spec-review
version: 1
phases:
  - id: review
    description: "Review the spec"
  - id: fix
    description: "Fix spec issues"
"#,
    );

    // Main pipeline: one uses-phase + one leaf phase
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: main
version: 1
phases:
  - id: spec-review
    uses: spec-review.yml
  - id: build
    description: "Build the project"
"#,
    );

    let expanded = expand_pipeline(&pipeline_path).expect("expand should succeed");
    assert_eq!(expanded.len(), 3);
    assert_eq!(expanded[0].id, "spec-review/review");
    assert_eq!(expanded[1].id, "spec-review/fix");
    assert_eq!(expanded[2].id, "build");
    assert_eq!(expanded[2].description, "Build the project");
}

/// Parent gate/regate are appended only to the LAST sub-phase.
#[test]
fn parent_gate_appended_to_last_sub_phase() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Sub-pipeline with 2 phases, no gates
    write_yaml(
        &dir,
        "sub.yml",
        r#"
name: sub
version: 1
phases:
  - id: a
    description: "Phase A"
  - id: b
    description: "Phase B"
"#,
    );

    // Main pipeline with gate + regate on the uses-phase
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: main
version: 1
phases:
  - id: parent
    uses: sub.yml
    gate:
      - cmd: "cargo test"
    regate:
      - execute
"#,
    );

    let expanded = expand_pipeline(&pipeline_path).expect("expand should succeed");
    assert_eq!(expanded.len(), 2);

    // First sub-phase (a): NO parent gate/regate
    assert!(expanded[0].gate.is_empty());
    assert!(expanded[0].regate.is_empty());

    // Last sub-phase (b): parent gate and regate appended
    assert_eq!(expanded[1].gate.len(), 1);
    assert_eq!(expanded[1].regate, vec!["execute"]);
}

/// Parent `when` propagates to all sub-phases that lack their own.
#[test]
fn when_propagated_to_sub_phases() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Sub-pipeline: first phase has its own when, second has none
    write_yaml(
        &dir,
        "sub.yml",
        r#"
name: sub
version: 1
phases:
  - id: alpha
    description: "Alpha"
  - id: beta
    description: "Beta"
"#,
    );

    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: main
version: 1
phases:
  - id: gated
    uses: sub.yml
    when: "args.smoke"
"#,
    );

    let expanded = expand_pipeline(&pipeline_path).expect("expand should succeed");
    assert_eq!(expanded.len(), 2);

    // Both sub-phases inherit the parent when
    assert_eq!(expanded[0].when.as_deref(), Some("args.smoke"));
    assert_eq!(expanded[1].when.as_deref(), Some("args.smoke"));
}

/// A leaf phase without a description returns `InvalidPipeline`.
#[test]
fn leaf_phase_without_description_returns_error() {
    let dir = TempDir::new().expect("failed to create tempdir");

    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: bad
version: 1
phases:
  - id: oops
    gate:
      - cmd: "echo hi"
"#,
    );

    let result = expand_pipeline(&pipeline_path);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        BeltError::InvalidPipeline { .. }
    ));
}
