#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

//! Binding lock test: docs/testing/cli-behavior/*.yml ↔ Rust doc-comment `/// scenario: <id>`.
//!
//! Walks crates/{belt,belt-agent,belt-core}/tests/ recursively. Strips block comments
//! (including block doc-comments `/** ... */`) before grep to avoid false positives.
//!
//! Source of truth:
//! - docs/testing/cli-behavior/{belt,belt-agent,belt-core}.yml — scenario IDs
//! - docs/testing/lock-ledger.md — locks-file frontmatter entries
//! - docs/testing/audit-template.md — `audit_template_version` = v1

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ScenariosFile {
    #[serde(default)]
    #[allow(dead_code)]
    scope: Option<String>,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
struct Scenario {
    id: String,
    #[allow(dead_code)]
    category: String,
    #[allow(dead_code)]
    severity: String,
    #[allow(dead_code)]
    given: String,
    #[allow(dead_code)]
    when: String,
    #[allow(dead_code)]
    then: String,
    #[serde(default)]
    #[allow(dead_code)]
    technique: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    preconditions: Option<Vec<String>>,
    #[serde(default)]
    #[allow(dead_code)]
    postconditions: Option<Vec<String>>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn load_scenarios(rel_path: &str) -> ScenariosFile {
    let path = repo_root().join(rel_path);
    let body =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_saphyr::from_str(&body).unwrap_or_else(|e| panic!("cannot parse {}: {e}", path.display()))
}

fn all_scenario_ids() -> HashSet<String> {
    let mut ids = HashSet::new();
    for rel in &[
        "docs/testing/cli-behavior/belt.yml",
        "docs/testing/cli-behavior/belt-core.yml",
        "docs/testing/cli-behavior/belt-agent.yml",
    ] {
        let file = load_scenarios(rel);
        for s in file.scenarios {
            assert!(
                ids.insert(s.id.clone()),
                "duplicate scenario id across yml files: {}",
                s.id
            );
        }
    }
    ids
}

/// Strip block comments (/* ... */, including /** ... */) from Rust source.
/// Replaces block-comment characters with spaces (preserves line numbers).
fn strip_block_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut in_block = false;
    while i < bytes.len() {
        if !in_block && i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            in_block = true;
            out.push(' ');
            out.push(' ');
            i += 2;
            continue;
        }
        if in_block && i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'/' {
            in_block = false;
            out.push(' ');
            out.push(' ');
            i += 2;
            continue;
        }
        if in_block {
            if bytes[i] == b'\n' {
                out.push('\n');
            } else {
                out.push(' ');
            }
        } else {
            out.push(bytes[i] as char);
        }
        i += 1;
    }
    out
}

/// Strip string literals (simple version: `"..."` on single line, no escape handling beyond \").
/// Good enough for CI test source which avoids complex string literal shapes.
fn strip_string_literals(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut in_str = false;
    let mut prev_escape = false;
    for c in src.chars() {
        if !in_str && c == '"' {
            in_str = true;
            out.push(' ');
            continue;
        }
        if in_str {
            if c == '"' && !prev_escape {
                in_str = false;
                out.push(' ');
            } else if c == '\n' {
                out.push('\n');
                in_str = false; // safety: strings don't span lines in CI sources
            } else {
                out.push(' ');
            }
            prev_escape = c == '\\' && !prev_escape;
            continue;
        }
        out.push(c);
        prev_escape = false;
    }
    out
}

/// Match a single line against `/// scenario: <id>` and return the id if matched.
fn match_scenario_line(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let rest = line.strip_prefix("///")?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix("scenario:")?;
    let rest = rest.trim_start();
    let id = rest.trim_end();
    if id.is_empty() || id.contains(char::is_whitespace) {
        None
    } else {
        Some(id)
    }
}

fn collect_rust_scenario_refs() -> HashSet<String> {
    let mut found = HashSet::new();
    for crate_tests in &[
        "crates/belt/tests",
        "crates/belt-agent/tests",
        "crates/belt-core/tests",
    ] {
        walk_rs_files(&repo_root().join(crate_tests), &mut |path| {
            let src = fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            let src = strip_block_comments(&src);
            let src = strip_string_literals(&src);
            for line in src.lines() {
                if let Some(id) = match_scenario_line(line) {
                    found.insert(id.to_string());
                }
            }
        });
    }
    found
}

fn walk_rs_files(dir: &Path, cb: &mut dyn FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk_rs_files(&p, cb);
        } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
            cb(&p);
        }
    }
}

#[test]
fn scenarios_yml_and_rust_docs_match() {
    let yml_ids = all_scenario_ids();
    let rust_ids = collect_rust_scenario_refs();

    let orphan_yml: Vec<_> = yml_ids.difference(&rust_ids).collect();
    let orphan_rust: Vec<_> = rust_ids.difference(&yml_ids).collect();

    assert!(
        orphan_yml.is_empty(),
        "orphan-yml (scenarios.yml に ID ありだが Rust 側 /// scenario: 未追加): {orphan_yml:?}"
    );
    assert!(
        orphan_rust.is_empty(),
        "orphan-rust (Rust に /// scenario: ID あるが scenarios.yml 未登録): {orphan_rust:?}"
    );
}

#[test]
fn lock_ledger_locks_files_exist() {
    let ledger = fs::read_to_string(repo_root().join("docs/testing/lock-ledger.md"))
        .expect("lock-ledger.md exists");
    let prefix = "locks-file:";
    for line in ledger.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix(prefix) else {
            continue;
        };
        let rel = rest.trim();
        let abs = repo_root().join(rel);
        assert!(
            abs.exists(),
            "lock-ledger.md references missing file: {rel}"
        );
    }
}

#[test]
fn audit_template_version_v1_matches_expected() {
    let tpl = fs::read_to_string(repo_root().join("docs/testing/audit-template.md"))
        .expect("audit-template.md exists");
    assert!(
        tpl.contains("audit_template_version: v1"),
        "audit-template.md frontmatter must declare audit_template_version: v1"
    );
}

#[test]
fn drift_regex_rejects_typo_senario() {
    let src = "/// senario: foo";
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(!has_match, "typo senario: must not match scenario: pattern");
}

#[test]
fn drift_regex_rejects_single_slash_prefix() {
    let src = "// scenario: foo";
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(
        !has_match,
        "single-slash // scenario: must not match triple-slash pattern"
    );
}

#[test]
fn drift_block_comment_with_scenario_is_stripped() {
    let src = "/* /// scenario: foo */";
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(
        !has_match,
        "block comment containing /// scenario: must be stripped"
    );
}

#[test]
fn drift_block_doc_comment_is_stripped() {
    let src = "/** scenario: foo */";
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(!has_match, "block doc-comment /** ... */ must be stripped");
}

#[test]
fn drift_string_literal_is_stripped() {
    let src = r#"let s = "/// scenario: foo";"#;
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(
        !has_match,
        "string literal containing /// scenario: must be stripped"
    );
}

#[test]
fn drift_inner_doc_comment_does_not_match() {
    let src = "//! scenario: foo";
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(
        !has_match,
        "inner doc-comment //! scenario: must not match /// pattern"
    );
}

#[test]
fn drift_positive_single_line_doc_comment_matches() {
    let src = "    /// scenario: belt-lint-valid-pipeline-ok";
    let stripped = strip_string_literals(&strip_block_comments(src));
    let matched: Vec<_> = stripped.lines().filter_map(match_scenario_line).collect();
    assert_eq!(
        matched.as_slice(),
        &["belt-lint-valid-pipeline-ok"],
        "valid single-line /// scenario: must match"
    );
}
