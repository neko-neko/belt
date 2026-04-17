#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use belt_core::error::BeltError;
use miette::Diagnostic;

/// scenario: belt-core-error-config-parse-renders-path-detail-and-code
#[test]
fn config_parse_error_display_includes_path_and_detail() {
    let err = BeltError::ConfigParse {
        path: "belt.toml".to_string(),
        detail: "expected `=`, found newline".to_string(),
    };
    assert_eq!(
        err.to_string(),
        "config parse error in belt.toml: expected `=`, found newline"
    );
}

/// scenario: belt-core-error-config-parse-renders-path-detail-and-code
#[test]
fn config_parse_error_diagnostic_code() {
    let err = BeltError::ConfigParse {
        path: "belt.toml".to_string(),
        detail: "missing field".to_string(),
    };
    let code = err
        .code()
        .expect("ConfigParse should have a diagnostic code");
    assert_eq!(code.to_string(), "belt::config_parse");
}

/// scenario: belt-core-error-regate-required-renders-phase-and-code
#[test]
fn regate_required_error_display() {
    let err = BeltError::RegateRequired {
        phase_id: "deploy".to_string(),
        targets: vec!["design".to_string(), "review".to_string()],
    };
    assert_eq!(
        err.to_string(),
        "regate required for phase 'deploy': run regate before step"
    );
}

/// scenario: belt-core-error-regate-required-renders-phase-and-code
#[test]
fn regate_required_error_diagnostic_code() {
    let err = BeltError::RegateRequired {
        phase_id: "deploy".to_string(),
        targets: vec!["design".to_string()],
    };
    let code = err
        .code()
        .expect("RegateRequired should have a diagnostic code");
    assert_eq!(code.to_string(), "belt::regate_required");
}

/// scenario: belt-core-error-regate-failed-renders-targets-and-code
#[test]
fn regate_failed_error_display() {
    let err = BeltError::RegateFailed {
        phase_id: "deploy".to_string(),
        targets: vec!["design".to_string()],
    };
    assert_eq!(
        err.to_string(),
        r#"regate failed for phase 'deploy': targets ["design"] did not pass"#
    );
}

/// scenario: belt-core-error-regate-failed-renders-targets-and-code
#[test]
fn regate_failed_error_diagnostic_code() {
    let err = BeltError::RegateFailed {
        phase_id: "deploy".to_string(),
        targets: vec!["design".to_string(), "review".to_string()],
    };
    let code = err
        .code()
        .expect("RegateFailed should have a diagnostic code");
    assert_eq!(code.to_string(), "belt::regate_failed");
}
