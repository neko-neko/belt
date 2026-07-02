#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use belt_core::error::BeltError;
use belt_core::expander::expand_pipeline;
use tempfile::TempDir;

mod common;
use common::helpers::write_yaml;

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

/// A standalone Pipeline-format YAML (with a top-level `args:` map) can be
/// referenced via `invoke: { pipeline: ... }`: parse_sub_pipeline ignores
/// the unknown `args` field. This is the dual-format guarantee that lets
/// one file serve both `belt-agent init` and sub-pipeline composition.
///
/// scenario: belt-core-expander-pipeline-format-accepted-as-sub-pipeline
#[test]
fn pipeline_format_yaml_accepted_as_sub_pipeline() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Pipeline format: has `args:` (unknown to SubPipeline) and description.
    write_yaml(
        &dir,
        "standalone.yml",
        r#"
name: standalone
version: 1
description: "Standalone pipeline also usable as a sub-pipeline"
args:
  e2e:
    type: bool
    default: false
    description: "flag"
phases:
  - id: work
    description: "Do the work"
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
  - id: stage
    invoke:
      pipeline: standalone.yml
"#,
    );

    let expanded = expand_pipeline(&pipeline_path).expect("dual-format expand should succeed");
    assert_eq!(expanded.len(), 1);
    assert_eq!(expanded[0].id, "stage/work");
}

/// Nested `invoke: { pipeline: ... }` references expand recursively with
/// `{parent}/{sub}/{subsub}` namespaced IDs, and the outermost parent's
/// gate is appended to the LAST innermost leaf.
///
/// scenario: belt-core-expander-nested-sub-pipeline-expands-recursively
#[test]
fn nested_sub_pipeline_expands_recursively() {
    let dir = TempDir::new().expect("failed to create tempdir");

    write_yaml(
        &dir,
        "inner.yml",
        r#"
name: inner
version: 1
phases:
  - id: monkey-test
    description: "Replay scenarios"
    gate:
      - cmd: "true"
  - id: dogfood
    description: "Exploratory testing"
    gate:
      - cmd: "true"
"#,
    );

    write_yaml(
        &dir,
        "middle.yml",
        r#"
name: middle
version: 1
phases:
  - id: execute
    description: "Implement"
    gate:
      - cmd: "true"
  - id: verify
    invoke:
      pipeline: inner.yml
"#,
    );

    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: main
version: 1
phases:
  - id: build
    invoke:
      pipeline: middle.yml
    gate:
      - git_clean: true
"#,
    );

    let expanded = expand_pipeline(&pipeline_path).expect("nested expand should succeed");
    let ids: Vec<&str> = expanded.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "build/execute",
            "build/verify/monkey-test",
            "build/verify/dogfood"
        ]
    );
    // Outer parent gate lands on the LAST innermost leaf only.
    assert_eq!(expanded[0].gate.len(), 1, "inner leaves keep own gates");
    assert_eq!(expanded[1].gate.len(), 1);
    assert_eq!(
        expanded[2].gate.len(),
        2,
        "last leaf inherits the outer parent gate appended"
    );
}

/// A cyclic sub-pipeline reference (a.yml -> b.yml -> a.yml) is rejected
/// with InvalidPipeline instead of infinite recursion.
///
/// scenario: belt-core-expander-cyclic-reference-yields-invalid-pipeline
#[test]
fn cyclic_sub_pipeline_reference_yields_invalid_pipeline() {
    let dir = TempDir::new().expect("failed to create tempdir");

    write_yaml(
        &dir,
        "a.yml",
        r#"
name: a
version: 1
phases:
  - id: to-b
    invoke:
      pipeline: b.yml
"#,
    );
    write_yaml(
        &dir,
        "b.yml",
        r#"
name: b
version: 1
phases:
  - id: back-to-a
    invoke:
      pipeline: a.yml
"#,
    );
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: main
version: 1
phases:
  - id: entry
    invoke:
      pipeline: a.yml
"#,
    );

    let err = expand_pipeline(&pipeline_path).expect_err("cycle must be rejected");
    assert!(
        matches!(err, BeltError::InvalidPipeline { ref message } if message.contains("cyclic")),
        "unexpected error: {err:?}"
    );
}

/// Nesting deeper than 4 sub-pipeline levels is rejected with
/// InvalidPipeline naming the depth limit.
///
/// scenario: belt-core-expander-depth-limit-exceeded-yields-invalid-pipeline
#[test]
fn nesting_beyond_depth_limit_yields_invalid_pipeline() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Chain: pipeline.yml -> d1 -> d2 -> d3 -> d4 -> d5 (leaf). Depth 5 > 4.
    for i in 1..=4 {
        write_yaml(
            &dir,
            &format!("d{i}.yml"),
            &format!(
                r#"
name: d{i}
version: 1
phases:
  - id: next
    invoke:
      pipeline: d{}.yml
"#,
                i + 1
            ),
        );
    }
    write_yaml(
        &dir,
        "d5.yml",
        r#"
name: d5
version: 1
phases:
  - id: leaf
    description: "Bottom"
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
  - id: entry
    invoke:
      pipeline: d1.yml
"#,
    );

    let err = expand_pipeline(&pipeline_path).expect_err("depth must be limited");
    assert!(
        matches!(err, BeltError::InvalidPipeline { ref message } if message.contains("depth")),
        "unexpected error: {err:?}"
    );
}

/// A regate target declared inside a sub-pipeline is renamed into the
/// sub-pipeline's expansion namespace so it points at the expanded id.
///
/// scenario: belt-core-expander-sub-internal-regate-targets-namespaced
#[test]
fn sub_internal_regate_targets_are_namespaced() {
    let dir = TempDir::new().expect("failed to create tempdir");

    write_yaml(
        &dir,
        "stage.yml",
        r#"
name: stage
version: 1
phases:
  - id: execute
    description: "Implement"
    gate:
      - cmd: "true"
  - id: code-review
    description: "Review"
    regate: [execute]
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
  - id: build
    invoke:
      pipeline: stage.yml
"#,
    );

    let expanded = expand_pipeline(&pipeline_path).expect("expand should succeed");
    assert_eq!(expanded[1].id, "build/code-review");
    assert_eq!(
        expanded[1].regate,
        vec!["build/execute".to_string()],
        "sub-internal regate target must be renamed into the expansion namespace"
    );
}

/// The outermost parent `when:` propagates through nested levels to every
/// expanded leaf that does not declare its own when.
///
/// scenario: belt-core-expander-parent-when-propagates-through-nested-levels
#[test]
fn parent_when_propagates_through_nested_levels() {
    let dir = TempDir::new().expect("failed to create tempdir");

    write_yaml(
        &dir,
        "inner.yml",
        r#"
name: inner
version: 1
phases:
  - id: check
    description: "Inner check"
    gate:
      - cmd: "true"
"#,
    );
    write_yaml(
        &dir,
        "middle.yml",
        r#"
name: middle
version: 1
phases:
  - id: deep
    invoke:
      pipeline: inner.yml
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
    when: "args.e2e"
    invoke:
      pipeline: middle.yml
"#,
    );

    let expanded = expand_pipeline(&pipeline_path).expect("expand should succeed");
    assert_eq!(expanded.len(), 1);
    assert_eq!(expanded[0].id, "gated/deep/check");
    assert_eq!(
        expanded[0].when.as_deref(),
        Some("args.e2e"),
        "outer when must reach the innermost leaf"
    );
}
