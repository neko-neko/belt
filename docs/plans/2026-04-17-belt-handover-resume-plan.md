# belt Handover & Resume Skills Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** belt pipeline の session 跨ぎ pause / resume を `/belt:handover` / `/belt:resume` 2 skills で実現し、feature-dev / bug-fix の execute 直前に `pre-execute-handover` checkpoint phase を挿入する。

**Architecture:** 新規 artifact は `.belt/runs/<id>/handover.md` 1 本のみ (frontmatter 3 field + Resume hint 3 項目、既存 `state.json` との二重管理回避)。`/belt:resume` は current worktree の最新 run のみを対象、args string `resume run_id=<id>` で owning pipeline skill を invoke、protocol driver は `plugins/belt-agent/skills/protocol/references/resume-mode.md` に従って `belt-agent init` を skip する。pipeline skill (feature-dev / bug-fix / debug-flow) の SKILL.md には handover / resume 概念を浸透させない (SSOT は新 skill 側)。

**Tech Stack:** Markdown (SKILL.md / references), YAML (pipeline.yml の 1 phase 追加), Rust integration tests (belt-core refresh tests の phase count 更新のみ、production code 変更なし)。

**Spec:** `docs/specs/2026-04-17-belt-handover-resume-design.md`

**Execution note:** dedicated worktree 推奨 (`wt switch -c feature/2026-04-17-belt-handover-resume`)。memory `project_parallel_session_worktree_isolation.md` の並行 session 散在リスク回避のため。

---

## Plan-wide Invariants

- **Commit scope**: 各 Phase 末で 1 commit。Phase 内の step は uncommitted 扱い。
- **Test execution**:
  - belt-core の refresh test を touch する Phase A のみ `cargo test -p belt-core --test feature_dev_refresh` / `cargo test -p belt-core --test bug_fix_refresh` を走らせる。
  - 最終 Phase F で `cargo clippy --workspace -- -D warnings && cargo test --workspace` を 1 回実行。
- **TDD order for Phase A**: 先に refresh test の expected phase list を 10 / 9 phases へ更新し、赤確認 → pipeline.yml に phase 追加 → 緑確認 → commit。
- **Skill body は ≤500 行 / 用語統一**: `handover.md` / `handover` / `resume` / `Resume hint` を spec と同一用法で使う。
- **Phase skip disable**: `pre-execute-handover` phase は `when:` を付けない (MVP default-on 固定、spec Open Questions に記載済み)。

---

## File Structure

### Modify (existing)

- `plugins/belt/skills/feature-dev/pipeline.yml` — `plan` と `execute` の間に `pre-execute-handover` phase を挿入
- `plugins/belt/skills/bug-fix/pipeline.yml` — `fix-plan-review` と `execute` の間に `pre-execute-handover` phase を挿入 (spec 記述の "fix-plan と execute の間" を文字通り execute 直前と解釈)
- `plugins/belt-agent/skills/protocol/SKILL.md` — `## Decision Rules` の末尾に 1 行で resume-mode.md pointer を追加
- `plugins/belt-agent/references/narrative-convention.md` — L7 の `/clear` 表現に `or /belt:resume` を追記
- `plugins/belt/skills/feature-dev/references/brainstorming-supplement.md` — L105 の `; resume from handover if set` を削除
- `plugins/belt/.claude-plugin/plugin.json` — `description` に `/belt:handover`, `/belt:resume` を列挙
- `crates/belt-core/tests/feature_dev_refresh.rs` — `feature_dev_has_nine_phases` test を 10 phases expectation に更新 + narrative phase list との整合維持
- `crates/belt-core/tests/bug_fix_refresh.rs` — `phase_count_and_order` test を 9 phases expectation に更新

### Create (new)

- `plugins/belt/skills/handover/SKILL.md` — /belt:handover skill 本体
- `plugins/belt/skills/resume/SKILL.md` — /belt:resume skill 本体
- `plugins/belt-agent/skills/protocol/references/resume-mode.md` — resume mode driver reference

### Delete

なし。

---

## Phase A: pipeline.yml 拡張 + refresh test 追従

