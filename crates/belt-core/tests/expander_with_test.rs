// Integration test: parent pipeline with a renamed `with` entry produces
// expanded sub-phases whose `args.X` references point at the parent's
// argument names. Pedantic `expect_used`/`panic` lints are allowed file-wide
// because clear panic-on-mismatch assertions are the plan-specified form.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::match_wildcard_for_single_variants,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use belt_core::expander::expand_pipeline;
use belt_core::model::{Invoker, IterationsSpec};
use std::fs;
use tempfile::tempdir;

#[test]
fn parent_with_rename_rewrites_sub_phase_iterations_template() {
    let dir = tempdir().expect("tempdir");
    let parent_path = dir.path().join("parent.yml");
    let sub_path = dir.path().join("custom-review.yml");

    fs::write(
        &sub_path,
        r#"
name: custom-review
version: 1
args:
  count: { type: number, default: 1 }
phases:
  - id: vote
    description: "Cast votes"
    invoke:
      agents: [v1, v2]
      iterations: "args.count"
"#,
    )
    .expect("write sub");

    fs::write(
        &parent_path,
        r#"
name: parent
version: 1
args:
  iterations: { type: number, default: 3 }
phases:
  - id: review
    invoke:
      pipeline: ./custom-review.yml
      with:
        count: "args.iterations"
"#,
    )
    .expect("write parent");

    let expanded = expand_pipeline(&parent_path).expect("expand");
    assert_eq!(expanded.len(), 1);
    assert_eq!(expanded[0].id, "review/vote");
    match &expanded[0].invoke {
        Some(Invoker::Agents { iterations, .. }) => match iterations {
            IterationsSpec::Template(s) => assert_eq!(s, "args.iterations"),
            other => panic!("expected Template, got {other:?}"),
        },
        other => panic!("expected Agents invoke, got {other:?}"),
    }
}
