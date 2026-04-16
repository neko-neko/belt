# Plugin SKILL.md Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite `feature-dev` / `bug-fix` / `belt-agent` SKILL.md so that `pipeline.yml` is the SSOT. Abolish phase-number hardcoding, remove `pipeline.yml` duplication, and strip `belt-agent` history tense. Keep `bug_fix_refresh.rs::skill_md_has_expected_sections` in lockstep with the new section names.

**Architecture:** Three SKILL.md files are verbatim-replaced with the drafts in spec Sections 4 / 5 / 6. One Rust integration test (`bug_fix_refresh.rs`) gets its expected-sections assertion updated to the new B1 structure (`## Supplement Loading` + `## Phase-specific Runtime Notes`). `pipeline.yml`, `belt-core`, supplement files, and criteria files stay untouched. Memory entries are added last.

**Tech Stack:** Markdown (SKILL.md), YAML frontmatter, Rust (belt-core integration tests).

**Spec:** `docs/superpowers/specs/2026-04-16-plugin-skill-md-refresh-design.md`

**Related context:**

- MEMORY `project_skill_md_authoring_principle.md` — Phase Map 禁止、3 責務
- MEMORY `project_review_skills_lock_test_pattern.md` — skill 削除時は同一 commit で test 更新
- MEMORY `feedback_subagent_prompt_verbatim_spec.md` — plan verbatim、paraphrase 禁止
- MEMORY `project_parallel_session_worktree_isolation.md` — branch-switch race 回避

**Prerequisite (optional but recommended):** Work in a dedicated worktree. If not using one, confirm current branch is clean apart from the spec file and `.claude/`.

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `docs/superpowers/specs/2026-04-16-plugin-skill-md-refresh-design.md` | Commit (already written) | Design SSOT |
| `plugins/feature-dev/skills/feature-dev/SKILL.md` | Full rewrite | feature-dev supplement loading + runtime notes |
| `plugins/bug-fix/skills/bug-fix/SKILL.md` | Full rewrite | bug-fix supplement loading + runtime notes |
| `skills/belt-agent/SKILL.md` | Full rewrite (audit) | belt-agent protocol reference, current-tense only |
| `crates/belt-core/tests/bug_fix_refresh.rs` | Modify `skill_md_has_expected_sections` + one docstring | Lock new section names |
| `~/.claude/projects/-Users-nishikataseiichi-go-src-github-com-neko-neko-belt/memory/project_skill_md_plugin_refresh_2026_04_16.md` | Create | Record refresh decision |
| `~/.claude/projects/-Users-nishikataseiichi-go-src-github-com-neko-neko-belt/memory/MEMORY.md` | Append one index line | Link to the new entry |

**Untouched (guard):**

- All `pipeline.yml` files (remain SSOT)
- `crates/belt-core/tests/feature_dev_refresh.rs` (no SKILL.md content assertion exists)
- `crates/belt-core/tests/review_skills_refresh.rs` (orthogonal)
- `plugins/*/skills/*/references/*-supplement.md` (content unchanged)
- `plugins/*/skills/*/criteria/*.md` (unchanged)
- `belt-core` / `belt-agent` source code

---

## Task 1: Commit the design spec

**Files:**

- Add: `docs/superpowers/specs/2026-04-16-plugin-skill-md-refresh-design.md`

- [ ] **Step 1: Confirm spec file exists and is untracked**

Run: `git status --short docs/superpowers/specs/2026-04-16-plugin-skill-md-refresh-design.md`
Expected: `?? docs/superpowers/specs/2026-04-16-plugin-skill-md-refresh-design.md`

- [ ] **Step 2: Stage and commit the spec**

```bash
git add docs/superpowers/specs/2026-04-16-plugin-skill-md-refresh-design.md
git commit -m "$(cat <<'EOF'
docs(specs): add 2026-04-16 plugin SKILL.md refresh design

Establish pipeline.yml as SSOT by purging Phase-number-keyed Invocation
Rules / Pipeline Overview / Args tables from feature-dev and bug-fix
SKILL.md, and eliminating history-tense sentences from belt-agent
SKILL.md. Adopt B1 two-section structure (Supplement Loading table +
Phase-specific Runtime Notes).
EOF
)"
```

- [ ] **Step 3: Verify clean state**

Run: `git status --short docs/superpowers/specs/2026-04-16-plugin-skill-md-refresh-design.md`
Expected: empty output (file is tracked and clean)

---

## Task 2: feature-dev SKILL.md rewrite

**Files:**

- Modify (full rewrite): `plugins/feature-dev/skills/feature-dev/SKILL.md`

- [ ] **Step 1: Confirm current baseline patterns exist (TDD red)**

Run: `grep -cE '^(## Pipeline Overview|## Args$|### Phase [0-9]+:)' plugins/feature-dev/skills/feature-dev/SKILL.md`
Expected: non-zero count (at least 11: 1 Pipeline Overview + 1 Args + 9 Phase headers)

