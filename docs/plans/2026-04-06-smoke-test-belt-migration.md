# smoke-test Belt Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 既存の `/smoke-test` スキルを belt pipeline YAML + スキル（SKILL.md + 5 reference files）の2層構造に移植する。

**Architecture:** `pipelines/smoke-test.yml` が4フェーズのフラットパイプラインを定義し、`skills/smoke-test/SKILL.md` がフロー概要と Red Flags を、`skills/smoke-test/references/` 配下の5ファイルがリファレンスデータを提供する。全ファイル英語。

**Tech Stack:** YAML (belt pipeline), Markdown (skill files)

**Spec:** [docs/specs/2026-04-06-smoke-test-belt-migration.md](../specs/2026-04-06-smoke-test-belt-migration.md)

---

## File Structure

```
pipelines/
└── smoke-test.yml                           # belt pipeline (4 phases, 9 args)
skills/
└── smoke-test/
    ├── SKILL.md                             # flow + rules + phase map
    └── references/
        ├── server-detection.md              # server auto-detection table
        ├── scenario-generation.md           # 5 perspectives + expansion + adversarial
        ├── report-template.md               # smoke-test-report.md template
        ├── vrt-detection.md                 # VRT tool detection + diff handling
        └── e2e-flaky-detection.md           # E2E detection + 2-pass flaky matrix
```

---

### Task 1: Create `pipelines/smoke-test.yml`

**Files:**
- Create: `pipelines/smoke-test.yml`

- [ ] **Step 1: Create pipeline YAML**

```yaml
name: smoke-test
version: 1
args:
  diff_base:    { type: string, default: "HEAD~1" }
  design:       { type: string, default: "" }
  server:       { type: string, default: "" }
  port:         { type: number, default: 0 }
  skip_vrt:     { type: bool, default: false }
  skip_e2e:     { type: bool, default: false }
  adhoc_only:   { type: bool, default: false }
  full_e2e:     { type: bool, default: false }
  perspectives: { type: string, default: "" }

phases:
  - id: env-setup
    description: "Start dev server and verify it is accessible."
    config:
      skill: "/smoke-test"
    gate:
      - cmd: "curl -sf http://localhost:${args.port:-3000}/ > /dev/null"

  - id: adhoc-test
    description: "Generate and execute ad-hoc smoke test scenarios via browser."
    config:
      skill: "/smoke-test"
    artifacts:
      - "smoke-test-report.md"
    gate:
      - file_exists: "smoke-test-report.md"
      - file_exists: "smoke-*.png"
    validate:
      - "At least one adversarial probe executed and documented in report"
      - "Test scenarios cover changes from diff (not just generic checks)"
    confirm: true

  - id: vrt-check
    description: "Run VRT diff check if VRT tooling is detected."
    when: "!args.skip_vrt"
    config:
      skill: "/smoke-test"

  - id: e2e-detection
    description: "Run E2E test suite with flaky detection (2-pass execution)."
    when: "!args.skip_e2e"
    config:
      skill: "/smoke-test"
```

- [ ] **Step 2: Validate with belt lint**

Run:
```bash
./target/debug/belt lint pipelines/smoke-test.yml
```

Expected: lint passes with no errors.

- [ ] **Step 3: Validate with belt-agent init**

Run:
```bash
./target/debug/belt-agent init pipelines/smoke-test.yml 2>/dev/null | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(f'pipeline: {d[\"pipeline\"]}')
print(f'first phase: {d[\"phase\"][\"id\"]}')
print(f'args count: {len(d[\"args\"])}')
gate_count = len(d.get('gate', []))
print(f'gate count: {gate_count}')
print('PASS' if d['phase']['id'] == 'env-setup' and gate_count == 1 else 'FAIL')
"
```

Expected: `PASS` with pipeline=smoke-test, first phase=env-setup, 1 gate (curl cmd).

- [ ] **Step 4: Validate skip_vrt/skip_e2e conditional phases**

