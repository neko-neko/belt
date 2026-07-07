# Four-Stage Agentic Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** feature-dev を design → plan → checkpoint → build → qa → integrate の 4 ステージ合成に再編し、QA 必須化(人間可読証跡付き)・agent 8→6 再編・定型業務 skill 2 本を実装する。

**Architecture:** belt-core 本体は無変更(D8)。plugins/ 配下の pipeline.yml / SKILL.md / agents と belt-core の shape lock tests のみを変更する。追加(Stage A: Task 1-7)→ 原子的切替(Stage B: Task 8-9)→ 削除(Stage C: Task 10-11)→ 検証(Task 12)の順で、各コミット時点で `cargo test -p belt-core` green を維持する。

**Tech Stack:** belt pipeline YAML / Claude Code plugin (SKILL.md, agents) / Rust integration tests (belt-core, belt-agent)

**Spec:** `docs/specs/2026-07-07-four-stage-agentic-pipeline-design.md` (D1〜D14)

## Global Constraints

- 作業ブランチ: `feature/2026-07-07-four-stage-pipeline`(superpowers:using-git-worktrees で worktree 作成。全タスクを同一 worktree で実行)
- plugins/ 配下の全コンテンツは英語(リポジトリ公開コンテンツ英語ポリシー)。docs/plans, docs/specs は日本語可
- SKILL.md / agent は `plugins/belt-agent/references/authoring-principles.md` に従う。criteria/*.md・references/*-supplement.md の**新設禁止**(diagnose の既存 criteria は改修のみ)
- pipeline.yml の `with:` は bare 全文字列形式(`codex: "args.codex"`)のみ。expander が置換する唯一の形式
- pipeline.yml に `{run_id}` template と `.belt/runs/` リテラルを書かない(lock test が禁止)
- `plugins/belt/agents/code-reviewer.md` と `quality-reviewer.md` の `## Filtering` 直後の先頭 3 bullet は byte-identical を維持(shared_filter_parity.rs が lock。本計画では両ファイルの Filtering セクションに触れない)
- cross-stage regate は engine 制約上不可。regate はどこにも宣言しない
- `Artifact.when` は produces にのみ有効(Phase.when とは別物)。新パイプラインでは Phase.when を一切使わない
- コミット前チェック: Rust 変更時は `cargo fmt --package belt-core`(または belt-agent)+ `cargo clippy --package <pkg> -- -D warnings` + `cargo test -p <pkg>`。pipeline.yml 変更時は `cargo run -p belt -- lint <file>` を全変更ファイルに実行
- CLAUDE.md は AGENTS.md への symlink。編集後は `git add AGENTS.md`
- GPG 署名エラー時は `git -c commit.gpgsign=false commit ...` で再試行
- 既知の前提: main 上で `crates/belt-agent/tests/cli_test.rs::feature_dev_migrated_pipeline_boots` が**既に FAIL**(sonnet-lean 刷新時の更新漏れ)。Task 8 で修正する。Task 8 より前のタスクでは `cargo test -p belt-agent` の 1 fail は既知として扱う

## File Map(操作一覧)

| 操作 | ファイル | タスク |
|---|---|---|
| Create | `plugins/belt/skills/plan/pipeline.yml`, `plugins/belt/skills/plan/SKILL.md` | 1 |
| Create | `plugins/belt/skills/qa/pipeline.yml`, `plugins/belt/skills/qa/SKILL.md` | 2 |
| Create | `plugins/belt-agent/agents/explorer.md`, `plugins/belt-agent/agents/implementer.md` | 3 |
| Create | `plugins/belt/agents/qa-verifier.md` | 4 |
| Modify | `plugins/belt/skills/spec-review/SKILL.md`, `plugins/belt/skills/code-review/SKILL.md`, `plugins/belt/agents/{spec-reviewer,quality-reviewer}.md` | 5 |
| Create | `plugins/belt/skills/requirements/SKILL.md` / Modify `plugins/belt/skills/goal/SKILL.md` | 6 |
| Create | `plugins/belt/skills/docs/SKILL.md` | 7 |
| Rewrite | `crates/belt-core/tests/feature_dev_refresh.rs`, `crates/belt-core/tests/bug_fix_refresh.rs` | 8 |
| Rewrite | `plugins/belt/skills/{design,build,diagnose,feature-dev,bug-fix}/pipeline.yml` | 8 |
| Modify | `plugins/belt/skills/diagnose/criteria/rca.md`, `references/{rca,fix-plan}-supplement.md` | 8 |
| Modify | `crates/belt-agent/tests/cli_test.rs`(boots test 修正), `docs/testing/lock-ledger.md` | 8 |
| Rewrite | `plugins/belt/skills/{design,build,feature-dev,bug-fix}/SKILL.md`, Modify `diagnose/SKILL.md` | 9 |
| Modify | `plugins/belt/skills/design/references/path-convention.md`, `plugins/belt-agent/references/authoring-principles.md` | 9 |
| Modify | `crates/belt-core/tests/review_skills_refresh.rs`(agent bundle lock 拡張) | 10 |
| Delete | `plugins/belt-agent/agents/{code-explorer,code-architect,impact-analyzer,feature-implementer,phase-auditor}.md` | 10 |
| Delete | `plugins/belt-agent/references/audit-protocol.md`, `plugins/belt/skills/verify/`(全体) | 10 |
| Modify | `plugins/belt-agent/references/_schema.md`(phase-auditor 参照除去) | 10 |
| Modify | `plugins/{belt,belt-agent}/.claude-plugin/plugin.json`, `README.md`, `AGENTS.md`, `CHANGELOG.md` | 11 |

---

### Task 1: /belt:plan ステージ skill 新設(additive)

**Files:**
- Create: `plugins/belt/skills/plan/pipeline.yml`
- Create: `plugins/belt/skills/plan/SKILL.md`

**Interfaces:**
- Consumes: `docs/features/*/goal-sheet.md`, `docs/features/*/design.md`(design ステージ産)
- Produces: `docs/features/*/plan.md`(`## Test Strategy` + `## Tasks`)、`docs/features/*/scenarios.yml`(setup + kind 付き)、artifact 名 `plan_doc` / `scenarios` / `findings-plan`。Task 8 の feature-dev が `../plan/pipeline.yml` を参照する

- [ ] **Step 1: pipeline.yml を作成**

`plugins/belt/skills/plan/pipeline.yml`:

```yaml
name: plan
version: 1
description: "Implementation planning stage (plan)"

args:
  codex:
    type: bool
    default: false
    description: "Enable Codex parallel review in the plan review"

phases:
  - id: plan
    description: "Write plan.md (test strategy + tasks) and scenarios.yml from goal-sheet.md + design.md, then review the plan (see plan SKILL.md)"
    produces:
      - name: plan_doc
        path: "docs/features/*/plan.md"
        description: "Test strategy and checkbox implementation tasks"
      - name: scenarios
        path: "docs/features/*/scenarios.yml"
        description: "QA replay scenarios (Given/When/Then, kind: browser|cli, setup block)"
      - name: findings-plan
        path: "belt://current/review/findings-plan.json"
        description: "Consolidated plan review findings"
    gate:
      - file_exists: "docs/features/*/plan.md"
      - file_exists: "docs/features/*/scenarios.yml"
      - file_exists: "belt://current/review/findings-plan.json"
    validate:
      - "plan.md contains Test Strategy and Tasks sections; Tasks is a checkbox list where every task names its target files and its test"
      - "Every acceptance criterion in goal-sheet.md maps to at least one Test Strategy row and at least one scenario"
      - "Every scenario has kind: browser or kind: cli; setup: is declared when any scenario is kind: browser"
      - "Every critical/high finding in findings-plan.json is fixed or explicitly rejected by the user"
      - "evidence.md has a plan entry"
    confirm: true
    max_retries: 3
```

- [ ] **Step 2: SKILL.md を作成**

`plugins/belt/skills/plan/SKILL.md`:

```markdown
---
name: plan
description: >-
  Runs the implementation planning stage: writes plan.md (test strategy +
  task list) and scenarios.yml (QA replay scenarios) from an approved
  design.md, then reviews the plan via belt:spec-reviewer. Use standalone
  when a design already exists, or composed between the design and build
  stages of /belt:feature-dev. --codex enables adversarial plan review.
user-invocable: true
argument-hint: "[--codex]"
---

# plan

Belt pipeline for the implementation planning stage. Structure, gates,
and done criteria live in `pipeline.yml`; this file defines how to
execute the single phase.

## Entry check

`docs/features/<topic>/goal-sheet.md` and `docs/features/<topic>/design.md`
must exist. In a composed run resolve them via `belt-agent status`
artifacts; standalone, take the most recently modified glob match. If
either is missing, stop and ask the user.

## Phase: plan

This phase has no `invoke` — execute these steps directly:

1. Read goal-sheet.md and design.md.
2. Write `docs/features/<topic>/plan.md` with exactly these sections:
   - `## Test Strategy` — for each acceptance criterion in
     goal-sheet.md, the test(s) that verify it (level:
     unit/integration/qa + test name)
   - `## Tasks` — checkbox list; every task names its target files and
     its test
3. Write `docs/features/<topic>/scenarios.yml` — at least one scenario
   per acceptance criterion, schema below.
4. Invoke `/belt:spec-review` with the plan.md path as the target and
   `findings-plan` as the output artifact (pass `--codex` if the codex
   arg is true), and complete its triage.
5. If a finding contests an approved design decision, do not edit
   design.md — present the finding to the user as an objection to the
   approved design; on acceptance, re-run the design stage standalone.
6. Append the plan entry to evidence.md.

## scenarios.yml schema

    setup:                        # required if any scenario is kind: browser
      start: "pnpm dev"           # launch command; omit when nothing to launch
      url: "http://localhost:3000"
      teardown: auto              # auto = QA kills processes it started
    scenarios:
      - id: login-ok              # kebab-case, unique in this file
        kind: browser             # browser | cli
        given: "a registered user on the login page"
        when: "they submit valid credentials"
        then: "the dashboard is shown"

For `kind: cli`, `when` is the exact command to run and `then` states
the expected stdout, exit code, or produced files.

## Red flags

- Never write a task without file paths — build dispatches subagents
  from this list alone.
- Never leave an acceptance criterion without both a Test Strategy row
  and a scenario.
- Never author a `kind: browser` scenario without a `setup:` block.
- Never hand-edit files under `docs/features/<topic>/` after a phase
  completes (breaks the phase-start mtime filter).

## References

- `../design/references/path-convention.md` — naming rules (SSOT)
- `plugins/belt-agent/references/authoring-principles.md` — evidence entry format
```

- [ ] **Step 3: lint を実行**

Run: `cargo run -p belt -- lint plugins/belt/skills/plan/pipeline.yml`
Expected: PASS(exit 0)

- [ ] **Step 4: Commit**

```bash
git add plugins/belt/skills/plan/
git commit -m "feat(belt): add /belt:plan stage skill (implementation planning split from design)"
```

---

### Task 2: /belt:qa ステージ skill 新設(additive)

**Files:**
- Create: `plugins/belt/skills/qa/pipeline.yml`
- Create: `plugins/belt/skills/qa/SKILL.md`

**Interfaces:**
- Consumes: `docs/features/*/scenarios.yml`(feature run)または `docs/features/*/rca-scenarios.yml`(bug run)、`belt.toml` の `[qa] evidence` キー
- Produces: `docs/features/*/qa-report.md`(artifact 名 `qa_report`)、証跡ファイル群(run dir 配下、非コミット)。Task 4 の `belt:qa-verifier` と Task 10 で削除する `/belt:verify` の後継

- [ ] **Step 1: pipeline.yml を作成**

`plugins/belt/skills/qa/pipeline.yml`:

```yaml
name: qa
version: 1
description: "QA verification stage (scenario replay + exploratory pass with human-readable evidence)"

phases:
  - id: qa
    description: "Dispatch belt:qa-verifier to replay scenarios with evidence capture, run the exploratory pass, and write qa-report.md (see qa SKILL.md)"
    produces:
      - name: qa_report
        path: "docs/features/*/qa-report.md"
        description: "Scenario results with evidence references, exploratory notes, verdict"
    gate:
      - file_exists: "docs/features/*/qa-report.md"
    validate:
      - "Every scenario in scenarios.yml (or rca-scenarios.yml for bug runs) has a PASS/FAIL row in qa-report.md referencing evidence files that exist in the run's qa evidence directory"
      - "Every kind: browser row references at least one screenshot; every kind: cli row references a transcript"
      - "Every FAIL was fixed and re-verified by belt:qa-verifier, or explicitly accepted by the user; QA fix commits are listed in evidence.md's qa entry"
      - "Verdict: SKIPPED only with a recorded user approval (timestamp + reason) in qa-report.md"
      - "evidence.md has a qa entry"
    max_retries: 3
