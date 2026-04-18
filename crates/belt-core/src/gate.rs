use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::model::GateCheck;

/// Resolves a `belt://` URI string to its absolute filesystem path.
///
/// Implemented by `belt_agent::resolver::Resolver` to enable gate-time URI
/// resolution without introducing a `belt-agent` -> `belt-core` cycle.
/// `belt-core` defines the trait; the binary crate implements it. Callers
/// that have no URI semantics (lint, tests against raw fixtures) can pass
/// `&NoopUriResolver` to keep the gate executor a no-op for URI strings.
pub trait UriResolver {
    /// Parse `uri` (a `belt://` URI string) and return its resolved
    /// filesystem path. Implementations MUST NOT assert file existence —
    /// callers handle existence as a separate concern (write targets vs
    /// read targets).
    ///
    /// # Errors
    ///
    /// Returns a string-form error when the URI is malformed or cannot be
    /// resolved (no current run, no completed run for selector, etc.).
    fn resolve(&self, uri: &str) -> Result<PathBuf, String>;
}

/// No-op resolver used by callers without URI semantics (e.g. lint, raw
/// fixture tests). Calling `.resolve()` with a `belt://` URI returns an
/// error; non-URI strings should never reach this resolver in production.
#[derive(Debug, Default)]
pub struct NoopUriResolver;

impl UriResolver for NoopUriResolver {
    fn resolve(&self, uri: &str) -> Result<PathBuf, String> {
        Err(format!(
            "NoopUriResolver cannot resolve '{uri}'; pass a real Resolver"
        ))
    }
}

/// Result of executing a single gate check.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Whether the check was terminated due to timeout.
    #[serde(default)]
    pub timed_out: bool,
}

/// Execute a single gate check.
///
/// # Arguments
/// * `check`      - The gate check variant to evaluate.
/// * `work_dir`   - Working directory for command execution and file lookups.
/// * `output_dir` - Directory where phase outputs are written (used by `has_output`).
/// * `resolver`   - URI resolver used to translate `belt://` URIs in
///   `file_exists` patterns to filesystem paths. Pass `&NoopUriResolver`
///   when no URI semantics apply.
#[must_use]
pub fn execute_gate(
    check: &GateCheck,
    work_dir: &Path,
    output_dir: &Path,
    resolver: &dyn UriResolver,
) -> GateResult {
    match check {
        GateCheck::Cmd { cmd, timeout } => execute_cmd(cmd, work_dir, *timeout),
        GateCheck::FileExists { file_exists } => {
            execute_file_exists(file_exists, work_dir, resolver)
        }
        GateCheck::GitClean { git_clean } => execute_git_clean(*git_clean, work_dir),
        GateCheck::HasOutput { has_output } => execute_has_output(*has_output, output_dir),
        GateCheck::Uses { uses, .. } => GateResult {
            check_type: "uses".to_owned(),
            passed: true,
            detail: Some(format!("uses: {uses} not yet resolved")),
            duration_ms: None,
            timed_out: false,
        },
    }
}