Run:
```bash
./target/debug/belt-agent init pipelines/smoke-test.yml --arg skip_vrt=true --arg skip_e2e=true 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin)['run_id'])" > /tmp/belt_run_id
RUN_ID=$(cat /tmp/belt_run_id)
# advance through all phases
./target/debug/belt-agent step --run "$RUN_ID" 2>/dev/null > /dev/null  # env-setup -> adhoc-test
./target/debug/belt-agent step --confirm --run "$RUN_ID" 2>/dev/null | python3 -c "
import sys, json
d = json.load(sys.stdin)
completed = d.get('completed', False)
print(f'completed: {completed}')
print('PASS' if completed else f'FAIL - to: {d.get(\"to\")}')
"
```

Expected: `PASS` — after env-setup and adhoc-test, pipeline completes (vrt-check and e2e-detection skipped).

- [ ] **Step 5: Cleanup test runs**

Run:
```bash
rm -rf .belt/runs/
```

---

### Task 2: Create `skills/smoke-test/SKILL.md`

**Files:**
- Create: `skills/smoke-test/SKILL.md`

- [ ] **Step 1: Create SKILL.md**

```markdown
# Smoke Test

Browser-based UI verification for code changes. Generates test scenarios from diffs
and design docs, executes them via browser interaction, and produces an evidence-backed
report.

This skill is used with the `pipelines/smoke-test.yml` belt pipeline. It follows the
[Belt Protocol](../belt-agent/SKILL.md) for pipeline driving.

## Output

- `smoke-test-report.md` — structured test report (not committed)
- `smoke-*.png` — browser screenshots (one per scenario minimum)
- Status: PASS / FAIL / PAUSE

## Phase Map

| Phase | What to do | Reference |
|-------|-----------|-----------|
| env-setup | Start dev server, verify accessible | [server-detection.md](references/server-detection.md) |
| adhoc-test | Generate & execute smoke scenarios | [scenario-generation.md](references/scenario-generation.md), [report-template.md](references/report-template.md) |
| vrt-check | Run VRT if tooling detected | [vrt-detection.md](references/vrt-detection.md) |
| e2e-detection | Run E2E with flaky detection | [e2e-flaky-detection.md](references/e2e-flaky-detection.md) |

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
```

- [ ] **Step 2: Verify file created correctly**

Run: `head -5 skills/smoke-test/SKILL.md`
Expected:
```
# Smoke Test

Browser-based UI verification for code changes. Generates test scenarios from diffs
and design docs, executes them via browser interaction, and produces an evidence-backed
report.
```

---

### Task 3: Create `skills/smoke-test/references/server-detection.md`

**Files:**
- Create: `skills/smoke-test/references/server-detection.md`

- [ ] **Step 1: Create server-detection.md**

```markdown
# Server Detection

Auto-detect and start a dev server for smoke testing.

## Override

If `args.server` and `args.port` are provided, skip detection and use them directly:

```bash
<args.server>   # e.g., "npm run dev"
# Then verify: curl -sf http://localhost:<args.port>/ > /dev/null
```

## Auto-Detection Table

Check in order. Use the first match.

| Check | Condition | Command | Default Port |
|-------|-----------|---------|-------------|
| package.json | `scripts.dev` exists | `npm run dev` | 3000 or 5173 (Vite) |
| package.json | `scripts.start` exists (no `dev`) | `npm start` | 3000 |
| Makefile | `dev` target exists | `make dev` | 8080 |
| Makefile | `serve` target exists (no `dev`) | `make serve` | 8080 |
| manage.py | file exists | `python manage.py runserver` | 8000 |
| docker-compose.yml | file exists | `docker compose up` | from `ports` mapping |

### How to check

- **package.json**: Read the file, check `scripts` object for key existence.
- **Makefile**: `grep -q '^dev:' Makefile` or `grep -q '^serve:' Makefile`
- **Vite detection**: If `vite` appears in `devDependencies`, default port is 5173 instead of 3000.

## Startup

1. Run detected command in background (`run_in_background: true`).
2. Poll `curl -sf http://localhost:<port>/` every 2 seconds.
3. Timeout after 30 seconds → PAUSE with message:
   "Server did not respond within 30 seconds. Check the startup command and port."