### Task A1: Update feature_dev_refresh test to expect 10 phases (RED)

**Files:**
- Modify: `crates/belt-core/tests/feature_dev_refresh.rs:29-46` (`feature_dev_has_nine_phases`)

- [ ] **Step 1: Rename test function and update expected phase list**

`crates/belt-core/tests/feature_dev_refresh.rs` の L1 コメントと該当関数を以下のように変更:

ファイル先頭 L1:
```rust
//! Integration tests for the refreshed feature-dev pipeline (10 phases).
```

L29 付近:
```rust
#[test]
fn feature_dev_has_ten_phases() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;

    let expected: &[&str] = &[
        "design",
        "test-scenarios",
        "spec-review",
        "plan",
        "pre-execute-handover",
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

- [ ] **Step 2: Run test to verify RED**

Run:
```bash
cargo test -p belt-core --test feature_dev_refresh feature_dev_has_ten_phases
```

Expected: FAIL — `assert_eq` mismatch because pipeline.yml still has 9 phases without `pre-execute-handover`.

### Task A2: Insert pre-execute-handover into feature-dev/pipeline.yml (GREEN)

**Files:**
- Modify: `plugins/belt/skills/feature-dev/pipeline.yml:89` (直後に insert)

- [ ] **Step 1: Insert the new phase between `plan` (ends at L89) and `execute` (starts at L91)**

`plugins/belt/skills/feature-dev/pipeline.yml` の L89 の `max_retries: 3` 直後、L91 の `- id: execute` 直前に以下を挿入:

```yaml

  - id: pre-execute-handover
    description: >-
      Context checkpoint before execute. Run `/belt:handover`, then `/clear`,
      then `/belt:resume` in a new session. The gate passes once the handover
      note exists under `.belt/runs/{run_id}/`.
    confirm: true
    gate:
      - file_exists: ".belt/runs/{run_id}/handover.md"
```

(先頭に空行 1 つ、その後 2 スペースインデントで phase 定義。既存 phase のインデントに合わせる。)

- [ ] **Step 2: Run test to verify GREEN**

Run:
```bash
cargo test -p belt-core --test feature_dev_refresh feature_dev_has_ten_phases
```

Expected: PASS — `test feature_dev_has_ten_phases ... ok`

- [ ] **Step 3: Run the whole refresh test file to confirm no regression**

Run:
```bash
cargo test -p belt-core --test feature_dev_refresh
```

Expected: すべての test が PASS。narrative-producing phase list (L237-269) は `pre-execute-handover` を含まないので影響なし。

### Task A3: Update bug_fix_refresh test to expect 9 phases (RED)

**Files:**
- Modify: `crates/belt-core/tests/bug_fix_refresh.rs:5-10` (top docstring)
- Modify: `crates/belt-core/tests/bug_fix_refresh.rs:89` 付近 (`phase_count_and_order`)

- [ ] **Step 1: Update top doc comment**

L5-L7 付近を以下のように書き換え:

```rust
//! - 9 phases: rca → fix-plan → fix-plan-review → pre-execute-handover → execute →
//!   code-review → monkey-test → dogfood → integrate
```

- [ ] **Step 2: Update `phase_count_and_order` expected list**

`phase_count_and_order` test の `expected` 配列に `"pre-execute-handover"` を `"fix-plan-review"` と `"execute"` の間に挿入:

```rust
#[test]
fn phase_count_and_order() {
    let pipeline = parse_pipeline_file();
    let actual: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
    let expected = vec![
        "rca",
        "fix-plan",
        "fix-plan-review",
        "pre-execute-handover",
        "execute",
        "code-review",
        "monkey-test",
        "dogfood",
        "integrate",
    ];
    assert_eq!(actual, expected, "phase IDs must match spec order");
}
```

- [ ] **Step 3: Run test to verify RED**

Run:
```bash
cargo test -p belt-core --test bug_fix_refresh phase_count_and_order
```

Expected: FAIL — `assert_eq` mismatch.

### Task A4: Insert pre-execute-handover into bug-fix/pipeline.yml (GREEN)

**Files:**
- Modify: `plugins/belt/skills/bug-fix/pipeline.yml:69` (直後に insert)

- [ ] **Step 1: Insert the new phase between `fix-plan-review` (L59-69) and `execute` (starts at L71)**

`plugins/belt/skills/bug-fix/pipeline.yml` の L69 `max_retries: 3` 直後、L71 `- id: execute` 直前に以下を挿入:

```yaml

  - id: pre-execute-handover
    description: >-
      Context checkpoint before execute. Run `/belt:handover`, then `/clear`,
      then `/belt:resume` in a new session. The gate passes once the handover
      note exists under `.belt/runs/{run_id}/`.
    confirm: true
    gate:
      - file_exists: ".belt/runs/{run_id}/handover.md"
