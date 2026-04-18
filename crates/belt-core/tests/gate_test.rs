#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use std::fs;
use std::path::{Path, PathBuf};

use belt_core::gate::{NoopUriResolver, UriResolver, all_passed, execute_gate, execute_gates};
use belt_core::model::GateCheck;

/// Test resolver that maps any `belt://current/<path>` to `<base>/<path>`.
struct TestCurrentResolver {
    base: PathBuf,
}

impl UriResolver for TestCurrentResolver {
    fn resolve(&self, uri: &str) -> Result<PathBuf, String> {
        let path = uri
            .strip_prefix("belt://current/")
            .ok_or_else(|| format!("not a belt://current/ uri: {uri}"))?;
        Ok(self.base.join(path))
    }
}

/// Initialize a git repository in a fresh tempdir; return the `TempDir` (scope controls cleanup).
fn git_init_tempdir() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let status = std::process::Command::new("git")
        .args([
            "-c",
            "init.defaultBranch=main",
            "-c",
            "core.excludesfile=/dev/null",
            "init",
        ])
        .current_dir(tmp.path())
        .status()
        .expect("git init");
    assert!(status.success(), "git init failed");
    tmp
}

/// `cmd: "true"` exits 0 -> gate passes.
/// scenario: belt-core-gate-cmd-zero-exit-passes
#[test]
fn cmd_gate_pass() {
    let check = GateCheck::Cmd {
        cmd: "true".to_owned(),
        timeout: 1800,
    };
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);

    assert_eq!(result.check_type, "cmd");
    assert!(result.passed);
    assert!(result.duration_ms.is_some());
}

/// `cmd: "false"` exits 1 -> gate fails.
/// scenario: belt-core-gate-cmd-nonzero-or-spawn-or-signal-fails-without-timeout
#[test]
fn cmd_gate_fail() {
    let check = GateCheck::Cmd {
        cmd: "false".to_owned(),
        timeout: 1800,
    };
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);

    assert_eq!(result.check_type, "cmd");
    assert!(!result.passed);
    assert!(result.duration_ms.is_some());
    // detail should contain the exit code
    assert!(
        result.detail.as_deref().unwrap_or("").contains("exit"),
        "expected detail to mention exit code, got: {:?}",
        result.detail
    );
}

/// `file_exists: "*.txt"` with a matching file -> gate passes.
/// scenario: belt-core-gate-file-exists-glob-matches
#[test]
fn file_exists_gate_pass() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(tmp.path().join("hello.txt"), "content").expect("write");

    let check = GateCheck::FileExists {
        file_exists: "*.txt".to_owned(),
    };
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);

    assert_eq!(result.check_type, "file_exists");
    assert!(result.passed);
    assert!(
        result.detail.as_deref().unwrap_or("").contains("matched"),
        "expected detail to mention matched files, got: {:?}",
        result.detail
    );
}

/// `file_exists: "*.txt"` with an empty directory -> gate fails.
/// scenario: belt-core-gate-file-exists-no-match-fails
#[test]
fn file_exists_gate_fail() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let check = GateCheck::FileExists {
        file_exists: "*.txt".to_owned(),
    };
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);

    assert_eq!(result.check_type, "file_exists");
    assert!(!result.passed);
    assert!(
        result.detail.as_deref().unwrap_or("").contains("no files"),
        "expected detail to mention no files, got: {:?}",
        result.detail
    );
}

/// `has_output: true` with a file in `output_dir` -> gate passes.
/// scenario: belt-core-gate-has-output-non-empty-passes
#[test]
fn has_output_gate_pass() {
    let work = tempfile::tempdir().expect("tempdir");
    let output = tempfile::tempdir().expect("tempdir");
    fs::write(output.path().join("artifact.bin"), b"data").expect("write");

    let check = GateCheck::HasOutput { has_output: true };
    let result = execute_gate(&check, work.path(), output.path(), &NoopUriResolver);

    assert_eq!(result.check_type, "has_output");
    assert!(result.passed);
    assert!(
        result.detail.as_deref().unwrap_or("").contains("non-empty"),
        "expected detail to mention non-empty, got: {:?}",
        result.detail
    );
}

/// `has_output: true` with an empty `output_dir` -> gate fails.
/// scenario: belt-core-gate-has-output-empty-dir-fails
#[test]
fn has_output_gate_fail_empty_dir() {
    let work = tempfile::tempdir().expect("tempdir");
    let output = tempfile::tempdir().expect("tempdir");

    let check = GateCheck::HasOutput { has_output: true };
    let result = execute_gate(&check, work.path(), output.path(), &NoopUriResolver);

    assert_eq!(result.check_type, "has_output");
    assert!(!result.passed);
    assert!(
        result.detail.as_deref().unwrap_or("").contains("empty"),
        "expected detail to mention empty, got: {:?}",
        result.detail
    );
}