## No Detection

If no match found in the detection table:
1. Ask user: "No dev server detected. What command starts the server, and on what port?"
2. If no response → PAUSE.
```

---

### Task 4: Create `skills/smoke-test/references/scenario-generation.md`

**Files:**
- Create: `skills/smoke-test/references/scenario-generation.md`

- [ ] **Step 1: Create scenario-generation.md**

```markdown
# Scenario Generation

Generate smoke test scenarios from code diffs, design docs, and optional review perspectives.

## Diff Collection

```bash
git diff <args.diff_base>...HEAD
```

Default `args.diff_base` is `HEAD~1`. The diff determines which areas of the application
to focus scenarios on.

## Base Perspectives (always applied)

Every smoke test run includes these 5 perspectives:

| # | Perspective | What to verify |
|---|-----------|----------------|
| 1 | Navigation | Pages load, routes work, no dead links in changed areas |
| 2 | User interaction | Clicks, inputs, form submissions work as expected |
| 3 | Error-free | No console errors, no failed network requests |
| 4 | Responsive | Desktop (1280x720) and mobile (375x667) both render correctly |
| 5 | Impact | Reverse dependencies and side effects from changed code |

## Perspective Expansion

### From design doc (`args.design`)

When a design document path is provided, extract additional test perspectives from:

| Section to find | What to extract |
|----------------|-----------------|
| "Test Perspectives" or "テスト観点" | Scenario ideas organized by category |
| "Must-Verify Checklist" (in Investigation Record) | Required verification items → mandatory scenarios |
| "Impact Analysis" (Reverse Dependencies / Side Effect Risks) | Impact-based scenarios |

### From review agents (`args.perspectives`)

Comma-separated list of perspective types. Dispatch the corresponding review agent
in parallel, each with the diff as input:

| Perspective | Agent type | Collects |
|------------|-----------|----------|
| security | code-review-security | XSS, CSRF, auth bypass, injection vectors |
| performance | code-review-performance | Render speed, large data display, N+1 queries |
| coverage | test-review-coverage | Boundary values, error paths, state transitions |

**Agent prompt structure:**

> Given the following diff, list smoke test items from a {perspective} viewpoint.
> Focus only on items verifiable through browser interaction (not code-level concerns).
>
> [diff content]
>
> Output: List of test items (name, what to verify, priority: high/medium)

### When both are specified

1. Start with design doc perspectives.
2. Add agent perspectives that are not already covered.
3. On overlap, keep the design doc version.

## Adversarial Probes

**At least one adversarial probe is required per run.** Choose from:

| Probe | What to do |
|-------|-----------|
| Empty input | Submit forms with empty/blank fields |
| Invalid input | Enter unexpected data types or extreme values |
| Idempotency | Repeat the same action twice rapidly |
| Nonexistent target | Navigate to a URL or interact with an element that shouldn't exist |
| State persistence | Perform an action, refresh the page, verify state survived |
| Reverse navigation | Complete a flow, then navigate backward through it |

## Execution Pattern

For each scenario, follow the reconnaissance-then-action pattern:

1. **Navigate** to the target page.
2. **Reconnaissance**: Read page state (elements, content, console) before acting.
3. **Act**: Perform the test action (click, type, submit).
4. **Observe**: Check the result (page state, console, network).
5. **Screenshot**: Save as `smoke-<scenario_name>.png`.
6. **Verdict**: PASS or FAIL based on observation.

On failure, retry up to 2 times with scenario adjustments before marking FAIL.
```

---

### Task 5: Create `skills/smoke-test/references/report-template.md`

**Files:**
- Create: `skills/smoke-test/references/report-template.md`

- [ ] **Step 1: Create report-template.md**

````markdown
# Report Template

Write the smoke test report to `smoke-test-report.md` in the working directory.
This file is not committed to git.

## Template

```markdown
# Smoke Test Report

**Date:** YYYY-MM-DD HH:MM
**Diff Base:** <branch or HEAD~1>
**Server:** <command> (port: <N>)
**Status:** PASS / FAIL / PAUSE

