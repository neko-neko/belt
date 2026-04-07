# BELT-30: verify per-check result persistence

**Linear**: [BELT-30](https://linear.app/neko-neko/issue/BELT-30)
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)
**Depends on**: BELT-29 (status enrichment, Done), BELT-31 (gate timeout, Done)
**Date**: 2026-04-07

## Summary

Persist verify and regate per-check results to `.belt/runs/{run_id}/verify/{phase_id}.json` and `.belt/runs/{run_id}/regate/{phase_id}.json`. Integrate into `status` output via `verify_checks` and `regate_checks` fields in `PhaseView`.

## Background

Currently, `belt-agent verify` executes gate checks and outputs per-check results to stdout, but results are not persisted. After the verify command completes, the only trace is `phase_verify_passed: bool` in `state.json`. When regate fails, the LLM has no way to check "what failed in the previous verify" without re-running verify.

File-based persistence follows belt's existing data flow pattern (`output_dir` for phase outputs, `.belt/runs/` for run state). verify already has side effects (writing to `state.json` via `verify_verdict()`), so file persistence is consistent.

## Design

### File Paths

```
.belt/runs/{run_id}/
├── verify/
│   ├── build.json
│   ├── review_triage.json      # sub-pipeline phase: / replaced with _
│   └── deploy.json
├── regate/
│   ├── build.json
│   └── deploy.json
├── state.json
└── {phase_id}/                  # existing output_dir
```

Phase IDs containing `/` (from sub-pipeline expansion) are sanitized by replacing `/` with `_`, consistent with the existing `output_dir` convention in `cmd_regate`.

### File Format

**verify/{phase_id}.json:**
```json
{
  "phase": "build",
  "verdict": "FAIL",
  "checks": [
    {
      "check_type": "cmd",
      "passed": true,
      "detail": null,
      "duration_ms": 234,
      "timed_out": false
    },
    {
      "check_type": "file_exists",
      "passed": false,
      "detail": "no files matched pattern: dist/*.tar.gz",
      "duration_ms": null,
      "timed_out": false
    }
  ],
  "attempt": 2,
  "timestamp": "2026-04-07T15:30:00Z"
}
```

**regate/{phase_id}.json:**
```json
{
  "phase": "audit",
  "targets": {
    "collect": {
      "passed": false,
      "checks": [
        {
          "check_type": "file_exists",
          "passed": false,
          "detail": "no files matched pattern: .belt/collected-context.json",
          "duration_ms": null,
          "timed_out": false
        }
      ]
    }
  },
  "all_passed": false,
  "timestamp": "2026-04-07T15:35:00Z"
}
```

### Write Responsibility

| Layer | Responsibility | Change |
|-------|---------------|--------|
| `gate.rs` | Execute gates, return `Vec<GateResult>` | **No change** (add `Deserialize` derive to `GateResult`) |
| `engine.rs` | `verify_verdict()` records verdict in state.json | **No change** |
| `cmd_verify` (main.rs) | stdout output + file write | **New**: write to `verify/{phase_id}.json` |
| `cmd_regate` (main.rs) | stdout output + file write | **New**: write to `regate/{phase_id}.json` |

The CLI layer writes the file after gate execution and before stdout output. This keeps `gate.rs` and `engine.rs` pure.

### File Write Logic (cmd_verify)

1. Construct verify result JSON (same as stdout output)
2. Add `timestamp` field
3. Compute file path: `{belt_dir}/runs/{run_id}/verify/{phase_id_sanitized}.json`
4. Create directory if not exists: `std::fs::create_dir_all()`
5. Write JSON: `std::fs::write()`
6. Proceed with stdout output (existing behavior)

File write failure is non-fatal: log to stderr and continue. verify must not fail because of a file write error.

### GateResult Deserialize

