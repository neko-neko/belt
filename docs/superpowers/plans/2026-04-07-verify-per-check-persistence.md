# Verify Per-Check Persistence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist verify and regate per-check results to JSON files under `.belt/runs/`, and surface them in `status` output via `verify_checks` / `regate_checks` fields.

**Architecture:** CLI layer (`cmd_verify`, `cmd_regate`) writes result files after gate execution. `view.rs` reads them at query time for `status`. `GateResult` gains `Deserialize` for file reading. `engine.rs::now_iso8601()` becomes `pub` for timestamp generation.

**Tech Stack:** Rust std, serde, serde_json. No new crates.

---

### File Structure

| File | Change | Responsibility |
|------|--------|----------------|
| `crates/belt-core/src/gate.rs` | Modify | Add `Deserialize` derive to `GateResult` |
| `crates/belt-core/src/engine.rs` | Modify | Make `now_iso8601()` `pub` |
| `crates/belt-core/src/view.rs` | Modify | Add `verify_checks`/`regate_checks` to `PhaseView`, add file reading |
| `crates/belt-agent/src/main.rs` | Modify | Add file writes to `cmd_verify` and `cmd_regate`, add `sanitize_phase_id()` |
| `crates/belt-core/tests/gate_test.rs` | Modify | `GateResult` deserialize tests |
| `crates/belt-core/tests/view_test.rs` | Modify | verify_checks/regate_checks in status tests |
| `crates/belt-agent/tests/cli_test.rs` | Modify | CLI integration tests |

---

### Task 1: GateResult — Add Deserialize

**Files:**
- Modify: `crates/belt-core/src/gate.rs:5,10`
- Test: `crates/belt-core/tests/gate_test.rs`

- [ ] **Step 1: Write failing tests for GateResult deserialization**

Add to `crates/belt-core/tests/gate_test.rs`:

```rust
/// GateResult round-trips through JSON serialization.
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

/// GateResult without timed_out field deserializes with default false.
#[test]
fn gate_result_deserialize_missing_timed_out() {
    let json = r#"{"check_type":"cmd","passed":true,"detail":null,"duration_ms":100}"#;
    let result: belt_core::gate::GateResult = serde_json::from_str(json).expect("deserialize");
    assert!(result.passed);
    assert!(!result.timed_out);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p belt-core gate_result_deserialize 2>&1 | tail -5`
Expected: FAIL — `GateResult` does not implement `Deserialize`.

- [ ] **Step 3: Add Deserialize derive to GateResult**

In `crates/belt-core/src/gate.rs`, change line 5:
```rust
use serde::{Deserialize, Serialize};
```

Change line 10:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p belt-core gate_result_deserialize 2>&1 | tail -5`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/src/gate.rs crates/belt-core/tests/gate_test.rs
git commit -m "feat(gate): add Deserialize derive to GateResult"
```

---

### Task 2: Engine — Make now_iso8601 pub

**Files:**
- Modify: `crates/belt-core/src/engine.rs:342`

- [ ] **Step 1: Change visibility**

In `crates/belt-core/src/engine.rs`, change line 342:
```rust
pub fn now_iso8601() -> String {
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p belt-core 2>&1 | tail -3`
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add crates/belt-core/src/engine.rs
git commit -m "refactor(engine): make now_iso8601() pub for CLI usage"
```

---

### Task 3: CLI — Verify file persistence

**Files:**
- Modify: `crates/belt-agent/src/main.rs:217-251`
- Test: `crates/belt-agent/tests/cli_test.rs`

- [ ] **Step 1: Write failing CLI tests for verify file persistence**

Add to `crates/belt-agent/tests/cli_test.rs`:

```rust
/// verify writes per-check result file.
#[test]
fn verify_writes_result_file() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: persist-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify"]);

    let verify_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("verify/build.json");
    assert!(verify_file.exists(), "verify file should exist");

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&verify_file).unwrap()).unwrap();
    assert_eq!(content["phase"], "build");
    assert_eq!(content["verdict"], "PASS");
    assert!(content["checks"].is_array());
    assert!(content["timestamp"].is_string());
}

/// verify FAIL writes result file with failure details.
#[test]
fn verify_fail_writes_result_file() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: persist-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "false"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify"]);

    let verify_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("verify/build.json");
    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&verify_file).unwrap()).unwrap();
    assert_eq!(content["verdict"], "FAIL");
    assert!(content["checks"][0]["detail"].is_string());
}

