# Plugin Consolidation to `belt:*` / `belt-agent:*` Namespace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Consolidate 7 Claude Code plugins into 2 (`belt` + `belt-agent`) with unified `belt:*` / `belt-agent:*` invocation namespace. Rename Belt Protocol skill slug `belt-agent` → `protocol` for disambiguation.

**Architecture:** Three atomic commits on a dedicated worktree. Commit 1 renames `belt-agents` plugin to `belt-agent` (singular) and skill slug to `protocol`. Commit 2 consolidates 6 user-facing plugins into a single `belt` plugin. Commit 3 updates human-facing docs (README / CHANGELOG / AGENTS.md). Each commit leaves `cargo test --workspace` green.

**Tech Stack:** Rust 1.94.1+ workspace (belt-core / belt / belt-agent crates), Claude Code plugin manifest (`plugins/<plugin>/.claude-plugin/plugin.json`), marketplace schema (`.claude-plugin/marketplace.json`), cargo test for path-lock integration tests.

**Spec:** `docs/superpowers/specs/2026-04-17-plugins-consolidation-to-belt-namespace-design.md`

---

## Setup

### Task 0: Worktree Creation

**Files:**
- Create: `.claude/worktrees/plugins-consolidation/`

- [ ] **Step 1: Create isolated worktree on a new branch**

Run:
```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
git worktree add -b 2026-04-17-plugins-consolidation .claude/worktrees/plugins-consolidation main
cd .claude/worktrees/plugins-consolidation
```

Expected: `Preparing worktree (new branch ...)` and `HEAD is now at 5264d21` (or later).

- [ ] **Step 2: Confirm clean state**

Run:
```bash
git status
```

Expected: `nothing to commit, working tree clean`

All subsequent tasks assume `pwd` is `.claude/worktrees/plugins-consolidation`.

---

## Phase A: `belt-agents` → `belt-agent` (Commit 1)

Goal: rename the foundation plugin (plural → singular), rename the skill slug (`belt-agent` → `protocol`), update all internal references in-place, and update marketplace.json for this plugin only. Leave the 6 user-facing plugins untouched in this commit.

### Task A1: Rename plugin directory and skill slug

**Files:**
- Move: `plugins/belt-agents/` → `plugins/belt-agent/`
- Move: `plugins/belt-agent/skills/belt-agent/` → `plugins/belt-agent/skills/protocol/`

- [ ] **Step 1: Rename the plugin directory**

Run:
```bash
git mv plugins/belt-agents plugins/belt-agent
```

Expected: silent success. `git status` shows 12 renamed files under `plugins/belt-agent/` (5 agents + 5 references + 1 SKILL.md + 1 plugin.json).

- [ ] **Step 2: Verify rename is clean (no content drift)**

Run:
```bash
git diff --cached --stat -M plugins/belt-agent/
```

Expected: each line ends with `R100` (100% rename, zero content change). If any shows `+N/-N`, **STOP** and investigate.

- [ ] **Step 3: Rename the skill slug subdirectory**

Run:
```bash
git mv plugins/belt-agent/skills/belt-agent plugins/belt-agent/skills/protocol
```

Expected: silent success. Additional rename in `git status`.

- [ ] **Step 4: Verify final directory layout**

Run:
```bash
ls plugins/belt-agent/
ls plugins/belt-agent/skills/
```

Expected:
```
.claude-plugin/  agents/  references/  skills/
```
```
protocol/
```

### Task A2: Update SKILL.md frontmatter and plugin.json

**Files:**
- Modify: `plugins/belt-agent/skills/protocol/SKILL.md:2`
- Modify: `plugins/belt-agent/.claude-plugin/plugin.json`

- [ ] **Step 1: Rename skill in SKILL.md frontmatter**

Use the Edit tool on `plugins/belt-agent/skills/protocol/SKILL.md`:

```
old_string: name: belt-agent
new_string: name: protocol
```

- [ ] **Step 2: Verify frontmatter**

Run:
```bash
head -5 plugins/belt-agent/skills/protocol/SKILL.md
```

Expected (line 2): `name: protocol`

- [ ] **Step 3: Update plugin.json**

Use the Write tool to replace `plugins/belt-agent/.claude-plugin/plugin.json` with:

```json
{
  "name": "belt-agent",
  "description": "Foundation: Belt Protocol skill (driver for belt-agent CLI) + 5 analysis agents (phase-auditor, feature-implementer, code-explorer, code-architect, impact-analyzer) + shared references",
  "version": "0.1.0",
  "author": { "name": "neko-neko" }
}
```

- [ ] **Step 4: Verify plugin.json is valid JSON**

Run:
```bash
python3 -c "import json; json.load(open('plugins/belt-agent/.claude-plugin/plugin.json')); print('valid')"
```

Expected: `valid`

### Task A3: Replace `belt-agents:` → `belt-agent:` in all affected files

