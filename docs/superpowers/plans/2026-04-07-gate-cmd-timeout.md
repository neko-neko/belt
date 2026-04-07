# Gate Cmd Timeout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `timeout` field to `GateCheck::Cmd` that kills the child process after a deadline (default 30 min, 0 = no timeout).

**Architecture:** Add `timeout: u64` to `GateCheck::Cmd` with serde default 1800. Replace `Command::output()` with `Command::spawn()` + `try_wait()` polling loop. Pipe stdout/stderr via reader threads to prevent deadlock. Add `timed_out: bool` to `GateResult`.

**Tech Stack:** Rust std (`std::process`, `std::thread`, `std::sync::mpsc`), no new crates.

---

### File Structure

| File | Change | Responsibility |
|------|--------|----------------|
| `crates/belt-core/src/model.rs` | Modify | Add `timeout` to `GateCheck::Cmd`, add `default_gate_timeout()` |
| `crates/belt-core/src/gate.rs` | Modify | Timeout logic in `execute_cmd()`, add `timed_out` to `GateResult` |
| `crates/belt-core/tests/model_test.rs` | Modify | Deserialization tests for timeout field |
| `crates/belt-core/tests/gate_test.rs` | Modify | Timeout behavior tests |
| `crates/belt-agent/tests/cli_test.rs` | Modify | CLI integration tests for timed_out in JSON |

---

### Task 1: Model — Add timeout to GateCheck::Cmd

**Files:**
- Modify: `crates/belt-core/src/model.rs:67-70`
- Test: `crates/belt-core/tests/model_test.rs`

- [ ] **Step 1: Write failing tests for timeout deserialization**

Add to `crates/belt-core/tests/model_test.rs`:

```rust
/// `cmd: "cargo test"` without timeout field deserializes with default 1800.
#[test]
fn cmd_default_timeout() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: build
    gate:
      - cmd: "cargo test"
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("parse");
    match &pipeline.phases[0].gate[0] {
        GateCheck::Cmd { cmd, timeout } => {
            assert_eq!(cmd, "cargo test");
            assert_eq!(*timeout, 1800);
        }
        other => panic!("expected Cmd, got {other:?}"),
    }
}

/// `{ cmd: "make lint", timeout: 60 }` deserializes with explicit timeout.
#[test]
fn cmd_explicit_timeout() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: build
    gate:
      - cmd: "make lint"
        timeout: 60
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("parse");
    match &pipeline.phases[0].gate[0] {
        GateCheck::Cmd { cmd, timeout } => {
            assert_eq!(cmd, "make lint");
            assert_eq!(*timeout, 60);
        }
        other => panic!("expected Cmd, got {other:?}"),
    }
}

/// `{ cmd: "sleep 999", timeout: 0 }` deserializes with zero (no timeout).
#[test]
fn cmd_timeout_zero() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: build
    gate:
      - cmd: "sleep 999"
        timeout: 0
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("parse");
    match &pipeline.phases[0].gate[0] {
        GateCheck::Cmd { cmd, timeout } => {
            assert_eq!(cmd, "sleep 999");
            assert_eq!(*timeout, 0);
        }
        other => panic!("expected Cmd, got {other:?}"),
    }
}

/// Adding timeout to Cmd does not affect other GateCheck variants.
#[test]
fn cmd_timeout_does_not_affect_other_variants() {
    let yaml = r#"
name: t
version: 1
phases:
  - id: build
    gate:
      - file_exists: "*.md"
      - cmd: "test"
        timeout: 10
"#;
    let pipeline: Pipeline = serde_saphyr::from_str(yaml).expect("parse");
    assert_eq!(pipeline.phases[0].gate.len(), 2);
    match &pipeline.phases[0].gate[0] {
        GateCheck::FileExists { file_exists } => assert_eq!(file_exists, "*.md"),
        other => panic!("expected FileExists, got {other:?}"),
    }
    match &pipeline.phases[0].gate[1] {
        GateCheck::Cmd { cmd, timeout } => {
            assert_eq!(cmd, "test");
            assert_eq!(*timeout, 10);
        }
        other => panic!("expected Cmd, got {other:?}"),
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p belt-core cmd_default_timeout cmd_explicit_timeout cmd_timeout_zero cmd_timeout_does_not_affect_other_variants 2>&1 | tail -5`
Expected: FAIL — `Cmd` variant does not have `timeout` field.

