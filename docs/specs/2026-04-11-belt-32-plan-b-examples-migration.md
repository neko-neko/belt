# BELT-32 Plan B: Examples Migration and Legacy Cutover

**Linear**: [BELT-32](https://linear.app/neko-neko/issue/BELT-32) (parent: [BELT-20](https://linear.app/neko-neko/issue/BELT-20))
**Parent spec**: `docs/specs/2026-04-11-belt-action-data-first-class.md`
**Plan A**: `docs/plans/2026-04-11-belt-action-data-first-class-plan.md` (completed; commits `6692149..4608fc4`)
**Status**: Draft
**Date**: 2026-04-11

## Summary

Apply the Plan A additive types (`Invoker`, `Artifact`, `ArtifactRef`, `ValidationSource`) across all example skills, delete the `audit-gate` sub-pipeline scaffolding, and cut over legacy fields (`Phase.artifacts: Vec<String>`, phase-level `Phase.uses: Option<String>`) in a single immediate removal. Plan B completes the BELT-32 philosophical intent: `belt-core` models Action and Data flow as first-class typed primitives, and `examples/skills/` exercise them end-to-end.

Plan A was strictly additive; `belt` currently tolerates both old and new shapes. Plan B is the cutover.

## Background

Plan A (commits `6692149..4608fc4`) added the new types to `belt-core` without touching `examples/skills/`. As of its completion, eight legacy examples (`audit-gate`, `feature-dev`, `debug-flow`, `smoke-test`, `spec-review`, `code-review`, `test-review`, `implementation-review`) still use the pre-Plan-A shapes. The five friction symptoms identified in the parent spec — static verification absent, path resolution implicit, multiple coexisting dispatch patterns, audit-phase scaffolding, fragmented phase intent — persist until the examples migrate.

Plan A locked the schema. Plan B pays the migration cost, deletes the scaffolding, and removes the legacy fallback.

## Scope

### In scope

- Migrate all seven example skills to the new format:
  - `smoke-test`, `spec-review`, `code-review`, `test-review`, `implementation-review` (leaf skills, no sub-pipeline dependencies)
  - `feature-dev`, `debug-flow` (orchestrator skills, depend on the leaf skills through `invoke: { pipeline: ... }`)
- Delete `examples/skills/audit-gate/` entirely (pipeline, done-criteria, references).
- Introduce a hybrid criteria directory: `examples/criteria/` for shared canonical done-criteria, `{skill}/criteria/` for skill-specific done-criteria. Place the shared `audit-protocol.md` at `examples/references/audit-protocol.md`.
- Implement `belt-core` phase-start mtime filter for glob resolution of `Artifact.path`.
- Implement `belt-core` `validate:` scalar shorthand parser (custom `deserialize_with`).
- Remove legacy fields from `belt-core::model::Phase`: `artifacts: Vec<String>` and phase-level `uses: Option<String>`. Update parser, expander, view, and lint to match.
- Update `skills/belt-agent/SKILL.md` (protocol): add "Reading phase.invoke" (four variants), "Artifact graph in status", "Validate file semantics" sections; remove the `config.skill` entry from "Well-known Config Keys".
- Document `max_retries` semantics for the collapsed work-and-validate phase model; confirm no counter-mechanism change is required.
- E2E smoke test: `belt-agent init → next → verify → step → next → ...` loop must traverse each migrated example cleanly.

### Out of scope (explicitly deferred)

- **BELT-28** (`on_escalation: skip/abort/pause`) — independent issue; Plan B documents `max_retries` semantics only.
- **Agent / Agents unification** — locked in Plan A; not reopened.
- **`Skill.reference` promotion** — smoke-test is the only user; pain not surfaced. Stays inside `Invoker::Skill.args`.
- **`confirm:` / `when:` / `regate:` type expansion** — forward-compat schema only per parent spec DD-6; no behavior change in Plan B.
- **Remote `uses:` (YAML Universe)** — separate future work.
- **New CLI commands** — `belt` and `belt-agent` binary interfaces are unchanged.
- **Deprecation warnings for legacy fields** — `belt` has no external consumers; the cutover is immediate.

### Pre-conditions locked by Plan A

- `Invoker` untagged enum variant ordering: `Skill → Agent → Agents → Pipeline`.
- `ArtifactRef` variant ordering: `Named → Qualified`.
- `ValidationSource` variant ordering: `Inline → File`.
- `Phase.invoke`, `Phase.produces`, `Phase.consumes` already exist in `belt-core`.
- `Phase.validate: Vec<ValidationSource>` already in place (list form only).
- 199 `belt-core` tests and 39 `belt-agent` tests pass on the Plan A baseline.

## Design decisions

### DD-1: Glob resolution via phase-start mtime filter

When `Artifact.path` contains a glob, `belt-core` resolves it automatically by recording the phase entry timestamp and filtering glob matches whose mtime is newer than that timestamp. The LLM is not asked to report the resolved path.

**Runtime state change.** Add `RunState.phase_start_times: HashMap<PhaseId, DateTime<Utc>>`. Written on phase entry inside `engine::step()`. Plan B is the first consumer.

**Resolution algorithm** (invoked from `view::build_status_view()` for each produces entry after the phase has started):

1. If the `Artifact.path` contains no glob metacharacters, `std::fs::metadata(path).is_ok()` is the existence answer — no filter needed.
2. Otherwise, list candidates via `glob::glob(path)`.
3. Retain candidates whose `metadata.modified()` is greater than or equal to `phase_start_times[phase_id]`.
4. If multiple remain, pick the newest mtime; break ties by ascending filename.
5. If none remain, report `missing` (existence false) in the status JSON.

**Retry and regate interaction.**

- `phase_start_times[phase_id]` is set exactly once per run, when `step` first transitions to the phase. Because a belt pipeline is a DAG (phases advance forward only; see `engine::step`'s handling of `completed_phases` and `current_phase`), the same phase is never re-entered within a run.
- Retries inside the phase (verify FAIL → fix → verify PASS) keep the original timestamp. Files produced during retries remain visible because they are all mtime-newer than the single phase-entry mark.
- `regate` is an in-place re-verification of earlier phases' gates and does not move the run back to those phases. It therefore does not affect `phase_start_times` for any phase. Earlier phases' `phase_start_times` also remain unchanged because the run never returns to them.

**Lint implications.** Glob syntax is validated at lint time; mtime filtering is a runtime concern and is not simulated by lint. No new lint rule beyond what Plan A already has for validate file existence.

**File-system quirks.** macOS HFS has one-second mtime granularity; on CI, tests that create files within the same phase should not rely on sub-second resolution. Integration tests insert a small `thread::sleep` if ordering matters.

### DD-2: `validate:` scalar shorthand parser

Plan A accepts `validate: [<items>]` where each item is either a string (`ValidationSource::Inline`) or `{ file: "..." }` (`ValidationSource::File`). Plan B adds a scalar shorthand for the most common case — a single criteria file — without touching the list form.

**Parser behavior table:**

| YAML | Rust value |
|------|------------|
| `validate: ./criteria/design.md` | `vec![File { file: "./criteria/design.md" }]` |
| `validate: /abs/path/criteria.md` | `vec![File { file: "/abs/path/criteria.md" }]` |
| `validate: "All checks pass"` | `vec![Inline("All checks pass")]` |
| `validate: ["a", "b"]` | `vec![Inline("a"), Inline("b")]` |
| `validate: [{ file: "./x.md" }]` | `vec![File { file: "./x.md" }]` |
| `validate: ["./x.md"]` | `vec![Inline("./x.md")]` — heuristic does **not** apply inside lists |

**Scalar classification rule.** A top-level scalar that starts with `./` or `/` is classified as `File`; otherwise it is `Inline`. The heuristic applies only to the scalar shorthand, not to list items. This keeps list semantics unambiguous and avoids having to reject `./foo` as an inline criterion.

**Implementation.** Add `#[serde(deserialize_with = "deserialize_validate")]` to `Phase.validate`. The deserializer uses a `serde::de::Visitor` that handles both `visit_str` / `visit_string` (scalar path) and `visit_seq` (list path, which delegates to the existing untagged enum handling). Approximately 50 lines of Rust including the visitor trait implementation and classification helper.

**Migration impact.** Every migrated audit phase collapses into a work phase with `validate: ./criteria/{name}.md`. Without the scalar shorthand, each such phase requires a two-line `validate:` block instead of a single-line directive.

### DD-3: `max_retries` semantics — documentation of existing behavior, no new mechanism

With Plan A's work-and-audit collapse, a single phase now includes both the invocation (`invoke:`) and the criterion judgment (`validate:`). Plan B **does not change** the `max_retries` implementation — it documents how the existing counter behaves in the collapsed model and makes the author-facing mental model explicit in `SKILL.md`.

**Existing baseline** (preserved; see `crates/belt-core/src/engine.rs::verify_verdict` and the `max_retries` guard in `step`):

- Every call to `verify_verdict` increments `phase_attempts[phase_id]`, regardless of whether the verdict was PASS or FAIL. The counter is scoped per phase.
- The `step` guard rejects advancement when `phase_attempts[phase_id] > max_retries`. This means `max_retries: 3` permits up to four verify invocations before the phase refuses to advance.
- `regate` is an in-place re-verification of earlier phases' gates performed as part of the current phase's protocol. It does **not** physically rewind to earlier phases, and therefore does not touch earlier phases' `phase_attempts` counters. This is the intended semantics from BELT-24 (memory: `project_belt24_regate_execution.md`).

**How the counter behaves under the collapsed model:**

- **Pure gate-side retry**: `verify` FAIL → fix → `verify` PASS → `step --confirm`. Two verify invocations, `phase_attempts[phase] = 2`. Works for any `max_retries >= 1`.
- **Validate-side retry**: `verify` PASS → LLM judges validate criteria not met → fix work → `verify` PASS again → `step --confirm`. Two verify invocations, same counter arithmetic. The LLM is expected to re-run `verify` after fixing the work so `belt-core` can re-observe the gate state; this is the same pattern as BELT-30's per-check persistence expectations.
- **Regate-triggered re-verify**: `verify` PASS → `regate` FAIL → LLM fixes earlier phase → `verify` PASS → `regate` PASS → `step --confirm`. Two verify invocations at the current phase. Earlier phases' counters are untouched.

In every case, the per-phase `max_retries` acts as a budget on how many verify cycles belt-core tolerates before giving up. The collapsed model does not require a new counter or a new reset rule.

**Escalation.** When `phase_attempts[phase_id] > max_retries`, `step` fails with `MaxRetriesExceeded`. Until BELT-28 introduces `on_escalation: skip/abort/pause`, this is a hard error that the orchestrator reports without further recovery options.

**Documentation deliverables:**

- `skills/belt-agent/SKILL.md` "Decision Rules" table gains a row describing "Every `verify` invocation counts toward `max_retries`, regardless of verdict; regate is an orthogonal in-place check and does not touch the counter; earlier phases' counters are never modified by regate."
- `crates/belt-core/src/engine.rs::verify_verdict` doc comment makes the per-phase scope explicit.

**Rationale.** A per-phase counter that increments on every verify is the simplest model and is already implemented. The collapsed phase model does not introduce new failure modes that belt-core needs to observe — validate judgment remains at the orchestrator layer, and the orchestrator signals fix-and-retry cycles by calling `verify` again. Adding a "regate reset" or "validate-only counter" would be a new mechanism solving an unobserved problem and is out of scope.

### DD-4: Legacy field removal — immediate cutover

Plan B removes `Phase.artifacts: Vec<String>` and phase-level `Phase.uses: Option<String>` from the `belt-core` model in a single sub-task, with no deprecation period. The cutover happens after all examples have been migrated.

**Ordering.**

1. Sub-tasks 1 through 10 migrate examples while the legacy fields are still accepted by the parser. Both old and new shapes parse.
2. Sub-task 11 deletes the legacy fields and the `audit-gate` skill directory in the same commit. From that commit forward, any remaining legacy usage fails to parse.
3. Sub-task 12 updates `skills/belt-agent/SKILL.md` and runs the E2E smoke test.

**Why immediate.** `belt` has no external consumers; a deprecation window would serve only a hypothetical future user. The Plan A commit set remains the rollback point — reverting past `4608fc4` restores legacy support in full. Git history is the migration guide.

**Pre-check for sub-task 11.** Before removing fields, run an automated grep to confirm no migrated pipeline still contains the legacy shapes:

```
grep -rE "^[[:space:]]+artifacts:" examples/skills/**/pipeline.yml        # must return 0 lines
grep -rE "^[[:space:]]+uses:" examples/skills/**/pipeline.yml             # must return 0 lines for phase-level uses
grep -rE "config:[[:space:]]*$" -A2 examples/skills/**/pipeline.yml | grep -E "(skill|criteria|agents):"  # must return 0 matches
```

Gate-level `uses:` inside `GateCheck::Uses { uses, with }` is preserved and is a separate concern; the grep above checks specifically for phase-level `uses:` at the same indentation as `id:`.

### DD-5: Hybrid criteria directory structure

Shared canonical done-criteria live at `examples/criteria/`. Skill-specific done-criteria live at `examples/skills/{skill}/criteria/`. The shared audit protocol document lives at `examples/references/audit-protocol.md`. The `audit-gate` directory is removed after migration.

**Target layout:**

```
examples/
  criteria/                          # shared canonical (promoted from audit-gate/done-criteria/)
    execute.md
    code-review.md
    smoke-test.md
    test-review.md
    _schema.md
  references/                        # shared protocol docs
    audit-protocol.md
  skills/
    feature-dev/
      pipeline.yml                   # validate: ../../criteria/execute.md
      SKILL.md
      criteria/                      # feature-dev specific
        design.md
        plan.md
        plan-review.md
        spec-review.md
        doc-audit.md
      references/                    # feature-dev specific (unchanged)
        evidence-plan-protocol.md
        fix-dispatch-strategy.md
    debug-flow/
      pipeline.yml
      SKILL.md
      criteria/                      # debug-flow specific
        rca.md
        fix-plan.md
        fix-plan-review.md
      references/
        evidence-plan-protocol.md    # debug-flow's variant (may differ from feature-dev's)
        fix-dispatch-strategy.md
    smoke-test/
      pipeline.yml
      SKILL.md
      references/                    # smoke-test specific (unchanged)
        env-setup-procedure.md
        adhoc-test-procedure.md
    spec-review/
      pipeline.yml
      SKILL.md
    code-review/
      pipeline.yml
      SKILL.md
    test-review/
      pipeline.yml
      SKILL.md
    implementation-review/
      pipeline.yml
      SKILL.md
    # audit-gate/ — DELETED by sub-task 11
```

**Drift handling.** The `audit-gate/done-criteria/*.md` set claims to be canonical; `feature-dev/references/done-criteria/*.md` is a concrete copy. These already differ (verified by `diff -q` on `execute.md`). Sub-task 1 performs a three-way merge:

1. If the difference is semantically meaningful (a feature-dev-specific requirement missing from the canonical version), preserve the canonical version at `examples/criteria/` and put the delta at `{skill}/criteria/{name}-extra.md`. Reference both files from the phase's `validate:` if necessary via the list form.
2. If the difference is copy-and-edit drift, preserve the canonical version verbatim.

In practice, option 2 is expected to cover most cases; option 1 is a rare escape hatch.

**Path references.** Relative paths in `pipeline.yml` resolve against the pipeline file's parent directory, matching the existing BELT-22 `uses:` resolution rule. A feature-dev phase references the shared `execute.md` as `../../criteria/execute.md` and the skill-specific `design.md` as `./criteria/design.md`. Plan A's validate-file-existence lint rule covers both.

### DD-6: Sub-task plan structure (twelve tasks)

Plan B runs as twelve sub-tasks, each executed through the subagent-driven-development review cycle (implementer → spec-reviewer → code-quality-reviewer), matching Plan A's structure.

| # | Sub-task | Depends on |
|---|---|---|
| 1 | Create `examples/criteria/` and `examples/references/audit-protocol.md`. Three-way merge done-criteria from `audit-gate/done-criteria/` and `feature-dev/references/done-criteria/`. Populate `_schema.md`. | — |
| 2 | Implement phase-start mtime filter in `belt-core`: add `RunState.phase_start_times`, plumb through `engine::step()`, extend `view::build_status_view()` to resolve globs. | — |
| 3 | Implement `validate:` scalar shorthand deserializer in `belt-core::model` (custom `deserialize_with`). | — |
| 4 | Migrate `examples/skills/smoke-test/pipeline.yml` to the new format. `config.skill` / `config.reference` become `invoke.skill.args.reference`. Declare `produces:` for `smoke-test-report.md`. | 1, 3 |
| 5 | Migrate `examples/skills/spec-review/pipeline.yml`. `config.agents` becomes `invoke.agents`. Declare `produces:` for the review findings. | 1, 3 |
| 6 | Migrate `examples/skills/code-review/pipeline.yml` (same shape as sub-task 5). | 1, 3 |
| 7 | Migrate `examples/skills/test-review/pipeline.yml`. | 1, 3 |
| 8 | Migrate `examples/skills/implementation-review/pipeline.yml`. | 1, 3 |
| 9 | Migrate `examples/skills/feature-dev/pipeline.yml` and `SKILL.md`. Collapse every `{phase}-audit` pair into a single phase with `validate: ../../criteria/{phase}.md` or `./criteria/{phase}.md`. Replace `uses: ../review/pipeline.yml` with `invoke: { pipeline: ../review/pipeline.yml }`. Shrink the dispatch-rule table in `SKILL.md`. | 1, 3, 5, 6, 7, 8 |
| 10 | Migrate `examples/skills/debug-flow/pipeline.yml` and `SKILL.md` using the same pattern as sub-task 9. `debug-flow` uses `implementation-review`, `code-review`, and `test-review` as sub-pipelines (but not `spec-review`). | 1, 3, 6, 7, 8 |
| 11 | Delete `examples/skills/audit-gate/`. Remove legacy fields from `belt-core::model::Phase` (`artifacts`, phase-level `uses`). Update parser, expander, view, and lint accordingly. Run the pre-check grep to confirm no legacy shapes remain. | 4, 5, 6, 7, 8, 9, 10 |
| 12 | Update `skills/belt-agent/SKILL.md`: add "Reading phase.invoke", "Artifact graph in status", "Validate file semantics" sections; update "Decision Rules" with `max_retries` semantics; remove `config.skill` from "Well-known Config Keys". Run the E2E smoke test across all migrated examples. | 11 |

**Commit granularity.** One commit per sub-task by default. Splitting is allowed when a sub-task produces an unavoidably large diff (sub-task 11 is a candidate).

**Parallelism.** Sub-tasks 4 through 8 are mutually independent. They may be executed concurrently in worktrees if desired, but the default is sequential execution under a single subagent-driven-development run.

### DD-7: `ArtifactRef::Qualified` — struct form only

`ArtifactRef::Qualified { name: String, from: String }` as implemented in Plan A (commit `2558247`) is the sole form. A string-encoded form such as `"phase.artifact"` is not introduced.

**Rationale.** Every migrated example uses `ArtifactRef::Named` exclusively because artifact names happen to be globally unique across the current example set. `Qualified` exists in the schema as a disambiguation escape hatch; its YAML syntax is the struct form. Introducing a parallel string form requires choosing between `.`, `/`, and `::` separators, introduces ambiguity for names that contain the chosen separator, and solves a problem that has zero occurrences today.

### DD-8: Phase without `invoke` is legal

A phase may omit `invoke:` if it has at least one of `gate:`, `validate:`, or `confirm: true`. Lint rejects only phases that have none of these and also lack `invoke:`.

**Lint rule.**

```rust
fn check_phase_is_not_meaningless(
    phase: &ExpandedPhase,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let has_action = phase.invoke.is_some();
    let has_verification = !phase.gate.is_empty() || !phase.validate.is_empty();
    let has_interaction = phase.confirm;
    if !has_action && !has_verification && !has_interaction {
        diagnostics.push(LintDiagnostic::EmptyPhase { phase_id: phase.id.clone() });
    }
}
```

**Rationale.** The parent spec's open question #5 describes "pure checkpoint phases" (waiting for external conditions, re-verifying prior phase output). A phase with only `validate:` and `confirm: true` is a valid checkpoint. Forbidding `invoke:`-less phases would reject this use case; accepting totally empty phases would hide authoring mistakes. The lint rule catches the latter only.

## Migration strategy

The dependency graph for the twelve sub-tasks is a DAG:

```
       ┌──────────┐   ┌──────────┐   ┌──────────┐
       │ 1.       │   │ 2.       │   │ 3.       │
       │ criteria │   │ mtime    │   │ validate │
       │ consol.  │   │ filter   │   │ shorthand│
       └────┬─────┘   └────┬─────┘   └────┬─────┘
            │              │              │
            └──────┬───────┴──────┬───────┘
                   ▼              ▼
             ┌─────┬─────┬─────┬─────┬─────┐
             │ 4.  │ 5.  │ 6.  │ 7.  │ 8.  │
             │smoke│spec │code │test │impl │
             │ test│ rev.│ rev.│ rev.│ rev.│
             └─────┴──┬──┴──┬──┴──┬──┴──┬──┘
                      └──┬──┴──┬──┴──┬──┘
                         │     │     │
                         ▼     ▼     ▼
                    ┌────────┴────────┐
                    │ 9.              │ 10.
                    │ feature-dev     │ debug-flow
                    └────────┬────────┘─────┐
                             │              │
                             └──────┬───────┘
                                    ▼
                            ┌───────┴──────┐
                            │ 11.          │
                            │ delete       │
                            │ audit-gate + │
                            │ legacy cutov.│
                            └───────┬──────┘
                                    ▼
                            ┌───────┴──────┐
                            │ 12.          │
                            │ SKILL.md +   │
                            │ E2E smoke    │
                            └──────────────┘
```

**Invariants during migration:**

1. After sub-tasks 1 through 10, every `examples/skills/*/pipeline.yml` is `belt lint` clean under the still-permissive parser (both old and new shapes accepted).
2. After sub-task 11, every example uses only new shapes, and the parser rejects legacy shapes.
3. After sub-task 12, `belt-agent init → next → verify → regate → step → next → ... → completed` works on every migrated example under representative arg combinations.

## Success criteria

- `cargo test -p belt-core` passes (~230 tests expected; Plan A baseline 199 plus ~30 new).
- `cargo test -p belt-agent` passes (~45 tests expected; baseline 39 plus E2E smoke).
- `cargo clippy --workspace -- -D warnings` clean.
- `cargo fmt --check` clean.
- Every `examples/skills/*/pipeline.yml` passes `cargo run -p belt -- lint`.
- `examples/skills/audit-gate/` does not exist.
- `grep -rE "config:" examples/skills/**/pipeline.yml | grep -E "skill:|criteria:|agents:"` returns zero lines.
- `grep -rE "^[[:space:]]+artifacts:" examples/skills/**/pipeline.yml` returns zero lines.
- `grep -rE "^[[:space:]]+uses:" examples/skills/**/pipeline.yml` returns zero phase-level (not gate-level) matches.
- `feature-dev/pipeline.yml` phase count drops from 19 to approximately 10.
- `feature-dev/SKILL.md` dispatch-rule table is at most three rows.
- `skills/belt-agent/SKILL.md` contains "Reading phase.invoke", "Artifact graph in status", and "Validate file semantics" sections.
- E2E smoke test: `belt-agent init examples/skills/feature-dev/pipeline.yml --arg smoke=true` advances phase-by-phase to `completed` without protocol violations.

## Risks and mitigation

| # | Risk | Severity | Mitigation |
|---|---|---|---|
| 1 | Three-way merge of `done-criteria` in sub-task 1 loses a feature-dev-specific requirement | medium | spec-reviewer carefully diffs each overlapping file; preserved via `{skill}/criteria/{name}-extra.md` if needed. |
| 2 | Sub-pipeline reference migration desync (feature-dev migrated before its review sub-pipelines) | high | Enforce the sub-task dependency graph; sub-task 9 waits for 5 through 8. |
| 3 | mtime filter precision varies across file systems (HFS one-second granularity, tmpfs ns) | low | Tests insert `thread::sleep(Duration::from_millis(10))` at phase boundaries where ordering matters. CI runs on macOS and Linux. |
| 4 | Legacy field removal breaks an external caller | none | `belt` has no external callers. Revert to commit `4608fc4` restores legacy support. |
| 5 | `validate:` scalar shorthand misclassifies an inline criterion that starts with `./` | medium | Parser tests cover every combination: `./path`, `/abs`, bare scalar, list with string, list with struct, and adversarial inputs like `./foo/bar`. |
| 6 | `feature-dev/references/evidence-plan-protocol.md` still assumes the old dispatch shape | medium | Sub-task 9 updates the protocol doc in the same commit as the pipeline migration; if the evidence plan is now better expressed as an artifact, capture that in DD followups rather than inside Plan B. |
| 7 | Additional open questions emerge during implementation | low | Open questions are resolved in-place by sub-task authors when small, or spun out as Plan B2 issues when large. |

## Test plan

### Unit tests (new)

| # | Target | Module |
|---|---|---|
| 1 | Phase-start mtime filter: single glob match newer than phase start resolves | `view_test.rs` |
| 2 | Phase-start mtime filter: multiple matches resolve to newest mtime | `view_test.rs` |
| 3 | Phase-start mtime filter: no matches after filter reports existence false | `view_test.rs` |
| 4 | Phase-start mtime filter: equal mtimes break tie via alphabetical order | `view_test.rs` |
| 5 | Phase-start mtime filter: retry within a phase does not update its `phase_start_times` entry | `engine_test.rs` |
| 6 | Phase-start mtime filter: `regate` does not modify any phase's `phase_start_times` (in-place semantics) | `engine_test.rs` |
| 7 | `validate:` scalar shorthand: `./path.md` → `vec![File]` | `model_test.rs` |
| 8 | `validate:` scalar shorthand: `"inline"` → `vec![Inline]` | `model_test.rs` |
| 9 | `validate:` list form backwards compatibility | `model_test.rs` |
| 10 | `validate:` list with bare string does not apply heuristic (stays `Inline`) | `model_test.rs` |
| 11 | Phase empty-of-everything triggers lint error | `lint_test.rs` |
| 12 | Phase with only `gate:` or only `validate:` or only `confirm:` passes lint | `lint_test.rs` |
| 13 | `max_retries` counter: verify invocation increments regardless of verdict (baseline behavior preserved) | `engine_test.rs` |
| 14 | `max_retries` counter: re-verify after validate-driven fix increments the current phase counter (document the collapsed-model flow) | `engine_test.rs` |
| 15 | `max_retries` counter: regate failure followed by re-verify increments the current phase counter; earlier phases' counters remain unchanged | `engine_test.rs` |

### Integration tests (new or updated)

| # | Target | Verification |
|---|---|---|
| 1 | Every migrated `examples/skills/*/pipeline.yml` is lint-clean | `cargo run -p belt -- lint` over each file |
| 2 | `feature-dev` full pipeline run reaches `completed` under `--arg smoke=false --arg doc=false` | `belt-agent` integration test |
| 3 | `debug-flow` full pipeline run reaches `completed` under representative args | `belt-agent` integration test |
| 4 | `smoke-test` full pipeline run reaches `completed` | `belt-agent` integration test |
| 5 | Review sub-pipelines resolve correctly when dispatched from `feature-dev` | `belt-agent` integration test |
| 6 | Artifact graph in `status` JSON contains `produces` from earlier phases with correct existence flags | `belt-agent` integration test |
| 7 | Parser rejects legacy `Phase.artifacts` after sub-task 11 | `parser_test.rs` |
| 8 | Parser rejects legacy phase-level `Phase.uses` after sub-task 11 | `parser_test.rs` |

### E2E smoke test

Sub-task 12 executes a scripted walk of each migrated example:

```bash
# Example, run for each skill
cd examples/skills/feature-dev
belt-agent init --arg smoke=false --arg e2e=false --arg doc=false
while :; do
    out=$(belt-agent next)
    status=$(echo "$out" | jq -r .status)
    [[ "$status" == "completed" ]] && break
    belt-agent verify
    belt-agent step --confirm || break
done
belt-agent status | jq -e '.status == "completed"'
```

The script is embedded in the integration test harness so the walk is verified by CI rather than by manual execution. A failing walk produces the exact phase where the protocol broke.

## References

- Parent spec: `docs/specs/2026-04-11-belt-action-data-first-class.md`
- Plan A: `docs/plans/2026-04-11-belt-action-data-first-class-plan.md`
- Plan A reference implementation: `crates/belt-core/tests/parser_test.rs::belt32_full_pipeline_with_all_new_types`
- BELT-20 (belt redesign): https://linear.app/neko-neko/issue/BELT-20
- BELT-32 (parent): https://linear.app/neko-neko/issue/BELT-32
- Memory: `project_belt32_invoker_artifact.md`, `feedback_pain_driven_first_class_principle.md`, `project_belt_additive_migration_pattern.md`, `feedback_lint_rule_narrow_exemption.md`