```

- [ ] **Step 2: Run test to verify GREEN**

Run:
```bash
cargo test -p belt-core --test bug_fix_refresh phase_count_and_order
```

Expected: PASS.

- [ ] **Step 3: Run all bug_fix_refresh tests**

Run:
```bash
cargo test -p belt-core --test bug_fix_refresh
```

Expected: すべての test が PASS。`all_phases_use_skill_invoke` は新 phase に `invoke` がないので false negative になりうるため次 step で確認する。

- [ ] **Step 4: Verify `all_phases_use_skill_invoke` behavior for the checkpoint phase**

`pre-execute-handover` は `invoke` を持たない (pure checkpoint phase、protocol SKILL.md の「If `invoke` is absent, the phase is a "pure checkpoint"」パターン)。`all_phases_use_skill_invoke` test がこれを強制する場合は failing になる。その時は test を「`invoke` を持つ phase は skill variant でなければならない」に緩和する必要がある。

Run:
```bash
cargo test -p belt-core --test bug_fix_refresh all_phases_use_skill_invoke
```

Expected: PASS 想定だが、FAIL した場合は test を読み、`phase.invoke.is_some()` の phase のみループする条件付きガードに書き換える。feature_dev_refresh.rs にも同種 test があれば同様に対処。

- [ ] **Step 5: Same check for feature_dev_refresh**

Run:
```bash
cargo test -p belt-core --test feature_dev_refresh
```

Expected: PASS。FAIL があれば step 4 と同じ対処。

### Task A5: Commit Phase A

- [ ] **Step 1: Stage changes and commit**

Run:
```bash
git add plugins/belt/skills/feature-dev/pipeline.yml \
        plugins/belt/skills/bug-fix/pipeline.yml \
        crates/belt-core/tests/feature_dev_refresh.rs \
        crates/belt-core/tests/bug_fix_refresh.rs
git commit -m "$(cat <<'EOF'
feat(pipelines): add pre-execute-handover checkpoint phase

Insert a pure checkpoint phase between plan/fix-plan-review and execute
in feature-dev and bug-fix pipelines. Description instructs the LLM to
prompt /belt:handover, /clear, and /belt:resume before executing; gate
file_exists .belt/runs/{run_id}/handover.md blocks advance until the
handover note is written.

Update refresh tests for the new 10/9 phase counts.
EOF
)"
```

---

## Phase B: Resume mode reference + protocol SKILL.md pointer

### Task B1: Create `protocol/references/resume-mode.md`

**Files:**
- Create: `plugins/belt-agent/skills/protocol/references/resume-mode.md`

- [ ] **Step 1: Write the reference file**

Create `plugins/belt-agent/skills/protocol/references/resume-mode.md`:

````markdown
# Resume Mode

Driver-side behavior when the belt protocol receives a resume invocation.

## Detection

If the Skill invoke `args` string contains the literal prefix
`resume run_id=` followed by a UUIDv7 run identifier, the protocol
driver is in **resume mode**. Example args string:

```
resume run_id=01947abc-0000-7000-8000-000000000000
```

Exact format; do not accept synonyms (`resume_run=`, `run=`, etc.).

## Steps

1. Do not run `belt-agent init`. The run already exists on disk.
2. Run `belt-agent status --run <id>` to read the current phase.
3. If `current_phase == "COMPLETED"`, report back to the caller and stop.
   (The caller `/belt:resume` is expected to catch this first via its
   precondition; this is a defensive second check.)
4. If `.belt/runs/<id>/handover.md` exists, read it and incorporate the
   `## Resume hint` section into the LLM context before resuming normal
   workflow.
