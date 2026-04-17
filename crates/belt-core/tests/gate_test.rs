#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use std::fs;

use belt_core::gate::{all_passed, execute_gate, execute_gates};
use belt_core::model::GateCheck;

/// `cmd: "true"` exits 0 -> gate passes.
/// scenario: belt-core-gate-cmd-zero-exit-passes
#[test]
fn cmd_gate_pass() {
    let check = GateCheck::Cmd {
        cmd: "true".to_owned(),
        timeout: 1800,
    };
    let tmp = tempfile::tempdir().expect("tempdir");
    let result = execute_gate(&check, tmp.path(), tmp.path());

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
    let result = execute_gate(&check, tmp.path(), tmp.path());

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
    let result = execute_gate(&check, tmp.path(), tmp.path());

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
    let result = execute_gate(&check, tmp.path(), tmp.path());

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
    let result = execute_gate(&check, work.path(), output.path());

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
    let result = execute_gate(&check, work.path(), output.path());

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
    let result = execute_gate(&check, tmp.path(), tmp.path());
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
    let result = execute_gate(&check, tmp.path(), tmp.path());
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
    let result = execute_gate(&check, tmp.path(), tmp.path());
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
    let result = execute_gate(&check, tmp.path(), tmp.path());
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
    let result = execute_gate(&check, tmp.path(), tmp.path());
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
    let result = execute_gate(&check, tmp.path(), tmp.path());
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
    let result = execute_gate(&check, tmp.path(), tmp.path());
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
    let result = execute_gate(&check, tmp.path(), tmp.path());
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
    let result = execute_gate(&check, tmp.path(), tmp.path());
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
    let result = execute_gate(&check, tmp.path(), tmp.path());
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
        .map(|c| execute_gate(c, tmp.path(), tmp.path()))
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
    let results = execute_gates(&checks, tmp.path(), tmp.path());

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
    let results = execute_gates(&checks, tmp.path(), tmp.path());

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
