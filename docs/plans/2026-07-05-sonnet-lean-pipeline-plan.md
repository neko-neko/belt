# Sonnet-lean Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** feature-dev を Sonnet で回る 7-leaf 構成(intake/design/checkpoint/execute/code-review/e2e/integrate)に圧縮し、/belt:goal(一括質問型 intake)・統合レビューア3体・evidence.md 証跡を導入する。

**Architecture:** belt-core(Rust)は無変更。プラグイン層のみ書き直す。criteria ファイルは pipeline.yml のインライン `validate:` に吸収、supplement 連鎖は廃止し各 SKILL.md を自己完結にする。superpowers の対話型スキル(brainstorming / writing-plans / subagent-driven-development)への依存を外し、belt 内で完結させる。lock tests(feature_dev_refresh / bug_fix_refresh / review_skills_refresh)は新形状に改訂する。

**Tech Stack:** YAML (belt pipeline) / Markdown (SKILL.md, agents) / Rust (belt-core integration tests のみ)。

**Spec:** `docs/specs/2026-07-05-sonnet-lean-pipeline-design.md` (approved)。1点逸脱あり: spec は execute を「/subagent-driven-development、無変更」としたが、design leaf が plan.md を作らなくなる(design.md の Implementation Tasks に吸収)ため、execute は `belt-agent:feature-implementer` 直接ディスパッチに書き直す(SDD の per-task 2段レビューは code-review phase と重複するため)。Task 7 で spec 側も1行修正する。

## Global Constraints

- belt-core production code (`crates/belt-core/src/`) は変更禁止。tests のみ改訂可
- plugins/ 配下は全て英語。docs/plans, docs/specs のみ日本語可
- Skill invoke / agent reference は常に fully-qualified(`/belt:code-review`, `belt-agent:feature-implementer`)。shorthand 禁止
- pipeline.yml / SKILL.md / agents に `.belt/runs/` リテラルと `{run_id}` テンプレートを書かない(lint が拒否)
- `with:` の値は bare `args.<name>` full-string form のみ(interpolated 形式は silent 素通り)
- Rust を触る Task のコミット前: `cargo fmt --package belt-core` / `cargo clippy --package belt-core -- -D warnings` / `cargo test -p belt-core`。YAML/Markdown のみの Task は `belt lint` + 関連 `cargo test` で検証
- CLAUDE.md は AGENTS.md への symlink。AGENTS.md 編集後は `git add AGENTS.md`
- 各 Task の終わりで cargo test green を維持する(コミット単位で red を作らない)
- diagnose/ 配下・bug-fix/pipeline.yml・handover/・resume/ は本 plan では変更しない(bug_fix_refresh.rs の EXPECTED_LEAVES は build 共有の影響でのみ改訂)
- MSRV 1.86.0 / Edition 2024。integration test は既存の `#![allow(...)]` ヘッダ様式を踏襲

## File Structure

```
新設:
  plugins/belt-agent/references/authoring-principles.md   — Sonnet プロンプト規範(全書き直しの根拠)
  plugins/belt/skills/goal/SKILL.md                       — /belt:goal(一括質問 intake)
  plugins/belt/agents/spec-reviewer.md                    — feasibility+ui-design+cross-cutting-spec 統合
  plugins/belt/agents/code-reviewer.md                    — security+cross-cutting 統合
  plugins/belt/agents/quality-reviewer.md                 — test+ai-antipattern 統合
書き直し:
  plugins/belt/skills/{design,build}/pipeline.yml         — インライン validate / evidence gate / e2e leaf
  plugins/belt/skills/{design,build,verify,spec-review,code-review,feature-dev}/SKILL.md
削除:
  plugins/belt/skills/test-scenarios/ , plugins/belt/skills/monkey-test/     (ディレクトリごと)
  plugins/belt/skills/verify/pipeline.yml , verify/{criteria,references}/
  plugins/belt/skills/design/criteria/ , design/references/{brainstorming,writing-plans}-supplement.md
  plugins/belt/skills/build/{criteria,references}/
  plugins/belt/agents/{security,test,ai-antipattern,cross-cutting,feasibility,ui-design,cross-cutting-spec}-reviewer.md
維持:
  plugins/belt/skills/design/references/path-convention.md (SSOT)
  plugins/belt-agent/references/narrative-convention.md    (diagnose が第2弾まで使用)
テスト改訂:
  crates/belt-core/tests/{feature_dev_refresh,bug_fix_refresh,review_skills_refresh}.rs
```

---

### Task 1: authoring-principles.md 新設 + AGENTS.md 参照

**Files:**
- Create: `plugins/belt-agent/references/authoring-principles.md`
- Modify: `AGENTS.md`(Plugin Authoring 関連セクション、`grep -n "fully-qualified" AGENTS.md` で 242 行目付近)

**Interfaces:**
- Produces: 後続 Task の全 SKILL.md / agents が従う規範ドキュメント。ファイルパス `plugins/belt-agent/references/authoring-principles.md` を後続タスクのレビュー基準として参照する

- [ ] **Step 1: authoring-principles.md を作成**

```markdown
# Authoring Principles (Sonnet-lean)

Prompt-layer rules for belt plugin skills and agents. Written so the
pipeline runs reliably on Sonnet-class models; stronger models simply run
faster. Introduced by docs/specs/2026-07-05-sonnet-lean-pipeline-design.md.

## 1. One phase, one file

Everything needed to execute a phase lives in the owning SKILL.md.
Done criteria live in pipeline.yml as inline `validate:` lists (3-6 items).
Do not create criteria/*.md files or references/*-supplement.md chains.
Exception: genuinely shared single-source-of-truth docs
(path-convention.md, this file).

## 2. No discretionary instructions

Never write "judge", "appropriately", "use LLM judgment", or open-ended
priority tables. Replace with explicit if-then rules that two different
models would execute identically. If a rule needs more than 3 conditions,
it is a design smell — simplify the rule.

## 3. Self-contained subagent prompts

A subagent prompt must contain: the resolved physical paths it reads and
writes, the exact output schema, and the completion condition. Subagents
never call belt-agent, never resolve URIs, and never read sibling agents'
outputs. Repeating a 3-line format across skills is cheaper than a
reference hop — prefer inlining over linking.

## 4. Batch dialogue

User questions go through AskUserQuestion in batches (up to 4 questions
per round, max 2 rounds). Never ask one question at a time across
multiple turns. Questions answerable by reading the codebase are not
asked at all.

## 5. Lines over tables

A lookup table is justified only when there are 4+ rows of homogeneous
data. For 3 or fewer conditions, write if-then bullet lines.

## Evidence entries

Every phase appends one entry to `docs/features/<topic>/evidence.md`
(created by the intake phase). Fixed 3-line format:

    ## <phase-id> — <ISO-8601 UTC>
    - Command: <command(s) actually run, or "(dialogue)" / "(authoring)">
    - Observed: <exit code / counts / PASS-FAIL summary>
    - Artifacts: <relative links to files this phase produced>

Only the orchestrator writes evidence.md — never subagents.
```

- [ ] **Step 2: AGENTS.md に参照を追記**

`AGENTS.md` の「Skill tool invoke および agent reference は常に fully-qualified」の箇条書き(242 行目付近)と同じリスト内に、次の1行を追加:

