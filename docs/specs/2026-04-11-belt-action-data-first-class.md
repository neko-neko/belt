# belt-core v2: Action (Invoker) and Data flow (Artifact) as First-Class Typed Primitives

**Linear**: [BELT-32](https://linear.app/neko-neko/issue/BELT-32)
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)
**Status**: Draft
**Date**: 2026-04-11

## Summary

Promote **Action** and **Data flow** from opaque `config` map to first-class typed primitives in `belt-core`'s `Phase` model, parallel to the existing `GateCheck` untagged enum:

1. **Invoker** — typed enum modeling what each phase invokes (Skill / Agent / Agents / Pipeline).
2. **Artifact** — typed struct modeling what each phase produces and consumes, via explicit `produces:` / `consumes:` fields.
3. **ValidationSource** — extend `validate:` to accept file references in addition to inline strings.

This completes `belt-core`'s state machine model. Pain-driven: it addresses 5 documented friction points in `examples/skills/`. BELT-20's "config is opaque pass-through" and "dispatch is LLM's responsibility" philosophies are preserved — `belt-core` knows the *shape* of an invocation but does not *execute* it.

## Background

### The five friction points

`examples/skills/` currently exposes five overlapping dispatch patterns, all prose-interpreted in each skill's `SKILL.md`:

| # | Pattern | Used by | Resolution |
|---|---------|---------|------------|
| 1 | `config.skill: "/slash"` | feature-dev work phases (design, plan, execute, integrate), debug-flow | Claude Code skill system |
| 2 | `config.skill` + `config.reference: "path"` | smoke-test sub-phases (env-setup, adhoc-test, vrt-check, e2e-detection) | Implicit: relative to pipeline's skill directory |
| 3 | `uses: ../audit-gate/pipeline.yml` + `config.criteria: "name"` | feature-dev and debug-flow audit phases | Implicit: `references/done-criteria/{name}.md` in the *parent* skill directory |
| 4 | `uses: ../other/pipeline.yml` | feature-dev review phases (spec-review, plan-review, code-review, test-review) | Sub-pipeline; `SKILL.md` implicitly co-located |
| 5 | `config.agents: [list]` | spec-review, code-review, test-review, implementation-review review phases | SKILL.md dispatch rule translates to parallel `Agent(subagent_type=...)` calls |

Each `SKILL.md` opens with a "Dispatch Rules" table that translates opaque config keys into orchestrator actions. These tables are prose, not structured data; `belt lint` cannot verify them.

This design surfaces five concrete friction symptoms:

- **A. Static verification absent.** `belt lint` cannot verify `config.reference` paths, `config.criteria` names, `config.skill` slash names, `config.agents` subagent types, or `uses:` SKILL.md co-location. Typos become runtime failures.
- **B. Path resolution implicit.** `config.reference: "references/foo.md"` is relative to *what*? `config.criteria: "design"` resolves to *which* `references/done-criteria/design.md`? These rules live in SKILL.md prose only.
- **C. Multiple coexisting dispatch patterns.** Authors must repeatedly ask "which pattern do I use?" with no structural guidance. Each new skill reinvents its own config vocabulary.
- **D. Audit phases are scaffolding.** `feature-dev/pipeline.yml` has 19 phases, 9 of which are `*-audit` entries that exist only to invoke `audit-gate` sub-pipeline with a matching `config.criteria`. The author must keep three names in sync manually: work phase id, audit phase id, criteria file name. Rename breaks silently.
- **E. A phase's full intent cannot be seen in one place.** Reading "what does the `design` phase actually do?" requires opening `pipeline.yml`, `SKILL.md` dispatch rules, `references/done-criteria/design.md`, and potentially sub-pipeline files. The phase entry is a fragment, not a unit.

### Additional friction: skill output tracking

`belt-core::model::Phase` defines `artifacts: Vec<String>`, introduced in BELT-20's original spec ("生成ファイル (ユーザー向けのみ)"). In practice, only `examples/skills/smoke-test/pipeline.yml::adhoc-test` uses it. feature-dev, debug-flow, and all review sub-pipelines ignore it entirely.

Consequences:

- "What did phase X produce?" is answerable only by inspecting the filesystem plus SKILL.md prose.
- "What does phase Y consume from earlier phases?" has no declarative answer. Data flow is implicit convention.
- `belt-agent status` cannot present a full artifact map to the orchestrator.
- Renaming an output file does not propagate or lint-fail.