- [ ] **Step 2: Write the new SKILL.md verbatim**

Use the `Write` tool to replace `plugins/feature-dev/skills/feature-dev/SKILL.md` with exactly this content (verbatim from spec Section 4):

````markdown
---
name: feature-dev
description: >-
  Runs a quality-gated feature-development pipeline spanning design, test
  strategy, spec review, implementation, and regression verification. Use when
  building a new feature that needs a structured design-to-integration flow.
  --e2e enables browser-based verification; --codex enables adversarial review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# feature-dev

Belt pipeline for quality-gated development. Pipeline structure, args, phase
order, and invocation mapping are defined in `pipeline.yml` and surfaced
dynamically by `belt-agent next` / `belt-agent status`. This SKILL.md covers
the supplement loading contract, phase-specific runtime notes, and red flags
that cannot be expressed in pipeline.yml.

## Supplement Loading

Before invoking a phase's skill, read the referenced supplement to inject
phase-specific overrides. Phases not listed (`test-scenarios` / `spec-review` /
`execute` / `code-review`) have no supplement; invoke their declared skill
directly.

| Phase | Supplement | Purpose |
|---|---|---|
| design | `./references/brainstorming-supplement.md` | parallel exploration (code-explorer / code-architect / impact-analyzer), implicit-rules extraction, required design sections, worktree creation order |
| plan | `./references/writing-plans-supplement.md` | path override, Must-Verify, scenarios cross-referencing |
| monkey-test | `./references/monkey-test-supplement.md` | context injection for replay |
| dogfood | `./references/dogfood-supplement.md` | overrides and prior-phase artifact hints |
| integrate | `./references/worktrunk-supplement.md` | A/B choice logic (wt merge / gh pr create) |

## Phase-specific Runtime Notes

- **spec-review**: grill-me dialogue for `requirements` / `design-judgment`
  findings; direct selection triage for the remaining observations.
- **execute**: orchestrator must reconstruct plan tasks into self-contained
  implementation specs before dispatching `belt-agents:feature-implementer`
  subagents. Do not forward broad research verbatim.
- **integrate**: prompt user for mode (A: `wt merge` / B: `gh pr create`)
  before executing `/worktrunk`.

## Narrative Notes

These phases produce a narrative note so context can be restored after `/clear`
(`.belt/runs/{run_id}/notes/phase-<id>.md`):

- `design` / `plan` / `execute` / `code-review`
- `monkey-test` (`--e2e`) / `dogfood` (`--e2e`)

Each note contains four sections (`## Decisions` / `## Concerns` /
`## Directives` / `## Observations`) and minimal frontmatter (`phase`,
`run_id`).

Full convention: [`plugins/belt-agents/references/narrative-convention.md`](plugins/belt-agents/references/narrative-convention.md)

`/clear` itself is the user's call — Claude Code runtime constraints prevent
automation. Use narrative notes as an option when context has grown large
after a heavy phase (for example, right after design, execute, or
code-review).

## Red Flags

- **Never skip supplement loading when listed above**: phase-specific overrides are lost and behavior drifts.
- **Never bypass the integrate A/B choice**: merge-vs-PR is always user-decided.
- **Never modify the consumed global skills**: all overrides go through `references/*-supplement.md`.
- **Never hand-edit files under `docs/features/<topic>/`**: they are phase-produced; manual edits break belt's phase-start mtime filter.
- **Never leave the narrative note's four sections blank**: the gate is `file_exists` only and empty sections still pass, but downstream consumers cannot restore context. Use at least `(none)` as a placeholder and always keep the heading.

## References

- `./references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `./references/brainstorming-supplement.md` — design phase overrides
- `./references/writing-plans-supplement.md` — plan phase overrides
- `./references/monkey-test-supplement.md` — monkey-test phase context injection
- `./references/dogfood-supplement.md` — dogfood phase overrides and hints
- `./references/worktrunk-supplement.md` — integrate phase A/B choice logic
- `plugins/belt-agents/references/narrative-convention.md` — narrative note schema (shared with bug-fix)
````

- [ ] **Step 3: Verify forbidden patterns are gone (TDD green)**

Run: `grep -cE '^(## Pipeline Overview|## Args$|### Phase [0-9]+:)' plugins/feature-dev/skills/feature-dev/SKILL.md`
Expected: `0`

Run: `grep -cE 'Phase [0-9]+:' plugins/feature-dev/skills/feature-dev/SKILL.md`
Expected: `0`

- [ ] **Step 4: Verify required sections are present**

Run: `grep -cE '^## (Supplement Loading|Phase-specific Runtime Notes|Narrative Notes|Red Flags|References)$' plugins/feature-dev/skills/feature-dev/SKILL.md`
Expected: `5`

Run: `grep -c 'argument-hint: "\[--e2e\] \[--codex\]"' plugins/feature-dev/skills/feature-dev/SKILL.md`
Expected: `1`

- [ ] **Step 5: Run feature-dev lock test (guards pipeline.yml)**

Run: `cargo test -p belt-core --test feature_dev_refresh`
Expected: all tests pass (this test file asserts `pipeline.yml` shape only; SKILL.md rewrite must not affect it)

- [ ] **Step 6: Commit**

```bash
git add plugins/feature-dev/skills/feature-dev/SKILL.md
git commit -m "$(cat <<'EOF'
docs(feature-dev): rewrite SKILL.md to Supplement Loading + runtime notes