```markdown
- SKILL.md / agents / pipeline.yml の記述は `plugins/belt-agent/references/authoring-principles.md`(Sonnet-lean 規範)に従う。criteria ファイル・supplement 連鎖の新設は禁止
```

- [ ] **Step 3: 検証してコミット**

Run: `cargo test -p belt-core 2>&1 | tail -3`
Expected: 全 green(このタスクはテスト対象外ファイルのみ)

```bash
git add plugins/belt-agent/references/authoring-principles.md AGENTS.md
git commit -m "docs(belt-agent): add sonnet-lean authoring principles"
```

---

### Task 2: /belt:goal スキル新設

**Files:**
- Create: `plugins/belt/skills/goal/SKILL.md`

**Interfaces:**
- Consumes: なし(パイプライン外でも単体起動可能)
- Produces: `docs/features/<YYYY-MM-DD-topic>/goal-sheet.md`(5セクション固定)と `docs/features/<topic>/evidence.md`(intake エントリ)。design leaf(Task 5)と feature-dev SKILL(Task 7)がこの skill 名 `/belt:goal` とアーティファクト名 `goal_sheet` を参照する

- [ ] **Step 1: SKILL.md を作成**

```markdown
---
name: goal
description: >-
  Batched-question intake that turns a Linear ticket, URL, or free-text task
  into a goal sheet (goal / scope / acceptance criteria). Lightweight
  replacement for grilling/brainstorming dialogues: investigates the codebase
  first, then asks only human-decidable questions in one batch. Use standalone
  before any feature work, or as the intake phase of /belt:feature-dev.
user-invocable: true
argument-hint: "<linear-id | url | free-text>"
---

# goal

Turn raw input into a reviewed goal sheet with at most 2 question rounds.

## Step 1 — Resolve input

Apply the first matching rule to the argument text:

- Matches `^[A-Z]+-[0-9]+$` → run `linear issue view <id>` and collect
  title, description, comments, and linked URLs.
- Starts with `http` and contains `slack.com` → fetch the thread via the
  slackcli skill.
- Starts with `http` (other) → fetch the page via WebFetch.
- Anything else → treat the text itself as the task description.

If the fetched content links to further tickets/PRs, fetch at most 2 of
them (the most directly referenced). Do not crawl deeper.

## Step 2 — Investigate the codebase

Grep/Read for every identifier, module, and feature name that appears in
the resolved input. Establish: which files the change will touch, what
existing patterns apply, and which open questions the code already
answers. A question answerable here MUST NOT be asked to the user.

## Step 3 — Batched questions

Collect the remaining human-decidable points (scope boundaries, UX
choices, priority trade-offs, acceptance thresholds). Present up to 4 of
them in ONE AskUserQuestion call, each with a recommended option first.
Run a second round only if a first-round answer created a new decision
point. Never exceed 2 rounds; resolve anything left with the recommended
option and record it under Open risks.

## Step 4 — Write the goal sheet

Create `docs/features/<YYYY-MM-DD-topic>/goal-sheet.md` (naming per
`plugins/belt/skills/design/references/path-convention.md`) with exactly
these 5 sections, none empty:

    # Goal sheet: <topic>
    ## Goal            — one paragraph, the outcome in user terms
    ## In-scope        — bullet list of what will be built
    ## Out-of-scope    — bullet list of what will NOT be built
    ## Acceptance criteria — numbered; each verifiable by a command,
                             test, or observable behavior
    ## Open risks      — decisions defaulted in Step 3 + known unknowns

Then create `docs/features/<topic>/evidence.md` (if absent) and append
the intake entry per the Evidence format in
`plugins/belt-agent/references/authoring-principles.md`:

    ## intake — <ISO-8601 UTC>
    - Command: <linear/fetch commands run, or "(free-text input)">
    - Observed: <question rounds used, decisions made>
    - Artifacts: [goal-sheet.md](./goal-sheet.md)

## Red flags

- Never ask a question the codebase can answer.
- Never run more than 2 question rounds.
- Never leave a goal-sheet section empty — write "(none)" only in
  Open risks; every other section must have real content.
```

- [ ] **Step 2: 検証してコミット**

Run: `cargo test -p belt-core 2>&1 | tail -3`
Expected: 全 green

```bash
git add plugins/belt/skills/goal/SKILL.md
git commit -m "feat(belt): add /belt:goal batched-question intake skill"
```

---

### Task 3: 統合レビューア3体 + spec-review / code-review SKILL 書き直し + review_skills_refresh.rs 改訂

旧7体の削除・新3体の新設・両 SKILL の書き直し・lock test 改訂は相互依存するため1コミットで行う。

**Files:**
- Create: `plugins/belt/agents/spec-reviewer.md`
- Create: `plugins/belt/agents/code-reviewer.md`
- Create: `plugins/belt/agents/quality-reviewer.md`
- Delete: `plugins/belt/agents/{security-reviewer,test-reviewer,ai-antipattern-reviewer,cross-cutting-reviewer,feasibility-reviewer,ui-design-reviewer,cross-cutting-spec-reviewer}.md`
- Modify: `plugins/belt/skills/spec-review/SKILL.md`(全置換)
- Modify: `plugins/belt/skills/code-review/SKILL.md`(全置換)
- Modify: `crates/belt-core/tests/review_skills_refresh.rs`

**Interfaces:**
- Consumes: Task 1 の authoring-principles(evidence 形式)
- Produces: agent type 名 `belt:spec-reviewer` / `belt:code-reviewer` / `belt:quality-reviewer`。findings アーティファクト名 `findings-spec` / `findings-code` / `findings-quality` / `findings`(merged)。Task 4 の pipeline.yml がこれらの produces 名を宣言する

- [ ] **Step 1: review_skills_refresh.rs を新形状に改訂(先に red を確認)**

ファイル全体を以下で置換:

```rust
//! Integration tests locking the 2026-07-05 sonnet-lean reviewer
//! consolidation (/belt:code-review, /belt:spec-review).
//!
//! Shape contract (docs/specs/2026-07-05-sonnet-lean-pipeline-design.md):
//! - reviewer agents are consolidated 7 -> 3:
//!   code-review  -> code-reviewer + quality-reviewer
//!   spec-review  -> spec-reviewer
//! - the seven per-observation agent files are DELETED
//! - review skills still have no pipeline.yml / belt.toml
//! - parent SKILL.md still describes parallel Task dispatch with
//!   findings-*.json artifacts and passes output_path to agents

#![allow(
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

mod common;
use common::helpers::repo_root;

const REVIEW_SKILLS: &[(&str, &[&str])] = &[
    ("code-review", &["code-reviewer", "quality-reviewer"]),
    ("spec-review", &["spec-reviewer"]),
];

const CONSOLIDATED_AWAY: &[&str] = &[
    "security-reviewer",
    "test-reviewer",
    "ai-antipattern-reviewer",
    "cross-cutting-reviewer",
    "feasibility-reviewer",
    "ui-design-reviewer",
    "cross-cutting-spec-reviewer",
];

#[test]
fn review_skills_pipeline_yml_is_deleted() {
    for (skill, _agents) in REVIEW_SKILLS {
        let path = repo_root()
            .join("plugins/belt/skills")
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
            .join("plugins/belt/skills")
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
fn consolidated_reviewer_agents_exist() {
    for (_skill, agents) in REVIEW_SKILLS {
        for agent in *agents {
            let path = repo_root()
                .join("plugins/belt/agents")
                .join(format!("{agent}.md"));
            assert!(
                path.exists(),
                "consolidated agent file must exist: {}",
                path.display()
            );
        }
    }
}

#[test]
fn per_observation_agent_files_are_deleted() {
    for name in CONSOLIDATED_AWAY {
        let path = repo_root()
            .join("plugins/belt/agents")
            .join(format!("{name}.md"));
        assert!(
            !path.exists(),
            "pre-consolidation agent file must be deleted: {}",
            path.display()
        );
    }
}

#[test]
fn review_skills_parent_skill_md_references_parallel_dispatch() {
    for (skill, _agents) in REVIEW_SKILLS {
        let path = repo_root()
            .join("plugins/belt/skills")
            .join(skill)
            .join("SKILL.md");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        assert!(
            content.contains("findings-") && content.contains("Task"),
            "{skill} SKILL.md must describe Task dispatch with findings-*.json: {}",
            path.display()
        );
    }
}

#[test]
fn consolidated_agents_use_output_path_arg_pattern() {
    use std::fs;
    for name in ["spec-reviewer.md", "code-reviewer.md", "quality-reviewer.md"] {
        let path = repo_root().join("plugins/belt/agents").join(name);
        let content =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert!(
            content.contains("output_path"),
            "{name} must reference 'output_path' runtime arg"
        );
        assert!(
            !content.contains(".belt/runs/"),
            "{name} must not hardcode .belt/runs/ literals"
        );
    }
}
```

- [ ] **Step 2: テストが FAIL することを確認**

Run: `cargo test -p belt-core --test review_skills_refresh 2>&1 | tail -5`
Expected: FAIL(`consolidated_reviewer_agents_exist` — 新3体がまだ存在しない)

- [ ] **Step 3: spec-reviewer.md を作成**

観点は旧 feasibility(全5項目)+ ui-design(2項目、UI 記述がある場合のみ)+ cross-cutting-spec(3観点)を1パスに統合。severity ルールは旧3ファイルの REJECT/WARNING 基準を観点別セクションにそのまま引き継ぐ。

```markdown
---
name: spec-reviewer
description: Consolidated spec reviewer. Verifies feasibility, requirements clarity, design judgment, codebase consistency, and (when the spec has UI content) UI-pattern alignment in one pass. Writes findings-spec.json.
memory: project
---

You are a consolidated spec reviewer. In one pass over the target spec
document, produce findings for the checklists below.

## Scope

Review the spec document at the path given in your prompt. Use Grep/Read
to verify referenced APIs, libraries, models, and existing patterns in
the codebase. Read-only: never modify the spec.

## Filtering

- Report only findings you are at least 80% confident in.
- Consolidate duplicate issues into one finding. If the same issue fits
  two checklists, report it once under the checklist listed first below.

## Checklist A — Feasibility

1. Referenced APIs, libraries, and features actually exist (verify with
   Grep/Read or the library's installed version).
2. No new dependency on deprecated/EOL technology.
3. Boundary conditions considered: empty input, maximum values,
   concurrency, error cases.
4. External dependencies and version compatibility stated.

Severity: nonexistent API/library → critical. Deprecated/EOL dependency
or missing boundary-condition coverage → high. Missing version notes or
unidentified bottleneck → medium.

## Checklist B — Requirements & design judgment

1. Requirements are concrete and verifiable (numbers, conditions,
   behaviors — not "handle appropriately").
2. Implicit assumptions are stated. Grep models/tables named in the spec;
   existing validations and branches the spec ignores are findings.
3. Chosen approach has stated rationale and trade-offs.
4. Edge cases and error paths are designed, not just the happy path.

Severity: unverifiable completion conditions, 3+ unstated assumptions,
rationale-free technology choice, or happy-path-only design → high.
Vague phrasing or shallow alternatives → medium.

## Checklist C — Consistency

1. Design aligns with existing code structure, naming, and layer
   patterns (verify against the codebase, not from memory).
2. No unresolved markers: TODO, TBD, "needs confirmation", FIXME.
3. Blast radius identified: callers/dependents of modified code are
   listed in the spec.

Severity: contradicts existing structure, unresolved markers remain, or
impact gaps → high. Naming mismatch → medium.

## Checklist D — UI (conditional)

If the spec has NO UI content (no screens, components, or UI flow), skip
this checklist entirely — do not fabricate findings.

1. State transitions considered: loading, error, empty, success.
2. Design aligns with existing screens/components/style-guide patterns.

Severity: missing state transitions or contradicting the design system
→ high. Unreferenced similar screen or thin interaction detail → medium.

## Output Format

Write findings to the path provided in your prompt's `output_path` field:

    {
      "observation": "spec",
      "findings": [
        {
          "id": "<uuid>",
          "checklist": "feasibility|requirements|consistency|ui",
          "severity": "critical|high|medium|low",
          "section": "<heading path in the spec>",
          "description": "...",
          "suggestion": "...",
          "source": "agent"
        }
      ]
    }

- Emit at most 10 findings; keep the highest-severity ones and note
  truncation in a final low-severity finding.
- If no findings, write `{"observation":"spec","findings":[]}`. Always
  create the file.

## Guardrails

- Read-only. Do not invoke subagents. Do not read other findings-*.json.
```

- [ ] **Step 4: code-reviewer.md を作成**

旧 security-reviewer(11項目チェックリスト+false-positive 注意+severity 基準)と旧 cross-cutting-reviewer の Impact/Performance 観点を統合。

```markdown
---
name: code-reviewer
description: Consolidated correctness reviewer. Detects security vulnerabilities (injection, auth flaws, secret leakage, SSRF, race conditions) and impact regressions (caller integrity, shared-state consistency, contract preservation, N+1/perf hazards) in the diff scope. Writes findings-code.json.
memory: project
---

You are a consolidated correctness reviewer covering security and change
impact in one pass over the diff.

## Scope

Review ONLY the files and lines in the provided diff. You MAY read
surrounding code and grep callers to verify impact. Read-only.

## Filtering

- Report only findings you are at least 80% confident in.
- Same pattern in multiple locations → one finding with occurrence count.
- No stylistic opinions.
- Not real secrets: `.env.example` values, explicit test credentials,
  intentionally public keys, checksums using SHA256/MD5.

## Checklist A — Security

1. Injection: SQL/XSS/command/path traversal/SSRF/XXE from unvalidated
   external input.
2. Auth: missing authentication/authorization checks, privilege
   escalation, weak password hashing.
3. Secrets: hardcoded API keys, tokens, passwords.
4. Data exposure: sensitive data in logs, internals in error messages.
5. CSRF on state-changing endpoints; missing rate limiting on auth/reset
   endpoints.
6. Insecure deserialization (eval, unsafe loaders) of user input.
7. Race conditions on critical state (balance, inventory) without
   locking/transactions.

Severity: unvalidated input reaching queries/exec/paths, hardcoded
secrets, SSRF, insecure deserialization → critical. Missing auth checks,
races on critical state → high. Log exposure, error-message leaks,
missing CSRF/rate-limiting → medium.

## Checklist B — Impact

1. Caller integrity: for every changed signature, grep all callers and
   verify each handles the change (params, return type, exceptions).
2. Shared state: for every changed schema/config/cache key, verify all
   readers and writers stay consistent.
3. Contract preservation: null safety, type invariants, ordering
   guarantees, validation rules still hold.
4. Performance hazards introduced by the change: DB/API calls inside
   loops (N+1), unbounded queries without LIMIT, missing timeouts on
   external calls.

Severity: changed signature with un-updated callers → critical.
Shared-state inconsistency → high. N+1, unbounded query, missing
timeout, weakened implicit constraint → medium.

## Output Format

Write findings to the path provided in your prompt's `output_path` field:

    {
      "observation": "code",
      "findings": [
        {
          "id": "<uuid>",
          "checklist": "security|impact",
          "severity": "critical|high|medium|low",
          "file": "<path relative to repo root>",
          "line": <integer or null>,
          "description": "...",
          "suggestion": "...",
          "source": "agent"
        }
      ]
    }

- Emit at most 8 findings; keep the highest-severity ones and note
  truncation in a final low-severity finding.
- If no findings, write `{"observation":"code","findings":[]}`. Always
  create the file.

## Guardrails

- Read-only. Do not invoke subagents. Do not read other findings-*.json.
- Do not rationalize toward softer verdicts: "minor", "works", and
  "fixable later" are not grounds to downgrade severity.
```

