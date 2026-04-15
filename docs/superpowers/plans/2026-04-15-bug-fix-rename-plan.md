# `debug-flow` → `bug-fix` Rename Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename `examples/skills/debug-flow/` to `bug-fix/` and update all live references in the skill directory and belt-core / belt-agent test crates, while preserving historical spec / plan references.

**Architecture:** Mechanical rename across two scopes — (A) the skill directory itself, and (B) live Rust code. Four case variants are swapped (`debug-flow`→`bug-fix`, `debug_flow`→`bug_fix`, `Debug Flow`→`Bug Fix`, `/debug-flow`→`/bug-fix`). Strings that name a dated spec / plan by filename or title are preserved verbatim (Exception Rule). All changes land in a single commit because the skill directory path is referenced from tests; partial states would leave tests broken.

**Tech Stack:** `git`, `cargo` (workspace: belt-core, belt-agent), `rg` (for residual-string scan).

**Spec:** `docs/superpowers/specs/2026-04-15-bug-fix-rename-design.md`

---

## File Structure

### Files renamed (git mv, history preserved)

- `examples/skills/debug-flow/` → `examples/skills/bug-fix/` (directory)
- `crates/belt-core/tests/debug_flow_refresh.rs` → `crates/belt-core/tests/bug_fix_refresh.rs`

### Files modified (contents updated)

Inside renamed directory (8 files):
- `examples/skills/bug-fix/SKILL.md`
- `examples/skills/bug-fix/pipeline.yml`
- `examples/skills/bug-fix/references/path-convention.md`
- `examples/skills/bug-fix/references/worktrunk-supplement.md`
- `examples/skills/bug-fix/references/rca-supplement.md`
- `examples/skills/bug-fix/references/monkey-test-supplement.md`
- `examples/skills/bug-fix/references/dogfood-supplement.md`
- `examples/skills/bug-fix/references/fix-plan-supplement.md`

Live Rust code (2 files):
- `crates/belt-core/tests/bug_fix_refresh.rs` (post-rename)
- `crates/belt-agent/tests/e2e_test.rs`

### Files NOT modified (Exception Rule)

Strings that reference historical spec / plan filenames or titles are preserved:
- `crates/belt-core/src/model.rs:183` — `see spec 2026-04-15-debug-flow-refresh-design.md` (historical spec filename)
- `crates/belt-core/src/view.rs:236` — `Grammar (MVP, debug-flow refresh spec, 2026-04-15)` (historical spec title)
- `crates/belt-core/tests/artifact_when_field.rs:3` — `2026-04-15-debug-flow-refresh-design.md` (historical spec filename)
- Line 3 of the renamed test file — same spec filename reference (see Task 3)

---

## Task 1: Rename skill directory and update in-directory references

**Files:**
- Rename: `examples/skills/debug-flow/` → `examples/skills/bug-fix/`
- Modify: 8 files inside the renamed directory (listed above)

- [x] **Step 1.1: Rename directory via `git mv`**

Run:
```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
git mv examples/skills/debug-flow examples/skills/bug-fix
```

Expected: directory moved, `git status` shows renamed files.

- [x] **Step 1.2: Update `examples/skills/bug-fix/SKILL.md`**

Apply these three exact replacements:

1. Line 2 (frontmatter): `name: debug-flow` → `name: bug-fix`
2. Line 11 (H1 heading): `# debug-flow` → `# bug-fix`
3. Line 72 (prose): `debug-flow 固有 override` → `bug-fix 固有 override`

Verify residuals are zero:
```bash
rg --pcre2 "debug[-_ ]?flow|Debug Flow|DebugFlow" examples/skills/bug-fix/SKILL.md
```
Expected: no output.

- [x] **Step 1.3: Update `examples/skills/bug-fix/pipeline.yml`**

Apply:
- Line 1: `name: debug-flow` → `name: bug-fix`

Verify:
```bash
rg --pcre2 "debug[-_ ]?flow|Debug Flow" examples/skills/bug-fix/pipeline.yml
```
Expected: no output.

- [x] **Step 1.4: Update `examples/skills/bug-fix/references/path-convention.md`**

Apply:
1. Line 1: `# Debug Flow Path Convention` → `# Bug Fix Path Convention`
2. Line 3: `for a debug-flow run.` → `for a bug-fix run.`
3. Line 7: `All debug-flow run outputs live under:` → `All bug-fix run outputs live under:`

Verify residuals:
```bash
rg --pcre2 "debug[-_ ]?flow|Debug Flow" examples/skills/bug-fix/references/path-convention.md
```
Expected: no output.

- [x] **Step 1.5: Update `examples/skills/bug-fix/references/worktrunk-supplement.md`**

Apply:
1. Line 3: `\`examples/skills/debug-flow/SKILL.md\`` → `\`examples/skills/bug-fix/SKILL.md\``
2. Line 15: `debug-flow Red Flag` → `bug-fix Red Flag`
3. Line 41: `further action in debug-flow` → `further action in bug-fix`

Verify:
```bash
rg --pcre2 "debug[-_ ]?flow|Debug Flow" examples/skills/bug-fix/references/worktrunk-supplement.md
```
Expected: no output.

