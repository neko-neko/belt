//! Integration test locking byte-identity of the `## Filtering` section prefix
//! bullets across per-observation reviewer agent markdown files.
//!
//! Background:
//! - The 2026-04-16 review-skills refactor split /code-review and /spec-review
//!   into per-observation subagents. Each agent's `## Filtering` section opens
//!   with the same bullet preface so that parallel findings can be merged and
//!   cross-agent deduplicated on uniform filtering contracts.
//! - Two drift incidents (Phase B I3 in commit f191a22 and Phase C C-1 in
//!   db73c7d) had the preface bullets diverge. Both were caught only at
//!   review time. This lock test closes that gap at commit time.
//!
//! Contract enforced here:
//! - code-review: the first 3 bullets after `## Filtering` are byte-identical
//!   across security / test / ai-antipattern / cross-cutting agents.
//! - spec-review: the first 2 bullets after `## Filtering` are byte-identical
//!   across feasibility / ui-design / cross-cutting-spec agents.
//!
//! The cross-cutting agents append an additional Internal self-dedup bullet
//! and may carry a heading suffix (e.g. "(applies to all four observations)"),
//! so this test intentionally ignores everything beyond the shared prefix.

#![allow(
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at crates/belt-core; walk two levels up to
    // reach the workspace root.
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path
}

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
    "plugins/belt/agents/security-reviewer.md",
    "plugins/belt/agents/test-reviewer.md",
    "plugins/belt/agents/ai-antipattern-reviewer.md",
    "plugins/belt/agents/cross-cutting-reviewer.md",
];

const SPEC_REVIEW_AGENTS: &[&str] = &[
    "plugins/belt/agents/feasibility-reviewer.md",
    "plugins/belt/agents/ui-design-reviewer.md",
    "plugins/belt/agents/cross-cutting-spec-reviewer.md",
];

fn assert_filtering_prefix_identical(agents: &[&str], prefix_bullets: usize) {
    let mut baseline: Option<(&str, String)> = None;
    for rel in agents {
        let path = repo_root().join(rel);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let extracted = extract_filtering_prefix_bullets(&content, prefix_bullets);
        match &baseline {
            None => baseline = Some((rel, extracted)),
            Some((base_path, base_content)) => {
                assert_eq!(
                    &extracted, base_content,
                    "Filtering prefix drift in {rel} vs baseline {base_path}"
                );
            }
        }
    }
}

#[test]
fn code_review_filtering_prefix_identical_across_agents() {
    assert_filtering_prefix_identical(CODE_REVIEW_AGENTS, 3);
}

#[test]
fn spec_review_filtering_prefix_identical_across_agents() {
    assert_filtering_prefix_identical(SPEC_REVIEW_AGENTS, 2);
}