- [ ] **Step 5: quality-reviewer.md を作成**

旧 test-reviewer(7項目)と旧 ai-antipattern-reviewer(9項目)を統合し、旧 cross-cutting の Quality/Simplification から重複しない要点(dead code, duplication, debug artifacts)を吸収。

```markdown
---
name: quality-reviewer
description: Consolidated quality reviewer. Detects test-coverage gaps, missing boundary/error-case tests, flaky risks, and AI-generated-code antipatterns (hallucinated APIs, scope creep, dead code, unnecessary compatibility shims, over-engineering) in the diff scope. Writes findings-quality.json.
memory: project
---

You are a consolidated quality reviewer covering test quality and
AI-antipatterns in one pass over the diff.

## Scope

Review ONLY the files and lines in the provided diff. If a design
document path is provided, cross-reference it for scope creep and
assumption errors. Read-only.

## Filtering

- Report only findings you are at least 80% confident in.
- Same pattern in multiple locations → one finding with occurrence count.
- No stylistic opinions.
- Review from the angle "might this code be wrong?" — when AI reviews
  AI-generated code, guard against shared bias toward "no issue".

## Checklist A — Tests

1. Changed implementation code has corresponding tests (new functions
   and branches covered).
2. Boundary values tested: 0, 1, max, empty, null.
3. Error paths and failure cases tested.
4. Flaky risk: timing, ordering, or external dependencies in tests.
5. Tests assert observable behavior, not implementation internals;
   mock-only tests that never exercise a real path are findings.

Severity: mock-only test suites, assert-free tests → high. Missing
boundary tests, flaky-risk patterns, excessive white-box coupling
→ medium.

## Checklist B — AI antipatterns

1. Hallucination: APIs, methods, options, or config keys that do not
   exist (verify with Grep/Read or installed library versions).
2. Assumption error: behavior the spec does not describe; contradicting
   the spec.
3. Scope creep: unrequested features, flags, or config options.
4. Dead code: exports with no importer, unreachable branches.
5. Copy-paste syndrome: one mistake replicated across locations.
6. Unnecessary backward compatibility: unrequested shims, `_deprecated`
   leftovers, re-exports of old names, "// removed" markers.
7. Over-engineering: single-caller helper abstractions, speculative
   generality.
8. Debug artifacts: leftover console.log/print/debugger; TODO/FIXME
   without a ticket reference.

Severity: hallucinated API (even one) → critical. Spec-contradicting
implementation, 3+ unrequested features → high. Dead code, shims,
over-engineering, debug artifacts → medium.

## Output Format

Write findings to the path provided in your prompt's `output_path` field:

    {
      "observation": "quality",
      "findings": [
        {
          "id": "<uuid>",
          "checklist": "tests|ai-antipattern",
          "severity": "critical|high|medium|low",
          "file": "<path relative to repo root>",
          "line": <integer or null>,
          "description": "...",
          "suggestion": "...",
          "source": "agent"
        }
      ]
    }

- Emit at most 8 findings; keep the highest-severity ones and note
  truncation in a final low-severity finding.
- If no findings, write `{"observation":"quality","findings":[]}`. Always
  create the file.

## Guardrails

- Read-only. Do not invoke subagents. Do not read other findings-*.json.
- Do not rationalize away missing tests because the implementation
  "looks correct".
```

- [ ] **Step 6: 旧7体を削除**

```bash
git rm plugins/belt/agents/security-reviewer.md \
       plugins/belt/agents/test-reviewer.md \
       plugins/belt/agents/ai-antipattern-reviewer.md \
       plugins/belt/agents/cross-cutting-reviewer.md \
       plugins/belt/agents/feasibility-reviewer.md \
       plugins/belt/agents/ui-design-reviewer.md \
       plugins/belt/agents/cross-cutting-spec-reviewer.md
```

- [ ] **Step 7: spec-review/SKILL.md を全置換**

grill-me 対話(one-question-at-a-time ループ)を廃止し、一括 triage に統一。

```markdown
---
name: spec-review
description: >-
  Spec review via the consolidated belt:spec-reviewer agent. Findings are
  triaged in one batched selection. --codex adds an adversarial pass via
  /codex:rescue in the same parallel batch.
argument-hint: "[--codex]"
---

# Spec Review

Runs in the main context because triage and fix apply need user dialogue
and Edit access.

## Target

The spec document: use the user-supplied path if given, otherwise the
most recently modified `*-design.md` or `goal-sheet.md` under `docs/`.

## Dispatch

1. Run `belt-agent status` and read `resolved_path` for artifacts
   `findings-spec` (and `findings-codex` when `--codex`). If no belt run
   is active (status fails), use `docs/features/<topic>/review/` as the
   output directory instead.
2. Dispatch `Task(subagent_type: belt:spec-reviewer, prompt: <spec path
   + output_path>)`. With `--codex`, invoke `/codex:rescue` in the same
   message with the spec path, the findings JSON schema from the
   spec-reviewer agent, and its own output_path.
3. Announce what was dispatched.

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

- [ ] **Step 8: code-review/SKILL.md を全置換**

```markdown
---
name: code-review
description: >-
  Two-agent parallel code review (belt:code-reviewer + belt:quality-reviewer)
  with deterministic merge. --codex adds an adversarial pass via
  /codex:rescue. Findings are triaged in one batched selection.
argument-hint: "[--codex]"
---

# Code Review

Runs in the main context because triage and fix apply need user dialogue
and Edit access.

## Scope detection

- Current branch differs from main → `git diff main...HEAD`
- Else staged changes exist → `git diff --staged`
- Else → report "no diff detected" and exit.

Pass the file list + line counts to each agent.

## Dispatch

1. Run `belt-agent status` and read `resolved_path` for artifacts
   `findings-code`, `findings-quality`, `findings` (and `findings-codex`
   when `--codex`). If no belt run is active, use
   `docs/features/<topic>/review/` as the output directory.
