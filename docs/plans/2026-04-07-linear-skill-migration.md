# linear-add / linear-cleanup Skill Migration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate `linear-add` and `linear-cleanup` SKILL.md from dotfiles to the belt repo, with symlinks for Claude Code skill discovery.

**Architecture:** Copy 2 SKILL.md files into `examples/skills/`, replace dotfiles originals with symlinks to belt repo, verify skill resolution.

**Spec:** `docs/specs/2026-04-07-linear-skill-migration.md`

---

### Task 1: Copy linear-add SKILL.md to belt repo

**Files:**
- Create: `examples/skills/linear-add/SKILL.md`

- [ ] **Step 1: Create directory and copy file**

```bash
mkdir -p examples/skills/linear-add
cp ~/go/src/github.com/neko-neko/dotfiles/claude/skills/linear-add/SKILL.md examples/skills/linear-add/SKILL.md
```

- [ ] **Step 2: Verify content matches source**

```bash
diff ~/go/src/github.com/neko-neko/dotfiles/claude/skills/linear-add/SKILL.md examples/skills/linear-add/SKILL.md
```

Expected: No output (files are identical).

- [ ] **Step 3: Commit**

```bash
git add examples/skills/linear-add/SKILL.md
git commit -m "feat(examples): add linear-add skill

Migrated from dotfiles to consolidate linear-refresh dependencies
in the belt repo. Symlinks will maintain skill discoverability."
```

---

### Task 2: Copy linear-cleanup SKILL.md to belt repo

**Files:**
- Create: `examples/skills/linear-cleanup/SKILL.md`

- [ ] **Step 1: Create directory and copy file**

```bash
mkdir -p examples/skills/linear-cleanup
cp ~/go/src/github.com/neko-neko/dotfiles/claude/skills/linear-cleanup/SKILL.md examples/skills/linear-cleanup/SKILL.md
```

- [ ] **Step 2: Verify content matches source**

```bash
diff ~/go/src/github.com/neko-neko/dotfiles/claude/skills/linear-cleanup/SKILL.md examples/skills/linear-cleanup/SKILL.md
```

Expected: No output (files are identical).

- [ ] **Step 3: Commit**

```bash
git add examples/skills/linear-cleanup/SKILL.md
git commit -m "feat(examples): add linear-cleanup skill

Migrated from dotfiles to consolidate linear-refresh dependencies
in the belt repo. Symlinks will maintain skill discoverability."
```

---

### Task 3: Replace dotfiles originals with symlinks

**Files:**
- Delete: `~/go/src/github.com/neko-neko/dotfiles/claude/skills/linear-add/SKILL.md`
- Delete: `~/go/src/github.com/neko-neko/dotfiles/claude/skills/linear-cleanup/SKILL.md`
- Create: `~/.claude/skills/linear-add` (symlink)
- Create: `~/.claude/skills/linear-cleanup` (symlink)

- [ ] **Step 1: Remove dotfiles linear-add directory**

```bash
rm -rf ~/go/src/github.com/neko-neko/dotfiles/claude/skills/linear-add
```

- [ ] **Step 2: Remove dotfiles linear-cleanup directory**

```bash
rm -rf ~/go/src/github.com/neko-neko/dotfiles/claude/skills/linear-cleanup
```

- [ ] **Step 3: Remove existing ~/.claude/skills symlinks or directories if present**

```bash
rm -rf ~/.claude/skills/linear-add
rm -rf ~/.claude/skills/linear-cleanup
```

- [ ] **Step 4: Create symlink for linear-add**

```bash
ln -s /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/linear-add ~/.claude/skills/linear-add
```

- [ ] **Step 5: Create symlink for linear-cleanup**

```bash
ln -s /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/linear-cleanup ~/.claude/skills/linear-cleanup
```

- [ ] **Step 6: Verify symlinks resolve correctly**

```bash
ls -la ~/.claude/skills/linear-add
ls -la ~/.claude/skills/linear-cleanup
cat ~/.claude/skills/linear-add/SKILL.md | head -5
cat ~/.claude/skills/linear-cleanup/SKILL.md | head -5
```

Expected:
- Both symlinks point to belt repo paths
- `cat` outputs the first 5 lines of each SKILL.md (frontmatter with `name: linear-add` / `name: linear-cleanup`)

- [ ] **Step 7: Commit dotfiles removal**

```bash
cd ~/go/src/github.com/neko-neko/dotfiles
git add -A claude/skills/linear-add claude/skills/linear-cleanup
git commit -m "refactor: migrate linear-add, linear-cleanup to belt repo

Skills are now maintained in neko-neko/belt examples/skills/.
~/.claude/skills/ symlinks maintain discoverability."
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
```

---

### Task 4: Verify end-to-end skill resolution

- [ ] **Step 1: Verify belt repo file structure**

```bash
ls -la examples/skills/linear-add/SKILL.md examples/skills/linear-cleanup/SKILL.md
```

Expected: Both files exist.

- [ ] **Step 2: Verify skill frontmatter is intact**

```bash
head -8 examples/skills/linear-add/SKILL.md
head -8 examples/skills/linear-cleanup/SKILL.md
```

Expected:
- linear-add: `name: linear-add`, `user-invocable: true`
- linear-cleanup: `name: linear-cleanup`, `user-invocable: true`

- [ ] **Step 3: Verify symlink chain**

```bash
readlink ~/.claude/skills/linear-add
readlink ~/.claude/skills/linear-cleanup
```

Expected:
- `/Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/linear-add`
- `/Users/nishikataseiichi/go/src/github.com/neko-neko/belt/examples/skills/linear-cleanup`

- [ ] **Step 4: Verify dotfiles originals are removed**

```bash
ls ~/go/src/github.com/neko-neko/dotfiles/claude/skills/linear-add 2>&1
ls ~/go/src/github.com/neko-neko/dotfiles/claude/skills/linear-cleanup 2>&1
```

Expected: `No such file or directory` for both.
