# BELT-21: Engine verify-before-step / max_retries Guards

**Linear**: [BELT-21](https://linear.app/neko-neko/issue/BELT-21)
**Status**: Design approved
**Date**: 2026-04-06

## Background

belt-agent CLI currently allows unrestricted `step` calls — an agent can advance phases without running `verify`, after a failed verification, or after exceeding the retry limit. These constraints are enforceable deterministically at the Engine level and should not rely on LLM self-discipline.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Verify state granularity | `HashMap<String, bool>` per phase | Future extensibility for regate re-verification |
| Gate-less phase handling | Auto-set `true` at init/step | Unified guard logic in `step()`, no branching |
| max_retries enforcement point | `step()` only | verify is side-effect-free; restricting it has no safety benefit |
| max_retries default (0) | Unlimited | Backward compatible; constraints are opt-in |
| Error representation | Dedicated `BeltError` variants | Structured matching in belt-agent; no string parsing |

## RunState Extension (`model.rs`)

Add `phase_verify_passed: HashMap<String, bool>` to `RunState`:

```rust
pub struct RunState {
    // ... existing fields ...
    #[serde(default)]
    pub phase_verify_passed: HashMap<String, bool>,
}
```

- `verify_verdict()` sets `phase_verify_passed[current_phase] = passed`
- `init()` / `step()` auto-set `true` for gate-empty phases

## Error Types (`error.rs`)

```rust
#[error("verify required for phase '{phase_id}': run verify before step")]
#[diagnostic(code(belt::verify_required))]
VerifyRequired { phase_id: String },

#[error("max retries exceeded for phase '{phase_id}': {attempts}/{max_retries}")]
#[diagnostic(code(belt::max_retries_exceeded))]
MaxRetriesExceeded { phase_id: String, attempts: u32, max_retries: u32 },
```

## Engine Logic Changes

### `init()`

After creating the initial `RunState`, if the first active phase has an empty `gate` vec, set `phase_verify_passed[phase_id] = true`.

### `verify_verdict()`

In addition to incrementing `phase_attempts`, update:

```
state.phase_verify_passed[current_phase] = passed
```

### `step()` Guard Evaluation Order

1. **verify-before-step**: `phase_verify_passed.get(current_phase) != Some(true)` -> `BeltError::VerifyRequired`
2. **max_retries**: `max_retries > 0 && phase_attempts[current_phase] > max_retries` -> `BeltError::MaxRetriesExceeded`
3. **confirm/validate check** (existing logic)
4. **Phase transition** + auto-set `true` for next gate-empty phase

verify-before-step is checked first because without a verify call, the attempt count is meaningless for max_retries evaluation.

## belt-agent JSON Output (`main.rs`)

`cmd_step` matches `BeltError` variants and outputs structured JSON:

**VerifyRequired:**
```json
{
  "advanced": false,
  "reason": "verify_required",
  "phase": "build"
}
```

**MaxRetriesExceeded:**
```json
{
  "advanced": false,
  "reason": "max_retries_exceeded",
  "phase": "build",
  "attempts": 4,
  "max_retries": 3,
  "escalation": true
}
```

## Test Fixtures

```
crates/belt-core/tests/fixtures/
  gate_pipeline.yml          -- 3-phase pipeline with gates
  gate_confirm_pipeline.yml  -- gate + confirm mixed phases
  max_retries_pipeline.yml   -- phases with max_retries set
  when_gate_pipeline.yml     -- when: conditional + gate
  regate_pipeline.yml        -- regate declaration + gate
```

## Test Cases (23 total)

### verify-before-step Guard (unit, inline YAML)

| # | Test Case | Expected |
|---|-----------|----------|
| 1 | gate phase, step without verify | `VerifyRequired` |
| 2 | gate phase, verify PASS then step | Success |
| 3 | gate phase, verify FAIL then step | `VerifyRequired` |
| 4 | gate-less phase, step without verify | Success (auto true) |
| 5 | verify PASS -> step -> next gate phase -> step without verify | `VerifyRequired` |

### max_retries Enforcement (unit, inline YAML)

| # | Test Case | Expected |
|---|-----------|----------|
| 6 | max_retries: 3, 3 attempts, verify PASS, step | Success |
| 7 | max_retries: 3, 4 attempts (all FAIL), step | `MaxRetriesExceeded` |
| 8 | max_retries: 0 (unlimited), many FAILs, verify PASS, step | Success |
| 9 | verify after max_retries exceeded | verify succeeds (attempt incremented) |

### Guard Evaluation Order (unit)

| # | Test Case | Expected |
|---|-----------|----------|
| 10 | verify not run + max_retries exceeded, step | `VerifyRequired` (not `MaxRetriesExceeded`) |

### Gate-less Phase Auto-true (unit)

| # | Test Case | Expected |
|---|-----------|----------|
| 11 | init, first phase gate-less | `phase_verify_passed[id] == true` |
| 12 | step to gate-less next phase | auto true set on transition target |

### RunState Persistence (unit)

| # | Test Case | Expected |
|---|-----------|----------|
| 13 | verify -> save -> load -> check phase_verify_passed | round-trip preserved |

### belt-agent JSON Output (unit)

| # | Test Case | Expected |
|---|-----------|----------|
| 14 | step without verify via belt-agent | `reason: "verify_required"` JSON |
| 15 | step after max_retries exceeded via belt-agent | `reason: "max_retries_exceeded", escalation: true` JSON |

### Full Lifecycle (fixture YAML)

| # | Fixture | Scenario |
|---|---------|----------|
| 16 | `gate_pipeline.yml` | init -> verify PASS -> step -> ... -> COMPLETED |
| 17 | `gate_confirm_pipeline.yml` | gate phase -> confirm-only phase (verify skip) -> gate phase |
| 18 | `max_retries_pipeline.yml` | verify FAIL x N -> PASS -> step recovery |

### Compound Conditions (fixture YAML)

| # | Fixture | Scenario |
|---|---------|----------|
| 19 | `when_gate_pipeline.yml` | when: false skipped phase does not pollute verify state |
| 20 | `regate_pipeline.yml` | regate-declared phase verify/step works correctly |

### Edge Cases / Boundary Values (fixture YAML)

| # | Fixture | Scenario |
|---|---------|----------|
| 21 | `max_retries_pipeline.yml` | max_retries: 1, 1 FAIL -> immediate escalation |
| 22 | `gate_pipeline.yml` | consecutive step without verify -> VerifyRequired every time |
| 23 | `max_retries_pipeline.yml` | after escalation, verify still works but step stays rejected |

## Affected Files

| File | Change |
|------|--------|
| `crates/belt-core/src/model.rs` | `RunState` + `phase_verify_passed` field |
| `crates/belt-core/src/error.rs` | `VerifyRequired`, `MaxRetriesExceeded` variants |
| `crates/belt-core/src/engine.rs` | Guard logic in `init`/`step`/`verify_verdict` |
| `crates/belt-agent/src/main.rs` | `cmd_step` error matching -> escalation JSON |
| `crates/belt-core/tests/engine_test.rs` | 23 new test cases |
| `crates/belt-core/tests/fixtures/*.yml` | 5 fixture files |

## NOT in Scope

- validate verification obligation (skill-layer responsibility)
- regate runtime re-verification (future extension)
- Sub-pipeline expansion compound tests
- Handover / session management