2. Send ONE message with parallel Task calls:
   - `Task(subagent_type: belt:code-reviewer, prompt: <diff + design doc
     path if one exists + output_path>)`
   - `Task(subagent_type: belt:quality-reviewer, prompt: <diff + design
     doc path if one exists + output_path>)`
   With `--codex`, add `/codex:rescue` to the same batch with the diff,
   the findings JSON schema, and its own output_path.
3. Announce what was dispatched.

## Merge (deterministic — no judgment)

1. Read findings-code.json and findings-quality.json.
2. Two findings are duplicates ONLY when `file` AND `line` are equal.
   Keep the higher severity; on equal severity keep the
   findings-code.json one. Codex findings are never deduplicated.
3. Sort by severity (critical > high > medium > low), cap at 20 (note
   truncation as a final low finding), write to the `findings` artifact
   path as `{"findings":[...]}`.

## Triage (batched)

Present ALL merged findings as one numbered list (severity order, one
line + suggestion each). Ask once which numbers to fix. No per-finding
dialogue across turns.

## Fix apply + verify

1. Apply selected suggestions serially with Edit.
2. Run the project linter and test suite (Cargo.toml → `cargo clippy --
   -D warnings` + `cargo test`; package.json → `npm run lint` + `npm
   test`; pyproject.toml → `ruff check .` + `pytest`; go.mod → `go vet
   ./...` + `go test ./...`; Makefile → `make lint` + `make test`).
3. Report failures honestly — never suppress.

## Red flags

- Never modify code before user selection.
- Never filter findings before presenting them.
- Never let an agent read another agent's findings-*.json.
```

- [ ] **Step 9: テスト green を確認してコミット**

Run: `cargo test -p belt-core --test review_skills_refresh 2>&1 | tail -3`
Expected: PASS(全テスト)

Run: `cargo test -p belt-core 2>&1 | tail -3`
Expected: 全 green

```bash
git add -A plugins/belt/agents plugins/belt/skills/spec-review plugins/belt/skills/code-review crates/belt-core/tests/review_skills_refresh.rs
git commit -m "feat(belt): consolidate reviewers 7->3 and batch review triage"
```

---

### Task 4: pipeline.yml 書き直し(design / build)+ verify/pipeline.yml 削除 + lock tests 改訂

pipeline 構造と lock tests は同一コミットでないと red になるため1タスクで行う。`feature-dev/pipeline.yml` と `bug-fix/pipeline.yml` は**無変更**(合成参照のみで stage 中身が変わる)。

**Files:**
- Modify: `plugins/belt/skills/design/pipeline.yml`(全置換)
- Modify: `plugins/belt/skills/build/pipeline.yml`(全置換)
- Delete: `plugins/belt/skills/verify/pipeline.yml`
- Modify: `crates/belt-core/tests/feature_dev_refresh.rs`
- Modify: `crates/belt-core/tests/bug_fix_refresh.rs`

**Interfaces:**
- Consumes: Task 2 のスキル名 `/belt:goal`、Task 3 のアーティファクト名 `findings-code` / `findings-quality` / `findings`
- Produces: 展開 leaf 列 `design/intake, design/design, pre-execute-handover/checkpoint, build/execute, build/code-review, build/e2e, build/integrate`。アーティファクト名 `goal_sheet` / `evidence` / `design_doc` / `scenarios` / `e2e_report`。Task 5-6 の SKILL.md がこの leaf 名とフェーズ実行手順を対応づける

- [ ] **Step 1: design/pipeline.yml を全置換**

```yaml
name: design
version: 1
description: "Feature design stage (intake -> design)"

args:
  e2e:
    type: bool
    default: false
    description: "Author agent-browser scenarios (scenarios.yml) alongside the design"
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
    confirm: true
    max_retries: 3

  - id: design
    description: "Write design.md (architecture + test strategy + implementation tasks), then spec-review it (see design SKILL.md, Phase: design)"
    consumes:
      - goal_sheet
    produces:
      - name: design_doc
        path: "docs/features/*/design.md"
        description: "Architecture, test strategy, and checkbox implementation tasks"
      - name: scenarios
        path: "docs/features/*/scenarios.yml"
        description: "Agent-browser replay scenarios (Given/When/Then YAML)"
        when: "args.e2e"
      - name: findings-spec
        path: "belt://current/review/findings-spec.json"
        description: "Consolidated spec review findings"
    gate:
      - file_exists: "docs/features/*/design.md"
      - file_exists: "belt://current/review/findings-spec.json"
    validate:
      - "design.md contains Architecture, Test Strategy, and Implementation Tasks sections"
      - "Implementation Tasks is a checkbox list where every task names its target files"
      - "Every acceptance criterion in goal-sheet.md maps to at least one item in Test Strategy"
      - "If the e2e arg is true: scenarios.yml exists and covers every acceptance criterion"
      - "Every critical/high finding in findings-spec.json is fixed or explicitly rejected by the user"
      - "evidence.md has a design entry"
    confirm: true
    max_retries: 3
```

- [ ] **Step 2: build/pipeline.yml を全置換**

```yaml
name: build
version: 1
description: "Shared build stage (execute -> code-review -> e2e -> integrate)"

args:
  e2e:
    type: bool
    default: false
    description: "Run the browser-based e2e phase via /belt:verify"
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
      - "Every checkbox in the plan document's task list (design.md Implementation Tasks, or fix-plan.md for bug runs) is checked"
      - "Project test suite passes; command and result recorded in evidence.md's execute entry"
      - "Project linter passes; command and result recorded in evidence.md's execute entry"
    confirm: true
    max_retries: 3

  - id: code-review
    description: "Two-agent parallel code review with batched triage"
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
      - "Every critical/high finding in findings.json is fixed or explicitly rejected by the user"
      - "Linter and test suite pass after fixes; recorded in evidence.md's code-review entry"
    confirm: true
    max_retries: 3

  - id: e2e
    when: "args.e2e"
    description: "Browser-based verification: scenario replay + exploratory pass via /belt:verify"
    invoke:
      skill: /belt:verify
    produces:
      - name: e2e_report
        path: "docs/features/*/e2e-report.md"
        description: "Scenario results, exploratory notes, verdict"
    gate:
      - file_exists: "docs/features/*/e2e-report.md"
    validate:
      - "Every scenario in scenarios.yml was replayed with PASS/FAIL recorded in e2e-report.md"
      - "Every FAIL was fixed and re-run, or explicitly accepted by the user"
      - "evidence.md has an e2e entry"
    confirm: true
    max_retries: 3

  - id: integrate
    description: "Integrate changes (user chooses: wt merge or gh pr create)"
    invoke:
      skill: /worktrunk
    validate:
      - "User explicitly chose the integration mode (wt merge or gh pr create)"
      - "evidence.md has one entry per completed phase, ending with integrate"
    confirm: true
    max_retries: 3
