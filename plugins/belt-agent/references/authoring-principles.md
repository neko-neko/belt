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
reference hop — prefer inlining over linking.

## 4. Batch dialogue

User questions go through AskUserQuestion in batches (up to 4 questions
per round, max 2 rounds). Never ask one question at a time across
multiple turns. Questions answerable by reading the codebase are not
asked at all.

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

Only the orchestrator writes evidence.md — never subagents.