/// verify overwrites result file on retry.
#[test]
fn verify_overwrites_on_retry() {
    let dir = TempDir::new().unwrap();
    let marker = dir.path().join("marker.txt");
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: persist-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - file_exists: "marker.txt"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    // First verify: FAIL
    run_belt_agent(&dir, &["verify"]);
    let verify_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("verify/build.json");
    let c1: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&verify_file).unwrap()).unwrap();
    assert_eq!(c1["verdict"], "FAIL");

    // Fix: create marker
    std::fs::write(&marker, "ok").unwrap();

    // Second verify: PASS
    run_belt_agent(&dir, &["verify"]);
    let c2: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&verify_file).unwrap()).unwrap();
    assert_eq!(c2["verdict"], "PASS");
    assert!(
        c2["attempt"].as_u64().unwrap() > c1["attempt"].as_u64().unwrap(),
        "attempt should increment"
    );
}

/// verify file includes timestamp field.
#[test]
fn verify_file_has_timestamp() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: ts-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify"]);

    let verify_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("verify/build.json");
    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&verify_file).unwrap()).unwrap();
    let ts = content["timestamp"].as_str().unwrap();
    assert!(ts.contains('T') && ts.ends_with('Z'), "bad timestamp: {ts}");
}

/// verify result file includes timed_out field from BELT-31.
#[test]
fn verify_result_file_includes_timed_out() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: timeout-persist
version: 1
phases:
  - id: hang
    description: "Hang"
    gate:
      - cmd: "sleep 60"
        timeout: 1
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify"]);

    let verify_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("verify/hang.json");
    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&verify_file).unwrap()).unwrap();
    assert_eq!(content["checks"][0]["timed_out"], true);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p belt-agent verify_writes_result_file 2>&1 | tail -5`
Expected: FAIL — no file written.

- [ ] **Step 3: Implement verify file write in cmd_verify**

In `crates/belt-agent/src/main.rs`, add a helper function near the top (after `belt_dir()`):

```rust
fn sanitize_phase_id(id: &str) -> String {
    id.replace('/', "_")
}

fn write_result_file(belt: &Path, run_id: &str, subdir: &str, phase_id: &str, json: &serde_json::Value) {
    let dir = belt.join("runs").join(run_id).join(subdir);
    if std::fs::create_dir_all(&dir).is_err() {
        eprintln!("warning: failed to create {}", dir.display());
        return;
    }
    let file = dir.join(format!("{}.json", sanitize_phase_id(phase_id)));
    if let Err(e) = std::fs::write(&file, serde_json::to_string_pretty(json).unwrap_or_default()) {
        eprintln!("warning: failed to write {}: {e}", file.display());
    }
}
```

In `cmd_verify` (around line 237), after `verify_verdict` and before stdout output, add:

```rust
    let verify_result = json!({
        "phase": phase.id,
        "verdict": if verdict { "PASS" } else { "FAIL" },
        "checks": results,
        "attempt": attempt,
        "timestamp": belt_core::engine::now_iso8601(),
    });
    write_result_file(&belt_dir(), &state.run_id, "verify", &phase.id, &verify_result);
```

The existing `out` JSON and `println!` remain unchanged for stdout output.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p belt-agent verify_writes_result verify_fail_writes verify_overwrites verify_file_has_timestamp verify_result_file_includes_timed_out 2>&1 | tail -5`
Expected: All PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-agent/src/main.rs crates/belt-agent/tests/cli_test.rs
git commit -m "feat(belt-agent): persist verify per-check results to file"
```

---

### Task 4: CLI — Regate file persistence

**Files:**
- Modify: `crates/belt-agent/src/main.rs:455-465`
- Test: `crates/belt-agent/tests/cli_test.rs`

- [ ] **Step 1: Write failing CLI tests for regate file persistence**

Add to `crates/belt-agent/tests/cli_test.rs`:

```rust
/// regate writes result file with targets.
#[test]
fn regate_writes_result_file() {
    let dir = TempDir::new().unwrap();
    let design_marker = dir.path().join("design.ok");
    std::fs::write(&design_marker, "ok").unwrap();

    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: regate-persist
version: 1
phases:
  - id: design
    description: "Design"
    gate:
      - file_exists: "design.ok"
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
    regate: [design]
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    // Advance past design
    run_belt_agent(&dir, &["verify"]);
    run_belt_agent(&dir, &["step"]);

    // Now at build: verify then regate
    run_belt_agent(&dir, &["verify"]);
    run_belt_agent(&dir, &["regate"]);

    let regate_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("regate/build.json");
    assert!(regate_file.exists(), "regate file should exist");

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&regate_file).unwrap()).unwrap();
    assert_eq!(content["phase"], "build");
    assert!(content["targets"].is_object());
    assert!(content["timestamp"].is_string());
}