```

- [ ] **Step 3: verify/pipeline.yml を削除**

```bash
git rm plugins/belt/skills/verify/pipeline.yml
```

- [ ] **Step 4: feature_dev_refresh.rs を新形状に改訂**

doc コメントの Shape contract を 2026-07-05 spec 参照に書き換え、以下を変更(それ以外のテスト関数は無変更で green のはず):

```rust
const EXPECTED_LEAVES: &[&str] = &[
    "design/intake",
    "design/design",
    "pre-execute-handover/checkpoint",
    "build/execute",
    "build/code-review",
    "build/e2e",
    "build/integrate",
];
```

`feature_dev_expands_to_ten_namespaced_leaves` → 関数名を `feature_dev_expands_to_seven_namespaced_leaves` に変更。

`expanded_regate_targets_are_namespaced` を全置換(regate は全廃):

```rust
#[test]
fn no_leaf_declares_regate() {
    let expanded =
        expand_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must expand");
    for leaf in &expanded {
        assert!(
            leaf.regate.is_empty(),
            "leaf '{}' must have empty regate (sonnet-lean removed regate), got {:?}",
            leaf.id,
            leaf.regate
        );
    }
}
```

`expanded_verify_leaves_inherit_e2e_when` を全置換:

```rust
#[test]
fn e2e_leaf_carries_when_and_others_do_not() {
    let expanded =
        expand_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must expand");
    let e2e = expanded
        .iter()
        .find(|p| p.id == "build/e2e")
        .expect("build/e2e leaf must exist");
    assert_eq!(
        e2e.when.as_deref(),
        Some("args.e2e"),
        "build/e2e must carry when: args.e2e"
    );
    for id in ["build/execute", "build/code-review", "build/integrate"] {
        let leaf = expanded
            .iter()
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("leaf '{id}' must exist"));
        assert_eq!(leaf.when, None, "leaf '{id}' must not carry a when");
    }
}
```

- [ ] **Step 5: bug_fix_refresh.rs を新形状に改訂**

doc コメント更新+以下のみ変更:

```rust
const EXPECTED_LEAVES: &[&str] = &[
    "diagnose/rca",
    "diagnose/fix-plan",
    "diagnose/fix-plan-review",
    "pre-execute-handover/checkpoint",
    "build/execute",
    "build/code-review",
    "build/e2e",
    "build/integrate",
];
```

`bug_fix_expands_to_nine_namespaced_leaves` → `bug_fix_expands_to_eight_namespaced_leaves` に改名。

`expanded_regate_targets_are_namespaced` を全置換(feature 側と同じ `no_leaf_declares_regate` パターン、`bug_fix_pipeline_path()` を使用)。

`expanded_verify_leaves_inherit_e2e_when` を全置換(feature 側 Step 4 と同じ `e2e_leaf_carries_when_and_others_do_not` パターン、`bug_fix_pipeline_path()` を使用)。

- [ ] **Step 6: lint + テスト green を確認してコミット**

Run: `cargo run --bin belt -- lint plugins/belt/skills/design/pipeline.yml && cargo run --bin belt -- lint plugins/belt/skills/build/pipeline.yml && cargo run --bin belt -- lint plugins/belt/skills/feature-dev/pipeline.yml && cargo run --bin belt -- lint plugins/belt/skills/bug-fix/pipeline.yml`
(belt バイナリが PATH にあれば `belt lint <path>` で可)
Expected: 4本とも lint PASS(インライン validate は既存機能)

Run: `cargo fmt --package belt-core && cargo clippy --package belt-core -- -D warnings && cargo test -p belt-core 2>&1 | tail -3`
Expected: 全 green

```bash
git add -A plugins/belt/skills/design/pipeline.yml plugins/belt/skills/build/pipeline.yml plugins/belt/skills/verify crates/belt-core/tests/feature_dev_refresh.rs crates/belt-core/tests/bug_fix_refresh.rs
git commit -m "feat(belt): compress design/build stages to 7 leaves with inline validate"
```

---

### Task 5: design SKILL.md 書き直し + design 配下整理 + test-scenarios スキル削除

**Files:**
- Modify: `plugins/belt/skills/design/SKILL.md`(全置換)
- Delete: `plugins/belt/skills/design/criteria/`(ディレクトリごと)
- Delete: `plugins/belt/skills/design/references/brainstorming-supplement.md`
- Delete: `plugins/belt/skills/design/references/writing-plans-supplement.md`
- Delete: `plugins/belt/skills/test-scenarios/`(ディレクトリごと)
- Keep: `plugins/belt/skills/design/references/path-convention.md`

**Interfaces:**
- Consumes: Task 2 の `/belt:goal`、Task 3 の `/belt:spec-review`、Task 4 の leaf 名 `intake` / `design`
- Produces: design leaf の実行手順(invoke なし leaf は SKILL.md の Phase セクションが作業を定義するという規約)。Task 7 の feature-dev SKILL がこの SKILL.md を stage map から参照する

- [ ] **Step 1: design/SKILL.md を全置換**

```markdown
---
name: design
description: >-
  Runs the feature design stage: goal-sheet intake (/belt:goal) followed by
  a single design document (architecture + test strategy + implementation
  tasks) reviewed by belt:spec-reviewer. Use standalone for design-only
  work, or composed as the upstream stage of /belt:feature-dev. --e2e also
  authors agent-browser scenarios; --codex enables adversarial spec review.
user-invocable: true
argument-hint: "<linear-id | url | free-text> [--e2e] [--codex]"
---

# design

Belt pipeline for the design stage. Structure, gates, and done criteria
live in `pipeline.yml`; this file defines how to execute each phase.

## Phase: intake

Invoke `/belt:goal`, passing the user's original input (ticket id, URL,
or free text) verbatim. The skill writes goal-sheet.md and evidence.md.

## Phase: design

This phase has no `invoke` — execute these steps directly:

1. Read goal-sheet.md (resolve the path via `belt-agent status`, artifact
   `goal_sheet`).
2. Explore the code the change touches with Grep/Read. Dispatch explorer
   subagents only if the area is unfamiliar AND spans 10+ files.
3. Write `docs/features/<topic>/design.md` with exactly these sections:
   - `## Architecture` — approach, components, data flow; rejected
     alternatives in one line each
   - `## Test Strategy` — for each acceptance criterion in goal-sheet.md,
     the test(s) that verify it (level: unit/integration/e2e + test name)
   - `## Implementation Tasks` — checkbox list; every task names its
     target files and its test
   If the e2e arg is true, also write `scenarios.yml` (Given/When/Then,
   one scenario per acceptance criterion).
4. Invoke `/belt:spec-review` (pass `--codex` if the codex arg is true)
   and complete its batched triage.
5. Append the design entry to evidence.md (format:
   `plugins/belt-agent/references/authoring-principles.md`).

## Red flags

- Never ask the user design questions one at a time — batch remaining
  open points in one AskUserQuestion call.
- Never write an Implementation Task without file paths — execute
  dispatches subagents from this list alone.
- Never hand-edit files under `docs/features/<topic>/` after a phase
  completes (breaks the phase-start mtime filter).

## References

- `./references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `plugins/belt-agent/references/authoring-principles.md` — evidence entry format
```

- [ ] **Step 2: 不要ファイルを削除**

```bash
git rm -r plugins/belt/skills/design/criteria \
          plugins/belt/skills/design/references/brainstorming-supplement.md \
          plugins/belt/skills/design/references/writing-plans-supplement.md \
          plugins/belt/skills/test-scenarios
```

- [ ] **Step 3: 検証してコミット**

