# README Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite `README.md` for engine-first positioning with an inline `belt lint` demo, per spec `docs/superpowers/specs/2026-04-16-readme-refresh-design.md`. Reposition `belt` as a workflow engine for LLM-driven Agent Skills and surface the value of static pre-checks.

**Architecture:** Seven localized edits to a single file (`README.md`) in source order (Intro → Why → Example+lint demo → CLI lint efficacy paragraph → Plugins heading + phase-enumeration replacement). Each edit is a verbatim transcription of the spec's Section 5 retain/delete/new enumeration; the plan must not paraphrase. No other source or test file is modified.

**Tech Stack:** Markdown. `belt lint` (Rust binary built from `crates/belt/`) is used as a read-only pre-flight check to confirm the demo's output format has not drifted.

**Spec:** `docs/superpowers/specs/2026-04-16-readme-refresh-design.md`

**Related context:**

- MEMORY `feedback_subagent_prompt_verbatim_spec.md` — plan controller prompts must transcribe spec retain/delete lists verbatim; paraphrase causes spec drift.
- MEMORY `project_skill_md_authoring_principle.md` — SSOT is `pipeline.yml`; README must not re-state phase ordering.
- MEMORY `project_parallel_session_worktree_isolation.md` — if another session is in flight, work in a dedicated worktree to avoid branch-switch races.

**Prerequisite (optional but recommended):** Work in a dedicated worktree (e.g., `wt switch --create readme-refresh`). If not using one, confirm the current branch has no uncommitted changes beyond untracked `.claude/` entries. The spec itself is already committed (`af03c43`).

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `README.md` | 5 localized edits (Tasks 2–6) | Engine-first narrative + lint demo + Plugins heading |

**Untouched (guard):**

- `CLAUDE.md` — responsibility split is declared in the commit message only; no edit
- `plugins/README.md` — not created (α case rejected in spec)
- All `pipeline.yml` files — SSOT; README refers to them but does not duplicate phase ordering
- All source code under `crates/` — read-only inspection in Task 1
- All integration tests under `crates/*/tests/` — no README assertion exists

---

## Task 1: Pre-flight — verify `belt lint` output format and implementation integrity

**Rationale:** Spec Section 4.3 hard-codes `ok: review-and-ship.yml` as the demo output. Spec Section 5.4 enumerates eight lint-error categories. Both must match the current `belt` implementation at the moment of editing; the spec was verified on 2026-04-16 but the plan may run later.

**Files:**

- Read: `crates/belt/src/main.rs`
- Read: `crates/belt-core/src/lint.rs`
- Execute (no changes): `cargo run -q -p belt -- lint <any-existing-pipeline.yml>`

- [ ] **Step 1: Confirm the success-path format string**

Run: `grep -nE 'ok: \{display\}' crates/belt/src/main.rs`
Expected: exactly one match reading `eprintln!("ok: {display}");` (success with no warnings).

If the match is absent or differs, STOP and report: the demo in `README.md` spec Section 5.3 assumes `ok: <path>` — the plan cannot proceed until the spec is updated.

- [ ] **Step 2: Confirm the warning-path and error-path formats**

Run: `grep -nE 'ok \(with warnings\): \{display\}|error: \{' crates/belt/src/main.rs`
Expected: at least one `ok (with warnings): {display}` literal and at least one `error: {...}` literal (both `eprintln!` calls). These are background fallbacks, not used in the demo, but their existence confirms `main.rs` has not been restructured.

- [ ] **Step 3: Run lint on an existing pipeline and observe output**

Run: `cargo run -q -p belt -- lint plugins/feature-dev/skills/feature-dev/pipeline.yml 2>&1 | tail -5`
Expected: the last line is either `ok: <path>` or `ok (with warnings): <path>`. Non-zero exit acceptable only if `error:` lines precede the summary (then STOP and report drift).

- [ ] **Step 4: Confirm the eight lint-error categories exist in `lint.rs`**

Run each grep; each must produce at least one match:

```
grep -n 'duplicate phase id' crates/belt-core/src/lint.rs
grep -n "regate target '{}' does not exist" crates/belt-core/src/lint.rs
grep -n 'when references undefined arg' crates/belt-core/src/lint.rs
grep -n 'leaf phase must have a description' crates/belt-core/src/lint.rs
grep -n "gate uses '{}' not found" crates/belt-core/src/lint.rs
grep -n "invoke pipeline '{}' not found" crates/belt-core/src/lint.rs
grep -n 'check_artifact_flow' crates/belt-core/src/lint.rs
grep -n 'expansion error' crates/belt-core/src/lint.rs
```

Expected: every grep returns at least one match. If any grep misses, STOP and report: the spec's Section 5.4 enumeration is drifting and needs update before the plan proceeds.

- [ ] **Step 5: Commit checkpoint (no file changes yet)**

No commit needed — this task is read-only verification. Proceed to Task 2 only if all four preceding steps passed.

---

## Task 2: Intro rewrite (`README.md` lines 1–4)

**Files:**

- Modify: `README.md` lines 1–4

- [ ] **Step 1: Apply the Intro replacement**

Use Edit with the verbatim strings from spec Section 5.1.

`old_string` (4 lines):

```
# belt

A lightweight workflow engine for AI agents. Define deterministic state machines
in YAML, drive them idempotently from any LLM.
```

`new_string` (5 lines):

```
# belt

A workflow engine for LLM-driven Agent Skills. Declare deterministic state
machines in YAML, drive them idempotently from any LLM, and lint them
statically before they ever reach execution.
```

- [ ] **Step 2: Verify the old Intro text is gone**

Run: `grep -nE 'A lightweight workflow engine for AI agents' README.md`
Expected: no output (grep exits non-zero).

- [ ] **Step 3: Verify the new Intro text is present**

Run: `grep -nE 'LLM-driven Agent Skills' README.md`
Expected: exactly one match on line 3.

- [ ] **Step 4: Commit checkpoint (no commit yet)**

No commit yet — all README edits are grouped into a single commit at Task 8. Proceed to Task 3.

---

## Task 3: Why belt? — append pre-check paragraph (`README.md` after line 13)

**Files:**

- Modify: `README.md` — insert a paragraph after the existing Why block

- [ ] **Step 1: Apply the paragraph insertion**

Use Edit with the verbatim strings from spec Section 5.2. The retained Why block ends with `The pipeline definition never enters the context window.`; the new paragraph follows after one blank line.

`old_string`:

```
to check gates. The pipeline definition never enters the context window.

## Example
```

`new_string`:

```
to check gates. The pipeline definition never enters the context window.

Pipelines are statically linted with `belt lint` before any LLM run, so
structural errors — missing phase IDs, invalid gate checks, broken `uses:`
references — never reach execution.

## Example
```

(The `## Example` heading is included in both sides to anchor the Edit to a unique location. The only change is the insertion of the three-line paragraph + one blank line between the Why block and `## Example`.)

- [ ] **Step 2: Verify the new paragraph is present**

Run: `grep -nE 'Pipelines are statically linted with' README.md`
Expected: exactly one match.

- [ ] **Step 3: Verify the Why block head is unchanged**

Run: `grep -nE 'When LLM agents control entire workflows' README.md`
Expected: exactly one match (same line number as before the edit; content unchanged).

- [ ] **Step 4: Commit checkpoint**

No commit yet. Proceed to Task 4.

---

## Task 4: Example + inline lint demo (`README.md` after line 48)

**Files:**

- Modify: `README.md` — insert a demo block after the Example YAML fenced block

- [ ] **Step 1: Apply the lint-demo insertion**

Use Edit with verbatim strings from spec Section 5.3. The Example YAML fenced block closes with a line containing exactly three backticks; the new demo block follows after one blank line, and is itself followed by one blank line then the `## CLI` heading.

Because the inserted text itself contains triple-backtick fences, the `old_string` and `new_string` cannot be rendered inside this plan as a nested fenced block. They are given below as line-by-line character sequences — copy each line verbatim into the Edit tool (including blank lines and the triple-backtick markers as literal text).