### The 2026-04-07 "SKILL.md Authoring Principle" was a symptom-level fix

`docs/specs/2026-04-07-skill-md-authoring-principle.md` and its CLAUDE.md addition formalized that `SKILL.md` should document only what `pipeline.yml` and `belt-agent/SKILL.md` cannot express, with three responsibilities: config key interpretation, domain-specific constraints, and `references/` pointers.

The principle is correct, but it is enforced by convention alone. The fact that a *constitutional* rule was required to prevent drift between `pipeline.yml` and `SKILL.md` is itself a signal that the underlying concepts need to become typed constraints. Conventions cannot scale; types can.

## The philosophical root cause

`belt-core` models:

| Concept | Representation | Typed? |
|---------|----------------|--------|
| State | `Pipeline` / `Phase` / `RunState` | ✓ |
| Transition | `step` / `next` / `when` / `regate` | ✓ |
| Pre-condition | `when: Option<String>` | ✓ |
| Post-condition (automated) | `GateCheck` untagged enum, 4 variants | ✓ |
| Post-condition (LLM judge) | `validate: Vec<String>` | ✓ (inline only) |
| Composition | `uses:` sub-pipeline reference | ✓ |
| Recovery | `max_retries` / `regate` | ✓ (thin but typed) |
| User interaction | `confirm: bool` | ✓ (thin but typed) |
| **Action** | `config` opaque map | ✗ |
| **Data flow** | implicit filesystem + unused `artifacts:` field | ✗ |

`belt-core` is a state machine engine. The classical definition of a state machine is `(States, Transitions, Actions)`. `belt-core` types the first two and leaves the third as an opaque slot. Sub-pipeline composition (a specific kind of action) is typed via `uses:`, but skill invocation — the most common action in every example — is not.

This asymmetry is the philosophical gap:

> `belt-core` models verification as first-class typed, but leaves action as untyped. Data flow is similarly untyped despite existing as an unused `artifacts:` field.

The five friction symptoms all derive from this asymmetry. `SKILL.md` prose is the current workaround, but it cannot be verified, cannot enforce consistency, and cannot be read as a single unit alongside `pipeline.yml`.

## Alternatives explored

This section summarises the alternatives considered during the 2026-04-11 brainstorming session, each with its fatal flaw.

### α — belt-core grows first-class fields (`phase.skill`, `phase.audit`, `phase.agents`)

Promote every dispatch kind to its own top-level `Phase` field. Pros: pipeline.yml becomes self-describing; single-file authoring unit. Cons: multiple specialized fields pollute the schema; `audit:` as a first-class field is too feature-dev-centric for `belt-core`; partial rejection during brainstorming ("audit までは行き過ぎ").

### β — `belt.toml` grows into a skill manifest

Keep `pipeline.yml` opaque; add a `[bindings.*]` section to `belt.toml` declaring per-phase dispatch metadata. Pros: belt-core unchanged, philosophically pure. Cons: authors must edit two files in lockstep. The split feels artifactual rather than essential; a phase's identity is in `pipeline.yml` but its meaning is in `belt.toml`.

### γ — Sub-pipeline templates via expander input substitution

Introduce reusable "step templates" (`skill-with-audit.yml`, `review-with-fix.yml`, etc.) that `pipeline.yml` phases `uses:` with `with:` parameters. Require a new `${inputs.X}` templating facility in the expander. Pros: leverages existing primitives. Cons: template selection is essentially the same problem as dispatch-pattern selection, just relocated one level down; `${inputs.X}` substitution is the first step of a templating engine that BELT-20 explicitly rejected ("複雑な assertion DSL" non-goal).

### Direction X — Extend `validate:` to accept file references only

Narrow fix: add `ValidationSource::File` to collapse the audit-gate scaffolding pattern. Pros: minimal change, honors BELT-20 spirit, eliminates the most painful symptom (D). Cons: solves only one of five friction points. The `config.skill` / `config.reference` / `config.agents` / `uses:` dispatch multiplicity remains untouched.

### Why the selected approach is not "just another symptom treatment"