Run: `cargo test -p belt-core 2>&1 | tail -3` および `belt lint plugins/belt/skills/design/pipeline.yml`
Expected: 全 green(Task 4 で validate は既にインライン化済みのため criteria 削除で lint は壊れない)

Run: `grep -rn "test-scenarios\|brainstorming-supplement\|writing-plans-supplement" plugins/belt/skills/design/ plugins/belt/skills/feature-dev/ ; echo "exit: $?"`
Expected: feature-dev/SKILL.md 内の残参照のみ(Task 7 で解消)。design/ 配下はヒットなし

```bash
git add -A plugins/belt/skills/design plugins/belt/skills/test-scenarios
git commit -m "feat(belt): rewrite design stage skill; drop test-scenarios and supplements"
```

---

### Task 6: build / verify SKILL.md 書き直し + build 配下整理 + monkey-test スキル削除

**Files:**
- Modify: `plugins/belt/skills/build/SKILL.md`(全置換)
- Modify: `plugins/belt/skills/verify/SKILL.md`(全置換)
- Delete: `plugins/belt/skills/build/criteria/`, `plugins/belt/skills/build/references/`(ディレクトリごと)
- Delete: `plugins/belt/skills/verify/criteria/`, `plugins/belt/skills/verify/references/`(ディレクトリごと)
- Delete: `plugins/belt/skills/monkey-test/`(ディレクトリごと)

**Interfaces:**
- Consumes: Task 3 の `/belt:code-review`、Task 4 の leaf 名 `execute` / `code-review` / `e2e` / `integrate`、agent type `belt-agent:feature-implementer`(既存・無変更)
- Produces: execute leaf の直接ディスパッチ手順、`/belt:verify` の単体実行契約(e2e-report.md)。bug-fix パイプラインも本 SKILL 経由で build stage を実行する

- [ ] **Step 1: build/SKILL.md を全置換**

```markdown
---
name: build
description: >-
  Runs the shared build stage: TDD implementation from the plan document's
  task list, two-agent code review, optional browser verification (--e2e),
  and integration. Use standalone with an existing design.md/fix-plan.md,
  or composed as the downstream stage of /belt:feature-dev and
  /belt:bug-fix.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# build

Belt pipeline for the build stage. Structure, gates, and done criteria
live in `pipeline.yml`; this file defines how to execute each phase.

## Entry check

A plan document must exist: `docs/features/<topic>/design.md` with an
Implementation Tasks section (feature runs) or
`docs/features/<topic>/fix-plan.md` (bug runs). If neither exists, stop
and ask the user. If `docs/features/<topic>/evidence.md` does not exist
(bug runs), create it now with the header `# Evidence: <topic>`.

## Phase: execute

This phase has no `invoke` — execute these steps directly:

1. Read the plan document's task list (Implementation Tasks in
   design.md, or the task list in fix-plan.md).
2. For each unchecked task, dispatch ONE `belt-agent:feature-implementer`
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

Invoke `/belt:code-review` (pass `--codex` if the codex arg is true) and
complete its batched triage. Append the code-review entry to
evidence.md.

## Phase: e2e (when the e2e arg is true)

Invoke `/belt:verify`. Append the e2e entry to evidence.md.

## Phase: integrate

Ask the user once: A) `wt merge` or B) `gh pr create`. Invoke
`/worktrunk` with the chosen mode. Confirm evidence.md has one entry per
completed phase, then append the integrate entry.

## Red flags

- Never start execute without the Entry check.
- Never forward the whole design doc to implementer subagents — copy the
  relevant constraints into each prompt.
- Never skip the per-task test run after a subagent returns.
- Never decide merge-vs-PR yourself — always the user's choice.
- Never let subagents write evidence.md — orchestrator only.

## References

- `plugins/belt/skills/design/references/path-convention.md` — naming rules (SSOT)
- `plugins/belt-agent/references/authoring-principles.md` — evidence entry format
```

- [ ] **Step 2: verify/SKILL.md を全置換**

```markdown
---
name: verify
description: >-
  Browser-based verification in one pass: replays
  docs/features/<topic>/scenarios.yml via agent-browser, then runs a short
  exploratory pass around the change scope. Writes e2e-report.md. Use
  standalone, or invoked by the e2e phase of /belt:build. Requires
  agent-browser.
user-invocable: true
---

# verify

Single-pass browser verification. No pipeline.yml — this skill runs its
three steps directly.

## Entry check

Locate the scenario file: `docs/features/<topic>/scenarios.yml` (feature
runs) or `docs/features/<topic>/rca-scenarios.yml` (bug runs). On glob
collision take the most recently modified. If none exists, stop and ask
the user — there is nothing to replay.

## Step 1 — Scenario replay

Load the agent-browser skill. For each scenario (Given/When/Then), drive
the browser through the steps and record PASS or FAIL with the observed
behavior. Never silently retry a FAIL — record it first; re-run only
after a fix.

## Step 2 — Exploratory pass

Around the changed screens/flows (file list from
`git diff main...HEAD`), probe beyond the scripted paths: invalid input,
empty states, rapid repeat actions, back/reload navigation. Cap the
exploration at 15 minutes. Record anything anomalous.

## Step 3 — Report

Write `docs/features/<topic>/e2e-report.md`:

    # E2E report: <topic>
    ## Scenario results   — table: scenario id | PASS/FAIL | note
    ## Exploratory notes  — bullet list of probes and observations
    ## Verdict            — PASS (all green) / FAIL (list) / SKIPPED

If no browser-reachable UI exists (CLI/backend-only repo), write the
report with Verdict: SKIPPED and the reason — never fabricate browser
runs.

## Red flags

- Never mark a scenario PASS without driving it in the browser.
- Never write files outside `docs/features/<topic>/`.
```

- [ ] **Step 3: 不要ファイルを削除**

```bash
git rm -r plugins/belt/skills/build/criteria \
          plugins/belt/skills/build/references \
          plugins/belt/skills/verify/criteria \
          plugins/belt/skills/verify/references \
          plugins/belt/skills/monkey-test
```

- [ ] **Step 4: 検証してコミット**

Run: `cargo test -p belt-core 2>&1 | tail -3`
Expected: 全 green

Run: `grep -rn "monkey-test\|dogfood\|supplement\|evidence-catalog\|worktrunk-supplement" plugins/belt/skills/build/ plugins/belt/skills/verify/ ; echo "exit: $?"`
Expected: ヒットなし(exit: 1)

```bash
git add -A plugins/belt/skills/build plugins/belt/skills/verify plugins/belt/skills/monkey-test
git commit -m "feat(belt): rewrite build/verify stage skills; drop monkey-test and supplements"
```

---

### Task 7: feature-dev SKILL.md 書き直し + ドキュメント/バージョン更新 + spec 1行修正

**Files:**
- Modify: `plugins/belt/skills/feature-dev/SKILL.md`(全置換)
- Modify: `README.md`(271行: belt プラグイン説明 / 275行・285行: monkey-test 依存表 / 319-320行: skill 例)
- Modify: `plugins/belt/.claude-plugin/plugin.json`(description + version 0.3.0)
- Modify: `plugins/belt-agent/.claude-plugin/plugin.json`(version 0.3.0)
- Modify: `docs/specs/2026-07-05-sonnet-lean-pipeline-design.md`(execute 行)

**Interfaces:**
- Consumes: Task 4 の leaf 名、Task 5-6 の stage SKILL.md
- Produces: 完成した `/belt:feature-dev` エントリポイント

- [ ] **Step 1: feature-dev/SKILL.md を全置換**

```markdown
---
name: feature-dev
description: >-
  Quality-gated feature pipeline from ticket to integration: goal-sheet
  intake, single design document with spec review, context-reset
  checkpoint, TDD implementation, two-agent code review, optional browser
  verification, and integration — with an evidence.md trail. Accepts a
  Linear id, URL, or free-text task. --e2e enables browser verification;
  --codex enables adversarial review.
