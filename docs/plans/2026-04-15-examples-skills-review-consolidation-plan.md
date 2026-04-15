# examples/skills Review Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete `examples/skills/test-review` and `examples/skills/implementation-review` (observation overlap with `code-review` / `spec-review`), insert `spec-review` as feature-dev Phase 3, and switch `bug-fix/fix-plan-review` invoke target from `/implementation-review` to `/spec-review`.

**Architecture:**
- **feature-dev (Tasks 1–2)**: 8→9 phase. New Phase 3 `spec-review` between `test-scenarios` and `plan`. New `criteria/spec-review.md` (audit-lite with 6 criteria). Integration test `feature_dev_refresh.rs` updated (9-phase expected array). SKILL.md description / phase list / Phase-Specific Invocation Rules updated.
- **bug-fix (Tasks 3–4)**: Phase 3 (`fix-plan-review`) invoke target switched to `/spec-review`; phase id and criteria filename unchanged (run-state compatibility). SKILL.md Phase 3 note + Red Flags updated. No integration-test changes required (existing `bug_fix_refresh.rs` does not assert skill invoke name).
- **Cleanup (Tasks 5–6)**: Delete `examples/skills/test-review/` and `examples/skills/implementation-review/` directories (no pipeline references remain after Task 3).
- **Verification (Task 7)**: `cargo test -p belt-core` + `cargo clippy -p belt-core -- -D warnings` + `belt lint` on both pipelines.

**Tech Stack:**
- Rust 1.94.1, Cargo workspace
- `serde-saphyr` (YAML parsing)
- belt-core + belt CLI (for `belt lint`)
- YAML (pipeline definitions) + Markdown (SKILL.md / criteria)

**Spec reference:** `docs/specs/2026-04-15-examples-skills-review-consolidation-design.md` (commit `9ac5a0a`)

---

## File Structure

### Create

- `examples/skills/feature-dev/criteria/spec-review.md` — audit-lite criteria for Phase 3 (Task 1)

### Modify

**belt-core:**
- `crates/belt-core/tests/feature_dev_refresh.rs` — Rename `feature_dev_has_eight_phases` → `feature_dev_has_nine_phases`; insert `"spec-review"` at index 2 of the expected phase-id array (Task 1)

**feature-dev (`examples/skills/feature-dev/`):**
- `pipeline.yml` — Insert Phase 3 `spec-review` block between `test-scenarios` and `plan` (Task 1)
- `SKILL.md` — frontmatter `description` "(8 phases)" → "(9 phases)" and phase list updated; `## Phase-Specific Invocation Rules` gains `### Phase 3: spec-review`; subsequent phase headings renumbered 4→9 (Task 2)

**bug-fix (`examples/skills/bug-fix/`):**
- `pipeline.yml` — `fix-plan-review` phase: `description` text + `invoke.skill` value switched from `/implementation-review` to `/spec-review` (Task 3)
- `SKILL.md` — Phase 3 section body replaced with spec-review note; `## Red Flags` bullet updated (Task 4)

### Delete

- `examples/skills/test-review/` — directory recursively (Task 5)
- `examples/skills/implementation-review/` — directory recursively (Task 6)

### Unchanged (intentional)

- `examples/skills/spec-review/` — used as-is from feature-dev Phase 3 and bug-fix Phase 3
- `examples/skills/code-review/` — no observation additions
- `examples/skills/feature-dev/criteria/test-scenarios.md` — phase numbering inside criteria docs does not include Phase 3 heading
- `examples/skills/bug-fix/criteria/fix-plan-review.md` — criteria content unchanged (skill invoke change only)
- `crates/belt-core/tests/bug_fix_refresh.rs` — existing assertions (phase order, codex passthrough, regate, criteria filenames, SKILL.md sections) remain valid

---

## Verification Commands

Run after Task 7:

```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt

# Rust tests + lint + fmt (belt-core only; criteria/SKILL.md changes do not require rustfmt)
cargo test -p belt-core
cargo clippy -p belt-core -- -D warnings

# Belt lint on both pipelines
cargo run -p belt -- lint examples/skills/feature-dev/pipeline.yml
cargo run -p belt -- lint examples/skills/bug-fix/pipeline.yml
```

Expected: all green. If `belt lint` detects a broken cross-reference (e.g. `validate: ./criteria/spec-review.md` missing), Task 1 was not completed atomically — re-run Task 1 steps.

---

## Tasks