/// regate with no targets writes result file.
#[test]
fn regate_no_targets_writes_file() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: no-regate
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify"]);
    run_belt_agent(&dir, &["regate"]);

    let regate_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("regate/build.json");
    assert!(regate_file.exists(), "regate file should exist even with no targets");

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&regate_file).unwrap()).unwrap();
    assert_eq!(content["all_passed"], true);
}

/// regate failure writes result file.
#[test]
fn regate_fail_writes_result_file() {
    let dir = TempDir::new().unwrap();

    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: regate-fail
version: 1
phases:
  - id: design
    description: "Design"
    gate:
      - file_exists: "design.ok"
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
    regate: [design]
"#,
    );

    // Create marker for init, then remove before regate
    let design_marker = dir.path().join("design.ok");
    std::fs::write(&design_marker, "ok").unwrap();

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().unwrap();

    run_belt_agent(&dir, &["verify"]);
    run_belt_agent(&dir, &["step"]);

    // At build: verify passes
    run_belt_agent(&dir, &["verify"]);

    // Remove design marker so regate fails
    std::fs::remove_file(&design_marker).unwrap();
    run_belt_agent(&dir, &["regate"]);

    let regate_file = dir
        .path()
        .join(".belt/runs")
        .join(run_id)
        .join("regate/build.json");
    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&regate_file).unwrap()).unwrap();
    assert_eq!(content["all_passed"], false);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p belt-agent regate_writes_result regate_no_targets_writes regate_fail_writes 2>&1 | tail -5`
Expected: FAIL — no regate file written.

- [ ] **Step 3: Implement regate file write in cmd_regate**

In `cmd_regate`, add file write before stdout output. There are two write points:

**A) No regate targets path** (around line 400-410): After the `json!({ ... "all_passed": true })`, add:

```rust
    let regate_result = json!({
        "phase": phase.id,
        "targets": {},
        "all_passed": true,
        "timestamp": belt_core::engine::now_iso8601(),
    });
    write_result_file(&belt_dir(), &state.run_id, "regate", &phase.id, &regate_result);
```

**B) Normal regate path** (around line 455-460): After `record_regate`, before stdout output, add:

```rust
    let regate_result = json!({
        "phase": phase.id,
        "targets": targets,
        "all_passed": all_passed_flag,
        "timestamp": belt_core::engine::now_iso8601(),
    });
    write_result_file(&belt_dir(), &state.run_id, "regate", &phase.id, &regate_result);
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p belt-agent regate_writes_result regate_no_targets_writes regate_fail_writes 2>&1 | tail -5`
Expected: All PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-agent/src/main.rs crates/belt-agent/tests/cli_test.rs
git commit -m "feat(belt-agent): persist regate per-check results to file"
```

---

### Task 5: View — Add verify_checks/regate_checks to PhaseView

**Files:**
- Modify: `crates/belt-core/src/view.rs:1-5,42-49,93-113,115-140`
- Test: `crates/belt-core/tests/view_test.rs`

- [ ] **Step 1: Write failing tests for verify_checks in status**

Add to `crates/belt-core/tests/view_test.rs`:

