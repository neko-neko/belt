//! Integration test to detect drift between feature-dev and bug-fix shared
//! criteria files (execute.md, code-review.md).
//!
//! After the plugin migration belt-agent cannot resolve cross-plugin paths,
//! so the two files are physically duplicated. This parity test fails fast
//! if they ever diverge.

#![allow(clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

fn workspace_path(rel: &str) -> PathBuf {
    // CARGO_MANIFEST_DIR points at crates/belt-core; walk two levels up to
    // reach the workspace root, then join the plugin-relative path.
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path.push(rel);
    path
}

#[test]
fn execute_criteria_identical_across_feature_dev_and_bug_fix() {
    let fd = fs::read_to_string(workspace_path(
        "plugins/feature-dev/skills/feature-dev/criteria/execute.md",
    ))
    .expect("feature-dev execute.md missing");
    let bf = fs::read_to_string(workspace_path(
        "plugins/bug-fix/skills/bug-fix/criteria/execute.md",
    ))
    .expect("bug-fix execute.md missing");
    assert_eq!(
        fd, bf,
        "execute.md drift: feature-dev and bug-fix must be byte-identical"
    );
}

#[test]
fn code_review_criteria_identical_across_feature_dev_and_bug_fix() {
    let fd = fs::read_to_string(workspace_path(
        "plugins/feature-dev/skills/feature-dev/criteria/code-review.md",
    ))
    .expect("feature-dev code-review.md missing");
    let bf = fs::read_to_string(workspace_path(
        "plugins/bug-fix/skills/bug-fix/criteria/code-review.md",
    ))
    .expect("bug-fix code-review.md missing");
    assert_eq!(
        fd, bf,
        "code-review.md drift: feature-dev and bug-fix must be byte-identical"
    );
}
