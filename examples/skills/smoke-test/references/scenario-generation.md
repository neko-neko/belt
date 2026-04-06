# Scenario Generation

Generate smoke test scenarios from code diffs, design docs, and optional review perspectives.

## Diff Collection

```
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
| "Test Perspectives" | Scenario ideas organized by category |
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
