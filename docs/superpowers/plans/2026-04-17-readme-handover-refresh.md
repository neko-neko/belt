# README Handover/Resume Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refresh `README.md` to surface `/belt:handover` and `/belt:resume` by restructuring the Continuity section into cold-start principles + an intra-run / cross-run resumption comparison, and by syncing the Plugins table and Usage block.

**Architecture:** Three localized edits to `README.md` only. No code, SKILL.md, pipeline.yml, or marketplace.json touches. Each task is an independent Edit + grep-verify + commit cycle. Task 4 is spec-verification only (no file changes).

**Tech Stack:** Markdown, plus `rg` / `git diff` for verification.

---

## File Structure

- Modify: `README.md` (three distinct regions only)
  - Continuity section (current lines ~126-157): full rewrite into 2-layer structure
  - Plugins table `belt` plugin row (current line ~239): append two skill names
  - Usage code block (current lines ~284-288): split into two labelled blocks

No new files. No other files touched.

## Spec reference

- `docs/superpowers/specs/2026-04-17-readme-handover-refresh-design.md`

Read this spec before starting. The "Target sections" section (§4) shows the exact final wording; the "Verification" section (§5) shows the acceptance checklist used in Task 4.

---

## Task 1: Rewrite the Continuity section

**Files:**
- Modify: `README.md` (replace the entire `## Continuity` section, currently lines 126-157)

- [ ] **Step 1: Confirm the Continuity section matches the expected baseline**

Run: `rg -n '^## Continuity$' README.md`
Expected: exactly 1 match at line 126.

Run: `rg -n '^This is ' README.md`
Expected: exactly 1 match at line 157 — the current closer line that starts with `This is` (it ends with `keeps the conclusions.`).

If either check fails, stop — the README has drifted from the spec baseline.

- [ ] **Step 2: Replace the entire Continuity section**

Use `Edit` with `replace_all=false`:

- `old_string`: everything from `## Continuity` through `This is \`/clear\` that keeps the conclusions.` (inclusive), i.e. the full current section.
- `new_string` (outer fence is four backticks so the nested three-backtick blocks inside render correctly):

````markdown
## Continuity

Long LLM sessions accumulate context that pollutes reasoning. Even with
summary compaction, prior failed attempts bias the next try. Sometimes you
want a fresh agent that has seen only what matters — not a long-memory one
that has seen everything.

### Cold-start principles

belt is built on two cold-start guarantees:

- **Per-command neutrality** — every `belt-agent` call works from a cold
  start. No conversation history is required; the run state on disk is the
  source of truth.
- **Narrative artifacts** — phase outputs are deterministic files protected
  by gates, not LLM memory. A new session reads them; it does not
  reconstruct them.

### Two resumption modes

On top of those principles, belt offers two complementary ways to continue
work without carrying a polluted context:

|                 | Intra-run handover                           | Cross-run inheritance                   |
|-----------------|----------------------------------------------|-----------------------------------------|
| When            | Same run, new session                        | New run, reads prior artifacts          |
| What is carried | Resume hint + existing state.json            | Gated artifacts via `belt://` URIs      |
| Command         | `/belt:handover` → `/clear` → `/belt:resume` | `belt-agent init --inherits-from <run>` |
| Typical use     | Context bloat mid-pipeline                   | Fresh run consumes prior conclusions    |

**Intra-run handover.** When a pipeline run is mid-flight and the session's
context has grown polluted, `/belt:handover` writes a short Resume hint
(pause reason, first action, transient context) under the current run
directory. After `/clear`, `/belt:resume` reads the hint and `state.json`
and the next session picks up exactly where it left off:

```
/belt:handover
/clear
/belt:resume
```

The pipeline is never re-initialized; the resumed session continues the
current phase with a fresh context but the same run.

**Cross-run inheritance.** `belt-agent init --inherits-from <run_id>` lets
a new run consume a prior run's artifacts via `belt://` URIs:

- `belt://latest/<pipeline>/<path>` — most recent COMPLETED run on the
  current branch
- `belt://workspace/<branch>/latest/<pipeline>/<path>` — branch-scoped
  variant
- `belt://run/<run_id>/<path>` — explicit run reference

A typical use case: a long bug investigation produces `rca.md` and stops.
A fresh agent later picks up the conclusions without inheriting the
original trial-and-error trace:

```
belt-agent init bug-fix.yml --inherits-from <prior-run-id>
```