### Task 1: Insert Phase 3 `spec-review` into feature-dev pipeline (atomic)

**Files:**
- Modify: `crates/belt-core/tests/feature_dev_refresh.rs` (lines ~19–37)
- Create: `examples/skills/feature-dev/criteria/spec-review.md`
- Modify: `examples/skills/feature-dev/pipeline.yml` (insert between test-scenarios at L50 and plan at L52)

**Rationale:** All three files must change together so the integration test fails on the first step (TDD red) and passes after pipeline.yml is updated (TDD green). Committing at an intermediate state would leave tests failing in history.

- [ ] **Step 1: Update integration test (TDD red)**

Replace the existing `fn feature_dev_has_eight_phases` (lines 19–37) in `crates/belt-core/tests/feature_dev_refresh.rs` with:

```rust
#[test]
fn feature_dev_has_nine_phases() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;

    let expected: &[&str] = &[
        "design",
        "test-scenarios",
        "spec-review",
        "plan",
        "execute",
        "code-review",
        "monkey-test",
        "dogfood",
        "integrate",
    ];

    let got: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(got, expected, "phase IDs must match spec order");
    Ok(())
}
```

Also update the file header doc comment on line 1:

```rust
//! Integration tests for the refreshed feature-dev pipeline (9 phases).
```

- [ ] **Step 2: Run test to confirm failure**

Run: `cargo test -p belt-core --test feature_dev_refresh feature_dev_has_nine_phases`

Expected: `FAIL` with panic message like `assertion 'left == right' failed ... left: ["design", "test-scenarios", "plan", ...], right: ["design", "test-scenarios", "spec-review", "plan", ...]`.

- [ ] **Step 3: Create `criteria/spec-review.md`**

Create `examples/skills/feature-dev/criteria/spec-review.md` with:

```markdown
---
name: spec-review-done-criteria
audit: lite
phase: spec-review
---

# Phase 3 (spec-review) Done Criteria

- **SREV-01**: `docs/features/<topic>/test-strategy.md` の必須セクション
  (`Test Design Techniques` / `Quality Characteristics` / `Priority Matrix`)
  が spec-review 後も保持されている (TEST-02 と同等の構造)。
- **SREV-02**: spec-review findings の triage が完了している
  (grill-me group と selection group の両方が処理済み、未処理 finding が残っていない)。
- **SREV-03**: user 承認済み findings のみが `test-strategy.md` / `scenarios.yml` に反映されている
  (grill-me: `accept` または `accept_current`、selection: user が番号指定したもののみ)。
- **SREV-04**: 未承認 findings (grill-me `reject` および selection で選択されなかったもの) の
  差分が成果物ファイルに含まれていない。
- **SREV-05**: `args.e2e` が true の場合、`docs/features/<topic>/scenarios.yml` も
  spec-review のレビュー対象に含まれている (findings 内で scenarios が参照されている)。
- **SREV-06**: `test-strategy.md` または `scenarios.yml` を書き換えた場合、
  対応するコミットが作成されている (unstaged 変更が残っていない)。
```

- [ ] **Step 4: Insert Phase 3 block into `pipeline.yml`**

Edit `examples/skills/feature-dev/pipeline.yml`. After the existing `test-scenarios` phase block (which ends at line 50 with `max_retries: 3`), and before the existing `plan` phase block (`  - id: plan` at line 52), insert:

```yaml
  - id: spec-review
    description: "Review test strategy (and scenarios if --e2e) via spec-review"
    invoke:
      skill: /spec-review
      args:
        codex: "args.codex"
    consumes:
      - test_strategy
    validate: ./criteria/spec-review.md
    regate: [test-scenarios]
    confirm: true
    max_retries: 3

```

(Note the trailing blank line so the subsequent `  - id: plan` block is separated per existing style.)

- [ ] **Step 5: Run test to confirm pass (TDD green)**

Run: `cargo test -p belt-core --test feature_dev_refresh`

Expected: all 6 tests in `feature_dev_refresh.rs` pass, including `feature_dev_has_nine_phases`.

- [ ] **Step 6: Run belt lint to confirm pipeline validity**

Run: `cargo run -p belt -- lint examples/skills/feature-dev/pipeline.yml`

Expected: exit 0 with no errors. If the `validate: ./criteria/spec-review.md` cross-reference fails, verify Step 3 created the file at the correct path.

- [ ] **Step 7: Run clippy (integration test touched)**

