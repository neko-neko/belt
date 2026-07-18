---
name: goal
description: >-
  Batched-question intake that turns a Linear ticket, URL, or free-text task
  into a goal sheet (goal / scope / acceptance criteria). Lightweight
  replacement for grilling/brainstorming dialogues: investigates the codebase
  first, then asks only human-decidable questions in dependency-ordered
  rounds (frontier interview). Use standalone before any feature work, or as
  the intake phase of /belt:feature-dev.
user-invocable: true
argument-hint: "<linear-id | url | free-text>"
---

# goal

Turn raw input into a reviewed goal sheet with config-bounded question
rounds (default 2).

## Step 1 — Resolve input

Apply the first matching rule to the argument text:

- Points to an existing local `requirements.md` (a path containing
  `docs/requirements/`) → read it; the goal sheet condenses its Goals /
  Acceptance criteria / Out-of-scope, and its Open decisions become
  Open risks.
- Matches `^[A-Z]+-[0-9]+$` → run `linear issue view <id>` and collect
  title, description, comments, and linked URLs.
- Starts with `http` and contains `slack.com` → fetch the thread via the
  slackcli skill.
- Starts with `http` (other) → fetch the page via WebFetch.
- Anything else → treat the text itself as the task description.

If the fetched content links to further tickets/PRs, fetch at most 2 of
them (the most directly referenced). Do not crawl deeper.

## Step 2 — Investigate the codebase

Grep/Read for every identifier, module, and feature name that appears in
the resolved input. Establish: which files the change will touch, what
existing patterns apply, and which open questions the code already
answers. A question answerable here MUST NOT be asked to the user.

## Step 3 — Frontier interview

Collect the remaining human-decidable points (scope boundaries, UX
choices, priority trade-offs, acceptance thresholds) and run the
frontier interview per authoring-principles.md §4: map the decisions as
a design tree, each round ask the frontier — the questions whose
prerequisites are settled — in ONE AskUserQuestion call (up to 4,
recommended option first), and defer any question that depends on an
answer still open this round. On hitting the round limit, resolve
anything left with the recommended option and record it under Open
risks.

## Config: [goal] rounds (belt.toml)

    [goal]
    rounds = 2    # 0 = keep asking until the frontier is empty

- Absent belt.toml or key → 2.
- `rounds = 0` — no cap; rounds continue until every decision is
  settled or explicitly deferred by the user to Open risks.

## Step 4 — Write the goal sheet

Create `docs/features/<YYYY-MM-DD-topic>/goal-sheet.md` (naming per
`plugins/belt/skills/design/references/path-convention.md`) with exactly
these 5 sections, none empty:

    # Goal sheet: <topic>
    ## Goal            — one paragraph, the outcome in user terms
    ## In-scope        — bullet list of what will be built
    ## Out-of-scope    — bullet list of what will NOT be built
    ## Acceptance criteria — numbered; each verifiable by a command,
                             test, or observable behavior
    ## Open risks      — decisions defaulted in Step 3 + known unknowns

Then create `docs/features/<topic>/evidence.md` (if absent) and append
the intake entry per the Evidence format in
`plugins/belt-agent/references/authoring-principles.md`:

    ## intake — <ISO-8601 UTC>
    - Command: <linear/fetch commands run, or "(free-text input)">
    - Observed: <question rounds used, decisions made>
    - Artifacts: [goal-sheet.md](./goal-sheet.md)

## Red flags

- Never ask a question the codebase can answer.
- Never exceed a non-zero configured `[goal] rounds` cap (0 = no cap;
  default 2).
- Never ask a question in the same round as a question it depends on.
- Never leave a goal-sheet section empty — write "(none)" only in
  Open risks; every other section must have real content.
