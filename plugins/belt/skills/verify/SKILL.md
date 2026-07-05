---
name: verify
description: >-
  Browser-based verification in one pass: replays
  docs/features/<topic>/scenarios.yml via agent-browser, then runs a short
  exploratory pass around the change scope. Writes e2e-report.md. Use
  standalone, or invoked by the e2e phase of /belt:build. Requires
  agent-browser.
user-invocable: true
---

# verify

Single-pass browser verification. No pipeline.yml — this skill runs its
three steps directly.

## Entry check

Locate the scenario file: `docs/features/<topic>/scenarios.yml` (feature
runs) or `docs/features/<topic>/rca-scenarios.yml` (bug runs). On glob
collision take the most recently modified. If none exists, stop and ask
the user — there is nothing to replay.

## Step 1 — Scenario replay

Load the agent-browser skill. For each scenario (Given/When/Then), drive
the browser through the steps and record PASS or FAIL with the observed
behavior. Never silently retry a FAIL — record it first; re-run only
after a fix.

## Step 2 — Exploratory pass

Around the changed screens/flows (file list from
`git diff main...HEAD`), probe beyond the scripted paths: invalid input,
empty states, rapid repeat actions, back/reload navigation. Cap the
exploration at 15 minutes. Record anything anomalous.

## Step 3 — Report

Write `docs/features/<topic>/e2e-report.md`:

    # E2E report: <topic>
    ## Scenario results   — table: scenario id | PASS/FAIL | note
    ## Exploratory notes  — bullet list of probes and observations
    ## Verdict            — PASS (all green) / FAIL (list) / SKIPPED

If no browser-reachable UI exists (CLI/backend-only repo), write the
report with Verdict: SKIPPED and the reason — never fabricate browser
runs.

## Red flags

- Never mark a scenario PASS without driving it in the browser.
- Never write files outside `docs/features/<topic>/`.