/// Execute all gate checks sequentially and return results.
#[must_use]
pub fn execute_gates(
    checks: &[GateCheck],
    work_dir: &Path,
    output_dir: &Path,
    resolver: &dyn UriResolver,
) -> Vec<GateResult> {
    checks
        .iter()
        .map(|c| execute_gate(c, work_dir, output_dir, resolver))
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

/// Run `sh -c <cmd>` in `work_dir` with a timeout in seconds.
/// `timeout_secs == 0` means no timeout (original behavior).
fn execute_cmd(cmd: &str, work_dir: &Path, timeout_secs: u64) -> GateResult {
    let start = Instant::now();

    if timeout_secs == 0 {
        return execute_cmd_no_timeout(cmd, work_dir, start);
    }

    let mut child = match Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(work_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return GateResult {
                check_type: "cmd".to_owned(),
                passed: false,
                detail: Some(format!("failed to spawn: {e}")),
                duration_ms: Some(elapsed_ms(start)),
                timed_out: false,
            };
        }
    };

    // Read stdout/stderr in threads to prevent pipe buffer deadlock.
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    let stdout_handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut r) = stdout_pipe {
            std::io::Read::read_to_end(&mut r, &mut buf).ok();
        }
        buf
    });
    let stderr_handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut r) = stderr_pipe {
            std::io::Read::read_to_end(&mut r, &mut buf).ok();
        }
        buf
    });

    let deadline = start + std::time::Duration::from_secs(timeout_secs);
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return GateResult {
                    check_type: "cmd".to_owned(),
                    passed: false,
                    detail: Some(format!("try_wait error: {e}")),
                    duration_ms: Some(elapsed_ms(start)),
                    timed_out: false,
                };
            }
        }
    };

    let stderr_bytes = stderr_handle.join().unwrap_or_default();
    let _stdout_bytes = stdout_handle.join().unwrap_or_default();
    let duration_ms = elapsed_ms(start);

    match status {
        None => GateResult {
            check_type: "cmd".to_owned(),
            passed: false,
            detail: Some(format!("timed out after {timeout_secs}s")),
            duration_ms: Some(duration_ms),
            timed_out: true,
        },
        Some(exit_status) => {
            let passed = exit_status.success();
            let detail = if passed {
                None
            } else {
                let stderr = String::from_utf8_lossy(&stderr_bytes);
                let code = exit_status
                    .code()
                    .map_or_else(|| "signal".to_owned(), |c| c.to_string());
                Some(format!("exit {code}: {}", stderr.trim_end()))
            };
            GateResult {
                check_type: "cmd".to_owned(),
                passed,
                detail,
                duration_ms: Some(duration_ms),
                timed_out: false,
            }
        }
    }
}

/// Execute cmd without timeout — uses simple `output()` (original behavior).
fn execute_cmd_no_timeout(cmd: &str, work_dir: &Path, start: Instant) -> GateResult {
    let result = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(work_dir)
        .output();
    let duration_ms = elapsed_ms(start);

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
                timed_out: false,
            }
        }
        Err(e) => GateResult {
            check_type: "cmd".to_owned(),
            passed: false,
            detail: Some(format!("failed to spawn: {e}")),
            duration_ms: Some(duration_ms),
            timed_out: false,
        },
    }
}

/// Helper: elapsed milliseconds since `start`, saturating to u64.
#[allow(clippy::cast_possible_truncation)]
fn elapsed_ms(start: Instant) -> u64 {
    let ms = start.elapsed().as_millis();
    ms.min(u128::from(u64::MAX)) as u64
}

/// Match `pattern` (glob) relative to `work_dir`.  Passes if at least one
/// file matches. When `pattern` starts with `belt://`, the resolver is used
/// to translate it to an absolute filesystem path BEFORE glob expansion;
/// otherwise the pattern is joined with `work_dir` (raw-path behavior).
fn execute_file_exists(pattern: &str, work_dir: &Path, resolver: &dyn UriResolver) -> GateResult {
    let resolved_pattern = if pattern.starts_with("belt://") {
        match resolver.resolve(pattern) {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(e) => {
                return GateResult {
                    check_type: "file_exists".to_owned(),
                    passed: false,
                    detail: Some(format!("URI resolution failed: {e}")),
                    duration_ms: None,
                    timed_out: false,
                };
            }
        }
    } else {
        work_dir.join(pattern).to_string_lossy().to_string()
    };

    match glob::glob(&resolved_pattern) {
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
                timed_out: false,
            }
        }
        Err(e) => GateResult {
            check_type: "file_exists".to_owned(),
            passed: false,
            detail: Some(format!("invalid glob pattern: {e}")),
            duration_ms: None,
            timed_out: false,
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
                timed_out: false,
            }
        }
        Err(e) => GateResult {
            check_type: "git_clean".to_owned(),
            passed: false,
            detail: Some(format!("failed to run git: {e}")),
            duration_ms: Some(duration_ms),
            timed_out: false,
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
        timed_out: false,
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
                timed_out: false,
            },
            GateResult {
                check_type: "b".to_owned(),
                passed: false,
                detail: None,
                duration_ms: None,
                timed_out: false,
            },
        ];
        assert!(!all_passed(&results));
    }
}
