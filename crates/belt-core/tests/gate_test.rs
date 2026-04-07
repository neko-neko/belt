use std::fs;

use belt_core::gate::{all_passed, execute_gate};
use belt_core::model::GateCheck;

/// `cmd: "true"` exits 0 -> gate passes.
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

/// `has_output: true` with a file in output_dir -> gate passes.
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

/// `has_output: true` with an empty output_dir -> gate fails.
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

/// `all_passed` returns true only when every result passed.
#[test]
fn all_passed_integration() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(tmp.path().join("hello.txt"), "content").expect("write");

    let checks = vec![
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
