//! Integration test locking byte-identity of the `## Filtering` section prefix
//! bullets across the consolidated diff-scope reviewer agent markdown files.
//!
//! Background:
//! - The 2026-07-05 sonnet-lean consolidation
//!   (docs/specs/2026-07-05-sonnet-lean-pipeline-design.md) merged the seven
//!   per-observation reviewer agents into three: spec-reviewer, code-reviewer,
//!   and quality-reviewer.
//! - /belt:code-review still dispatches code-reviewer and quality-reviewer in
//!   parallel and merges their findings, so both agents open their
//!   `## Filtering` section with the same bullet preface (confidence
//!   threshold, occurrence-count folding, no stylistic opinions). Two earlier
//!   drift incidents (Phase B I3 in commit f191a22 and Phase C C-1 in
//!   db73c7d) showed this preface diverges silently without a commit-time
//!   lock.
//!
//! Contract enforced here:
//! - code-review: the first 3 bullets after `## Filtering` are byte-identical
//!   across code-reviewer / quality-reviewer.
//! - spec-reviewer is intentionally OUT of scope: it reviews spec documents,
//!   not diffs, so its Filtering wording legitimately differs.
//!
//! Each agent appends further agent-specific bullets after the shared prefix,
//! so this test intentionally ignores everything beyond the first 3 bullets.

#![allow(
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use std::fs;

mod common;
use common::helpers::repo_root;

/// Extract the first `n` bullet lines that appear after the first line whose
/// content begins with `## Filtering` in the given markdown text.
///
/// A bullet line starts with `- `. Scanning stops early if a new heading is
/// encountered before `n` bullets are collected, and the function panics in
/// that case so the lock test fails loudly rather than masking drift.
fn extract_filtering_prefix_bullets(content: &str, n: usize) -> String {
    let mut lines = content.lines();
    let mut heading_found = false;
    for line in lines.by_ref() {
        if line.starts_with("## Filtering") {
            heading_found = true;
            break;
        }
    }
    assert!(
        heading_found,
        "extract_filtering_prefix_bullets: `## Filtering` heading not found"
    );

    let mut bullets = Vec::with_capacity(n);
    for line in lines {
        if line.starts_with("- ") {
            bullets.push(line.to_string());
            if bullets.len() == n {
                return bullets.join("\n");
            }
        } else if line.starts_with("## ") || line.starts_with("### ") {
            break;
        }
    }

    panic!(
        "extract_filtering_prefix_bullets: expected {n} bullets after `## Filtering`, found {}",
        bullets.len()
    );
}

const CODE_REVIEW_AGENTS: &[&str] = &[
    "plugins/belt/agents/code-reviewer.md",
    "plugins/belt/agents/quality-reviewer.md",
];

/// Markers that must appear in the shared prefix so the parity check cannot
/// silently pass on the wrong three bullets.
const SHARED_PREFIX_MARKERS: &[&str] = &[
    "at least 80% confident",
    "occurrence count",
    "No stylistic opinions",
];

#[test]
fn code_review_filtering_prefix_identical_across_agents() {
    let mut extracted: Vec<(&str, String)> = Vec::with_capacity(CODE_REVIEW_AGENTS.len());
    for rel in CODE_REVIEW_AGENTS {
        let path = repo_root().join(rel);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        extracted.push((rel, extract_filtering_prefix_bullets(&content, 3)));
    }

    let (base_rel, baseline) = &extracted[0];
    for marker in SHARED_PREFIX_MARKERS {
        assert!(
            baseline.contains(marker),
            "{base_rel}: shared Filtering prefix must contain {marker:?}, got:\n{baseline}"
        );
    }

    for (rel, bullets) in &extracted[1..] {
        assert_eq!(
            bullets, baseline,
            "Filtering prefix drift in {rel} vs baseline {base_rel}"
        );
    }
}