```

注意: `confirm:` は書かない(default false = 自律進行、D4)。

- [ ] **Step 2: SKILL.md を作成**

`plugins/belt/skills/qa/SKILL.md`:

```markdown
---
name: qa
description: >-
  Runs the QA verification stage: an independent belt:qa-verifier subagent
  replays docs/features/<topic>/scenarios.yml (browser scenarios via
  agent-browser with screenshots, cli scenarios by executing the real
  commands with transcripts), runs an exploratory pass, and writes
  qa-report.md. Evidence goes to the run directory and is published to
  PR/Linear per the [qa] evidence config. Use standalone after a build,
  or composed as the qa stage of /belt:feature-dev and /belt:bug-fix.
user-invocable: true
---

# qa

Belt pipeline for the QA stage. The single phase dispatches the
`belt:qa-verifier` subagent — the orchestrator never replays scenarios
itself, so verification stays independent from the implementation.

## Entry check

Locate the scenario file: `docs/features/<topic>/scenarios.yml` (feature
runs) or `docs/features/<topic>/rca-scenarios.yml` (bug runs). On glob
collision take the most recently modified. If none exists, stop and ask
the user — skipping QA requires their approval, recorded in
qa-report.md (Verdict: SKIPPED + timestamp + reason).

Resolve the evidence directory:

- Active belt run (`belt-agent status` succeeds) → read `run_id` from
  status and use the run directory's `qa/` subdirectory (the run
  directory is never committed; `.belt/` is gitignored).
- No active run → `.belt/qa-adhoc/<UTC YYYYMMDD-HHMMSS>/`.

## Phase: qa

1. Dispatch `Task(subagent_type: belt:qa-verifier)` with a
   self-contained prompt containing: the scenario file path, the
   evidence directory path, the report path
   `docs/features/<topic>/qa-report.md`, the change scope (file list
   from `git diff main...HEAD`), and the acceptance criteria copied
   from goal-sheet.md (bug runs: the reproduction condition from
   rca-report.md).
2. Read qa-report.md. For each FAIL: dispatch one
   `belt-agent:implementer` fix subagent (self-contained prompt: the
   failing scenario text, observed vs expected, evidence file paths,
   target files), run the project test suite, commit the fix, then
   re-dispatch belt:qa-verifier for the failed scenario ids only.
   Maximum 2 fix rounds; leftovers go to the user via the validate
   criteria. Code fixed during QA is NOT re-reviewed by
   /belt:code-review (D12) — the fix commits are reported at integrate.
3. Record every QA fix commit hash in evidence.md's qa entry.
4. Publish: if the `[qa] evidence` config resolves to `linear` (or
   `auto` with a known Linear issue id), attach the evidence files to
   the Linear issue now. Use the linear cli's native file upload; if it
   does not support uploads, post an issue comment with the evidence
   branch URLs instead (see integrate publishing in the orchestrator
   SKILL.md). The `pr` destination is published by integrate, after the
   PR exists.
5. Append the qa entry to evidence.md.

## Config: [qa] evidence (belt.toml)

    [qa]
    evidence = "auto"    # "pr" | "linear" | "local" | "auto"

- `pr` — publish to the PR comment at integrate.
- `linear` — attach to the Linear issue at the end of this phase.
- `local` — keep evidence in the run directory only; integrate reports
  the local path.
- `auto` (default; also when belt.toml or the key is absent) — PR if
  integrate creates one; else Linear if an issue id is known; else
  local with an explicit warning at integrate.

## Red flags

- Never replay scenarios in the orchestrator context — always through
  belt:qa-verifier.
- Never mark SKIPPED without recorded user approval.
- Never exceed 2 fix rounds silently — surface leftovers to the user.
- Never commit evidence binaries to the repository.

## References

- `../design/references/path-convention.md` — naming rules (SSOT)
- `../plan/SKILL.md` — scenarios.yml schema (writer side)
- `plugins/belt-agent/references/authoring-principles.md` — evidence entry format
```

- [ ] **Step 3: lint + Linear 添付可否の実測**

Run: `cargo run -p belt -- lint plugins/belt/skills/qa/pipeline.yml`
Expected: PASS

Run: `linear --help 2>&1 | grep -i "attach\|upload" ; echo "exit=$?"`
Expected: どちらでも可。ヒットなし(exit=1)の場合、qa SKILL.md の記述は既に fallback(コメント + URL)を含むため変更不要。ヒットあり(exit=0)の場合はそのサブコマンド名を SKILL.md の Step 4 の「native file upload」の後に括弧で追記する(例: `(linear issue attach)`)。

- [ ] **Step 4: Commit**

```bash
git add plugins/belt/skills/qa/
git commit -m "feat(belt): add /belt:qa stage skill (mandatory QA with human-readable evidence)"
```

---

### Task 3: belt-agent 新 agents — explorer + implementer(additive)

**Files:**
- Create: `plugins/belt-agent/agents/explorer.md`
- Create: `plugins/belt-agent/agents/implementer.md`

**Interfaces:**
- Produces: `belt-agent:explorer`(focus: flow | patterns | impact をプロンプトで受ける)、`belt-agent:implementer`(TDD 実行者)。Task 6/7 の requirements/docs skill、Task 9 の build SKILL.md が参照する
- 旧 5 agent の削除は Task 10(このタスクでは削除しない — additive 維持)

- [ ] **Step 1: explorer.md を作成**

`plugins/belt-agent/agents/explorer.md`:

```markdown
---
name: explorer
description: >-
  Unified codebase explorer. Traces feature flow end-to-end, extracts
  architectural patterns and conventions, or maps the blast radius of a
  planned change — selected by the focus parameter in the prompt
  (focus: flow | patterns | impact). Use during intake, design,
  requirements, and documentation work.
memory: project
effort: max
---

You are a codebase explorer. Your prompt names a target (feature,
module, or change area) and a focus. If no focus is given, use `flow`.
Read-only: never modify files, never invoke subagents.

## focus: flow — how does it work today?

- Find entry points (API, UI components, CLI commands, event handlers)
  and core implementation files.
- Follow call chains from entry to output; note data transformations,
  state changes, and side effects at each step.
- Map abstraction layers and the interfaces between components.

## focus: patterns — how is this codebase built?

- Extract module organization, data access, error handling, and testing
  patterns with file:line references.
- Find project convention documents (CLAUDE.md / AGENTS.md / docs).
- Locate similar existing features; note reusable components,
  utilities, and extension points the new work should use.

## focus: impact — what breaks if we change it?

- From the change target, find all callers recursively (LSP first, Grep
  fallback); trace import chains and the tests exercising the target.
- Find shared state: the same tables, config keys, cache keys, env
  vars, and file paths read or written elsewhere.
- Extract implicit contracts (invariants, validation rules, ordering,
  error contracts). Check paired computations (write/read,
  serialize/deserialize, aggregate/detail, plan/actual) and flag filter
  asymmetries between the pair members.

## Output Format

### Summary
2-3 sentences describing the target and the focus taken.

### Key Files
Bulleted `file:line` references with one-line descriptions.

### Findings
The focus-specific body: flow steps with transformations (flow) /
established patterns, reuse candidates, and constraints (patterns) /
reverse dependencies, shared state, implicit contracts, and risks with
severity (impact).

### Must-Verify Checklist
Actionable, specific items the caller must verify during design,
implementation, or testing.
```

- [ ] **Step 2: implementer.md を作成**

現行 `plugins/belt-agent/agents/feature-implementer.md` を土台に、name を `implementer` に変更し、`## Fix Tasks` セクションの Audit Gate 文言を差し替える。全文:

```markdown
---
name: implementer
description: >-
  Executes implementation tasks via test-driven development. Use
  proactively for pipeline implementation and fix dispatch tasks (from
  code-review findings or QA scenario failures) where each task has
  self-contained specs.
skills:
  - superpowers:test-driven-development
memory: project
effort: max
---

You are an implementer following Test-Driven Development. The TDD skill is preloaded in your context — follow its workflow strictly.

## Process

1. Read the task specification completely before starting
2. Follow TDD cycle: write failing test → implement minimal code → verify pass → refactor
3. Commit after each green test

## Task Contract

Treat the prompt as an implementation contract. Before editing, identify these items from the prompt and keep them explicit in your working memory:

1. **Purpose** — why this task exists
2. **Scope** — target files, symbols, or failure surface
3. **Done condition** — what must be true for the task to count as complete
4. **Verification** — which command or checks prove the result
5. **Constraints** — boundaries such as "do not modify X" or required evidence collection

If the prompt contains broad research context, anchor on the explicit implementation contract rather than inheriting exploratory noise. Do not invent additional scope.

## Evidence Collection

If Evidence Collection Requirements are provided in your prompt, collect all specified evidence to the designated paths. This includes:
- Test execution logs (redirect stdout/stderr to specified files)
- Build logs
- Lint logs
- Git diff snapshots

## Fix Tasks

When dispatched to fix a code-review finding or a QA scenario failure:
1. Read the fix instruction completely (finding/scenario, observed vs expected, target files, verification steps)
2. Reproduce the failure first (run the failing test or scenario-derived check)
3. Fix the root cause described by the instruction, not just the visible symptom
4. Apply the fix following TDD (write a test for the expected behavior, then fix)
5. Verify with the steps named in the prompt

## Verification Discipline

Your own verification is the first QA layer, not the final gate.

- Run the concrete verification steps named in the prompt, not a weaker substitute
- Report exact commands and outcomes faithfully when asked
- If verification fails, do not downgrade the result into "partial success" without evidence
- If a dedicated verifier will run after you, do not claim the overall task is fully verified unless the prompt explicitly limits you to self-checking

## Return Format

When a return format is specified in your prompt, output results in that exact format.

Default return schema for a Fix Task (injected by the orchestrator):
```json
{ "fix_status": "completed|partial|blocked", "completed_fixes": [], "blocked_fixes": [], "changes_summary": "" }
```
```

- [ ] **Step 3: Commit**

```bash
git add plugins/belt-agent/agents/explorer.md plugins/belt-agent/agents/implementer.md
git commit -m "feat(belt-agent): add explorer (3-in-1 consolidation) and implementer agents"
```

---

### Task 4: belt:qa-verifier agent 新設(additive)

**Files:**
- Create: `plugins/belt/agents/qa-verifier.md`

**Interfaces:**
- Consumes: プロンプトから scenario file path / evidence_dir / report path / acceptance criteria / change scope を受ける(Task 2 の qa SKILL.md が渡す)
- Produces: `docs/features/<topic>/qa-report.md` と `<evidence_dir>/` 配下の証跡ファイル。コード編集は禁止

- [ ] **Step 1: qa-verifier.md を作成**

`plugins/belt/agents/qa-verifier.md`:

```markdown
---
name: qa-verifier
description: Independent QA verifier. Replays scenarios (browser scenarios via agent-browser with per-step screenshots, cli scenarios by executing the real commands with full transcripts), runs a bounded exploratory pass, and writes qa-report.md. Never edits code. Evidence goes under the evidence directory passed in the prompt.
memory: project
---

You are an independent QA verifier. You verify what was built; you
never fix it. Your prompt provides: the scenario file path, the
evidence directory (`evidence_dir`), the report path, the change scope,
and the acceptance criteria.

## Setup

Execute the scenario file's `setup:` block exactly as declared: run
`start` (if present) in the background, wait until `url` responds. If
setup fails, write the report recording the setup failure (command,
output) and stop — a setup failure is NOT a scenario FAIL. On finish,
if `teardown: auto`, kill every process you started.

## Scenario replay

For each scenario, in file order:

- `kind: browser` — load the agent-browser skill and drive the steps.
  Save a screenshot per meaningful step to
  `<evidence_dir>/<scenario-id>/NN-<step>.png` (2-digit NN, kebab-case
  step name, at most 5 per scenario — the steps needed to judge the
  outcome). If agent-browser is unavailable, report and stop — never
  simulate a browser run.
- `kind: cli` — execute the `when:` command exactly. Write
  `<evidence_dir>/<scenario-id>/transcript.txt` containing the command
  line prefixed with `$ `, the full stdout/stderr, and a final line
  `exit: <code>`.

Record PASS or FAIL against `then:` with the observed behavior. Never
retry a FAIL silently — record it; a re-run happens only when you are
re-dispatched after a fix.

## Exploratory pass

Around the change scope from your prompt, probe beyond the scripted
paths: invalid input, empty states, repeated actions, back/reload
navigation (browser) or invalid flags and missing files (cli). Cap at
15 minutes. Save evidence for every anomaly to
`<evidence_dir>/exploratory/<probe>-NN.png|txt`. An anomaly is advisory
unless it violates an acceptance criterion from your prompt — then it
is a FAIL row in the report.

## Report

Write the report to the path from your prompt:

    # QA report: <topic>
    ## Run                — run id (or adhoc timestamp) identifying the evidence directory
    ## Scenario results   — table: scenario | kind | result | evidence (paths relative to evidence_dir)
    ## Exploratory notes  — bullets: probe, observation, evidence path, advisory|FAIL
    ## Verdict            — PASS / FAIL (list failing ids) / SKIPPED (user approval: timestamp + reason)

## Guardrails

- Never edit code, tests, or documents other than the report file.
- Never mark PASS without executing the scenario.
- Never fabricate evidence paths — every referenced file must exist.
- Setup failure, missing agent-browser, or a missing scenario file →
  report and stop; never write SKIPPED on your own judgment.
```

