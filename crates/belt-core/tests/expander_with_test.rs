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
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use belt_core::expander::expand_pipeline;
use belt_core::model::ExpandedPhase;
use tempfile::TempDir;

mod common;
use common::helpers::write_yaml;

/// Assert the rendered `invoke` Debug for `phase_id` contains every string
/// in `must_contain` and none in `must_not_contain`. Lets multi-phase
/// isolation tests stay readable without re-stating the find/format dance
/// once per phase. Tagged `#[track_caller]` so panics point at the test
/// site, not this helper.
#[track_caller]
fn assert_invoke_strings(
    expanded: &[ExpandedPhase],
    phase_id: &str,
    must_contain: &[&str],
    must_not_contain: &[&str],
) {
    let phase = expanded
        .iter()
        .find(|p| p.id == phase_id)
        .unwrap_or_else(|| panic!("phase '{phase_id}' present in expansion"));
    let rendered = format!("{:?}", phase.invoke);
    for needle in must_contain {
        assert!(
            rendered.contains(needle),
            "phase '{phase_id}' invoke missing expected '{needle}': {rendered}"
        );
    }
    for needle in must_not_contain {
        assert!(
            !rendered.contains(needle),
            "phase '{phase_id}' invoke leaked '{needle}': {rendered}"
        );
    }
}

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
/// substitution, and each sub-pipeline call must receive its own scoped
/// substitution without bleeding into a sibling call. The parent has
/// three leaves: `call-sub` invokes `sub.yml` with `name: "/sub-override"`,
/// `call-sub-2` invokes the same sub-pipeline with `name: "/leaf-override"`,
/// and `parent-leaf` is a direct-skill leaf whose `invoke.args` references
/// `args.name`. After expansion the two sub-pipeline calls must each carry
/// only their own override, and the parent-leaf must keep its literal
/// `args.name` template intact.
///
/// This probes the parent-scope isolation invariant directly via the
/// `Vec<ExpandedPhase>` output of `expand_pipeline`. An earlier revision
/// of this test inspected `pipeline.args["name"].default`, which Rust
/// ownership alone guaranteed because `expand_pipeline` takes `&Path` and
/// cannot physically mutate the caller's `pipeline` binding. That shape
/// would pass even under a hypothetically broken expander that rewrote
/// parent values, so it did not test the spec intent. The second
/// sibling sub-pipeline call covers the subtler mutation where a broken
/// expander might share a single substitution buffer across sibling
/// calls — catching it requires two calls with distinct override values.
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
  - id: call-sub-2
    invoke:
      pipeline: sub.yml
      with:
        name: "/leaf-override"
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

    let expanded = expand_pipeline(&parent).expect("expand");

    // parent-leaf: literal `args.name` preserved; neither sibling's override leaks.
    assert_invoke_strings(
        &expanded,
        "parent-leaf",
        &["args.name"],
        &["/sub-override", "/leaf-override"],
    );
    // call-sub/step: receives its own override; nothing from the other sibling
    // and no remaining template — positive control + cross-call isolation.
    assert_invoke_strings(
        &expanded,
        "call-sub/step",
        &["/sub-override"],
        &["args.name", "/leaf-override"],
    );
    // call-sub-2/step: receives its own distinct override — proves per-call
    // scoping rather than a shared substitution buffer.
    assert_invoke_strings(
        &expanded,
        "call-sub-2/step",
        &["/leaf-override"],
        &["args.name", "/sub-override"],
    );
}
