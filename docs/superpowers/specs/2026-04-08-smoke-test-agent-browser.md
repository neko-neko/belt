# smoke-test: adhoc-test browser tool specification

## Problem

The smoke-test example's adhoc-test phase says "Execute each scenario via browser" without specifying which browser tool to use. Agents default to Playwright MCP tools, which is not the intended behavior.

## Decision

Specify `/agent-browser` as the browser automation tool in `adhoc-test-procedure.md`.

## Scope

- **In scope**: `examples/skills/smoke-test/references/adhoc-test-procedure.md` step 3 only
- **Out of scope**: vrt-check, e2e-detection (existing test framework detection), env-setup, gate definitions, screenshot naming

## Change

Step 3 of adhoc-test-procedure.md:

**Before:**
```
3. Execute each scenario via browser (reconnaissance-then-action pattern).
```

**After:**
```
3. Invoke `/agent-browser` and execute each scenario (reconnaissance-then-action pattern).
   - Do NOT use Playwright MCP tools or other browser automation directly.
```

## Rationale

- `/agent-browser` is the project's standard browser automation skill
- Pointer only (no command examples) -- concrete commands are the skill's responsibility
- Negative instruction ("Do NOT use Playwright") prevents agents from falling back to MCP tools