- [ ] **Step 3: Add timeout field to GateCheck::Cmd and default function**

In `crates/belt-core/src/model.rs`, change the `Cmd` variant:

```rust
Cmd {
    cmd: String,
    #[serde(default = "default_gate_timeout")]
    timeout: u64,
},
```

Add at the bottom of the file (before closing):

```rust
/// Default gate command timeout in seconds (30 minutes).
fn default_gate_timeout() -> u64 {
    1800
}
```

- [ ] **Step 4: Fix existing code that constructs GateCheck::Cmd**

All existing `GateCheck::Cmd { cmd: ... }` constructions in test files need the `timeout` field. Search and update:

In `crates/belt-core/tests/gate_test.rs`, update every `GateCheck::Cmd { cmd: "...".to_owned() }` to `GateCheck::Cmd { cmd: "...".to_owned(), timeout: 1800 }`.

Affected tests: `cmd_gate_pass`, `cmd_gate_fail`, `all_passed_integration`.

In `crates/belt-core/tests/model_test.rs`, update match arms from `GateCheck::Cmd { cmd }` to `GateCheck::Cmd { cmd, .. }` (using `..` to ignore timeout in existing tests that don't care about it).

Affected tests: `parse_minimal_pipeline`, `parse_phase_all_fields`, `parse_gate_definition`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p belt-core 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/model.rs crates/belt-core/tests/model_test.rs crates/belt-core/tests/gate_test.rs
git commit -m "feat(model): add timeout field to GateCheck::Cmd (default 1800s)"
```

---

### Task 2: Gate — Add timed_out to GateResult

**Files:**
- Modify: `crates/belt-core/src/gate.rs:9-21`
- Test: `crates/belt-core/tests/gate_test.rs`

- [ ] **Step 1: Write failing tests for timed_out field**

Add to `crates/belt-core/tests/gate_test.rs`:

```rust
/// GateResult for non-cmd gates has timed_out = false.
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

/// GateResult.timed_out serializes to JSON correctly.
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p belt-core gate_result_timed_out 2>&1 | tail -5`
Expected: FAIL — `timed_out` field does not exist on `GateResult`.

- [ ] **Step 3: Add timed_out field to GateResult**

In `crates/belt-core/src/gate.rs`, modify `GateResult`:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct GateResult {
    pub check_type: String,
    pub passed: bool,
    pub detail: Option<String>,
    pub duration_ms: Option<u64>,
    /// Whether the check was terminated due to timeout.
    #[serde(default)]
    pub timed_out: bool,
}
```

Update every `GateResult { ... }` construction in `gate.rs` to include `timed_out: false`. There are 8 construction sites:

1. `execute_cmd` → Ok branch (line 92): add `timed_out: false`
2. `execute_cmd` → Err branch (line 99): add `timed_out: false`
3. `execute_file_exists` → Ok branch (line 121): add `timed_out: false`
4. `execute_file_exists` → Err branch (line 128): add `timed_out: false`
5. `execute_git_clean` → Ok branch (line 161): add `timed_out: false`
6. `execute_git_clean` → Err branch (line 168): add `timed_out: false`
7. `execute_has_output` (line 191): add `timed_out: false`
8. `Uses` match arm (line 36): add `timed_out: false`

Also update unit tests in `gate.rs`:
- `all_passed_empty_slice` — no change needed (doesn't construct GateResult)
- `all_passed_mixed` — add `timed_out: false` to both GateResult constructions

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p belt-core 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/src/gate.rs crates/belt-core/tests/gate_test.rs
git commit -m "feat(gate): add timed_out field to GateResult"
```

---

### Task 3: Gate — Implement timeout logic in execute_cmd

**Files:**
- Modify: `crates/belt-core/src/gate.rs:64-106`
- Test: `crates/belt-core/tests/gate_test.rs`

- [ ] **Step 1: Write failing tests for timeout behavior**

Add to `crates/belt-core/tests/gate_test.rs`:

```rust
/// cmd completes within timeout — passes normally.
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

/// cmd exceeds timeout — killed, timed_out = true.
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

/// Timeout duration_ms reflects the timeout value, not the command's potential runtime.
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

/// Spawn failure with timeout set — not a timeout, but a spawn error.
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
        result.detail.as_deref().unwrap_or("").contains("exit") ||
        result.detail.as_deref().unwrap_or("").contains("not found"),
        "detail: {:?}",
        result.detail
    );
}

