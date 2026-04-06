# E2E + Flaky Detection

Detect E2E test suites, run them twice, and classify results.

## Suite Detection Table

Check in order. Use the first match.

| Tool | Condition | Command |
|------|-----------|---------|
| Playwright | `playwright.config.*` exists | `npx playwright test` |
| Cypress | `cypress.config.*` exists | `npx cypress run` |
| Other | `scripts.test:e2e` in package.json | `npm run test:e2e` |

If no E2E suite detected → skip this phase entirely. No action needed.

## Test Scope

| Condition | Scope |
|-----------|-------|
| `args.full_e2e` is true | Run all test files |
| Default | Run only tests related to changed files |

### Finding related tests (default scope)

1. Get changed files: `git diff <args.diff_base>...HEAD --name-only`
2. Find test files among changes: `*.spec.ts`, `*.e2e.ts`, `*.test.ts`
3. For changed source files, search for corresponding test files.
4. If no related tests found → run full suite once, re-run only failures.

## 2-Pass Flaky Detection

Run the test suite twice. Classify each test:

| Run 1 | Run 2 | Classification | Action |
|-------|-------|---------------|--------|
| PASS | PASS | Stable pass | No action |
| FAIL | FAIL | Implementation failure | FAIL. Generate fix suggestion. |
| PASS | FAIL | Flaky | Report only. Do NOT block. |
| FAIL | PASS | Flaky | Report only. Do NOT block. |

### Flaky test reporting

For each flaky test, include:
- Test name and file path
- Error message and stack trace
- Suspected cause (one of):
  - **Timing dependency**: race conditions, animation waits, async operations
  - **External dependency**: network calls, third-party services
  - **Nondeterministic data**: random values, timestamps, UUIDs
  - **DOM state dependency**: element visibility, rendering timing
- Suggested fix based on suspected cause

### Implementation failure reporting

For each implementation failure (FAIL/FAIL), include:
- Test name and file path
- Error message from both runs
- Suggested fix based on error analysis