## Ad-hoc Smoke Test

| Scenario | Perspective | Result | Screenshot |
|----------|------------|--------|------------|
| <name> | <perspective> | PASS/FAIL | smoke-<name>.png |

### Evidence Log

#### Check: <scenario name>
- **Action:** <what was done (browser navigation, click, input, etc.)>
- **Observed:** <what was seen (page state, console output, network response)>
- **Result:** PASS / FAIL

## VRT Diff Check

<PASS / SKIP / DIFF_DETECTED (with details)>

## E2E + Flaky Detection

### Test Results

| Test | Run 1 | Run 2 | Verdict |
|------|-------|-------|---------|
| <test name> | PASS/FAIL | PASS/FAIL | stable / implementation failure / flaky |

### Flaky Tests

| Test | Suspected Cause | Suggested Fix |
|------|----------------|---------------|
| <test> | timing / external dependency / nondeterministic data / DOM state | <suggestion> |

### Implementation Failures

| Test | Error | Suggested Fix |
|------|-------|---------------|
| <test> | <error message> | <suggestion> |
```

## Status Determination

| Condition | Status |
|-----------|--------|
| All steps PASS (flaky tolerated) | PASS |
| adhoc-test scenario fails after 2 retries | FAIL |
| Adversarial probe not executed | FAIL |
| E2E implementation failure (both runs FAIL) | FAIL |
| Server could not start | PAUSE |
| Only flaky tests detected (rest PASS) | PASS |
````

---

### Task 6: Create `skills/smoke-test/references/vrt-detection.md`

**Files:**
- Create: `skills/smoke-test/references/vrt-detection.md`

- [ ] **Step 1: Create vrt-detection.md**

```markdown
# VRT Detection

Detect Visual Regression Testing tooling and run checks.

## Detection Table

Check in order. Use the first match.

| Tool | Condition | Command |
|------|-----------|---------|
| Playwright snapshots | `playwright.config.*` exists AND uses `toMatchSnapshot` | `npx playwright test --grep snapshot` |
| reg-suit | `.reg/` directory or `regconfig.json` exists | `npx reg-suit run` |
| Storycap + reg-suit | `storycap` in devDependencies | `npx storycap && npx reg-suit run` |
| Loki | `loki` in devDependencies or `.lokirc` exists | `npx loki test` |
| Percy | `@percy/cli` in devDependencies | Skip (CI-only tool) |

If no VRT tooling detected → skip this phase entirely. No action needed.

## Diff Handling

When VRT detects visual differences:

1. Identify which diff images were produced (tool-specific output directory).
2. Present diff images to the user using the Read tool.
3. Ask user for decision:
   - **Approve**: Update baseline images and commit:
     ```bash
     git add <baseline-directory>
     git commit -m "test: update VRT baselines"
     ```
   - **Reject**: Record in report only. Do not update baselines.
4. Record the outcome in the VRT section of the smoke test report.
```

---

### Task 7: Create `skills/smoke-test/references/e2e-flaky-detection.md`

**Files:**
- Create: `skills/smoke-test/references/e2e-flaky-detection.md`

- [ ] **Step 1: Create e2e-flaky-detection.md**

```markdown
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
```

---

### Task 8: Commit

**Files:**
- Stage: all files in `pipelines/` and `skills/smoke-test/`

- [ ] **Step 1: Commit all files**

Run:
```bash
git add pipelines/smoke-test.yml skills/smoke-test/
git commit -m "feat: add smoke-test belt pipeline and skill

Migrates /smoke-test to belt's 2-layer architecture:
- pipelines/smoke-test.yml: 4-phase pipeline (env-setup, adhoc-test, vrt-check, e2e-detection)
- skills/smoke-test/SKILL.md: flow + rules + phase map
- skills/smoke-test/references/: 5 reference docs (server-detection, scenario-generation,
  report-template, vrt-detection, e2e-flaky-detection)"
```

---

### Task 9: Cleanup

- [ ] **Step 1: Remove test runs created during validation**

Run:
```bash
rm -rf .belt/runs/
```