The selected approach (Invoker + Artifact first-class, with validate file-ref as a companion extension) is the *minimum complete* model that closes the philosophical asymmetry, not a targeted fix to any single symptom. It matches the shape of the existing `GateCheck` precedent: a typed untagged enum whose variants enumerate the categorically different kinds of action. Authors pick a variant by answering "what am I invoking?" — the same categorical question as "what am I verifying?" for gates.

## Design decisions

### DD-1: Promote Action to a typed `Invoker` enum

Add `Invoker` as an untagged enum with four variants, parallel to `GateCheck`. Add `Phase.invoke: Option<Invoker>`.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Invoker {
    Skill {
        skill: String,
        #[serde(default)]
        args: HashMap<String, serde_json::Value>,
    },
    Agent {
        agent: String,
        #[serde(default)]
        args: HashMap<String, serde_json::Value>,
    },
    Agents {
        agents: Vec<String>,
        #[serde(default)]
        iterations: u32,
        #[serde(default)]
        args: HashMap<String, serde_json::Value>,
    },
    Pipeline {
        pipeline: String,
        #[serde(default)]
        with: HashMap<String, serde_json::Value>,
    },
}
```

**Variant semantics:**

- `Skill` — invoke a Claude Code slash-command skill (e.g., `/brainstorming`, `/writing-plans`). The dominant form.
- `Agent` — dispatch a single subagent via the `Agent` tool with a `subagent_type`. Used for single-agent patterns (e.g., `phase-auditor`, individual exploration agents).
- `Agents` — dispatch multiple subagents in parallel, optionally with N-iteration voting. Used by review sub-pipelines (spec-review, code-review, test-review, implementation-review).
- `Pipeline` — invoke another pipeline as a sub-pipeline. Subsumes the current phase-level `uses:` field.

**`belt-core` responsibility boundary:** `belt-core` knows the *shape* of the invocation (what variant, what arguments). It does not execute the skill, dispatch the agent, or run the sub-pipeline — those remain LLM responsibilities as established in BELT-20. `belt-agent next` returns the `Invoker` as JSON; the orchestrator reads it and acts.

**Serde-saphyr untagged enum variant ordering:** following the `GateCheck` precedent documented in `project_belt_mvp_implementation.md`, variants must be ordered so that field-name disambiguation is unambiguous. Variants with more specific required fields come first; variants whose required field names might collide with other structures come last. Concrete ordering is an implementation-plan concern, but fixture tests must cover all four variants.

### DD-2: Promote Data flow to `Artifact` with `produces:` / `consumes:`

Replace the underused `artifacts: Vec<String>` field with a typed `Artifact` struct and introduce the symmetric `consumes:` field.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArtifactRef {
    Named(String),
    Qualified {
        name: String,
        from: String,
    },
}

pub struct Phase {
    // ...
    #[serde(default)]
    pub produces: Vec<Artifact>,
    #[serde(default)]
    pub consumes: Vec<ArtifactRef>,
    // artifacts: Vec<String> — removed
}
```

**Semantics:**

- `name` is the logical identifier by which downstream phases reference the artifact. Scope: unique within a run.
- `path` is the filesystem path the LLM is expected to produce. Glob is permitted for runtime-determined filenames (e.g., `docs/plans/*-design.md` where the slug is chosen during the phase).
- `description` is a one-line human-readable purpose statement.
- `ArtifactRef::Named` is the short form, resolved by lint to the most recent earlier phase that produced that name.
- `ArtifactRef::Qualified` is the explicit form for disambiguation when multiple earlier phases produce the same name.

**Glob resolution semantics deferred.** How exactly `path: "docs/plans/*-design.md"` resolves to a concrete file at runtime (mtime filter, LLM-reported path, exact-match constraint, etc.) is intentionally *not* specified in this design document. Candidates include (1) phase-start mtime filtering inside `belt-core`, (2) LLM-reported resolution via `belt-agent step --produced name=path`, and (3) exact-match restriction via runtime templating. The implementation plan will select one based on cost and ergonomics.

### DD-3: Extend `validate:` to accept file references

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValidationSource {
    Inline(String),
    File(String),  // file path relative to pipeline.yml's parent dir
}

