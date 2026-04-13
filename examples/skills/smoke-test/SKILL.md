---
name: smoke-test
description: Browser-based UI verification for code changes. Generates test scenarios from diffs and design docs, executes them via browser, and produces an evidence-backed report.
argument-hint: "[--diff-base <branch>] [--skip-vrt] [--skip-e2e]"
---

# Smoke Test

Browser-based UI verification for code changes. Generates test scenarios from diffs
and design docs, executes them via browser interaction, and produces an evidence-backed
report.

Dispatching and `invoke:` semantics follow `skills/belt-agent/SKILL.md`. This document covers only smoke-test-specific concerns (output artifacts, red flags).

## Output

- `smoke-test-report.md` — structured test report (not committed)
- `smoke-*.png` — browser screenshots (one per scenario minimum)
- Status: PASS / FAIL / PAUSE

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
