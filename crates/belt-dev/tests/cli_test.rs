#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

fn belt_bin() -> String {
    env!("CARGO_BIN_EXE_belt-dev").to_string()
}

#[test]
fn cli_without_args_shows_help_with_exit_64() {
    // Exit code 64 = POSIX EX_USAGE (command/invocation error).
    // `belt-dev` without args is a missing required subcommand → clap parse error
    // → main() maps to ExitCode::from(64) via try_parse.
    let output = Command::new(belt_bin()).output().expect("run belt-dev");
    assert_eq!(output.status.code(), Some(64));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage:"));
    assert!(stderr.contains("pipeline"));
}

#[test]
fn cli_pipeline_lint_subcommand_is_recognized() {
    let output = Command::new(belt_bin())
        .args(["pipeline", "lint", "--help"])
        .output()
        .expect("run belt-dev pipeline lint --help");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lint"));
}

#[test]
fn cli_pipeline_fmt_subcommand_is_recognized() {
    let output = Command::new(belt_bin())
        .args(["pipeline", "fmt", "--help"])
        .output()
        .expect("run belt-dev pipeline fmt --help");
    assert_eq!(output.status.code(), Some(0));
}