- [ ] **Step 2: `.belt/runs/` リテラル不在を確認**

Run: `grep -c ".belt/runs/" plugins/belt/agents/qa-verifier.md; echo exit=$?`
Expected: `0` / `exit=1`(Task 10 で review_skills_refresh の output_path パターン lock に qa-verifier を加えるための前提)

- [ ] **Step 3: Commit**

```bash
git add plugins/belt/agents/qa-verifier.md
git commit -m "feat(belt): add qa-verifier agent (independent scenario replay with evidence capture)"
```

---

### Task 5: spec-review / code-review SKILL.md 改修

**Files:**
- Modify: `plugins/belt/skills/spec-review/SKILL.md`
- Modify: `plugins/belt/skills/code-review/SKILL.md`
- Modify: `plugins/belt/agents/spec-reviewer.md`(description のみ)
- Modify: `plugins/belt/agents/quality-reviewer.md`(Scope 1 行のみ)

**Interfaces:**
- Produces: spec-review は「target path + 出力 artifact 名/出力ディレクトリ指定」を受ける(Task 1 の plan phase が `findings-plan` を、Task 6 の requirements が出力ディレクトリを渡す)。code-review は belt run 有無で autonomous / batched triage を切り替える(Task 8 の build code-review phase 前提)
- 制約: `review_skills_refresh.rs::review_skills_parent_skill_md_references_parallel_dispatch` が「`findings-` と `Task` を含む」ことを lock — 改修後も両文字列を含むこと

- [ ] **Step 1: spec-review SKILL.md を書き換え**

`plugins/belt/skills/spec-review/SKILL.md` 全文を以下で置換:

```markdown
---
name: spec-review
description: >-
  Spec review via the consolidated belt:spec-reviewer agent. Reviews any
  spec-family document (requirements.md, goal-sheet.md, design.md,
  plan.md). Findings are triaged in one batched selection. --codex adds
  an adversarial pass via /codex:rescue in the same parallel batch.
argument-hint: "[<target-path>] [--codex]"
---

# Spec Review

Runs in the main context because triage and fix apply need user dialogue
and Edit access.

## Target

The spec document: use the caller-supplied path if given, otherwise the
most recently modified `design.md`, `plan.md`, `*-design.md`,
`goal-sheet.md`, or `requirements.md` under `docs/`.

## Output resolution

1. Caller supplied an output artifact name (e.g. `findings-plan`) →
   run `belt-agent status` and read that artifact's `resolved_path`.
2. No artifact name but a belt run is active → use the `findings-spec`
   artifact's `resolved_path`.
3. No belt run active → use the caller-supplied output directory; if
   none was supplied, use `<target document's directory>/review/`.
   The findings file is `findings-spec.json` in that directory.

## Dispatch

1. Dispatch `Task(subagent_type: belt:spec-reviewer, prompt: <spec path
   + output_path>)`. With `--codex`, invoke `/codex:rescue` in the same
   message with the spec path, the findings JSON schema from the
   spec-reviewer agent, and its own output_path.
2. Announce what was dispatched.

## Triage (batched)

Read the findings JSON file(s). Present ALL findings as one numbered
list, sorted by severity (critical > high > medium > low). For each:
one line of description + the suggested fix. Ask the user once, via
AskUserQuestion or a single message, which numbers to apply. Do not ask
per-finding questions across multiple turns.

## Fix apply

Apply accepted suggestions to the spec with Edit. Then:

1. `git diff` — confirm only the target spec changed.
2. Re-check internal links and headings still resolve.

## Red flags

- Never modify the spec before user selection.
- Never filter findings before presenting them.
- Never ask one-question-at-a-time across turns — batch the triage.
```

- [ ] **Step 2: code-review SKILL.md の Triage 以降を書き換え**

`plugins/belt/skills/code-review/SKILL.md` の `## Triage (batched)` から `## Red flags` の直前までを以下で置換(Scope detection / Dispatch / Merge は現行のまま):

```markdown
## Triage

Determine the mode first: run `belt-agent status`.

- **Pipeline mode (status succeeds):** autonomous triage. For each
  critical/high finding, in severity order: apply the suggested fix
  with Edit, run the project linter and test suite, and commit. If the
  fix would change the approved design/plan scope, or the second fix
  attempt still fails lint/tests, revert it and record the finding as
  deferred (id, severity, reason) — the orchestrator writes deferred
  findings into evidence.md's code-review entry and integrate reports
  them to the user. Medium/low findings are recorded, not fixed.
- **Standalone mode (status fails):** batched user triage. Present ALL
  merged findings as one numbered list (severity order, one line +
  suggestion each). Ask once which numbers to fix. No per-finding
  dialogue across turns. Apply the selected fixes serially with Edit.

## Verify

1. Run the project linter and test suite (Cargo.toml → `cargo clippy --
   -D warnings` + `cargo test`; package.json → `npm run lint` + `npm
   test`; pyproject.toml → `ruff check .` + `pytest`; go.mod → `go vet
   ./...` + `go test ./...`; Makefile → `make lint` + `make test`).
2. Report failures honestly — never suppress.
```

同ファイルの `## Red flags` を以下で置換:

```markdown
## Red flags

- Never fix a finding in standalone mode before user selection.
- Never filter findings before presenting or recording them.
- Never let an agent read another agent's findings-*.json.
- Never leave a deferred finding unrecorded — no silent drops.
```

- [ ] **Step 3: code-review Dispatch 行と reviewer agent の対象文書参照を更新**

1. `plugins/belt/skills/code-review/SKILL.md` の Dispatch 節の 2 つの Task 行にある `design doc
   path if one exists` を `design.md and plan.md (or fix-plan.md) paths if they exist` に置換
   (2 箇所)。
2. `plugins/belt/agents/spec-reviewer.md` の frontmatter description の先頭文を以下に置換
   (`## Filtering` セクションには触れない):

```
description: Consolidated spec reviewer for requirements.md, goal-sheet.md, design.md, and plan.md. Verifies feasibility, requirements clarity, design judgment, codebase consistency, and (when the spec has UI content) UI-pattern alignment in one pass. Writes findings to the output_path from the prompt.
```

3. `plugins/belt/agents/quality-reviewer.md` の `## Scope` 節の `If a design document path is
   provided` を `If design/plan document paths are provided` に置換(Filtering セクションは
   触らない — shared_filter_parity lock)。

- [ ] **Step 4: lock 前提の grep を確認**

Run: `grep -l "findings-" plugins/belt/skills/spec-review/SKILL.md plugins/belt/skills/code-review/SKILL.md && grep -l "Task" plugins/belt/skills/spec-review/SKILL.md plugins/belt/skills/code-review/SKILL.md`
Expected: 4 行(両ファイルが両方の文字列を含む)

Run: `cargo test -p belt-core --test shared_filter_parity`
Expected: PASS(Filtering prefix 不変)

Run: `cargo test -p belt-core --test review_skills_refresh`
Expected: PASS(6 tests)

- [ ] **Step 5: Commit**

```bash
git add plugins/belt/skills/spec-review/SKILL.md plugins/belt/skills/code-review/SKILL.md \
  plugins/belt/agents/spec-reviewer.md plugins/belt/agents/quality-reviewer.md
git commit -m "feat(belt): parameterize spec-review output; autonomous code-review triage in pipeline mode"
```

---

### Task 6: /belt:requirements 新設 + goal 入力ルール追記

**Files:**
- Create: `plugins/belt/skills/requirements/SKILL.md`
- Modify: `plugins/belt/skills/goal/SKILL.md:19-26`(Step 1 の入力解決ルール)

**Interfaces:**
- Produces: `docs/requirements/<YYYY-MM-DD-topic>/requirements.md`。`/belt:goal` が同パスを入力として受理する

- [ ] **Step 1: requirements SKILL.md を作成**

`plugins/belt/skills/requirements/SKILL.md`:

```markdown
---
name: requirements
description: >-
  Interview-driven requirements definition. Resolves a Linear ticket,
  URL, or free-text request, investigates the codebase, asks only
  human-decidable questions in one batch, and writes
  docs/requirements/<YYYY-MM-DD-topic>/requirements.md reviewed by
  belt:spec-reviewer. The result feeds /belt:feature-dev as input.
user-invocable: true
argument-hint: "<linear-id | url | free-text>"
---

# requirements

Turn raw input into a reviewed requirements document with at most 2
question rounds. No pipeline.yml — dialogue-centric skills do not run
under belt.

## Step 1 — Resolve input

Apply the first matching rule to the argument text:

- Matches `^[A-Z]+-[0-9]+$` → run `linear issue view <id>` and collect
  title, description, comments, and linked URLs.
- Starts with `http` and contains `slack.com` → fetch the thread via
  the slackcli skill.
- Starts with `http` (other) → fetch the page via WebFetch.
- Anything else → treat the text itself as the request.

If the fetched content links to further tickets/PRs, fetch at most 2 of
them (the most directly referenced). Do not crawl deeper.

## Step 2 — Investigate the codebase

Grep/Read every identifier, module, and feature name in the resolved
input. For unfamiliar areas spanning 10+ files, dispatch
`belt-agent:explorer` subagents in parallel (focus: flow or patterns).
A question answerable here MUST NOT be asked to the user.

## Step 3 — Batched questions

Ask the remaining human-decidable points (business goals, scope
boundaries, priorities, non-functional targets) via AskUserQuestion —
up to 4 questions per round, max 2 rounds. Unresolved points default to
the recommended option and are recorded under Open decisions.

## Step 4 — Write the document

Create `docs/requirements/<YYYY-MM-DD-topic>/requirements.md` (topic
slug rules follow
`plugins/belt/skills/design/references/path-convention.md`) with
exactly these sections, none empty (write "(none)" only under Open
decisions):

    # Requirements: <topic>
    ## Background                  — why now, current pain
    ## Goals                       — measurable outcomes
    ## Functional requirements     — numbered; each verifiable
    ## Non-functional requirements — performance / security / operability targets
    ## Acceptance criteria         — numbered; each verifiable by a command, test, or observable behavior
    ## Out-of-scope                — explicit exclusions
    ## Open decisions              — defaulted choices + known unknowns

## Step 5 — Review

Invoke `/belt:spec-review` with the requirements.md path as the target
and `docs/requirements/<topic>/review/` as the output directory (no
belt run is active), and complete its triage.

Hand the file path to `/belt:feature-dev` (or `/belt:goal`) to start
development from it.

## Red flags

- Never ask a question the codebase can answer.
- Never run more than 2 question rounds.
- Never write requirements as implementation instructions — state
  outcomes, not designs.
```

- [ ] **Step 2: goal SKILL.md の入力解決ルールに 1 行追加**

`plugins/belt/skills/goal/SKILL.md` の Step 1 リスト(`- Matches ^[A-Z]+-[0-9]+$ ...` の直前)に次の行を追加:

```markdown
- Points to an existing local `requirements.md` (a path containing
  `docs/requirements/`) → read it; the goal sheet condenses its Goals /
  Acceptance criteria / Out-of-scope, and its Open decisions become
  Open risks.
```

- [ ] **Step 3: Commit**

```bash
git add plugins/belt/skills/requirements/ plugins/belt/skills/goal/SKILL.md
git commit -m "feat(belt): add /belt:requirements intake skill; goal accepts requirements.md input"
```

---

### Task 7: /belt:docs 新設

**Files:**
- Create: `plugins/belt/skills/docs/SKILL.md`

**Interfaces:**
- Produces: 対象リポジトリの `docs/` 配下の新規/更新ドキュメント。`belt-agent:explorer` を調査に使う

- [ ] **Step 1: docs SKILL.md を作成**

`plugins/belt/skills/docs/SKILL.md`:

```markdown
---
name: docs
description: >-
  General-purpose documentation writing. Takes a topic (feature, module,
  or theme), investigates the code, and creates or updates documents
  under docs/ following the repository's existing documentation
  conventions. Use for architecture overviews, usage guides, and
  reference pages.
user-invocable: true
argument-hint: "<topic or target path>"
---

# docs

Write documentation that matches the target repository, not a template.
No pipeline.yml — dialogue-centric skills do not run under belt.

## Step 1 — Survey

- Read the existing `docs/` tree: language, tone, heading style, index
  files, directory taxonomy.
- Investigate the code the topic covers with Grep/Read. For unfamiliar
  areas spanning 10+ files, dispatch `belt-agent:explorer` subagents in
  parallel (focus: flow or patterns).

## Step 2 — Placement decision

Determine the target path and document type (architecture overview /
usage guide / reference) from the existing taxonomy. If either is still
ambiguous after the survey, ask once via AskUserQuestion (one batch, up
to 4 questions) — placement, type, audience, depth.

## Step 3 — Write

- Follow the repository's documentation language and style; when the
  repo has no stated policy, write in English.
- Verify every code statement in the document (file paths, command
  names, config keys, API names) against the actual source — never
  write from memory.
- Update cross-links: link related existing documents, and add the new
  document to the index (README or docs index) when one exists.

## Step 4 — Verify

- Check that every path referenced by the touched documents exists.
- Show the user the diff summary: files created/updated + one-line
  purpose each.

## Red flags

- Never restate what a referenced document already says — link it.
- Never document APIs or commands without checking they exist in the
  code.
- Never invent a new docs/ subdirectory when an existing one fits.
```