Run: `cargo clippy -p belt-core --tests -- -D warnings`

Expected: clean, no warnings.

- [ ] **Step 8: Commit**

```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
git add crates/belt-core/tests/feature_dev_refresh.rs \
        examples/skills/feature-dev/criteria/spec-review.md \
        examples/skills/feature-dev/pipeline.yml
git commit -m "$(cat <<'EOF'
feat(feature-dev): insert spec-review as Phase 3

Add spec-review phase between test-scenarios and plan with codex
passthrough, regate to test-scenarios, and new criteria/spec-review.md
(SREV-01..06). Integration test updated for 9-phase expected order.
EOF
)"
```

---

### Task 2: Update feature-dev/SKILL.md for 9 phases

**Files:**
- Modify: `examples/skills/feature-dev/SKILL.md` (frontmatter description, Phase-Specific Invocation Rules section, Phase 4–9 heading renumbering)

- [ ] **Step 1: Update frontmatter description**

Replace lines 3–7 (the `description:` multi-line block). Find:

```yaml
description: >-
  Quality-gated development pipeline (8 phases). Design → test scenarios → plan →
  execute → code review → monkey test (E2E scripted) → dogfood (E2E exploratory) →
  integrate. Web UI testing phases are conditional on --e2e.
```

Replace with:

```yaml
description: >-
  Quality-gated development pipeline (9 phases). Design → test scenarios →
  spec review → plan → execute → code review → monkey test (E2E scripted) →
  dogfood (E2E exploratory) → integrate. Web UI testing phases are conditional
  on --e2e.
```

- [ ] **Step 2: Insert Phase 3 section**

After the existing `### Phase 2: test-scenarios` block and before `### Phase 3: plan` (current line numbering), insert:

```markdown
### Phase 3: spec-review

- **INVOKE**: Skill tool `/spec-review` with `codex` passed through from args.
- Targets `test-strategy.md`. If `scenarios.yml` exists (`args.e2e`), include
  it in the review scope.
- grill-me dialogue for `requirements` / `design-judgment` findings; direct
  selection triage for the remaining observations.
- regate: `test-scenarios`; fix loop capped at `max_retries: 3`.

```

- [ ] **Step 3: Renumber subsequent phase headings 4→9**

Find and replace exact headings (order matters — rename from bottom up to avoid collisions):

| Before | After |
|---|---|
| `### Phase 8: integrate` | `### Phase 9: integrate` |
| `### Phase 7: dogfood (when e2e)` | `### Phase 8: dogfood (when e2e)` |
| `### Phase 6: monkey-test (when e2e)` | `### Phase 7: monkey-test (when e2e)` |
| `### Phase 5: code-review` | `### Phase 6: code-review` |
| `### Phase 4: execute` | `### Phase 5: execute` |
| `### Phase 3: plan` | `### Phase 4: plan` |

Do not modify the body text of these sections (only the heading line changes).

- [ ] **Step 4: Verify no residual "8 phases" references**

Run: `grep -n "8 phases\|Phase 3: plan\|Phase 4: execute\|Phase 5: code-review\|Phase 6: monkey-test\|Phase 7: dogfood\|Phase 8: integrate" examples/skills/feature-dev/SKILL.md`

Expected: no output.

Run: `grep -cn "9 phases\|Phase 3: spec-review\|Phase 4: plan\|Phase 5: execute\|Phase 6: code-review\|Phase 7: monkey-test\|Phase 8: dogfood\|Phase 9: integrate" examples/skills/feature-dev/SKILL.md`

Expected: at least 8 matches (one per renamed or newly-added heading + description).

- [ ] **Step 5: Commit**

```bash
git add examples/skills/feature-dev/SKILL.md
git commit -m "$(cat <<'EOF'
docs(feature-dev): document Phase 3 spec-review in SKILL.md

Update frontmatter description to 9 phases and insert Phase 3 spec-review
invocation rule. Renumber Phase 4–9 headings accordingly.
EOF
)"
```

---

### Task 3: Switch bug-fix fix-plan-review invoke to `/spec-review`

**Files:**
- Modify: `examples/skills/bug-fix/pipeline.yml` (Phase 3 `fix-plan-review` block, lines 50–60 area)

- [ ] **Step 1: Edit `pipeline.yml` Phase 3 block**

In `examples/skills/bug-fix/pipeline.yml`, find the `fix-plan-review` phase block:

```yaml
  - id: fix-plan-review
    description: "Plan review via implementation-review"
    invoke:
      skill: /implementation-review
      args:
        codex: "args.codex"
    consumes:
      - fix_plan_doc
    validate: ./criteria/fix-plan-review.md
    confirm: true
    max_retries: 3
```

Replace the two changed lines to get:

```yaml
  - id: fix-plan-review
    description: "Plan review via spec-review"
    invoke:
      skill: /spec-review
      args:
        codex: "args.codex"
    consumes:
      - fix_plan_doc
    validate: ./criteria/fix-plan-review.md
    confirm: true
    max_retries: 3
```

(`id`, `consumes`, `validate`, `confirm`, `max_retries` are unchanged.)

- [ ] **Step 2: Run integration test**

Run: `cargo test -p belt-core --test bug_fix_refresh`

Expected: all 13 tests pass. Existing assertions check phase order, codex passthrough presence, regate shape, criteria filenames, and SKILL.md sections — none assert the invoke skill name, so Step 1's change should not break them.

- [ ] **Step 3: Run belt lint**

Run: `cargo run -p belt -- lint examples/skills/bug-fix/pipeline.yml`

Expected: exit 0, no errors.

- [ ] **Step 4: Commit**

```bash
git add examples/skills/bug-fix/pipeline.yml
git commit -m "$(cat <<'EOF'
refactor(bug-fix): switch fix-plan-review invoke to /spec-review

Drop dependency on /implementation-review in favor of /spec-review
(observation overlap resolved per consolidation design). Phase id,
criteria filename, and all assertions preserved.
EOF
)"
```

---

### Task 4: Update bug-fix/SKILL.md for spec-review delegation

**Files:**
- Modify: `examples/skills/bug-fix/SKILL.md` (Phase 3 section lines ~37–40, Red Flags line ~75)

- [ ] **Step 1: Replace Phase 3 section body**

Find the existing Phase 3 block (around lines 37–39):

```markdown
### Phase 3: fix-plan-review

- **INVOKE**: Skill tool `/implementation-review` with `codex` passed through.
- No supplement required; the skill is self-contained.
```

Replace with:

```markdown
### Phase 3: fix-plan-review

- **INVOKE**: Skill tool `/spec-review` with `codex` passed through.
- No supplement required; the skill is self-contained.
- Note: spec-review を fix-plan レビューに流用する。`design-judgment` 観点の
  grill-me は原則発動しない (設計判断は rca / fix-plan で決定済みのため)。
  発動した場合は上流 (rca / fix-plan) の見直しサインとして扱う。
```

- [ ] **Step 2: Update Red Flags bullet**

Find the line (around line 75):

```markdown
- **Never filter or omit review findings**: `/code-review`, `/implementation-review` の triage は user 責務.
```

Replace with:

```markdown
- **Never filter or omit review findings**: `/code-review`, `/spec-review` の triage は user 責務.
```

- [ ] **Step 3: Verify no residual `/implementation-review` references**

Run: `grep -n "implementation-review" examples/skills/bug-fix/SKILL.md`

Expected: no output.

- [ ] **Step 4: Run integration test (SKILL.md section assertions)**

Run: `cargo test -p belt-core --test bug_fix_refresh skill_md`

Expected: `skill_md_has_expected_sections` and `skill_md_declares_supplement_injection_per_phase` both pass. (The test asserts section headers and supplement filenames, not the specific invoke skill.)

- [ ] **Step 5: Commit**

```bash
git add examples/skills/bug-fix/SKILL.md
git commit -m "$(cat <<'EOF'
docs(bug-fix): update SKILL.md for spec-review delegation

Rewrite Phase 3 INVOKE line and add note about design-judgment grill-me
behavior when /spec-review is applied to fix-plan docs. Update Red Flags
bullet to reference /spec-review instead of /implementation-review.
EOF
)"
```

---

### Task 5: Remove `examples/skills/test-review/`

**Files:**
- Delete: `examples/skills/test-review/` (directory — `SKILL.md`, `pipeline.yml`, `belt.toml`)

- [ ] **Step 1: Confirm no remaining references**

Run: `grep -rn "test-review" examples/skills/ crates/ docs/specs/ docs/plans/ 2>/dev/null | grep -v "docs/specs/2026-04-15-examples-skills-review-consolidation-design.md\|docs/plans/2026-04-15-examples-skills-review-consolidation-plan.md\|examples/skills/test-review/"`