pub struct Phase {
    // ...
    pub validate: Vec<ValidationSource>,
    // was: validate: Vec<String>
}
```

**Rationale.** BELT-20 defined `validate:` as "criteria the LLM judges". Externalized criteria files (`examples/skills/*/references/done-criteria/*.md`) are the same concept, only stored out-of-band. Admitting a file-reference variant collapses the audit-gate sub-pipeline detour: feature-dev's `design → design-audit` 2-phase pair becomes a single `design` phase with `validate: ./criteria/design.md`. The orchestrator reads the file and dispatches `phase-auditor` exactly as it does today; the structural scaffolding disappears.

**Backwards compatibility.** `validate: ["one criterion", "another"]` continues to parse as `Vec<ValidationSource::Inline>`. The only new form is `validate: [{ file: "./path.md" }]`, or the single-file shorthand `validate: "./path.md"` where the YAML engine parses a bare string into a one-element vector. (Exact syntax permitted is an implementation-plan concern.)

### DD-4: Absorb phase-level `uses:` into `Invoker::Pipeline`

The current phase-level `uses:` field is removed from `Phase`; sub-pipeline composition is expressed as `invoke: { pipeline: ... }`.

```yaml
# Before
- id: spec-review
  uses: ../spec-review/pipeline.yml

# After
- id: spec-review
  invoke:
    pipeline: ../spec-review/pipeline.yml
```

**Gate-level `uses:` is unchanged.** `GateCheck::Uses { uses, with }` remains for referencing reusable gate definition files. This is a different concept from phase composition.

### DD-5: `config` remains opaque for non-invocation metadata

`Phase.config: HashMap<String, serde_json::Value>` is retained. It is the designated place for:

- Truly custom dispatch shapes not covered by Invoker variants (forward-compatibility for future dispatch kinds).
- Runtime parameters orthogonal to the invocation's identity (e.g., `codex: true`, `ui: true`, pipeline-specific flags).

BELT-20's "config opaque pass-through" and "dispatch is LLM's responsibility" principles are preserved. The change is that invocation *identity* is no longer carried in `config`.

### DD-6: Forward-compatibility for thin fields

The following fields remain unchanged in this redesign but must have schema designs that permit future typed expansion without a breaking migration. Rule: current syntax must remain a valid proper subset of any future richer syntax.

| Field | Current | Future expansion path |
|---|---|---|
| `confirm: bool` | yes/no flag | serde untagged enum to `Confirmation { Simple(bool), Interactive { prompt, choices, default } }` |
| `max_retries: u32` | count | new `retry: Option<RetryPolicy>` field added alongside (not renamed); BELT-28 on_escalation dependent |
| `when: Option<String>` | boolean expression | serde untagged enum to `Condition { Expression(String), ArtifactPredicate { artifact_exists: String }, Compound { ... } }` |
| `regate: Vec<String>` | phase id list | serde untagged enum to `RegateSpec { Simple(Vec<String>), Conditional { targets, when } }` |

The migration pattern is **serde untagged enum**, which allows `confirm: true` and `confirm: { prompt: "..." }` to coexist without a version bump. Where a field must be renamed (e.g., `max_retries` → `retry`), use the "add alongside" pattern: introduce the new field, deprecate but retain the old field, cut over in a future release.

**Principle (one-line):** *Pain-driven first-class. Type what is painful today; keep thin what is not; ensure schema allows future expansion without breaking migration.*

This principle is the meta-rule for all future `belt-core` growth decisions.

## Concrete example migrations

### Example 1: feature-dev `design` phase

**Before** (two phases, 4 reference files, prose dispatch rules):

```yaml
- id: design
  description: "Create design spec via brainstorming"
  config:
    skill: "/brainstorming"
    swarm: "args.swarm"
  gate:
    - file_exists: "docs/plans/*-design.md"

- id: design-audit
  uses: ../audit-gate/pipeline.yml
  config:
    criteria: "design"
```

plus:

- `examples/skills/feature-dev/references/done-criteria/design.md`
- `examples/skills/feature-dev/SKILL.md` dispatch rule: `config.audit: required → Read references/done-criteria/{config.criteria}.md ...`
- `examples/skills/audit-gate/pipeline.yml` (1-phase sub-pipeline)
- `examples/skills/audit-gate/references/audit-protocol.md`
- Implicit naming chain: `design` phase id → `design-audit` phase id → `config.criteria: "design"` → `references/done-criteria/design.md`

**After** (single phase, single criteria file, no dispatch prose for audit):

```yaml
- id: design
  description: "Create design spec via brainstorming"
  invoke:
    skill: /brainstorming
    args:
      swarm: "args.swarm"
  produces:
    - name: design_doc
      path: "docs/plans/*-design.md"
      description: "Brainstormed design with requirements, impact scope, test perspectives"
  gate:
    - file_exists: "docs/plans/*-design.md"
  validate: ./criteria/design.md
  confirm: true
  max_retries: 3
```

### Example 2: feature-dev `execute` phase consuming prior phases

**After:**

```yaml
- id: execute
  description: "TDD implementation following the plan"
  invoke:
    skill: /subagent-driven-development
  consumes:
    - design_doc
    - plan_doc
    - test_cases
  produces:
    - name: implementation_diff
      path: "git://diff/HEAD"  # or a concrete manifest path
      description: "Changed files and additions"
  validate: ./criteria/execute.md
  confirm: true
  max_retries: 3
```

Downstream phases (`code-review`, `test-review`) can declare `consumes: [implementation_diff, design_doc]` to make their dependencies explicit.

### Example 3: feature-dev sub-pipeline phase

**Before:**

```yaml
- id: code-review
  uses: ../code-review/pipeline.yml

- id: code-review-audit
  uses: ../audit-gate/pipeline.yml
  config:
    criteria: "code-review"
  regate: [execute, smoke-test, doc-audit]
```

**After:**

```yaml
- id: code-review
  invoke:
    pipeline: ../code-review/pipeline.yml
  validate: ./criteria/code-review.md
  regate: [execute, smoke-test, doc-audit]
  confirm: true
```

One phase instead of two. Sub-pipeline invocation and its audit are expressed together.

### Example 4: spec-review multi-agent review phase

**Before:**

```yaml
# examples/skills/spec-review/pipeline.yml
- id: review
  description: "4-perspective spec review"
  config:
    agents:
      - spec-review-requirements
      - spec-review-design-judgment
      - spec-review-feasibility
      - spec-review-consistency
    ui_agent: spec-review-ui-design
    codex: "args.codex"
    iterations: "args.iterations"
    swarm: "args.swarm"
  confirm: true
```

**After:**

```yaml
- id: review
  description: "4-perspective spec review"
  invoke:
    agents:
      - spec-review-requirements
      - spec-review-design-judgment
      - spec-review-feasibility
      - spec-review-consistency
    iterations: "args.iterations"
    args:
      ui_agent: spec-review-ui-design
      codex: "args.codex"
      swarm: "args.swarm"
  produces:
    - name: review_findings
      path: "{output_dir}/findings.json"
      description: "Deduplicated N-way voted findings across all perspectives"
  confirm: true
```

`ui_agent`, `codex`, `swarm` remain in `args` because they are qualifiers on how the `Agents` invocation runs, not identity. The `iterations` field is hoisted to the top of the `Agents` variant because it is an intrinsic property of the invocation.

### Example 5: smoke-test env-setup phase

**Before:**

```yaml
- id: env-setup
  description: "Start dev server and verify it is accessible."
  config:
    skill: "/smoke-test"
    reference: "references/env-setup-procedure.md"
  gate:
    - cmd: "curl -sf http://localhost:${args.port:-3000}/ > /dev/null"
```

**After:**

```yaml
- id: env-setup
  description: "Start dev server and verify it is accessible."
  invoke:
    skill: /smoke-test
    args:
      reference: ./references/env-setup-procedure.md
  gate:
    - cmd: "curl -sf http://localhost:${args.port:-3000}/ > /dev/null"
```

The `reference:` remains inside `Skill.args` because it is smoke-test-specific, not universal. (If future examples reveal a broadly used "procedure file" pattern, `reference:` could be promoted to a first-class sub-field of `Skill`. Pain-driven.)

## Impact analysis

### `belt-core` changes

| Module | Change | ~LOC |
|--------|--------|------|
| `model` | Add `Invoker`, `Artifact`, `ArtifactRef`, `ValidationSource`. Modify `Phase` (add `invoke`, `produces`, `consumes`; replace `artifacts` with `produces`; extend `validate`; remove phase-level `uses`). | 150 |
| `parser` | Wire new fields through serde (most is automatic). | 30 |
| `expander` | Pass new fields through sub-pipeline expansion. `Invoker::Pipeline` replaces the old `uses:` path but existing expander logic is reusable. | 50 |
| `lint` | New rules: invoker shape verification (slash format, agent name format, pipeline path existence, with/inputs match), artifact name uniqueness, consumes resolution against earlier produces, validate file existence. | 200 |
| `engine` | Enrich `next` / `verify` / `status` JSON with typed invoker and artifact data. | 100 |
| `view` | Extend the BELT-29 `StatusView` to include per-phase invoke, produces (with existence checks), and consumes (with resolved paths). | 80 |
| Tests | Model unit tests (including untagged enum variant ordering), parser integration, lint rule integration, engine E2E. | 400 |
| **Total** | | **~1000** |

### `belt` (lint CLI) changes

No new commands. Lint rules from `belt-core::lint` are automatically surfaced. Integration tests validate that lint output is clear for each new rule.

### `belt-agent` changes

- No new commands.
- `next` JSON output includes `invoke`, `produces`, `consumes` fields on the current phase.
- `status` JSON output includes the artifact graph (per-phase produces with existence, consumes with resolved paths).
- `verify` / `step` / `regate` command behavior unchanged at the CLI level; the file-reference variant of `validate:` is handled by the orchestrator (LLM) reading the file, not by `belt-agent`.

### `skills/belt-agent/SKILL.md` (protocol skill) changes

Add a "Reading phase.invoke" section documenting each Invoker variant and the expected orchestrator action for each. Add an "Artifact graph in status" section. Remove references to `config.skill` as a canonical pattern (it is no longer the recommended form).

### `examples/` migration

| Skill | Change |
|-------|--------|
| `feature-dev` | 19 phases → ~10 phases (all `*-audit` entries collapse). All `config.skill` migrated to `invoke: { skill: ... }`. `produces:` / `consumes:` declared for all artifact-generating phases. Dispatch rule table in `SKILL.md` reduces to 2 rules (one per invoker variant encountered). |
| `debug-flow` | 16 phases → ~10 phases. Same pattern as feature-dev. |
| `smoke-test` | All `config.skill` / `config.reference` migrated. `produces:` declared for `smoke-test-report.md`. |
| `spec-review` | `config.agents` → `invoke: { agents: ... }`. Produces `review_findings`. |
| `code-review` | Same pattern as spec-review. |
| `test-review` | Same pattern. |
| `implementation-review` | Same pattern. |
| `audit-gate` | **Deleted.** No longer needed: the "audit" concept is expressed by `validate: ./criteria/X.md` on the work phase. Associated `done-criteria/` and `references/audit-protocol.md` either move to a shared location or are referenced directly by each skill's `criteria/` directory. |

### Duplicate `done-criteria` consolidation

Currently, four done-criteria files exist in both `audit-gate/done-criteria/` (claimed as "generic canonical") and `feature-dev/references/done-criteria/` (feature-dev specific). In practice, the feature-dev copies are read at runtime because the dispatch rule resolves relative to the parent skill directory.

After this redesign, the `audit-gate` directory is deleted. Canonical "generic" done-criteria, if still desired as a shared library, can live under `examples/criteria/` and be referenced by multiple skills' `validate:` fields. Duplication is eliminated by explicit file references.

## Scope

### In scope for this redesign

- `Invoker` untagged enum in `belt-core::model`
- `Artifact` struct and `ArtifactRef` untagged enum in `belt-core::model`
- `ValidationSource` untagged enum in `belt-core::model`
- `Phase` field modifications (add `invoke`, `produces`, `consumes`; replace `artifacts`; extend `validate`; remove phase-level `uses`)
- Parser, expander, and engine updates to flow new fields
- Lint rules for invoker shape, consumes resolution, validate file existence, invoker pipeline path existence
- `belt-agent next` / `verify` / `status` JSON enrichment
- `view` module extension for artifact graph in status
- Migration of all example skills (`feature-dev`, `debug-flow`, `smoke-test`, `spec-review`, `code-review`, `test-review`, `implementation-review`)
- Deletion of `examples/skills/audit-gate/` after migration completes
- `skills/belt-agent/SKILL.md` protocol documentation update
- Forward-compatibility schema for `confirm:`, `max_retries:`, `when:`, `regate:` (design only; no behavior change)

### Out of scope (explicitly deferred)

- **Glob resolution semantics for `Artifact.path`.** Deferred to implementation plan.
- **`confirm:` type expansion.** Forward-compat schema only; no new behavior.
- **`retry:` policy typing.** Blocked on BELT-28 (`on_escalation` design).
- **`when:` expression richness.** Forward-compat schema only.
- **`regate:` conditional targeting.** Forward-compat schema only.
- **Phase-level timeout.** BELT-31 solved gate-cmd timeout; phase-level is a separate future concern.
- **Parallel phase composition / async execution.** BELT-20 Non-Goal.
- **Gate-level `uses:`.** Unchanged; different concept from phase composition.
- **New CLI commands in `belt-agent`.**

## Migration strategy

### Phase 1 — Additive (backwards compatible)

1. Add all new fields to `Phase` and supporting types.
2. Parser accepts both old (`config.skill`, `uses:`, `artifacts:`, `validate: list[string]`) and new (`invoke:`, `produces:`, `consumes:`, `validate: list[ValidationSource]`) forms.
3. Lint rules for new fields are opt-in warnings for old forms.
4. `belt-agent` returns both old and new JSON shapes during the transition.

### Phase 2 — Example migration

Migrate all `examples/skills/` in a single PR series:

1. `smoke-test` (smallest, no audit-gate dependency)
2. `spec-review`, `code-review`, `test-review`, `implementation-review` (sub-pipelines, no audit-gate dependency)
3. `feature-dev` (depends on audit-gate collapse via `validate:`)
4. `debug-flow` (same as feature-dev)
5. Delete `examples/skills/audit-gate/`

Each migration PR updates the skill's `pipeline.yml`, `SKILL.md` (shrinks dispatch rules), and migrates `references/done-criteria/*.md` paths.

### Phase 3 — Deprecation and removal

1. Lint warns on old forms.
2. After one release cycle (or per user decision, since `belt` has no external consumers yet), remove old field support from the parser.
3. `Phase` loses `artifacts` and phase-level `uses:`.

Because `belt` currently has no external consumers beyond this repository, the Phase 3 cutover can happen immediately after Phase 2 completes, without a long deprecation window.

## Open questions (to be resolved in the implementation plan)

1. **Glob resolution for `Artifact.path`.** Options: (a) phase-start mtime filter, (b) LLM-reported via `belt-agent step --produced name=path`, (c) exact-match only with runtime templating, (d) hybrid (mtime filter default with explicit override). The implementation plan will select based on ergonomics and complexity cost.
2. **`ArtifactRef::Qualified` syntax.** Currently proposed as `{ name: "design_doc", from: "design" }`. Alternative: a string form `"design.design_doc"`. Pick one.
3. **`Invoker::Agent` vs `Invoker::Agents`.** Should they be unified with `Agents { agents: Vec<String>, iterations: u32 }` handling both single-agent (length 1) and multi-agent cases? Or kept separate for clarity? Trade-off: schema simplicity vs author intent expressivity.
4. **`Skill.reference` promotion.** smoke-test uses `config.reference` with every `config.skill`. Should `reference:` become an optional sub-field of `Invoker::Skill`, or remain inside `args`? Pain-driven: if more skills adopt the pattern, promote later.
5. **Phase without `invoke`.** Is a phase with only `gate` + `validate` (no action) legal? Use case: pure checkpoint phases, e.g., "wait for external condition" or "verify prior phase's output one more time". Currently all phases have dispatch intent, so this is rare but possible.
6. **`max_retries` semantics after work+audit collapse.** In the new model, a single phase contains both the work invocation and its validation. Does `max_retries: 3` count only work failures, or work+validate combined? Clarify in docs.
7. **Exact `validate:` syntax for file references.** Candidates: `validate: [{ file: "./x.md" }]`, `validate: ["./x.md"]` (string-heuristic: path if starts with `./` or `/`), `validate: "./x.md"` (single string shorthand). Select based on least surprise.
8. **Untagged enum variant ordering for `Invoker`.** Ordering affects serde-saphyr's field-name disambiguation (per the `GateCheck` precedent). Draft ordering: `Pipeline` last (because `pipeline` field name might collide), `Skill` first (most common), `Agent` before `Agents` (more specific field first). Verify with fixture tests.

## Test plan

### Model / parser unit tests (belt-core)

| # | Target | Verification |
|---|--------|--------------|
| 1 | Parse `invoke: { skill: "/foo", args: { a: 1 } }` | `Invoker::Skill` variant with correct fields |
| 2 | Parse `invoke: { agent: "phase-auditor" }` | `Invoker::Agent` variant |
| 3 | Parse `invoke: { agents: [a, b, c], iterations: 3 }` | `Invoker::Agents` with default args |
| 4 | Parse `invoke: { pipeline: "./sub.yml", with: { k: v } }` | `Invoker::Pipeline` variant |
| 5 | Parse `produces: [{ name, path, description }]` | `Vec<Artifact>` with all fields |
| 6 | Parse `consumes: ["design_doc"]` | `Vec<ArtifactRef::Named>` |
| 7 | Parse `consumes: [{ name: "d", from: "p" }]` | `Vec<ArtifactRef::Qualified>` |
| 8 | Parse `consumes: [short, { name, from }]` mixed | Both variants in vec |
| 9 | Parse `validate: ["inline"]` | `ValidationSource::Inline` (backwards compat) |
| 10 | Parse `validate: [{ file: "./x.md" }]` | `ValidationSource::File` |
| 11 | Parse `validate: ["inline", { file: "./x.md" }]` mixed | Mixed vec |
| 12 | Untagged enum variant order (serde-saphyr disambiguation) | All 4 Invoker variants + 2 ArtifactRef variants + 2 ValidationSource variants parse correctly without cross-contamination |

### Lint integration tests

| # | Target | Verification |
|---|--------|--------------|
| 1 | `invoke: { skill: "no-slash" }` | Error: missing leading slash |
| 2 | `invoke: { pipeline: "./nonexistent.yml" }` | Error: file not found |
| 3 | `consumes: ["unknown_artifact"]` with no earlier producer | Error: unresolved artifact reference |
| 4 | `consumes: [{ name: "foo", from: "unknown_phase" }]` | Error: unknown phase |
| 5 | `produces: [{ name: "x" }, { name: "x" }]` duplicate in one phase | Error: duplicate artifact name |
| 6 | `validate: [{ file: "./nonexistent.md" }]` | Error: file not found |
| 7 | Valid feature-dev-style pipeline with full Invoker + Artifact + validate-file | No errors |

### Engine / belt-agent E2E tests

| # | Target | Verification |
|---|--------|--------------|
| 1 | `belt-agent init` on migrated feature-dev example | Succeeds; state.json contains new fields |
| 2 | `belt-agent next` returns phase with `invoke`, `produces`, `consumes` in JSON | JSON shape validated |
| 3 | `belt-agent status` returns artifact graph with existence checks | JSON shape validated |
| 4 | Full mocked run through `design → plan → execute → code-review` phases | Succeeds end-to-end |
| 5 | Re-gate flow through a migrated code-review phase with `validate: ./criteria/code-review.md` | Verdict file flow works correctly |

### Migration verification

| # | Target | Verification |
|---|--------|--------------|
| 1 | Each migrated example lint-clean | `cargo run -p belt -- lint` |
| 2 | `feature-dev` has zero `*-audit` scaffolding phases | Manual inspection of pipeline.yml |
| 3 | `audit-gate` directory deleted | `ls` |
| 4 | No `done-criteria` file exists in more than one directory | `find` + dedup check |
| 5 | All `SKILL.md` dispatch rule tables are ≤ 3 rows | Manual inspection |

## References

- **BELT-20**: belt 再設計: LLM 向け超軽量ワークフローエンジン CLI (parent spec)
- **BELT-28**: design: on_escalation field (pause/skip/abort) — backlog; blocks full retry policy typing
- **BELT-29**: enhance: status command enrichment — `view` module that this redesign extends
- **BELT-31**: feat: add timeout to gate cmd execution — complementary, gate-level only
- `docs/specs/2026-04-06-belt-redesign.md` — Original belt redesign spec
- `docs/specs/2026-04-07-skill-md-authoring-principle.md` — Authoring principle that pointed to this underlying gap
- `docs/specs/2026-04-07-feature-dev-belt-migration.md` — Migration that first exposed the five friction symptoms at scale
- `examples/skills/feature-dev/pipeline.yml` — Canonical example of the five dispatch pattern problem
- Memory files: `project_belt_architecture.md`, `project_belt_redesign_2026_04_06.md`, `project_belt_mvp_implementation.md`, `project_skill_md_authoring_principle.md`, `project_feature_dev_belt_migration.md`, `project_belt_pipeline_fitness.md`