- [x] **Step 1.6: Update `examples/skills/bug-fix/references/rca-supplement.md`**

Apply:
1. Line 3: `\`examples/skills/debug-flow/SKILL.md\`` → `\`examples/skills/bug-fix/SKILL.md\``
2. Line 39: `debug-flow SKILL.md Red Flag` → `bug-fix SKILL.md Red Flag`

Verify:
```bash
rg --pcre2 "debug[-_ ]?flow|Debug Flow" examples/skills/bug-fix/references/rca-supplement.md
```
Expected: no output.

- [x] **Step 1.7: Update `examples/skills/bug-fix/references/monkey-test-supplement.md`**

Apply:
1. Line 3: `\`examples/skills/debug-flow/SKILL.md\`` → `\`examples/skills/bug-fix/SKILL.md\``
2. Line 7: `In debug-flow, override to:` → `In bug-fix, override to:`

Verify:
```bash
rg --pcre2 "debug[-_ ]?flow|Debug Flow" examples/skills/bug-fix/references/monkey-test-supplement.md
```
Expected: no output.

- [x] **Step 1.8: Update `examples/skills/bug-fix/references/dogfood-supplement.md`**

Apply:
- Line 3: `\`examples/skills/debug-flow/SKILL.md\`` → `\`examples/skills/bug-fix/SKILL.md\``

Verify:
```bash
rg --pcre2 "debug[-_ ]?flow|Debug Flow" examples/skills/bug-fix/references/dogfood-supplement.md
```
Expected: no output.

- [x] **Step 1.9: Update `examples/skills/bug-fix/references/fix-plan-supplement.md`**

Apply:
- Line 3: `\`examples/skills/debug-flow/SKILL.md\`` → `\`examples/skills/bug-fix/SKILL.md\``

Verify:
```bash
rg --pcre2 "debug[-_ ]?flow|Debug Flow" examples/skills/bug-fix/references/fix-plan-supplement.md
```
Expected: no output.

- [x] **Step 1.10: Full in-directory residual scan**

Run:
```bash
rg --pcre2 "debug[-_ ]?flow|Debug Flow|DebugFlow|/debug-flow" examples/skills/bug-fix/
```
Expected: no output. If output exists, fix the remaining occurrence before proceeding.

---

## Task 2: Update belt-agent e2e test fixture

**Files:**
- Modify: `crates/belt-agent/tests/e2e_test.rs:39`

- [x] **Step 2.1: Replace the inline YAML fixture pipeline name**

Apply:
- Line 39: `r#"name: debug-flow` → `r#"name: bug-fix`

Exact context (before):
```rust
    std::fs::write(
        tmp.join("consumer.yml"),
        r#"name: debug-flow
version: 1
phases:
```

After:
```rust
    std::fs::write(
        tmp.join("consumer.yml"),
        r#"name: bug-fix
version: 1
phases:
```

- [x] **Step 2.2: Residual scan**

Run:
```bash
rg --pcre2 "debug[-_ ]?flow|Debug Flow" crates/belt-agent/tests/e2e_test.rs
```
Expected: no output.

---

## Task 3: Rename and update belt-core integration test

**Files:**
- Rename: `crates/belt-core/tests/debug_flow_refresh.rs` → `crates/belt-core/tests/bug_fix_refresh.rs`
- Modify: contents of the renamed file

- [x] **Step 3.1: Rename test file via `git mv`**

Run:
```bash
git mv crates/belt-core/tests/debug_flow_refresh.rs crates/belt-core/tests/bug_fix_refresh.rs
```

- [x] **Step 3.2: Apply replacements in `bug_fix_refresh.rs`**

Apply the six ordered `replace_all` operations below to `crates/belt-core/tests/bug_fix_refresh.rs`. **Order matters** because `debug_flow_pipeline_path` uses `debug_flow_pipeline` as a prefix — replacing the shorter name first would corrupt the longer one.

| # | Old (replace_all) | New | Covers |
|---|-------------------|-----|--------|
| 1 | `debug_flow_pipeline_path` | `bug_fix_pipeline_path` | definition (line 35) + all call sites (lines 40, 181, 198) |
| 2 | `debug_flow_pipeline` | `bug_fix_pipeline` | definition (line 39) + all call sites (lines 56, 71, 82, 89, 113, 140, 161, 209) |
| 3 | `debug_flow_dir` | `bug_fix_dir` | definition (line 31) + all call sites (lines 36, 222, 240, 251, 279, 296) |
| 4 | `examples/skills/debug-flow` | `examples/skills/bug-fix` | path string (line 32), comment (line 267) |
| 5 | `debug-flow pipeline.yml must parse` | `bug-fix pipeline.yml must parse` | expect message (line 40) |
| 6 | `/debug-flow pipeline.` | `/bug-fix pipeline.` | doc-comment (line 1) |

**Line 3 (EXCEPTION — preserve verbatim, do NOT change):**
- `//! Shape contract (spec docs/specs/2026-04-15-debug-flow-refresh-design.md):` stays as-is.