`old_string` lines (exact characters, 4 lines total):

- Line A: `    confirm: true`
- Line B: ` ``` ` (three backticks, no surrounding spaces — this is the Example's closing fence)
- Line C: (empty)
- Line D: `## CLI`

`new_string` lines (exact characters, 15 lines total):

- Line A: `    confirm: true`
- Line B: ` ``` ` (three backticks — Example closing fence, unchanged)
- Line C: (empty)
- Line D: `Lint it before handing it to the agent:`
- Line E: (empty)
- Line F: ` ``` ` (three backticks — new demo opening fence)
- Line G: `$ belt lint review-and-ship.yml`
- Line H: `ok: review-and-ship.yml`
- Line I: ` ``` ` (three backticks — new demo closing fence)
- Line J: (empty)
- Line K: `If any phase id is duplicated, a gate is malformed, or a ` + backtick + `uses:` + backtick + ` reference is`
- Line L: `unresolvable, lint exits non-zero with a descriptive diagnostic and the agent`
- Line M: `is never invoked.`
- Line N: (empty)
- Line O: `## CLI`

**Implementer note:** If the Edit tool's uniqueness check fails because `    confirm: true` appears elsewhere, broaden `old_string` to include several preceding Example YAML lines (e.g., starting from `  - id: ship`). The anchor logic is: match the end of the Example fenced block immediately before the `## CLI` heading.

- [ ] **Step 2: Verify the demo code block is present**

Run: `grep -nE '^\$ belt lint review-and-ship\.yml$' README.md`
Expected: exactly one match.

- [ ] **Step 3: Verify the demo success-line is present**

Run: `grep -nE '^ok: review-and-ship\.yml$' README.md`
Expected: exactly one match.

- [ ] **Step 4: Verify the compensation sentence is present**

Run: `grep -nE 'lint exits non-zero with a descriptive diagnostic' README.md`
Expected: exactly one match.

- [ ] **Step 5: Commit checkpoint**

No commit yet. Proceed to Task 5.

---

## Task 5: CLI lint-efficacy paragraph (`README.md` after the Binary Separation table)

**Files:**

- Modify: `README.md` — insert a paragraph between the Binary Separation table and the `### Agent loop` heading

- [ ] **Step 1: Apply the paragraph insertion**

Use Edit with verbatim strings from spec Section 5.4. The Binary Separation table's last row ends with `... runtime` (the `belt-agent` row's description); the new paragraph follows after one blank line, before the `### Agent loop` heading.

`old_string`:

```
| `belt-agent` | LLM / CI / scripts | `init`, `next`, `verify`, `step`, `status` — runtime |

### Agent loop
```

`new_string`:

```
| `belt-agent` | LLM / CI / scripts | `init`, `next`, `verify`, `step`, `status` — runtime |

`belt lint` is the pipeline author's fast feedback loop: it runs in
milliseconds, catches structural errors (duplicate phase IDs, unknown `regate`
targets, undefined args referenced from `when:`, missing descriptions,
unresolvable `uses:` / `invoke.pipeline:` references, artifact flow
violations, and sub-pipeline expansion failures), and exits non-zero on any
finding — ideal for pre-commit hooks and CI.

### Agent loop
```

- [ ] **Step 2: Verify the paragraph is present**

Run: `grep -nE 'fast feedback loop' README.md`
Expected: exactly one match.

- [ ] **Step 3: Verify the lint-error list includes all eight categories**

Run: `grep -E "duplicate phase IDs.*unknown .regate. targets.*undefined args.*missing descriptions.*unresolvable.*artifact flow violations.*sub-pipeline expansion failures" README.md`
Expected: exactly one match (the full enumeration appears on one or more adjacent lines; grep may match either the flattened line or the paragraph depending on `multiline` setting — if the default grep misses, use `rg -U` or `grep -zE`).

Alternative explicit sub-greps (preferred for reliability):