/// stderr output preserved on normal failure with timeout.
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p belt-core cmd_timeout_kills cmd_with_timeout_passes 2>&1 | tail -10`
Expected: FAIL — timeout logic not yet implemented.

- [ ] **Step 3: Implement timeout logic in execute_cmd**

Replace `execute_cmd` in `crates/belt-core/src/gate.rs` with:

```rust
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
```

Update the `execute_gate` match arm for `Cmd`:

```rust
GateCheck::Cmd { cmd, timeout } => execute_cmd(cmd, work_dir, *timeout),
```

Remove the old `#[allow(clippy::cast_possible_truncation)]` annotation from the old `execute_cmd` body (it's now in `elapsed_ms`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p belt-core 2>&1 | tail -5`
Expected: All tests pass. Timeout tests (`cmd_timeout_kills_hanging_process`, `cmd_timeout_duration_reflects_timeout`) take ~1-2 seconds each.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/src/gate.rs crates/belt-core/tests/gate_test.rs
git commit -m "feat(gate): implement cmd timeout with try_wait polling

- timeout > 0: spawn + try_wait(100ms) + kill on deadline
- timeout = 0: original Command::output() behavior
- Pipe stdout/stderr via reader threads (deadlock prevention)
- No async runtime, no unsafe, no new crates"
```

---

### Task 4: Gate — execute_gates integration tests

**Files:**
- Test: `crates/belt-core/tests/gate_test.rs`

- [ ] **Step 1: Write integration tests for execute_gates with timeout**

Add to `crates/belt-core/tests/gate_test.rs`:

```rust
use belt_core::gate::execute_gates;

/// One timeout among multiple checks fails the overall result.
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
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p belt-core execute_gates 2>&1 | tail -5`
Expected: All pass. `execute_gates_one_timeout_fails_all` takes ~1 second.

- [ ] **Step 3: Commit**

```bash
git add crates/belt-core/tests/gate_test.rs
git commit -m "test(gate): add execute_gates integration tests with timeout"
```

---

### Task 5: CLI — Integration tests for timed_out in JSON

**Files:**
- Test: `crates/belt-agent/tests/cli_test.rs`

- [ ] **Step 1: Write CLI integration tests**

Add to `crates/belt-agent/tests/cli_test.rs`:

```rust
/// verify outputs timed_out field in JSON for passing cmd.
#[test]
fn verify_outputs_timed_out_field() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: timeout-test
version: 1
phases:
  - id: fast
    description: "Fast check"
    gate:
      - cmd: "true"
        timeout: 5
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    assert!(init["run_id"].is_string());

    let verify = run_belt_agent(&dir, &["verify"]);
    assert_eq!(verify["verdict"], "PASS");
    assert_eq!(verify["checks"][0]["timed_out"], false);
}

/// verify returns FAIL with timed_out = true for hanging cmd.
#[test]
fn verify_timeout_returns_fail() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: timeout-test
version: 1
phases:
  - id: hang
    description: "Hanging check"
    gate:
      - cmd: "sleep 60"
        timeout: 1
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    assert!(init["run_id"].is_string());

    let verify = run_belt_agent(&dir, &["verify"]);
    assert_eq!(verify["verdict"], "FAIL");
    assert_eq!(verify["checks"][0]["timed_out"], true);
    assert!(
        verify["checks"][0]["detail"]
            .as_str()
            .unwrap_or("")
            .contains("timed out"),
        "detail: {}",
        verify["checks"][0]["detail"]
    );
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p belt-agent verify_outputs_timed_out verify_timeout_returns_fail 2>&1 | tail -5`
Expected: All pass.

- [ ] **Step 3: Run full workspace tests**

Run: `cargo test --workspace 2>&1 | tail -10`
Expected: All tests pass.

- [ ] **Step 4: Run clippy and fmt on changed crates**

Run: `cargo clippy -p belt-core -p belt-agent -- -D warnings && cargo fmt --check -p belt-core -p belt-agent`
Expected: No warnings, no format issues.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-agent/tests/cli_test.rs
git commit -m "test(belt-agent): add CLI integration tests for gate timeout"
```
