---
name: requirements
description: >-
  Interview-driven requirements definition. Resolves a Linear ticket,
  URL, or free-text request, investigates the codebase, asks only
  human-decidable questions in one batch, and writes
  docs/requirements/<YYYY-MM-DD-topic>/requirements.md reviewed by
  belt:spec-reviewer. The result feeds /belt:feature-dev as input.
user-invocable: true
argument-hint: "<linear-id | url | free-text>"
---

# requirements

Turn raw input into a reviewed requirements document with at most 2
question rounds. No pipeline.yml — dialogue-centric skills do not run
under belt.

## Step 1 — Resolve input

Apply the first matching rule to the argument text:

- Matches `^[A-Z]+-[0-9]+$` → run `linear issue view <id>` and collect
  title, description, comments, and linked URLs.
- Starts with `http` and contains `slack.com` → fetch the thread via
  the slackcli skill.
- Starts with `http` (other) → fetch the page via WebFetch.
- Anything else → treat the text itself as the request.

If the fetched content links to further tickets/PRs, fetch at most 2 of
them (the most directly referenced). Do not crawl deeper.

## Step 2 — Investigate the codebase

Grep/Read every identifier, module, and feature name in the resolved
input. For unfamiliar areas spanning 10+ files, dispatch
`belt-agent:explorer` subagents in parallel (focus: flow or patterns).
A question answerable here MUST NOT be asked to the user.

## Step 3 — Batched questions

Ask the remaining human-decidable points (business goals, scope
boundaries, priorities, non-functional targets) via AskUserQuestion —
up to 4 questions per round, max 2 rounds. Unresolved points default to
the recommended option and are recorded under Open decisions.

## Step 4 — Write the document

Create `docs/requirements/<YYYY-MM-DD-topic>/requirements.md` (topic
slug rules follow
`plugins/belt/skills/design/references/path-convention.md`) with
exactly these sections, none empty (write "(none)" only under Open
decisions):

    # Requirements: <topic>
    ## Background                  — why now, current pain
    ## Goals                       — measurable outcomes
    ## Functional requirements     — numbered; each verifiable
    ## Non-functional requirements — performance / security / operability targets
    ## Acceptance criteria         — numbered; each verifiable by a command, test, or observable behavior
    ## Out-of-scope                — explicit exclusions
    ## Open decisions              — defaulted choices + known unknowns

## Step 5 — Review

Invoke `/belt:spec-review` with the requirements.md path as the target
and `docs/requirements/<topic>/review/` as the output directory (no
belt run is active), and complete its triage.

Hand the file path to `/belt:feature-dev` (or `/belt:goal`) to start
development from it.

## Red flags

- Never ask a question the codebase can answer.
- Never run more than 2 question rounds.
- Never write requirements as implementation instructions — state
  outcomes, not designs.