```
grep -n 'duplicate phase IDs' README.md
grep -n 'unknown `regate` targets' README.md
grep -n 'undefined args referenced from `when:`' README.md
grep -n 'missing descriptions' README.md
grep -n 'unresolvable `uses:` / `invoke.pipeline:` references' README.md
grep -n 'artifact flow violations' README.md
grep -n 'sub-pipeline expansion failures' README.md
```

Expected: every sub-grep produces at least one match.

- [ ] **Step 4: Commit checkpoint**

No commit yet. Proceed to Task 6.

---

## Task 6: Plugins heading + phase-enumeration replacement (`README.md` lines 127 and 137–138)

**Files:**

- Modify: `README.md` — rename the Plugins section heading and replace two table rows

- [ ] **Step 1: Rename the Plugins section heading**

Use Edit with verbatim strings from spec Section 5.7.

`old_string`: `## Claude Code Plugins`
`new_string`: `## Claude Code Plugins (Working Examples)`

(If `## Claude Code Plugins` is not unique on its own — e.g., appears in another paragraph — broaden `old_string` to include the following blank line and the intro sentence `belt ships 7 Claude Code plugins`.)

- [ ] **Step 2: Replace the `feature-dev` and `bug-fix` table rows**

Use Edit with verbatim strings from spec Section 5.7. These two rows are consecutive.

`old_string`:

```
| `feature-dev` | 9-phase development pipeline (design → test-scenarios → spec-review → plan → execute → code-review → monkey-test → dogfood → integrate) |
| `bug-fix` | 8-phase debugging pipeline (rca → fix-plan → fix-plan-review → execute → code-review → monkey-test → dogfood → integrate) |
```

`new_string`:

```
| `feature-dev` | Quality-gated feature-development pipeline (design → implementation → review → integration). Phase structure is declared in the plugin's `pipeline.yml` |
| `bug-fix` | Quality-gated debugging pipeline (RCA → fix → review → integration). Phase structure is declared in the plugin's `pipeline.yml` |
```

- [ ] **Step 3: Verify the new heading is present**

Run: `grep -nE '^## Claude Code Plugins \(Working Examples\)$' README.md`
Expected: exactly one match.

- [ ] **Step 4: Verify the old heading is gone**

Run: `grep -nE '^## Claude Code Plugins$' README.md`
Expected: no output.

- [ ] **Step 5: Verify the old phase enumerations are gone**

Run: `grep -nE '9-phase development pipeline \(design →' README.md`
Expected: no output.

Run: `grep -nE '8-phase debugging pipeline \(rca →' README.md`
Expected: no output.

- [ ] **Step 6: Verify the new table rows are present**

Run: `grep -nE 'Quality-gated feature-development pipeline' README.md`
Expected: exactly one match.

Run: `grep -nE 'Quality-gated debugging pipeline' README.md`
Expected: exactly one match.

Run: `grep -nE "Phase structure is declared in the plugin's .pipeline\.yml." README.md`
Expected: exactly two matches (one for each row).

- [ ] **Step 7: Commit checkpoint**

No commit yet. Proceed to Task 7.

---

## Task 7: Consolidated verification

**Files:**

- Read only: `README.md`

- [ ] **Step 1: Confirm the line count is in the expected range**

Run: `wc -l README.md`
Expected: between 225 and 240 lines. (Spec Section 4 forecasts 232 lines.)

If outside the range, re-read the diff (`git diff README.md`) and confirm none of Tasks 2–6 inserted or removed extra blank lines. Acceptable tolerance is ±5 from 232.

- [ ] **Step 2: Run the spec's grep check 7.2 — deletions present**

Run each; each MUST produce no output (grep exits non-zero):

```
grep -nE '9-phase development pipeline \(design →' README.md
grep -nE '8-phase debugging pipeline \(rca →' README.md
grep -nE '^## Claude Code Plugins$' README.md
grep -nE 'A lightweight workflow engine for AI agents' README.md
```

- [ ] **Step 3: Run the spec's grep check 7.3 — additions present**

Run each; each MUST produce exactly one match:

```
grep -nE 'LLM-driven Agent Skills' README.md
grep -nE 'Pipelines are statically linted with' README.md
grep -nE '\$ belt lint review-and-ship\.yml' README.md
grep -nE 'ok: review-and-ship\.yml' README.md
grep -nE 'fast feedback loop' README.md
grep -nE '## Claude Code Plugins \(Working Examples\)' README.md
```

- [ ] **Step 4: Manual proof-read**

Read the entire `README.md` top to bottom once (use the Read tool). Verify:

1. All section headings are in order: `# belt` → `## Why belt?` → `## Example` → `## CLI` → `## Key Concepts` → `## Build` → `## Claude Code Plugins (Working Examples)` → `## License`.
2. The lint demo code block immediately follows the Example YAML block, with one blank line between them.
3. The CLI lint-efficacy paragraph immediately follows the Binary Separation table, with one blank line between.
4. No stray `Phase 1:` / `Phase 2:` / `9-phase` / `8-phase` enumerations remain.
5. All fenced code blocks have matching openers and closers.

If any check fails, revert the offending edit with Edit tool and re-run the relevant task.

- [ ] **Step 5: No-op Rust test sanity check**

Run: `cargo test -p belt-core --test feature_dev_refresh`
Expected: PASS (README changes are orthogonal to this test; this run is a regression check, not a README validator).

Run: `cargo test -p belt-core --test bug_fix_refresh`
Expected: PASS (same rationale).

- [ ] **Step 6: Commit checkpoint**

No commit yet. Proceed to Task 8.

---

## Task 8: Commit the README refresh

**Files:**

- Commit: `README.md`

- [ ] **Step 1: Stage the file**

Run: `git add README.md`

- [ ] **Step 2: Confirm the staged diff matches expectations**

Run: `git diff --cached --stat README.md`
Expected: `README.md | X +++-- Y ---` where X (insertions) is ~30 and Y (deletions) is ~5. The exact numbers depend on blank-line handling; what matters is that the diff is localized (no spurious reflows).

Run: `git diff --cached README.md | head -120`
Expected: the diff hunks correspond to Tasks 2–6 only — Intro, Why paragraph, Example demo, CLI paragraph, Plugins heading + two table rows. No changes to Key Concepts, Build, the rest of the Plugins section, or License.

If unrelated changes appear (e.g., trailing whitespace elsewhere, line-ending shifts), STOP and investigate with `git diff --cached -w README.md` before proceeding.

- [ ] **Step 3: Create the commit**

```bash
git commit -m "$(cat <<'EOF'
docs(readme): refresh for engine-first positioning with inline lint demo

Reframe belt as a workflow engine for LLM-driven Agent Skills and
surface the static pre-check value of `belt lint`. Rewrite the Intro,
append a pre-check paragraph to Why, inline a `belt lint` demo after
the Example, add a lint-efficacy paragraph to the CLI section, rename
the Plugins heading to "Claude Code Plugins (Working Examples)", and
replace the phase enumeration in the feature-dev / bug-fix rows with
a pointer to each plugin's pipeline.yml (SSOT).

Spec: docs/superpowers/specs/2026-04-16-readme-refresh-design.md
EOF
)"
```

- [ ] **Step 4: Verify the commit landed**

Run: `git log --oneline -2`
Expected: the new commit at HEAD with the subject `docs(readme): refresh for engine-first positioning with inline lint demo`, followed by `af03c43 docs(specs): add 2026-04-16 README refresh design`.

- [ ] **Step 5: Final working-tree state**

Run: `git status --short README.md`
Expected: empty (README is tracked and clean).

---

## Done Criteria

- `README.md` reflects every retain/delete/new instruction in spec Section 5 verbatim.
- All grep assertions in Tasks 2–7 pass.
- `wc -l README.md` is between 225 and 240.
- `cargo test -p belt-core --test feature_dev_refresh` and `--test bug_fix_refresh` both PASS.
- Exactly one new commit sits atop `af03c43` with the subject above.
- No file outside `README.md` is modified.