The narrow, non-overlapping match on replacement #6 (`/debug-flow pipeline.`) protects line 3 from accidental rewrite.

- [x] **Step 3.3: Residual scan on the renamed test file**

Run:
```bash
rg --pcre2 "debug[-_ ]?flow|Debug Flow|/debug-flow" crates/belt-core/tests/bug_fix_refresh.rs
```
Expected: only line 3 matches (the historical spec filename reference). Exactly one line of output:
```
3://! Shape contract (spec docs/specs/2026-04-15-debug-flow-refresh-design.md):
```

If any other line matches, fix it before proceeding.

- [x] **Step 3.4: Compile check**

Run:
```bash
cargo check -p belt-core --tests
```
Expected: PASS (no unresolved symbols).

If this fails with `unresolved function \`debug_flow_dir\`` or similar, a call site was missed — revisit Step 3.2.

---

## Task 4: Full verification

- [x] **Step 4.1: Run belt-core renamed test**

Run:
```bash
cargo test -p belt-core --test bug_fix_refresh
```
Expected: all tests PASS.

If `supplement_files_exist` or `criteria_files_exist` fails, the directory rename (Task 1) is incomplete — check `examples/skills/bug-fix/references/` and `examples/skills/bug-fix/criteria/` exist.

- [x] **Step 4.2: Run belt-agent e2e test**

Run:
```bash
cargo test -p belt-agent --test e2e_test
```
Expected: all tests PASS.

- [x] **Step 4.3: Run full workspace tests**

Run:
```bash
cargo test --workspace
```
Expected: all tests PASS.

- [x] **Step 4.4: Run clippy on changed crates** (see report: pre-existing warnings, not caused by this change)

Run:
```bash
cargo clippy -p belt-core --tests -- -D warnings
cargo clippy -p belt-agent --tests -- -D warnings
```
Expected: PASS.

- [x] **Step 4.5: Run fmt check on changed files**

Run:
```bash
cargo fmt -p belt-core -- --check
cargo fmt -p belt-agent -- --check
```
Expected: PASS. If it fails, run without `--check` to fix.

- [x] **Step 4.6: Parse the renamed pipeline with belt-agent**

Run from inside the renamed skill directory (belt-agent looks for `pipeline.yml` in the current working directory):
```bash
(cd examples/skills/bug-fix && cargo run -q -p belt-agent -- init)
```
Expected: JSON output from `belt-agent init` (new `run_id`, initial phase). If it fails with a parse error, revisit Task 1 string replacements.

Clean up the transient run state:
```bash
rm -rf examples/skills/bug-fix/.belt/
```

Note: the actual `belt-agent init` invocation contract (subcommand, flags) is defined by the binary — if the above command fails with an argument-parsing error rather than a YAML parse error, consult `cargo run -q -p belt-agent -- --help` and adjust the invocation; the verification intent is simply that `pipeline.yml` parses.

- [x] **Step 4.7: Repo-wide residual scan in scope A + B**

Run:
```bash
rg --pcre2 "debug[-_ ]?flow|Debug Flow|DebugFlow|/debug-flow" \
  examples/skills/bug-fix \
  crates/belt-core/src \
  crates/belt-core/tests \
  crates/belt-agent/tests
```

Expected output: ONLY the four Exception Rule references below. If anything else appears, fix it.

Exception-Rule allow-list (these MUST remain):
1. `crates/belt-core/src/model.rs:183` — `2026-04-15-debug-flow-refresh-design.md`
2. `crates/belt-core/src/view.rs:236` — `debug-flow refresh spec, 2026-04-15`
3. `crates/belt-core/tests/artifact_when_field.rs:3` — `2026-04-15-debug-flow-refresh-design.md`
4. `crates/belt-core/tests/bug_fix_refresh.rs:3` — `2026-04-15-debug-flow-refresh-design.md`

---

## Task 5: Commit

- [x] **Step 5.1: Stage changes**

Run:
```bash
git status
```
Verify:
- Renamed directory shows as `renamed: examples/skills/debug-flow/... -> examples/skills/bug-fix/...`
- Renamed test file shows as `renamed: crates/belt-core/tests/debug_flow_refresh.rs -> crates/belt-core/tests/bug_fix_refresh.rs`
- Modified files: 8 skill files + 2 test files

Stage explicitly (avoid `git add .`):
```bash
git add examples/skills/bug-fix/
git add crates/belt-core/tests/bug_fix_refresh.rs
git add crates/belt-agent/tests/e2e_test.rs
```

- [x] **Step 5.2: Commit**

Run:
```bash
git commit -m "$(cat <<'EOF'
refactor: rename examples/skills/debug-flow to bug-fix

Rename the debug-flow skill directory and its internal references to
bug-fix. Update live code references in belt-core integration test and
belt-agent e2e test fixture. Historical spec / plan references are
preserved verbatim per the Exception Rule.

Spec: docs/superpowers/specs/2026-04-15-bug-fix-rename-design.md
EOF
)"
```

- [x] **Step 5.3: Verify commit**

Run:
```bash
git log -1 --stat
```
Expected: one commit with renames and modifications listed. Untracked `.claude/agent-memory/` files should NOT be staged.