5. Continue with the normal protocol loop: `belt-agent next --run <id>` →
   `verify --run <id>` / `regate --run <id>` / `step --run <id>` as usual.

## Cross-pipeline applicability

Resume mode is defined once here and applies to every pipeline driven by
this protocol. No per-pipeline SKILL.md change is needed.
````

- [ ] **Step 2: Verify the file was written correctly**

Run:
```bash
wc -l plugins/belt-agent/skills/protocol/references/resume-mode.md
```

Expected: 25 〜 30 行程度。

### Task B2: Add pointer line to protocol SKILL.md

**Files:**
- Modify: `plugins/belt-agent/skills/protocol/SKILL.md` の `## Decision Rules` 表の末尾、もしくは `## Step Troubleshooting` の前

- [ ] **Step 1: Insert the pointer line**

`plugins/belt-agent/skills/protocol/SKILL.md` L101 (`## Decision Rules` セクション末尾) の `regate is an in-place re-verification ...` 段落の後、`## Step Troubleshooting` の前に以下を挿入:

```markdown

When invoked with `resume run_id=<id>` args, follow [`./references/resume-mode.md`](./references/resume-mode.md).
```

(先頭に空行 1 つを置く。)

- [ ] **Step 2: Verify the line was inserted at the right place**

Run:
```bash
grep -n "resume run_id" plugins/belt-agent/skills/protocol/SKILL.md
```

Expected: 1 行だけヒット (新規追加行)。

### Task B3: Commit Phase B

- [ ] **Step 1: Stage and commit**

Run:
```bash
git add plugins/belt-agent/skills/protocol/SKILL.md \
        plugins/belt-agent/skills/protocol/references/resume-mode.md
git commit -m "$(cat <<'EOF'
feat(protocol): add resume-mode reference for /belt:resume args

Introduce references/resume-mode.md describing how the protocol driver
behaves when Skill args contain `resume run_id=<id>`. Link from protocol
SKILL.md with a single pointer line; the reference is the SSOT and
applies to every pipeline driven by this protocol.
EOF
)"
```

---

## Phase C: /belt:handover skill

### Task C1: Create handover SKILL.md

**Files:**
- Create: `plugins/belt/skills/handover/SKILL.md`

- [ ] **Step 1: Write the skill file**

Create `plugins/belt/skills/handover/SKILL.md`:

````markdown
---
name: handover
description: >-
  Writes a handover note (Resume hint) under the current belt run directory
  so a later session can pick up where the pipeline was paused. Use when
  pausing a multi-phase belt pipeline (feature-dev, bug-fix, debug-flow)
  before /clear or session end, or when the user invokes /belt:handover.
user-invocable: true
---

# /belt:handover

Save a Resume hint so a later session can resume an in-progress belt
pipeline run.

## Overview

When a belt pipeline run is in progress, `/belt:handover` writes
`.belt/runs/<run_id>/handover.md` in the current worktree. The note captures
why the pause happened and what the resumed session should do first. All
other pipeline state (run_id, pipeline, branch, current_phase, pipeline_file)
is already kept in `state.json` — this skill saves only the transient context
that would otherwise be lost.

## Workflow

```
Handover Progress:
- [ ] Step 1: Verify belt-agent is on PATH
- [ ] Step 2: Verify cwd is inside a git worktree
- [ ] Step 3: Query latest run via `belt-agent status`
- [ ] Step 4: Draft the Resume hint (Pause reason / First action / Transient context)
- [ ] Step 5: Write .belt/runs/<run_id>/handover.md
- [ ] Step 6: Tell the user "Handover written. Run /clear then /belt:resume to continue."
```

### Step detail

