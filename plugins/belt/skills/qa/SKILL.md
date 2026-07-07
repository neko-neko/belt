---
name: qa
description: >-
  Runs the QA verification stage: an independent belt:qa-verifier subagent
  replays docs/features/<topic>/scenarios.yml (browser scenarios via
  agent-browser with screenshots, cli scenarios by executing the real
  commands with transcripts), runs an exploratory pass, and writes
  qa-report.md. Evidence goes to the run directory and is published to
  PR/Linear per the [qa] evidence config. Use standalone after a build,
  or composed as the qa stage of /belt:feature-dev and /belt:bug-fix.
user-invocable: true
---

# qa

Belt pipeline for the QA stage. The single phase dispatches the
`belt:qa-verifier` subagent — the orchestrator never replays scenarios
itself, so verification stays independent from the implementation.

## Entry check

Locate the scenario file: `docs/features/<topic>/scenarios.yml` (feature
runs) or `docs/features/<topic>/rca-scenarios.yml` (bug runs). On glob
collision take the most recently modified. If none exists, stop and ask
the user — skipping QA requires their approval, recorded in
qa-report.md (Verdict: SKIPPED + timestamp + reason).

Resolve the evidence directory:

- Active belt run (`belt-agent status` succeeds and at least one phase
  is not COMPLETED or SKIPPED) → read `run_id` from
  status and use the run directory's `qa/` subdirectory (the run
  directory is never committed; `.belt/` is gitignored).
- No active run → `.belt/qa-adhoc/<UTC YYYYMMDD-HHMMSS>/`.

## Phase: qa

1. Dispatch `Task(subagent_type: belt:qa-verifier)` with a
   self-contained prompt containing: the scenario file path, the
   evidence directory path, the report path
   `docs/features/<topic>/qa-report.md`, the change scope (file list
   from `git diff main...HEAD`), and the acceptance criteria copied
   from goal-sheet.md (bug runs: the reproduction condition from
   rca-report.md).
2. Read qa-report.md. For each FAIL: dispatch one
   `belt-agent:implementer` fix subagent (self-contained prompt: the
   failing scenario text, observed vs expected, evidence file paths,
   target files), run the project test suite, commit the fix, then
   re-dispatch belt:qa-verifier for the failed scenario ids only.
   Maximum 2 fix rounds; leftovers go to the user via the validate
   criteria. Code fixed during QA is NOT re-reviewed by
   /belt:code-review (D12) — the fix commits are reported at integrate.
3. Record every QA fix commit hash in evidence.md's qa entry.
4. Publish: if the `[qa] evidence` config is exactly `linear`, attach
   the evidence files to the Linear issue now. Under `auto`, publishing
   is resolved at integrate (PR if one is created; else Linear if an
   issue id is known; else local with a warning). Use the linear cli's
   native file upload; if it does not support uploads, post an issue
   comment with the evidence branch URLs instead (see integrate
   publishing in the orchestrator SKILL.md). The `pr` destination is
   published by integrate, after the PR exists.
5. Append the qa entry to evidence.md.

## Config: [qa] evidence (belt.toml)

    [qa]
    evidence = "auto"    # "pr" | "linear" | "local" | "auto"

- `pr` — publish to the PR comment at integrate.
- `linear` — attach to the Linear issue at the end of this phase.
- `local` — keep evidence in the run directory only; integrate reports
  the local path.
- `auto` (default; also when belt.toml or the key is absent) — PR if
  integrate creates one; else Linear if an issue id is known; else
  local with an explicit warning at integrate.

## Red flags

- Never replay scenarios in the orchestrator context — always through
  belt:qa-verifier.
- Never mark SKIPPED without recorded user approval.
- Never exceed 2 fix rounds silently — surface leftovers to the user.
- Never commit evidence binaries to the repository.

## References

- `../design/references/path-convention.md` — naming rules (SSOT)
- `../plan/SKILL.md` — scenarios.yml schema (writer side)
- `plugins/belt-agent/references/authoring-principles.md` — evidence entry format