`GateResult` needs `Deserialize` derive for `view.rs` to read the persisted files:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub check_type: String,
    pub passed: bool,
    pub detail: Option<String>,
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub timed_out: bool,
}
```

### Status Integration (view.rs)

Add to `PhaseView`:

```rust
pub struct PhaseView {
    pub id: String,
    pub status: PhaseState,
    pub verify_passed: Option<bool>,
    pub regate_passed: Option<bool>,
    pub attempt: u32,
    pub outputs: Vec<String>,
    pub verify_checks: Option<Vec<GateResult>>,   // NEW
    pub regate_checks: Option<serde_json::Value>,  // NEW
}
```

`verify_checks` is `Option<Vec<GateResult>>` — parsed from `verify/{phase_id}.json` `checks` array. `None` if file doesn't exist.

`regate_checks` is `Option<serde_json::Value>` — the `targets` object from `regate/{phase_id}.json`. Kept as opaque JSON because regate targets structure varies. `None` if file doesn't exist.

**Read logic in `build_status_view()`:**

For each phase:
1. Try to read `{run_dir}/verify/{phase_id_sanitized}.json`
2. If exists and parses: extract `checks` array → `verify_checks`
3. If missing or parse error: `verify_checks = None`
4. Same for `{run_dir}/regate/{phase_id_sanitized}.json` → `regate_checks` (extract `targets`)

Graceful degradation: parse errors result in `None`, not errors. Status must never fail due to corrupt verify/regate files.

### Timestamp Generation

Reuse `engine.rs::now_iso8601()` — but it's currently `pub(crate)` in belt-core. Two options:

A) Make `now_iso8601()` `pub` and use from CLI
B) Duplicate a simple timestamp function in CLI

**Choice: A** — `now_iso8601()` is a utility with no state. Making it `pub` is appropriate.

### Phase ID Sanitization

Phase IDs from sub-pipeline expansion contain `/` (e.g., `review/triage`). File paths sanitize by replacing `/` with `_` (e.g., `review_triage.json`). This is already the convention in `cmd_regate` for output directories.

Extract a shared helper: `fn sanitize_phase_id(id: &str) -> String { id.replace('/', "_") }`

## What Changes

| File | Change |
|------|--------|
| `crates/belt-core/src/gate.rs` | Add `Deserialize` derive to `GateResult` |
| `crates/belt-core/src/view.rs` | Add `verify_checks` / `regate_checks` to `PhaseView`, add file reading logic |
| `crates/belt-core/src/engine.rs` | Make `now_iso8601()` `pub` (was `pub(crate)`) |
| `crates/belt-agent/src/main.rs` | `cmd_verify`: add file write. `cmd_regate`: add file write. Extract `sanitize_phase_id()`. |
| `crates/belt-core/tests/view_test.rs` | New tests for verify_checks/regate_checks in status |
| `crates/belt-core/tests/gate_test.rs` | Test GateResult Deserialize round-trip |
| `crates/belt-agent/tests/cli_test.rs` | CLI integration tests |

## What Does NOT Change

| Item | Reason |
|------|--------|
| `engine.rs` logic | state.json schema unchanged |
| `model.rs` | RunState unchanged |
| `gate.rs` execution logic | Only derive added |
| `verify` stdout output | File write is additive |
| `regate` stdout output | File write is additive |

## Test Plan

### A. GateResult Deserialize (gate_test.rs, 2 tests)

| # | Test | Setup | Expected |
|---|------|-------|----------|
| 1 | `gate_result_deserialize_round_trip` | Serialize GateResult to JSON, deserialize back | All fields preserved including `timed_out` |
| 2 | `gate_result_deserialize_missing_timed_out` | JSON without `timed_out` field | Deserializes with `timed_out: false` (serde default) |

### B. Verify file persistence (cli_test.rs, 5 tests)

| # | Test | Setup | Expected |
|---|------|-------|----------|
| 3 | `verify_writes_result_file` | init → verify (PASS) | `.belt/runs/{id}/verify/{phase}.json` exists, contains `verdict: "PASS"`, `checks` array |
| 4 | `verify_fail_writes_result_file` | init → verify (FAIL) | File exists, `verdict: "FAIL"`, failing check detail present |
| 5 | `verify_overwrites_on_retry` | init → verify (FAIL) → fix → verify (PASS) | File updated with `verdict: "PASS"`, attempt incremented |
| 6 | `verify_file_has_timestamp` | init → verify | File contains `timestamp` field, non-empty string |
| 7 | `verify_sub_pipeline_phase_sanitized` | Pipeline with `uses:` → verify on expanded phase | File path uses `_` instead of `/` in phase ID |

### C. Regate file persistence (cli_test.rs, 3 tests)

| # | Test | Setup | Expected |
|---|------|-------|----------|
| 8 | `regate_writes_result_file` | init → verify → regate (targets exist) | `.belt/runs/{id}/regate/{phase}.json` exists, contains `targets` object |
| 9 | `regate_no_targets_writes_file` | init → verify → regate (no targets) | File exists, `targets: {}`, `all_passed: true` |
| 10 | `regate_fail_writes_result_file` | init → verify → regate (target fails) | File exists, target `passed: false`, `all_passed: false` |

### D. Status integration — verify_checks (view_test.rs, 4 tests)

| # | Test | Setup | Expected |
|---|------|-------|----------|
| 11 | `status_includes_verify_checks` | Write valid verify JSON file → build status view | `verify_checks` contains parsed checks array |
| 12 | `status_verify_checks_none_when_no_file` | No verify file exists → build status view | `verify_checks: null` in JSON |
| 13 | `status_verify_checks_none_on_corrupt_file` | Write invalid JSON to verify file → build status view | `verify_checks: null` (graceful degradation) |
| 14 | `status_verify_checks_sub_pipeline_phase` | Write verify file with sanitized sub-pipeline ID → build status view | Correctly reads `review_triage.json` for phase `review/triage` |

### E. Status integration — regate_checks (view_test.rs, 3 tests)

| # | Test | Setup | Expected |
|---|------|-------|----------|
| 15 | `status_includes_regate_checks` | Write valid regate JSON file → build status view | `regate_checks` contains targets object |
| 16 | `status_regate_checks_none_when_no_file` | No regate file exists → build status view | `regate_checks: null` |
| 17 | `status_regate_checks_none_on_corrupt_file` | Write invalid JSON to regate file → build status view | `regate_checks: null` |

### F. CLI integration — status with checks (cli_test.rs, 2 tests)

| # | Test | Setup | Expected |
|---|------|-------|----------|
| 18 | `status_shows_verify_checks_after_verify` | init → verify → status | `phases[0].verify_checks` is non-null array |
| 19 | `status_shows_regate_checks_after_regate` | init → verify → regate → status | `phases[0].regate_checks` is non-null object |

### G. Edge cases (cli_test.rs, 2 tests)

| # | Test | Setup | Expected |
|---|------|-------|----------|
| 20 | `verify_file_write_failure_non_fatal` | init → make verify dir read-only → verify | verify still succeeds (stdout output), file write error logged to stderr |
| 21 | `verify_result_file_includes_timed_out` | Pipeline with `cmd: "sleep 60", timeout: 1` → verify | File contains `timed_out: true` in checks |

## Known Limitations

- **File size**: Large pipelines with many gate checks may produce sizable verify files. Not a concern for current use cases (< 10 checks per phase).
- **Concurrent writes**: No file locking. Two concurrent verify calls on the same run could corrupt the file. Acceptable for current single-process design.
- **regate_checks as opaque JSON**: `regate_checks` is `serde_json::Value` rather than a typed struct because regate targets structure (per-target checks) is more complex. This is a pragmatic choice — typed parsing can be added later if needed.