1. If `command -v belt-agent` fails, abort with `"belt-agent CLI not found. Install or fix PATH."`
2. If `git rev-parse --git-dir` fails, abort with `"Not inside a git worktree; belt handover requires a git-based pipeline workspace."`
3. Run `belt-agent status` with no `--run` flag. Parse the JSON for `run_id` and `branch` (the two fields written to frontmatter). If no run is returned, abort with `"No belt run in progress; nothing to hand over."`
4. Draft the Resume hint in the LLM's head. Three bullets, each 1–3 lines:
   - **Pause reason**: why stopping here now (context heat, end of day, waiting on a decision)
   - **First action on resume**: the very next concrete step (for example: run `belt-agent status`, read `phase-plan.md`, start execute Task 1)
   - **Transient context**: anything said verbally or decided that is not yet written to `state.json` or `phase-*.md`
5. Write the file at `.belt/runs/<run_id>/handover.md` with the schema below. Overwrite if it exists.
6. Emit a single-line confirmation to the user: `Handover written. Run /clear then /belt:resume to continue.`

## Schema

```markdown
---
run_id: <uuid-v7>
branch: <branch captured in step 3>
created_at: <ISO 8601 UTC, e.g. 2026-04-17T18:23:44Z>
---

## Resume hint

- **Pause reason**: <one or two sentences>
- **First action on resume**: <concrete next step>
- **Transient context**: <context not in state.json or phase-*.md>
```

### Frontmatter rules

- Exactly three fields: `run_id`, `branch`, `created_at`. No additions.
- `run_id` matches the run directory name.
- `branch` is the branch at handover time (`git rev-parse --abbrev-ref HEAD` or the `branch` field from `belt-agent status`).
- `created_at` is UTC ISO 8601 with a `Z` suffix.

### Body rules

- Exactly one section: `## Resume hint` with exactly three bullet items.
- Do **not** describe phase progress. That is the role of `notes/phase-<id>.md`. Overlap between the two files is forbidden.

## Red Flags

- **Never write phase-level narrative in handover.md** — use `notes/phase-<id>.md` for phase records.
- **Never add fields to the frontmatter** — the three listed are the contract; extra fields are a future-compatibility hazard.
- **Never run /belt:handover outside a belt run** — if `belt-agent status` returns no run, abort with a clear message.

## References

- `plugins/belt-agent/skills/protocol/references/resume-mode.md` — driver-side resume handling
- `plugins/belt-agent/references/narrative-convention.md` — phase narrative note convention (separate file, separate role)
````

- [ ] **Step 2: Verify file and size**

Run:
```bash
wc -l plugins/belt/skills/handover/SKILL.md
```

Expected: 80 〜 130 行程度 (500 行制限に余裕)。

### Task C2: Commit Phase C

- [ ] **Step 1: Stage and commit**

Run:
```bash
git add plugins/belt/skills/handover/SKILL.md
git commit -m "$(cat <<'EOF'
feat(skills): add /belt:handover skill

New user-facing skill that writes .belt/runs/<id>/handover.md with a
Resume hint (Pause reason / First action / Transient context). Only
this transient information is new — run_id, pipeline, branch, and
current_phase stay canonical in state.json.
EOF
)"
```

---

## Phase D: /belt:resume skill

### Task D1: Create resume SKILL.md

**Files:**
- Create: `plugins/belt/skills/resume/SKILL.md`

- [ ] **Step 1: Write the skill file**

Create `plugins/belt/skills/resume/SKILL.md`:

````markdown
---
name: resume
description: >-
  Resumes a previously handed-over belt pipeline run by reading handover.md
  and state.json from the current worktree, verifying preconditions, and
  invoking the owning pipeline skill in resume mode via Skill tool args.
  Use when continuing belt pipeline work after /belt:handover and /clear,
  when the user invokes /belt:resume, or after a session restart to pick up
  an in-progress run in the current worktree.
user-invocable: true
---

# /belt:resume

Resume an in-progress belt pipeline run from a handover note.

## Overview

`/belt:resume` is the counterpart of `/belt:handover`. It reads the latest
run's `handover.md`, loads the Resume hint, and invokes the owning pipeline
skill (feature-dev, bug-fix, etc.) in resume mode. The pipeline's protocol
driver sees the `resume run_id=<id>` args and follows `resume-mode.md`,
which skips `belt-agent init` and jumps to `belt-agent status --run <id>`.

