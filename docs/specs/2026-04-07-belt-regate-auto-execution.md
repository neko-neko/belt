# Regate Auto-Execution via Independent Command

**Linear**: BELT-24 (to be created)
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)
**Status**: Draft
**Date**: 2026-04-07

## Summary

Add `belt-agent regate` as an independent command that deterministically re-executes gate checks for regate target phases, replacing the current LLM-goodwill-based approach. Regate results are persisted in `RunState` and enforced as a step guard, consistent with the existing verify-before-step pattern.

## Background

### Problem

The current MVP stores `regate` targets in the data model and returns them in `cmd_next` JSON output, but never executes them. The intent — "re-verify target phase gates after the current phase passes" — relies entirely on the LLM agent following skill protocol instructions. This contradicts belt's core design philosophy: **gate execution is belt's deterministic control, not LLM self-discipline**.

### Motivating Example

In `examples/skills/linear-refresh/pipeline.yml`:

```yaml
- id: audit
  gate:
    - file_exists: ".belt/refresh-plan.json"
  validate:
    - "Every In Progress ticket's latest context is reflected in the plan"
  regate: [collect]
  max_retries: 2
```

When `audit` verify passes, belt should automatically re-execute `collect`'s gates (`file_exists: ".belt/collected-context.json"`) to confirm collect's outputs remain valid. If regate fails, `step` is blocked — the agent must fix the target's outputs and retry.

### External Feedback (MVP Review)

> regate こそ belt の決定論的制御に落とすべき。verify が PASS した後、regate 対象フェーズの gate を自動再実行して、結果を JSON に含めて返すだけ。実装量は少ないはず（execute_gates を regate 対象に対して呼ぶだけ）。

This spec addresses the highest-priority item from the MVP review feedback.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Command separation | Independent `regate` command, not inline in `verify` | **Context neutrality**: belt must not assume single vs. multi-context orchestration. Separate command works for both. Each command does one thing (Linux philosophy). |
| Regate state | `regate_passed: HashMap<String, bool>` in RunState | Same pattern as `phase_verify_passed`. Per-phase tracking. |
| Regate FAIL behavior | Blocks `step` | Gate enforcement is deterministic. Agreed as design principle. |
| Verify resets regate | `verify_verdict()` clears `regate_passed` for current phase | Forces verify → regate ordering through state management. |
| Step guard order | verify → regate → max_retries | Cannot evaluate regate without verify; cannot evaluate retries without both. |
| Regate before verify | CLI returns error JSON (not engine-enforced) | Soft constraint for UX. System is safe even without: step guard catches the inconsistency. |
| Engine API | `record_regate(state, all_passed)` — stores only | Follows existing pattern: CLI orchestrates execution, engine manages state. |
| Skipped regate target | Auto-passed (skipped targets produce no outputs to verify) | Aligned with redesign spec: "regate 対象フェーズが when: で無効なら自動スキップ" |
| Regate FAIL recovery | Requires re-verify before re-regate | Fixing target outputs may invalidate current phase. Belt enforces correctness over efficiency. |
| max_retries scope | Counts `verify_verdict()` calls only | Regate failures do not directly increment attempt count. Each fix cycle (regate FAIL → fix → verify) costs one verify attempt. |
| Regate target with empty gate | Treated as passed (`all_passed(&[])` returns `true`) | Consistent with gate-less phase auto-verify behavior. |

### Context Neutrality Principle

Belt's command design must be **agnostic to context strategy**:
- Single LLM context running all phases: `next → verify → regate → step` in sequence
- Multiple contexts with handoff: orchestrator dispatches regate to a different context
- Belt does not know or care which model applies

Making `regate` an independent command (not embedded in `verify`) is essential for this. The skill/orchestrator decides recovery strategy when regate fails.

## New Command: `belt-agent regate`

### CLI Definition

```
belt-agent regate [--run <run_id>]
```

### Behavior