- [ ] **Step 2: Commit**

```bash
git add plugins/belt/skills/docs/
git commit -m "feat(belt): add /belt:docs general documentation skill"
```

---

### Task 8: Pipeline cutover(lock tests 改訂 + 5 pipeline.yml 書き換え、原子的 1 コミット)

**Files:**
- Rewrite: `crates/belt-core/tests/feature_dev_refresh.rs`
- Rewrite: `crates/belt-core/tests/bug_fix_refresh.rs`
- Rewrite: `plugins/belt/skills/design/pipeline.yml`
- Rewrite: `plugins/belt/skills/build/pipeline.yml`
- Rewrite: `plugins/belt/skills/diagnose/pipeline.yml`
- Rewrite: `plugins/belt/skills/feature-dev/pipeline.yml`
- Rewrite: `plugins/belt/skills/bug-fix/pipeline.yml`
- Modify: `plugins/belt/skills/diagnose/criteria/rca.md:100-112`、`plugins/belt/skills/diagnose/references/rca-supplement.md:48-56`、`plugins/belt/skills/diagnose/references/fix-plan-supplement.md:63`
- Modify: `crates/belt-agent/tests/cli_test.rs`(`feature_dev_migrated_pipeline_boots`、main 上で既に FAIL している既存バグの修正を fold-in)
- Modify: `docs/testing/lock-ledger.md`(feature_dev_refresh / bug_fix_refresh entry)

**Interfaces:**
- Consumes: Task 1 の `../plan/pipeline.yml`、Task 2 の `../qa/pipeline.yml`
- Produces: 新トポロジー feature-dev(8 leaves: `design/intake, design/design, plan/plan, pre-execute-handover/checkpoint, build/execute, build/code-review, qa/qa, integrate`)/ bug-fix(8 leaves: `diagnose/rca, diagnose/fix-plan, diagnose/fix-plan-review, pre-execute-handover/checkpoint, build/execute, build/code-review, qa/qa, integrate`)。args は両者 `codex` のみ。confirm leaf: feature-dev = `design/design, plan/plan, pre-execute-handover/checkpoint, integrate` / bug-fix = `diagnose/fix-plan-review, pre-execute-handover/checkpoint, integrate`

- [ ] **Step 1: feature_dev_refresh.rs を新 shape contract に書き換え(failing test 先行)**

`crates/belt-core/tests/feature_dev_refresh.rs` 全文を以下で置換:

```rust
//! Integration tests for the composed feature-dev pipeline (2026-07-07
//! four-stage rewrite): design(sub) + plan(sub) + pre-execute-handover(sub)
//! + build(sub) + qa(sub) + integrate(leaf).
//!
//! Shape contract (spec docs/specs/2026-07-07-four-stage-agentic-pipeline-design.md):
//! - args = { codex: bool } only (e2e removed — QA is mandatory, D2)
//! - 6 top-level phases: design/plan/build delegate with { codex },
//!   pre-execute-handover and qa delegate with empty `with`,
//!   integrate is an inline leaf (Invoker::Skill /worktrunk)
//! - expansion flattens to exactly 8 namespaced leaves
//! - no leaf declares regate; no leaf carries a phase-level when
//! - confirm leaves are exactly design/design, plan/plan,
//!   pre-execute-handover/checkpoint, integrate (D4)
//! - the integrate leaf is byte-equivalent (as serde_json::Value) to the
//!   bug-fix integrate leaf (D14 inline duplication + identity lock)

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use std::path::PathBuf;

use belt_core::{
    error::BeltError,
    expander::expand_pipeline,
    model::{ArgType, Invoker},
    parser::parse_pipeline,
};

mod common;
use common::helpers::repo_root;

fn feature_dev_pipeline_path() -> PathBuf {
    repo_root().join("plugins/belt/skills/feature-dev/pipeline.yml")
}

fn bug_fix_pipeline_path() -> PathBuf {
    repo_root().join("plugins/belt/skills/bug-fix/pipeline.yml")
}

const EXPECTED_LEAVES: &[&str] = &[
    "design/intake",
    "design/design",
    "plan/plan",
    "pre-execute-handover/checkpoint",
    "build/execute",
    "build/code-review",
    "qa/qa",
    "integrate",
];

const CONFIRM_LEAVES: &[&str] = &[
    "design/design",
    "plan/plan",
    "pre-execute-handover/checkpoint",
    "integrate",
];

#[test]
fn feature_dev_composes_six_stages() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;
    let got: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        got,
        vec![
            "design",
            "plan",
            "pre-execute-handover",
            "build",
            "qa",
            "integrate"
        ],
        "top-level composition must be design -> plan -> checkpoint -> build -> qa -> integrate"
    );
    Ok(())
}

#[test]
fn stages_delegate_with_codex_passthrough() {
    let pipeline =
        parse_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must parse");
    for (phase_id, expected_sub) in [
        ("design", "../design/pipeline.yml"),
        ("plan", "../plan/pipeline.yml"),
        ("build", "../build/pipeline.yml"),
    ] {
        let phase = pipeline
            .phases
            .iter()
            .find(|p| p.id == phase_id)
            .unwrap_or_else(|| panic!("phase '{phase_id}' must exist"));
        let Some(Invoker::Pipeline {
            pipeline: sub_path,
            with,
        }) = phase.invoke.as_ref()
        else {
            panic!("phase '{phase_id}' must use Invoker::Pipeline");
        };
        assert_eq!(sub_path, expected_sub, "phase '{phase_id}' sub path");
        let keys: Vec<&str> = with.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            vec!["codex"],
            "phase '{phase_id}' must pass exactly {{codex}}"
        );
        assert_eq!(
            with.get("codex").and_then(|v| v.as_str()),
            Some("args.codex"),
            "phase '{phase_id}' codex must be the bare full-string form"
        );
    }
}

#[test]
fn checkpoint_and_qa_delegate_with_no_args() {
    let pipeline =
        parse_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must parse");
    for (phase_id, expected_sub) in [
        ("pre-execute-handover", "../handover/checkpoint.yml"),
        ("qa", "../qa/pipeline.yml"),
    ] {
        let phase = pipeline
            .phases
            .iter()
            .find(|p| p.id == phase_id)
            .unwrap_or_else(|| panic!("phase '{phase_id}' must exist"));
        match phase.invoke.as_ref() {
            Some(Invoker::Pipeline {
                pipeline: sub_path,
                with,
            }) => {
                assert_eq!(sub_path, expected_sub, "phase '{phase_id}' sub path");
                assert!(
                    with.is_empty(),
                    "phase '{phase_id}' delegation must not pass any `with` args"
                );
            }
            other => panic!("phase '{phase_id}' must use Invoker::Pipeline, got {other:?}"),
        }
    }
}

#[test]
fn top_level_args_are_codex_only() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;

    let names: Vec<&str> = pipeline.args.keys().map(String::as_str).collect();
    assert_eq!(names, vec!["codex"], "args must be exactly {{codex}}");

    for (name, def) in &pipeline.args {
        assert!(
            matches!(def.arg_type, ArgType::Bool),
            "arg '{name}' must be typed bool"
        );
        assert_eq!(
            def.default.as_ref().and_then(serde_json::Value::as_bool),
            Some(false),
            "arg '{name}' default must be false"
        );
    }
    Ok(())
}

#[test]
fn feature_dev_expands_to_eight_namespaced_leaves() {
    let expanded =
        expand_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must expand");
    let ids: Vec<&str> = expanded.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, EXPECTED_LEAVES, "expanded leaf ids + order must match");
}

#[test]
fn no_leaf_declares_regate_or_when() {
    let expanded =
        expand_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must expand");
    for leaf in &expanded {
        assert!(
            leaf.regate.is_empty(),
            "leaf '{}' must have empty regate, got {:?}",
            leaf.id,
            leaf.regate
        );
        assert_eq!(
            leaf.when, None,
            "leaf '{}' must not carry a phase-level when (e2e opt-in removed)",
            leaf.id
        );
    }
}

#[test]
fn confirm_leaves_match_the_four_touchpoints() {
    let expanded =
        expand_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must expand");
    let confirmed: Vec<&str> = expanded
        .iter()
        .filter(|p| p.confirm)
        .map(|p| p.id.as_str())
        .collect();
    assert_eq!(
        confirmed, CONFIRM_LEAVES,
        "confirm leaves must be exactly the four human touchpoints (D4)"
    );
}

#[test]
fn integrate_leaf_identical_across_orchestrators() {
    let feature = parse_pipeline(&feature_dev_pipeline_path()).expect("feature-dev must parse");
    let bug = parse_pipeline(&bug_fix_pipeline_path()).expect("bug-fix must parse");
    let f_integrate = feature
        .phases
        .iter()
        .find(|p| p.id == "integrate")
        .expect("feature-dev integrate leaf must exist");
    let b_integrate = bug
        .phases
        .iter()
        .find(|p| p.id == "integrate")
        .expect("bug-fix integrate leaf must exist");
    let f_val = serde_json::to_value(f_integrate).expect("serialize feature-dev integrate");
    let b_val = serde_json::to_value(b_integrate).expect("serialize bug-fix integrate");
    assert_eq!(
        f_val, b_val,
        "integrate leaf must be identical across feature-dev and bug-fix (D14)"
    );
}

#[test]
fn feature_dev_pipeline_has_no_run_id_template() {
    let yaml = std::fs::read_to_string(feature_dev_pipeline_path())
        .expect("feature-dev pipeline.yml must be readable");
    assert!(
        !yaml.contains("{run_id}"),
        "pipeline must not contain {{run_id}} template anywhere"
    );
    assert!(
        !yaml.contains(".belt/runs/"),
        "pipeline must not contain .belt/runs/ literal anywhere"
    );
}
```

- [ ] **Step 2: bug_fix_refresh.rs を新 shape contract に書き換え**

`crates/belt-core/tests/bug_fix_refresh.rs` 全文を以下で置換:

```rust
//! Integration tests for the composed bug-fix pipeline (2026-07-07
//! four-stage rewrite): diagnose(sub) + pre-execute-handover(sub) +
//! build(sub) + qa(sub) + integrate(leaf).
//!
//! Shape contract (spec docs/specs/2026-07-07-four-stage-agentic-pipeline-design.md):
//! - args = { codex: bool } only (e2e removed — QA is mandatory, D2)
//! - 5 top-level phases: diagnose/build delegate with { codex },
//!   pre-execute-handover and qa delegate with empty `with`,
//!   integrate is an inline leaf (Invoker::Skill /worktrunk)
//! - expansion flattens to exactly 8 namespaced leaves
//! - no leaf declares regate; no leaf carries a phase-level when
//! - confirm leaves are exactly diagnose/fix-plan-review,
//!   pre-execute-handover/checkpoint, integrate (D4: one diagnosis
//!   approval point)
//! - integrate identity with feature-dev is locked in
//!   feature_dev_refresh.rs::integrate_leaf_identical_across_orchestrators

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use std::path::PathBuf;

use belt_core::{
    error::BeltError,
    expander::expand_pipeline,
    model::{ArgType, Invoker},
    parser::parse_pipeline,
};

mod common;
use common::helpers::repo_root;

fn bug_fix_pipeline_path() -> PathBuf {
    repo_root().join("plugins/belt/skills/bug-fix/pipeline.yml")
}

const EXPECTED_LEAVES: &[&str] = &[
    "diagnose/rca",
    "diagnose/fix-plan",
    "diagnose/fix-plan-review",
    "pre-execute-handover/checkpoint",
    "build/execute",
    "build/code-review",
    "qa/qa",
    "integrate",
];

const CONFIRM_LEAVES: &[&str] = &[
    "diagnose/fix-plan-review",
    "pre-execute-handover/checkpoint",
    "integrate",
];

#[test]
fn bug_fix_composes_five_stages() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&bug_fix_pipeline_path())?;
    let got: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        got,
        vec![
            "diagnose",
            "pre-execute-handover",
            "build",
            "qa",
            "integrate"
        ],
        "top-level composition must be diagnose -> checkpoint -> build -> qa -> integrate"
    );
    Ok(())
}

#[test]
fn stages_delegate_with_codex_passthrough() {
    let pipeline = parse_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must parse");
    for (phase_id, expected_sub) in [
        ("diagnose", "../diagnose/pipeline.yml"),
        ("build", "../build/pipeline.yml"),
    ] {
        let phase = pipeline
            .phases
            .iter()
            .find(|p| p.id == phase_id)
            .unwrap_or_else(|| panic!("phase '{phase_id}' must exist"));
        let Some(Invoker::Pipeline {
            pipeline: sub_path,
            with,
        }) = phase.invoke.as_ref()
        else {
            panic!("phase '{phase_id}' must use Invoker::Pipeline");
        };
        assert_eq!(sub_path, expected_sub, "phase '{phase_id}' sub path");
        let keys: Vec<&str> = with.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            vec!["codex"],
            "phase '{phase_id}' must pass exactly {{codex}}"
        );
        assert_eq!(
            with.get("codex").and_then(|v| v.as_str()),
            Some("args.codex"),
            "phase '{phase_id}' codex must be the bare full-string form"
        );
    }
}

#[test]
fn checkpoint_and_qa_delegate_with_no_args() {
    let pipeline = parse_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must parse");
    for (phase_id, expected_sub) in [
        ("pre-execute-handover", "../handover/checkpoint.yml"),
        ("qa", "../qa/pipeline.yml"),
    ] {
        let phase = pipeline
            .phases
            .iter()
            .find(|p| p.id == phase_id)
            .unwrap_or_else(|| panic!("phase '{phase_id}' must exist"));
        match phase.invoke.as_ref() {
            Some(Invoker::Pipeline {
                pipeline: sub_path,
                with,
            }) => {
                assert_eq!(sub_path, expected_sub, "phase '{phase_id}' sub path");
                assert!(
                    with.is_empty(),
                    "phase '{phase_id}' delegation must not pass any `with` args"
                );
            }
            other => panic!("phase '{phase_id}' must use Invoker::Pipeline, got {other:?}"),
        }
    }
}

#[test]
fn args_are_codex_only() {
    let pipeline = parse_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must parse");
    let keys: Vec<&str> = pipeline.args.keys().map(String::as_str).collect();
    assert_eq!(keys, vec!["codex"]);

    for (name, def) in &pipeline.args {
        assert!(
            matches!(def.arg_type, ArgType::Bool),
            "arg '{name}' must be bool"
        );
        assert_eq!(
            def.default.as_ref().and_then(serde_json::Value::as_bool),
            Some(false),
            "arg '{name}' default must be false"
        );
    }
}

#[test]
fn no_legacy_args() {
    let pipeline = parse_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must parse");
    for legacy in ["iterations", "swarm", "ui", "smoke", "e2e"] {
        assert!(
            !pipeline.args.contains_key(legacy),
            "legacy arg '{legacy}' must be removed"
        );
    }
}

#[test]
fn bug_fix_expands_to_eight_namespaced_leaves() {
    let expanded = expand_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must expand");
    let ids: Vec<&str> = expanded.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, EXPECTED_LEAVES, "expanded leaf ids + order must match");
}

#[test]
fn no_leaf_declares_regate_or_when() {
    let expanded = expand_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must expand");
    for leaf in &expanded {
        assert!(
            leaf.regate.is_empty(),
            "leaf '{}' must have empty regate, got {:?}",
            leaf.id,
            leaf.regate
        );
        assert_eq!(
            leaf.when, None,
            "leaf '{}' must not carry a phase-level when (e2e opt-in removed)",
            leaf.id
        );
    }
}

#[test]
fn confirm_leaves_match_the_three_touchpoints() {
    let expanded = expand_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must expand");
    let confirmed: Vec<&str> = expanded
        .iter()
        .filter(|p| p.confirm)
        .map(|p| p.id.as_str())
        .collect();
    assert_eq!(
        confirmed, CONFIRM_LEAVES,
        "confirm leaves must be exactly the three human touchpoints (D4)"
    );
}

#[test]
fn bug_fix_pipeline_has_no_run_id_template() {
    let yaml = std::fs::read_to_string(bug_fix_pipeline_path())
        .expect("bug-fix pipeline.yml must be readable");
    assert!(
        !yaml.contains("{run_id}"),
        "pipeline must not contain {{run_id}} template anywhere"
    );
    assert!(
        !yaml.contains(".belt/runs/"),
        "pipeline must not contain .belt/runs/ literal anywhere"
    );
}
```

- [ ] **Step 3: 失敗を確認**

Run: `cargo test -p belt-core --test feature_dev_refresh --test bug_fix_refresh 2>&1 | tail -5`
Expected: FAIL(旧 pipeline.yml のため。`feature_dev_composes_six_stages` 等が落ちる)

- [ ] **Step 4: design/pipeline.yml を書き換え**

`plugins/belt/skills/design/pipeline.yml` 全文を以下で置換(e2e arg 除去、intake confirm 除去、design.md から Test Strategy / Tasks / scenarios を除去):

```yaml
name: design
version: 1
description: "Feature design stage (intake -> design)"

args:
  codex:
    type: bool
    default: false
    description: "Enable Codex parallel review in the design spec review"

phases:
  - id: intake
    description: "Resolve ticket/URL/free-text input into a goal sheet via /belt:goal"
    invoke:
      skill: /belt:goal
    produces:
      - name: goal_sheet
        path: "docs/features/*/goal-sheet.md"
        description: "Goal / In-scope / Out-of-scope / Acceptance criteria / Open risks"
      - name: evidence
        path: "docs/features/*/evidence.md"
        description: "Per-phase evidence log (Command / Observed / Artifacts)"
    gate:
      - file_exists: "docs/features/*/goal-sheet.md"
      - file_exists: "docs/features/*/evidence.md"
    validate:
      - "goal-sheet.md has all 5 sections (Goal / In-scope / Out-of-scope / Acceptance criteria / Open risks), none empty"
      - "Every acceptance criterion is verifiable by a command, test, or observable behavior"
      - "evidence.md has an intake entry (Command / Observed / Artifacts)"
    max_retries: 3

  - id: design
    description: "Write design.md (architecture + key decisions), then spec-review it (see design SKILL.md, Phase: design)"
    consumes:
      - goal_sheet
    produces:
      - name: design_doc
        path: "docs/features/*/design.md"
        description: "Architecture and key decisions (rejected alternatives one line each)"
      - name: findings-spec
        path: "belt://current/review/findings-spec.json"
        description: "Consolidated spec review findings"
    gate:
      - file_exists: "docs/features/*/design.md"
      - file_exists: "belt://current/review/findings-spec.json"
    validate:
      - "design.md contains Architecture and Key Decisions sections and no task list (tasks live in plan.md)"
      - "Every in-scope item in goal-sheet.md is addressed by the Architecture section"
      - "Every critical/high finding in findings-spec.json is fixed or explicitly rejected by the user"
      - "evidence.md has a design entry"
    confirm: true
    max_retries: 3
```

- [ ] **Step 5: build/pipeline.yml を書き換え**

`plugins/belt/skills/build/pipeline.yml` 全文を以下で置換(e2e/integrate phase 除去、confirm 除去、plan.md 参照、autonomous triage):

```yaml
name: build
version: 1
description: "Shared build stage (execute -> code-review)"

args:
  codex:
    type: bool
    default: false
    description: "Enable Codex parallel review in code-review"

phases:
  - id: execute
    description: "Implement the plan document's task list via TDD subagents (see build SKILL.md, Phase: execute)"
    produces:
      - name: evidence
        path: "docs/features/*/evidence.md"
        description: "Per-phase evidence log"
    gate:
      - file_exists: "docs/features/*/evidence.md"
    validate:
      - "Every checkbox in the plan document's task list (plan.md Tasks, or fix-plan.md for bug runs) is checked"
      - "Project test suite passes; command and result recorded in evidence.md's execute entry"
      - "Project linter passes; command and result recorded in evidence.md's execute entry"
    max_retries: 3

  - id: code-review
    description: "Two-agent parallel code review with autonomous triage (critical/high auto-fixed or deferred with reason)"
    invoke:
      skill: /belt:code-review
      args:
        codex: "args.codex"
    produces:
      - name: findings-code
        path: "belt://current/review/findings-code.json"
        description: "Security + impact findings"
      - name: findings-quality
        path: "belt://current/review/findings-quality.json"
        description: "Test + AI-antipattern findings"
      - name: findings-codex
        path: "belt://current/review/findings-codex.json"
        description: "Codex adversarial findings"
        when: "args.codex"
      - name: findings
        path: "belt://current/review/findings.json"
        description: "Merged findings"
    gate:
      - file_exists: "belt://current/review/findings.json"
    validate:
      - "Every critical/high finding in findings.json is fixed, or recorded as deferred with a reason in evidence.md's code-review entry"
      - "Linter and test suite pass after fixes; recorded in evidence.md's code-review entry"
    max_retries: 3
```

- [ ] **Step 6: diagnose/pipeline.yml を書き換え**

`plugins/belt/skills/diagnose/pipeline.yml` 全文を以下で置換(e2e arg 除去、rca_scenarios を常時 produce、confirm を fix-plan-review のみに):

```yaml
name: diagnose
version: 1
description: "Bug diagnosis stage (rca -> fix-plan -> fix-plan-review)"

args:
  codex:
    type: bool
    default: false
    description: "Enable Codex parallel review in fix-plan-review"

phases:
  - id: rca
    description: "Investigate root cause via parallel exploration"
    invoke:
      skill: /systematic-debugging
    produces:
      - name: rca_report
        path: "docs/features/*/rca-report.md"
        description: "Root cause analysis report (Symptom / Investigation Record / Root Cause / Reproduction Test / Fix Strategy)"
      - name: rca_scenarios
        path: "docs/features/*/rca-scenarios.yml"
        description: "Reproduction scenarios in Given/When/Then YAML (kind: browser|cli) for QA replay"
      - name: rca_notes
        path: "belt://current/notes/phase-rca.md"
        description: "Phase narrative: decisions, concerns, directives, observations"
    gate:
      - file_exists: "docs/features/*/rca-report.md"
      - file_exists: "docs/features/*/rca-scenarios.yml"
      - file_exists: "belt://current/notes/phase-rca.md"
    validate: ./criteria/rca.md
    max_retries: 3

  - id: fix-plan
    description: "Create fix plan from RCA report"
    invoke:
      skill: /writing-plans
    consumes:
      - rca_report
      - rca_notes
    produces:
      - name: fix_plan_doc
        path: "docs/features/*/fix-plan.md"
        description: "Fix plan with RCA Fix Strategy -> task mapping"
      - name: fix_plan_notes
        path: "belt://current/notes/phase-fix-plan.md"
        description: "Phase narrative"
    gate:
      - file_exists: "docs/features/*/fix-plan.md"
      - file_exists: "belt://current/notes/phase-fix-plan.md"
    validate: ./criteria/fix-plan.md
    max_retries: 3

  - id: fix-plan-review
    description: "Plan review via spec-review (presents the RCA summary and the fix plan together for the single diagnosis approval)"
    invoke:
      skill: /belt:spec-review
      args:
        codex: "args.codex"
    consumes:
      - fix_plan_doc
    produces:
      - name: findings-spec
        path: "belt://current/review/findings-spec.json"
        description: "Consolidated spec review findings"
      - name: findings-codex
        path: "belt://current/review/findings-codex.json"
        description: "Codex adversarial spec review findings"
        when: "args.codex"
    gate:
      - file_exists: "belt://current/review/findings-spec.json"
    validate: ./criteria/fix-plan-review.md
    confirm: true
    max_retries: 3
```

- [ ] **Step 7: feature-dev / bug-fix pipeline.yml を書き換え**

`plugins/belt/skills/feature-dev/pipeline.yml` 全文:

```yaml
name: feature-dev
version: 1
description: "Quality-gated development pipeline (composed: design -> plan -> checkpoint -> build -> qa -> integrate)"

args:
  codex:
    type: bool
    default: false
    description: "Enable Codex parallel review in spec review, plan review, and code-review"

phases:
  - id: design
    invoke:
      pipeline: ../design/pipeline.yml
      with:
        codex: "args.codex"

  - id: plan
    invoke:
      pipeline: ../plan/pipeline.yml
      with:
        codex: "args.codex"

  - id: pre-execute-handover
    invoke:
      pipeline: ../handover/checkpoint.yml

  - id: build
    invoke:
      pipeline: ../build/pipeline.yml
      with:
        codex: "args.codex"

  - id: qa
    invoke:
      pipeline: ../qa/pipeline.yml

  - id: integrate
    description: "Integrate changes (user chooses: wt merge or gh pr create); publish QA evidence (see orchestrator SKILL.md)"
    invoke:
      skill: /worktrunk
    validate:
      - "User explicitly chose the integration mode (wt merge or gh pr create)"
      - "Deferred findings, accepted FAILs, QA fix commits, and exploratory advisories were presented to the user"
      - "QA evidence was published to the destination (PR comment / Linear), or the local-only fallback was explicitly reported"
      - "evidence.md has one entry per completed phase, ending with integrate"
    confirm: true
    max_retries: 3
```

