# smoke-test: agent-browser specification — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Specify `/agent-browser` as the browser tool in smoke-test adhoc-test procedure to prevent agents from using Playwright.

**Architecture:** Single file edit — add tool directive to step 3 of adhoc-test-procedure.md.

**Tech Stack:** N/A (documentation change only)

---

### Task 1: Update adhoc-test procedure

**Files:**
- Modify: `examples/skills/smoke-test/references/adhoc-test-procedure.md:13`

- [ ] **Step 1: Edit step 3 in adhoc-test-procedure.md**

Replace line 13:

```markdown
3. Execute each scenario via browser (reconnaissance-then-action pattern).
```

With:

```markdown
3. Invoke `/agent-browser` and execute each scenario (reconnaissance-then-action pattern).
   - Do NOT use Playwright MCP tools or other browser automation directly.
```

- [ ] **Step 2: Verify the change**

Run: `cat examples/skills/smoke-test/references/adhoc-test-procedure.md`

Expected: Step 3 mentions `/agent-browser` and includes the Playwright prohibition.

- [ ] **Step 3: Commit**

```bash
git add examples/skills/smoke-test/references/adhoc-test-procedure.md
git commit -m "fix(example): specify /agent-browser in smoke-test adhoc-test procedure"
```