1. Load `RunState` and resolve current phase
2. **Pre-check**: If `current_phase == "COMPLETED"`, return error JSON (`"error": "pipeline_completed"`)
3. **Pre-check**: If `phase_verify_passed[current_phase] != true`, return error JSON (`"error": "verify_not_passed"`)
4. Get current phase's `regate` targets from expanded pipeline
5. If no targets: return empty success (`all_passed: true`)
6. For each target:
   a. Find target phase in expanded pipeline. If not found → runtime error
   b. If target is in `state.skipped_phases` → treat as passed (auto-skip, no gate execution)
   c. Otherwise: execute `execute_gates(&target.gate, work_dir, output_dir)`
      - `work_dir`: current working directory (same as verify)
      - `output_dir`: `{belt_dir}/runs/{run_id}/{target_id.replace('/', '_')}` (target phase's output directory)
   d. If target gate is empty (`gate: []`) → treated as passed (consistent with `all_passed(&[])`)
7. Compute `all_passed` from all target results
8. Call `engine.record_regate(state, all_passed)` to persist
9. Output JSON

### JSON Output

**Success (targets exist):**
```json
{
  "run_id": "01961a2b-...",
  "phase": "audit",
  "targets": {
    "collect": {
      "passed": false,
      "checks": [
        {
          "check_type": "file_exists",
          "passed": false,
          "detail": "no files matched pattern: .belt/collected-context.json",
          "duration_ms": null
        }
      ]
    }
  },
  "all_passed": false
}
```

**No regate targets:**
```json
{
  "run_id": "01961a2b-...",
  "phase": "build",
  "targets": {},
  "all_passed": true
}
```

**Verify not passed:**
```json
{
  "error": "verify_not_passed",
  "phase": "audit",
  "message": "verify must pass before regate"
}
```

## RunState Changes (`model.rs`)

```rust
pub struct RunState {
    // ... existing fields ...
    #[serde(default)]
    pub phase_verify_passed: HashMap<String, bool>,
    #[serde(default)]
    pub regate_passed: HashMap<String, bool>,  // NEW
    // ...
}
```

Backward compatible: `#[serde(default)]` means existing `state.json` files deserialize with an empty map.

## Engine Changes (`engine.rs`)

### New Method: `record_regate()`

```rust
pub fn record_regate(&self, state: &mut RunState, all_passed: bool) -> BeltResult<()> {
    state.regate_passed.insert(state.current_phase.clone(), all_passed);
    state.updated_at = now_iso8601();
    self.save_state(state)?;
    Ok(())
}
```

### Modified: `verify_verdict()`

Clear regate state when verify is (re-)run:

```rust
pub fn verify_verdict(&self, state: &mut RunState, passed: bool) -> BeltResult<bool> {
    // ... existing attempt increment ...
    state.phase_verify_passed.insert(state.current_phase.clone(), passed);
    state.regate_passed.remove(&state.current_phase);  // NEW: force re-regate
    // ... save ...
}
```

### Modified: `step()` — Guard Order

```rust
// Guard 1: verify-before-step (existing)
if state.phase_verify_passed.get(&state.current_phase) != Some(&true) {
    return Err(BeltError::VerifyRequired { ... });
}

// Guard 2: regate-before-step (NEW)
if !current_phase_def.regate.is_empty() {
    match state.regate_passed.get(&state.current_phase) {
        Some(true) => {},  // OK
        Some(false) => return Err(BeltError::RegateFailed {
            phase_id: state.current_phase.clone(),
            targets: current_phase_def.regate.clone(),
        }),
        None => return Err(BeltError::RegateRequired {
            phase_id: state.current_phase.clone(),
            targets: current_phase_def.regate.clone(),
        }),
    }
}

// Guard 3: max_retries (existing)
```

## Error Types (`error.rs`)

```rust
#[error("regate required for phase '{phase_id}': run regate before step")]
#[diagnostic(code(belt::regate_required))]
RegateRequired { phase_id: String, targets: Vec<String> },

#[error("regate failed for phase '{phase_id}': targets {targets:?} did not pass")]
#[diagnostic(code(belt::regate_failed))]
RegateFailed { phase_id: String, targets: Vec<String> },
```

## belt-agent JSON Output for Step Errors (`main.rs`)

**RegateRequired:**
```json
{
  "advanced": false,
  "reason": "regate_not_executed",
  "phase": "audit",
  "regate_targets": ["collect"]
}
```

**RegateFailed:**
```json
{
  "advanced": false,
  "reason": "regate_failed",
  "phase": "audit",
  "regate_targets": ["collect"]
}
```

## Flow Diagrams

### Phase without regate (unchanged)

```
next → work → verify(PASS) → step ✓
```

### Phase with regate

```
next → work → verify(PASS) → regate(PASS) → step ✓
                                ↓ FAIL
                         skill decides recovery
                         fix targets → verify → regate → step
```

### Regate retry loop with max_retries

```
                    ┌────────────────────────────────────┐
                    │                                    │
next → work → verify(PASS) → regate(FAIL) ─── fix ─────┘
                                         (attempt++ via verify)
              ... after max_retries exceeded ...
         step → "max_retries_exceeded" + escalation: true
```

### Why regate FAIL requires re-verify (not just re-regate)

`verify_verdict()` clears `regate_passed`, forcing the verify → regate sequence. This is intentional:

1. Fixing a regate target's outputs may invalidate the current phase's work (e.g., fixing `collect` data may require re-auditing in `audit`)
2. Belt enforces correctness: re-verify is cheap (gate checks only) and ensures the current phase's state is still valid
3. Each re-verify increments `phase_attempts`, providing a natural bound via `max_retries`

Consequence: re-regate without re-verify is not possible by design. Each fix cycle costs one verify attempt.

### Full Guard Evaluation Order at `step`

Guards are evaluated in two layers. Earlier guards take priority:

| Layer | Guard | Error | Checked By |
|-------|-------|-------|------------|
| 1 (CLI) | confirm/validate acknowledgment | `confirmation_required` | `cmd_step` in main.rs |
| 2 (Engine) | verify-before-step | `verify_required` | `Engine::step()` |
| 3 (Engine) | regate-before-step | `regate_not_executed` / `regate_failed` | `Engine::step()` |
| 4 (Engine) | max_retries | `max_retries_exceeded` + `escalation: true` | `Engine::step()` |

## What Changes

| File | Change |
|------|--------|
| `crates/belt-core/src/model.rs` | Add `regate_passed: HashMap<String, bool>` to `RunState` |
| `crates/belt-core/src/error.rs` | Add `RegateRequired`, `RegateFailed` variants |
| `crates/belt-core/src/engine.rs` | Add `record_regate()`, modify `verify_verdict()` (clear regate), modify `step()` (regate guard) |
| `crates/belt-agent/src/main.rs` | Add `Regate` subcommand + `cmd_regate()`, add step error handling for regate errors |
| `crates/belt-core/tests/engine_test.rs` | New test cases |
| `crates/belt-agent/tests/cli_test.rs` | New CLI integration tests |
| `crates/belt-core/tests/fixtures/` | Update `regate_pipeline.yml` or add new fixtures |

## What Does NOT Change

| Item | Reason |
|------|--------|
| `verify` command | Runs current phase gates only. No regate logic. |
| `next` command | Already returns `regate` field. No change needed. |
| `init` command | No regate state at init time. |
| `belt` (human CLI) | Lint already validates regate targets. No change. |
| `gate.rs` | `execute_gates()` is reused as-is for regate targets. |
| `expander.rs` | Regate inheritance in sub-pipeline expansion is unchanged. |

## Test Plan (28 cases)

### A. Engine: Regate State Management (4 tests)

| # | Test Name | Setup | Assertion |
|---|-----------|-------|-----------|
| 1 | `init_does_not_set_regate_passed` | init with regate pipeline | `state.regate_passed` is empty |
| 2 | `record_regate_stores_result` | verify(PASS) → record_regate(true) | `state.regate_passed[phase] == true` |
| 3 | `verify_clears_regate_passed` | verify(PASS) → record_regate(PASS) → verify again | `regate_passed` entry removed for current phase |
| 4 | `regate_passed_persists_across_save_load` | record_regate(true) → save → load | `regate_passed[phase] == true` preserved |

### B. Engine: Step Guard — regate-before-step (6 tests)

| # | Test Name | Setup | Expected |
|---|-----------|-------|----------|
| 5 | `step_requires_regate_when_targets_exist` | verify(PASS), regate not run | `RegateRequired { targets: ["design"] }` |
| 6 | `step_blocked_when_regate_failed` | verify(PASS), record_regate(false) | `RegateFailed { targets: ["design"] }` |
| 7 | `step_succeeds_after_verify_and_regate_pass` | verify(PASS) → record_regate(true) → step | Ok(Some(next_phase)) |
| 8 | `step_succeeds_without_regate_when_no_targets` | phase without regate, verify(PASS) → step | Ok (existing behavior, regression guard) |
| 9 | `verify_guard_priority_over_regate_guard` | verify not run, regate not run | `VerifyRequired` (not `RegateRequired`) |
| 10 | `regate_guard_priority_over_max_retries` | verify(PASS), regate not run, max_retries exceeded | `RegateRequired` (not `MaxRetriesExceeded`) |

### C. Engine: Verify-Regate Interaction (4 tests)

| # | Test Name | Setup | Expected |
|---|-----------|-------|----------|
| 11 | `verify_regate_reverify_resets_regate` | verify → regate(PASS) → re-verify → step | `RegateRequired` (regate was cleared) |
| 12 | `regate_fail_then_reverify_clears_state` | regate(FAIL) → verify again | `regate_passed` entry removed; fresh retry possible |
| 13 | `multiple_regate_targets_partial_fail` | 2 targets, target_a passes, target_b fails → record_regate(false) | `regate_passed[phase] == false`, step blocked |
| 14 | `record_regate_idempotent` | record_regate(true) → record_regate(true) | Same state, no error |

### D. belt-agent CLI: cmd_regate (5 tests)

| # | Test Name | Setup | Expected JSON |
|---|-----------|-------|---------------|
| 15 | `regate_command_runs_target_gates` | init → touch gate files → verify(PASS) → regate | `targets.design.passed: true`, `all_passed: true` |
| 16 | `regate_command_target_gate_fails` | init → verify(PASS) → regate (target gate file missing) | `targets.design.passed: false`, `all_passed: false` |
| 17 | `regate_no_targets_returns_empty` | phase without regate → regate | `targets: {}`, `all_passed: true` |
| 18 | `regate_before_verify_returns_error` | init → regate (without verify) | `error: "verify_not_passed"` |
| 19 | `step_json_regate_not_executed` | verify(PASS) → step (skip regate) | `reason: "regate_not_executed"`, `regate_targets: [...]` |

### E. Full Lifecycle (3 tests)

| # | Test Name | Fixture | Scenario |
|---|-----------|---------|----------|
| 20 | `regate_pipeline_full_lifecycle` | `regate_pipeline.yml` | init → (design: verify → step) → (build: verify → regate → step) → (test: verify → step) → COMPLETED |
| 21 | `regate_fail_retry_lifecycle` | `regate_pipeline.yml` | build: verify(PASS) → regate(FAIL) → fix target → verify(PASS) → regate(PASS) → step |
| 22 | `regate_with_max_retries_escalation` | new `regate_max_retries_pipeline.yml` | regate loop exhausts max_retries → step returns escalation JSON |

### F. Edge Cases / Boundary Values (6 tests)

| # | Test Name | Setup | Expected |
|---|-----------|-------|----------|
| 23 | `gateless_phase_with_regate_targets` | phase: no gate, regate: [target] | auto-verify(PASS), must still regate before step |
| 24 | `regate_target_not_found_returns_error` | regate target ID removed from pipeline between init and regate | Runtime error with clear message |
| 25 | `regate_target_skipped_phase_auto_passed` | regate target has `when: "args.x"`, args.x=false → target skipped | Skipped target treated as passed, not gate-executed |
| 26 | `regate_target_with_empty_gate` | regate target phase has `gate: []` | Target treated as passed (`all_passed(&[])` = true) |
| 27 | `regate_self_reference_detected` | phase has `regate: [self_id]` (self-referencing) | Lint catches it (or runtime error if lint bypassed) |
| 28 | `regate_on_completed_pipeline_returns_error` | Pipeline already COMPLETED → call regate | `error: "pipeline_completed"` or `verify_not_passed` |

### Test Fixtures

**Existing (updated):**
- `regate_pipeline.yml` — 3 phases: design → build(regate:[design]) → test

**New:**
- `regate_max_retries_pipeline.yml` — phase with regate + max_retries for escalation test
- `regate_gateless_pipeline.yml` — gateless phase with regate targets
- `regate_multi_target_pipeline.yml` — phase with 2 regate targets (for partial fail test)
- `regate_when_skip_pipeline.yml` — regate target with when: condition (for skip test)

### Test Implementation Notes

- Engine tests use `TempDir` + inline YAML via `write_yaml()` helper
- CLI tests use `assert_cmd::Command::cargo_bin("belt-agent")`
- Gate files (e.g., `design.ok`) are created/removed via `std::fs::write` / `std::fs::remove_file` to control gate pass/fail
- Regate target output_dir is computed as `{belt_dir}/runs/{run_id}/{target_id.replace('/', '_')}`
- Skipped target detection: check `state.skipped_phases.contains(target_id)` in cmd_regate

## Not in Scope

| Item | Reason |
|------|--------|
| `on_escalation` (pause/skip/abort) | Deferred to separate design discussion |
| `status` enrichment | Separate ticket; DRY context provider for multi-context orchestration |
| verify JSON output includes regate | Separate small fix ticket (B items) |
| `now_iso8601()` rename | Separate small fix ticket |
| `regex` crate removal | Separate small fix ticket |
| SKILL.md protocol updates | Skill layer, not CLI spec |
| Gate execution timeout | Existing limitation (also affects verify). Not regate-specific. |
| Sub-pipeline expanded ID as regate target | Lint validates pre-expansion IDs. `uses:` expanded targets (e.g., `review/analysis`) as regate targets is a future concern. |

## Known Limitations

- **Self-referencing regate** (`regate: [self_id]`): Current lint only checks target existence, not self-reference. Should be caught by lint in a future improvement. Runtime behavior: the phase's own gates would be re-executed as regate, which is semantically redundant (verify already ran them).
- **Sub-pipeline regate targets**: If a `uses:` phase is specified as a regate target, the expanded phase IDs (`{parent}/{sub}`) won't match the original ID. Lint validates pre-expansion IDs. This is acceptable for MVP; sub-pipeline regate targets are not a current use case.