Expected: no output (any hit outside the spec/plan docs or the target directory itself indicates a missed reference to fix before deletion).

- [ ] **Step 2: Remove directory**

Run: `git rm -r examples/skills/test-review/`

Expected: git reports deletion of 3 files (`SKILL.md`, `pipeline.yml`, `belt.toml`).

- [ ] **Step 3: Commit**

```bash
git commit -m "$(cat <<'EOF'
chore: drop examples/skills/test-review

Observations (coverage, quality) overlap with code-review/test;
design-alignment (requirement_map) is upstream-covered by spec-review
after test-scenarios. No pipeline references remain.
EOF
)"
```

---

### Task 6: Remove `examples/skills/implementation-review/`

**Files:**
- Delete: `examples/skills/implementation-review/` (directory — `SKILL.md`, `pipeline.yml`, `belt.toml`)

- [ ] **Step 1: Confirm no remaining references**

Run: `grep -rn "implementation-review" examples/skills/ crates/ docs/specs/ docs/plans/ 2>/dev/null | grep -v "docs/specs/2026-04-15-examples-skills-review-consolidation-design.md\|docs/plans/2026-04-15-examples-skills-review-consolidation-plan.md\|examples/skills/implementation-review/"`

Expected: no output. Task 3 and Task 4 already replaced pipeline + SKILL.md references in bug-fix.

- [ ] **Step 2: Remove directory**

Run: `git rm -r examples/skills/implementation-review/`

Expected: git reports deletion of 3 files.

- [ ] **Step 3: Commit**

```bash
git commit -m "$(cat <<'EOF'
chore: drop examples/skills/implementation-review

Observations (feasibility, consistency, ui-spec) overlap with spec-review;
clarity is absorbed by spec-review consistency + feasibility. Last caller
(bug-fix/fix-plan-review) was switched to /spec-review in prior commit.
EOF
)"
```

---

### Task 7: Final verification (no commit)

**Files:** (verification only — no changes)

- [ ] **Step 1: Full belt-core test suite**

Run: `cargo test -p belt-core`

Expected: all tests pass (including `feature_dev_refresh`, `bug_fix_refresh`, and any unrelated tests).

- [ ] **Step 2: Clippy clean**

Run: `cargo clippy -p belt-core -- -D warnings`

Expected: exit 0, no warnings.

- [ ] **Step 3: Formatter check (sanity — no Rust source modified)**

Run: `cargo fmt --package belt-core --check`

Expected: exit 0.

- [ ] **Step 4: Belt lint both pipelines**

Run:
```bash
cargo run -p belt -- lint examples/skills/feature-dev/pipeline.yml
cargo run -p belt -- lint examples/skills/bug-fix/pipeline.yml
```

Expected: both exit 0.

- [ ] **Step 5: Confirm examples/skills final shape**

Run: `ls examples/skills/`

Expected output exactly (alphabetical):

```
bug-fix
code-review
feature-dev
monkey-test
spec-review
test-scenarios
```

If either `test-review/` or `implementation-review/` still appears, Task 5 / Task 6 was incomplete.

- [ ] **Step 6: Confirm commit history**

Run: `git log --oneline -7`

Expected: 6 new commits on top of `9ac5a0a` (the design doc commit), matching the order of Tasks 1–6.

---

## Rollback

If Task 1 is committed but a later task fails verification, the safest rollback is per-task revert:

```bash
# Example: revert Task 6 only
git revert HEAD --no-edit
```

Do not `git reset --hard` unless the working tree has no other uncommitted work.

## Notes for Executor

- **spec-review SKILL.md is intentionally not modified.** Its current `grill-me` triage (requirements / design-judgment high/medium) is acceptable for both test-strategy review (feature-dev Phase 3) and fix-plan review (bug-fix Phase 3); plan-domain grill-me is expected to be rare.
- **Phase id `fix-plan-review` in bug-fix is unchanged.** This preserves belt run-state compatibility and avoids touching `criteria/fix-plan-review.md`.
- **No changes to `code-review` skill.** The design explicitly rules out importing test-review's `requirement_map` observation into code-review.
- **All commits should pass pre-commit hooks** (see `CLAUDE.md`: cargo fmt / cargo clippy / cargo test for the touched crate). Only Task 1 touches Rust source (the integration test); Tasks 2, 4 touch Markdown only; Tasks 3 touches YAML only; Tasks 5, 6 are pure deletions.
