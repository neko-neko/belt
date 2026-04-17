#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

//! Integration tests for belt-core `BeltUri` parsing.
//! White-box unit tests for each error variant live in src/uri.rs #[cfg(test)] mod.
//! This file covers black-box behavior of the three selector variants (Run / Latest /
//! `WorkspaceLatest`) and the overall parse contract.

use belt_core::uri::{BeltUri, UriParseError};

/// scenario: belt-core-uri-latest-selector-parses
#[test]
fn latest_selector_parses_with_pipeline_and_path() {
    let u = BeltUri::parse("belt://latest/feature-dev/notes/phase-review.md")
        .expect("valid Latest URI must parse");
    match u {
        BeltUri::Latest { pipeline, path } => {
            assert_eq!(pipeline, "feature-dev");
            assert_eq!(path, "notes/phase-review.md");
        }
        other => panic!("expected BeltUri::Latest, got {other:?}"),
    }
}

/// scenario: belt-core-uri-workspace-latest-selector-parses
#[test]
fn workspace_latest_selector_parses_with_branch_pipeline_path() {
    let u = BeltUri::parse("belt://workspace/develop/latest/feature-dev/notes/phase-review.md")
        .expect("valid WorkspaceLatest URI must parse");
    match u {
        BeltUri::WorkspaceLatest {
            branch,
            pipeline,
            path,
        } => {
            assert_eq!(branch, "develop");
            assert_eq!(pipeline, "feature-dev");
            assert_eq!(path, "notes/phase-review.md");
        }
        other => panic!("expected BeltUri::WorkspaceLatest, got {other:?}"),
    }
}

/// scenario: belt-core-uri-run-selector-parses
#[test]
fn run_selector_parses_with_run_id_and_path() {
    let u = BeltUri::parse("belt://run/01947abc-0000-7000-8000-000000000000/notes/phase-review.md")
        .expect("valid Run URI must parse");
    match u {
        BeltUri::Run { run_id, path } => {
            assert_eq!(run_id, "01947abc-0000-7000-8000-000000000000");
            assert_eq!(path, "notes/phase-review.md");
        }
        other => panic!("expected BeltUri::Run, got {other:?}"),
    }
}

/// scenario: belt-core-uri-missing-scheme-rejected
#[test]
fn non_belt_scheme_is_rejected() {
    let err =
        BeltUri::parse("https://example.com/foo").expect_err("non-belt scheme must be rejected");
    assert!(
        matches!(err, UriParseError::MissingScheme(_)),
        "expected MissingScheme, got {err:?}"
    );
}

/// scenario: belt-core-uri-path-traversal-rejected
#[test]
fn path_traversal_segment_is_rejected() {
    let err = BeltUri::parse("belt://latest/feature-dev/../etc/passwd")
        .expect_err("path traversal must be rejected");
    assert!(
        matches!(err, UriParseError::PathTraversal { .. }),
        "expected PathTraversal, got {err:?}"
    );
}
