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

/// Strip string literals (regular `"..."` and raw strings `r"..."` / `r#"..."#` /
/// `r##"..."##` etc) from Rust source. Replaces string contents with spaces,
/// preserving line numbers.
///
/// Supports:
/// - Regular strings with `\"` / `\\` escape handling (single-line only — strings
///   are terminated at `\n` as a safety net).
/// - Raw strings `r"..."`, `r#"..."#`, `r##"..."##`, etc, including multiline.
///   Closing tag matches the exact hash count recorded at the opening.
#[allow(
    clippy::many_single_char_names,
    reason = "byte-scan indices (i, j, k, m) are idiomatic for a tokenizer and match the plan's verbatim algorithm"
)]
fn strip_string_literals(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];

        // Raw string: r"..." / r#"..."# / r##"..."## ...
        if b == b'r' {
            let mut hashes = 0;
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] == b'#' {
                hashes += 1;
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'"' {
                // Emit 'r' + hashes + '"' as spaces (preserve line breaks via newline detection below)
                for _ in 0..=(hashes + 1) {
                    out.push(' ');
                }
                let content_start = j + 1;
                let mut k = content_start;
                // Close tag: `"` followed by the same number of `#`
                let close_needed = hashes;
                let mut found_close_at = None;
                while k < bytes.len() {
                    if bytes[k] == b'"' {
                        let mut m = k + 1;
                        let mut count = 0;
                        while m < bytes.len() && bytes[m] == b'#' && count < close_needed {
                            count += 1;
                            m += 1;
                        }
                        if count == close_needed {
                            found_close_at = Some((k, m));
                            break;
                        }
                    }
                    k += 1;
                }
                let Some((close_quote, close_end)) = found_close_at else {
                    // Unterminated raw string: rest of file is inside string.
                    // Safety: emit spaces/newlines to end to avoid infinite loop.
                    for byte_in_tail in &bytes[content_start..] {
                        if *byte_in_tail == b'\n' {
                            out.push('\n');
                        } else {
                            out.push(' ');
                        }
                    }
                    return out;
                };
                for byte_in_content in &bytes[content_start..close_quote] {
                    if *byte_in_content == b'\n' {
                        out.push('\n');
                    } else {
                        out.push(' ');
                    }
                }
                // Emit closing `"` + hashes as spaces
                for _ in close_quote..close_end {
                    out.push(' ');
                }
                i = close_end;
                continue;
            }
        }

        // Regular string: "..."
        if b == b'"' {
            out.push(' ');
            let mut j = i + 1;
            let mut prev_escape = false;
            while j < bytes.len() {
                let c = bytes[j];
                if c == b'"' && !prev_escape {
                    out.push(' ');
                    j += 1;
                    break;
                }
                if c == b'\n' {
                    // Safety: regular strings should not span lines in test sources.
                    out.push('\n');
                    j += 1;
                    break;
                }
                out.push(' ');
                prev_escape = c == b'\\' && !prev_escape;
                j += 1;
            }
            i = j;
            continue;
        }

        // Default: copy byte as char (ASCII assumption for test sources)
        out.push(b as char);
        i += 1;
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

#[test]
fn drift_multiline_raw_string_is_stripped() {
    let src = r##"
let s = r#"
    /// scenario: belt-core-multiline-raw-false-positive
    some content
"#;
"##;
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(
        !has_match,
        "multiline raw string containing /// scenario: must be stripped"
    );
}

#[test]
fn drift_raw_string_with_hash_is_stripped() {
    let src = r###"
let s = r##"
    /// scenario: belt-core-raw-hash-false-positive
"##;
"###;
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(
        !has_match,
        "raw string with hashes containing /// scenario: must be stripped"
    );
}

#[test]
fn drift_doc_comment_outside_string_still_matches_after_fix() {
    let src = r##"
/// scenario: belt-core-positive-outside-string
fn test_fn() {
    let _s = r#"
        /// scenario: belt-core-inside-string-false-positive
    "#;
}
"##;
    let stripped = strip_string_literals(&strip_block_comments(src));
    let matched: Vec<_> = stripped.lines().filter_map(match_scenario_line).collect();
    assert_eq!(
        matched.as_slice(),
        &["belt-core-positive-outside-string"],
        "fix must preserve doc-comment match outside raw strings (false-negative check)"
    );
}