Both are `/clear` that keeps what matters —
handover keeps the run, inheritance keeps the conclusions.
````

- [ ] **Step 3: Verify structural markers exist**

Run: `rg -n '^### Cold-start principles$' README.md`
Expected: exactly 1 match.

Run: `rg -n '^### Two resumption modes$' README.md`
Expected: exactly 1 match.

Run: `rg -n '^\| When ' README.md`
Expected: exactly 1 match (the comparison table's "When" row).

Run: `rg -n '\*\*Intra-run handover\.\*\*' README.md`
Expected: exactly 1 match.

Run: `rg -n '\*\*Cross-run inheritance\.\*\*' README.md`
Expected: exactly 1 match.

Run: `rg -n 'handover keeps the run, inheritance keeps the conclusions' README.md`
Expected: exactly 1 match.

- [ ] **Step 4: Verify the superseded closing line is gone**

Run: `rg -n '^This is ' README.md`
Expected: 0 matches. (The new closer starts with `Both are`, not `This is`. Do not use the substring `keeps the conclusions` for this check — the new closer also contains that phrase.)

- [ ] **Step 5: Verify no file-path detail leaked in**

Run: `rg -n '\.belt/runs/' README.md`
Expected: 0 matches. (The spec routes that detail to SKILL.md.)

- [ ] **Step 6: Commit**

```bash
git add README.md
git commit -m "docs(readme): restructure Continuity into cold-start + two resumption modes

Split the Continuity section into two subsections: cold-start
principles (per-command neutrality + narrative artifacts) and two
resumption modes (intra-run handover via /belt:handover + /belt:resume,
and cross-run inheritance via belt-agent init --inherits-from). Adds a
comparison table and a three-step handover workflow snippet."
```

---

## Task 2: Append handover/resume to the Plugins table `belt` row

**Files:**
- Modify: `README.md` (line ~239, the `belt` plugin row in the Plugins table)

- [ ] **Step 1: Confirm the row's current tail**

Run: `rg -n '/belt:test-scenarios.*Requires .belt-agent' README.md`
Expected: exactly 1 match on the `belt` plugin row. (The `.` sidesteps the surrounding literal backticks.)

- [ ] **Step 2: Append the two new skills to the end of the skill list**

Use `Edit` with `replace_all=false`:

- `old_string`: `` `/belt:monkey-test`, `/belt:test-scenarios`. Requires `belt-agent` ``
- `new_string`: `` `/belt:monkey-test`, `/belt:test-scenarios`, `/belt:handover`, `/belt:resume`. Requires `belt-agent` ``

Nothing else on the row changes — `(4 observation reviewers)` and `(3 observation reviewers)` stay as-is per the spec.

- [ ] **Step 3: Verify the new entries sit on the Plugins row**

Run: `rg -n '/belt:handover.*/belt:resume.*Requires .belt-agent' README.md`
Expected: exactly 1 match (the Plugins row).

- [ ] **Step 4: Verify the untouched content is still present on the same row**

Run: `rg -n '\(4 observation reviewers\)' README.md`
Expected: exactly 1 match.

Run: `rg -n '\(3 observation reviewers\)' README.md`
Expected: exactly 1 match.

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs(readme): list /belt:handover and /belt:resume in Plugins table

Append the two handover/resume skills to the belt plugin row so the
README matches marketplace.json's skill list."
```

---

## Task 3: Split Usage into Start / Pause-and-resume blocks

**Files:**
- Modify: `README.md` (lines ~284-288, the code block that follows `After install:`)

- [ ] **Step 1: Confirm the current Usage block shape**

Run: `rg -n '# start a new feature' README.md`
Expected: exactly 1 match on the `/belt:feature-dev` line.

Run: `rg -n '# standalone spec review' README.md`
Expected: exactly 1 match on the `/belt:spec-review` line.

- [ ] **Step 2: Replace the four-line block with the two-block version**

Use `Edit` with `replace_all=false`:

- `old_string`:

```
/belt:feature-dev             # start a new feature
/belt:bug-fix                 # start a bug investigation
/belt:code-review             # standalone code review
/belt:spec-review             # standalone spec review
```

- `new_string`:

```
# Start a pipeline
/belt:feature-dev
/belt:bug-fix
/belt:code-review
/belt:spec-review

# Pause & resume an in-progress run
/belt:handover
/belt:resume
```

- [ ] **Step 3: Verify the new block headers exist**

Run: `rg -n '^# Start a pipeline$' README.md`
Expected: exactly 1 match.

Run: `rg -n '^# Pause & resume an in-progress run$' README.md`
Expected: exactly 1 match.

- [ ] **Step 4: Verify the old inline comments are gone**

Run: `rg -n '# start a new feature|# start a bug investigation|# standalone code review|# standalone spec review' README.md`
Expected: 0 matches.

- [ ] **Step 5: Verify handover/resume appear as standalone lines in both the Continuity 3-step block and the new Usage sub-block**

Run: `rg -nc '^/belt:handover$' README.md`
Expected: exactly 2 matches — one inside the Continuity section's three-step handover code block (added by Task 1), one inside the new Usage Pause-and-resume sub-block (added by this task).

Run: `rg -nc '^/belt:resume$' README.md`
Expected: exactly 2 matches — same two locations as above.

- [ ] **Step 6: Commit**

```bash
git add README.md
git commit -m "docs(readme): split Usage into Start and Pause-and-resume blocks

Separate pipeline-start skills (feature-dev/bug-fix/code-review/spec-review)
from the mid-run pause-and-resume skills (handover/resume) so readers see
which skills are invoked at run start vs. during an in-progress run."
```

---

## Task 4: Spec verification (no file changes)

**Files:**
- None. This task verifies the three prior tasks against the spec's §5 Verification checklist and confirms no out-of-scope edits happened.

- [ ] **Step 1: Run the spec's acceptance checks**

Run: `rg -n '^### Cold-start principles$|^### Two resumption modes$' README.md`
Expected: two matches, in that order (Cold-start before Two resumption).

Run: `rg -n '^\| When |^\| What is carried |^\| Command |^\| Typical use' README.md`
Expected: 4 matches — the four data rows of the comparison table, in order.

Run: `rg -n '/belt:handover. → ./clear. → ./belt:resume' README.md`
Expected: at least 1 match (the `Command` cell of the table).

Run: `rg -n '^/belt:handover$' README.md`
Expected: exactly 2 matches — one in the Continuity section's three-step code block (from Task 1), one in the Usage Pause-and-resume sub-block (from Task 3).

Run: `rg -n '^/clear$' README.md`
Expected: exactly 1 match (line 2 of the Intra-run handover three-step code block).

Run: `rg -n '^/belt:resume$' README.md`
Expected: exactly 2 matches — same two locations as above (Continuity three-step block + Usage Pause-and-resume sub-block).

Run: `rg -n '/belt:handover.*/belt:resume.*Requires .belt-agent' README.md`
Expected: exactly 1 match (the Plugins table row).

Run: `rg -n 'handover keeps the run, inheritance keeps the conclusions' README.md`
Expected: exactly 1 match.

Run: `rg -n '\.belt/runs/' README.md`
Expected: 0 matches.

- [ ] **Step 2: Confirm every edit sits inside the three permitted regions**

Run: `git log --oneline -4`
Expected: the top three entries are Task 1-3 commits (Continuity / Plugins / Usage), and the fourth is `docs(specs): add README handover/resume refresh design`.

Run: `git diff HEAD~3 HEAD -- README.md | rg -n '^@@'`
Expected: hunks fall only inside the three target regions (Continuity, Plugins `belt` row, Usage code block). No hunks touch `## Why belt?`, the Example block, `## CLI`, the `## Install` subsections, `## Key Concepts`, or the External skill dependencies table.

Run: `git diff HEAD~3 HEAD -- README.md | wc -l`
Expected: non-zero and bounded — a back-of-envelope sanity check that the diff is localized, not a full-file churn. (If > 400 lines, stop and inspect; it likely means the Continuity rewrite pulled in unintended surroundings.)

Run: `git status`
Expected: `working tree clean`.

- [ ] **Step 3: No commit**

This task is verification only.

---

## Notes on out-of-scope work

The following are explicitly **not** part of this plan (see spec §2 Non-goals and §6 Out-of-scope follow-ups):

- `CHANGELOG.md` entry for handover/resume (separate decision)
- `marketplace.json` `(X reviewers)` ↔ README `(X observation reviewers)` unification (separate PR)
- `plugins/README.md` creation
- Japanese-language README
- `## Why belt?` changes that surface handover/resume as a headline value prop

If a reviewer asks for any of these, surface the spec link and defer.