## Workflow

```
Resume Progress:
- [ ] Step 1: Run preconditions #1..#5 (short-circuit on first failure)
- [ ] Step 2: Read .belt/runs/<run_id>/handover.md; load Resume hint into context
- [ ] Step 3: Invoke Skill(skill="belt:<pipeline>", args="resume run_id=<id>")
- [ ] Step 4: Protocol driver follows resume-mode.md (init is skipped)
```

## Preconditions

| # | Check | Failure message | Recovery |
|---|---|---|---|
| 1 | `command -v belt-agent` succeeds | `"belt-agent CLI not installed or not on PATH"` | install / fix PATH / abort |
| 2 | `belt-agent status` returns a latest run (exit 0) | `"No belt runs found in current directory"` | `cd <worktree>` then rerun / abort |
| 3 | `.belt/runs/<run_id>/handover.md` exists | `"No handover note for latest run. Run /belt:handover first."` | run `/belt:handover` first / abort |
| 4 | `current_phase != "COMPLETED"` | `"Last run already completed. Start a new run?"` | `belt-agent init` / abort |
| 5 | current branch (`git rev-parse --abbrev-ref HEAD`) equals handover.md `branch` | `"Branch changed A→B since handover. Continue anyway? (y/N)"` | `y` proceeds / `N` aborts / `git checkout <branch>` then rerun |

Preconditions run in order; the first failure stops the flow. #4 is
informational (COMPLETED runs are a valid state, not a fault). #5 is a
warning — the user decides whether to proceed.

## Skill args format

```
Skill(
  skill="belt:feature-dev",
  args="resume run_id=01947abc-0000-7000-8000-000000000000"
)
```

Literal prefix `resume ` followed by `run_id=<uuid>`. The protocol driver
detects this shape and follows `resume-mode.md`; extra tokens are not
permitted and will not be detected as resume mode.

The `<pipeline>` segment of `skill="belt:<pipeline>"` comes from
`state.json`'s `pipeline` field — read it through `belt-agent status` before
constructing the Skill call.

## Error Recovery

When a precondition fails, show the exact message and the available options
below it. Never auto-execute recovery actions; the user keeps control over
the working tree.

| Precondition | Options shown to user |
|---|---|
| #1 | (a) install belt-agent, (b) fix PATH, (c) abort |
| #2 | (a) `cd <correct worktree>` then rerun, (b) abort |
| #3 | (a) run `/belt:handover` first then rerun, (b) abort |
| #4 | (a) start a new run via `belt-agent init <pipeline.yml>`, (b) abort |
| #5 | (a) `y` to proceed on current branch, (b) `N` / abort, (c) `git checkout <branch>` then rerun |

## Red Flags

- **Never skip preconditions** — they are the contract; short-circuiting defeats fail-loud.
- **Never pass additional args beyond `resume run_id=<id>`** — the protocol driver's detection is narrow by design.
- **Never auto-cd or auto-checkout** — the user keeps control over working tree and branch.
- **Never assume the owning pipeline skill re-init automatically** — resume mode deliberately suppresses `belt-agent init` via the reference.

## References

- `plugins/belt-agent/skills/protocol/references/resume-mode.md` — driver-side behavior under `resume run_id=<id>` args
- `plugins/belt/skills/handover/SKILL.md` — the counterpart writer
````

- [ ] **Step 2: Verify file and size**

Run:
```bash
wc -l plugins/belt/skills/resume/SKILL.md
```

Expected: 100 〜 150 行 (500 行制限に余裕)。

### Task D2: Commit Phase D

- [ ] **Step 1: Stage and commit**

Run:
```bash
git add plugins/belt/skills/resume/SKILL.md
git commit -m "$(cat <<'EOF'
feat(skills): add /belt:resume skill

Restores a handed-over belt pipeline run by reading handover.md +
state.json from the current worktree, running 5 fail-loud preconditions
(PATH / latest run / handover note / not COMPLETED / branch match),
then invoking the owning pipeline skill with `resume run_id=<id>` args.
The protocol driver follows resume-mode.md to skip init.
EOF
)"
```