`plugins/belt/skills/bug-fix/pipeline.yml` 全文(integrate leaf は feature-dev と **1 字違わず同一** — lock test が serde 同値を assert する):

```yaml
name: bug-fix
version: 1
description: "Quality-gated debugging pipeline (composed: diagnose -> checkpoint -> build -> qa -> integrate)"

args:
  codex:
    type: bool
    default: false
    description: "Enable Codex parallel review in fix-plan-review and code-review"

phases:
  - id: diagnose
    invoke:
      pipeline: ../diagnose/pipeline.yml
      with:
        codex: "args.codex"

  - id: pre-execute-handover
    invoke:
      pipeline: ../handover/checkpoint.yml

  - id: build
    invoke:
      pipeline: ../build/pipeline.yml
      with:
        codex: "args.codex"

  - id: qa
    invoke:
      pipeline: ../qa/pipeline.yml

  - id: integrate
    description: "Integrate changes (user chooses: wt merge or gh pr create); publish QA evidence (see orchestrator SKILL.md)"
    invoke:
      skill: /worktrunk
    validate:
      - "User explicitly chose the integration mode (wt merge or gh pr create)"
      - "Deferred findings, accepted FAILs, QA fix commits, and exploratory advisories were presented to the user"
      - "QA evidence was published to the destination (PR comment / Linear), or the local-only fallback was explicitly reported"
      - "evidence.md has one entry per completed phase, ending with integrate"
    confirm: true
    max_retries: 3
```

- [ ] **Step 8: diagnose criteria / supplement の e2e 条件を常時化**

1. `plugins/belt/skills/diagnose/criteria/rca.md` の `### RCA-09` ブロック(現行 100-112 行)を以下で置換:

```markdown
### RCA-09: Reproduction scenarios file exists

- **check**:
  1. Search for the scenarios file using `Glob("docs/features/*/rca-scenarios.yml")`
  2. Verify it contains at least one Given/When/Then scenario and every scenario has `kind: browser` or `kind: cli`
- **pass_condition**: file exists with ≥1 scenario, each carrying a kind
- **fail_diagnosis_hint**: If the file is missing, the RCA executor did not load `rca-supplement.md`. Confirm supplement injection in the diagnose SKILL.md rca invocation
- **depends_on_artifacts**: [docs/features/*/rca-scenarios.yml]
- **forward_check**: the qa phase replays `rca_scenarios` via belt:qa-verifier
```

2. `plugins/belt/skills/diagnose/references/rca-supplement.md` の `## \`--e2e\` additional output` セクション(現行 48-56 行)を以下で置換(`--e2e` 条件と、既に存在しない `monkey-test-supplement.md` への参照を除去):

```markdown
## Reproduction scenarios output

Always produce, alongside the RCA report:

    docs/features/<YYYY-MM-DD-topic>/rca-scenarios.yml

Content: Given/When/Then YAML with at least one scenario, each carrying
`kind: browser` or `kind: cli` (schema: `plugins/belt/skills/plan/SKILL.md`).
The first scenario MUST correspond to the RCA Reproduction Test.
```

3. `plugins/belt/skills/diagnose/references/fix-plan-supplement.md:63` の行を以下で置換:

```markdown
- Reference `rca-scenarios.yml` so the qa phase can extend the scenarios list with fix-specific Given/When/Then entries
```

- [ ] **Step 9: belt-agent boots test を修正(main 上の既存 FAIL の fold-in)**

`crates/belt-agent/tests/cli_test.rs` の `feature_dev_migrated_pipeline_boots` 内、init の引数配列を以下に変更:

```rust
        .args(["init", pipeline.to_str().unwrap(), "--arg", "codex=false"])
```

同テスト末尾の assert 2 箇所を以下に変更:

```rust
    assert_eq!(
        next_json["phase"]["id"].as_str(),
        Some("design/intake"),
        "first phase should be the expanded leaf 'design/intake'"
    );
    // The phase should carry the new invoke shape.
    let invoke = &next_json["phase"]["invoke"];
    assert!(invoke.is_object(), "invoke must be present");
    assert_eq!(invoke["skill"].as_str(), Some("/belt:goal"));
```

- [ ] **Step 10: テスト green + lint を確認**

Run: `cargo test -p belt-core --test feature_dev_refresh --test bug_fix_refresh`
Expected: PASS(feature_dev 9 tests + bug_fix 9 tests)

Run: `cargo test -p belt-core 2>&1 | grep -E "FAILED|test result: ok" | tail -3`
Expected: 全 suite ok(他の lock/契約テストへの回帰なし)

Run: `cargo test -p belt-agent 2>&1 | tail -3`
Expected: 54 passed; 0 failed(既存 FAIL の解消)

Run: `for f in design build diagnose feature-dev bug-fix plan qa; do cargo run -q -p belt -- lint plugins/belt/skills/$f/pipeline.yml || echo "LINT FAIL: $f"; done`
Expected: LINT FAIL 出力なし

- [ ] **Step 11: lock-ledger.md を更新**

`docs/testing/lock-ledger.md` の `## feature_dev_refresh.rs` と `## bug_fix_refresh.rs` の 2 entry を、Step 1-2 の新テストに合わせて書き換える:
- test fn 名リストを新ファイルの `#[test]` fn 名と一致させる(feature_dev 9 件: `feature_dev_composes_six_stages` / `stages_delegate_with_codex_passthrough` / `checkpoint_and_qa_delegate_with_no_args` / `top_level_args_are_codex_only` / `feature_dev_expands_to_eight_namespaced_leaves` / `no_leaf_declares_regate_or_when` / `confirm_leaves_match_the_four_touchpoints` / `integrate_leaf_identical_across_orchestrators` / `feature_dev_pipeline_has_no_run_id_template`。bug_fix 9 件: `bug_fix_composes_five_stages` / `stages_delegate_with_codex_passthrough` / `checkpoint_and_qa_delegate_with_no_args` / `args_are_codex_only` / `no_legacy_args` / `bug_fix_expands_to_eight_namespaced_leaves` / `no_leaf_declares_regate_or_when` / `confirm_leaves_match_the_three_touchpoints` / `bug_fix_pipeline_has_no_run_id_template`)
- `test-fn-count:` を 9 / 9 に更新
- shape dimensions 節を新契約(args codex のみ / 6 top-level / 8 leaves / confirm 4 leaf / integrate 同一性 / when 全廃)で書き換える
- 日付言及は `2026-07-07 four-stage rewrite` とする

Run: `cargo test -p belt-core --test scenarios_contract`
Expected: PASS(locks-file 照合が通る)

- [ ] **Step 12: fmt / clippy / Commit(1 コミット)**

```bash
cargo fmt --package belt-core --package belt-agent
cargo clippy --package belt-core --package belt-agent -- -D warnings
git add crates/belt-core/tests/feature_dev_refresh.rs crates/belt-core/tests/bug_fix_refresh.rs \
  crates/belt-agent/tests/cli_test.rs plugins/belt/skills/design/pipeline.yml \
  plugins/belt/skills/build/pipeline.yml plugins/belt/skills/diagnose/pipeline.yml \
  plugins/belt/skills/feature-dev/pipeline.yml plugins/belt/skills/bug-fix/pipeline.yml \
  plugins/belt/skills/diagnose/criteria/rca.md plugins/belt/skills/diagnose/references/ \
  docs/testing/lock-ledger.md
git commit -m "feat(belt): cut over to four-stage composition (plan/qa stages, codex-only args, 4 confirm touchpoints)

- feature-dev: design -> plan -> checkpoint -> build -> qa -> integrate (8 leaves)
- bug-fix: diagnose -> checkpoint -> build -> qa -> integrate (8 leaves)
- drop --e2e everywhere (QA mandatory, D2); rca-scenarios always authored
- integrate moved out of build; identical leaf locked across orchestrators (D14)
- fix stale feature_dev_migrated_pipeline_boots (pre-existing failure on main)"
```

---

### Task 9: ステージ SKILL.md 改稿 + 参照ドキュメント同期

**Files:**
- Rewrite: `plugins/belt/skills/design/SKILL.md`
- Rewrite: `plugins/belt/skills/build/SKILL.md`
- Rewrite: `plugins/belt/skills/feature-dev/SKILL.md`
- Rewrite: `plugins/belt/skills/bug-fix/SKILL.md`
- Modify: `plugins/belt/skills/diagnose/SKILL.md`(frontmatter + supplement 表の 1 行)
- Modify: `plugins/belt/skills/design/references/path-convention.md`(file layout 表)
- Modify: `plugins/belt-agent/references/authoring-principles.md`(QA evidence 節追記)

**Interfaces:**
- Consumes: Task 8 の新 pipeline トポロジー
- Produces: 各ステージの実行手順プロース。`/belt:verify` への参照ゼロ(Task 10 の削除前提)、`feature-implementer` への参照ゼロ(Task 10 の削除前提)

- [ ] **Step 1: design/SKILL.md を書き換え**

`plugins/belt/skills/design/SKILL.md` 全文:

```markdown
---
name: design
description: >-
  Runs the feature design stage: goal-sheet intake (/belt:goal) followed
  by a design document (architecture + key decisions) reviewed by
  belt:spec-reviewer. Use standalone for design-only work, or composed
  as the first stage of /belt:feature-dev. Task breakdown lives in the
  plan stage (/belt:plan), not here. --codex enables adversarial spec
  review.
user-invocable: true
argument-hint: "<linear-id | url | free-text> [--codex]"
---

# design

Belt pipeline for the design stage. Structure, gates, and done criteria
live in `pipeline.yml`; this file defines how to execute each phase.

## Phase: intake

Invoke `/belt:goal`, passing the user's original input (ticket id, URL,
free text, or a requirements.md path) verbatim. The skill writes
goal-sheet.md and evidence.md.

## Phase: design

This phase has no `invoke` — execute these steps directly:

1. Read goal-sheet.md (resolve the path via `belt-agent status`,
   artifact `goal_sheet`).
2. Explore the code the change touches with Grep/Read. Dispatch
   `belt-agent:explorer` subagents (focus: flow or patterns) only if
   the area is unfamiliar AND spans 10+ files.
3. Write `docs/features/<topic>/design.md` with exactly these sections:
   - `## Architecture` — approach, components, data flow
   - `## Key Decisions` — each decision with a one-line rationale;
     rejected alternatives in one line each
   No task list and no test strategy here — both live in plan.md
   (/belt:plan).
4. Invoke `/belt:spec-review` with the design.md path as the target
   (pass `--codex` if the codex arg is true) and complete its batched
   triage.
5. Append the design entry to evidence.md (format:
   `plugins/belt-agent/references/authoring-principles.md`).

## Red flags

- Never ask the user design questions one at a time — batch remaining
  open points in one AskUserQuestion call.
- Never write an Implementation Tasks section in design.md — task
  breakdown belongs to /belt:plan.
- Never hand-edit files under `docs/features/<topic>/` after a phase
  completes (breaks the phase-start mtime filter).

## References

- `./references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `plugins/belt-agent/references/authoring-principles.md` — evidence entry format
```

- [ ] **Step 2: build/SKILL.md を書き換え**

`plugins/belt/skills/build/SKILL.md` 全文:

```markdown
---
name: build
description: >-
  Runs the shared build stage: TDD implementation from the plan
  document's task list and two-agent code review with autonomous triage.
  Use standalone with an existing plan.md/fix-plan.md, or composed as
  the build stage of /belt:feature-dev and /belt:bug-fix. QA runs as its
  own stage (/belt:qa) after build; integration happens at the
  orchestrator's integrate phase.
user-invocable: true
argument-hint: "[--codex]"
---

# build

Belt pipeline for the build stage. Structure, gates, and done criteria
live in `pipeline.yml`; this file defines how to execute each phase.

## Entry check

A plan document must exist: `docs/features/<topic>/plan.md` with a
Tasks section (feature runs) or `docs/features/<topic>/fix-plan.md`
(bug runs). If neither exists, stop and ask the user. If
`docs/features/<topic>/evidence.md` does not exist, create it now with
the header `# Evidence: <topic>`.

## Phase: execute

This phase has no `invoke` — execute these steps directly:

1. Read the plan document's task list (Tasks in plan.md, or the task
   list in fix-plan.md).
2. For each unchecked task, dispatch ONE `belt-agent:implementer`
   subagent with a self-contained prompt containing: the task text, the
   exact file paths, the test(s) to write first, the relevant design
   constraints copied into the prompt (never "see the design doc"), and
   the project's test command.
3. After each subagent returns: run the project test suite yourself,
   check the task's checkbox in the plan document, and commit.
4. Tasks whose target files do not overlap MAY run in parallel;
   overlapping tasks run serially.
5. Append the execute entry to evidence.md (test + lint commands and
   observed results).

## Phase: code-review

Invoke `/belt:code-review` (pass `--codex` if the codex arg is true).
In pipeline mode its triage is autonomous: critical/high findings are
fixed and committed, or recorded as deferred with a reason. Append the
code-review entry to evidence.md including the deferred list.

