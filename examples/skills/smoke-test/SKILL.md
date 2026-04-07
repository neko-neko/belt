---
name: smoke-test
description: Browser-based UI verification for code changes. Generates test scenarios from diffs and design docs, executes them via browser, and produces an evidence-backed report.
argument-hint: "[--diff-base <branch>] [--skip-vrt] [--skip-e2e]"
---

# Smoke Test

Browser-based UI verification for code changes. Generates test scenarios from diffs
and design docs, executes them via browser interaction, and produces an evidence-backed
report.

This skill is used with the `pipelines/smoke-test.yml` belt pipeline.
Invoke /belt-agent for protocol details.

## Output

- `smoke-test-report.md` — structured test report (not committed)
- `smoke-*.png` — browser screenshots (one per scenario minimum)
- Status: PASS / FAIL / PAUSE

## Phase: env-setup

1. If `args.server` and `args.port` are set, use them directly.
2. Otherwise, read [server-detection.md](references/server-detection.md) and auto-detect.
3. Start the server in the background.
4. Wait for the server to respond (timeout: 30 seconds).
5. If timeout → report PAUSE status.

## Phase: adhoc-test

1. Collect diff: `git diff <args.diff_base>...HEAD`
2. Read [scenario-generation.md](references/scenario-generation.md) to generate scenarios:
   - Start with 5 base perspectives.
   - If `args.design` is set, expand from design doc.
   - If `args.perspectives` is set, dispatch review agents for additional perspectives.
3. Execute each scenario via browser (reconnaissance-then-action pattern).
4. Take a screenshot after each scenario: `smoke-<scenario_name>.png`
5. On scenario failure, retry up to 2 times before marking FAIL.
6. Write report per [report-template.md](references/report-template.md).

## Phase: vrt-check

1. Read [vrt-detection.md](references/vrt-detection.md) to detect VRT tooling.
2. If no VRT tooling detected → skip this phase (no action needed).
3. Run VRT command.
4. If diffs found → present diff images to user for review.
   - User approves → update baseline and commit.
   - User rejects → record in report only.

## Phase: e2e-detection

1. Read [e2e-flaky-detection.md](references/e2e-flaky-detection.md) to detect E2E suite.
2. If no E2E suite detected → skip this phase (no action needed).
3. Determine test scope: `args.full_e2e` → all tests, otherwise changed-files-only.
4. Execute tests twice (2-pass flaky detection).
5. Classify results: stable pass / implementation failure / flaky.
6. Flaky tests → PASS with report note (do not block).
7. Implementation failures → FAIL with fix suggestions.

## Red Flags

**Never:**
- Mark a failing test as PASS silently
- Update VRT baselines without explicit user approval
- Classify flaky tests as implementation failures
- Simplify or skip steps due to environment issues (report PAUSE instead)

**Always:**
- Take at least one screenshot per scenario
- Clean up server processes when done
- Include adversarial probe results in report