Remove Pipeline Overview, Args table, and the nine phase-numbered
Invocation Rules subsections — all redundant with pipeline.yml (SSOT).
Replace with Supplement Loading table (phase id → supplement path) and
Phase-specific Runtime Notes. Abolish phase numbering so pipeline
reordering no longer forces SKILL.md churn.

Spec: docs/superpowers/specs/2026-04-16-plugin-skill-md-refresh-design.md
EOF
)"
```

---

## Task 3: bug-fix SKILL.md rewrite + lock test update

**Files:**

- Modify: `crates/belt-core/tests/bug_fix_refresh.rs:287-322` (test function `skill_md_has_expected_sections` + docstring on `skill_md_declares_supplement_injection_per_phase`)
- Modify (full rewrite): `plugins/bug-fix/skills/bug-fix/SKILL.md`

- [ ] **Step 1: Update lock test first (TDD — write the new assertion)**

Use the `Edit` tool on `crates/belt-core/tests/bug_fix_refresh.rs`.

Replace (old string, exactly 15 lines including the closing brace):

```rust
#[test]
fn skill_md_has_expected_sections() {
    let skill_md = bug_fix_dir().join("SKILL.md");
    let content = std::fs::read_to_string(&skill_md).expect("SKILL.md must exist");
    for section in [
        "## Phase-Specific Invocation Rules",
        "## Red Flags",
        "## References",
        "argument-hint:",
    ] {
        assert!(
            content.contains(section),
            "SKILL.md must contain '{section}'"
        );
    }
}
```

With (new string):

```rust
#[test]
fn skill_md_has_expected_sections() {
    let skill_md = bug_fix_dir().join("SKILL.md");
    let content = std::fs::read_to_string(&skill_md).expect("SKILL.md must exist");
    for section in [
        "## Supplement Loading",
        "## Phase-specific Runtime Notes",
        "## Red Flags",
        "## References",
        "argument-hint:",
    ] {
        assert!(
            content.contains(section),
            "SKILL.md must contain '{section}'"
        );
    }
}
```

- [ ] **Step 2: Update the second test's comment (phase-number reference)**

Use the `Edit` tool on `crates/belt-core/tests/bug_fix_refresh.rs`.

Replace:

```rust
    // Phases 1 (rca), 2 (fix-plan), 6 (monkey-test), 7 (dogfood), 8 (integrate)
    // must each reference a specific supplement via INVOKE 1 in SKILL.md.
```

With:

```rust
    // rca, fix-plan, monkey-test, dogfood, and integrate phases must each
    // reference a specific supplement inside SKILL.md's Supplement Loading table.
