// Integration test: parent pipeline with a renamed `with` entry produces
// expanded sub-phases whose `args.X` references point at the parent's
// argument names. Pedantic `expect_used`/`panic` lints are allowed file-wide
// because clear panic-on-mismatch assertions are the plan-specified form.
//
// The original `parent_with_rename_rewrites_sub_phase_iterations_template`
// test was removed on 2026-04-16 together with `Invoker::Agents` /
// `IterationsSpec` (see docs/specs/2026-04-16-review-skills-subagent-
// boundary-design.md). The remaining expander with-merge coverage lives in
// `crates/belt-core/src/expander.rs`'s unit test module (Skill / Pipeline
// args rewriting).
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::match_wildcard_for_single_variants,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use belt_core::expander::expand_pipeline;
use belt_core::parser::parse_pipeline;
use tempfile::TempDir;

mod common;
use common::helpers::write_yaml;

/// Integration coverage for `expand_pipeline` substituting `args.<name>`
/// templates inside a sub-phase's `invoke.args` map from the parent's
/// `invoke.pipeline.with` map. Only the `args` map is substituted — the
/// `skill` field on `Invoker::Skill` is intentionally not rewritten
/// (matches `substitute_in_invoker` in expander.rs). The YAML literal uses
/// `invoke: { pipeline: ..., with: {...} }` which is the supported MVP
/// spelling (the `uses:` spelling is sub-pipeline author syntax and lives
/// only inside sub-pipeline YAMLs, not the parent).
///
/// scenario: belt-core-expander-with-string-substitution-integration
#[test]
fn expand_pipeline_with_string_substitution_end_to_end() {
    let dir = TempDir::new().expect("tempdir");
    write_yaml(
        &dir,
        "sub.yml",
        r#"
name: sub
version: 1
inputs:
  skill:
    type: string
    default: "/default"
phases:
  - id: step
    description: "run"
    invoke:
      skill: "/runner"
      args:
        name: "args.skill"
    gate:
      - cmd: "true"
"#,
    );
    let parent = write_yaml(
        &dir,
        "parent.yml",
        r#"
name: parent
version: 1
phases:
  - id: phase
    invoke:
      pipeline: sub.yml
      with:
        skill: "/custom"
"#,
    );

    // Round-trip through `parse_pipeline` first so the test exercises the
    // full declarative path (parse → expand), not just the expander.
    let _ = parse_pipeline(&parent).expect("parse parent");
    let expanded = expand_pipeline(&parent).expect("expand");

    // Exactly one expanded phase whose namespaced id carries the sub-phase.
    let phase = expanded
        .iter()
        .find(|p| p.id == "phase/step")
        .expect("sub phase phase/step");
    let rendered = format!("{:?}", phase.invoke);
    assert!(
        rendered.contains("/custom"),
        "expected skill arg override propagated into invoke.args: {rendered}"
    );
}

/// Covers bool + null type preservation during `with` substitution — the
/// two non-string scalar variants that the YAML surface exposes. The
/// substitute path runs per-value: a `Value::String("args.flag")` is
/// replaced by the underlying `Value::Bool(true)` without being
/// stringified.
///
/// scenario: belt-core-expander-with-bool-and-null-substitution-preserves-types
#[test]
fn expand_pipeline_with_bool_and_null_substitution_preserves_types() {
    let dir = TempDir::new().expect("tempdir");
    write_yaml(
        &dir,
        "sub.yml",
        r#"
name: sub
version: 1
inputs:
  flag:
    type: bool
    default: false
  opt:
    type: string
phases:
  - id: step
    description: "run"
    invoke:
      skill: "/runner"
      args:
        flag: "args.flag"
        opt: "args.opt"
    gate:
      - cmd: "true"
"#,
    );
    let parent = write_yaml(
        &dir,
        "parent.yml",
        r"
name: parent
version: 1
phases:
  - id: phase
    invoke:
      pipeline: sub.yml
      with:
        flag: true
        opt: null
",
    );

    let expanded = expand_pipeline(&parent).expect("expand");
    let phase = expanded
        .iter()
        .find(|p| p.id == "phase/step")
        .expect("sub phase phase/step");
    let rendered = format!("{:?}", phase.invoke);
    // `serde_json::Value::Bool(true)` Debug renders as `Bool(true)`;
    // `serde_json::Value::Null` renders as `Null`. We assert on the
    // variant name (not the scalar word) so the test cannot be satisfied
    // by the un-substituted template `String("args.flag")`.
    assert!(
        rendered.contains("Bool(true)"),
        "expected Value::Bool(true) preserved in invoke.args Debug: {rendered}"
    );
    assert!(
        rendered.contains("Null"),
        "expected Value::Null preserved in invoke.args Debug: {rendered}"
    );
    // Sanity: the un-substituted templates must no longer appear — if
    // they do, substitution did not happen.
    assert!(
        !rendered.contains("args.flag"),
        "args.flag template was not substituted: {rendered}"
    );
    assert!(
        !rendered.contains("args.opt"),
        "args.opt template was not substituted: {rendered}"
    );
}