## Red flags

- Never start execute without the Entry check.
- Never forward the whole plan document to implementer subagents — copy
  the relevant constraints into each prompt.
- Never skip the per-task test run after a subagent returns.
- Never let subagents write evidence.md — orchestrator only.

## References

- `plugins/belt/skills/design/references/path-convention.md` — naming rules (SSOT)
- `plugins/belt-agent/references/authoring-principles.md` — evidence entry format
```

- [ ] **Step 3: feature-dev/SKILL.md を書き換え**

`plugins/belt/skills/feature-dev/SKILL.md` 全文:

```markdown
---
name: feature-dev
description: >-
  Quality-gated feature pipeline from ticket to integration: goal-sheet
  intake, design document with spec review, implementation plan with QA
  scenarios, context-reset checkpoint, TDD implementation, autonomous
  code review, mandatory QA with human-readable evidence (screenshots /
  transcripts), and integration with evidence publishing. Accepts a
  Linear id, URL, free text, or a requirements.md path. --codex enables
  adversarial review.
user-invocable: true
argument-hint: "<linear-id | url | free-text | requirements.md path> [--codex]"
---

# feature-dev

Composed pipeline: design → plan → checkpoint → build → qa → integrate.
`belt-agent init` expands the five `invoke.pipeline` references into
namespaced leaves (`design/intake` ... `qa/qa`) plus the `integrate`
leaf in a single run.

Keep the user's original task input (ticket id, URL, free text, or
requirements.md path): the `design/intake` phase passes it verbatim to
`/belt:goal`.

Human touchpoints are exactly four: design approval, plan approval, the
checkpoint pause, and integrate. build and qa run autonomously; their
leftovers (deferred findings, accepted FAILs, QA fix commits,
exploratory advisories) are reported at integrate.

## Stage skills

When `next` returns a phase, read the owning stage's SKILL.md before
executing it:

- `design/*` → `plugins/belt/skills/design/SKILL.md`
- `plan/*` → `plugins/belt/skills/plan/SKILL.md`
- `pre-execute-handover/*` → run `/belt:handover`, then `/clear`, then
  `/belt:resume` in the new session
- `build/*` → `plugins/belt/skills/build/SKILL.md`
- `qa/*` → `plugins/belt/skills/qa/SKILL.md`
- `integrate` → this file, below

## Phase: integrate

Ask the user once: A) `wt merge` or B) `gh pr create`, presenting in
the same message the deferred findings, accepted FAILs, QA fix commits,
and exploratory advisories collected in evidence.md. Invoke `/worktrunk`
with the chosen mode. Then publish QA evidence per the `[qa] evidence`
config (see `plugins/belt/skills/qa/SKILL.md`):

- PR route: push the QA evidence directory to the `qa-evidence` orphan
  branch under `<run-id>/`, then post one PR comment containing the
  qa-report scenario table with evidence links (public repos: inline
  image embeds via raw URLs; private repos: blob URL links).
- No PR and Linear attachment not done in qa: report the local evidence
  path to the user with an explicit warning.

Record the published destination URL in evidence.md's integrate entry.

## Red flags

- Never execute a stage phase without its stage SKILL.md loaded.
- Never bypass the pre-execute-handover checkpoint — the context reset
  before execute is the pipeline's core ergonomics.
- Never merge or create the PR before qa-report.md exists.

## Smaller runs

`/belt:design` (design only), `/belt:plan` (plan from an existing
design), `/belt:build` (plan already exists), `/belt:qa` (QA only),
`/belt:goal` (intake only), `/belt:requirements` (requirements
document, upstream of this pipeline).
```

- [ ] **Step 4: bug-fix/SKILL.md を書き換え**

`plugins/belt/skills/bug-fix/SKILL.md` 全文:

```markdown
---
name: bug-fix
description: >-
  Runs a quality-gated debugging pipeline with root-cause analysis, fix
  planning, TDD repair, autonomous code review, mandatory QA replay of
  the reproduction scenarios with human-readable evidence, and
  integration. Use when a bug needs structured diagnosis and verified
  repair. --codex enables adversarial review.
user-invocable: true
argument-hint: "[--codex]"
---

# bug-fix

Composed belt pipeline: diagnose → checkpoint → build → qa → integrate.
`pipeline.yml` declares four `invoke.pipeline` references plus the
`integrate` leaf; `belt-agent init` expands them inline, so `next`
returns namespaced leaf phases (`diagnose/rca`, `build/execute`,
`qa/qa`, ...) in a single run.

Human touchpoints are exactly three: the diagnosis approval
(fix-plan-review presents the RCA summary and the fix plan together),
the checkpoint pause, and integrate.

## Stage Skills

When `next` returns a phase, load the owning stage's SKILL.md before
executing it — entry checks, phase execution steps, and red flags live
there:

- `diagnose/*` → `plugins/belt/skills/diagnose/SKILL.md`
- `pre-execute-handover/*` → (none — follow the phase description:
  `/belt:handover`, `/clear`, `/belt:resume`)
- `build/*` → `plugins/belt/skills/build/SKILL.md`
- `qa/*` → `plugins/belt/skills/qa/SKILL.md` (replays
  rca-scenarios.yml)
- `integrate` → `plugins/belt/skills/feature-dev/SKILL.md`, Phase:
  integrate (the leaf definition is identical by contract)

Smaller runs: `/belt:diagnose` for diagnosis-only work, `/belt:build`
when a fix plan already exists, `/belt:qa` for QA alone.

## Red Flags

