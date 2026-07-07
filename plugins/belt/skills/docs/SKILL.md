---
name: docs
description: >-
  General-purpose documentation writing. Takes a topic (feature, module,
  or theme), investigates the code, and creates or updates documents
  under docs/ following the repository's existing documentation
  conventions. Use for architecture overviews, usage guides, and
  reference pages.
user-invocable: true
argument-hint: "<topic or target path>"
---

# docs

Write documentation that matches the target repository, not a template.
No pipeline.yml — dialogue-centric skills do not run under belt.

## Step 1 — Survey

- Read the existing `docs/` tree: language, tone, heading style, index
  files, directory taxonomy.
- Investigate the code the topic covers with Grep/Read. For unfamiliar
  areas spanning 10+ files, dispatch `belt-agent:explorer` subagents in
  parallel (focus: flow or patterns).

## Step 2 — Placement decision

Determine the target path and document type (architecture overview /
usage guide / reference) from the existing taxonomy. If either is still
ambiguous after the survey, ask once via AskUserQuestion (one batch, up
to 4 questions) — placement, type, audience, depth.

## Step 3 — Write

- Follow the repository's documentation language and style; when the
  repo has no stated policy, write in English.
- Verify every code statement in the document (file paths, command
  names, config keys, API names) against the actual source — never
  write from memory.
- Update cross-links: link related existing documents, and add the new
  document to the index (README or docs index) when one exists.

## Step 4 — Verify

- Check that every path referenced by the touched documents exists.
- Show the user the diff summary: files created/updated + one-line
  purpose each.

## Red flags

- Never restate what a referenced document already says — link it.
- Never document APIs or commands without checking they exist in the
  code.
- Never invent a new docs/ subdirectory when an existing one fits.