---

## Phase E: Existing doc updates + plugin.json

### Task E1: Update narrative-convention.md L7

**Files:**
- Modify: `plugins/belt-agent/references/narrative-convention.md:7`

- [ ] **Step 1: Extend `/clear` phrase**

L7 の以下を:

```markdown
After the user resets session context with `/clear`, reading the narrative note restores each phase's decisions, concerns, directives, and observations. Domain artifacts (`design.md`, `plan.md`, `rca-report.md`, etc.) record **what was produced**, while narrative notes record **why the call was made, what remains unresolved, and what the next phase must assume**.
```

以下に変更:

```markdown
After the user resets session context with `/clear` or `/belt:resume`, reading the narrative note restores each phase's decisions, concerns, directives, and observations. Domain artifacts (`design.md`, `plan.md`, `rca-report.md`, etc.) record **what was produced**, while narrative notes record **why the call was made, what remains unresolved, and what the next phase must assume**.
```

(`with /clear` → `with /clear or /belt:resume` の 1 箇所のみ。L49 / L88 の `/clear` は canonical example として据え置き。)

- [ ] **Step 2: Verify the single edit**

Run:
```bash
grep -n "/clear or /belt:resume\|with /clear" plugins/belt-agent/references/narrative-convention.md
```

Expected: L7 に `/clear or /belt:resume` が 1 箇所ヒットし、L49 / L88 の `/clear` は変更前のまま。

### Task E2: Update brainstorming-supplement.md L105

**Files:**
- Modify: `plugins/belt/skills/feature-dev/references/brainstorming-supplement.md:105`

- [ ] **Step 1: Remove the "resume from handover" phrase**

L105 の以下:

```markdown
   (base = current branch at Phase 1 start; resume from handover if set).
```

を以下に変更:

```markdown
   (base = current branch at Phase 1 start).
```

(`; resume from handover if set` の 28 文字を削除するのみ。)

- [ ] **Step 2: Verify the edit**

Run:
```bash
grep -n "resume from handover" plugins/belt/skills/feature-dev/references/brainstorming-supplement.md
```

Expected: 0 件。

### Task E3: Update plugin.json description

**Files:**
- Modify: `plugins/belt/.claude-plugin/plugin.json:3`

- [ ] **Step 1: Extend description to list new skills**

現在:

```json
  "description": "User-invocable skills and their reviewer agents: /belt:feature-dev, /belt:bug-fix, /belt:code-review (4 reviewers), /belt:spec-review (3 reviewers), /belt:monkey-test, /belt:test-scenarios. Requires belt-agent plugin",
```

以下に変更:

```json
  "description": "User-invocable skills and their reviewer agents: /belt:feature-dev, /belt:bug-fix, /belt:code-review (4 reviewers), /belt:spec-review (3 reviewers), /belt:monkey-test, /belt:test-scenarios, /belt:handover, /belt:resume. Requires belt-agent plugin",
```

(末尾の `, /belt:test-scenarios. Requires` を `, /belt:test-scenarios, /belt:handover, /belt:resume. Requires` に差し替え。)

- [ ] **Step 2: Verify JSON is still valid**

Run:
```bash
python3 -c "import json; json.load(open('plugins/belt/.claude-plugin/plugin.json'))" && echo OK
```

Expected: `OK`。

### Task E4: Commit Phase E

- [ ] **Step 1: Stage and commit**

Run:
```bash
git add plugins/belt-agent/references/narrative-convention.md \
        plugins/belt/skills/feature-dev/references/brainstorming-supplement.md \
        plugins/belt/.claude-plugin/plugin.json
git commit -m "$(cat <<'EOF'
docs(plugins): surface /belt:handover and /belt:resume

Extend narrative-convention.md's /clear phrase to mention /belt:resume
as a second context-reset pathway. Drop the stale "resume from handover"
note from brainstorming-supplement (its generic-/handover semantic is
now subsumed by /belt:resume). List the two new skills in belt plugin
description for discoverability.
EOF
)"
```

