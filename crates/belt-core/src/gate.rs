use std::path::Path;
use std::process::Command;
use std::time::Instant;

use serde::Serialize;

use crate::model::GateCheck;

/// Result of executing a single gate check.
#[derive(Debug, Clone, Serialize)]
pub struct GateResult {
    /// Human-readable check type (e.g. "cmd", "`file_exists`", "`git_clean`").
    pub check_type: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Optional detail message explaining the result.
    pub detail: Option<String>,
    /// Wall-clock duration in milliseconds (populated for checks that run
    /// external processes).
    pub duration_ms: Option<u64>,
}

/// Execute a single gate check.
///
/// # Arguments
/// * `check`      - The gate check variant to evaluate.
/// * `work_dir`   - Working directory for command execution and file lookups.
/// * `output_dir` - Directory where phase outputs are written (used by `has_output`).
#[must_use]
pub fn execute_gate(check: &GateCheck, work_dir: &Path, output_dir: &Path) -> GateResult {
    match check {
        GateCheck::Cmd { cmd, .. } => execute_cmd(cmd, work_dir),
        GateCheck::FileExists { file_exists } => execute_file_exists(file_exists, work_dir),
        GateCheck::GitClean { git_clean } => execute_git_clean(*git_clean, work_dir),
        GateCheck::HasOutput { has_output } => execute_has_output(*has_output, output_dir),
        GateCheck::Uses { uses, .. } => GateResult {
            check_type: "uses".to_owned(),
            passed: true,
            detail: Some(format!("uses: {uses} not yet resolved")),
            duration_ms: None,
        },
    }
}

/// Execute all gate checks sequentially and return results.
#[must_use]
pub fn execute_gates(checks: &[GateCheck], work_dir: &Path, output_dir: &Path) -> Vec<GateResult> {
    checks
        .iter()
        .map(|c| execute_gate(c, work_dir, output_dir))
        .collect()
}

/// Return `true` if every result in the slice passed.
#[must_use]
pub fn all_passed(results: &[GateResult]) -> bool {
    results.iter().all(|r| r.passed)
}

// ---------------------------------------------------------------------------
// Per-variant handlers
// ---------------------------------------------------------------------------

/// Run `sh -c <cmd>` in `work_dir` and check exit status.
fn execute_cmd(cmd: &str, work_dir: &Path) -> GateResult {
    let start = Instant::now();
    let result = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(work_dir)
        .output();
    let elapsed = start.elapsed().as_millis();

    // Saturating cast: u128 -> u64 (overflow extremely unlikely for
    // wall-clock durations, but we stay safe).
    #[allow(clippy::cast_possible_truncation)]
    let duration_ms = elapsed.min(u128::from(u64::MAX)) as u64;

    match result {
        Ok(output) => {
            let passed = output.status.success();
            let detail = if passed {
                None
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let code = output
                    .status
                    .code()
                    .map_or_else(|| "signal".to_owned(), |c| c.to_string());
                Some(format!("exit {code}: {}", stderr.trim_end()))
            };
            GateResult {
                check_type: "cmd".to_owned(),
                passed,
                detail,
                duration_ms: Some(duration_ms),
            }
        }
        Err(e) => GateResult {
            check_type: "cmd".to_owned(),
            passed: false,
            detail: Some(format!("failed to spawn: {e}")),
            duration_ms: Some(duration_ms),
        },
    }
}

/// Match `pattern` (glob) relative to `work_dir`.  Passes if at least one
/// file matches.
fn execute_file_exists(pattern: &str, work_dir: &Path) -> GateResult {
    let full_pattern = work_dir.join(pattern).to_string_lossy().to_string();
    match glob::glob(&full_pattern) {
        Ok(paths) => {
            let matches: Vec<_> = paths.filter_map(Result::ok).collect();
            let passed = !matches.is_empty();
            let detail = if passed {
                Some(format!("matched {} file(s)", matches.len()))
            } else {
                Some(format!("no files matched pattern: {pattern}"))
            };
            GateResult {
                check_type: "file_exists".to_owned(),
                passed,
                detail,
                duration_ms: None,
            }
        }
        Err(e) => GateResult {
            check_type: "file_exists".to_owned(),
            passed: false,
            detail: Some(format!("invalid glob pattern: {e}")),
            duration_ms: None,
        },
    }
}

/// Run `git status --porcelain` in `work_dir`.  If `expect_clean` is true,
/// the check passes when the output is empty (working tree is clean).
fn execute_git_clean(expect_clean: bool, work_dir: &Path) -> GateResult {
    let start = Instant::now();
    let result = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(work_dir)
        .output();
    let elapsed = start.elapsed().as_millis();

    #[allow(clippy::cast_possible_truncation)]
    let duration_ms = elapsed.min(u128::from(u64::MAX)) as u64;

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let is_clean = stdout.trim().is_empty();
            let passed = is_clean == expect_clean;
            let detail = if is_clean {
                Some("working tree clean".to_owned())
            } else {
                let changed = stdout.lines().count();
                Some(format!("{changed} file(s) with uncommitted changes"))
            };
            GateResult {
                check_type: "git_clean".to_owned(),
                passed,
                detail,
                duration_ms: Some(duration_ms),
            }
        }
        Err(e) => GateResult {
            check_type: "git_clean".to_owned(),
            passed: false,
            detail: Some(format!("failed to run git: {e}")),
            duration_ms: Some(duration_ms),
        },
    }
}

/// Check whether `output_dir` contains at least one file.  If `expect_output`
/// is true, the check passes when the directory is non-empty.
fn execute_has_output(expect_output: bool, output_dir: &Path) -> GateResult {
    let has_files = std::fs::read_dir(output_dir)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false);

    let passed = has_files == expect_output;
    let detail = if has_files {
        Some("output directory is non-empty".to_owned())
    } else {
        Some("output directory is empty or does not exist".to_owned())
    };

    GateResult {
        check_type: "has_output".to_owned(),
        passed,
        detail,
        duration_ms: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_passed_empty_slice() {
        assert!(all_passed(&[]));
    }

    #[test]
    fn all_passed_mixed() {
        let results = vec![
            GateResult {
                check_type: "a".to_owned(),
                passed: true,
                detail: None,
                duration_ms: None,
            },
            GateResult {
                check_type: "b".to_owned(),
                passed: false,
                detail: None,
                duration_ms: None,
            },
        ];
        assert!(!all_passed(&results));
    }
}