user-invocable: true
argument-hint: "<linear-id | url | free-text> [--e2e] [--codex]"
---

# feature-dev

Composed pipeline: design stage → checkpoint → build stage. `belt-agent
init` expands the three `invoke.pipeline` references into namespaced
leaves (`design/intake` ... `build/integrate`) in a single run.

Keep the user's original task input (ticket id, URL, or free text): the
`design/intake` phase passes it verbatim to `/belt:goal`.

## Stage skills

When `next` returns a phase, read the owning stage's SKILL.md before
executing it:

- `design/*` → `plugins/belt/skills/design/SKILL.md`
- `pre-execute-handover/*` → run `/belt:handover`, then `/clear`, then
  `/belt:resume` in the new session
- `build/*` → `plugins/belt/skills/build/SKILL.md`

Smaller runs: `/belt:design` (design only), `/belt:build` (plan already
exists), `/belt:goal` (intake only), `/belt:verify` (browser check only).

## Red flags

- Never execute a stage phase without its stage SKILL.md loaded.
- Never bypass the pre-execute-handover checkpoint — the context reset
  before execute is the pipeline's core ergonomics.
```

- [ ] **Step 2: README.md を更新**

271行(belt プラグイン行)を以下で置換:

```markdown
| `belt` | User-invocable pipelines and reviewer agents: `/belt:feature-dev`, `/belt:bug-fix`, `/belt:goal`, `/belt:design`, `/belt:build`, `/belt:verify`, `/belt:code-review` (2 reviewers), `/belt:spec-review` (1 reviewer), `/belt:handover`, `/belt:resume`. Requires `belt-agent` |
```

275行付近の「`/belt:monkey-test` requires ...」の文と 285行の依存表の行を、`/belt:verify` を主語にした同内容に書き換える(agent-browser 依存自体は変わらない):

```markdown
| `agent-browser` CLI + `/agent-browser` skill | [vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser) | `/belt:verify` always, `/belt:feature-dev` / `/belt:bug-fix` `e2e` phase (when `--e2e`) |
```

- [ ] **Step 3: plugin.json ×2 を更新**

`plugins/belt/.claude-plugin/plugin.json`:

```json
{
  "name": "belt",
  "description": "User-invocable skills and their reviewer agents: /belt:feature-dev, /belt:bug-fix, /belt:goal, /belt:design, /belt:build, /belt:verify, /belt:code-review (2 reviewers), /belt:spec-review (1 reviewer), /belt:handover, /belt:resume. Requires belt-agent plugin",
  "version": "0.3.0",
  "author": { "name": "neko-neko" }
}
```

`plugins/belt-agent/.claude-plugin/plugin.json` は `"version": "0.3.0"` のみ変更。

- [ ] **Step 4: spec の execute 記述を実装に合わせて修正**

`docs/specs/2026-07-05-sonnet-lean-pipeline-design.md` の
`[4] execute      — TDD 実装 (/subagent-driven-development、無変更)` を
`[4] execute      — TDD 実装 (belt-agent:feature-implementer を build SKILL.md から直接ディスパッチ)` に置換し、「触らないもの」等に矛盾が残らないことを確認。

- [ ] **Step 5: 検証してコミット**

Run: `cargo test -p belt-core 2>&1 | tail -3`
Expected: 全 green

Run: `grep -rn "test-scenarios\|monkey-test" plugins/belt/skills/feature-dev/ README.md ; echo "exit: $?"`
Expected: ヒットなし(exit: 1)

```bash
git add plugins/belt/skills/feature-dev README.md plugins/belt/.claude-plugin/plugin.json plugins/belt-agent/.claude-plugin/plugin.json docs/specs/2026-07-05-sonnet-lean-pipeline-design.md
git commit -m "feat(belt): rewrite feature-dev entry; bump plugins to 0.3.0"
```

---

### Task 8: 最終検証(grep sweep + lint + full test)

**Files:**
- なし(検証のみ。問題発見時は該当 Task の方式で修正)

- [ ] **Step 1: 削除ファイルへの残参照を sweep**

Run: `grep -rn "criteria/\|supplement\|monkey-test\|test-scenarios\|narrative note\|phase-design.md\|phase-execute.md" plugins/belt/skills/feature-dev plugins/belt/skills/design plugins/belt/skills/build plugins/belt/skills/verify plugins/belt/skills/goal plugins/belt/skills/spec-review plugins/belt/skills/code-review plugins/belt/agents ; echo "exit: $?"`
Expected: ヒットなし(exit: 1)。diagnose/bug-fix/handover/resume 配下は対象外(第2弾)

- [ ] **Step 2: 全 pipeline lint**

Run: `for p in feature-dev bug-fix design build diagnose; do belt lint plugins/belt/skills/$p/pipeline.yml || echo "FAIL: $p"; done && belt lint plugins/belt/skills/handover/checkpoint.yml`
Expected: 全て PASS、"FAIL:" 出力なし

- [ ] **Step 3: full test + fmt + clippy**

Run: `cargo fmt --package belt-core --check && cargo clippy --package belt-core -- -D warnings && cargo test -p belt-core 2>&1 | tail -5`
Expected: 全 green

- [ ] **Step 4: 展開スモークテスト**

Run: `belt-agent init plugins/belt/skills/feature-dev/pipeline.yml --arg e2e=true --arg codex=false && belt-agent next && belt-agent status | head -40`
Expected: init 成功、`next` が `design/intake`(invoke: `/belt:goal`)を返す。確認後 `.belt/runs/` の当該 run は破棄してよい

- [ ] **Step 5: コミット(修正が発生した場合のみ)**

```bash
git add -A && git commit -m "chore(belt): post-rewrite sweep fixes"
```

---

## Self-Review 結果

- **Spec coverage**: D1〜D8 全決定に対応タスクあり(D1: 全タスクで Rust src 無変更 / D2: Task 2 / D3: Task 1+3 / D4: Task 2+7 / D5: Task 4 / D6: Task 1+4〜6 / D7: Task 3 / D8: Task 7)。spec の「レビューア統合表」「evidence 形式」「/belt:goal 仕様」「スキル増減表」全て反映
- **逸脱1件(明示)**: execute の SDD 依存廃止 → Task 7 Step 4 で spec 側を修正
- **Type consistency**: leaf 名(design/intake, build/e2e 等)・アーティファクト名(goal_sheet, findings-code 等)・agent 名(belt:spec-reviewer 等)は Task 2〜7 で同一文字列を使用
- **既知の残課題(第2弾)**: diagnose 圧縮、bug-fix の goal 統合、narrative-convention.md の退役

