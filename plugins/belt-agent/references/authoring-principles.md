# Authoring Principles (Sonnet-lean)

Prompt-layer rules for belt plugin skills and agents. Written so the
pipeline runs reliably on Sonnet-class models; stronger models simply run
faster. Introduced by docs/specs/2026-07-05-sonnet-lean-pipeline-design.md.

## 1. One phase, one file

Everything needed to execute a phase lives in the owning SKILL.md.
Done criteria live in pipeline.yml as inline `validate:` lists (3-6 items).
Do not create criteria/*.md files or references/*-supplement.md chains.
Exception: genuinely shared single-source-of-truth docs
(path-convention.md, this file).

## 2. No discretionary instructions

Never write "judge", "appropriately", "use LLM judgment", or open-ended
priority tables. Replace with explicit if-then rules that two different
models would execute identically. If a rule needs more than 3 conditions,
it is a design smell — simplify the rule.

## 3. Self-contained subagent prompts

A subagent prompt must contain: the resolved physical paths it reads and
writes, the exact output schema, and the completion condition. Subagents
never call belt-agent, never resolve URIs, and never read sibling agents'
outputs. Repeating a 3-line format across skills is cheaper than a
reference hop — prefer inlining over linking. When one skill invokes
another, pass the target document's path explicitly — never rely on
glob-fallback discovery.

## 4. Batch dialogue (frontier interview)

Map open decisions as a design tree: each decision branches into the
decisions that depend on it. The frontier is every decision whose
prerequisites are already settled. Each round, ask the frontier in ONE
AskUserQuestion call (up to 4 questions, recommended option first). A
question whose answer depends on another question still open in this
round belongs to a later round. If the frontier exceeds 4, ask the 4
with the most dependent decisions; the rest stay in the frontier.

Facts are never questions: anything answerable by reading the codebase
or running a command is looked up (dispatch a subagent for areas
spanning 10+ files), not asked. While a lookup runs, hold back only the
questions downstream of it — ask the rest of the frontier now. A
decision the user explicitly defers counts as settled and is recorded
in the document's open-decisions section.

The round limit is 2 unless the skill declares a `rounds` key in its
`## Config` section; `rounds = 0` means no cap — rounds continue until
the frontier is empty. On hitting a non-zero limit, settle remaining
decisions with the recommended option and record them in the document's
open-decisions section. Never ask one question at a time across
multiple turns.

(Frontier interview model adapted from mattpocock/skills
`batch-grill-me`, MIT.)

## 5. Lines over tables

A lookup table is justified only when there are 4+ rows of homogeneous
data. For 3 or fewer conditions, write if-then bullet lines.

## Evidence entries

Every phase appends one entry to `docs/features/<topic>/evidence.md`
(created by the intake phase). Fixed 3-line format:

    ## <phase-id> — <ISO-8601 UTC>
    - Command: <command(s) actually run, or "(dialogue)" / "(authoring)">
    - Observed: <exit code / counts / PASS-FAIL summary>
    - Artifacts: <relative links to files this phase produced>

Use the bare leaf name (e.g. `intake`, `execute`) as `<phase-id>`, not
the namespaced run id (`design/intake`).

Only the orchestrator writes evidence.md — never subagents.

## QA evidence

QA evidence binaries (screenshots, transcripts) live under the run
directory's `qa/` subdirectory — never under `docs/` and never
committed. `docs/features/<topic>/qa-report.md` (text) is the committed
index; it references evidence by run-relative path. Publishing to
PR/Linear is governed by the `[qa] evidence` key in belt.toml
(interpretation rules: `plugins/belt/skills/qa/SKILL.md`).