---

## Phase F: Workspace verification

### Task F1: cargo fmt

- [ ] **Step 1: Run rustfmt across workspace (touched test files only, no production Rust code changed)**

Run:
```bash
cargo fmt --package belt-core
```

Expected: 無出力 (touch した test ファイルは既存スタイル踏襲のため fmt 差分なし想定)。差分が出た場合は `git diff` で確認して必要なら stage。

### Task F2: cargo clippy

- [ ] **Step 1: Workspace-wide clippy with -D warnings**

Run:
```bash
cargo clippy --workspace -- -D warnings
```

Expected: 警告・エラーなし。既存 workspace lints (unwrap_used / expect_used / panic は warn) は test ファイル側で allow 済み。

### Task F3: cargo test workspace

- [ ] **Step 1: Workspace test**

Run:
```bash
cargo test --workspace
```

Expected: すべて PASS。少なくとも以下が GREEN:
- `feature_dev_refresh::feature_dev_has_ten_phases`
- `feature_dev_refresh::feature_dev_expands_cleanly` (expanded.len == pipeline.phases.len、10 個になる)
- `bug_fix_refresh::phase_count_and_order`
- その他 workspace 全 crate の既存 test

### Task F4: Commit fmt/clippy fix-ups if any

- [ ] **Step 1: If fmt or clippy made any changes, commit them**

Run (only if `git status` shows modifications):
```bash
git add -A
git commit -m "chore: fmt and clippy fix-ups"
```

If no changes, skip this step. `git status` should be clean after Phase F.

---

## Post-implementation verification (manual, offline)

以下は automated test では拾えない挙動の手動確認 (spec の Evaluations を 1 回だけ回す)。実装完了直後の smoke として 1 回走らせる想定:

### EV-1 Happy path smoke

1. Temporary feature-dev run を起動: `belt-agent init plugins/belt/skills/feature-dev/pipeline.yml`
2. `design` / `test-scenarios` / `spec-review` / `plan` phase を通過
3. `pre-execute-handover` phase に到達することを `belt-agent status` で確認
4. `/belt:handover` を実行 (本 plan で作った skill) → `.belt/runs/<id>/handover.md` が schema 通り生成されるか確認
5. `/clear` → 新 session で `/belt:resume` 実行
6. protocol driver が `belt-agent status --run <id>` から再開し、`belt-agent verify` → `belt-agent step --confirm` で `execute` に進めるか確認

### EV-2 〜 EV-4

spec `docs/specs/2026-04-17-belt-handover-resume-design.md` の該当節に従い、必要に応じて手動シナリオで挙動確認。自動化は F2a 実装完了後の scenarios_contract follow-up 扱い (Open Questions 済み)。

---

## Rollback

想定される rollback ポイント:

- Phase A 後: `pre-execute-handover` phase が想定外に pipeline を止める → `git revert` で Phase A commit を戻す
- Phase B-D の skill 追加: `git rm -r plugins/belt/skills/handover plugins/belt/skills/resume plugins/belt-agent/skills/protocol/references/resume-mode.md` で削除、plugin.json + protocol SKILL.md + narrative-convention.md + brainstorming-supplement.md の 4 edits を手動 revert

各 Phase が独立 commit なので、任意の Phase 単位で revert 可能。

---

## Checklist verification

実装完了時に spec の "Checklist for Effective Skills" を満たすか確認:

- [ ] handover SKILL.md description は what + when 両方 (3 人称) を満たす
- [ ] resume SKILL.md description 同上
- [ ] 両 SKILL.md body < 500 行
- [ ] references は一段階のみ深 (protocol references 経由のみ)
- [ ] 用語統一 (`handover.md` / `handover` / `resume` / `Resume hint` を spec と同一用法)
- [ ] forward slash path のみ使用
- [ ] precondition 1-5 が skill 内で明示 (resume SKILL.md の表)
- [ ] EV-1..EV-4 が spec に記述されている
- [ ] feature_dev_refresh / bug_fix_refresh test 更新、phase count 新値で PASS
