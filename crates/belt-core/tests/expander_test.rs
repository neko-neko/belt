#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

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

/// `invoke: { pipeline: ... }` phases are expanded to namespaced IDs, and
/// leaf phases are preserved.
///
/// scenario: belt-core-expander-invoke-pipeline-to-namespaced-ids
#[test]
fn expand_invoke_pipeline_phase_to_namespaced_ids() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Sub-pipeline with 2 phases. Each sub-phase needs a description AND
    // at least one of: invoke, gate, validate, confirm (spec DD-8).
    write_yaml(
        &dir,
        "spec-review.yml",
        r#"
name: spec-review
version: 1
phases:
  - id: review
    description: "Review the spec"
    gate:
      - cmd: "true"
  - id: fix
    description: "Fix spec issues"
    gate:
      - cmd: "true"
"#,
    );

    // Main pipeline: one sub-pipeline phase + one leaf phase
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: main
version: 1
phases:
  - id: spec-review
    invoke:
      pipeline: spec-review.yml
  - id: build
    description: "Build the project"
    gate:
      - cmd: "true"
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
///
/// scenario: belt-core-expander-parent-gate-regate-inherited-by-last-sub-phase-only
#[test]
fn parent_gate_appended_to_last_sub_phase() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Sub-pipeline with 2 phases, no parent-provided gate (each sub-phase
    // has its own `cmd: true` gate to satisfy the empty-phase lint rule).
    write_yaml(
        &dir,
        "sub.yml",
        r#"
name: sub
version: 1
phases:
  - id: a
    description: "Phase A"
    gate:
      - cmd: "true"
  - id: b
    description: "Phase B"
    gate:
      - cmd: "true"
"#,
    );

    // Main pipeline with gate + regate on the sub-pipeline phase.
    // The "execute" phase referenced by regate must exist as a phase id.
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: main
version: 1
phases:
  - id: execute
    description: "Execute"
    gate:
      - cmd: "true"
  - id: parent
    invoke:
      pipeline: sub.yml
    gate:
      - cmd: "cargo test"
    regate:
      - execute
"#,
    );

    let expanded = expand_pipeline(&pipeline_path).expect("expand should succeed");
    assert_eq!(expanded.len(), 3);

    // expanded[0] is the leaf "execute" phase.
    assert_eq!(expanded[0].id, "execute");

    // First sub-phase (parent/a): only its own gate, no parent gate/regate.
    assert_eq!(expanded[1].id, "parent/a");
    assert_eq!(expanded[1].gate.len(), 1);
    assert!(expanded[1].regate.is_empty());

    // Last sub-phase (parent/b): parent gate and regate appended.
    assert_eq!(expanded[2].id, "parent/b");
    assert_eq!(expanded[2].gate.len(), 2);
    assert_eq!(expanded[2].regate, vec!["execute"]);
}

/// Parent `when` propagates to all sub-phases that lack their own.
///
/// scenario: belt-core-expander-parent-when-propagates-to-all-sub-phases
#[test]
fn when_propagated_to_sub_phases() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Sub-pipeline: first phase has its own when, second has none. Each
    // sub-phase has at least one gate to satisfy the empty-phase lint.
    write_yaml(
        &dir,
        "sub.yml",
        r#"
name: sub
version: 1
phases:
  - id: alpha
    description: "Alpha"
    gate:
      - cmd: "true"
  - id: beta
    description: "Beta"
    gate:
      - cmd: "true"
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
    invoke:
      pipeline: sub.yml
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
///
/// scenario: belt-core-expander-leaf-phase-without-description-yields-invalid-pipeline
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

/// A phase using `invoke: { pipeline: "./sub.yml" }` expands into the
/// sub-pipeline's phases with namespaced IDs.
///
/// scenario: belt-core-expander-invoke-pipeline-to-namespaced-ids
#[test]
fn expand_invoke_pipeline_variant() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Sub-pipeline with 2 phases.
    write_yaml(
        &dir,
        "sub.yml",
        r#"
name: sub
version: 1
phases:
  - id: work
    description: "sub work"
  - id: audit
    description: "sub audit"
"#,
    );

    // Top-level pipeline using invoke: { pipeline: ... }.
    let top_path = write_yaml(
        &dir,
        "pipeline.yml",
        r"
name: top
version: 1
phases:
  - id: review
    invoke:
      pipeline: ./sub.yml
",
    );

    let expanded = expand_pipeline(&top_path).expect("expand should succeed");

    // Expect 2 phases, namespaced `review/work` and `review/audit`.
    assert_eq!(expanded.len(), 2);
    assert_eq!(expanded[0].id, "review/work");
    assert_eq!(expanded[1].id, "review/audit");
}
