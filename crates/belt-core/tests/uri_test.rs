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

/// scenario: belt-core-uri-unknown-selector-rejected
#[test]
fn unknown_selector_is_rejected() {
    let err = BeltUri::parse("belt://unknown/pipeline/path.md").expect_err("should reject");
    assert!(matches!(err, UriParseError::UnknownSelector { .. }));
}

/// scenario: belt-core-uri-empty-pipeline-rejected
#[test]
fn empty_pipeline_is_rejected() {
    let err = BeltUri::parse("belt://latest//notes/x.md").expect_err("should reject");
    assert!(matches!(err, UriParseError::EmptyPipeline { .. }));
}

/// scenario: belt-core-uri-empty-run-id-rejected
#[test]
fn empty_run_id_is_rejected() {
    let err = BeltUri::parse("belt://run//notes/x.md").expect_err("should reject");
    assert!(matches!(err, UriParseError::EmptyRunId { .. }));
}

/// scenario: belt-core-uri-empty-path-rejected
#[test]
fn empty_path_is_rejected() {
    let err = BeltUri::parse("belt://latest/feature-dev/").expect_err("should reject");
    assert!(matches!(err, UriParseError::EmptyPath { .. }));
}

/// scenario: belt-core-uri-absolute-path-rejected
#[test]
fn absolute_path_is_rejected() {
    let err = BeltUri::parse("belt://latest/feature-dev//notes/x.md").expect_err("should reject");
    assert!(matches!(err, UriParseError::PathTraversal { .. }));
}

/// scenario: belt-core-uri-workspace-missing-latest-rejected
#[test]
fn workspace_missing_latest_is_rejected() {
    let err = BeltUri::parse("belt://workspace/develop/notlatest/pipeline/path.md")
        .expect_err("should reject");
    assert!(matches!(err, UriParseError::Malformed { .. }));
}

/// scenario: belt-core-uri-to-string-roundtrip-all-variants
#[test]
fn to_string_roundtrip_all_variants() {
    let inputs = [
        "belt://run/01932000-0000-7000-8000-000000000001/notes/x.md",
        "belt://latest/feature-dev/notes/y.md",
        "belt://workspace/develop/latest/feature-dev/z.md",
    ];
    for s in inputs {
        let parsed = BeltUri::parse(s).expect("parse ok");
        let restr = parsed.to_string();
        let reparsed = BeltUri::parse(&restr).expect("reparse ok");
        assert_eq!(parsed, reparsed, "roundtrip mismatch: {s} -> {restr}");
    }
}
