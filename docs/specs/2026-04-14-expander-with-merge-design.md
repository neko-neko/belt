# Expander `Invoker::Pipeline.with` merge into sub-phase args

> Linear: follow-up to [BELT-32](https://linear.app/neko-neko/issue/BELT-32)
> Date: 2026-04-14
> Supersedes: 2026-04-13 decision "C-β not adopted" (retained `belt-core state machine single-responsibility` rationale is revised — see Context)

## Context

BELT-32 follow-up #7 (recorded in 2026-04-14 handover Known Issues, severity **medium**): the expander in `crates/belt-core/src/expander.rs:28` destructures `Invoker::Pipeline { pipeline, .. }` and therefore drops the `with:` map declared by the parent phase. The schema (added in Plan A / Plan B) promises that `with` re-binds sub-pipeline argument references to parent expressions, but the runtime is inert — sub-pipelines are flattened into the parent run and any `args.X` reference in a sub-phase resolves against the parent's `RunState.args` directly.

Today this is non-fatal because the orchestrator pipelines (`examples/skills/feature-dev/pipeline.yml`, `examples/skills/debug-flow/pipeline.yml`) and their sub-pipelines (spec-review / implementation-review / code-review / test-review) all use identical argument names (`iterations` / `codex` / `ui` / `swarm`). A rename (e.g. parent exposes `--iterations` but a sub-pipeline internally names it `count`) would silently fail: the sub-phase would dereference `args.count`, which does not exist in the parent run.

### Decision reversal

The 2026-04-13 spec (`docs/specs/2026-04-13-belt32-followup-design.md`) adopted **C-α** (pipeline.yml carries `with:`, resolution is the orchestrator's job) and explicitly rejected **C-β** (belt-core resolves `with`), citing "belt-core state machine single-responsibility" and "reversibility." This spec **revises that decision**: the continued drift between schema and runtime is a larger long-term cost than the single-responsibility concern, and the chosen implementation strategy (static AST-level rewrite at expand time, not runtime template evaluation) preserves the existing separation — `belt-core` still does not resolve `args.X` to runtime values; it only rewrites which `args.X` name a sub-phase refers to.

### Prerequisites

- `Invoker::Pipeline { pipeline, with: HashMap<String, serde_json::Value> }` exists in `crates/belt-core/src/model.rs:260-264`.
- Sub-pipelines are flattened into the parent's `RunState.args` at init time (`crates/belt-core/src/engine.rs`). No nested run is created.
- `IterationsSpec::Template(String)` precedent (`model.rs:217-222`): template values of the form `"args.X"` are carried through belt-core untouched and resolved by the skill orchestrator at dispatch.
- No existing orchestrator pipeline renames an argument across the parent↔sub boundary, so this change is a no-op for current examples. Behaviour for identity renames (`with.iterations: "args.iterations"` against a sub-phase that references `args.iterations`) is unchanged.

## Design

### Architecture

All changes are confined to **`crates/belt-core/src/expander.rs`** plus new unit tests in the same file. No changes to `model.rs`, `engine.rs`, `view.rs`, `lint.rs`, `parser.rs`, `gate.rs`, or any binary crate. No YAML changes in `examples/`. The `belt-agent` JSON contract is unchanged (the rewrite happens before sub-phases reach `build_status_view`).

The expander gains one responsibility: when it flattens a sub-pipeline into the parent run, it rewrites the sub-phases' `args.X` references so they point at the names that actually exist in the parent's `RunState.args`.

### Substitution model

Given a parent phase with `invoke: Pipeline { pipeline: P, with: W }` where `W: HashMap<String, serde_json::Value>`:

- For each sub-phase produced by expanding `P`, walk the sub-phase's fields and replace occurrences of the string literal `"args.<name>"` (full-string match, not substring) whenever `<name>` is a key in `W`.
- If `W["<name>"]` is `serde_json::Value::String(s)` where `s` itself matches the shape `"args.<other>"`, the occurrence is rewritten to `"args.<other>"` (rename chain).
- If `W["<name>"]` is any other JSON value (`Bool`, `Number`, non-template `String`, `Null`, `Array`, `Object`), the occurrence is rewritten to that literal.
- If a sub-phase field being rewritten is typed more narrowly than a generic string (notably `IterationsSpec::Template`), a best-effort type conversion is attempted (see "Typed field conversions" below). Conversion failure leaves the original template in place — the rewrite is lossy-safe.
- `W` keys that no sub-phase references are silently ignored. (Lint-time detection of dead keys is out of scope — tracked as a follow-up ticket.)

### Target fields inside each sub-phase

| # | Field | Type | Substitution behavior |
|---|---|---|---|
| 1 | `Invoker::Agents { iterations, .. }` | `IterationsSpec::Template(String)` | If template equals `"args.<name>"` and `W["<name>"]` is a number → `IterationsSpec::Literal(u32)`; if `W["<name>"]` is a template string → updated `IterationsSpec::Template`. |
| 2 | `Invoker::{Skill, Agent, Agents} { args, .. }` values | `HashMap<String, serde_json::Value>` | Each value that is `String("args.<name>")` is rewritten to `W["<name>"]`. |
| 3 | `Invoker::Pipeline { with, .. }` values (nested) | `HashMap<String, serde_json::Value>` | Same rule as #2 — applied recursively so that `parent.with → middle.with → leaf.phase` chains stay consistent. |
| 4 | `Phase.when` | `Option<String>` | If the string equals `"args.<name>"` exactly, rewrite to the substituted string form (or literal booleans coerced to string — see below). Non-exact matches (e.g. `"args.foo && args.bar"`) are left alone. |
| 5 | `Phase.config` values | `HashMap<String, serde_json::Value>` | Same rule as #2. |

Fields explicitly **not** rewritten: `Phase.id`, `Phase.description`, `Phase.produces[].{name,path,description}`, `Phase.consumes[]`, `Phase.gate[]`, `Phase.regate[]`, `Phase.validate[]`, `Phase.max_retries`, `Phase.confirm`. Arguments in glob patterns or shell commands are not rewritten (#Q1 decision).

### Typed field conversions

- **`IterationsSpec::Template("args.N") ← W["N"] = Number(v)`**: convert via `v.as_u64()`. If the value is a non-integer or does not fit in `u32`, leave the template unchanged (defensive). If `v.as_u64()` succeeds, produce `IterationsSpec::Literal(v as u32)` (saturating on overflow — but `u32::MAX` iterations is a pathological config and not a realistic case).
- **`Phase.when = "args.X" ← W["X"] = Bool(b)`**: rewrite as the literal string `"true"` / `"false"`. The orchestrator's existing `when` evaluator treats bare `"true"` / `"false"` as literals (see `engine.rs` eval_when). This keeps the rewrite round-trippable without introducing a new `Phase.when` type.
- All other numeric / bool / null overrides flowing into `HashMap<String, serde_json::Value>` preserve their JSON type verbatim.

### Recursion rule for nested sub-pipelines

When the sub-phase itself is an `Invoker::Pipeline { with: inner_w, .. }`, its `inner_w` values are rewritten **before** the expander recurses to expand that nested pipeline (which is not the current architecture — sub-pipelines are flattened via `expander_sub_pipeline`, not re-expanded). Concretely: the current `expand_pipeline` already handles `Invoker::Pipeline` only at the top level; nested references inside a sub-pipeline are stored but not re-expanded. Therefore the recursive-rewrite guarantees apply to the **values stored in the flattened sub-phase's** `invoke.Pipeline.with`, not to a second expansion pass. If nested expansion is added later, the rewrite remains correct because each layer normalizes its own `with` before flattening.

### Example (rename semantics, synthetic)

Parent `feature-dev.yml`:
```yaml
args:
  iterations: { type: number, default: 3 }
phases:
  - id: review
    invoke:
      pipeline: ../custom-review/pipeline.yml
      with:
        count: "args.iterations"   # sub-pipeline names it `count`, parent names it `iterations`
```

Sub `custom-review/pipeline.yml`:
```yaml
args:
  count: { type: number, default: 1 }
phases:
  - id: vote
    invoke:
      agents: [v1, v2]
      iterations: "args.count"
```

Before this change: the expanded phase carries `iterations: Template("args.count")`. At runtime the orchestrator looks up `args.count` in `RunState.args` — not present — and the substitution fails silently (or produces whatever the orchestrator's fallback is).

After this change: expander rewrites `iterations: Template("args.count")` → `iterations: Template("args.iterations")` (because `W["count"] = String("args.iterations")`). The orchestrator resolves `args.iterations` against the parent run and the rename works.

### Error & edge cases

- **Empty `with`**: fast-path, no rewrite.
- **`with` key references a sub-phase arg that no field uses**: ignored (no-op). A future lint rule can warn; not in this task.
- **`with` value is `null`**: rewrite target value to `serde_json::Value::Null`. For `IterationsSpec::Template` fields, leave the template unchanged (null is not a valid iteration count).
- **`with` value is a non-template `String("foo")`**: rewrite to `String("foo")`. For `IterationsSpec::Template` fields, leave the template unchanged (raw strings are not iteration counts).
- **Circular rename (`with.a: "args.b"` and `with.b: "args.a"`)**: not a concern — rewrite is single-pass (one parent→sub step), not iterative. Each sub-phase reference is rewritten at most once per expansion. Nested pipelines get a fresh rewrite pass at their own layer.

## Tests

Added to `crates/belt-core/src/expander.rs` inline `#[cfg(test)] mod tests` (new module) or to `crates/belt-core/tests/parser_test.rs` if integration-style fixtures are preferred. Minimum cases:

1. **identity**: `with.iterations: "args.iterations"`, sub-phase uses `args.iterations` → rewritten identifier is still `args.iterations`. Current `examples/` all take this path; must remain green.
2. **rename**: `with.count: "args.iterations"`, sub-phase uses `args.count` → rewritten to `args.iterations`.
3. **literal number → IterationsSpec::Literal**: `with.count: 5`, sub-phase `iterations: Template("args.count")` → `Literal(5)`.
4. **literal bool into when**: `with.enabled: true`, sub-phase `when: "args.enabled"` → `when: "true"`.
5. **nested Invoker::Pipeline.with**: sub-phase is itself `Invoker::Pipeline { with: { inner: "args.count" } }`, parent `with.count: "args.iterations"` → sub-phase's inner `with` becomes `{ inner: "args.iterations" }`.
6. **no-op (empty with)**: empty `with: {}` → sub-phases unchanged (byte-identical).
7. **type-conversion failure**: `with.count: "not-a-number"` → sub-phase `iterations: Template("args.count")` remains `Template("not-a-number")` (defensive fallback, does not panic).

No new integration test fixture is strictly required; the existing `belt32_full_pipeline_with_all_new_types` in `crates/belt-core/tests/parser_test.rs` already uses identity-rename semantics and must continue to pass unchanged. A new narrow integration test exercising a rename (synthetic fixture) is added to lock the new behavior end-to-end.

## Verification

1. `cargo test -p belt-core` — all existing + new unit tests pass.
2. `cargo test -p belt-agent` — `feature_dev_migrated_pipeline_boots` and all status/next integration tests pass (identity rename path).
3. `cargo clippy --package belt-core --package belt-agent -- -D warnings`.
4. `cargo fmt --package belt-core --package belt-agent -- --check`.
5. `cargo run --bin belt -- lint examples/skills/*/pipeline.yml` — all lint clean (no lint rule changes in this task).
6. `cargo run --bin belt-agent -- init examples/skills/feature-dev/pipeline.yml --arg iterations=3` in a scratch dir — subsequent `status` reflects the expected sub-phases with `args.iterations` references (identity case, current behavior preserved).
7. **Adversarial probe**: hand-craft a minimal rename fixture (parent `with.count: "args.iterations"`, sub uses `args.count`) as a one-off pipeline pair; expand it via `belt-agent init` and assert the resulting status shows the rewritten reference. This is outside `examples/` (not committed) — documented as a smoke step.

## Non-goals

- Runtime template-to-value resolution inside belt-core. Remains the orchestrator's job.
- Lint rule for `with` keys that do not match any sub-pipeline `inputs` declaration (follow-up ticket).
- String-interpolation substitution (e.g. `"${args.port:-3000}"` inside shell commands). Full-string `"args.X"` only.
- Re-opening nested-expansion architecture (Plan B's flatten-into-parent design is retained).
- Changes to `belt-agent` JSON shape, CLI surface, or lint output format.

## Rollout

- Single commit scope: `expander.rs` + new tests.
- No migration needed — existing `examples/` use identity renames and remain byte-identical in their expanded form.
- Revert strategy: the change is additive in logic (substitution is gated by `!with.is_empty()`); reverting the commit restores prior behavior with zero side-effects.

## References

- Existing code: `crates/belt-core/src/expander.rs`, `crates/belt-core/src/model.rs` (Invoker, IterationsSpec, Phase)
- Predecessor decision: `docs/specs/2026-04-13-belt32-followup-design.md` §"Task C — C-α"
- Precedent pattern: `IterationsSpec::Template` (Plan A integration test `belt32_full_pipeline_with_all_new_types`)
- Handover directive: `.agents/handover/main/20260414-101808/project-state.json` session_notes §directive