/// Parent-scope args MUST NOT be rewritten by sub-pipeline `with`
/// substitution. The parent has a sibling leaf phase (`parent-leaf`) whose
/// `invoke.args` map references `args.name`. A sibling phase (`call-sub`)
/// invokes a sub-pipeline with `with: { name: "/sub-override" }`. After
/// expansion, the substitution applied inside the sub-pipeline MUST NOT
/// leak to the parent-leaf's `invoke.args` map. The sub-phase's own
/// `invoke.args` gets substituted (sub scope) while the parent-leaf keeps
/// its original `args.name` template intact (parent scope).
///
/// This probes the parent-scope isolation invariant directly via the
/// `Vec<ExpandedPhase>` output of `expand_pipeline`. An earlier revision
/// of this test inspected `pipeline.args["name"].default`, which Rust
/// ownership alone guaranteed because `expand_pipeline` takes `&Path` and
/// cannot physically mutate the caller's `pipeline` binding. That shape
/// would pass even under a hypothetically broken expander that rewrote
/// parent values, so it did not test the spec intent.
///
/// scenario: belt-core-expander-with-parent-scope-not-rewritten-by-sub-substitution
#[test]
fn expand_pipeline_parent_scope_not_rewritten_by_sub_substitution() {
    let dir = TempDir::new().expect("tempdir");
    write_yaml(
        &dir,
        "sub.yml",
        r#"
name: sub
version: 1
inputs:
  name:
    type: string
    default: "/sub-default"
phases:
  - id: step
    description: "run"
    invoke:
      skill: "/runner"
      args:
        resolved: "args.name"
    gate:
      - cmd: "true"
"#,
    );
    let parent = write_yaml(
        &dir,
        "parent.yml",
        r#"
name: parent
version: 1
args:
  name:
    type: string
    default: "/parent-name"
phases:
  - id: call-sub
    invoke:
      pipeline: sub.yml
      with:
        name: "/sub-override"
  - id: parent-leaf
    description: "parent scope leaf"
    invoke:
      skill: "/runner"
      args:
        name: "args.name"
    gate:
      - cmd: "true"
"#,
    );

    // Round-trip parse for the full declarative path.
    let _ = parse_pipeline(&parent).expect("parse parent");
    let expanded = expand_pipeline(&parent).expect("expand");

    // The parent-leaf sibling must not be touched by the sub-pipeline's
    // `with` substitution — its `invoke.args.name` template must stay
    // literal, and must NOT contain the sub-scope override value.
    let parent_leaf = expanded
        .iter()
        .find(|p| p.id == "parent-leaf")
        .expect("parent-leaf phase present in expansion");
    let parent_rendered = format!("{:?}", parent_leaf.invoke);
    assert!(
        parent_rendered.contains("args.name"),
        "parent-leaf invoke.args template was rewritten (expected literal 'args.name'): {parent_rendered}"
    );
    assert!(
        !parent_rendered.contains("/sub-override"),
        "parent-leaf leaked sub-pipeline with override into parent scope: {parent_rendered}"
    );

    // The sub-phase itself DID receive the with-substitution — this
    // confirms substitution happened in sub scope (positive control).
    let sub_phase = expanded
        .iter()
        .find(|p| p.id == "call-sub/step")
        .expect("sub phase call-sub/step present in expansion");
    let sub_rendered = format!("{:?}", sub_phase.invoke);
    assert!(
        sub_rendered.contains("/sub-override"),
        "sub-phase did not receive with substitution (expected '/sub-override'): {sub_rendered}"
    );
    assert!(
        !sub_rendered.contains("args.name"),
        "sub-phase template was not substituted (still contains 'args.name'): {sub_rendered}"
    );
}