/// `GateResult` for non-cmd gates has `timed_out` = false.
/// scenario: belt-core-gate-result-json-round-trip-and-timed-out-default
#[test]
fn gate_result_timed_out_default_false() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::write(tmp.path().join("a.txt"), "x").expect("write");
    let check = GateCheck::FileExists {
        file_exists: "*.txt".to_owned(),
    };
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);
    assert!(!result.timed_out);
}

/// `GateResult.timed_out` serializes to JSON correctly.
/// scenario: belt-core-gate-result-json-round-trip-and-timed-out-default
#[test]
fn gate_result_timed_out_serializes() {
    let result = belt_core::gate::GateResult {
        check_type: "cmd".to_owned(),
        passed: false,
        detail: Some("timed out after 1s".to_owned()),
        duration_ms: Some(1000),
        timed_out: true,
    };
    let json = serde_json::to_string(&result).expect("serialize");
    assert!(json.contains(r#""timed_out":true"#), "json: {json}");
}

/// cmd completes within timeout — passes normally.
/// scenario: belt-core-gate-cmd-zero-exit-passes
#[test]
fn cmd_with_timeout_passes() {
    let check = GateCheck::Cmd {
        cmd: "true".to_owned(),
        timeout: 5,
    };
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);
    assert!(result.passed);
    assert!(!result.timed_out);
    assert!(result.duration_ms.unwrap() < 5000);
}

/// cmd fails normally (non-zero exit) within timeout — FAIL, not timeout.
/// scenario: belt-core-gate-cmd-nonzero-or-spawn-or-signal-fails-without-timeout
#[test]
fn cmd_with_timeout_fails_normally() {
    let check = GateCheck::Cmd {
        cmd: "false".to_owned(),
        timeout: 5,
    };
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);
    assert!(!result.passed);
    assert!(!result.timed_out);
    assert!(result.detail.as_deref().unwrap_or("").contains("exit"));
}

/// cmd with timeout: 0 completes normally (no timeout applied).
/// scenario: belt-core-gate-cmd-zero-exit-passes
#[test]
fn cmd_with_timeout_zero_passes() {
    let check = GateCheck::Cmd {
        cmd: "true".to_owned(),
        timeout: 0,
    };
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);
    assert!(result.passed);
    assert!(!result.timed_out);
}

/// cmd exceeds timeout — killed, `timed_out` = true.
/// scenario: belt-core-gate-cmd-timeout-kills-and-reports
#[test]
fn cmd_timeout_kills_hanging_process() {
    let check = GateCheck::Cmd {
        cmd: "sleep 60".to_owned(),
        timeout: 1,
    };
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);
    assert!(!result.passed);
    assert!(result.timed_out);
    assert!(
        result.detail.as_deref().unwrap_or("").contains("timed out"),
        "detail: {:?}",
        result.detail
    );
}

/// Timeout `duration_ms` reflects the timeout value.
/// scenario: belt-core-gate-cmd-timeout-kills-and-reports
#[test]
fn cmd_timeout_duration_reflects_timeout() {
    let check = GateCheck::Cmd {
        cmd: "sleep 60".to_owned(),
        timeout: 2,
    };
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);
    assert!(result.timed_out);
    let ms = result.duration_ms.unwrap();
    assert!(ms >= 2000, "duration_ms too low: {ms}");
    assert!(ms < 4000, "duration_ms too high: {ms}");
}

/// Fast command finishes before timeout.
/// scenario: belt-core-gate-cmd-zero-exit-passes
#[test]
fn cmd_fast_finish_before_timeout() {
    let check = GateCheck::Cmd {
        cmd: "echo fast".to_owned(),
        timeout: 1,
    };
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);
    assert!(result.passed);
    assert!(!result.timed_out);
    assert!(result.duration_ms.unwrap() < 1000);
}

/// Spawn failure with timeout — not a timeout error.
/// scenario: belt-core-gate-cmd-nonzero-or-spawn-or-signal-fails-without-timeout
#[test]
fn cmd_spawn_failure_with_timeout() {
    let check = GateCheck::Cmd {
        cmd: "/nonexistent_binary_xyz_123".to_owned(),
        timeout: 5,
    };
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);
    assert!(!result.passed);
    assert!(!result.timed_out);
    // sh -c returns exit 127 for command not found
    assert!(
        result.detail.as_deref().unwrap_or("").contains("exit")
            || result.detail.as_deref().unwrap_or("").contains("not found"),
        "detail: {:?}",
        result.detail
    );
}

