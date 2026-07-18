---
name: requirements
description: >-
  Interview-driven requirements definition. Resolves a Linear ticket,
  URL, or free-text request, investigates the codebase, asks only
  human-decidable questions in dependency-ordered rounds until the
  frontier is empty, and writes
  docs/requirements/<YYYY-MM-DD-topic>/requirements.md reviewed by
  belt:spec-reviewer. The result feeds /belt:feature-dev as input.
user-invocable: true
argument-hint: "<linear-id | url | free-text>"
---

# requirements

Belt pipeline for the requirements stage. Structure, gates, and done
criteria live in `pipeline.yml`; this file defines how to execute each
phase. By default, question rounds continue until nothing is left
silently assumed.

## Phase: interview

This phase has no `invoke` — execute these steps directly:

### 1 — Resolve input

Apply the first matching rule to the argument text:

- Matches `^[A-Z]+-[0-9]+$` → run `linear issue view <id>` and collect
  title, description, comments, and linked URLs.
- Starts with `http` and contains `slack.com` → fetch the thread via
  the slackcli skill.
- Starts with `http` (other) → fetch the page via WebFetch.
- Anything else → treat the text itself as the request.

If the fetched content links to further tickets/PRs, fetch at most 2 of
them (the most directly referenced). Do not crawl deeper.

### 2 — Investigate the codebase

Grep/Read every identifier, module, and feature name in the resolved
input. For unfamiliar areas spanning 10+ files, dispatch
`belt-agent:explorer` subagents in parallel (focus: flow or patterns).
A question answerable here MUST NOT be asked to the user.

### 3 — Frontier interview

Ask the remaining human-decidable points (business goals, scope
boundaries, priorities, non-functional targets) via the frontier
interview per authoring-principles.md §4: map the decisions as a design
tree, each round ask the frontier — the questions whose prerequisites
are settled — in ONE AskUserQuestion call (up to 4, recommended option
first), and defer any question that depends on an answer still open
this round. Points the user explicitly defers are recorded under Open
decisions.

### 4 — Write the document

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

## Config: [requirements] rounds (belt.toml)

    [requirements]
    rounds = 0    # default; 0 = until the frontier is empty

- Absent belt.toml or key → 0 (no cap; nothing left silently assumed).
- `rounds = N` (N ≥ 1) — cap at N rounds; unresolved points default to
  the recommended option and are recorded under Open decisions.

## Phase: review

The invoke is declared in `pipeline.yml`; pass the requirements.md
path as the review target. Complete the review's batched triage.

Hand the file path to `/belt:feature-dev` (or `/belt:goal`) to start
development from it.

## Red flags

- Never ask a question the codebase can answer.
- Never end the interview while frontier questions remain, unless a
  non-zero `[requirements] rounds` cap was hit.
- Never ask a question in the same round as a question it depends on.
- Never write requirements as implementation instructions — state
  outcomes, not designs.