```rust
/// status includes verify_checks when verify file exists.
#[test]
fn status_includes_verify_checks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let run_dir = dir.path().join("run1");
    std::fs::create_dir_all(run_dir.join("verify")).unwrap();
    std::fs::write(
        run_dir.join("verify/build.json"),
        r#"{"phase":"build","verdict":"PASS","checks":[{"check_type":"cmd","passed":true,"detail":null,"duration_ms":100,"timed_out":false}],"attempt":1,"timestamp":"2026-04-07T12:00:00Z"}"#,
    )
    .unwrap();

    let state = make_state("build", &[], &[]);
    let view = build_status_view(&state, &["build".to_string()], &run_dir);

    assert!(view.phases[0].verify_checks.is_some());
    let checks = view.phases[0].verify_checks.as_ref().unwrap();
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].check_type, "cmd");
    assert!(checks[0].passed);
}

/// status returns verify_checks = None when no verify file.
#[test]
fn status_verify_checks_none_when_no_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let run_dir = dir.path().join("run1");
    std::fs::create_dir_all(&run_dir).unwrap();

    let state = make_state("build", &[], &[]);
    let view = build_status_view(&state, &["build".to_string()], &run_dir);

    assert!(view.phases[0].verify_checks.is_none());
}

/// status returns verify_checks = None on corrupt file (graceful degradation).
#[test]
fn status_verify_checks_none_on_corrupt_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let run_dir = dir.path().join("run1");
    std::fs::create_dir_all(run_dir.join("verify")).unwrap();
    std::fs::write(run_dir.join("verify/build.json"), "not json{{{").unwrap();

    let state = make_state("build", &[], &[]);
    let view = build_status_view(&state, &["build".to_string()], &run_dir);

    assert!(view.phases[0].verify_checks.is_none());
}

/// status reads verify file for sub-pipeline phase with sanitized ID.
#[test]
fn status_verify_checks_sub_pipeline_phase() {
    let dir = tempfile::tempdir().expect("tempdir");
    let run_dir = dir.path().join("run1");
    std::fs::create_dir_all(run_dir.join("verify")).unwrap();
    std::fs::write(
        run_dir.join("verify/review_triage.json"),
        r#"{"phase":"review/triage","verdict":"FAIL","checks":[{"check_type":"cmd","passed":false,"detail":"exit 1:","duration_ms":50,"timed_out":false}],"attempt":1,"timestamp":"2026-04-07T12:00:00Z"}"#,
    )
    .unwrap();

    let state = make_state("review/triage", &[], &[]);
    let view = build_status_view(&state, &["review/triage".to_string()], &run_dir);

    assert!(view.phases[0].verify_checks.is_some());
    assert!(!view.phases[0].verify_checks.as_ref().unwrap()[0].passed);
}

/// status includes regate_checks when regate file exists.
#[test]
fn status_includes_regate_checks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let run_dir = dir.path().join("run1");
    std::fs::create_dir_all(run_dir.join("regate")).unwrap();
    std::fs::write(
        run_dir.join("regate/audit.json"),
        r#"{"phase":"audit","targets":{"collect":{"passed":true,"checks":[]}},"all_passed":true,"timestamp":"2026-04-07T12:00:00Z"}"#,
    )
    .unwrap();

    let state = make_state("audit", &[], &[]);
    let view = build_status_view(&state, &["audit".to_string()], &run_dir);

    assert!(view.phases[0].regate_checks.is_some());
    let targets = view.phases[0].regate_checks.as_ref().unwrap();
    assert!(targets["collect"]["passed"].as_bool().unwrap());
}

/// status returns regate_checks = None when no regate file.
#[test]
fn status_regate_checks_none_when_no_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let run_dir = dir.path().join("run1");
    std::fs::create_dir_all(&run_dir).unwrap();

    let state = make_state("build", &[], &[]);
    let view = build_status_view(&state, &["build".to_string()], &run_dir);

    assert!(view.phases[0].regate_checks.is_none());
}

/// status returns regate_checks = None on corrupt file.
#[test]
fn status_regate_checks_none_on_corrupt_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let run_dir = dir.path().join("run1");
    std::fs::create_dir_all(run_dir.join("regate")).unwrap();
    std::fs::write(run_dir.join("regate/build.json"), "corrupt!!!").unwrap();

    let state = make_state("build", &[], &[]);
    let view = build_status_view(&state, &["build".to_string()], &run_dir);

    assert!(view.phases[0].regate_checks.is_none());
}
```

Note: `make_state` is a test helper that likely exists in view_test.rs. If not, you need to create a minimal `RunState` for tests. Check the existing test file for the helper pattern.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p belt-core status_includes_verify status_verify_checks_none 2>&1 | tail -5`
Expected: FAIL — `verify_checks` field does not exist on `PhaseView`.

- [ ] **Step 3: Add verify_checks/regate_checks to PhaseView and implement reading**

In `crates/belt-core/src/view.rs`, add import at top:

```rust
use crate::gate::GateResult;
```

Add fields to `PhaseView` (after `outputs`):

```rust
pub struct PhaseView {
    pub id: String,
    pub status: PhaseState,
    pub verify_passed: Option<bool>,
    pub regate_passed: Option<bool>,
    pub attempt: u32,
    pub outputs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify_checks: Option<Vec<GateResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regate_checks: Option<serde_json::Value>,
}
```

Add two helper functions:

```rust
/// Read verify per-check results from file. Returns None on any error.
fn read_verify_checks(run_dir: &Path, phase_id: &str) -> Option<Vec<GateResult>> {
    let file = run_dir
        .join("verify")
        .join(format!("{}.json", phase_id.replace('/', "_")));
    let content = std::fs::read_to_string(&file).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    let checks = parsed.get("checks")?;
    serde_json::from_value(checks.clone()).ok()
}