/// stderr output preserved on normal failure with timeout.
/// scenario: belt-core-gate-cmd-nonzero-or-spawn-or-signal-fails-without-timeout
#[test]
fn cmd_stderr_output_on_failure_with_timeout() {
    let check = GateCheck::Cmd {
        cmd: "echo err >&2 && false".to_owned(),
        timeout: 5,
    };
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);
    assert!(!result.passed);
    assert!(!result.timed_out);
    assert!(
        result.detail.as_deref().unwrap_or("").contains("err"),
        "detail should contain stderr, got: {:?}",
        result.detail
    );
}

/// Signal exit with timeout — not a timeout.
/// scenario: belt-core-gate-cmd-nonzero-or-spawn-or-signal-fails-without-timeout
#[test]
fn cmd_signal_exit_with_timeout() {
    let check = GateCheck::Cmd {
        cmd: "kill -9 $$".to_owned(),
        timeout: 5,
    };
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);
    assert!(!result.passed);
    assert!(!result.timed_out);
    assert!(
        result.detail.as_deref().unwrap_or("").contains("signal"),
        "detail should mention signal, got: {:?}",
        result.detail
    );
}

/// `all_passed` returns true only when every result passed.
/// scenario: belt-core-gate-execute-gates-and-all-passed-aggregate
#[test]
fn all_passed_integration() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(tmp.path().join("hello.txt"), "content").expect("write");

    let checks = [
        GateCheck::Cmd {
            cmd: "true".to_owned(),
            timeout: 1800,
        },
        GateCheck::FileExists {
            file_exists: "*.txt".to_owned(),
        },
    ];

    let results: Vec<_> = checks
        .iter()
        .map(|c| execute_gate(c, tmp.path(), tmp.path(), &NoopUriResolver))
        .collect();

    assert!(all_passed(&results));
}

/// One timeout among multiple checks fails the overall result.
/// scenario: belt-core-gate-execute-gates-and-all-passed-aggregate
#[test]
fn execute_gates_one_timeout_fails_all() {
    let checks = vec![
        GateCheck::Cmd {
            cmd: "true".to_owned(),
            timeout: 5,
        },
        GateCheck::Cmd {
            cmd: "sleep 60".to_owned(),
            timeout: 1,
        },
    ];
    let tmp = tempfile::tempdir().expect("tempdir");
    let results = execute_gates(&checks, tmp.path(), tmp.path(), &NoopUriResolver);

    assert_eq!(results.len(), 2);
    assert!(results[0].passed);
    assert!(!results[0].timed_out);
    assert!(!results[1].passed);
    assert!(results[1].timed_out);
    assert!(!all_passed(&results));
}

/// All checks pass with timeout set.
/// scenario: belt-core-gate-execute-gates-and-all-passed-aggregate
#[test]
fn execute_gates_all_pass_with_timeout() {
    let checks = vec![
        GateCheck::Cmd {
            cmd: "true".to_owned(),
            timeout: 5,
        },
        GateCheck::Cmd {
            cmd: "echo ok".to_owned(),
            timeout: 5,
        },
    ];
    let tmp = tempfile::tempdir().expect("tempdir");
    let results = execute_gates(&checks, tmp.path(), tmp.path(), &NoopUriResolver);

    assert_eq!(results.len(), 2);
    assert!(results[0].passed);
    assert!(results[1].passed);
    assert!(!results[0].timed_out);
    assert!(!results[1].timed_out);
    assert!(all_passed(&results));
}

/// `GateResult` round-trips through JSON serialization.
/// scenario: belt-core-gate-result-json-round-trip-and-timed-out-default
#[test]
fn gate_result_deserialize_round_trip() {
    let original = belt_core::gate::GateResult {
        check_type: "cmd".to_owned(),
        passed: false,
        detail: Some("exit 1: error".to_owned()),
        duration_ms: Some(234),
        timed_out: false,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let restored: belt_core::gate::GateResult = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.check_type, "cmd");
    assert!(!restored.passed);
    assert_eq!(restored.detail.as_deref(), Some("exit 1: error"));
    assert_eq!(restored.duration_ms, Some(234));
    assert!(!restored.timed_out);
}

/// `GateResult` without `timed_out` field deserializes with default false.
/// scenario: belt-core-gate-result-json-round-trip-and-timed-out-default
#[test]
fn gate_result_deserialize_missing_timed_out() {
    let json = r#"{"check_type":"cmd","passed":true,"detail":null,"duration_ms":100}"#;
    let result: belt_core::gate::GateResult = serde_json::from_str(json).expect("deserialize");
    assert!(result.passed);
    assert!(!result.timed_out);
}