- **Never execute a stage phase without loading its stage SKILL.md.**
- **Never skip diagnose**: root cause must precede fix. "Fix first" is
  the anti-pattern (enforced by the diagnose stage's own red flags).
- **Never bypass the pre-execute-handover checkpoint.**

## References

- `plugins/belt/skills/diagnose/SKILL.md` — diagnose stage contract
- `plugins/belt/skills/build/SKILL.md` — build stage contract
- `plugins/belt/skills/qa/SKILL.md` — QA stage contract
- `plugins/belt/skills/design/references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
```

- [ ] **Step 5: diagnose/SKILL.md を最小改修**

1. frontmatter の `description` を以下で置換:

```yaml
description: >-
  Runs the bug diagnosis stage: root-cause analysis with a failing
  reproduction test and reproduction scenarios, fix planning, and
  adversarial plan review. Use standalone for diagnosis-only work, or
  composed as the upstream stage of /belt:bug-fix. --codex enables
  adversarial plan review.
```

2. `argument-hint` を `"[--codex]"` に変更。

3. Supplement 表の rca 行の Purpose 中 `` `rca-scenarios.yml` produce (when `--e2e`) `` を `` `rca-scenarios.yml` produce (always) `` に変更。

- [ ] **Step 6: path-convention.md の File Layout 表を更新**

`plugins/belt/skills/design/references/path-convention.md` の `## File Layout per Topic` 表を以下で置換:

```markdown
| File | Producing phase | Producer | When |
|------|-----------------|----------|------|
| `goal-sheet.md` | intake | /belt:goal | feature runs |
| `evidence.md` | intake | /belt:goal (later phases append); the build Entry check creates it for bug runs | feature runs; bug runs (from build) |
| `design.md` | design | /belt:design (Phase: design) | feature runs |
| `plan.md` | plan | /belt:plan (Phase: plan) | feature runs |
| `scenarios.yml` | plan | /belt:plan (Phase: plan) | feature runs, always |
| `rca-report.md` | rca | /systematic-debugging | bug runs |
| `rca-scenarios.yml` | rca | /systematic-debugging | bug runs, always |
| `fix-plan.md` | fix-plan | /writing-plans | bug runs |
| `qa-report.md` | qa | /belt:qa (belt:qa-verifier) | always |
```

同ファイル frontmatter の `(design / diagnose / build / verify)` を `(design / plan / diagnose / build / qa)` に変更。表直後の説明段落の `e2e-report.md` 言及があれば `qa-report.md` に置換し、「QA 証跡バイナリは run directory 配下(非コミット)、qa-report.md のみ commit 対象」の 1 文を追加。

- [ ] **Step 7: authoring-principles.md に QA evidence 節を追記**

`plugins/belt-agent/references/authoring-principles.md` の `## Evidence entries` 節の末尾に追加:

```markdown
## QA evidence

QA evidence binaries (screenshots, transcripts) live under the run
directory's `qa/` subdirectory — never under `docs/` and never
committed. `docs/features/<topic>/qa-report.md` (text) is the committed
index; it references evidence by run-relative path. Publishing to
PR/Linear is governed by the `[qa] evidence` key in belt.toml
(interpretation rules: `plugins/belt/skills/qa/SKILL.md`).
```

- [ ] **Step 8: 参照整合を確認**

Run: `grep -rn "belt:verify\|feature-implementer\|Implementation Tasks" plugins/belt/skills/{design,build,feature-dev,bug-fix,plan,qa,diagnose}/SKILL.md; echo exit=$?`
Expected: `exit=1`(ヒットなし。design.md の旧 Implementation Tasks 参照が残っていないこと)

Run: `cargo test -p belt-core --test review_skills_refresh --test shared_filter_parity`
Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add plugins/belt/skills/design/SKILL.md plugins/belt/skills/build/SKILL.md \
  plugins/belt/skills/feature-dev/SKILL.md plugins/belt/skills/bug-fix/SKILL.md \
  plugins/belt/skills/diagnose/SKILL.md plugins/belt/skills/design/references/path-convention.md \
  plugins/belt-agent/references/authoring-principles.md
git commit -m "docs(belt): rewrite stage SKILL.md prose for four-stage topology; sync path-convention"
```

---

### Task 10: Cleanup — 旧 agent 5 + verify skill + audit-protocol 削除、lock 拡張

**Files:**
- Modify: `crates/belt-core/tests/review_skills_refresh.rs`(agent bundle lock 拡張)
- Delete: `plugins/belt-agent/agents/code-explorer.md`, `code-architect.md`, `impact-analyzer.md`, `feature-implementer.md`, `phase-auditor.md`
- Delete: `plugins/belt-agent/references/audit-protocol.md`
- Delete: `plugins/belt/skills/verify/`(ディレクトリごと)
- Modify: `plugins/belt-agent/references/_schema.md`(phase-auditor 参照の置換)
- Modify: `docs/testing/lock-ledger.md`(review_skills_refresh entry)

**Interfaces:**
- Consumes: Task 3-4 の新 agent(explorer / implementer / qa-verifier が存在すること)
- Produces: agent 構成 8→6 の確定。`review_skills_refresh.rs` が新旧 agent 構成を恒久 lock

- [ ] **Step 1: review_skills_refresh.rs に agent bundle lock を追加(failing test 先行)**

`crates/belt-core/tests/review_skills_refresh.rs` の module doc の末尾に以下を追記:

```rust
//! - 2026-07-07 four-stage rewrite: the pipeline agent bundle is locked —
//!   belt = {code-reviewer, quality-reviewer, spec-reviewer, qa-verifier},
//!   belt-agent = {explorer, implementer}; the five pre-rewrite belt-agent
//!   agents and audit-protocol.md are DELETED, and /belt:verify is replaced
//!   by /belt:qa
```

`CONSOLIDATED_AWAY` 定数の直後に以下を追加:

```rust
const BELT_AGENT_BUNDLE: &[&str] = &["explorer", "implementer"];

const BELT_BUNDLE_EXTRA: &[&str] = &["qa-verifier"];

const RETIRED_BELT_AGENT_AGENTS: &[&str] = &[
    "code-explorer",
    "code-architect",
    "impact-analyzer",
    "feature-implementer",
    "phase-auditor",
];
```

ファイル末尾に以下のテストを追加:

```rust
#[test]
fn pipeline_agent_bundle_exists() {
    for agent in BELT_AGENT_BUNDLE {
        let path = repo_root()
            .join("plugins/belt-agent/agents")
            .join(format!("{agent}.md"));
        assert!(
            path.exists(),
            "belt-agent bundle agent must exist: {}",
            path.display()
        );
    }
    for agent in BELT_BUNDLE_EXTRA {
        let path = repo_root()
            .join("plugins/belt/agents")
            .join(format!("{agent}.md"));
        assert!(
            path.exists(),
            "belt bundle agent must exist: {}",
            path.display()
        );
    }
}

#[test]
fn retired_belt_agent_agents_are_deleted() {
    for name in RETIRED_BELT_AGENT_AGENTS {
        let path = repo_root()
            .join("plugins/belt-agent/agents")
            .join(format!("{name}.md"));
        assert!(
            !path.exists(),
            "retired belt-agent agent file must be deleted: {}",
            path.display()
        );
    }
    let audit_protocol = repo_root().join("plugins/belt-agent/references/audit-protocol.md");
    assert!(
        !audit_protocol.exists(),
        "audit-protocol.md must be deleted (dead wiring): {}",
        audit_protocol.display()
    );
}

#[test]
fn verify_skill_is_replaced_by_qa() {
    let verify_dir = repo_root().join("plugins/belt/skills/verify");
    assert!(
        !verify_dir.exists(),
        "verify skill directory must be deleted: {}",
        verify_dir.display()
    );
    let qa_skill = repo_root().join("plugins/belt/skills/qa/SKILL.md");
    assert!(
        qa_skill.exists(),
        "qa skill must exist as the replacement: {}",
        qa_skill.display()
    );
}

#[test]
fn qa_verifier_uses_evidence_dir_arg_pattern() {
    let path = repo_root().join("plugins/belt/agents/qa-verifier.md");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    assert!(
        content.contains("evidence_dir"),
        "qa-verifier.md must reference the 'evidence_dir' runtime arg"
    );
    assert!(
        !content.contains(".belt/runs/"),
        "qa-verifier.md must not hardcode .belt/runs/ literals"
    );
}
```

Run: `cargo test -p belt-core --test review_skills_refresh 2>&1 | tail -3`
Expected: FAIL(`retired_belt_agent_agents_are_deleted` と `verify_skill_is_replaced_by_qa` が落ちる — 削除前)

- [ ] **Step 2: ファイル削除 + _schema.md 修正**

```bash
git rm plugins/belt-agent/agents/code-explorer.md plugins/belt-agent/agents/code-architect.md \
  plugins/belt-agent/agents/impact-analyzer.md plugins/belt-agent/agents/feature-implementer.md \
  plugins/belt-agent/agents/phase-auditor.md plugins/belt-agent/references/audit-protocol.md
git rm -r plugins/belt/skills/verify/
```

`plugins/belt-agent/references/_schema.md` の phase-auditor 言及 3 箇所を置換:
- 冒頭 description の `consumed by the phase-auditor subagent during audit phases` → `consumed by the orchestrator when evaluating validate: file criteria`
- 本文 `Each done-criteria file is consumed by the `phase-auditor` subagent during audit phases.` → `Each done-criteria file is consumed by the orchestrator when it evaluates a phase's validate: file reference (the audit-gate pattern is retired).`
- `The phase-auditor MUST include ...` で始まる行(および `fail_diagnosis_hint` 表内の `for auditor` 文言)→ 主語を `the orchestrator` に置換

- [ ] **Step 3: orphan 参照ゼロを確認**

Run: `grep -rn "phase-auditor\|audit-protocol\|feature-implementer\|code-explorer\|code-architect\|impact-analyzer\|belt:verify" plugins/ ; echo exit=$?`
Expected: `exit=1`(ヒットなし)

Run: `cargo test -p belt-core --test review_skills_refresh`
Expected: PASS(10 tests)

- [ ] **Step 4: lock-ledger.md の review_skills_refresh entry を更新**

`docs/testing/lock-ledger.md` に `## review_skills_refresh.rs` entry があれば test-fn 一覧に Step 1 の 4 fn を追加し、なければ既存 entry 形式(locks-file / test-fn-count / cross-coupling)で追記する。`scenarios_contract` の機械照合を再実行:

Run: `cargo test -p belt-core --test scenarios_contract`
Expected: PASS

- [ ] **Step 5: fmt / clippy / Commit**

```bash
cargo fmt --package belt-core
cargo clippy --package belt-core -- -D warnings
git add crates/belt-core/tests/review_skills_refresh.rs plugins/belt-agent/references/_schema.md docs/testing/lock-ledger.md
git commit -m "feat(belt): retire 5 pre-rewrite agents, audit-protocol, and /belt:verify; lock the 6-agent bundle"
```

---

### Task 11: plugin.json 0.4.0 + README / AGENTS.md / CHANGELOG

**Files:**
- Modify: `plugins/belt/.claude-plugin/plugin.json`
- Modify: `plugins/belt-agent/.claude-plugin/plugin.json`
- Modify: `README.md`
- Modify: `AGENTS.md`(CLAUDE.md は symlink)
- Modify: `CHANGELOG.md`

**Interfaces:**
- Produces: 0.4.0 リリース準備完了状態のメタデータ・ドキュメント

- [ ] **Step 1: plugin.json を 2 件更新**

`plugins/belt/.claude-plugin/plugin.json`:

```json
{
  "name": "belt",
  "description": "User-invocable skills and their agents: /belt:feature-dev, /belt:bug-fix, /belt:requirements, /belt:docs, /belt:goal, /belt:design, /belt:plan, /belt:build, /belt:qa, /belt:code-review (2 reviewers), /belt:spec-review (1 reviewer), /belt:handover, /belt:resume, plus the qa-verifier agent. Requires belt-agent plugin",
  "version": "0.4.0",
  "author": { "name": "neko-neko" }
}
```

`plugins/belt-agent/.claude-plugin/plugin.json`:

```json
{
  "name": "belt-agent",
  "description": "Foundation: Belt Protocol skill (driver for belt-agent CLI) + 2 analysis agents (explorer, implementer) + shared references",
  "version": "0.4.0",
  "author": { "name": "neko-neko" }
}
```

- [ ] **Step 2: CHANGELOG.md の [Unreleased] に追記**

`CHANGELOG.md` の `<!-- next-header -->` 直後の `## [Unreleased]` 節(なければマーカー直後に新設)に追加:

```markdown
### Added

- `/belt:plan` stage skill — implementation planning (plan.md + scenarios.yml) split out of design (D1)
- `/belt:qa` stage skill — mandatory QA with human-readable evidence: browser screenshots / cli transcripts captured by the new independent `belt:qa-verifier` agent (D2, D3)
- `/belt:requirements` — interview-driven requirements definition writing docs/requirements/
- `/belt:docs` — general-purpose documentation writing under docs/
- `belt-agent:explorer` — unified codebase explorer (focus: flow | patterns | impact)
- `[qa] evidence` belt.toml key — evidence destination: pr | linear | local | auto (D9)

### Changed

- **BREAKING**: `--e2e` removed from feature-dev / bug-fix / design / diagnose / build — QA is always on (D2)
- feature-dev is now design → plan → checkpoint → build → qa → integrate; bug-fix is diagnose → checkpoint → build → qa → integrate; integrate moved out of build (D5)
- Human confirms reduced to design approval, plan approval, checkpoint, integrate (feature) / diagnosis approval, checkpoint, integrate (bug) (D4)
- `/belt:code-review` triage is autonomous in pipeline mode (critical/high auto-fixed or deferred with reason)
- `/belt:spec-review` accepts a target path and an output artifact name / directory
- `belt-agent:feature-implementer` renamed to `belt-agent:implementer`
- rca-scenarios.yml is always authored (was `--e2e` gated)

### Removed

- **BREAKING**: `/belt:verify` (replaced by `/belt:qa`)
- **BREAKING**: agents `belt-agent:code-explorer`, `belt-agent:code-architect`, `belt-agent:impact-analyzer`, `belt-agent:phase-auditor` (consolidated into `belt-agent:explorer` / retired), and `references/audit-protocol.md`
```

- [ ] **Step 3: README.md を更新**

`grep -n "verify\|e2e\|feature-implementer\|phase-auditor\|code-explorer\|code-architect\|impact-analyzer" README.md` でヒットした各行を確認し、以下の終了状態にする:

- skill 一覧表・パイプライン図・使用例に `/belt:plan` `/belt:qa` `/belt:requirements` `/belt:docs` が載っている
- `/belt:verify` と `--e2e` の言及が 0 件
- agent 一覧が explorer / implementer / spec-reviewer / code-reviewer / quality-reviewer / qa-verifier の 6 体
- feature-dev のステージ列挙が design → plan → checkpoint → build → qa → integrate
- QA 証跡の 1 段落(run dir 生成・PR/Linear 公開・`[qa] evidence` キー)を Install/Usage 近くの適切な節に追加

検証: `grep -c "belt:qa" README.md`(≥1)、`grep -c "\-\-e2e" README.md`(0)、`grep -c "belt:verify" README.md`(0)

- [ ] **Step 4: AGENTS.md を更新**

対象箇所(`grep -n "verify\|e2e\|criteria" AGENTS.md` で位置特定):

- 「CLI 体系」行はそのまま(belt-agent CLI の verify サブコマンドは無関係のため触らない)
- Plugin Architecture 節に「agents 6 体(belt: 3 reviewer + qa-verifier / belt-agent: explorer + implementer)」を反映
- `/belt:code-review` 等の fully-qualified 例はそのまま
- belt プラグイン説明で `/belt:verify` に言及していれば `/belt:qa` に置換

検証: `grep -c "belt:verify" AGENTS.md`(0)

- [ ] **Step 5: Commit**

```bash
git add plugins/belt/.claude-plugin/plugin.json plugins/belt-agent/.claude-plugin/plugin.json \
  CHANGELOG.md README.md AGENTS.md
git commit -m "chore(belt): bump plugins to 0.4.0; sync README/AGENTS/CHANGELOG for four-stage pipeline"
```

---

### Task 12: 最終検証(独立 verification + dogfood)

**Files:** なし(検証のみ。発見された欠陥は個別コミットで修正)

- [ ] **Step 1: workspace 全体チェック**

Run: `cargo fmt --all -- --check && cargo clippy --workspace -- -D warnings && cargo test --workspace 2>&1 | tail -5`
Expected: fmt/clippy クリーン、全テスト PASS(belt-agent 54 passed 含む)

- [ ] **Step 2: 全 pipeline lint**

Run: `for f in plugins/belt/skills/*/pipeline.yml plugins/belt/skills/handover/checkpoint.yml; do cargo run -q -p belt -- lint "$f" || echo "LINT FAIL: $f"; done`
Expected: LINT FAIL 出力なし(design / build / diagnose / feature-dev / bug-fix / plan / qa / checkpoint)

- [ ] **Step 3: dogfood — 展開実測(adversarial probe 含む)**

```bash
export REPO="$(git rev-parse --show-toplevel)"   # worktree root で実行
cargo build -q -p belt-agent
cd "$(mktemp -d)"
"$REPO/target/debug/belt-agent" init "$REPO/plugins/belt/skills/feature-dev/pipeline.yml" --arg codex=false
"$REPO/target/debug/belt-agent" status | python3 -c "import json,sys; d=json.load(sys.stdin); ps=d['phases']; print(len(ps)); print([p['id'] for p in ps])"
```

Expected: 8 leaves、順序 `design/intake, design/design, plan/plan, pre-execute-handover/checkpoint, build/execute, build/code-review, qa/qa, integrate`(status の JSON 形状が異なる場合は `belt-agent next` と組み合わせて leaf 列を確認する)

同様に bug-fix を init し 8 leaves を確認。adversarial probe:
- `belt-agent init ... --arg e2e=true` の挙動を記録(受理されても展開に `when` leaf が存在しないこと = e2e が完全に死んでいることを status で確認)
- feature-dev を `next` → 最初の phase が `design/intake` で invoke が `/belt:goal` であること

- [ ] **Step 4: 独立 verification(fresh subagent)**

fresh context のサブエージェントに以下を自己完結プロンプトで依頼し、結果を記録する:
「spec `docs/specs/2026-07-07-four-stage-agentic-pipeline-design.md` の D1〜D14 それぞれについて、実装(plugins/ と crates/belt-core/tests/)が満たしているかを検証し、D 番号ごとに PASS/FAIL と根拠(ファイル+行)を返せ。実行した確認コマンドと出力を含めること。」

Expected: D1〜D14 全て PASS。FAIL があれば修正して該当タスクのコミット規律で追加コミット。

- [ ] **Step 5: 完了報告**

検証出力(コマンドと結果)を添えて完了を報告する。`verified` の主張は Step 1-4 の実出力を伴うこと。

---

## Self-Review 済み事項

- **Spec coverage:** D1(Task 1, 8)/ D2(Task 2, 8)/ D3(Task 2, 4)/ D4(Task 8)/ D5(Task 8)/ D6(Task 3, 4, 10)/ D7(Task 6, 7)/ D8(Rust 変更は tests のみ — Task 8, 10)/ D9-D10(Task 2, 9 の qa/feature-dev SKILL.md)/ D11(Task 1 schema + Task 2)/ D12(Task 2 SKILL.md + Task 5 code-review)/ D13(Task 4 qa-verifier)/ D14(Task 8 identity lock)
- **既知の fold-in:** `feature_dev_migrated_pipeline_boots`(main 上で既に FAIL)を Task 8 Step 9 で修正。`rca-supplement.md` の dangling 参照(存在しない monkey-test-supplement.md)を Task 8 Step 8 で解消
- **型整合:** lock test の Phase フィールド(`confirm: bool` / `when: Option<String>` / `regate: Vec<String>`)と `serde_json::to_value(Phase)`(Phase は Serialize derive 済み)は `crates/belt-core/src/model.rs:38-63` で確認済み。`args` の HashMap iteration は要素 1 個(codex)のため sort 不要
- **順序依存:** Task 8 は Task 1-2 に依存(plan/qa pipeline の存在)。Task 10 は Task 3-4 に依存(新 agent の存在)。Task 9 は Task 8 の後(新トポロジー前提のプロース)。その他は記載順で実行