/// Read regate targets from file. Returns None on any error.
fn read_regate_checks(run_dir: &Path, phase_id: &str) -> Option<serde_json::Value> {
    let file = run_dir
        .join("regate")
        .join(format!("{}.json", phase_id.replace('/', "_")));
    let content = std::fs::read_to_string(&file).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    parsed.get("targets").cloned()
}
```

Update every `PhaseView` construction in `build_status_view()` to include the new fields. There are 3 construction sites:

**A) Main phase loop** (around line 104-112):
```rust
PhaseView {
    id: id.clone(),
    status,
    verify_passed: state.phase_verify_passed.get(id).copied(),
    regate_passed: state.regate_passed.get(id).copied(),
    attempt: state.phase_attempts.get(id).copied().unwrap_or(0),
    outputs: scan_phase_outputs(run_dir, id),
    verify_checks: read_verify_checks(run_dir, id),
    regate_checks: read_regate_checks(run_dir, id),
}
```

**B) Orphan completed phases** (around line 119-126):
```rust
PhaseView {
    id: id.clone(),
    status: PhaseState::Completed,
    verify_passed: state.phase_verify_passed.get(id).copied(),
    regate_passed: state.regate_passed.get(id).copied(),
    attempt: state.phase_attempts.get(id).copied().unwrap_or(0),
    outputs: scan_phase_outputs(run_dir, id),
    verify_checks: read_verify_checks(run_dir, id),
    regate_checks: read_regate_checks(run_dir, id),
}
```

**C) Orphan skipped phases** (around line 131-138):
```rust
PhaseView {
    id: id.clone(),
    status: PhaseState::Skipped,
    verify_passed: None,
    regate_passed: None,
    attempt: 0,
    outputs: Vec::new(),
    verify_checks: None,
    regate_checks: None,
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p belt-core 2>&1 | tail -5`
Expected: All tests pass (existing + 7 new view tests).

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/src/view.rs crates/belt-core/tests/view_test.rs
git commit -m "feat(view): add verify_checks/regate_checks to PhaseView status output"
```

---

### Task 6: CLI integration — status with checks

**Files:**
- Test: `crates/belt-agent/tests/cli_test.rs`

- [ ] **Step 1: Write CLI integration tests for status with checks**

Add to `crates/belt-agent/tests/cli_test.rs`:

```rust
/// status shows verify_checks after verify.
#[test]
fn status_shows_verify_checks_after_verify() {
    let dir = TempDir::new().unwrap();
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: status-checks
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
"#,
    );

    run_belt_agent(&dir, &["init", "pipeline.yml"]);
    run_belt_agent(&dir, &["verify"]);
    let status = run_belt_agent(&dir, &["status"]);

    let checks = &status["phases"][0]["verify_checks"];
    assert!(checks.is_array(), "verify_checks should be array, got: {checks}");
    assert_eq!(checks[0]["check_type"], "cmd");
    assert_eq!(checks[0]["passed"], true);
}

/// status shows regate_checks after regate.
#[test]
fn status_shows_regate_checks_after_regate() {
    let dir = TempDir::new().unwrap();
    let design_marker = dir.path().join("design.ok");
    std::fs::write(&design_marker, "ok").unwrap();

    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: status-regate
version: 1
phases:
  - id: design
    description: "Design"
    gate:
      - file_exists: "design.ok"
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
    regate: [design]
"#,
    );

    run_belt_agent(&dir, &["init", "pipeline.yml"]);
    run_belt_agent(&dir, &["verify"]);
    run_belt_agent(&dir, &["step"]);
    run_belt_agent(&dir, &["verify"]);
    run_belt_agent(&dir, &["regate"]);

    let status = run_belt_agent(&dir, &["status"]);

    let regate = &status["phases"][1]["regate_checks"];
    assert!(regate.is_object(), "regate_checks should be object, got: {regate}");
    assert!(regate["design"]["passed"].as_bool().unwrap());
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p belt-agent status_shows_verify_checks status_shows_regate_checks 2>&1 | tail -5`
Expected: PASS.

- [ ] **Step 3: Run full workspace tests and clippy**

Run: `cargo test --workspace 2>&1 | grep "test result"` and `cargo clippy --workspace -- -D warnings 2>&1 | tail -3`
Expected: All tests pass, no clippy warnings.

- [ ] **Step 4: Commit**

```bash
git add crates/belt-agent/tests/cli_test.rs
git commit -m "test(belt-agent): add CLI integration tests for status with verify/regate checks"
```