/// scenario: belt-core-gate-git-clean-clean-repo-with-expect-clean-passes
#[test]
fn git_clean_clean_repo_with_expect_clean_passes() {
    let tmp = git_init_tempdir();
    let check = GateCheck::GitClean { git_clean: true };
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);

    assert_eq!(result.check_type, "git_clean");
    assert!(result.passed, "clean repo + expect_clean=true should pass");
    assert_eq!(result.detail.as_deref(), Some("working tree clean"));
}

/// scenario: belt-core-gate-git-clean-dirty-repo-with-expect-dirty-passes
#[test]
fn git_clean_dirty_repo_with_expect_dirty_passes() {
    let tmp = git_init_tempdir();
    std::fs::write(tmp.path().join("dirty.txt"), "x").expect("write");
    let check = GateCheck::GitClean { git_clean: false };
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);

    assert!(result.passed, "dirty repo + expect_clean=false should pass");
    let detail = result.detail.as_deref().unwrap_or("");
    assert!(
        detail.contains("file(s) with uncommitted changes"),
        "detail mismatch: {detail}"
    );
}

/// scenario: belt-core-gate-git-clean-clean-repo-with-expect-dirty-fails
#[test]
fn git_clean_clean_repo_with_expect_dirty_fails() {
    let tmp = git_init_tempdir();
    let check = GateCheck::GitClean { git_clean: false };
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);

    assert!(
        !result.passed,
        "clean repo + expect_clean=false should fail"
    );
    assert_eq!(result.detail.as_deref(), Some("working tree clean"));
}

/// scenario: belt-core-gate-git-clean-dirty-repo-with-expect-clean-fails
#[test]
fn git_clean_dirty_repo_with_expect_clean_fails() {
    let tmp = git_init_tempdir();
    std::fs::write(tmp.path().join("dirty.txt"), "x").expect("write");
    let check = GateCheck::GitClean { git_clean: true };
    let result = execute_gate(&check, tmp.path(), tmp.path(), &NoopUriResolver);

    assert!(!result.passed, "dirty repo + expect_clean=true should fail");
    let detail = result.detail.as_deref().unwrap_or("");
    assert!(
        detail.contains("file(s) with uncommitted changes"),
        "detail mismatch: {detail}"
    );
}

/// scenario: belt-core-gate-git-clean-missing-work-dir-yields-spawn-failure
#[test]
fn git_clean_missing_work_dir_yields_spawn_failure() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let missing = tmp.path().join("does-not-exist");
    assert!(!missing.exists());
    let check = GateCheck::GitClean { git_clean: true };
    let result = execute_gate(&check, &missing, &missing, &NoopUriResolver);

    assert!(!result.passed, "missing work_dir should spawn-fail");
    let detail = result.detail.as_deref().unwrap_or("");
    assert!(
        detail.starts_with("failed to run git:"),
        "detail mismatch: {detail}"
    );
}

/// scenario: belt-core-gate-resolves-belt-current-via-uri-resolver
#[test]
fn gate_resolves_belt_current_via_uri_resolver() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let target = tmp.path().join("notes").join("phase-design.md");
    std::fs::create_dir_all(target.parent().expect("parent")).expect("create_dir_all");
    std::fs::write(&target, "x").expect("write");

    let resolver = TestCurrentResolver {
        base: tmp.path().to_path_buf(),
    };
    let gates = vec![GateCheck::FileExists {
        file_exists: "belt://current/notes/phase-design.md".to_owned(),
    }];
    let results = execute_gates(&gates, Path::new("/"), Path::new("/"), &resolver);
    assert_eq!(results.len(), 1);
    assert!(results[0].passed, "URI-resolved file must exist");
}

/// scenario: belt-core-gate-passes-raw-domain-path-untouched
#[test]
fn gate_passes_raw_domain_path_untouched() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let target = tmp.path().join("docs").join("design.md");
    std::fs::create_dir_all(target.parent().expect("parent")).expect("create_dir_all");
    std::fs::write(&target, "x").expect("write");

    let resolver = NoopUriResolver;
    let gates = vec![GateCheck::FileExists {
        file_exists: "docs/design.md".to_owned(),
    }];
    // Raw path: no belt:// prefix, resolver MUST NOT be invoked.
    let results = execute_gates(&gates, tmp.path(), Path::new("/"), &resolver);
    assert_eq!(results.len(), 1);
    assert!(results[0].passed, "raw glob must match against work_dir");
}