```

- [ ] **Step 3: Run the updated lock test — should FAIL now (TDD red)**

Run: `cargo test -p belt-core --test bug_fix_refresh skill_md_has_expected_sections`
Expected: FAIL with message `SKILL.md must contain '## Supplement Loading'` (the current bug-fix SKILL.md does not yet have the new section)

- [ ] **Step 4: Write the new bug-fix SKILL.md verbatim**

Use the `Write` tool to replace `plugins/bug-fix/skills/bug-fix/SKILL.md` with exactly this content (verbatim from spec Section 5):

````markdown
---
name: bug-fix
description: >-
  Runs a quality-gated debugging pipeline with root-cause analysis, fix planning,
  code review, and regression verification. Use when a bug needs structured
  diagnosis and verified repair. --e2e adds browser-based regression tests;
  --codex enables adversarial review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# bug-fix

Belt pipeline for quality-gated debugging. Pipeline structure, args, phase
order, and invocation mapping are defined in `pipeline.yml` and surfaced
dynamically by `belt-agent next` / `belt-agent status`. This SKILL.md covers
the supplement loading contract, phase-specific runtime notes, and red flags.

## Supplement Loading

Before invoking a phase's skill, read the referenced supplement to inject
phase-specific overrides. Phases not listed (`fix-plan-review` / `execute` /
`code-review`) have no supplement; invoke their declared skill directly.

| Phase | Supplement | Purpose |
|---|---|---|
| rca | `./references/rca-supplement.md` | RCA Report 5 sections, Symmetry Check, Reproduction Test FAIL, parallel exploration order, `rca-scenarios.yml` produce (when `--e2e`) |
| fix-plan | `./references/fix-plan-supplement.md` | RCA Fix Strategy → task traceability, Given/When/Then test cases, verifiable completion conditions, task granularity |
| monkey-test | `./references/monkey-test-supplement.md` | scenarios source = `docs/plans/*-rca-scenarios.yml`, first scenario verifies Reproduction Test now PASSes, glob collision resolution |
| dogfood | `./references/dogfood-supplement.md` | Impact Scope + Symmetry exploration, Root Cause re-emergence flag, CLI-only graceful degradation |
| integrate | `./references/worktrunk-supplement.md` | A/B choice logic (wt merge / gh pr create) |

## Phase-specific Runtime Notes

- **fix-plan-review**: `/spec-review:spec-review` is reused for fix-plan
  review. The grill-me prompt under the `design-judgment` observation does
  not fire by default (design decisions are already settled in rca /
  fix-plan). If it fires, treat it as a signal that upstream phases need
  to be revisited.
- **execute**: orchestrator must reconstruct fix plan tasks into
  self-contained implementation specs before dispatching
  `belt-agents:feature-implementer` subagents. Do not forward RCA / Fix
  Plan excerpts verbatim.
- **integrate**: prompt user for mode (A: `wt merge` / B: `gh pr create`)
  before executing `/worktrunk`.

## Narrative Notes

These phases produce a narrative note so context can be restored after `/clear`
(`.belt/runs/{run_id}/notes/phase-<id>.md`):

- `rca` / `fix-plan` / `execute` / `code-review`
- `monkey-test` (`--e2e`) / `dogfood` (`--e2e`)

Each note contains four sections (`## Decisions` / `## Concerns` /
`## Directives` / `## Observations`) and minimal frontmatter.

Full convention: [`plugins/belt-agents/references/narrative-convention.md`](plugins/belt-agents/references/narrative-convention.md)

## Red Flags

- **Never skip rca**: root cause must precede fix. "Fix first" is anti-pattern.
- **Never skip supplement loading when listed above**: without bug-fix specific overrides, behavior drifts.
- **Never delegate root cause synthesis to subagents**: the orchestrator must reconstruct parallel exploration results.
- **Never proceed without a failing reproduction test**: RCA blocker.
- **Never filter or omit review findings**: triage of `/code-review:code-review` and `/spec-review:spec-review` output is the user's responsibility.
- **Never bypass the integrate A/B choice**: merge-vs-PR is always user-decided.
- **Never hand-edit files under `docs/plans/<topic>-*`**: they are phase-produced; manual edits break belt's phase-start mtime filter.
- **Never modify the consumed global skills**: overrides go through `references/*-supplement.md` only.
- **Never leave the narrative note's four sections blank**: gate is `file_exists` only; empty sections pass but break downstream consumers.

## References

- `./references/path-convention.md` — `docs/plans/YYYY-MM-DD-<topic>-*` naming (SSOT)
- `./references/rca-supplement.md` — rca phase override
- `./references/fix-plan-supplement.md` — fix-plan phase override
- `./references/monkey-test-supplement.md` — monkey-test phase override
- `./references/dogfood-supplement.md` — dogfood phase override and CLI-only degradation
- `./references/worktrunk-supplement.md` — integrate phase A/B choice logic
- `plugins/belt-agents/references/narrative-convention.md` — narrative note schema (shared with feature-dev)
````

- [ ] **Step 5: Re-run the lock test — should PASS (TDD green)**

Run: `cargo test -p belt-core --test bug_fix_refresh`
Expected: all tests pass, including `skill_md_has_expected_sections` and `skill_md_declares_supplement_injection_per_phase`.

- [ ] **Step 6: Verify forbidden patterns gone in SKILL.md**

Run: `grep -cE '^(## Pipeline Overview|## Args$|### Phase [0-9]+:)' plugins/bug-fix/skills/bug-fix/SKILL.md`
Expected: `0`

Run: `grep -cE 'Phase [0-9]+:' plugins/bug-fix/skills/bug-fix/SKILL.md`
Expected: `0`

- [ ] **Step 7: Verify required sections present**

Run: `grep -cE '^## (Supplement Loading|Phase-specific Runtime Notes|Narrative Notes|Red Flags|References)$' plugins/bug-fix/skills/bug-fix/SKILL.md`
Expected: `5`

- [ ] **Step 8: Format + clippy on the updated test file**

Run: `cargo fmt -p belt-core`
Expected: no output (formatter has nothing to reformat, since the edit kept Rust style)

Run: `cargo clippy -p belt-core --tests -- -D warnings`
Expected: `warning:` / `error:` count = 0 — no new lints

- [ ] **Step 9: Commit**

```bash
git add plugins/bug-fix/skills/bug-fix/SKILL.md crates/belt-core/tests/bug_fix_refresh.rs
git commit -m "$(cat <<'EOF'
docs(bug-fix): rewrite SKILL.md, lock new section names in bug_fix_refresh

Remove Pipeline Overview, Args table, and the eight phase-numbered
Invocation Rules subsections from bug-fix SKILL.md — redundant with
pipeline.yml (SSOT). Adopt the B1 structure: Supplement Loading table
plus Phase-specific Runtime Notes. Update
bug_fix_refresh::skill_md_has_expected_sections so the section-name
lock tracks the new structure; retune the companion test's phase-number
comment to phase ids.

Spec: docs/superpowers/specs/2026-04-16-plugin-skill-md-refresh-design.md
EOF
)"
```

---

## Task 4: belt-agent SKILL.md audit + rewrite

**Files:**

- Modify (full rewrite): `skills/belt-agent/SKILL.md`

- [ ] **Step 1: Confirm current history-tense patterns exist (TDD red)**

Run: `grep -nE 'Note \(2026-|As of 2026-|As of 2025-|were removed|have been replaced|moved to the typed' skills/belt-agent/SKILL.md`
Expected: matches on lines around 58, 171, 173 (Note block + Config Keys past tense + "As of 2026-04-16")

- [ ] **Step 2: Write the new belt-agent SKILL.md verbatim**

Use the `Write` tool to replace `skills/belt-agent/SKILL.md` with exactly this content (verbatim from spec Section 6):

`````markdown
---
name: belt-agent
description: Belt Protocol for driving belt-agent CLI. Defines command loop, response interpretation, invoke/artifact/validate semantics, and safety constraints for LLM agents driving belt pipelines.
user-invocable: false
---

# Belt Protocol

Protocol for LLM agents driving `belt-agent` CLI — a deterministic state
machine for pipeline execution.

## Commands

```bash
belt-agent init   <pipeline.yml> [--arg key=value ...]  # Start a new run
belt-agent next   [--run <id>]                          # Get current phase info (or completion signal)
belt-agent verify [--run <id>]                          # Run gate checks for current phase
belt-agent regate [--run <id>]                          # Run regate checks for target phases
belt-agent step   [--confirm] [--run <id>]              # Advance to next phase
belt-agent status [--run <id>]                          # Inspect full run state (enriched)
```

`--run <id>` is optional on all commands; omit to use the latest run.

## Workflow

```
init → next → read phase.invoke → execute per variant →
verify (if gates) → regate (if targets) → step → next → ... → completed
```

## Reading `phase.invoke`

Every phase returned by `next` may carry an `invoke` field with one of two
variants. Read the variant and take the matching action.

| Variant | Shape | Orchestrator action |
|---|---|---|
| `skill` | `{ skill: "/name", args: { ... } }` | Invoke the Claude Code slash-command skill named in `invoke.skill`, passing `invoke.args` as parameters. |
| `pipeline` | `{ pipeline: "./path.yml", with: { ... } }` | Initialise a nested `belt-agent` run on the referenced sub-pipeline. Pass `with` as args. Treat the nested run as a black-box until it reports `completed`. |

**`pipeline` invoke — `with` template resolution.** When a `with` entry's
value is a string of the form `"args.X"` (literal prefix `args.` followed by
a single arg identifier — no nested dotted paths), resolve it against the
parent run's `args` before calling `belt-agent init --arg X=<value>`. Literal
values (bool, number, non-template string) are passed through verbatim. If
`args.X` is absent in the parent, omit the `--arg` instead of passing `null`;
the sub-pipeline's declared default applies.

If `invoke` is absent, the phase is a "pure checkpoint" with only `gate:`,
`validate:`, or `confirm:`. Proceed directly to the verify/step loop.

## Artifact Graph in `status`

`belt-agent status` returns each phase's `produces` and `consumes` as part of
the enriched view.

`produces` entries are resolved artifacts:

```json
{
  "name": "design_doc",
  "path": "docs/plans/*-design.md",
  "description": "Brainstormed design...",
  "exists": true,
  "resolved_path": "docs/plans/2026-04-11-feature-x-design.md"
}
```

`belt-core` resolves glob paths using the phase-start mtime filter: the
matching file with the newest mtime (>= phase entry timestamp) wins, ties
broken lexicographically. For concrete paths, `exists` is a direct
`std::fs::metadata` check. The `resolved_path` field is omitted from JSON
when unresolved.

`consumes` entries are artifact references — either a string (resolved by
lint against the most recent earlier phase producing that name) or
`{ "name": "...", "from": "..." }` for explicit disambiguation.

**`next` and `init` emit declared artifacts, not resolved.** The `produces`
array in `next`/`init` carries raw `{ name, path, description }` entries from
pipeline.yml — without `exists` or `resolved_path`. Filesystem resolution
only happens in `status`. Call `belt-agent status` whenever you need the
concrete path of a prior phase's output.

## Validate File Semantics

Phases may use either:

- `validate: ./criteria/name.md` (scalar file reference, relative to pipeline.yml directory)
- `validate: /abs/path.md` (absolute path)
- `validate: ["criterion one", "criterion two"]` (inline list)
- `validate: [{ file: "./x.md" }, "inline"]` (mixed)

When a validate entry is a file reference, the orchestrator MUST read the
file before `step --confirm`. The file contains the actual criteria; the
scalar in pipeline.yml is just the pointer. See
`plugins/belt-agents/references/audit-protocol.md` for the expected
criteria file format.

## Decision Rules

| Situation | Action |
|---|---|
| Phase has no `gate` | Skip `verify`. Go directly to `step`. |
| `verify` returns FAIL | Read `checks` array. Fix failing items. Re-run `verify`. Each verify invocation counts toward `max_retries`. |
| Phase has `regate` targets | After `verify` PASS, run `regate`. On FAIL, fix target phases and re-run `verify` then `regate`. |
| Phase has no `regate` targets | Skip `regate`. Go directly to `step`. |
| Phase has `validate` criteria | Verify each criterion yourself (file-ref: read file; inline: judge strings), then `step --confirm`. |
| `phase_attempts[phase] > max_retries` | `step` fails with `max_retries_exceeded`. Escalate per pipeline's `on_escalation` policy. |

Every call to `verify` increments the current phase's attempts counter
regardless of verdict. `regate` is an in-place re-verification of earlier
phases' gates; it does not modify any phase's attempts counter.

## Step Troubleshooting

When `step` returns `advanced: false`, read the `reason` field:

| `reason` | Action |
|---|---|
| `confirmation_required` | Phase has `validate` or `confirm`. Verify criteria, then `step --confirm`. |
| `verify_required` | Run `verify` first. |
| `regate_not_executed` | Run `regate` first. |
| `regate_failed` | Fix regate target phases. Re-run `verify` then `regate`. |
| `max_retries_exceeded` | Escalate. Pipeline author defines recovery via `on_escalation`. |

## Status Output

`status` returns an enriched view assembled from run state, pipeline YAML,
and output directories:

```json
{
  "status": "in_progress",
  "current_phase": "review",
  "progress": { "completed": 2, "skipped": 0, "remaining": 2, "total": 4 },
  "phases": [
    {
      "id": "build",
      "status": "completed",
      "invoke": { "skill": "/brainstorming" },
      "produces": [{ "name": "design_doc", "exists": true, "resolved_path": "docs/plans/2026-04-11-feature-x-design.md" }],
      "consumes": [],
      "outputs": ["report.json"]
    },
    {
      "id": "review",
      "status": "current",
      "invoke": { "pipeline": "../spec-review/pipeline.yml", "with": {} },
      "consumes": ["design_doc"]
    }
  ]
}
```

`produces`, `consumes`, and `invoke` are omitted when empty/absent. Treat
absence as equivalent to an empty array (or `null` for `invoke`). Use
`status` for context recovery or progress checks.

## Well-known Config Keys

`config` is an opaque map that belt passes through without interpretation.
Use it for phase-specific flags orthogonal to invocation identity (e.g.,
`codex: true`, `ui: true`, or pipeline-specific arguments). Unknown keys
MAY be ignored.

Phase-level invocation identity belongs in the typed `invoke:` field. Agent
dispatch and iteration loops are skill-layer concerns; `pipeline.yml`
references only `invoke.skill` or `invoke.pipeline`.

## HARD-GATE

<HARD-GATE>
When validate criteria exist for a phase (inline or file-reference), you
MUST NOT run `belt-agent step --confirm` without verifying each criterion.

For inline `validate: ["..."]` criteria, judge each string directly.

For file-reference `validate: ./criteria/name.md` or `validate: /abs/path.md`,
you MUST Read the referenced file first, then judge each criterion defined
inside that file. The file is the authoritative source; the scalar in
pipeline.yml is just the pointer.

The --confirm flag is the LLM's declaration that criteria have been verified.
Passing it without verification is a protocol violation.
</HARD-GATE>
`````

- [ ] **Step 3: Verify history patterns are gone (TDD green)**

Run: `grep -cE 'Note \(2026-|As of 2026-|As of 2025-|were removed|have been replaced|moved to the typed' skills/belt-agent/SKILL.md`
Expected: `0`

- [ ] **Step 4: Verify structural sections retained**

Run: `grep -cE '^## (Commands|Workflow|Reading `phase\.invoke`|Artifact Graph in `status`|Validate File Semantics|Decision Rules|Step Troubleshooting|Status Output|Well-known Config Keys|HARD-GATE)$' skills/belt-agent/SKILL.md`
Expected: `10`

- [ ] **Step 5: Verify HARD-GATE block preserved**

Run: `grep -c '<HARD-GATE>' skills/belt-agent/SKILL.md`
Expected: `1`

Run: `grep -c '</HARD-GATE>' skills/belt-agent/SKILL.md`
Expected: `1`

- [ ] **Step 6: Commit**

```bash
git add skills/belt-agent/SKILL.md
git commit -m "$(cat <<'EOF'
docs(belt-agent): audit SKILL.md, drop history tense, trim examples

Remove the "Note (2026-04-16)" block under Reading `phase.invoke`, the
"moved to the typed invoke:" past-tense sentence in Well-known Config
Keys, and the "As of 2026-04-16, ..." sentence — all historical, already
subsumed by the current-tense body. Consolidate six per-command code
blocks in the Commands section into one unified block. Compress the
Status Output JSON example to the smallest shape that still demonstrates
both variants. Restate Well-known Config Keys in current tense only.

Spec: docs/superpowers/specs/2026-04-16-plugin-skill-md-refresh-design.md
EOF
)"
```

---

## Task 5: External reference audit (read-only)

**Files:** no modifications expected; abort only if a blocking reference is found.

- [ ] **Step 1: Search active plugins / skills / agents for phase-number references**

Run:

```bash
grep -rnE 'Phase [1-9][0-9]?:' plugins/ skills/ .claude/agents/ 2>/dev/null | grep -v '^docs/'
```

Expected: empty output (no matches in active docs outside `docs/`).

If matches appear inside `.claude/agents/` or active `plugins/`/`skills/` markdown, halt and report; treat as spec gap.

- [ ] **Step 2: Search for legacy SKILL.md section names in active docs**

Run:

```bash
grep -rnE 'Phase-Specific Invocation Rules|^## Pipeline Overview' plugins/ skills/ .claude/agents/ 2>/dev/null
```

Expected: empty output.

Historical documents under `docs/plans/` and `docs/specs/` are allowed to keep these strings (frozen-in-time records) — do not modify them.

- [ ] **Step 3: Search for belt-agent history anti-patterns outside the new SKILL.md**

Run:

```bash
grep -rnE 'As of 2026-04-16|partial revert of BELT-32' plugins/ skills/ .claude/agents/ 2>/dev/null | grep -v '^skills/belt-agent/SKILL\.md'
```

Expected: empty output.

- [ ] **Step 4: (No commit — read-only audit)**

If all three greps are empty, proceed. Otherwise, create an ad-hoc follow-up task in the plan before advancing.

---

## Task 6: Full lock-test sweep (verification only)

**Files:** no modifications.

- [ ] **Step 1: feature-dev lock test**

Run: `cargo test -p belt-core --test feature_dev_refresh`
Expected: all tests pass.

- [ ] **Step 2: bug-fix lock test**

Run: `cargo test -p belt-core --test bug_fix_refresh`
Expected: all tests pass (including the updated `skill_md_has_expected_sections`).

- [ ] **Step 3: review-skills lock test (orthogonal)**

Run: `cargo test -p belt-core --test review_skills_refresh`
Expected: all tests pass (this refactor must not disturb the review-skills boundary).

- [ ] **Step 4: Full belt-core test suite (lock tests + unit tests in modified crate)**

Run: `cargo test -p belt-core`
Expected: all tests pass.

- [ ] **Step 5: (No commit — verification only)**

---

## Task 7: MEMORY entry + index update

**Files:**

- Create: `/Users/nishikataseiichi/.claude/projects/-Users-nishikataseiichi-go-src-github-com-neko-neko-belt/memory/project_skill_md_plugin_refresh_2026_04_16.md`
- Modify: `/Users/nishikataseiichi/.claude/projects/-Users-nishikataseiichi-go-src-github-com-neko-neko-belt/memory/MEMORY.md` (append one line to the entry list)

- [ ] **Step 1: Create the memory entry file**

Use the `Write` tool to create `/Users/nishikataseiichi/.claude/projects/-Users-nishikataseiichi-go-src-github-com-neko-neko-belt/memory/project_skill_md_plugin_refresh_2026_04_16.md` with this content:

```markdown
---
name: belt plugin SKILL.md refresh 2026-04-16
description: feature-dev / bug-fix / belt-agent SKILL.md を SSOT として pipeline.yml に寄せた refresh (2026-04-16)。Phase 番号撤廃、Pipeline Overview / Args 表削除、belt-agent の history tense 全削除、B1 (Supplement 表 + runtime notes 2 節) 採用。bug_fix_refresh.rs::skill_md_has_expected_sections の section-name lock を同一 commit で更新。
type: project
---

2026-04-16 に 3 つの SKILL.md を refresh し、pipeline.yml を SSOT として確立した:

- `plugins/feature-dev/skills/feature-dev/SKILL.md` — Pipeline Overview / Args / Phase-Specific Invocation Rules を削除、`## Supplement Loading` 表 + `## Phase-specific Runtime Notes` の 2 節 (B1) に再構成
- `plugins/bug-fix/skills/bug-fix/SKILL.md` — 同じ B1 構造へ
- `skills/belt-agent/SKILL.md` — Commands 6 code block を 1 block に統合、Line 58 Note / Line 171 past tense / Line 173 "As of 2026-04-16" を全削除、Well-known Config Keys を current tense に統一

**Why:** 既存原則 `project_skill_md_authoring_principle.md` (Phase Map 禁止、config key 解釈 + ドメイン制約 + references ポインタの 3 責務) に実装が追随していなかった。Phase 番号ハードコードで phase 追加・順序変更のたびに SKILL.md churn が発生していた。Anthropic の "Avoid time-sensitive information" ベストプラクティスに belt-agent SKILL.md が反していた。

**How to apply:**

- 新規 SKILL.md 追加時は phase 番号を使わず phase id で参照
- `pipeline.yml` で表現可能な情報 (invoke / gate / regate / max_retries / args / produces / consumes) は SKILL.md に書かない
- 仕様変更を語るときは history tense (`were removed`, `As of YYYY-MM-DD`) を避け current tense で書き直す
- `bug_fix_refresh.rs::skill_md_has_expected_sections` の section 名は lock。SKILL.md の section 構造を変える時は test も同一 commit で更新

Spec: `docs/superpowers/specs/2026-04-16-plugin-skill-md-refresh-design.md`
```

- [ ] **Step 2: Append index entry to MEMORY.md**

Use the `Edit` tool on `/Users/nishikataseiichi/.claude/projects/-Users-nishikataseiichi-go-src-github-com-neko-neko-belt/memory/MEMORY.md`.

Find the last existing bullet entry line (e.g., the line for `project_review_skills_2026_04_16_boundary.md`) and append this new line after it:

```
- [project_skill_md_plugin_refresh_2026_04_16.md](./project_skill_md_plugin_refresh_2026_04_16.md) — 2026-04-16 SKILL.md refresh (feature-dev / bug-fix / belt-agent を pipeline.yml 寄せ + history tense 排除 + B1 構造採用 + bug_fix_refresh.rs::skill_md_has_expected_sections 同時更新)
```

(Keep `MEMORY.md` under the truncation limit noted in its docstring; entries after 200 lines are dropped.)

- [ ] **Step 3: Verify MEMORY.md has the new entry**

Run: `grep -c 'project_skill_md_plugin_refresh_2026_04_16' /Users/nishikataseiichi/.claude/projects/-Users-nishikataseiichi-go-src-github-com-neko-neko-belt/memory/MEMORY.md`
Expected: `1`

Run: `ls -la /Users/nishikataseiichi/.claude/projects/-Users-nishikataseiichi-go-src-github-com-neko-neko-belt/memory/project_skill_md_plugin_refresh_2026_04_16.md`
Expected: file exists.

- [ ] **Step 4: (No git commit — memory files live outside the belt repo)**

The belt repo's `.gitignore` and directory structure exclude this path entirely. Memory persists through Claude's harness, not through git.

---

## Final Verification Checklist

Run these as a single sanity pass after Task 7:

- [ ] **3 SKILL.md files rewritten:**
  Run: `git log --oneline -n 10 -- plugins/feature-dev/skills/feature-dev/SKILL.md plugins/bug-fix/skills/bug-fix/SKILL.md skills/belt-agent/SKILL.md`
  Expected: three new commits, one per file (Task 2, 3, 4).

- [ ] **Forbidden patterns globally absent in the three files:**
  Run:
  ```bash
  grep -lE '## Pipeline Overview|Phase-Specific Invocation Rules|As of 2026-|were removed|have been replaced|moved to the typed' \
    plugins/feature-dev/skills/feature-dev/SKILL.md \
    plugins/bug-fix/skills/bug-fix/SKILL.md \
    skills/belt-agent/SKILL.md
  ```
  Expected: empty (no file listed).

- [ ] **Phase numbering abolished in the two plugin SKILL.md:**
  Run:
  ```bash
  grep -lE 'Phase [0-9]+:' \
    plugins/feature-dev/skills/feature-dev/SKILL.md \
    plugins/bug-fix/skills/bug-fix/SKILL.md
  ```
  Expected: empty.

- [ ] **Lock tests green:**
  Run: `cargo test -p belt-core --tests`
  Expected: all pass.

- [ ] **MEMORY entry linked from index:**
  Run: `grep 'project_skill_md_plugin_refresh_2026_04_16' /Users/nishikataseiichi/.claude/projects/-Users-nishikataseiichi-go-src-github-com-neko-neko-belt/memory/MEMORY.md`
  Expected: one hit.

- [ ] **No uncommitted source drift in belt repo apart from `.claude/`:**
  Run: `git status --short`
  Expected: only `?? .claude/` (pre-existing) remains untracked; no other dirty paths.

All boxes green → the refresh is complete. Open a PR or merge to main per standard policy; this plan does not include the merge step (use `/worktrunk` or direct `git merge` per user preference).