**Files (14 files with 25 total occurrences):**
- Modify: `plugins/bug-fix/skills/bug-fix/SKILL.md` (2 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/execute.md` (2 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/dogfood.md` (1 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/integrate.md` (1 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/rca.md` (5 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/code-review.md` (1 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/fix-plan-review.md` (1 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/fix-plan.md` (1 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/monkey-test.md` (1 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/references/rca-supplement.md` (3 occ)
- Modify: `plugins/feature-dev/skills/feature-dev/SKILL.md` (1 occ)
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/execute.md` (2 occ)
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/code-review.md` (1 occ)
- Modify: `plugins/feature-dev/skills/feature-dev/references/brainstorming-supplement.md` (3 occ)

**Constraint:** `feature-dev/criteria/execute.md` and `bug-fix/criteria/execute.md` must remain byte-identical (`shared_criteria_parity.rs` test). Same for `code-review.md`. Use the **same replacement** on both pairs in this task.

- [ ] **Step 1: Confirm current occurrence count**

Run:
```bash
grep -rn "belt-agents:" plugins/bug-fix/ plugins/feature-dev/ | wc -l
```

Expected: `25`

- [ ] **Step 2: Apply global replacement**

For each file in the Files list above, use the Edit tool with:
```
old_string: belt-agents:
new_string: belt-agent:
replace_all: true
```

Apply in the same logical group so that paired files (feature-dev/bug-fix execute.md and code-review.md) get the same diff.

- [ ] **Step 3: Verify all occurrences replaced**

Run:
```bash
grep -rn "belt-agents:" plugins/bug-fix/ plugins/feature-dev/ | wc -l
```

Expected: `0`

- [ ] **Step 4: Verify new references exist**

Run:
```bash
grep -rn "belt-agent:" plugins/bug-fix/ plugins/feature-dev/ | wc -l
```

Expected: `25`

- [ ] **Step 5: Verify shared-criteria parity preserved**

Run:
```bash
diff plugins/feature-dev/skills/feature-dev/criteria/execute.md plugins/bug-fix/skills/bug-fix/criteria/execute.md
diff plugins/feature-dev/skills/feature-dev/criteria/code-review.md plugins/bug-fix/skills/bug-fix/criteria/code-review.md
```

Expected: no output (files byte-identical) for both diff commands.

### Task A4: Replace `plugins/belt-agents/references/` → `plugins/belt-agent/references/`

**Files (15 files with 18 total occurrences):**
- Modify: `plugins/feature-dev/skills/feature-dev/SKILL.md` (2 occ)
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/design.md` (1 occ)
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/plan.md` (1 occ)
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/code-review.md` (1 occ)
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/execute.md` (1 occ)
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/monkey-test.md` (1 occ)
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/dogfood.md` (1 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/SKILL.md` (2 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/rca.md` (1 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/fix-plan.md` (1 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/execute.md` (1 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/code-review.md` (1 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/monkey-test.md` (1 occ)
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/dogfood.md` (1 occ)
- Modify: `plugins/belt-agent/skills/protocol/SKILL.md` (1 occ, line 98)
- Modify: `plugins/belt-agent/references/narrative-convention.md` (1 occ, line 97)

- [ ] **Step 1: Confirm current occurrence count**

Run:
```bash
grep -rn "plugins/belt-agents/references/" plugins/ | wc -l
```

Expected: `18`

- [ ] **Step 2: Apply replacement on each file**

For each file in the Files list, use the Edit tool with:
```
old_string: plugins/belt-agents/references/
new_string: plugins/belt-agent/references/
replace_all: true
```

- [ ] **Step 3: Verify all occurrences replaced**

Run:
```bash
grep -rn "plugins/belt-agents/references/" plugins/ | wc -l
```

Expected: `0`

- [ ] **Step 4: Verify new references exist**

Run:
```bash
grep -rn "plugins/belt-agent/references/" plugins/ | wc -l
```

Expected: `18` (or more, accounting for any internal self-references in the reference docs themselves)

- [ ] **Step 5: Verify shared-criteria parity preserved**

Run:
```bash
diff plugins/feature-dev/skills/feature-dev/criteria/execute.md plugins/bug-fix/skills/bug-fix/criteria/execute.md
diff plugins/feature-dev/skills/feature-dev/criteria/code-review.md plugins/bug-fix/skills/bug-fix/criteria/code-review.md
```

Expected: no output (byte-identical).

### Task A5: Fix pre-existing stale references

**Files:**
- Modify: `plugins/belt-agent/agents/phase-auditor.md:43`
- Modify: `plugins/belt-agent/skills/protocol/SKILL.md:150`

These are pre-existing bugs from earlier refactors. Fix them while the files are being touched.

- [ ] **Step 1: Fix phase-auditor.md stale reference**

Use the Edit tool on `plugins/belt-agent/agents/phase-auditor.md`:
```
old_string: 3. Read `claude/agents/references/evidence-catalog.md`
new_string: 3. Read `./references/evidence-catalog.md`
```

Rationale: `claude/agents/references/` was the dotfiles path before the plugin migration. The correct location is `plugins/belt-agent/references/evidence-catalog.md`, but since this agent file lives in the same plugin, a relative reference (`./references/`) is simpler and refactor-stable.

- [ ] **Step 2: Verify phase-auditor.md fix**

Run:
```bash
grep -n "claude/agents" plugins/belt-agent/agents/phase-auditor.md
```

Expected: no output.

Run:
```bash
grep -n "./references/evidence-catalog.md" plugins/belt-agent/agents/phase-auditor.md
```

Expected: line 43 with the new reference.

- [ ] **Step 3: Fix protocol/SKILL.md stale pipeline example**

Use the Edit tool on `plugins/belt-agent/skills/protocol/SKILL.md`:
```
old_string: "invoke": { "pipeline": "../spec-review/pipeline.yml", "with": {} },
new_string: "invoke": { "pipeline": "./nested-pipeline.yml", "with": {} },
```

Rationale: `spec-review/pipeline.yml` was deleted in the 2026-04-16 review-skills-subagent-boundary refactor. Use an abstract placeholder so this example does not re-rot on future structural changes.

- [ ] **Step 4: Verify protocol SKILL.md fix**

Run:
```bash
grep -n "spec-review/pipeline.yml" plugins/belt-agent/skills/protocol/SKILL.md
```

Expected: no output.

Run:
```bash
grep -n "nested-pipeline.yml" plugins/belt-agent/skills/protocol/SKILL.md
```

Expected: line 150 with the new placeholder.

### Task A6: Update marketplace.json — belt-agents entry

**Files:**
- Modify: `.claude-plugin/marketplace.json`

- [ ] **Step 1: Edit the belt-agents entry**

Use the Edit tool on `.claude-plugin/marketplace.json`:
```
old_string:     {
      "name": "belt-agents",
      "description": "Base analysis agents + Belt Protocol skill for belt-based quality-gated development pipelines (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer)",
      "source": "./plugins/belt-agents",
      "category": "development"
    },
new_string:     {
      "name": "belt-agent",
      "description": "Foundation: Belt Protocol skill (driver for belt-agent CLI) + 5 analysis agents (phase-auditor, feature-implementer, code-explorer, code-architect, impact-analyzer) + shared references",
      "source": "./plugins/belt-agent",
      "category": "development"
    },
```

- [ ] **Step 2: Verify the marketplace.json is valid JSON**

Run:
```bash
python3 -c "import json; json.load(open('.claude-plugin/marketplace.json')); print('valid')"
```

Expected: `valid`

- [ ] **Step 3: Confirm the updated entry**

Run:
```bash
grep -A 3 '"name": "belt-agent"' .claude-plugin/marketplace.json
```

Expected output includes `"source": "./plugins/belt-agent"`.

### Task A7: Verify workspace and commit Phase A

- [ ] **Step 1: Run Rust tests**

Run:
```bash
cargo test --workspace
```

Expected: all tests pass. (Phase A did not touch any Rust test's hardcoded paths because `plugins/belt-agents/` is not referenced from any Rust test.)

- [ ] **Step 2: Run clippy**

Run:
```bash
cargo clippy --workspace -- -D warnings
```

Expected: clean (no warnings).

- [ ] **Step 3: Lint the pipelines (old paths, since Phase B not yet done)**

Run:
```bash
cargo run -p belt -- lint plugins/feature-dev/skills/feature-dev/pipeline.yml
cargo run -p belt -- lint plugins/bug-fix/skills/bug-fix/pipeline.yml
```

Expected: `ok:` for both.

- [ ] **Step 4: Stale reference sweep**

Run:
```bash
grep -rn "belt-agents" plugins/ crates/ .claude-plugin/ 2>/dev/null
```

Expected: no output.

- [ ] **Step 5: Stage and inspect the diff**

Run:
```bash
git add -A plugins/belt-agent plugins/feature-dev plugins/bug-fix .claude-plugin/marketplace.json
git status
git diff --cached --stat
```

Expected: renames under `plugins/belt-agent/`, modified files under `plugins/feature-dev/` and `plugins/bug-fix/`, and modified `marketplace.json`. Around 30-35 files in total.

- [ ] **Step 6: Commit Phase A**

Run:
```bash
git -c commit.gpgsign=false commit -m "$(cat <<'EOF'
refactor(plugins): rename belt-agents → belt-agent, skill belt-agent → protocol

Rename the foundation plugin from plural to singular so it matches the
belt-agent CLI binary name, and rename the Belt Protocol skill slug from
belt-agent → protocol to drop the belt-agent:belt-agent redundancy.

Update all in-repo references:
- belt-agents:<agent> → belt-agent:<agent> (25 occurrences, 14 files)
- plugins/belt-agents/references/ → plugins/belt-agent/references/
  (18 occurrences, 15 files)
- .claude-plugin/marketplace.json: belt-agents entry → belt-agent

Fix pre-existing stale references while touching the files:
- phase-auditor.md: dotfiles-era claude/agents/... path → ./references/
- protocol/SKILL.md: deleted ../spec-review/pipeline.yml example →
  abstract ./nested-pipeline.yml placeholder

Part of the 2-plugin consolidation spec:
docs/superpowers/specs/2026-04-17-plugins-consolidation-to-belt-namespace-design.md
EOF
)"
```

Expected: commit succeeds with roughly 30-35 files changed.

---

## Phase B: Consolidate 6 user-facing plugins into `belt` (Commit 2)

Goal: move `feature-dev`, `bug-fix`, `code-review`, `spec-review`, `monkey-test`, `test-scenarios` into a single `belt` plugin with a flat `plugins/belt/agents/` directory aggregating all 7 reviewer agents.

### Task B1: Create `belt` plugin directory scaffolding

**Files:**
- Create: `plugins/belt/.claude-plugin/plugin.json`

- [ ] **Step 1: Create the plugin directory**

Run:
```bash
mkdir -p plugins/belt/.claude-plugin plugins/belt/skills plugins/belt/agents
```

Expected: silent success.

- [ ] **Step 2: Write plugin.json**

Use the Write tool to create `plugins/belt/.claude-plugin/plugin.json`:

```json
{
  "name": "belt",
  "description": "User-invocable skills and their reviewer agents: /belt:feature-dev, /belt:bug-fix, /belt:code-review (4 reviewers), /belt:spec-review (3 reviewers), /belt:monkey-test, /belt:test-scenarios. Requires belt-agent plugin",
  "version": "0.1.0",
  "author": { "name": "neko-neko" }
}
```

- [ ] **Step 3: Verify JSON**

Run:
```bash
python3 -c "import json; json.load(open('plugins/belt/.claude-plugin/plugin.json')); print('valid')"
```

Expected: `valid`

### Task B2: Move feature-dev skill

**Files:**
- Move: `plugins/feature-dev/skills/feature-dev/` → `plugins/belt/skills/feature-dev/`

- [ ] **Step 1: Move the skill directory**

Run:
```bash
git mv plugins/feature-dev/skills/feature-dev plugins/belt/skills/feature-dev
```

Expected: silent success.

- [ ] **Step 2: Verify rename is content-clean**

Run:
```bash
git diff --cached --stat -M plugins/belt/skills/feature-dev/ | head -25
```

Expected: all lines end with `R100`.

- [ ] **Step 3: Remove the now-empty skills subdir, but leave the plugin directory for Task B8**

Run:
```bash
rmdir plugins/feature-dev/skills
ls plugins/feature-dev/
```

Expected: `rmdir` succeeds silently. `ls` prints `.claude-plugin` only. The `plugins/feature-dev/.claude-plugin/plugin.json` is handled in Task B8.

### Task B3: Move bug-fix skill

**Files:**
- Move: `plugins/bug-fix/skills/bug-fix/` → `plugins/belt/skills/bug-fix/`

- [ ] **Step 1: Move the skill directory**

Run:
```bash
git mv plugins/bug-fix/skills/bug-fix plugins/belt/skills/bug-fix
```

Expected: silent success.

- [ ] **Step 2: Verify rename is content-clean**

Run:
```bash
git diff --cached --stat -M plugins/belt/skills/bug-fix/ | head -25
```

Expected: all lines end with `R100`.

- [ ] **Step 3: Remove the now-empty skills subdir**

Run:
```bash
rmdir plugins/bug-fix/skills
```

Expected: silent success. `plugins/bug-fix/.claude-plugin/plugin.json` remains for Task B8.

### Task B4: Move code-review skill + its reviewer agents

**Files:**
- Move: `plugins/code-review/skills/code-review/SKILL.md` → `plugins/belt/skills/code-review/SKILL.md`
- Move: `plugins/code-review/agents/security-reviewer.md` → `plugins/belt/agents/security-reviewer.md`
- Move: `plugins/code-review/agents/test-reviewer.md` → `plugins/belt/agents/test-reviewer.md`
- Move: `plugins/code-review/agents/ai-antipattern-reviewer.md` → `plugins/belt/agents/ai-antipattern-reviewer.md`
- Move: `plugins/code-review/agents/cross-cutting-reviewer.md` → `plugins/belt/agents/cross-cutting-reviewer.md`

- [ ] **Step 1: Move the SKILL.md**

Run:
```bash
mkdir -p plugins/belt/skills/code-review
git mv plugins/code-review/skills/code-review/SKILL.md plugins/belt/skills/code-review/SKILL.md
```

Expected: silent success.

- [ ] **Step 2: Move the 4 reviewer agents**

Run:
```bash
git mv plugins/code-review/agents/security-reviewer.md plugins/belt/agents/security-reviewer.md
git mv plugins/code-review/agents/test-reviewer.md plugins/belt/agents/test-reviewer.md
git mv plugins/code-review/agents/ai-antipattern-reviewer.md plugins/belt/agents/ai-antipattern-reviewer.md
git mv plugins/code-review/agents/cross-cutting-reviewer.md plugins/belt/agents/cross-cutting-reviewer.md
```

Expected: silent success for each.

- [ ] **Step 3: Remove empty subdirectories**

Run:
```bash
rmdir plugins/code-review/skills/code-review plugins/code-review/skills plugins/code-review/agents
```

Expected: silent success.

### Task B5: Move spec-review skill + its reviewer agents

**Files:**
- Move: `plugins/spec-review/skills/spec-review/SKILL.md` → `plugins/belt/skills/spec-review/SKILL.md`
- Move: `plugins/spec-review/agents/feasibility-reviewer.md` → `plugins/belt/agents/feasibility-reviewer.md`
- Move: `plugins/spec-review/agents/ui-design-reviewer.md` → `plugins/belt/agents/ui-design-reviewer.md`
- Move: `plugins/spec-review/agents/cross-cutting-spec-reviewer.md` → `plugins/belt/agents/cross-cutting-spec-reviewer.md`

- [ ] **Step 1: Move the SKILL.md**

Run:
```bash
mkdir -p plugins/belt/skills/spec-review
git mv plugins/spec-review/skills/spec-review/SKILL.md plugins/belt/skills/spec-review/SKILL.md
```

Expected: silent success.

- [ ] **Step 2: Move the 3 reviewer agents**

Run:
```bash
git mv plugins/spec-review/agents/feasibility-reviewer.md plugins/belt/agents/feasibility-reviewer.md
git mv plugins/spec-review/agents/ui-design-reviewer.md plugins/belt/agents/ui-design-reviewer.md
git mv plugins/spec-review/agents/cross-cutting-spec-reviewer.md plugins/belt/agents/cross-cutting-spec-reviewer.md
```

Expected: silent success for each.

- [ ] **Step 3: Remove empty subdirectories**

Run:
```bash
rmdir plugins/spec-review/skills/spec-review plugins/spec-review/skills plugins/spec-review/agents
```

Expected: silent success.

### Task B6: Move monkey-test and test-scenarios skills

**Files:**
- Move: `plugins/monkey-test/skills/monkey-test/SKILL.md` → `plugins/belt/skills/monkey-test/SKILL.md`
- Move: `plugins/test-scenarios/skills/test-scenarios/SKILL.md` → `plugins/belt/skills/test-scenarios/SKILL.md`

- [ ] **Step 1: Move monkey-test**

Run:
```bash
mkdir -p plugins/belt/skills/monkey-test
git mv plugins/monkey-test/skills/monkey-test/SKILL.md plugins/belt/skills/monkey-test/SKILL.md
rmdir plugins/monkey-test/skills/monkey-test plugins/monkey-test/skills
```

Expected: silent success.

- [ ] **Step 2: Move test-scenarios**

Run:
```bash
mkdir -p plugins/belt/skills/test-scenarios
git mv plugins/test-scenarios/skills/test-scenarios/SKILL.md plugins/belt/skills/test-scenarios/SKILL.md
rmdir plugins/test-scenarios/skills/test-scenarios plugins/test-scenarios/skills
```

Expected: silent success.

### Task B7: Verify directory layout after moves

- [ ] **Step 1: Inspect plugins/belt/**

Run:
```bash
ls plugins/belt/
ls plugins/belt/skills/
ls plugins/belt/agents/
```

Expected:
```
.claude-plugin/  agents/  skills/
```
```
bug-fix/  code-review/  feature-dev/  monkey-test/  spec-review/  test-scenarios/
```
```
ai-antipattern-reviewer.md   cross-cutting-spec-reviewer.md  security-reviewer.md
cross-cutting-reviewer.md    feasibility-reviewer.md         test-reviewer.md
                             ui-design-reviewer.md
```
(7 agents in flat layout)

- [ ] **Step 2: Verify each remaining source plugin only has .claude-plugin/**

Run:
```bash
ls plugins/feature-dev/ plugins/bug-fix/ plugins/code-review/ plugins/spec-review/ plugins/monkey-test/ plugins/test-scenarios/
```

Expected: each prints only `.claude-plugin`.

### Task B8: Delete old plugin.json files and empty directories

**Files:**
- Delete: `plugins/feature-dev/.claude-plugin/plugin.json`
- Delete: `plugins/bug-fix/.claude-plugin/plugin.json`
- Delete: `plugins/code-review/.claude-plugin/plugin.json`
- Delete: `plugins/spec-review/.claude-plugin/plugin.json`
- Delete: `plugins/monkey-test/.claude-plugin/plugin.json`
- Delete: `plugins/test-scenarios/.claude-plugin/plugin.json`
- Delete: `plugins/feature-dev/`, `plugins/bug-fix/`, `plugins/code-review/`, `plugins/spec-review/`, `plugins/monkey-test/`, `plugins/test-scenarios/`

- [ ] **Step 1: git rm the 6 old plugin.json files**

Run:
```bash
git rm plugins/feature-dev/.claude-plugin/plugin.json
git rm plugins/bug-fix/.claude-plugin/plugin.json
git rm plugins/code-review/.claude-plugin/plugin.json
git rm plugins/spec-review/.claude-plugin/plugin.json
git rm plugins/monkey-test/.claude-plugin/plugin.json
git rm plugins/test-scenarios/.claude-plugin/plugin.json
```

Expected: each prints `rm 'plugins/...'`.

- [ ] **Step 2: Remove empty parent directories from the filesystem**

Run:
```bash
rmdir plugins/feature-dev/.claude-plugin plugins/feature-dev
rmdir plugins/bug-fix/.claude-plugin plugins/bug-fix
rmdir plugins/code-review/.claude-plugin plugins/code-review
rmdir plugins/spec-review/.claude-plugin plugins/spec-review
rmdir plugins/monkey-test/.claude-plugin plugins/monkey-test
rmdir plugins/test-scenarios/.claude-plugin plugins/test-scenarios
```

Expected: silent success for each. (Git does not track empty directories, so these are only on the working tree.)

- [ ] **Step 3: Verify only belt and belt-agent remain under plugins/**

Run:
```bash
ls plugins/
```

Expected:
```
belt  belt-agent
```

### Task B9: Replace `/<skill>:<skill>` → `/belt:<skill>` in all affected files

**Files (9 files with 22 total occurrences; paths reflect post-move state):**
- Modify: `plugins/belt/skills/feature-dev/pipeline.yml` (4 occ)
- Modify: `plugins/belt/skills/feature-dev/criteria/code-review.md` (1 occ)
- Modify: `plugins/belt/skills/bug-fix/pipeline.yml` (3 occ)
- Modify: `plugins/belt/skills/bug-fix/SKILL.md` (3 occ)
- Modify: `plugins/belt/skills/bug-fix/criteria/code-review.md` (1 occ)
- Modify: `plugins/belt/skills/bug-fix/criteria/fix-plan-review.md` (2 occ)
- Modify: `plugins/belt/skills/bug-fix/criteria/monkey-test.md` (1 occ)
- Modify: `plugins/belt/skills/bug-fix/references/monkey-test-supplement.md` (3 occ)
- Modify: `README.md` (4 occ — `/feature-dev:`, `/bug-fix:`, `/code-review:`, `/spec-review:` in Usage)

**Constraint:** `feature-dev/criteria/code-review.md` and `bug-fix/criteria/code-review.md` must remain byte-identical.

- [ ] **Step 1: Count current occurrences**

Run:
```bash
grep -rn "/feature-dev:feature-dev\|/bug-fix:bug-fix\|/code-review:code-review\|/spec-review:spec-review\|/monkey-test:monkey-test\|/test-scenarios:test-scenarios" plugins/belt/ README.md | wc -l
```

Expected: `22`

- [ ] **Step 2: Apply replacements**

For each of the 6 patterns, apply across all affected files. Use the Edit tool with `replace_all: true` per file:

Pattern 1 — `/feature-dev:feature-dev` → `/belt:feature-dev` (README.md only)
Pattern 2 — `/bug-fix:bug-fix` → `/belt:bug-fix` (README.md only)
Pattern 3 — `/code-review:code-review` → `/belt:code-review` (feature-dev pipeline.yml, feature-dev criteria/code-review.md, bug-fix pipeline.yml, bug-fix SKILL.md, bug-fix criteria/code-review.md, README.md)
Pattern 4 — `/spec-review:spec-review` → `/belt:spec-review` (feature-dev pipeline.yml, bug-fix pipeline.yml, bug-fix SKILL.md [×2 occ], bug-fix criteria/fix-plan-review.md [×2 occ], README.md)
Pattern 5 — `/monkey-test:monkey-test` → `/belt:monkey-test` (feature-dev pipeline.yml, bug-fix pipeline.yml, bug-fix criteria/monkey-test.md, bug-fix references/monkey-test-supplement.md [×3 occ])
Pattern 6 — `/test-scenarios:test-scenarios` → `/belt:test-scenarios` (feature-dev pipeline.yml)

For each affected file, use a single Edit with `replace_all: true` per pattern.

- [ ] **Step 3: Verify all occurrences replaced**

Run:
```bash
grep -rn "/feature-dev:feature-dev\|/bug-fix:bug-fix\|/code-review:code-review\|/spec-review:spec-review\|/monkey-test:monkey-test\|/test-scenarios:test-scenarios" plugins/ README.md
```

Expected: no output.

- [ ] **Step 4: Verify new references exist**

Run:
```bash
grep -rn "/belt:feature-dev\|/belt:bug-fix\|/belt:code-review\|/belt:spec-review\|/belt:monkey-test\|/belt:test-scenarios" plugins/belt/ README.md | wc -l
```

Expected: `22`

- [ ] **Step 5: Verify shared-criteria parity**

Run:
```bash
diff plugins/belt/skills/feature-dev/criteria/execute.md plugins/belt/skills/bug-fix/criteria/execute.md
diff plugins/belt/skills/feature-dev/criteria/code-review.md plugins/belt/skills/bug-fix/criteria/code-review.md
```

Expected: no output (byte-identical) for both.

### Task B10: Replace reviewer agent namespace `code-review:<r>` / `spec-review:<r>` → `belt:<r>`

**Files (2 files with 7 total occurrences):**
- Modify: `plugins/belt/skills/code-review/SKILL.md` (4 occ)
- Modify: `plugins/belt/skills/spec-review/SKILL.md` (3 occ)

- [ ] **Step 1: Count current occurrences**

Run:
```bash
grep -rn "code-review:security-reviewer\|code-review:test-reviewer\|code-review:ai-antipattern-reviewer\|code-review:cross-cutting-reviewer\|spec-review:feasibility-reviewer\|spec-review:ui-design-reviewer\|spec-review:cross-cutting-spec-reviewer" plugins/belt/ | wc -l
```

Expected: `7`

- [ ] **Step 2: Apply 7 specific replacements**

In `plugins/belt/skills/code-review/SKILL.md`, use the Edit tool 4 times (one per pattern) with `replace_all: true`:
- `code-review:security-reviewer` → `belt:security-reviewer`
- `code-review:test-reviewer` → `belt:test-reviewer`
- `code-review:ai-antipattern-reviewer` → `belt:ai-antipattern-reviewer`
- `code-review:cross-cutting-reviewer` → `belt:cross-cutting-reviewer`

In `plugins/belt/skills/spec-review/SKILL.md`, use the Edit tool 3 times (one per pattern):
- `spec-review:feasibility-reviewer` → `belt:feasibility-reviewer`
- `spec-review:ui-design-reviewer` → `belt:ui-design-reviewer`
- `spec-review:cross-cutting-spec-reviewer` → `belt:cross-cutting-spec-reviewer`

- [ ] **Step 3: Verify all occurrences replaced**

Run:
```bash
grep -rn "code-review:security-reviewer\|code-review:test-reviewer\|code-review:ai-antipattern-reviewer\|code-review:cross-cutting-reviewer\|spec-review:feasibility-reviewer\|spec-review:ui-design-reviewer\|spec-review:cross-cutting-spec-reviewer" plugins/belt/
```

Expected: no output.

- [ ] **Step 4: Verify new references**

Run:
```bash
grep -rn "belt:security-reviewer\|belt:test-reviewer\|belt:ai-antipattern-reviewer\|belt:cross-cutting-reviewer\|belt:feasibility-reviewer\|belt:ui-design-reviewer\|belt:cross-cutting-spec-reviewer" plugins/belt/ | wc -l
```

Expected: `7`

### Task B11: Update marketplace.json — add belt entry, remove 6 old entries

**Files:**
- Modify: `.claude-plugin/marketplace.json`

- [ ] **Step 1: Rewrite marketplace.json**

Use the Write tool to replace `.claude-plugin/marketplace.json` entirely with:

```json
{
  "$schema": "https://anthropic.com/claude-code/marketplace.schema.json",
  "name": "belt",
  "description": "Quality-gated AI development pipeline plugins built on the belt workflow engine",
  "owner": {
    "name": "neko-neko"
  },
  "plugins": [
    {
      "name": "belt-agent",
      "description": "Foundation: Belt Protocol skill (driver for belt-agent CLI) + 5 analysis agents (phase-auditor, feature-implementer, code-explorer, code-architect, impact-analyzer) + shared references",
      "source": "./plugins/belt-agent",
      "category": "development"
    },
    {
      "name": "belt",
      "description": "User-invocable skills and their reviewer agents: /belt:feature-dev, /belt:bug-fix, /belt:code-review (4 reviewers), /belt:spec-review (3 reviewers), /belt:monkey-test, /belt:test-scenarios. Requires belt-agent plugin",
      "source": "./plugins/belt",
      "category": "development"
    }
  ]
}
```

- [ ] **Step 2: Verify JSON**

Run:
```bash
python3 -c "import json; d=json.load(open('.claude-plugin/marketplace.json')); assert len(d['plugins'])==2; print('valid, 2 entries')"
```

Expected: `valid, 2 entries`

### Task B12: Update `review_skills_refresh.rs`

**Files:**
- Modify: `crates/belt-core/tests/review_skills_refresh.rs`

Current structure uses `REVIEW_PLUGINS: &[(&str, &[&str])]` where the first tuple element was the plugin name (`code-review` / `spec-review`). After consolidation, both skills live in the `belt` plugin, and the first element must become the skill name, with the base path built as `plugins/belt/`.

- [ ] **Step 1: Replace the REVIEW_PLUGINS constant and the loops that use it**

Use the Edit tool on `crates/belt-core/tests/review_skills_refresh.rs`:

```
old_string: /// (plugin, expected agent file basenames after refactor)
const REVIEW_PLUGINS: &[(&str, &[&str])] = &[
    (
        "code-review",
        &[
            "security-reviewer",
            "test-reviewer",
            "ai-antipattern-reviewer",
            "cross-cutting-reviewer",
        ],
    ),
    (
        "spec-review",
        &[
            "feasibility-reviewer",
            "ui-design-reviewer",
            "cross-cutting-spec-reviewer",
        ],
    ),
];

#[test]
fn review_plugins_pipeline_yml_is_deleted() {
    for (plugin, _agents) in REVIEW_PLUGINS {
        let path = repo_root()
            .join("plugins")
            .join(plugin)
            .join("skills")
            .join(plugin)
            .join("pipeline.yml");
        assert!(
            !path.exists(),
            "pipeline.yml must be deleted for {plugin}: {}",
            path.display()
        );
    }
}

#[test]
fn review_plugins_belt_toml_is_deleted() {
    for (plugin, _agents) in REVIEW_PLUGINS {
        let path = repo_root()
            .join("plugins")
            .join(plugin)
            .join("skills")
            .join(plugin)
            .join("belt.toml");
        assert!(
            !path.exists(),
            "belt.toml must be deleted for {plugin}: {}",
            path.display()
        );
    }
}

#[test]
fn review_plugins_legacy_consolidated_agent_is_deleted() {
    const LEGACY_CONSOLIDATED: &[(&str, &str)] = &[
        ("code-review", "code-reviewer"),
        ("spec-review", "spec-reviewer"),
    ];
    for (plugin, legacy) in LEGACY_CONSOLIDATED {
        let path = repo_root()
            .join("plugins")
            .join(plugin)
            .join("agents")
            .join(format!("{legacy}.md"));
        assert!(
            !path.exists(),
            "legacy consolidated agent must be deleted: {}",
            path.display()
        );
    }
}

#[test]
fn review_plugins_new_observation_agents_exist() {
    for (plugin, agents) in REVIEW_PLUGINS {
        for agent in *agents {
            let path = repo_root()
                .join("plugins")
                .join(plugin)
                .join("agents")
                .join(format!("{agent}.md"));
            assert!(
                path.exists(),
                "new observation agent file must exist: {}",
                path.display()
            );
        }
    }
}

#[test]
fn review_plugins_parent_skill_md_references_parallel_dispatch() {
    for (plugin, _agents) in REVIEW_PLUGINS {
        let path = repo_root()
            .join("plugins")
            .join(plugin)
            .join("skills")
            .join(plugin)
            .join("SKILL.md");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        assert!(
            content.contains("findings-") && content.contains("Task"),
            "{plugin} SKILL.md must describe parallel Task dispatch with findings-*.json: {}",
            path.display()
        );
    }
}
new_string: /// (skill_name, expected agent file basenames after refactor)
///
/// After the 2026-04-17 consolidation, both review skills live under the
/// single `belt` plugin, and the reviewer agents are flat under
/// `plugins/belt/agents/`. `skill_name` identifies the skill directory.
const REVIEW_SKILLS: &[(&str, &[&str])] = &[
    (
        "code-review",
        &[
            "security-reviewer",
            "test-reviewer",
            "ai-antipattern-reviewer",
            "cross-cutting-reviewer",
        ],
    ),
    (
        "spec-review",
        &[
            "feasibility-reviewer",
            "ui-design-reviewer",
            "cross-cutting-spec-reviewer",
        ],
    ),
];

#[test]
fn review_skills_pipeline_yml_is_deleted() {
    for (skill, _agents) in REVIEW_SKILLS {
        let path = repo_root()
            .join("plugins")
            .join("belt")
            .join("skills")
            .join(skill)
            .join("pipeline.yml");
        assert!(
            !path.exists(),
            "pipeline.yml must be deleted for {skill}: {}",
            path.display()
        );
    }
}

#[test]
fn review_skills_belt_toml_is_deleted() {
    for (skill, _agents) in REVIEW_SKILLS {
        let path = repo_root()
            .join("plugins")
            .join("belt")
            .join("skills")
            .join(skill)
            .join("belt.toml");
        assert!(
            !path.exists(),
            "belt.toml must be deleted for {skill}: {}",
            path.display()
        );
    }
}

#[test]
fn review_skills_legacy_consolidated_agent_is_deleted() {
    const LEGACY_CONSOLIDATED: &[&str] = &["code-reviewer", "spec-reviewer"];
    for legacy in LEGACY_CONSOLIDATED {
        let path = repo_root()
            .join("plugins")
            .join("belt")
            .join("agents")
            .join(format!("{legacy}.md"));
        assert!(
            !path.exists(),
            "legacy consolidated agent must be deleted: {}",
            path.display()
        );
    }
}

#[test]
fn review_skills_new_observation_agents_exist() {
    for (_skill, agents) in REVIEW_SKILLS {
        for agent in *agents {
            let path = repo_root()
                .join("plugins")
                .join("belt")
                .join("agents")
                .join(format!("{agent}.md"));
            assert!(
                path.exists(),
                "new observation agent file must exist: {}",
                path.display()
            );
        }
    }
}

#[test]
fn review_skills_parent_skill_md_references_parallel_dispatch() {
    for (skill, _agents) in REVIEW_SKILLS {
        let path = repo_root()
            .join("plugins")
            .join("belt")
            .join("skills")
            .join(skill)
            .join("SKILL.md");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        assert!(
            content.contains("findings-") && content.contains("Task"),
            "{skill} SKILL.md must describe parallel Task dispatch with findings-*.json: {}",
            path.display()
        );
    }
}
```

- [ ] **Step 2: Also update the top-of-file doc comment to match the new shape**

Use the Edit tool on `crates/belt-core/tests/review_skills_refresh.rs`:

```
old_string: //! Shape contract:
//! - plugins/<plugin>/skills/<plugin>/pipeline.yml is DELETED
//! - plugins/<plugin>/skills/<plugin>/belt.toml is DELETED
//! - plugins/<plugin>/agents/<single>.md is DELETED (code-reviewer / spec-reviewer)
//! - New per-observation agent files exist in plugins/<plugin>/agents/
//! - Parent SKILL.md references parallel Task dispatch and cross-agent merge
//! - Legacy per-observation agent files (from the pre-2026-04-15 era) remain
//!   absent (locked by the untouched LEGACY list below).
new_string: //! Shape contract (post-2026-04-17 consolidation):
//! - plugins/belt/skills/<skill>/pipeline.yml is DELETED for review skills
//! - plugins/belt/skills/<skill>/belt.toml is DELETED for review skills
//! - plugins/belt/agents/<consolidated>.md is DELETED (code-reviewer / spec-reviewer)
//! - New per-observation agent files exist flat under plugins/belt/agents/
//! - Parent SKILL.md references parallel Task dispatch and cross-agent merge
//! - Legacy per-observation agent files (from the pre-2026-04-15 era) remain
//!   absent (locked by the untouched LEGACY list below).
```

### Task B13: Update `shared_filter_parity.rs`

**Files:**
- Modify: `crates/belt-core/tests/shared_filter_parity.rs`

- [ ] **Step 1: Update CODE_REVIEW_AGENTS constant**

Use the Edit tool:
```
old_string: const CODE_REVIEW_AGENTS: &[&str] = &[
    "plugins/code-review/agents/security-reviewer.md",
    "plugins/code-review/agents/test-reviewer.md",
    "plugins/code-review/agents/ai-antipattern-reviewer.md",
    "plugins/code-review/agents/cross-cutting-reviewer.md",
];
new_string: const CODE_REVIEW_AGENTS: &[&str] = &[
    "plugins/belt/agents/security-reviewer.md",
    "plugins/belt/agents/test-reviewer.md",
    "plugins/belt/agents/ai-antipattern-reviewer.md",
    "plugins/belt/agents/cross-cutting-reviewer.md",
];
```

- [ ] **Step 2: Update SPEC_REVIEW_AGENTS constant**

Use the Edit tool:
```
old_string: const SPEC_REVIEW_AGENTS: &[&str] = &[
    "plugins/spec-review/agents/feasibility-reviewer.md",
    "plugins/spec-review/agents/ui-design-reviewer.md",
    "plugins/spec-review/agents/cross-cutting-spec-reviewer.md",
];
new_string: const SPEC_REVIEW_AGENTS: &[&str] = &[
    "plugins/belt/agents/feasibility-reviewer.md",
    "plugins/belt/agents/ui-design-reviewer.md",
    "plugins/belt/agents/cross-cutting-spec-reviewer.md",
];
```

### Task B14: Update `shared_criteria_parity.rs`

**Files:**
- Modify: `crates/belt-core/tests/shared_criteria_parity.rs`

- [ ] **Step 1: Update the 4 paths**

Use the Edit tool:
```
old_string:     let fd = fs::read_to_string(workspace_path(
        "plugins/feature-dev/skills/feature-dev/criteria/execute.md",
    ))
    .expect("feature-dev execute.md missing");
    let bf = fs::read_to_string(workspace_path(
        "plugins/bug-fix/skills/bug-fix/criteria/execute.md",
    ))
    .expect("bug-fix execute.md missing");
new_string:     let fd = fs::read_to_string(workspace_path(
        "plugins/belt/skills/feature-dev/criteria/execute.md",
    ))
    .expect("feature-dev execute.md missing");
    let bf = fs::read_to_string(workspace_path(
        "plugins/belt/skills/bug-fix/criteria/execute.md",
    ))
    .expect("bug-fix execute.md missing");
```

Then:
```
old_string:     let fd = fs::read_to_string(workspace_path(
        "plugins/feature-dev/skills/feature-dev/criteria/code-review.md",
    ))
    .expect("feature-dev code-review.md missing");
    let bf = fs::read_to_string(workspace_path(
        "plugins/bug-fix/skills/bug-fix/criteria/code-review.md",
    ))
    .expect("bug-fix code-review.md missing");
new_string:     let fd = fs::read_to_string(workspace_path(
        "plugins/belt/skills/feature-dev/criteria/code-review.md",
    ))
    .expect("feature-dev code-review.md missing");
    let bf = fs::read_to_string(workspace_path(
        "plugins/belt/skills/bug-fix/criteria/code-review.md",
    ))
    .expect("bug-fix code-review.md missing");
```

### Task B15: Update `feature_dev_refresh.rs` and `bug_fix_refresh.rs`

**Files:**
- Modify: `crates/belt-core/tests/feature_dev_refresh.rs:24`
- Modify: `crates/belt-core/tests/bug_fix_refresh.rs:40`

- [ ] **Step 1: Update feature_dev_refresh.rs path**

Use the Edit tool on `crates/belt-core/tests/feature_dev_refresh.rs`:
```
old_string:     path.push("plugins/feature-dev/skills/feature-dev/pipeline.yml");
new_string:     path.push("plugins/belt/skills/feature-dev/pipeline.yml");
```

- [ ] **Step 2: Update bug_fix_refresh.rs path**

Use the Edit tool on `crates/belt-core/tests/bug_fix_refresh.rs`:
```
old_string:     repo_root().join("plugins/bug-fix/skills/bug-fix")
new_string:     repo_root().join("plugins/belt/skills/bug-fix")
```

### Task B16: Update `cli_test.rs` comment

**Files:**
- Modify: `crates/belt-agent/tests/cli_test.rs:1659`

- [ ] **Step 1: Update the stale path in the doc comment**

Use the Edit tool on `crates/belt-agent/tests/cli_test.rs`:
```
old_string: /// real plugins/feature-dev/skills/feature-dev tree. This test is not meant to simulate
new_string: /// real plugins/belt/skills/feature-dev tree. This test is not meant to simulate
```

### Task B17: Verify workspace and commit Phase B

- [ ] **Step 1: Run Rust tests**

Run:
```bash
cargo test --workspace
```

Expected: all tests pass. Particularly the 5 refactor-impacted tests (`review_skills_refresh`, `shared_filter_parity`, `shared_criteria_parity`, `feature_dev_refresh`, `bug_fix_refresh`).

If any fails, inspect the error message and verify the path update in the corresponding test file was applied correctly.

- [ ] **Step 2: Run clippy**

Run:
```bash
cargo clippy --workspace -- -D warnings
```

Expected: clean.

- [ ] **Step 3: Lint the relocated pipelines**

Run:
```bash
cargo run -p belt -- lint plugins/belt/skills/feature-dev/pipeline.yml
cargo run -p belt -- lint plugins/belt/skills/bug-fix/pipeline.yml
```

Expected: `ok:` for both.

- [ ] **Step 4: Stale-path sweep**

Run:
```bash
grep -rn "plugins/feature-dev\|plugins/bug-fix\|plugins/code-review\|plugins/spec-review\|plugins/monkey-test\|plugins/test-scenarios" plugins/ crates/ .claude-plugin/ 2>/dev/null
```

Expected: no output.

Run:
```bash
grep -rn "/feature-dev:feature-dev\|/bug-fix:bug-fix\|/code-review:code-review\|/spec-review:spec-review\|/monkey-test:monkey-test\|/test-scenarios:test-scenarios" plugins/ crates/ 2>/dev/null
```

Expected: no output.

Run:
```bash
grep -rn "code-review:security-reviewer\|code-review:test-reviewer\|code-review:ai-antipattern-reviewer\|code-review:cross-cutting-reviewer\|spec-review:feasibility-reviewer\|spec-review:ui-design-reviewer\|spec-review:cross-cutting-spec-reviewer" plugins/ crates/ 2>/dev/null
```

Expected: no output.

- [ ] **Step 5: Verify shared-criteria parity after all B tasks**

Run:
```bash
diff plugins/belt/skills/feature-dev/criteria/execute.md plugins/belt/skills/bug-fix/criteria/execute.md
diff plugins/belt/skills/feature-dev/criteria/code-review.md plugins/belt/skills/bug-fix/criteria/code-review.md
```

Expected: no output (byte-identical) for both.

- [ ] **Step 6: Stage and inspect the diff**

Run:
```bash
git add -A plugins/ crates/ .claude-plugin/ README.md
git status
git diff --cached --stat | tail -20
```

Expected: 70-80 files in the diff (55 moves + ~25 text edits + 6 Rust tests + marketplace.json + README.md). Note: README.md is only partially updated in Phase B (just the `/x:x` → `/belt:x` patterns). Full README rewrite is in Phase C.

- [ ] **Step 7: Commit Phase B**

Run:
```bash
git -c commit.gpgsign=false commit -m "$(cat <<'EOF'
refactor(plugins): consolidate 6 user-facing plugins into single "belt" plugin

Move feature-dev, bug-fix, code-review, spec-review, monkey-test, and
test-scenarios into a single belt plugin. Reviewer agents are flattened
under plugins/belt/agents/ (7 files). Each user-facing skill is invoked
as /belt:<skill>, and each reviewer agent as belt:<reviewer>.

- git mv 6 skill directories → plugins/belt/skills/<skill>/
- git mv 7 reviewer agents → plugins/belt/agents/
- New plugins/belt/.claude-plugin/plugin.json
- Delete 6 old .claude-plugin/plugin.json files
- marketplace.json: 7 entries → 2 entries (belt-agent, belt)
- Text: /<skill>:<skill> → /belt:<skill> (22 occurrences, 9 files)
- Text: code-review:<reviewer> / spec-review:<reviewer> → belt:<reviewer>
  (7 occurrences, 2 files)
- Update Rust integration test paths (review_skills_refresh,
  shared_filter_parity, shared_criteria_parity, feature_dev_refresh,
  bug_fix_refresh, cli_test.rs)

Part of the 2-plugin consolidation spec:
docs/superpowers/specs/2026-04-17-plugins-consolidation-to-belt-namespace-design.md
EOF
)"
```

Expected: commit succeeds with roughly 70-80 files.

---

## Phase C: Update human-facing docs (Commit 3)

### Task C1: Rewrite README.md plugin sections

**Files:**
- Modify: `README.md` (lines ~229-307)

- [ ] **Step 1: Rewrite the "Claude Code Plugins" section**

In `README.md`, find the section starting with `## Claude Code Plugins (Working Examples)`.

Use the Edit tool to replace the full section. The current section runs from line ~229 to line ~307. Replace it with:

```
old_string: ## Claude Code Plugins (Working Examples)

belt ships 7 Claude Code plugins under `plugins/` — working examples and
production tooling for quality-gated AI-driven development.

### Plugins in this repo

| Plugin | Purpose |
|---|---|
| `belt-agents` | Base layer: 5 analysis agents (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer) + Belt Protocol skill + references |
| `feature-dev` | Quality-gated feature-development pipeline (design → implementation → review → integration). Phase structure is declared in the plugin's `pipeline.yml` |
| `bug-fix` | Quality-gated debugging pipeline (RCA → fix → review → integration). Phase structure is declared in the plugin's `pipeline.yml` |
| `code-review` | Multi-perspective code review (7 observations: quality / security / perf / test / ai-antipattern / impact / simplification) |
| `spec-review` | Multi-perspective spec review (5 observations: requirements / design-judgment / feasibility / consistency / ui-design) |
| `monkey-test` | Scripted E2E regression via agent-browser (Given/When/Then replay) |
| `test-scenarios` | Test strategy (ISTQB + ISO 25010) + Given/When/Then scenarios |

### External skill dependencies

`feature-dev` and `bug-fix` invoke skills from other plugins. `monkey-test`
requires the `agent-browser` CLI. Install them before the belt plugins that
use them:

| Dependency | Source | Required by |
|---|---|---|
| `/brainstorming` | [obra/superpowers](https://github.com/obra/superpowers) | feature-dev `design` |
| `/writing-plans` | obra/superpowers | feature-dev `plan`, bug-fix `fix-plan` |
| `/subagent-driven-development` | obra/superpowers | feature-dev `execute`, bug-fix `execute` |
| `/systematic-debugging` | obra/superpowers | bug-fix `rca` |
| `/worktrunk` | [max-sixty/worktrunk](https://github.com/max-sixty/worktrunk) | feature-dev `integrate`, bug-fix `integrate` |
| `agent-browser` CLI + `/agent-browser` skill | [vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser) | monkey-test plugin (always), feature-dev `monkey-test` phase, bug-fix `monkey-test` phase (when `--e2e`) |
| `/dogfood` | vercel-labs/agent-browser | feature-dev `dogfood`, bug-fix `dogfood` (when `--e2e`) |

### Install

belt plugins are distributed via the Claude Code plugin marketplace. Install
external plugin dependencies first, then add belt as a marketplace and install
the plugins you need.

```
# In Claude Code:

# 1. Add external plugin dependencies
/install-plugin obra/superpowers-marketplace superpowers
/install-plugin max-sixty/worktrunk worktrunk
/install-plugin vercel-labs/agent-browser agent-browser

# 2. Add belt marketplace and install plugins
/install-plugin neko-neko/belt belt-agents
/install-plugin neko-neko/belt feature-dev
/install-plugin neko-neko/belt bug-fix
/install-plugin neko-neko/belt code-review
/install-plugin neko-neko/belt spec-review
/install-plugin neko-neko/belt monkey-test
/install-plugin neko-neko/belt test-scenarios
```

Plugin discovery uses `.claude-plugin/marketplace.json` (Claude Code
marketplace format) at belt repo root.

### Internal dependencies (plugin-to-plugin)

- `feature-dev` invokes `spec-review`, `code-review`, `test-scenarios`, `monkey-test`
- `bug-fix` invokes `spec-review`, `code-review`, `monkey-test`
- `feature-dev`, `bug-fix` require `belt-agents` (analysis agents referenced by criteria and supplements)
- `code-review`, `spec-review`, `monkey-test`, `test-scenarios`, `belt-agents` are standalone

### Usage

After install:

```
/belt:feature-dev         # start a new feature
/belt:bug-fix                 # start a bug investigation
/belt:code-review         # standalone code review
/belt:spec-review         # standalone spec review
```

See each plugin's `SKILL.md` for phase details and arg reference.
new_string: ## Claude Code Plugins (Working Examples)

belt ships 2 Claude Code plugins under `plugins/` — working examples and
production tooling for quality-gated AI-driven development.

### Plugins in this repo

| Plugin | Purpose |
|---|---|
| `belt-agent` | Foundation: Belt Protocol driver skill + 5 analysis agents (phase-auditor, feature-implementer, code-explorer, code-architect, impact-analyzer) + shared references |
| `belt` | User-invocable pipelines and reviewer agents: `/belt:feature-dev`, `/belt:bug-fix`, `/belt:code-review` (4 observation reviewers), `/belt:spec-review` (3 observation reviewers), `/belt:monkey-test`, `/belt:test-scenarios`. Requires `belt-agent` |

### External skill dependencies

The belt skills invoke skills from other plugins. `/belt:monkey-test` requires
the `agent-browser` CLI. Install these before the belt plugins that use them:

| Dependency | Source | Required by |
|---|---|---|
| `/brainstorming` | [obra/superpowers](https://github.com/obra/superpowers) | `/belt:feature-dev` `design` phase |
| `/writing-plans` | obra/superpowers | `/belt:feature-dev` `plan` phase, `/belt:bug-fix` `fix-plan` phase |
| `/subagent-driven-development` | obra/superpowers | `/belt:feature-dev` `execute` phase, `/belt:bug-fix` `execute` phase |
| `/systematic-debugging` | obra/superpowers | `/belt:bug-fix` `rca` phase |
| `/worktrunk` | [max-sixty/worktrunk](https://github.com/max-sixty/worktrunk) | `/belt:feature-dev` `integrate` phase, `/belt:bug-fix` `integrate` phase |
| `agent-browser` CLI + `/agent-browser` skill | [vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser) | `/belt:monkey-test` always, `/belt:feature-dev` `monkey-test` phase, `/belt:bug-fix` `monkey-test` phase (when `--e2e`) |
| `/dogfood` | vercel-labs/agent-browser | `/belt:feature-dev` `dogfood` phase, `/belt:bug-fix` `dogfood` phase (when `--e2e`) |

### Install

belt plugins are distributed via the Claude Code plugin marketplace. Install
external plugin dependencies first, then add belt as a marketplace and install
the two belt plugins.

```
# In Claude Code:

# 1. Add external plugin dependencies
/install-plugin obra/superpowers-marketplace superpowers
/install-plugin max-sixty/worktrunk worktrunk
/install-plugin vercel-labs/agent-browser agent-browser

# 2. Add belt marketplace and install both belt plugins
/install-plugin neko-neko/belt belt-agent
/install-plugin neko-neko/belt belt
```

`belt` requires `belt-agent`; install `belt-agent` first. Plugin discovery
uses `.claude-plugin/marketplace.json` (Claude Code marketplace format) at
belt repo root.

### Usage

After install:

```
/belt:feature-dev             # start a new feature
/belt:bug-fix                 # start a bug investigation
/belt:code-review             # standalone code review
/belt:spec-review             # standalone spec review
```

See each skill's `SKILL.md` (under `plugins/belt/skills/<skill>/`) for phase
details and arg reference. Skill tool invocations inside criteria and
supplements are always written fully-qualified (`/belt:code-review`,
`belt-agent:phase-auditor`) — shorthand (`/code-review`) is not used.
```

- [ ] **Step 2: Verify README no longer references removed items**

Run:
```bash
grep -n "7 Claude Code plugins\|7 plugins\|belt-agents\b\|/feature-dev:feature-dev\|/bug-fix:bug-fix\|/code-review:code-review\|/spec-review:spec-review\|/monkey-test:monkey-test\|/test-scenarios:test-scenarios" README.md
```

Expected: no output.

Run:
```bash
grep -n "2 Claude Code plugins\|/belt:feature-dev\|/belt:bug-fix\|belt-agent" README.md | head
```

Expected: several hits confirming the new phrasing.

### Task C2: Add BREAKING note to CHANGELOG.md

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Fill in the empty [Unreleased] section**

Use the Edit tool on `CHANGELOG.md`:
```
old_string: <!-- next-header -->

## [Unreleased] - ReleaseDate

## [0.1.0] - 2026-04-17
new_string: <!-- next-header -->

## [Unreleased] - ReleaseDate

### Changed (BREAKING)
- Plugin consolidation: 7 plugins (`belt-agents`, `feature-dev`, `bug-fix`, `code-review`, `spec-review`, `monkey-test`, `test-scenarios`) → 2 plugins (`belt`, `belt-agent`).
- Skill invocation renamed:
  - `/feature-dev:feature-dev` → `/belt:feature-dev`
  - `/bug-fix:bug-fix` → `/belt:bug-fix`
  - `/code-review:code-review` → `/belt:code-review`
  - `/spec-review:spec-review` → `/belt:spec-review`
  - `/monkey-test:monkey-test` → `/belt:monkey-test`
  - `/test-scenarios:test-scenarios` → `/belt:test-scenarios`
- Agent namespace renamed:
  - `belt-agents:<agent>` → `belt-agent:<agent>` (5 base analysis agents)
  - `code-review:<reviewer>` → `belt:<reviewer>` (4 observation reviewers)
  - `spec-review:<reviewer>` → `belt:<reviewer>` (3 observation reviewers)
- Belt Protocol skill slug: `belt-agents:belt-agent` → `belt-agent:protocol`
- Installation: `/install-plugin neko-neko/belt <plugin>` now takes 2 plugin names (`belt-agent` and `belt`) instead of 7.

## [0.1.0] - 2026-04-17
```

- [ ] **Step 2: Verify CHANGELOG**

Run:
```bash
grep -n "BREAKING\|/belt:feature-dev" CHANGELOG.md
```

Expected: at least one line containing `### Changed (BREAKING)` and one or more `/belt:` invocations.

### Task C3: Add Plugin Architecture section to AGENTS.md

**Files:**
- Modify: `AGENTS.md`

- [ ] **Step 1: Insert a new section before Non-Goals**

Locate the `## Non-Goals (やらないこと)` line. Use the Edit tool to insert a new section directly above it:

```
old_string: ## Non-Goals (やらないこと)
new_string: ## Plugin Architecture

belt は Claude Code plugin として **2 plugin 構成**で配布する:

| Plugin | 責務 | 呼び出し namespace |
|---|---|---|
| `belt` | user-invocable skills + それに紐づく reviewer agents | `/belt:<skill>`, `belt:<reviewer>` |
| `belt-agent` | Belt Protocol driver skill + 汎用 analysis agents + shared references | `belt-agent:protocol`, `belt-agent:<agent>` |

- `belt` は `belt-agent` を依存として要求する (Claude Code plugin manifest に hard dependency field が無いため、README / CHANGELOG で明示)
- Skill tool invoke および agent reference は常に fully-qualified (`/belt:code-review`, `belt-agent:phase-auditor`) で記述する。Shorthand (`/code-review`) は使用禁止
- CLI binary `belt-agent` と plugin `belt-agent` が同名だが、前者は executable、後者は Claude Code config。context で区別する

## Non-Goals (やらないこと)
```

- [ ] **Step 2: Verify the section is in place**

Run:
```bash
grep -n "## Plugin Architecture\|## Non-Goals" AGENTS.md
```

Expected: the `## Plugin Architecture` line appears on a line number *smaller* than `## Non-Goals`.

### Task C4: Verify and commit Phase C

- [ ] **Step 1: Run Rust tests once more (no Rust changes but sanity check)**

Run:
```bash
cargo test --workspace
```

Expected: all pass.

- [ ] **Step 2: Final stale-reference sweep across all docs**

Run:
```bash
grep -rn "belt-agents\|/feature-dev:feature-dev\|/bug-fix:bug-fix\|/code-review:code-review\|/spec-review:spec-review\|/monkey-test:monkey-test\|/test-scenarios:test-scenarios\|plugins/feature-dev\|plugins/bug-fix\|plugins/code-review\|plugins/spec-review\|plugins/monkey-test\|plugins/test-scenarios" plugins/ crates/ .claude-plugin/ README.md CHANGELOG.md AGENTS.md 2>/dev/null
```

Expected: no output.

- [ ] **Step 3: Stage and inspect**

Run:
```bash
git add -A README.md CHANGELOG.md AGENTS.md
git status
git diff --cached --stat
```

Expected: 3 files modified.

- [ ] **Step 4: Commit Phase C**

Run:
```bash
git -c commit.gpgsign=false commit -m "$(cat <<'EOF'
docs: update README/CHANGELOG/AGENTS.md for belt + belt-agent 2-plugin layout

- README.md: rewrite Plugins table (7 → 2), External skill dependencies,
  Install commands, Internal dependencies, Usage for /belt:<skill> form
- CHANGELOG.md: add BREAKING note under [Unreleased] describing the
  plugin consolidation and all namespace renames
- AGENTS.md: add "Plugin Architecture" section before Non-Goals with
  2-plugin responsibilities, namespace rules, and CLI-vs-plugin naming

Part of the 2-plugin consolidation spec:
docs/superpowers/specs/2026-04-17-plugins-consolidation-to-belt-namespace-design.md
EOF
)"
```

Expected: commit succeeds.

---

## Phase D: Final verification and dogfood (no commit unless cleanup needed)

### Task D1: Full verification pass

- [ ] **Step 1: Final Rust check**

Run:
```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

Expected: all green.

- [ ] **Step 2: Lint both pipelines**

Run:
```bash
cargo run -p belt -- lint plugins/belt/skills/feature-dev/pipeline.yml
cargo run -p belt -- lint plugins/belt/skills/bug-fix/pipeline.yml
```

Expected: `ok:` for both.

- [ ] **Step 3: Directory structure final check**

Run:
```bash
ls plugins/
ls plugins/belt/
ls plugins/belt/skills/
ls plugins/belt/agents/ | wc -l
ls plugins/belt-agent/
ls plugins/belt-agent/skills/
ls plugins/belt-agent/agents/ | wc -l
ls plugins/belt-agent/references/ | wc -l
```

Expected:
```
belt  belt-agent
```
```
.claude-plugin  agents  skills
```
```
bug-fix  code-review  feature-dev  monkey-test  spec-review  test-scenarios
```
`7` (reviewer agents)
```
.claude-plugin  agents  references  skills
```
```
protocol
```
`5` (base agents)
`5` (references)

- [ ] **Step 4: Skill / agent name sanity check**

Run:
```bash
for f in plugins/belt/skills/*/SKILL.md; do head -5 "$f" | grep -E "^name:"; done
for f in plugins/belt-agent/skills/*/SKILL.md; do head -5 "$f" | grep -E "^name:"; done
for f in plugins/belt/agents/*.md; do head -5 "$f" | grep -E "^name:"; done
for f in plugins/belt-agent/agents/*.md; do head -5 "$f" | grep -E "^name:"; done
```

Expected: each SKILL.md / agent file has a `name:` frontmatter line matching the filename basename (e.g., `name: protocol` in `protocol/SKILL.md`, `name: security-reviewer` in `security-reviewer.md`).

### Task D2: Manual dogfood with local plugin loading

- [ ] **Step 1: Launch Claude Code with the local plugin dirs**

Run in a separate terminal (outside the brainstorming session):
```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/.claude/worktrees/plugins-consolidation
claude --plugin-dir ./plugins/belt-agent --plugin-dir ./plugins/belt
```

- [ ] **Step 2: Verify skill list includes /belt:<skill>**

Inside the Claude Code session, run `/help` or check the skill list. Expected:
- `/belt:feature-dev` appears as an invocable skill
- `/belt:bug-fix` appears
- `/belt:code-review`, `/belt:spec-review`, `/belt:monkey-test`, `/belt:test-scenarios` all appear

- [ ] **Step 3: Invoke /belt:feature-dev and verify resolve**

Inside Claude Code:
```
/belt:feature-dev
```

Expected: the SKILL.md loads, brainstorming begins (first phase `design`). If the loader fails to resolve `belt-agent:phase-auditor` at any subsequent phase, the error surfaces at the relevant criteria read.

If everything resolves, exit without completing the pipeline. This is sufficient as a smoke test.

- [ ] **Step 4: Document the dogfood outcome**

Record the outcome in the PR body:
- Which skills were listed
- Which invocation was executed
- Any loader error observed (expected: none)

### Task D3: Push the branch and open a PR

- [ ] **Step 1: Push the branch**

Run:
```bash
git push -u origin 2026-04-17-plugins-consolidation
```

Expected: upstream tracking set.

- [ ] **Step 2: Open a PR**

Run:
```bash
gh pr create --title "refactor(plugins): consolidate to belt:* / belt-agent:* namespace (2 plugins)" --body "$(cat <<'EOF'
## Summary

- Consolidate 7 Claude Code plugins into 2: `belt` (user-invocable skills + reviewer agents) and `belt-agent` (Belt Protocol + base analysis agents + references)
- Rename invocation namespaces: `/belt:<skill>`, `belt:<reviewer>`, `belt-agent:<agent>`, `belt-agent:protocol`
- Update all internal references, Rust path-lock tests, README/CHANGELOG/AGENTS.md

Spec: docs/superpowers/specs/2026-04-17-plugins-consolidation-to-belt-namespace-design.md

## Test plan

- [x] `cargo test --workspace` passes
- [x] `cargo clippy --workspace -- -D warnings` clean
- [x] `cargo run -p belt -- lint plugins/belt/skills/feature-dev/pipeline.yml` ok
- [x] `cargo run -p belt -- lint plugins/belt/skills/bug-fix/pipeline.yml` ok
- [x] Manual dogfood with `claude --plugin-dir ./plugins/belt-agent --plugin-dir ./plugins/belt`: `/belt:feature-dev` loads, `belt-agent:phase-auditor` and `belt:security-reviewer` resolve
- [x] Stale-reference sweep shows no matches of old namespace patterns
EOF
)"
```

Expected: PR URL printed.

---

## Post-PR (not part of this plan but noted)

After this PR merges to main:

1. Manually bump to v0.2.0:
   ```bash
   cargo release minor -x
   ```
   This updates version in all crate Cargo.toml files, replaces CHANGELOG `[Unreleased]` placeholders with `[0.2.0] - <date>`, commits, tags, and pushes.

2. The `v0.2.0` tag push triggers cargo-dist CI, which builds release assets for all 4 targets and publishes the GitHub Release.

3. Update the README `v0.1.0` reference in the Verify section (L210) to `v0.2.0` as part of the release commit, or in a follow-up.
