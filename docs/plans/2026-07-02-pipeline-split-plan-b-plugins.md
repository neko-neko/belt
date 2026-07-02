# Pipeline Split Plan B: Plugin Layer Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** plugins/belt を 4 stage pipeline (design / diagnose / build / verify) に分割し、feature-dev / bug-fix を `invoke.pipeline` 3-phase 合成に置き換える。criteria/references を担当 stage へ再配置 (structured 統一 + docs/features path 統一 + 共有 1 本化)、lock tests 改訂、protocol SKILL.md の drift 修正、codex timeout 追記、AGENTS.md 更新、plugin version 0.3.0。

**Architecture:** Plan A で実装済みの再帰 expander (`{a}/{b}/{c}` namespace、深さ上限 4、sibling regate リネーム) を前提に、単体 init 可能な Pipeline 形式 YAML 4 本を新設し (dual-format PASS 済みのため `args:` 付きのまま sub 参照可)、feature-dev / bug-fix は `with: { e2e: "args.e2e", codex: "args.codex" }` の identity 伝播で合成する。追加 (Tasks 1–4) → 合成 cutover (Tasks 5–6) → lock 統合 (Task 7) → docs/version (Tasks 8–11) → 最終検証 (Task 12) の順で、各 commit を green に保つ。

**Tech Stack:** YAML (belt pipeline) / Markdown (SKILL.md, criteria, references) / Rust (belt-core integration tests のみ、production code 変更なし)。

**Spec:** `docs/specs/2026-07-02-pipeline-split-design.md` (approved)。Plan A (`docs/plans/2026-07-02-pipeline-split-plan-a-expander.md`) 完了済み (spike: dual-format PASS)。

## Spec Gap Notes (writing-plans 段階で確定した補足決定)

1. **README.md の最小補正を Task 11 に同梱**: spec は AGENTS.md のみ挙げるが、README.md 18 行目の `uses:` 言及と 271 行目の skill 一覧は同種の drift のため 2 行だけ直す (全面 refresh は follow-up)。
2. **bug-fix 由来ファイルは copy + 後続削除 (git mv でない)**: `bug_fix_refresh.rs` が bug-fix 配下の criteria/references の物理存在を lock しており、cutover (Task 6) まで原本を消せない。paths/audit 編集も入るため rename 検出は元々効かない。feature-dev 由来ファイル (存在 lock なし) は spec どおり git mv する。
3. **criteria structured 統一で `# Phase N (id) Done Criteria` H1 は削除**: spec は `# id Done Criteria` への置換を指示するが、structured template (criteria-template.md) には H1 が存在しない。conversion により見出しごと消えるのが正しい形 (序数除去の意図は満たす)。
4. **criteria ID は `{PHASE-ID}-NN` に統一**: 旧 `SREV-`/`TEST-`/`INT-` 略記は phase id 大文字化 (`SPEC-REVIEW-` / `TEST-SCENARIOS-` / `INTEGRATE-`) に改める。criteria-template.md の ID Convention も同時更新 (Task 8)。
5. **audit 厳格度**: execute / code-review = `required`、その他全て = `lite` (rca / fix-plan / fix-plan-review は required → lite に変更)。
6. **verify pipeline は args を宣言しない**: spec は「各新 pipeline は args: e2e / codex を自前宣言」と読めるが、verify の 2 phase はどちらの flag も参照しない (e2e ゲートは composing 側 `build.verify` の `when:` が担い、単体 verify はユーザーが意図的に起動する場面のみ)。不使用 arg の宣言は lint 的にもノイズのため宣言しない。

## Global Constraints

- **`with:` の値は bare `args.<name>` full-string form のみ置換される** (interpolated 形式は silent 素通り)。合成 YAML の with は必ず `e2e: "args.e2e"` / `codex: "args.codex"` の形で書く
- **cross-stage regate は解決されない** (rename_sibling_target は同一ファイル内 sibling のみ)。regate は各 pipeline ファイル内で閉じる: design 内 `spec-review → [test-scenarios]`、build 内 `code-review → [execute]`。合成側 (feature-dev / bug-fix) に regate を書かない
- **3 段合成 (feature-dev → build → verify) は depth 4 以内** (visited.len()==3 で上限内)。外側 gate は最内 last leaf に着地、`when:` は全レベル貫通 (自前 when を持たない leaf のみ)
- **dual-format PASS 済み**: `args:` 付き Pipeline 形式 YAML はそのまま `invoke.pipeline` で参照できる (serde-saphyr が unknown field を無視、model.rs 無変更)
- **全 leaf phase は `description:` 必須** (Plan A の挙動変更。sub 参照 phase 自体には不要)
- `belt lint` は `validate:` file 参照のディスク存在を検査する → pipeline.yml と criteria は同一 commit で整合させる
- `scenarios_contract.rs` が `docs/testing/lock-ledger.md` の `locks-file:` と実ファイル存在を機械照合する → lock test の追加/削除と ledger 更新は同一 commit
- リポジトリコンテンツは英語 (docs/plans, docs/specs のみ日本語可)。plugins/ 配下は全て英語
- Rust を触る Task のコミット前: `cargo fmt --package belt-core` / `cargo clippy --package belt-core -- -D warnings` / `cargo test -p belt-core`。YAML/Markdown のみの Task は `belt lint` + 関連 `cargo test` で検証
- CLAUDE.md は AGENTS.md への symlink。AGENTS.md 編集後は `git add AGENTS.md` (CLAUDE.md を add しても symlink しか stage されない)
- MSRV 1.86.0 / Edition 2024 / toolchain 1.94.1。integration test は既存の `#![allow(...)]` ヘッダ様式を踏襲

### 既知の transient 状態 (Tasks 2–4 の間のみ)

Tasks 2–3 で feature-dev 配下の criteria/references を git mv した後、Task 5 の cutover まで旧 `feature-dev/pipeline.yml` の `validate:` / SKILL.md 参照が dangling になる。**cargo test は green を維持する** (実 pipeline を belt lint する自動テストは存在しない)。この間 `belt lint plugins/belt/skills/feature-dev/pipeline.yml` は FAIL するが想定内。bug-fix 側は Task 6 まで一切無傷 (lock が存在検査するため copy 方式)。

## File Structure

| Path | 責務 |
|---|---|
| `plugins/belt/skills/verify/{pipeline.yml,belt.toml,SKILL.md}` | 新設: monkey-test → dogfood (e2e 専用 stage、args なし) |
| `plugins/belt/skills/verify/criteria/{monkey-test,dogfood}.md` | fd(freeform)+bf(structured) 両版を structured に統合 (新規著作) |
| `plugins/belt/skills/verify/references/{monkey-test,dogfood}-supplement.md` | 両版 supplement を統合 (新規著作) |
| `plugins/belt/skills/build/{pipeline.yml,belt.toml,SKILL.md}` | 新設: execute → code-review → verify(sub) → integrate (共有 stage) |
| `plugins/belt/skills/build/criteria/{execute,code-review}.md` | feature-dev から git mv + docs/features path 補正 |
| `plugins/belt/skills/build/criteria/integrate.md` | fd+bf 両版を structured に統合 (新規著作) |
| `plugins/belt/skills/build/references/{worktrunk-supplement,evidence-catalog}.md` | feature-dev から git mv + bf 差分 fold-in / 中立化 |
| `plugins/belt/skills/design/{pipeline.yml,belt.toml,SKILL.md}` | 新設: design → test-scenarios → spec-review → plan |
| `plugins/belt/skills/design/criteria/{design,test-scenarios,spec-review,plan}.md` | feature-dev から git mv + structured 変換 |
| `plugins/belt/skills/design/references/{brainstorming,writing-plans}-supplement.md` | feature-dev から git mv + 序数除去 |
| `plugins/belt/skills/design/references/path-convention.md` | feature-dev から git mv + 序数除去 + bug-fix 節追加 (唯一の SSOT) |
| `plugins/belt/skills/diagnose/{pipeline.yml,belt.toml,SKILL.md}` | 新設: rca → fix-plan → fix-plan-review |
| `plugins/belt/skills/diagnose/criteria/{rca,fix-plan,fix-plan-review}.md` | bug-fix から copy + docs/features path + audit lite 化 |
| `plugins/belt/skills/diagnose/references/{rca,fix-plan}-supplement.md` | bug-fix から copy + docs/features path |
| `plugins/belt/skills/feature-dev/{pipeline.yml,SKILL.md}` | 3-phase 合成に書き換え。criteria/ references/ は空になり削除 |
| `plugins/belt/skills/bug-fix/{pipeline.yml,SKILL.md}` | 3-phase 合成に書き換え。criteria/ references/ は全削除 |
| `crates/belt-core/tests/feature_dev_refresh.rs` | 合成 shape lock に全面改訂 |
| `crates/belt-core/tests/bug_fix_refresh.rs` | 合成 shape lock に全面改訂 |
| `crates/belt-core/tests/pipeline_split_refresh.rs` | 新設: 4 stage pipeline の shape lock 統合 |
| `crates/belt-core/tests/shared_criteria_parity.rs` | 削除 (物理複製解消のため) |
| `crates/belt-agent/tests/cli_test.rs` | `feature_dev_migrated_pipeline_boots` の期待 phase id 修正 |
| `docs/testing/lock-ledger.md` | 上記 lock test 変更の台帳同期 |
| `plugins/belt-agent/skills/protocol/SKILL.md` | invoke.pipeline 節を inline 展開に修正 + Commands 表に locate 追加 |
| `plugins/belt-agent/references/criteria-template.md` | `phase: {N}` frontmatter と `DN-NN` ID convention の序数除去 |
| `plugins/belt/skills/{code-review,spec-review}/SKILL.md` | codex no-response skip 規則を追記 |
| `AGENTS.md` / `README.md` | `uses:` → `invoke.pipeline`、CLI コマンド一覧、skill 一覧更新 |
| `plugins/{belt,belt-agent}/.claude-plugin/plugin.json` + `.claude-plugin/marketplace.json` | version 0.3.0 + description 同期 |

**合成後の展開形 (lock 対象):**

```
feature-dev (10 leaves):
  design/design, design/test-scenarios, design/spec-review, design/plan,
  pre-execute-handover/checkpoint,
  build/execute, build/code-review,
  build/verify/monkey-test, build/verify/dogfood,   ← when: args.e2e (build.verify から貫通)
  build/integrate

bug-fix (9 leaves):
  diagnose/rca, diagnose/fix-plan, diagnose/fix-plan-review,
  pre-execute-handover/checkpoint,
  build/execute, build/code-review,
  build/verify/monkey-test, build/verify/dogfood,
  build/integrate
```

**artifact 受け渡し (spec 準拠):** build / verify は上流産物 (design_doc / plan_doc / rca_report / fix_plan_doc / scenarios) を **consumes 宣言しない** (単体 lint が通らないため)。単体実行時の所在確認は build / verify SKILL.md の Entry Check が担う。narrative accumulating consumes は各 pipeline ファイル内に閉じる (build/code-review は execute_notes のみ、verify/dogfood は monkey_test_* のみ)。

---

### Task 1: verify stage skill 新設

**Files:**
- Create: `plugins/belt/skills/verify/pipeline.yml`
- Create: `plugins/belt/skills/verify/belt.toml`
- Create: `plugins/belt/skills/verify/SKILL.md`
- Create: `plugins/belt/skills/verify/criteria/monkey-test.md`
- Create: `plugins/belt/skills/verify/criteria/dogfood.md`
- Create: `plugins/belt/skills/verify/references/monkey-test-supplement.md`
- Create: `plugins/belt/skills/verify/references/dogfood-supplement.md`

**Interfaces:**
- Produces: `../verify/pipeline.yml` — Task 2 の build が `invoke.pipeline` で参照する。phase ids `monkey-test` / `dogfood`。args 宣言なし (e2e ゲートは composing 側の `when:` が担う)
- Produces: criteria 2 本は structured 記法 / `audit: lite` / ID prefix `MONKEY-TEST-` `DOGFOOD-`

feature-dev 版 (freeform) と bug-fix 版 (structured) の criteria / supplement を統合した**新規著作**。旧 4 ファイル (`feature-dev/{criteria,references}` と `bug-fix/{criteria,references}` の monkey-test / dogfood) はこの Task では触らない (Tasks 5–6 で削除)。

- [ ] **Step 1: pipeline.yml と belt.toml を書く**

`plugins/belt/skills/verify/pipeline.yml`:

```yaml
name: verify
version: 1
description: "Browser-based verification stage (monkey-test -> dogfood)"

phases:
  - id: monkey-test
    description: "Replay pre-defined scenarios via agent-browser"
    invoke:
      skill: /belt:monkey-test
    produces:
      - name: monkey_test_report
        path: "docs/features/*/monkey-test-report.md"
      - name: monkey_test_results
        path: "docs/features/*/monkey-test-results.json"
      - name: monkey_test_notes
        path: "belt://current/notes/phase-monkey-test.md"
        description: "Phase narrative"
    gate:
      - file_exists: "docs/features/*/monkey-test-report.md"
      - file_exists: "belt://current/notes/phase-monkey-test.md"
    validate: ./criteria/monkey-test.md
    confirm: true
    max_retries: 3

  - id: dogfood
    description: "Exploratory testing via agent-browser around the change scope"
    invoke:
      skill: /dogfood
    consumes:
      - monkey_test_report
      - monkey_test_results
      - monkey_test_notes
    produces:
      - name: dogfood_report
        path: "docs/features/*/dogfood-report/report.md"
      - name: dogfood_notes
        path: "belt://current/notes/phase-dogfood.md"
        description: "Phase narrative"
    gate:
      - file_exists: "docs/features/*/dogfood-report/report.md"
      - file_exists: "belt://current/notes/phase-dogfood.md"
    validate: ./criteria/dogfood.md
    confirm: true
    max_retries: 3
```

`plugins/belt/skills/verify/belt.toml`:

```toml
pipeline = "./pipeline.yml"
```

- [ ] **Step 2: criteria/monkey-test.md を書く (structured 統合版)**

```markdown
---
name: monkey-test
max_retries: 3
audit: lite
---

## Criteria

### MONKEY-TEST-01: Monkey test report file exists and is committed
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Search with `Glob("docs/features/*/monkey-test-report.md")`
  2. Run `git status --porcelain -- docs/features/*/monkey-test-*` and confirm zero output lines
- **pass_condition**: At least one Glob match AND zero uncommitted monkey-test artifacts
- **fail_diagnosis_hint**: `/belt:monkey-test` did not produce or commit the report. Confirm the monkey-test supplement was loaded and the scenarios source was resolvable
- **depends_on_artifacts**: [docs/features/*/monkey-test-report.md]

### MONKEY-TEST-02: Results JSON exists and validates against the schema
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Search with `Glob("docs/features/*/monkey-test-results.json")` and parse as JSON
  2. Verify the top-level shape matches the schema in `references/monkey-test-supplement.md` (`scenarios` array + `summary` object with total/passed/failed/skipped)
- **pass_condition**: File exists AND JSON parses AND both `scenarios` and `summary` fields are present
- **fail_diagnosis_hint**: Re-run the report-writing step of `/belt:monkey-test`; compare the emitted JSON against the supplement schema
- **depends_on_artifacts**: [docs/features/*/monkey-test-results.json]

### MONKEY-TEST-03: Every scenario in the source file has a result entry
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Resolve the scenarios source: `docs/features/*/scenarios.yml` (feature runs) or `docs/features/*/rca-scenarios.yml` (bug runs) — whichever exists; on both, prefer the one referenced in the run's notes
  2. Enumerate scenario ids from the source file
  3. Verify each id has a matching entry in `results.json.scenarios` with status `PASS`, `FAIL`, or `SKIP`
- **pass_condition**: Zero source scenarios without a result entry
- **fail_diagnosis_hint**: List missing ids; check whether the replay loop aborted mid-run and resume from the first missing scenario
- **depends_on_artifacts**: [docs/features/*/monkey-test-results.json]

### MONKEY-TEST-04: Reproduction scenario transitions to PASS (bug runs)
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. If the scenarios source is NOT `rca-scenarios.yml`, PASS (vacuously satisfied — feature runs have no reproduction scenario)
  2. Otherwise, the first scenario corresponds to the RCA Reproduction Test; confirm its result in `results.json` is PASS (it FAILed pre-fix per the rca criteria)
- **pass_condition**: Non-bug run, OR first scenario status is PASS
- **fail_diagnosis_hint**: The fix did not resolve the root cause — re-examine the Fix Strategy and execute-phase output. If the scenario itself is malformed, correct its Given/When/Then and re-run monkey-test
- **depends_on_artifacts**: [docs/features/*/monkey-test-results.json, docs/features/*/rca-scenarios.yml]

### MONKEY-TEST-05: Critical/high failures are detailed in the report
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Filter `results.json.scenarios` for status FAIL with severity `critical` or `high`
  2. Verify each such failure appears in the report's primary section with expected-vs-actual and at least one screenshot reference
- **pass_condition**: Zero critical/high FAILs missing from the report's primary section
- **fail_diagnosis_hint**: Cross-reference the missing ids against the report; re-emit the report from results.json
- **depends_on_artifacts**: [docs/features/*/monkey-test-report.md, docs/features/*/monkey-test-results.json]

### MONKEY-TEST-06: SKIP entries carry a non-empty skip_reason
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Filter `results.json.scenarios` for status SKIP
  2. Verify each has a non-empty `skip_reason` (feature runs: referencing the incomplete plan task; bug runs: a documented rationale)
- **pass_condition**: Zero SKIP entries with an empty or missing skip_reason
- **fail_diagnosis_hint**: Identify undocumented SKIPs and either replay them or record why they cannot run
- **depends_on_artifacts**: [docs/features/*/monkey-test-results.json]

### MONKEY-TEST-07: Narrative note captures replay outcomes
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read `belt-agent status` and locate the artifact `monkey_test_notes` resolved_path
  2. Verify the file exists at the resolved_path
  3. Verify frontmatter contains `phase: monkey-test` and `run_id: <run_id>`
  4. Verify 4 required sections exist: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  5. Verify Observations records per-scenario results (bug runs: whether the reproduction scenario now PASSes)
  6. Verify Directives carries forward dogfood exploration targets surfaced by replay
- **pass_condition**: Steps 1-6 all pass
- **fail_diagnosis_hint**: If Observations lacks outcomes, re-derive from `monkey-test-results.json`. If Directives empty, identify regression hotspots dogfood should explore. See `plugins/belt-agent/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [monkey_test_notes]

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

- [ ] **Step 3: criteria/dogfood.md を書く (structured 統合版)**

```markdown
---
name: dogfood
max_retries: 3
audit: lite
---

## Criteria

### DOGFOOD-01: Dogfood report exists (directory form) and is committed
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Search with `Glob("docs/features/*/dogfood-report/report.md")`
  2. Run `git status --porcelain -- docs/features/*/dogfood-report/` and confirm zero output lines
- **pass_condition**: At least one Glob match AND zero uncommitted changes under the report directory
- **fail_diagnosis_hint**: `/dogfood` default output is `./dogfood-output/`; the dogfood supplement must override it to `docs/features/<topic>/dogfood-report/`. Confirm the supplement was loaded
- **depends_on_artifacts**: [docs/features/*/dogfood-report/report.md]

### DOGFOOD-02: Must-Verify Checklist is verified (feature runs)
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. If `docs/features/<topic>/design.md` does not exist (bug runs), PASS (vacuously satisfied)
  2. Otherwise verify every item in design.md's `Must-Verify Checklist` has a status (`PASS`, `FAIL`, `N/A`) in the report's `Must-Verify Checklist Verification` section
- **pass_condition**: No design.md, OR zero checklist items without a recorded status
- **fail_diagnosis_hint**: List unverified items and explore each; N/A requires a one-line justification
- **depends_on_artifacts**: [docs/features/*/design.md, docs/features/*/dogfood-report/report.md]

### DOGFOOD-03: Root Cause mechanism does not re-emerge (bug runs)
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. If `docs/features/<topic>/rca-report.md` does not exist (feature runs), PASS (vacuously satisfied)
  2. Otherwise read the RCA `## Root Cause` mechanism and confirm the report contains an explicit re-verification statement (e.g. "after the fix, the <mechanism> condition no longer triggers"), and that exploration found no re-manifestation in adjacent code paths
- **pass_condition**: Non-bug run, OR (re-verification statement present AND zero re-emergence findings)
- **fail_diagnosis_hint**: Fix is incomplete or asymmetric. Re-examine the RCA Symmetry Check output and Fix Strategy
- **depends_on_artifacts**: [docs/features/*/rca-report.md, docs/features/*/dogfood-report/report.md]

### DOGFOOD-04: Every monkey-test FAIL is addressed in the report
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read all FAIL entries from `monkey-test-results.json`
  2. Verify each is addressed in the report's `Known Issues Re-encountered` section (still broken / now passing)
- **pass_condition**: Zero FAIL entries missing from the section
- **fail_diagnosis_hint**: Retry each missing FAIL by hand and record the observation
- **depends_on_artifacts**: [docs/features/*/monkey-test-results.json, docs/features/*/dogfood-report/report.md]

### DOGFOOD-05: Evidence exists (screenshots/videos OR CLI-only rationale)
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Check `docs/features/<topic>/dogfood-report/screenshots/` or `.../videos/` for at least one evidence file
  2. If the change scope contains zero UI files (CLI / API / backend-only), accept a rationale paragraph in report.md explaining the CLI-only exploration scope instead
- **pass_condition**: >= 1 evidence file, OR a CLI-only rationale paragraph is present
- **fail_diagnosis_hint**: For UI-touching changes `/dogfood` emits screenshots by default; for CLI-only changes ensure the rationale paragraph was added per the supplement
- **depends_on_artifacts**: [docs/features/*/dogfood-report/]

### DOGFOOD-06: New issues documented, or an explicit all-clear
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Verify the report either documents newly found issues (severity, reproduction steps, evidence), OR explicitly states "No critical or high issues found" with a rationale paragraph
- **pass_condition**: One of the two forms is present
- **fail_diagnosis_hint**: An empty findings section without an explicit all-clear reads as unfinished exploration — add one or the other
- **depends_on_artifacts**: [docs/features/*/dogfood-report/report.md]

### DOGFOOD-07: Summary counts are consistent with detail sections
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Compare the report's `Summary` counts (new issues by severity, known issues re-encountered, checklist coverage) against the corresponding detail sections
- **pass_condition**: Zero count mismatches
- **fail_diagnosis_hint**: Recount from the detail sections and correct the Summary
- **depends_on_artifacts**: [docs/features/*/dogfood-report/report.md]

### DOGFOOD-08: Narrative note captures exploratory results
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read `belt-agent status` and locate the artifact `dogfood_notes` resolved_path
  2. Verify the file exists at the resolved_path
  3. Verify frontmatter contains `phase: dogfood` and `run_id: <run_id>`
  4. Verify 4 required sections exist: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  5. Verify Observations records exploration coverage (feature runs: beyond-script findings; bug runs: Symmetry-Pair probe results)
  6. Verify Concerns flags unresolved risks and Directives carries forward regression guards for integrate
- **pass_condition**: Steps 1-6 all pass
- **fail_diagnosis_hint**: If Observations is thin, re-derive from the report's detail sections. If Concerns is empty, explicitly affirm that no regression signals surfaced. See `plugins/belt-agent/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [dogfood_notes]

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

- [ ] **Step 4: references/monkey-test-supplement.md を書く (統合版)**

```markdown
---
name: monkey-test-supplement
description: >-
  verify stage only. Read BEFORE invoking /belt:monkey-test to resolve the
  scenarios source, inject prior-phase artifacts as interpretation hints, and
  fix output paths.
---

# Monkey-Test Supplement for the verify stage

Read BEFORE invoking `/belt:monkey-test`. Path convention reference:
`plugins/belt/skills/design/references/path-convention.md`.

## Scenarios Source

- Feature runs: `docs/features/<topic>/scenarios.yml`
- Bug runs: `docs/features/<topic>/rca-scenarios.yml`

Resolve whichever exists for the current topic. On glob collision (multiple
topics), select the most recently modified (mtime DESC). If neither exists,
monkey-test cannot run — pause and report per the verify SKILL.md Entry Check.

## Hint Inputs (read those that exist into context)

- `docs/features/<topic>/design.md`
  - Resolve ambiguity in scenarios' natural-language Given/When/Then
    (e.g., "valid email" -> the exact validation rule from design).
  - Use Impact Analysis to predict likely regressions.
- `docs/features/<topic>/test-strategy.md`
  - Use `category`/`severity` from each scenario's matching strategy entry
    to set the failure severity in results.
- `docs/features/<topic>/plan.md` (or `fix-plan.md`)
  - Use to decide `SKIP` verdicts: if a scenario targets a feature whose
    implementing task is incomplete, SKIP with a reason.
- `docs/features/<topic>/rca-report.md` (bug runs)
  - The first scenario in `rca-scenarios.yml` corresponds to the RCA
    Reproduction Test. After the fix it is expected to PASS (it FAILed
    pre-fix). `criteria/monkey-test.md` MONKEY-TEST-04 verifies this
    transition. Subsequent scenarios cover Symmetry pairs and Impact Scope
    regressions from the RCA report.

## Output Paths

- `docs/features/<topic>/monkey-test-report.md` — human-readable
- `docs/features/<topic>/monkey-test-results.json` — machine-readable
- `docs/features/<topic>/monkey-test-screenshots/` — step screenshots

## Behavior

1. Parse the scenarios source; collect `id`, `given`, `when`, `then`, `severity`.
2. For each scenario:
   a. Determine SKIP if an associated plan task is incomplete.
   b. Launch agent-browser (restore auth-state if present).
   c. Interpret `given` -> navigate/setup; `when` -> actions; `then` ->
      assertions. Resolve ambiguity via `design.md` / `rca-report.md`.
   d. Capture a screenshot at each step (save under
      `docs/features/<topic>/monkey-test-screenshots/` — create if missing).
   e. Record result.
3. After all scenarios, write both outputs.

## results.json Schema

```json
{
  "scenarios": [
    {
      "id": "string",
      "status": "PASS | FAIL | SKIP",
      "severity": "critical | high | medium | low",
      "duration_ms": 1234,
      "error": "string (only when FAIL)",
      "skip_reason": "string (only when SKIP)",
      "screenshots": ["docs/features/<topic>/monkey-test-screenshots/<id>-step1.png", "..."]
    }
  ],
  "summary": {
    "total": 10,
    "passed": 8,
    "failed": 1,
    "skipped": 1
  }
}
```

## Completion Criteria (for the monkey-test gate)

- Both output files exist and are committed.
- Every scenario id in the source file is present in `results.json.scenarios`.
- `results.json` validates against the schema above.
- Every FAIL with severity `critical` or `high` is surfaced in the primary
  section of `monkey-test-report.md`.
- Bug runs: the first (reproduction) scenario PASSes.
```

- [ ] **Step 5: references/dogfood-supplement.md を書く (統合版)**

```markdown
---
name: dogfood-supplement
description: >-
  verify stage only. Read BEFORE invoking /dogfood to override the output
  directory, scope exploration to the change diff, filter severity, and inject
  prior-phase artifacts as exploration hints.
---

# Dogfood Supplement for the verify stage

Read BEFORE invoking `/dogfood`. Path convention reference:
`plugins/belt/skills/design/references/path-convention.md`.

## Output Path Override

```
docs/features/<topic>/dogfood-report/
├── report.md
├── screenshots/
└── videos/
```

This overrides /dogfood's default `./dogfood-output/`. Always create the
`screenshots/` and `videos/` directories (empty is acceptable) to keep the
artifact structure consistent.

## Scope Override

Restrict exploration to code areas changed by the branch:

```bash
git diff <base>..HEAD --name-only
```

Map changed files to corresponding UI pages/components; prioritize those.
Do NOT explore the full site. Bug runs additionally prioritize:

1. **Impact Scope areas** from `fix-plan.md` — the files and modules modified by the fix
2. **Symmetry pairs** from the RCA report — paired paths that may exhibit the same mechanism
3. **Root Cause mechanism re-emergence** — verify the mechanism described in
   the RCA `## Root Cause` section does not recur in adjacent code paths, and
   include an explicit statement in the report:

   > After the fix, the <mechanism description from RCA Root Cause> condition
   > no longer triggers. Verified by: <specific exploration path / inputs>.

   This satisfies `criteria/dogfood.md` DOGFOOD-03.

## Severity Filter

- `critical` and `high` issues: full detail in report.md primary section.
- `medium` and `low` issues: summary only (counts + one-line description).

## Context Injection (read those that exist BEFORE starting exploration)

### 1. `docs/features/<topic>/design.md` (feature runs)
Focus on: **Prerequisites** (a violation is a likely bug), **Impact Scope**,
**Impact Analysis > Side Effect Risks** (attempt to reproduce each risk), and
**Must-Verify Checklist** (VERIFY EVERY ITEM during dogfood).

### 2. `docs/features/<topic>/rca-report.md` + `fix-plan.md` (bug runs)
Focus on: **Root Cause** mechanism (re-verification target), **Symmetry
Check** pairs, and the fix's **Impact Scope**.

### 3. `docs/features/<topic>/test-strategy.md`
Focus on non-functional requirements and boundary / state-transition items
requiring exotic combinations — typically uncovered by scripted tests.

### 4. `docs/features/<topic>/scenarios.yml` / `rca-scenarios.yml`
Use to AVOID redundant exploration of scripted paths. Spend effort on
combinations NOT in the scenarios file (scenario A then B, mid-flow
interrupt, concurrent operations, long-idle resumes).

### 5. `docs/features/<topic>/monkey-test-results.json`
- Read all `FAIL` entries. Retry each by hand: still broken -> file as
  "Known issue re-encountered" (do not double-count); fixed -> note as
  "Previously failed, now passing".
- Read all `SKIP` entries. Verify the SKIP reason still holds.

## CLI-only Graceful Degradation (UI-free changes)

When the change scope contains **zero UI files** (CLI / API / backend-only):

1. Substitute visual exploration with CLI output capture (stdout / stderr),
   API response inspection (JSON / headers), log file inspection, and DB
   state queries.
2. DOGFOOD-05's evidence requirement is satisfied by a rationale paragraph
   in `report.md`:

   > The change scope contains no UI files (<list affected paths>).
   > Exploration is CLI-only; evidence is captured as CLI output / API
   > response / log excerpts in this report.

## Report Structure

```markdown
# Dogfood Report: <topic>

## Summary
- Exploration time: XX min
- Pages visited: N
- New issues found: { critical: N, high: N, medium: N, low: N }
- Known issues re-encountered: N (from monkey-test-results.json)
- Must-Verify Checklist: X/Y items verified (feature runs; list any unverified)

## Critical and High Issues (new findings)
<per-issue: id, severity, repro steps, screenshot/video evidence>

## Must-Verify Checklist Verification  (feature runs)
<table: item, status (PASS/FAIL/N/A), notes>

## Root Cause Re-verification  (bug runs)
<explicit mechanism re-verification statement + exploration coverage map>

## Known Issues Re-encountered
<per-issue: scenario id, status from monkey-test, dogfood observation>

## Medium and Low Issues (summary)
<counts plus one-line descriptions>
```

## Completion Criteria (for the dogfood gate)

- `docs/features/<topic>/dogfood-report/report.md` exists and is committed.
- Feature runs: every Must-Verify Checklist item has a verification status.
- Bug runs: the Root Cause re-verification statement is present.
- Every `FAIL` scenario in `monkey-test-results.json` is addressed in
  "Known Issues Re-encountered".
- Evidence files exist, or the CLI-only rationale paragraph is present.
```

- [ ] **Step 6: SKILL.md を書く**

`plugins/belt/skills/verify/SKILL.md`:

```markdown
---
name: verify
description: >-
  Runs the browser-based verification stage: scripted scenario replay
  (monkey-test) followed by exploratory testing (dogfood). Use standalone to
  verify a change against existing scenarios, or composed as the e2e leg of
  /belt:build. Requires agent-browser.
user-invocable: true
---

# verify

Belt pipeline for the browser-based verification stage. Pipeline structure,
phase order, and invocation mapping are defined in `pipeline.yml` and surfaced
dynamically by `belt-agent next` / `belt-agent status`. This SKILL.md covers
the entry check, supplement loading contract, and red flags.

## Entry Check (standalone runs)

Before `belt-agent init`, locate the scenarios source:

- `docs/features/<topic>/scenarios.yml` (feature runs), or
- `docs/features/<topic>/rca-scenarios.yml` (bug runs).

On glob collision, prefer the most recently modified. If neither exists,
pause and ask the user — monkey-test has nothing to replay. When this stage
runs composed under `/belt:build` (phase ids `verify/monkey-test` /
`verify/dogfood`), the same resolution applies at the monkey-test phase.

## Supplement Loading

Before invoking a phase's skill, read the referenced supplement to inject
phase-specific overrides.

| Phase | Supplement | Purpose |
|---|---|---|
| monkey-test | `./references/monkey-test-supplement.md` | scenarios source resolution, context injection, reproduction-scenario rule (bug runs), output paths |
| dogfood | `./references/dogfood-supplement.md` | output override, diff-scoped exploration, Root Cause re-verification (bug runs), CLI-only degradation |

## Narrative Notes

Both phases produce a narrative note so context can be restored after
`/clear`. Note paths are declared in `pipeline.yml` as
`belt://current/notes/phase-<id>.md` URIs; resolve the physical path via
`belt-agent status` (read `phases[].produces[].resolved_path`) or
`belt-agent locate belt://current/notes/phase-<id>.md`.

Each note contains four sections (`## Decisions` / `## Concerns` /
`## Directives` / `## Observations`) and minimal frontmatter (`phase`,
`run_id`).

Full convention: [`plugins/belt-agent/references/narrative-convention.md`](plugins/belt-agent/references/narrative-convention.md)

## Red Flags

- **Never skip supplement loading**: output paths and scope overrides live there.
- **Never write files outside `docs/features/<topic>/`.**
- **Never auto-retry FAIL scenarios silently** — report them.
- **Never leave the narrative note's four sections blank**: use `(none)` placeholders, keep headings.

## References

- `./references/monkey-test-supplement.md` — monkey-test phase overrides
- `./references/dogfood-supplement.md` — dogfood phase overrides
- `plugins/belt/skills/design/references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `plugins/belt-agent/references/narrative-convention.md` — narrative note schema
```

- [ ] **Step 7: belt lint で検証する**

Run: `cargo run -p belt -- lint plugins/belt/skills/verify/pipeline.yml && echo LINT-OK`
Expected: `LINT-OK` (exit 0、diagnostics なし)

Run: `cargo test -p belt-core`
Expected: 全 PASS (既存テストに影響なし)

- [ ] **Step 8: Commit**

```bash
git add plugins/belt/skills/verify/
git commit -m "feat(plugins): add verify stage pipeline (monkey-test -> dogfood)"
```

---

### Task 2: build stage skill 新設 + 共有 criteria 1 本化 (parity lock 削除)

**Files:**
- Create: `plugins/belt/skills/build/pipeline.yml`
- Create: `plugins/belt/skills/build/belt.toml`
- Create: `plugins/belt/skills/build/SKILL.md`
- git mv: `plugins/belt/skills/feature-dev/criteria/execute.md` → `plugins/belt/skills/build/criteria/execute.md` (+ path 補正)
- git mv: `plugins/belt/skills/feature-dev/criteria/code-review.md` → `plugins/belt/skills/build/criteria/code-review.md` (無編集)
- Create: `plugins/belt/skills/build/criteria/integrate.md` (fd/bf 統合の新規著作)
- git mv: `plugins/belt/skills/feature-dev/references/worktrunk-supplement.md` → `plugins/belt/skills/build/references/worktrunk-supplement.md` (+ bf 差分 fold-in)
- git mv: `plugins/belt/skills/feature-dev/references/evidence-catalog.md` → `plugins/belt/skills/build/references/evidence-catalog.md` (+ 中立化 3 行)
- Delete: `crates/belt-core/tests/shared_criteria_parity.rs`
- Modify: `docs/testing/lock-ledger.md` (shared_criteria_parity 節を削除、feature_dev/bug_fix 節の cross-coupling リストから同名行を削除)

**Interfaces:**
- Consumes: Task 1 の `plugins/belt/skills/verify/pipeline.yml` (build.verify が `../verify/pipeline.yml` を参照)
- Produces: `../build/pipeline.yml` — Tasks 5–6 の合成が参照。phase ids `execute` / `code-review` / `verify` (sub) / `integrate`。args `{e2e, codex}`。`code-review.regate = [execute]` はこのファイル内で閉じる
- Produces: `build/criteria/{execute,code-review,integrate}.md`、`build/references/{worktrunk-supplement,evidence-catalog}.md`

**注意 (transient):** この Task で feature-dev 側の execute.md / code-review.md / worktrunk-supplement.md / evidence-catalog.md が消えるため、旧 `feature-dev/pipeline.yml` の belt lint は Task 5 まで FAIL する (cargo test は green — Global Constraints の transient 節参照)。bug-fix 側のコピーは Task 6 まで無傷。`shared_criteria_parity.rs` は比較元 (feature-dev 側) が消えるため**この commit で削除必須** (ledger 同期も同 commit)。

- [ ] **Step 1: pipeline.yml と belt.toml を書く**

`plugins/belt/skills/build/pipeline.yml`:

```yaml
name: build
version: 1
description: "Shared build stage (execute -> code-review -> verify -> integrate)"

args:
  e2e:
    type: bool
    default: false
    description: "Run the browser-based verify stage (monkey-test, dogfood)"
  codex:
    type: bool
    default: false
    description: "Enable Codex parallel review in code-review"

phases:
  - id: execute
    description: "Execute the implementation plan via TDD subagents"
    invoke:
      skill: /subagent-driven-development
    produces:
      - name: execute_notes
        path: "belt://current/notes/phase-execute.md"
        description: "Phase narrative"
    gate:
      - file_exists: "belt://current/notes/phase-execute.md"
    validate: ./criteria/execute.md
    confirm: true
    max_retries: 3

  - id: code-review
    description: "Multi-perspective code review"
    invoke:
      skill: /belt:code-review
      args:
        codex: "args.codex"
    consumes:
      - execute_notes
    produces:
      - name: findings-security
        path: "belt://current/review/findings-security.json"
        description: "Security observation findings"
      - name: findings-test
        path: "belt://current/review/findings-test.json"
        description: "Test observation findings"
      - name: findings-ai-antipattern
        path: "belt://current/review/findings-ai-antipattern.json"
        description: "AI antipattern observation findings"
      - name: findings-cross-cutting
        path: "belt://current/review/findings-cross-cutting.json"
        description: "Cross-cutting observation findings"
      - name: findings-codex
        path: "belt://current/review/findings-codex.json"
        description: "Codex adversarial review findings"
        when: "args.codex"
      - name: findings
        path: "belt://current/review/findings.json"
        description: "Merged findings (post-dedup)"
      - name: code_review_notes
        path: "belt://current/notes/phase-code-review.md"
        description: "Phase narrative"
    gate:
      - file_exists: "belt://current/notes/phase-code-review.md"
      - file_exists: "belt://current/review/findings.json"
    validate: ./criteria/code-review.md
    regate: [execute]
    confirm: true
    max_retries: 3

  - id: verify
    when: "args.e2e"
    invoke:
      pipeline: ../verify/pipeline.yml

  - id: integrate
    description: "Integrate changes (user chooses: wt merge or gh pr create)"
    invoke:
      skill: /worktrunk
    validate: ./criteria/integrate.md
    confirm: true
    max_retries: 3
```

`plugins/belt/skills/build/belt.toml`:

```toml
pipeline = "./pipeline.yml"
```

- [ ] **Step 2: 共有 criteria を git mv して path 補正する**

```bash
mkdir -p plugins/belt/skills/build/criteria plugins/belt/skills/build/references
git mv plugins/belt/skills/feature-dev/criteria/execute.md plugins/belt/skills/build/criteria/execute.md
git mv plugins/belt/skills/feature-dev/criteria/code-review.md plugins/belt/skills/build/criteria/code-review.md
```

`build/criteria/code-review.md` は無編集。`build/criteria/execute.md` に以下の編集を加える:

(a) `## Criteria` の直前 (frontmatter の後) に document mapping 注記を挿入:

```markdown
Document mapping: feature runs read `design.md` / `plan.md`; bug runs read
`rca-report.md` / `fix-plan.md` as the equivalent design/plan pair. Criteria
below say "design document" / "plan document" generically.

## Criteria
```

(b) `depends_on_artifacts` の docs/plans パスを docs/features に置換 (5 箇所、exact match):

| 対象 | old | new |
|---|---|---|
| EXECUTE-01 | `[docs/plans/*-plan.md]` | `[docs/features/*/plan.md, docs/features/*/fix-plan.md]` |
| EXECUTE-05 | `[docs/plans/*-plan.md, tests/]` | `[docs/features/*/plan.md, docs/features/*/fix-plan.md, tests/]` |
| EXECUTE-06 | `[docs/plans/*-design.md, src/]` | `[docs/features/*/design.md, docs/features/*/rca-report.md, src/]` |
| EXECUTE-07 | `[docs/plans/*-design.md, docs/plans/*-plan.md, src/]` | `[docs/features/*/design.md, docs/features/*/plan.md, docs/features/*/rca-report.md, docs/features/*/fix-plan.md, src/]` |
| EXECUTE-09 | `[docs/plans/*-design.md, tests/, src/]` | `[docs/features/*/design.md, docs/features/*/rca-report.md, tests/, src/]` |

確認: `grep -c "docs/plans" plugins/belt/skills/build/criteria/execute.md` → `0`

- [ ] **Step 3: criteria/integrate.md を書く (structured 統合版)**

```markdown
---
name: integrate
max_retries: 3
audit: lite
---

## Criteria

### INTEGRATE-01: Integration method was chosen by the user and executed
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. The worktrunk-supplement A/B prompt was presented to the user
  2. Either `wt merge` (option A) or `gh pr create` (option B) was executed
  3. Execution logs (or git state) reflect the chosen method
- **pass_condition**: One of the two methods was executed per an explicit user choice
- **fail_diagnosis_hint**: If no explicit choice is recorded, re-present the A/B prompt — never default silently
- **depends_on_artifacts**: []

### INTEGRATE-02: All pre-merge checks pass
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Project test suite (e.g. `cargo test`) exit 0
  2. Project linter (e.g. `cargo clippy --workspace -- -D warnings`) exit 0
  3. Formatter check (e.g. `cargo fmt --check`) exit 0 for modified packages
  4. `belt lint` exit 0 for any modified pipeline.yml files
- **pass_condition**: All applicable checks exit 0
- **fail_diagnosis_hint**: Fix failures before integration; a red pre-merge check must never be merged around
- **depends_on_artifacts**: []

### INTEGRATE-03: Merge flow completed (A selected)
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. If option B was selected, PASS (vacuously satisfied)
  2. Verify a merge commit containing the branch exists on the parent branch and the pre-merge hook succeeded
  3. Verify the worktree has been removed (`wt list` no longer lists the branch)
- **pass_condition**: Option B, OR (merge commit exists AND worktree removed)
- **fail_diagnosis_hint**: If the pre-merge hook failed, resolve and re-run `wt merge`; if the worktree remains, run `wt remove`
- **depends_on_artifacts**: []

### INTEGRATE-04: PR flow completed (B selected)
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. If option A was selected, PASS (vacuously satisfied)
  2. Verify a PR exists on origin with a non-empty body whose sections (`Summary`, `Changes`, `Testing`, `Must-Verify Checklist`, `Spec and Plan`) are populated from the worktrunk-supplement template
  3. Verify no literal `<...>` placeholder remains in the published body
- **pass_condition**: Option A, OR (PR exists AND all template sections populated AND zero placeholders)
- **fail_diagnosis_hint**: Re-generate the body from the template and `gh pr edit` the published PR
- **depends_on_artifacts**: []

### INTEGRATE-05: All produced artifacts are present at the integrated commit
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Enumerate the run's produced domain artifacts under `docs/features/<topic>/`
  2. Verify each is present in the parent branch at the merge commit (A) or at the PR head commit (B)
- **pass_condition**: Zero missing artifacts at the integrated commit
- **fail_diagnosis_hint**: An uncommitted or unpushed artifact — commit it and amend the merge/PR
- **depends_on_artifacts**: [docs/features/]

### INTEGRATE-06: Reproduction test PASSes on the integrated branch (bug runs)
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. If `docs/features/<topic>/rca-report.md` does not exist (feature runs), PASS (vacuously satisfied)
  2. Re-run the reproduction test identified in the RCA report's Reproduction Test section on the integrated branch (post-merge main, or the PR head)
  3. Confirm it PASSes (it FAILed pre-fix)
- **pass_condition**: Non-bug run, OR reproduction test PASSes on the integrated branch
- **fail_diagnosis_hint**: The merge introduced a regression or test expectations drifted — review the integration diff
- **depends_on_artifacts**: [docs/features/*/rca-report.md]

### INTEGRATE-07: No uncommitted changes remain
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Run `git status --porcelain` in the worktree (A: before `wt remove`; B: at PR head) and confirm zero output lines.
- **pass_condition**: `git status --porcelain` output is empty
- **fail_diagnosis_hint**: Commit or intentionally discard the stragglers before closing the phase
- **depends_on_artifacts**: []

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

- [ ] **Step 4: references を git mv して統合する**

```bash
git mv plugins/belt/skills/feature-dev/references/worktrunk-supplement.md plugins/belt/skills/build/references/worktrunk-supplement.md
git mv plugins/belt/skills/feature-dev/references/evidence-catalog.md plugins/belt/skills/build/references/evidence-catalog.md
```

`build/references/evidence-catalog.md` の中立化 (3 行、feature-dev 表記のみ):

| old | new |
|---|---|
| `Concrete evidence catalog for feature-dev pipeline. Conforms to` | `Concrete evidence catalog for the build stage. Conforms to` |
| `# Evidence Catalog (feature-dev)` | `# Evidence Catalog (build)` |
| `Concrete evidence items available to feature-dev pipeline phases. Each` | `Concrete evidence items available to build stage phases. Each` |

`build/references/worktrunk-supplement.md` の編集 (fd 版がベース。bug-fix 版の固有内容を fold-in し序数を除去):

(a) frontmatter description と見出し・冒頭:

```markdown
---
name: worktrunk-supplement
description: >-
  build stage integrate phase only. Read BEFORE invoking /worktrunk to define
  the merge-vs-PR user choice flow, pre-merge checks, and the PR-body template.
---

# Worktrunk Supplement for the build stage (integrate)

Read BEFORE invoking `/worktrunk` in the integrate phase.
```

(旧: `feature-dev Phase 8 only. Read BEFORE invoking /worktrunk to define the` / `merge-vs-PR user choice flow and the PR-body template.` / `# Worktrunk Supplement for feature-dev (Phase 8 Integrate)` / `Read BEFORE invoking `/worktrunk` in Phase 8.`)

(b) `## Required User Prompt` 内の `At the start of Phase 8, present exactly:` → `At the start of the integrate phase, present exactly:`

(c) `## Required User Prompt` の後、`## (A) Merge Flow` の前に bug-fix 版から Pre-merge Checks 節を挿入:

```markdown
## Branch Naming

- Feature runs: `feature/<YYYY-MM-DD-topic>`
- Bug runs: `bugfix/<YYYY-MM-DD-topic>`

See `plugins/belt/skills/design/references/path-convention.md`.

## Pre-merge Checks

Before invoking `/worktrunk`:

1. Project test suite (e.g. `cargo test`) exit 0
2. Project linter (e.g. `cargo clippy --workspace -- -D warnings`) exit 0
3. Formatter check (e.g. `cargo fmt --check`) exit 0 for modified packages
4. `belt lint` exit 0 for any modified pipeline.yml files
5. Bug runs: the reproduction test (from the RCA report) PASSes on this branch

If any check fails, abort the integrate phase and report to the user — do
NOT merge.
```

(d) `## PR Body Template` の後、`## Completion Criteria` の前に bug-fix 版から commit 規約節を挿入:

```markdown
## Commit Message Convention (bug runs)

Fix commits (from the execute phase) should follow:

```
fix(<scope>): <short description of bug fix>
```

Where `<scope>` is derived from the RCA Impact Scope (primary module).
Example: `fix(auth): redirect expired session cookies to /login instead of 500`.
```

(e) 末尾 `## Completion Criteria (for Phase 8 gate)` → `## Completion Criteria (for the integrate gate)`

- [ ] **Step 5: SKILL.md を書く**

`plugins/belt/skills/build/SKILL.md`:

```markdown
---
name: build
description: >-
  Runs the shared build stage: TDD implementation, multi-perspective code
  review, optional browser-based verification, and integration. Use standalone
  with a hand-written or pre-existing plan, or composed as the downstream
  stage of /belt:feature-dev and /belt:bug-fix. --e2e runs the verify
  sub-stage; --codex enables adversarial review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# build

Belt pipeline for the shared build stage. Pipeline structure, args, phase
order, and invocation mapping are defined in `pipeline.yml` and surfaced
dynamically by `belt-agent next` / `belt-agent status`. This SKILL.md covers
the entry check, supplement loading contract, phase-specific runtime notes,
and red flags.

## Entry Check (standalone runs)

Before `belt-agent init`, confirm an implementation plan exists:
`docs/features/<topic>/plan.md` (feature runs), `docs/features/<topic>/fix-plan.md`
(bug runs), or a user-provided plan document. If none exists, pause and ask
the user — execute has nothing to implement. The build stage intentionally
declares no upstream `consumes`; locating the plan is this skill's job.

## Supplement Loading

Before invoking a phase's skill, read the referenced supplement to inject
phase-specific overrides. Phases not listed (`execute` / `code-review`) have
no supplement; invoke their declared skill directly.

| Phase | Supplement | Purpose |
|---|---|---|
| integrate | `./references/worktrunk-supplement.md` | A/B choice logic (wt merge / gh pr create), pre-merge checks, PR-body template |

## Stage Delegation

When `args.e2e` is true, `next` returns the verify sub-stage's leaf phases as
`verify/monkey-test` and `verify/dogfood`. Before executing them, read
`plugins/belt/skills/verify/SKILL.md` — its entry check and supplements apply.

## Phase-specific Runtime Notes

- **execute**: orchestrator must reconstruct plan tasks into self-contained
  implementation specs before dispatching `belt-agent:feature-implementer`
  subagents. Do not forward broad research or plan excerpts verbatim.
- **integrate**: prompt user for mode (A: `wt merge` / B: `gh pr create`)
  before executing `/worktrunk`.

## Narrative Notes

`execute` and `code-review` produce a narrative note so context can be
restored after `/clear`. Note paths are declared in `pipeline.yml` as
`belt://current/notes/phase-<id>.md` URIs; resolve the physical path via
`belt-agent status` (read `phases[].produces[].resolved_path`) or
`belt-agent locate belt://current/notes/phase-<id>.md`.

Each note contains four sections (`## Decisions` / `## Concerns` /
`## Directives` / `## Observations`) and minimal frontmatter (`phase`,
`run_id`).

Full convention: [`plugins/belt-agent/references/narrative-convention.md`](plugins/belt-agent/references/narrative-convention.md)

## Red Flags

- **Never start execute without the Entry Check**: implementing without a plan is the anti-pattern this stage exists to prevent.
- **Never filter or omit review findings**: triage of `/belt:code-review` output is the user's responsibility.
- **Never bypass the integrate A/B choice**: merge-vs-PR is always user-decided.
- **Never modify the consumed global skills**: overrides go through `references/*-supplement.md`.
- **Never leave the narrative note's four sections blank**: use `(none)` placeholders, keep headings.

## References

- `./references/worktrunk-supplement.md` — integrate phase A/B choice logic
- `./references/evidence-catalog.md` — evidence items for execute / code-review criteria
- `plugins/belt/skills/verify/SKILL.md` — verify sub-stage contract (when `--e2e`)
- `plugins/belt/skills/design/references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `plugins/belt-agent/references/narrative-convention.md` — narrative note schema
```

- [ ] **Step 6: shared_criteria_parity.rs と ledger を削除・同期する**

```bash
git rm crates/belt-core/tests/shared_criteria_parity.rs
```

`docs/testing/lock-ledger.md` の編集:
1. `## shared_criteria_parity.rs` 節全体 (見出しから次の `---` まで) を削除
2. `## feature_dev_refresh.rs` 節の cross-coupling リストから `  - crates/belt-core/tests/shared_criteria_parity.rs` の行を削除
3. `## bug_fix_refresh.rs` 節の cross-coupling リストから `  - crates/belt-core/tests/shared_criteria_parity.rs` の行を削除
4. 同じく両節の Cross-coupling (C) 本文から `shared_criteria_parity.rs` を参照する bullet を削除

- [ ] **Step 7: 検証する**

Run: `cargo run -p belt -- lint plugins/belt/skills/build/pipeline.yml && echo LINT-OK`
Expected: `LINT-OK` (verify サブ参照が Task 1 の実ファイルに解決され、nested 展開込みで diagnostics なし)

Run: `cargo test -p belt-core`
Expected: 全 PASS。`shared_criteria_parity` は消えている (`cargo test -p belt-core --test shared_criteria_parity` は "error: no test target named" で FAIL するのが正しい)。`bug_fix_refresh` は bug-fix 側コピーが無傷のため PASS を維持

Run: `cargo test -p belt-core --test scenarios_contract`
Expected: PASS (ledger の locks-file 参照が全て実在)

- [ ] **Step 8: Commit**

```bash
git add plugins/belt/skills/build/ plugins/belt/skills/feature-dev/ docs/testing/lock-ledger.md
git add -u crates/belt-core/tests/
git commit -m "feat(plugins): add build stage pipeline; unify shared criteria (drop parity lock)"
```

---

### Task 3: design stage skill 新設 (criteria structured 変換 + path-convention SSOT 化)

**Files:**
- Create: `plugins/belt/skills/design/pipeline.yml`
- Create: `plugins/belt/skills/design/belt.toml`
- Create: `plugins/belt/skills/design/SKILL.md`
- git mv + 全面書換: `plugins/belt/skills/feature-dev/criteria/{design,test-scenarios,spec-review,plan}.md` → `plugins/belt/skills/design/criteria/` (freeform → structured 変換)
- git mv + 編集: `plugins/belt/skills/feature-dev/references/{brainstorming-supplement,writing-plans-supplement,path-convention}.md` → `plugins/belt/skills/design/references/`

**Interfaces:**
- Consumes: なし (独立)
- Produces: `../design/pipeline.yml` — Task 5 の合成が参照。phase ids `design` / `test-scenarios` / `spec-review` / `plan`。args `{e2e, codex}`。`spec-review.regate = [test-scenarios]` はこのファイル内で閉じる
- Produces: `design/references/path-convention.md` — 全 stage が参照する唯一の SSOT (bug-fix path 規約もここに統合)

- [ ] **Step 1: pipeline.yml と belt.toml を書く**

`plugins/belt/skills/design/pipeline.yml` (phases 本体は現行 feature-dev の 1–4 phase を verbatim 移植):

```yaml
name: design
version: 1
description: "Feature design stage (design -> test-scenarios -> spec-review -> plan)"

args:
  e2e:
    type: bool
    default: false
    description: "Author agent-browser scenarios (scenarios.yml) alongside the test strategy"
  codex:
    type: bool
    default: false
    description: "Enable Codex parallel review in spec-review"

phases:
  - id: design
    description: "Generate design document via interactive brainstorming"
    invoke:
      skill: /brainstorming
    produces:
      - name: design_doc
        path: "docs/features/*/design.md"
        description: "Design document with explored context and test perspectives"
      - name: design_notes
        path: "belt://current/notes/phase-design.md"
        description: "Phase narrative: decisions, concerns, directives, observations"
    gate:
      - file_exists: "docs/features/*/design.md"
      - file_exists: "belt://current/notes/phase-design.md"
    validate: ./criteria/design.md
    confirm: true
    max_retries: 3

  - id: test-scenarios
    description: "Design comprehensive test cases and agent-browser scenarios"
    invoke:
      skill: /belt:test-scenarios
      args:
        e2e: "args.e2e"
    consumes:
      - design_doc
    produces:
      - name: test_strategy
        path: "docs/features/*/test-strategy.md"
        description: "Human-readable test strategy (ISTQB/ISO 25010)"
      - name: scenarios
        path: "docs/features/*/scenarios.yml"
        description: "Agent-browser replay scenarios (Given/When/Then YAML)"
        when: "args.e2e"
    gate:
      - file_exists: "docs/features/*/test-strategy.md"
    validate: ./criteria/test-scenarios.md
    confirm: true
    max_retries: 3

  - id: spec-review
    description: "Review test strategy (and scenarios if --e2e) via spec-review"
    invoke:
      skill: /belt:spec-review
      args:
        codex: "args.codex"
    consumes:
      - test_strategy
    validate: ./criteria/spec-review.md
    regate: [test-scenarios]
    confirm: true
    max_retries: 3

  - id: plan
    description: "Generate implementation plan from design and test strategy"
    invoke:
      skill: /writing-plans
    consumes:
      - design_doc
      - test_strategy
      - design_notes
    produces:
      - name: plan_doc
        path: "docs/features/*/plan.md"
        description: "Task-level implementation plan (TDD)"
      - name: plan_notes
        path: "belt://current/notes/phase-plan.md"
        description: "Phase narrative"
    gate:
      - file_exists: "docs/features/*/plan.md"
      - file_exists: "belt://current/notes/phase-plan.md"
    validate: ./criteria/plan.md
    confirm: true
    max_retries: 3
```

`plugins/belt/skills/design/belt.toml`:

```toml
pipeline = "./pipeline.yml"
```

- [ ] **Step 2: criteria 4 本を git mv して structured に全面書換する**

```bash
mkdir -p plugins/belt/skills/design/criteria plugins/belt/skills/design/references
git mv plugins/belt/skills/feature-dev/criteria/design.md plugins/belt/skills/design/criteria/design.md
git mv plugins/belt/skills/feature-dev/criteria/test-scenarios.md plugins/belt/skills/design/criteria/test-scenarios.md
git mv plugins/belt/skills/feature-dev/criteria/spec-review.md plugins/belt/skills/design/criteria/spec-review.md
git mv plugins/belt/skills/feature-dev/criteria/plan.md plugins/belt/skills/design/criteria/plan.md
```

各ファイルを以下の内容で**全面置換**する (旧 freeform の判定内容を structured へ 1:1 変換。判定基準の実質は変えない)。

`design/criteria/design.md`:

```markdown
---
name: design
max_retries: 3
audit: lite
---

## Criteria

### DESIGN-01: Design document exists and is committed
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Search with `Glob("docs/features/*/design.md")`
  2. Run `git status --porcelain -- docs/features/*/design.md` and confirm zero output lines
- **pass_condition**: At least one Glob match AND zero uncommitted changes for the matched file
- **fail_diagnosis_hint**: Confirm the design phase wrote the document under `docs/features/<YYYY-MM-DD-topic>/` (see `../references/path-convention.md`) and committed it inside the feature worktree
- **depends_on_artifacts**: [docs/features/*/design.md]

### DESIGN-02: Required sections are present
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the design document
  2. Verify headings exist: `Prerequisites`, `Impact Scope`, `Impact Analysis` (with subsections `Reverse Dependencies`, `Shared State`, `Implicit Contracts`, `Side Effect Risks`), `Must-Verify Checklist`, `Test Perspectives`
- **pass_condition**: Zero missing headings from the list above
- **fail_diagnosis_hint**: The brainstorming supplement defines the required sections — resume the design conversation for the missing ones
- **depends_on_artifacts**: [docs/features/*/design.md]

### DESIGN-03: Test Perspectives covers the four case classes
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the `Test Perspectives` section
  2. Verify at least one case exists for EACH of: normal, boundary, abnormal, state-transition
- **pass_condition**: All four classes have at least one case
- **fail_diagnosis_hint**: Derive the missing classes from the design's input parameters and state model
- **depends_on_artifacts**: [docs/features/*/design.md]

### DESIGN-04: Feature worktree branch exists
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Take the topic directory name from the DESIGN-01 Glob match
  2. Run `git branch --list "feature/<YYYY-MM-DD-topic>"` and confirm the branch is listed
- **pass_condition**: The branch exists and its name matches the `docs/features/` directory name
- **fail_diagnosis_hint**: The worktree creation order in the brainstorming supplement was skipped — create the worktree/branch now
- **depends_on_artifacts**: [docs/features/*/design.md]

### DESIGN-05: Baseline tests pass at the design commit
- **severity**: quality
- **verify_type**: automated
- **verification**:
  Run the project-appropriate test command in the worktree (or read the worktrunk pre-start hook output captured at worktree creation) and record the exit code.
- **pass_condition**: Test command exit code is 0
- **fail_diagnosis_hint**: A red baseline invalidates later execute-phase regression attribution — fix the baseline or record the known failures before proceeding
- **depends_on_artifacts**: []

### DESIGN-06: Worktree is clean after the design commit
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Run `git status --porcelain` in the worktree and confirm zero output lines.
- **pass_condition**: `git status --porcelain` output is empty
- **fail_diagnosis_hint**: Commit or intentionally discard the stragglers
- **depends_on_artifacts**: []

### DESIGN-07: Narrative note captures design decisions
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read `belt-agent status` and locate the artifact `design_notes` resolved_path
  2. Verify the file exists at the resolved_path
  3. Verify frontmatter contains `phase: design` and `run_id: <run_id>`
  4. Verify 4 required sections exist: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  5. Verify Decisions records the chosen approach and rejected alternatives with rationale
  6. Verify Directives records constraints for the plan / execute phases
- **pass_condition**: Steps 1-6 all pass; empty sections may carry `(none)` but headings must be present
- **fail_diagnosis_hint**: If Decisions lacks rejected alternatives, re-derive them from the brainstorming dialogue. See `plugins/belt-agent/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [design_notes]

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

`design/criteria/test-scenarios.md`:

```markdown
---
name: test-scenarios
max_retries: 3
audit: lite
---

## Criteria

### TEST-SCENARIOS-01: Test strategy document exists and is committed
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Search with `Glob("docs/features/*/test-strategy.md")`
  2. Run `git status --porcelain -- docs/features/*/test-strategy.md` and confirm zero output lines
- **pass_condition**: At least one Glob match AND zero uncommitted changes
- **fail_diagnosis_hint**: Confirm `/belt:test-scenarios` wrote and committed the strategy under the topic directory
- **depends_on_artifacts**: [docs/features/*/test-strategy.md]

### TEST-SCENARIOS-02: Required strategy sections are present
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the strategy document
  2. Verify sections exist: `Test Design Techniques` (ISTQB-based: equivalence partitioning, boundary-value analysis, decision tables, state transitions), `Quality Characteristics` (ISO 25010-based), `Priority Matrix` mapping characteristics to criticality
- **pass_condition**: All three sections are present
- **fail_diagnosis_hint**: Re-invoke `/belt:test-scenarios` for the missing sections; the section names are its output contract
- **depends_on_artifacts**: [docs/features/*/test-strategy.md]

### TEST-SCENARIOS-03: Must-Verify Checklist items are covered
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate every item ID in the design document's `Must-Verify Checklist`
  2. For each ID, verify at least one corresponding entry exists in the strategy (ID cross-reference)
  3. List IDs with no corresponding entry
- **pass_condition**: Step 3 list is empty
- **fail_diagnosis_hint**: Add strategy entries for the uncovered checklist IDs
- **depends_on_artifacts**: [docs/features/*/design.md, docs/features/*/test-strategy.md]

### TEST-SCENARIOS-04: Scenarios file exists when --e2e
- **severity**: blocker
- **verify_type**: automated + inspection
- **verification**:
  1. Read `args.e2e` from `belt-agent status` JSON output
  2. If `args.e2e=false`, PASS (vacuously satisfied)
  3. If `args.e2e=true`:
     a. Search with `Glob("docs/features/*/scenarios.yml")` and confirm the file is committed
     b. Verify it contains at least 3 scenarios
     c. Verify every scenario has `id` (kebab-case), `category`, `severity` (`critical|high|medium|low`), `given`, `when`, `then`
     d. Verify `preconditions` / `postconditions` are present when applicable
- **pass_condition**: `args.e2e=false`, OR (file exists, committed, >= 3 scenarios, zero scenarios missing required keys)
- **fail_diagnosis_hint**: If missing, the e2e flag did not reach `/belt:test-scenarios` — check the invoke args passthrough
- **depends_on_artifacts**: [docs/features/*/scenarios.yml]

### TEST-SCENARIOS-05: At least one non-functional requirement with acceptance criterion
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Read the strategy document
  2. Verify at least one non-functional requirement (performance, security, or accessibility) is listed with a concrete acceptance criterion (numeric threshold or pattern-matchable assertion)
- **pass_condition**: At least one such requirement with a concrete criterion exists
- **fail_diagnosis_hint**: Derive one from the design's Quality Characteristics discussion; vague adjectives do not count as criteria
- **depends_on_artifacts**: [docs/features/*/test-strategy.md]

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

`design/criteria/spec-review.md`:

```markdown
---
name: spec-review
max_retries: 3
audit: lite
---

## Criteria

### SPEC-REVIEW-01: Strategy structure survives the review
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the post-review `test-strategy.md`
  2. Verify the required sections (`Test Design Techniques` / `Quality Characteristics` / `Priority Matrix`) remain intact (structural parity with TEST-SCENARIOS-02)
- **pass_condition**: All three sections still present after applied fixes
- **fail_diagnosis_hint**: A review fix deleted or renamed a required section — restore it from git history
- **depends_on_artifacts**: [docs/features/*/test-strategy.md]

### SPEC-REVIEW-02: Finding triage is complete
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the merged findings (locate the `findings` artifact via `belt-agent status`)
  2. Verify both the grill-me group and the selection group are fully processed (every finding has a recorded resolution or selection)
- **pass_condition**: Zero unhandled findings
- **fail_diagnosis_hint**: Resume the grill-me dialogue / selection prompt in the main context; the orchestrator must not auto-resolve
- **depends_on_artifacts**: [findings]

### SPEC-REVIEW-03: Only user-approved findings are reflected
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate applied changes to `test-strategy.md` / `scenarios.yml`
  2. Verify each traces to a user-approved finding (grill-me: `accept` or `accept_current`; selection: picked by number)
- **pass_condition**: Zero applied changes without a user-approved finding
- **fail_diagnosis_hint**: Revert unapproved edits; re-run triage if attribution is unclear
- **depends_on_artifacts**: [findings, docs/features/*/test-strategy.md]

### SPEC-REVIEW-04: Applied changes are confined to the deliverable files
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Run `git diff --name-only` against the parent phase baseline
  2. Verify only `docs/features/<topic>/test-strategy.md` (and `scenarios.yml` when `args.e2e` is true) are modified — no source, test, or unrelated doc changes
- **pass_condition**: All diff entries fall within the deliverable scope; zero out-of-scope files
- **fail_diagnosis_hint**: Revert out-of-scope edits or move them to the owning phase
- **depends_on_artifacts**: [docs/features/*/test-strategy.md]

### SPEC-REVIEW-05: Scenarios are in review scope when --e2e
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. If `args.e2e=false`, PASS (vacuously satisfied)
  2. Verify `scenarios.yml` was in scope for the review (scenarios are referenced in the findings)
- **pass_condition**: Non-e2e run, OR scenarios referenced in at least one finding context
- **fail_diagnosis_hint**: Re-dispatch the review with `scenarios.yml` included in the reviewed spec set
- **depends_on_artifacts**: [findings, docs/features/*/scenarios.yml]

### SPEC-REVIEW-06: Modified deliverables are committed
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  Run `git status --porcelain -- docs/features/` and confirm zero output lines.
- **pass_condition**: Zero unstaged/uncommitted deliverable changes
- **fail_diagnosis_hint**: Commit the applied review fixes
- **depends_on_artifacts**: [docs/features/*/test-strategy.md]

### SPEC-REVIEW-07: Merged findings.json exists and parses
- **severity**: quality
- **verify_type**: automated
- **verification**:
  1. Read `belt-agent status` and locate the artifact `findings` resolved_path
  2. Verify the file exists, parses as JSON, and contains a `findings` array
- **pass_condition**: File exists AND valid JSON AND `findings` array present
- **fail_diagnosis_hint**: The `/belt:spec-review` merge step was interrupted — re-invoke from the spec-review phase
- **depends_on_artifacts**: [findings]

### SPEC-REVIEW-08: Internal markdown links still resolve
- **severity**: quality
- **verify_type**: inspection
- **verification**:
  1. Extract internal links (`[text](./path)` / `[text](#anchor)`) from the updated deliverables
  2. Verify each target file/heading exists; list broken links
- **pass_condition**: Step 2 list is empty
- **fail_diagnosis_hint**: Fix the broken targets (path typo, renamed heading) or update the link
- **depends_on_artifacts**: [docs/features/*/test-strategy.md]

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

`design/criteria/plan.md`:

```markdown
---
name: plan
max_retries: 3
audit: lite
---

## Criteria

### PLAN-01: Plan document exists and is committed
- **severity**: blocker
- **verify_type**: automated
- **verification**:
  1. Search with `Glob("docs/features/*/plan.md")`
  2. Run `git status --porcelain -- docs/features/*/plan.md` and confirm zero output lines
- **pass_condition**: At least one Glob match AND zero uncommitted changes
- **fail_diagnosis_hint**: Confirm `/writing-plans` honored the writing-plans supplement's path override
- **depends_on_artifacts**: [docs/features/*/plan.md]

### PLAN-02: Required plan header is present
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read the plan document
  2. Verify it contains `Goal`, `Architecture`, `Tech Stack`, and at least one `Task N` section
- **pass_condition**: All four elements present
- **fail_diagnosis_hint**: The writing-plans header template was skipped — regenerate the header
- **depends_on_artifacts**: [docs/features/*/plan.md]

### PLAN-03: Every task follows the TDD shape
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. For each `Task N`, verify the step sequence covers: failing test -> minimal implementation -> passing test -> commit
  2. Verify code steps contain explicit code blocks and run steps contain explicit commands
- **pass_condition**: Zero tasks missing the TDD sequence or explicit code/commands
- **fail_diagnosis_hint**: Identify non-conforming tasks and expand them per the writing-plans skill's task structure
- **depends_on_artifacts**: [docs/features/*/plan.md]

### PLAN-04: Must-Verify Checklist items are cited by tasks
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate item IDs from the design document's `Must-Verify Checklist` (e.g. `MV-01`)
  2. Verify each ID is cited by at least one task
  3. List uncited IDs
- **pass_condition**: Step 3 list is empty
- **fail_diagnosis_hint**: Map each uncited item to an existing task or add a covering task
- **depends_on_artifacts**: [docs/features/*/design.md, docs/features/*/plan.md]

### PLAN-05: No placeholder language remains
- **severity**: blocker
- **verify_type**: automated + inspection
- **verification**:
  1. `Grep` the plan for placeholder patterns: `TBD`, `TODO`, `add appropriate error handling`, `similar to Task`
  2. Verify no referenced type/function is undefined across all tasks
- **pass_condition**: Zero placeholder matches AND zero unresolved references
- **fail_diagnosis_hint**: Expand each placeholder into concrete content; the executor cannot fill gaps
- **depends_on_artifacts**: [docs/features/*/plan.md]

### PLAN-06: Scenario IDs are cited when --e2e
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. If `args.e2e=false`, PASS (vacuously satisfied)
  2. Enumerate every `id` in `scenarios.yml` and verify each is cited by at least one task
- **pass_condition**: Non-e2e run, OR zero uncited scenario ids
- **fail_diagnosis_hint**: Add citations mapping scenarios to the tasks that implement their behavior
- **depends_on_artifacts**: [docs/features/*/scenarios.yml, docs/features/*/plan.md]

### PLAN-07: Input parameters have four-class Given/When/Then coverage
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Enumerate input parameters surfaced in `test-strategy.md`
  2. For each, verify Given/When/Then coverage exists for: normal, boundary, abnormal, state-transition
- **pass_condition**: Zero parameters missing any of the four classes
- **fail_diagnosis_hint**: Derive the missing cases from the strategy's test design techniques section
- **depends_on_artifacts**: [docs/features/*/test-strategy.md, docs/features/*/plan.md]

### PLAN-08: Narrative note captures plan decomposition
- **severity**: blocker
- **verify_type**: inspection
- **verification**:
  1. Read `belt-agent status` and locate the artifact `plan_notes` resolved_path
  2. Verify the file exists at the resolved_path
  3. Verify frontmatter contains `phase: plan` and `run_id: <run_id>`
  4. Verify 4 required sections exist: `## Decisions`, `## Concerns`, `## Directives`, `## Observations`
  5. Verify Decisions records task decomposition rationale and granularity choices
  6. Verify Directives records constraints for the execute phase (e.g. commit granularity rules, test-first enforcement)
- **pass_condition**: Steps 1-6 all pass
- **fail_diagnosis_hint**: See `plugins/belt-agent/references/narrative-convention.md` for schema
- **depends_on_artifacts**: [plan_notes]

## Observation Collection

The belt-agent:phase-auditor MUST include `observations[]` in its verdict output.
Record quality/warning-level findings even for criteria that PASS.
Observations accumulate in the pipeline's audit trail.
```

- [ ] **Step 3: references を git mv して序数を除去する**

```bash
git mv plugins/belt/skills/feature-dev/references/brainstorming-supplement.md plugins/belt/skills/design/references/brainstorming-supplement.md
git mv plugins/belt/skills/feature-dev/references/writing-plans-supplement.md plugins/belt/skills/design/references/writing-plans-supplement.md
git mv plugins/belt/skills/feature-dev/references/path-convention.md plugins/belt/skills/design/references/path-convention.md
```

`design/references/brainstorming-supplement.md` の序数除去 (7 箇所、exact match):

| old | new |
|---|---|
| `  feature-dev Phase 1 only. Read BEFORE invoking superpowers:brainstorming to` | `  design stage, design phase only. Read BEFORE invoking superpowers:brainstorming to` |
| `feature-dev Phase 1. Once loaded, the constraints below override/augment the` | `the design phase. Once loaded, the constraints below override/augment the` |
| `Quality bar (applied in Phase 3 when expanding to Given/When/Then):` | `Quality bar (applied in test-scenarios when expanding to Given/When/Then):` |
| `Cases failing to meet this bar will be rejected by Phase 2 (test-scenarios)` | `Cases failing to meet this bar will be rejected by test-scenarios` |
| `and Phase 5 (code-review) review.` | `and code-review review.` |
| `   (base = current branch at Phase 1 start).` | `   (base = current branch at design phase start).` |
| `## Completion Criteria (for Phase 1 gate)` | `## Completion Criteria (for the design gate)` |

`design/references/writing-plans-supplement.md` の序数除去 (3 箇所):

| old | new |
|---|---|
| `  feature-dev Phase 3 only. Read BEFORE invoking superpowers:writing-plans to` | `  design stage, plan phase only. Read BEFORE invoking superpowers:writing-plans to` |
| ``Read BEFORE invoking `/writing-plans` in Phase 3. Path convention reference:`` | ``Read BEFORE invoking `/writing-plans` in the plan phase. Path convention reference:`` |
| `## Completion Criteria (for Phase 3 gate)` | `## Completion Criteria (for the plan gate)` |

編集後の確認: `grep -c "Phase [0-9]" plugins/belt/skills/design/references/*.md` → 全て `0` (path-convention.md は Step 4 で全面置換)

- [ ] **Step 4: path-convention.md を bug-fix 統合版に全面置換する**

`plugins/belt/skills/design/references/path-convention.md`:

```markdown
---
name: path-convention
description: >-
  Single source of truth for the docs/features/<YYYY-MM-DD-topic>/ directory
  naming and file layout used by all belt stage pipelines (design / diagnose /
  build / verify) and their composed entry points (feature-dev / bug-fix).
---

# Path Convention for belt Stage Artifacts

All belt run artifact files — feature runs and bug runs alike — live under
`docs/features/<YYYY-MM-DD-topic>/`. Every supplement references this
document for naming rules. The former bug-fix convention
(`docs/plans/YYYY-MM-DD-<topic>-*` flat files) is retired.

## Directory Name

`docs/features/<YYYY-MM-DD-topic>/`

- `<YYYY-MM-DD>`: the date the run's first phase (design for feature runs,
  rca for bug runs) is first invoked, in UTC (ISO 8601).
- `<topic>`: a kebab-case slug (lowercase letters, digits, hyphens; no spaces,
  no underscores). Chosen interactively with the user during the first phase.

Examples:
- `docs/features/2026-04-14-user-authentication/`
- `docs/features/2026-05-01-payment-refactor/`

## Topic Slug Rules

- Only `[a-z0-9-]`, no leading/trailing hyphens, no consecutive hyphens.
- Minimum 3 characters, maximum 48 characters.
- Must not collide with an existing directory under `docs/features/`.
- Must be stable for the duration of the run (do not rename mid-flight).

If a collision is detected, the first phase's supplement appends `-N`
(e.g. `-2`) until unique.

## Worktree Branch Correspondence

The worktree branch created in the first phase must match:

- Feature runs: `feature/<YYYY-MM-DD-topic>`
- Bug runs: `bugfix/<YYYY-MM-DD-topic>`

Example: directory `docs/features/2026-04-14-user-authentication/` maps to
branch `feature/2026-04-14-user-authentication`.

## File Layout per Topic

| File | Producing phase | Producer | When |
|------|-----------------|----------|------|
| `design.md` | design | /brainstorming (+ brainstorming-supplement) | feature runs |
| `test-strategy.md` | test-scenarios | /belt:test-scenarios | feature runs |
| `scenarios.yml` | test-scenarios | /belt:test-scenarios | feature runs, when `args.e2e` |
| `plan.md` | plan | /writing-plans (+ writing-plans-supplement) | feature runs |
| `rca-report.md` | rca | /systematic-debugging (+ rca-supplement) | bug runs |
| `rca-scenarios.yml` | rca | /systematic-debugging (+ rca-supplement) | bug runs, when `args.e2e` |
| `fix-plan.md` | fix-plan | /writing-plans (+ fix-plan-supplement) | bug runs |
| `monkey-test-report.md` | monkey-test | /belt:monkey-test | when `args.e2e` |
| `monkey-test-results.json` | monkey-test | /belt:monkey-test | when `args.e2e` |
| `monkey-test-screenshots/*` | monkey-test | /belt:monkey-test | when `args.e2e` |
| `dogfood-report/report.md` | dogfood | /dogfood (+ dogfood-supplement) | when `args.e2e` |
| `dogfood-report/screenshots/*` | dogfood | /dogfood | when `args.e2e` |
| `dogfood-report/videos/*` | dogfood | /dogfood | when `args.e2e` |

The execute and code-review phases write to git history and
`belt://current/review/findings.json` (resolve via `belt-agent status` or
`belt-agent locate belt://current/review/findings.json`), not under
`docs/features/`.

The integrate phase consumes from `docs/features/<topic>/` but does not
write there.

## Glob Resolution

belt-agent resolves `docs/features/*/<name>` glob patterns with the
phase-start mtime filter; on ambiguity (multiple matching topics), the most
recently modified file wins (mtime DESC).

## Editing Rules

- Phases generate these files; do not hand-edit.
- Hand-edits break belt's phase-start mtime filter (BELT-32 DD-1) used for
  artifact glob resolution.
- If a correction is needed, re-run the owning phase (verify -> regate -> step).
```

- [ ] **Step 5: SKILL.md を書く**

`plugins/belt/skills/design/SKILL.md`:

```markdown
---
name: design
description: >-
  Runs the feature design stage: brainstormed design document, test strategy,
  spec review, and implementation plan. Use standalone for design-only work,
  or composed as the upstream stage of /belt:feature-dev. --e2e also authors
  agent-browser scenarios; --codex enables adversarial spec review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# design

Belt pipeline for the feature design stage. Pipeline structure, args, phase
order, and invocation mapping are defined in `pipeline.yml` and surfaced
dynamically by `belt-agent next` / `belt-agent status`. This SKILL.md covers
the supplement loading contract, phase-specific runtime notes, and red flags.

## Supplement Loading

Before invoking a phase's skill, read the referenced supplement to inject
phase-specific overrides. Phases not listed (`test-scenarios` / `spec-review`)
have no supplement; invoke their declared skill directly.

| Phase | Supplement | Purpose |
|---|---|---|
| design | `./references/brainstorming-supplement.md` | parallel exploration (code-explorer / code-architect / impact-analyzer), implicit-rules extraction, required design sections, worktree creation order |
| plan | `./references/writing-plans-supplement.md` | path override, Must-Verify, scenarios cross-referencing |

## Phase-specific Runtime Notes

- **spec-review**: grill-me dialogue for `requirements` / `design-judgment`
  findings; direct selection triage for the remaining observations.

## Narrative Notes

`design` and `plan` produce a narrative note so context can be restored after
`/clear`. Note paths are declared in `pipeline.yml` as
`belt://current/notes/phase-<id>.md` URIs; resolve the physical path via
`belt-agent status` (read `phases[].produces[].resolved_path`) or
`belt-agent locate belt://current/notes/phase-<id>.md`.

Each note contains four sections (`## Decisions` / `## Concerns` /
`## Directives` / `## Observations`) and minimal frontmatter (`phase`,
`run_id`).

Full convention: [`plugins/belt-agent/references/narrative-convention.md`](plugins/belt-agent/references/narrative-convention.md)

## Red Flags

- **Never skip supplement loading when listed above**: phase-specific overrides are lost and behavior drifts.
- **Never modify the consumed global skills**: all overrides go through `references/*-supplement.md`.
- **Never hand-edit files under `docs/features/<topic>/`**: they are phase-produced; manual edits break belt's phase-start mtime filter.
- **Never leave the narrative note's four sections blank**: use `(none)` placeholders and keep headings.

## References

- `./references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT, shared by all stages)
- `./references/brainstorming-supplement.md` — design phase overrides
- `./references/writing-plans-supplement.md` — plan phase overrides
- `plugins/belt-agent/references/narrative-convention.md` — narrative note schema
```

- [ ] **Step 6: 検証する**

Run: `cargo run -p belt -- lint plugins/belt/skills/design/pipeline.yml && echo LINT-OK`
Expected: `LINT-OK`

Run: `grep -rc "# Phase " plugins/belt/skills/design/criteria/ | grep -v ":0"`
Expected: 出力なし (序数見出しゼロ)

Run: `cargo test -p belt-core`
Expected: 全 PASS (feature_dev_refresh は criteria 存在を検査しないため green 維持)

- [ ] **Step 7: Commit**

```bash
git add plugins/belt/skills/design/ plugins/belt/skills/feature-dev/
git commit -m "feat(plugins): add design stage pipeline with structured criteria"
```

---

### Task 4: diagnose stage skill 新設 (docs/features path 統一 + audit lite 化)

**Files:**
- Create: `plugins/belt/skills/diagnose/pipeline.yml`
- Create: `plugins/belt/skills/diagnose/belt.toml`
- Create: `plugins/belt/skills/diagnose/SKILL.md`
- Create (copy + 編集): `plugins/belt/skills/diagnose/criteria/{rca,fix-plan,fix-plan-review}.md` ← `plugins/belt/skills/bug-fix/criteria/` (原本は Task 6 で削除。Spec Gap Note 2 のとおり `bug_fix_refresh.rs` の存在 lock があるため git mv 不可)
- Create (copy + 編集): `plugins/belt/skills/diagnose/references/{rca-supplement,fix-plan-supplement}.md` ← `plugins/belt/skills/bug-fix/references/`

**Interfaces:**
- Consumes: なし (独立)
- Produces: `../diagnose/pipeline.yml` — Task 6 の合成が参照。phase ids `rca` / `fix-plan` / `fix-plan-review`。args `{e2e, codex}`。regate なし (現行 bug-fix と同じ)
- Produces: diagnose criteria は `audit: lite` (Spec Gap Note 5)、artifact path は `docs/features/*/` 統一

- [ ] **Step 1: pipeline.yml と belt.toml を書く**

`plugins/belt/skills/diagnose/pipeline.yml` (現行 bug-fix の 1–3 phase を docs/features path に更新して移植):

```yaml
name: diagnose
version: 1
description: "Bug diagnosis stage (rca -> fix-plan -> fix-plan-review)"

args:
  e2e:
    type: bool
    default: false
    description: "Author reproduction scenarios (rca-scenarios.yml) for monkey-test replay"
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
        description: "Reproduction scenarios in Given/When/Then YAML for monkey-test replay"
        when: "args.e2e"
      - name: rca_notes
        path: "belt://current/notes/phase-rca.md"
        description: "Phase narrative: decisions, concerns, directives, observations"
    gate:
      - file_exists: "docs/features/*/rca-report.md"
      - file_exists: "belt://current/notes/phase-rca.md"
    validate: ./criteria/rca.md
    confirm: true
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
    confirm: true
    max_retries: 3

  - id: fix-plan-review
    description: "Plan review via spec-review"
    invoke:
      skill: /belt:spec-review
      args:
        codex: "args.codex"
    consumes:
      - fix_plan_doc
    produces:
      - name: findings-feasibility
        path: "belt://current/review/findings-feasibility.json"
        description: "Feasibility observation findings"
      - name: findings-cross-cutting-spec
        path: "belt://current/review/findings-cross-cutting-spec.json"
        description: "Cross-cutting spec observation findings"
      - name: findings-ui-design
        path: "belt://current/review/findings-ui-design.json"
        description: "UI design observation findings"
      - name: findings-codex
        path: "belt://current/review/findings-codex.json"
        description: "Codex adversarial spec review findings"
        when: "args.codex"
      - name: findings
        path: "belt://current/review/findings.json"
        description: "Merged spec-review findings"
    gate:
      - file_exists: "belt://current/review/findings.json"
    validate: ./criteria/fix-plan-review.md
    confirm: true
    max_retries: 3
```

`plugins/belt/skills/diagnose/belt.toml`:

```toml
pipeline = "./pipeline.yml"
```

- [ ] **Step 2: criteria 3 本を copy して編集する**

```bash
mkdir -p plugins/belt/skills/diagnose/criteria plugins/belt/skills/diagnose/references
cp plugins/belt/skills/bug-fix/criteria/rca.md plugins/belt/skills/diagnose/criteria/rca.md
cp plugins/belt/skills/bug-fix/criteria/fix-plan.md plugins/belt/skills/diagnose/criteria/fix-plan.md
cp plugins/belt/skills/bug-fix/criteria/fix-plan-review.md plugins/belt/skills/diagnose/criteria/fix-plan-review.md
```

3 ファイル共通の編集:
1. frontmatter `audit: required` → `audit: lite`
2. path 置換 (機械的、exact match):

| old | new |
|---|---|
| `docs/plans/*-rca-report.md` | `docs/features/*/rca-report.md` |
| `docs/plans/*-rca-scenarios.yml` | `docs/features/*/rca-scenarios.yml` |
| `docs/plans/*-fix-plan.md` | `docs/features/*/fix-plan.md` |
| `docs/plans/YYYY-MM-DD-*-fix-plan.md` | `docs/features/*/fix-plan.md` |
| `docs/plans/` (上記に該当しない残り: RCA-01 hint の `under docs/plans/`、RCA-01/FIX-PLAN-01 depends の `[docs/plans/]`、FIX-PLAN-01 hint、FIX-PLAN-REVIEW-02 hint の `git log --oneline -- docs/plans/`) | `docs/features/` |

個別の編集 (`diagnose/criteria/rca.md`):

| old | new |
|---|---|
| `  1. Read `args.e2e` from `belt-agent status --run-id <id>` JSON output` | `  1. Read `args.e2e` from `belt-agent status` JSON output` |
| `- **fail_diagnosis_hint**: If `--e2e=true` and file is missing, the RCA executor did not load `rca-supplement.md`. Confirm supplement injection in SKILL.md Phase 1 invocation` | `- **fail_diagnosis_hint**: If `--e2e=true` and file is missing, the RCA executor did not load `rca-supplement.md`. Confirm supplement injection in the diagnose SKILL.md rca invocation` |

確認: `grep -c "docs/plans" plugins/belt/skills/diagnose/criteria/*.md` → 全て `0`、`grep -c "audit: lite" plugins/belt/skills/diagnose/criteria/*.md` → 全て `1`

- [ ] **Step 3: references 2 本を copy して編集する**

```bash
cp plugins/belt/skills/bug-fix/references/rca-supplement.md plugins/belt/skills/diagnose/references/rca-supplement.md
cp plugins/belt/skills/bug-fix/references/fix-plan-supplement.md plugins/belt/skills/diagnose/references/fix-plan-supplement.md
```

`diagnose/references/rca-supplement.md` の編集 (exact match):

| old | new |
|---|---|
| `# RCA Supplement (Phase 1 override for `/systematic-debugging`)` | `# RCA Supplement (rca phase override for `/systematic-debugging`)` |
| `**Invoked by:** `SKILL.md` Phase 1 (INVOKE 1 = Read this file; INVOKE 2 = `/systematic-debugging`).` | `**Invoked by:** diagnose `SKILL.md`, rca phase (INVOKE 1 = Read this file; INVOKE 2 = `/systematic-debugging`).` |
| `docs/plans/YYYY-MM-DD-<topic>-rca-report.md` | `docs/features/<YYYY-MM-DD-topic>/rca-report.md` |
| `Path convention: see `./path-convention.md`.` | `Path convention: see `plugins/belt/skills/design/references/path-convention.md`.` |
| `docs/plans/YYYY-MM-DD-<topic>-rca-scenarios.yml` | `docs/features/<YYYY-MM-DD-topic>/rca-scenarios.yml` |
| `(see `monkey-test-supplement.md`)` | `(see `plugins/belt/skills/verify/references/monkey-test-supplement.md`)` |
| `See bug-fix SKILL.md Red Flag "Never delegate root cause synthesis to subagents."` | `See diagnose SKILL.md Red Flag "Never delegate root cause synthesis to subagents."` |

`diagnose/references/fix-plan-supplement.md` の編集 (exact match):

| old | new |
|---|---|
| `# Fix Plan Supplement (Phase 2 override for `/writing-plans`)` | `# Fix Plan Supplement (fix-plan phase override for `/writing-plans`)` |
| `**Invoked by:** `SKILL.md` Phase 2 (INVOKE 1 = Read this file; INVOKE 2 = `/writing-plans`).` | `**Invoked by:** diagnose `SKILL.md`, fix-plan phase (INVOKE 1 = Read this file; INVOKE 2 = `/writing-plans`).` |
| `docs/plans/YYYY-MM-DD-<topic>-fix-plan.md` | `docs/features/<YYYY-MM-DD-topic>/fix-plan.md` |
| `Path convention: see `./path-convention.md`.` | `Path convention: see `plugins/belt/skills/design/references/path-convention.md`.` |
| `- `docs/plans/YYYY-MM-DD-<topic>-rca-report.md` (produced by Phase 1)` | `- `docs/features/<YYYY-MM-DD-topic>/rca-report.md` (produced by the rca phase)` |
| `- Include a reference line to the consumed RCA Report at the top of the fix plan (e.g., "Based on: `docs/plans/YYYY-MM-DD-<topic>-rca-report.md`")` | `- Include a reference line to the consumed RCA Report at the top of the fix plan (e.g., "Based on: `docs/features/<YYYY-MM-DD-topic>/rca-report.md`")` |

確認: `grep -c "docs/plans\|Phase [0-9]" plugins/belt/skills/diagnose/references/*.md` → 全て `0`

- [ ] **Step 4: SKILL.md を書く**

`plugins/belt/skills/diagnose/SKILL.md`:

```markdown
---
name: diagnose
description: >-
  Runs the bug diagnosis stage: root-cause analysis with a failing
  reproduction test, fix planning, and adversarial plan review. Use standalone
  for diagnosis-only work, or composed as the upstream stage of
  /belt:bug-fix. --e2e also authors reproduction scenarios; --codex enables
  adversarial plan review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# diagnose

Belt pipeline for the bug diagnosis stage. Pipeline structure, args, phase
order, and invocation mapping are defined in `pipeline.yml` and surfaced
dynamically by `belt-agent next` / `belt-agent status`. This SKILL.md covers
the supplement loading contract, phase-specific runtime notes, and red flags.

Artifacts follow the unified `docs/features/<YYYY-MM-DD-topic>/` layout with
branch `bugfix/<YYYY-MM-DD-topic>` — see
`plugins/belt/skills/design/references/path-convention.md` (SSOT).

## Supplement Loading

Before invoking a phase's skill, read the referenced supplement to inject
phase-specific overrides. `fix-plan-review` has no supplement; invoke its
declared skill directly.

| Phase | Supplement | Purpose |
|---|---|---|
| rca | `./references/rca-supplement.md` | RCA Report 5 sections, Symmetry Check, Reproduction Test FAIL, parallel exploration order, `rca-scenarios.yml` produce (when `--e2e`) |
| fix-plan | `./references/fix-plan-supplement.md` | RCA Fix Strategy -> task traceability, Given/When/Then test cases, verifiable completion conditions, task granularity |

## Phase-specific Runtime Notes

- **fix-plan-review**: `/belt:spec-review` is reused for fix-plan review. The
  grill-me prompt under the `design-judgment` observation does not fire by
  default (design decisions are already settled in rca / fix-plan). If it
  fires, treat it as a signal that upstream phases need to be revisited.

## Narrative Notes

`rca` and `fix-plan` produce a narrative note so context can be restored
after `/clear`. Note paths are declared in `pipeline.yml` as
`belt://current/notes/phase-<id>.md` URIs; resolve the physical path via
`belt-agent status` (read `phases[].produces[].resolved_path`) or
`belt-agent locate belt://current/notes/phase-<id>.md`.

Each note contains four sections (`## Decisions` / `## Concerns` /
`## Directives` / `## Observations`) and minimal frontmatter (`phase`,
`run_id`).

Full convention: [`plugins/belt-agent/references/narrative-convention.md`](plugins/belt-agent/references/narrative-convention.md)

## Red Flags

- **Never skip rca**: root cause must precede fix. "Fix first" is the anti-pattern.
- **Never proceed without a failing reproduction test**: RCA blocker.
- **Never delegate root cause synthesis to subagents**: the orchestrator must reconstruct parallel exploration results.
- **Never skip supplement loading when listed above.**
- **Never hand-edit files under `docs/features/<topic>/`**: they are phase-produced; manual edits break belt's phase-start mtime filter.
- **Never leave the narrative note's four sections blank**: use `(none)` placeholders and keep headings.

## References

- `./references/rca-supplement.md` — rca phase override
- `./references/fix-plan-supplement.md` — fix-plan phase override
- `plugins/belt/skills/design/references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
- `plugins/belt-agent/references/narrative-convention.md` — narrative note schema
```

- [ ] **Step 5: 検証する**

Run: `cargo run -p belt -- lint plugins/belt/skills/diagnose/pipeline.yml && echo LINT-OK`
Expected: `LINT-OK`

Run: `cargo test -p belt-core --test bug_fix_refresh`
Expected: 全 PASS (bug-fix 側は copy 方式のため一切無傷)

- [ ] **Step 6: Commit**

```bash
git add plugins/belt/skills/diagnose/
git commit -m "feat(plugins): add diagnose stage pipeline on the unified docs/features path"
```

---

### Task 5: feature-dev 合成 cutover

**Files:**
- Modify (全面書換): `plugins/belt/skills/feature-dev/pipeline.yml`
- Modify (全面書換): `plugins/belt/skills/feature-dev/SKILL.md`
- Delete: `plugins/belt/skills/feature-dev/criteria/{monkey-test,dogfood,integrate}.md` (残存 3 本 — verify/build へ統合済み)
- Delete: `plugins/belt/skills/feature-dev/references/{monkey-test-supplement,dogfood-supplement}.md` (verify へ統合済み)
- Modify (全面書換): `crates/belt-core/tests/feature_dev_refresh.rs`
- Modify: `crates/belt-agent/tests/cli_test.rs` (`feature_dev_migrated_pipeline_boots` の期待値)
- Modify: `docs/testing/lock-ledger.md` (`## feature_dev_refresh.rs` 節を全面書換)
- 無変更: `plugins/belt/skills/feature-dev/belt.toml`

**Interfaces:**
- Consumes: Tasks 1–3 の `../design/pipeline.yml` / `../build/pipeline.yml` (+ 既存 `../handover/checkpoint.yml`)
- Produces: 合成 feature-dev — 展開 10 leaves (File Structure 節の展開形)。`belt lint plugins/belt/skills/feature-dev/pipeline.yml` がこの Task で green に復帰する

- [ ] **Step 1: pipeline.yml を合成形に全面置換する**

`plugins/belt/skills/feature-dev/pipeline.yml`:

```yaml
name: feature-dev
version: 1
description: "Quality-gated development pipeline (composed: design -> checkpoint -> build)"

args:
  e2e:
    type: bool
    default: false
    description: "Enable E2E verification (monkey-test, dogfood)"
  codex:
    type: bool
    default: false
    description: "Enable Codex parallel review in spec-review and code-review"

phases:
  - id: design
    invoke:
      pipeline: ../design/pipeline.yml
      with:
        e2e: "args.e2e"
        codex: "args.codex"

  - id: pre-execute-handover
    invoke:
      pipeline: ../handover/checkpoint.yml

  - id: build
    invoke:
      pipeline: ../build/pipeline.yml
      with:
        e2e: "args.e2e"
        codex: "args.codex"
```

- [ ] **Step 2: SKILL.md を stage-map 形に全面置換する**

`plugins/belt/skills/feature-dev/SKILL.md`:

```markdown
---
name: feature-dev
description: >-
  Runs a quality-gated feature-development pipeline spanning design, test
  strategy, spec review, implementation, and regression verification. Use when
  building a new feature that needs a structured design-to-integration flow.
  --e2e enables browser-based verification; --codex enables adversarial review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# feature-dev

Composed belt pipeline: the design stage, a context-reset checkpoint, and the
shared build stage. `pipeline.yml` declares three `invoke.pipeline`
references; `belt-agent init` expands them inline, so `next` returns
namespaced leaf phases (`design/design`, `build/execute`,
`build/verify/monkey-test`, ...) in a single run — status, resume, and
narrative notes work exactly as in a flat pipeline.

## Stage Skills

When `next` returns a phase, load the owning stage's SKILL.md before
executing it — the supplement loading contracts, entry checks, and red flags
live there:

| Phase id prefix | Stage skill |
|---|---|
| `design/` | `plugins/belt/skills/design/SKILL.md` |
| `pre-execute-handover/` | (none — follow the phase description: `/belt:handover`, `/clear`, `/belt:resume`) |
| `build/verify/` | `plugins/belt/skills/verify/SKILL.md` |
| `build/` (other) | `plugins/belt/skills/build/SKILL.md` |

Smaller runs are available directly: `/belt:design` for design-only work,
`/belt:build` when a plan already exists, `/belt:verify` for browser
verification alone.

## Red Flags

- **Never execute a stage phase without loading its stage SKILL.md**: supplement contracts are defined per stage, not here.
- **Never bypass the pre-execute-handover checkpoint**: the context reset before execute is the pipeline's core ergonomics.

## References

- `plugins/belt/skills/design/SKILL.md` — design stage contract
- `plugins/belt/skills/build/SKILL.md` — build stage contract
- `plugins/belt/skills/verify/SKILL.md` — verify stage contract (when `--e2e`)
- `plugins/belt/skills/design/references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
```

- [ ] **Step 3: 残存する旧 criteria / references を削除する**

```bash
git rm plugins/belt/skills/feature-dev/criteria/monkey-test.md \
       plugins/belt/skills/feature-dev/criteria/dogfood.md \
       plugins/belt/skills/feature-dev/criteria/integrate.md \
       plugins/belt/skills/feature-dev/references/monkey-test-supplement.md \
       plugins/belt/skills/feature-dev/references/dogfood-supplement.md
```

feature-dev 配下に残るのは `pipeline.yml` / `SKILL.md` / `belt.toml` のみ (criteria/ と references/ は空ディレクトリとして消滅)。

- [ ] **Step 4: feature_dev_refresh.rs を合成 shape lock に全面置換する**

`crates/belt-core/tests/feature_dev_refresh.rs`:

```rust
//! Integration tests for the composed feature-dev pipeline (2026-07-02
//! pipeline split): design(sub) + pre-execute-handover(sub) + build(sub).
//!
//! Shape contract (spec docs/specs/2026-07-02-pipeline-split-design.md):
//! - args = { e2e: bool, codex: bool } only
//! - 3 top-level phases, all Invoker::Pipeline delegations:
//!   design -> ../design/pipeline.yml,
//!   pre-execute-handover -> ../handover/checkpoint.yml,
//!   build -> ../build/pipeline.yml
//! - design/build receive with = { e2e: "args.e2e", codex: "args.codex" }
//!   (bare full-string form — the only form the expander substitutes)
//! - expansion flattens to exactly 10 namespaced leaves
//! - stage-internal regate expands namespaced (never crosses a stage file)
//! - verify leaves inherit when: "args.e2e" from build's verify phase
//!
//! Stage-internal shape (phase order, narrative notes, criteria files) is
//! locked per stage in `pipeline_split_refresh.rs`.

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

const EXPECTED_LEAVES: &[&str] = &[
    "design/design",
    "design/test-scenarios",
    "design/spec-review",
    "design/plan",
    "pre-execute-handover/checkpoint",
    "build/execute",
    "build/code-review",
    "build/verify/monkey-test",
    "build/verify/dogfood",
    "build/integrate",
];

#[test]
fn feature_dev_composes_three_stages() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;
    let got: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        got,
        vec!["design", "pre-execute-handover", "build"],
        "top-level composition must be design -> checkpoint -> build"
    );
    Ok(())
}

#[test]
fn stages_delegate_with_e2e_and_codex_passthrough() {
    let pipeline =
        parse_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must parse");
    for (phase_id, expected_sub) in [
        ("design", "../design/pipeline.yml"),
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
        let mut keys: Vec<&str> = with.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec!["codex", "e2e"],
            "phase '{phase_id}' must pass exactly {{codex, e2e}}"
        );
        assert_eq!(
            with.get("e2e").and_then(|v| v.as_str()),
            Some("args.e2e"),
            "phase '{phase_id}' e2e must be the bare full-string form"
        );
        assert_eq!(
            with.get("codex").and_then(|v| v.as_str()),
            Some("args.codex"),
            "phase '{phase_id}' codex must be the bare full-string form"
        );
    }
}

#[test]
fn checkpoint_delegates_with_no_args() {
    let pipeline =
        parse_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must parse");
    let phase = pipeline
        .phases
        .iter()
        .find(|p| p.id == "pre-execute-handover")
        .expect("pre-execute-handover phase must exist");
    match phase.invoke.as_ref() {
        Some(Invoker::Pipeline {
            pipeline: sub_path,
            with,
        }) => {
            assert_eq!(
                sub_path, "../handover/checkpoint.yml",
                "pre-execute-handover must delegate to ../handover/checkpoint.yml"
            );
            assert!(
                with.is_empty(),
                "pre-execute-handover delegation must not pass any `with` args"
            );
        }
        other => panic!("pre-execute-handover must use Invoker::Pipeline, got {other:?}"),
    }
}

#[test]
fn top_level_args_are_e2e_and_codex_only() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;

    let mut names: Vec<&str> = pipeline.args.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec!["codex", "e2e"],
        "args must be exactly {{codex, e2e}}"
    );

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
fn feature_dev_expands_to_ten_namespaced_leaves() {
    let expanded =
        expand_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must expand");
    let ids: Vec<&str> = expanded.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, EXPECTED_LEAVES, "expanded leaf ids + order must match");
}

#[test]
fn expanded_regate_targets_are_namespaced() {
    let expanded =
        expand_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must expand");
    let spec_review = expanded
        .iter()
        .find(|p| p.id == "design/spec-review")
        .expect("design/spec-review leaf must exist");
    assert_eq!(
        spec_review.regate,
        vec!["design/test-scenarios".to_string()],
        "stage-internal regate must expand into the stage namespace"
    );
    let code_review = expanded
        .iter()
        .find(|p| p.id == "build/code-review")
        .expect("build/code-review leaf must exist");
    assert_eq!(
        code_review.regate,
        vec!["build/execute".to_string()],
        "stage-internal regate must expand into the stage namespace"
    );
}

#[test]
fn expanded_verify_leaves_inherit_e2e_when() {
    let expanded =
        expand_pipeline(&feature_dev_pipeline_path()).expect("feature-dev pipeline must expand");
    for id in ["build/verify/monkey-test", "build/verify/dogfood"] {
        let leaf = expanded
            .iter()
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("leaf '{id}' must exist"));
        assert_eq!(
            leaf.when.as_deref(),
            Some("args.e2e"),
            "leaf '{id}' must inherit when: args.e2e from build's verify phase"
        );
    }
    let execute = expanded
        .iter()
        .find(|p| p.id == "build/execute")
        .expect("build/execute leaf must exist");
    assert_eq!(execute.when, None, "build/execute must not inherit a when");
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

- [ ] **Step 5: cli_test.rs の boot テストを合成後の期待値に更新する**

`crates/belt-agent/tests/cli_test.rs` の `feature_dev_migrated_pipeline_boots` 内、以下 2 箇所を編集:

old:
```rust
    assert_eq!(
        next_json["phase"]["id"].as_str(),
        Some("design"),
        "first phase should be 'design'"
    );
```

new:
```rust
    assert_eq!(
        next_json["phase"]["id"].as_str(),
        Some("design/design"),
        "first phase should be the expanded leaf 'design/design'"
    );
```

doc コメントの `it only drives belt-agent through init → next to prove that` の直後行 `the new-format pipeline boots and surfaces the first phase correctly.` はそのまま。`invoke["skill"]` の assert (`/brainstorming`) は不変 (design/design leaf が引き継ぐ)。

- [ ] **Step 6: lock-ledger.md の feature_dev_refresh 節を書き換える**

`## feature_dev_refresh.rs` 節全体を以下で置換:

```markdown
## feature_dev_refresh.rs

​```yaml
locks-file: crates/belt-core/tests/feature_dev_refresh.rs
pipeline: plugins/belt/skills/feature-dev/pipeline.yml
test-fn-count: 8
cross-coupling:
  - crates/belt-core/tests/bug_fix_refresh.rs
  - crates/belt-core/tests/pipeline_split_refresh.rs
  - crates/belt-core/tests/review_skills_refresh.rs
  - crates/belt-core/tests/shared_filter_parity.rs
​```

**8 test fn 名** (A):

- `feature_dev_composes_three_stages`
- `stages_delegate_with_e2e_and_codex_passthrough`
- `checkpoint_delegates_with_no_args`
- `top_level_args_are_e2e_and_codex_only`
- `feature_dev_expands_to_ten_namespaced_leaves`
- `expanded_regate_targets_are_namespaced`
- `expanded_verify_leaves_inherit_e2e_when`
- `feature_dev_pipeline_has_no_run_id_template`

**pipeline.yml shape dimensions locked** (B):

- `args` set が exactly `{codex, e2e}` + 全 arg が `ArgType::Bool` / default false
- 3 top-level phase の順序 (`design → pre-execute-handover → build`)、全て `Invoker::Pipeline`
- design/build の `with` が exactly `{e2e: "args.e2e", codex: "args.codex"}` (bare full-string form)、checkpoint は `with` 空
- 展開 leaf ids が exactly 10 件 (`design/design → ... → build/integrate`、File Structure 節の展開形)
- stage 内 regate の namespace 展開 (`design/spec-review → [design/test-scenarios]`、`build/code-review → [build/execute]`)
- verify leaves (`build/verify/monkey-test` / `build/verify/dogfood`) が `when: args.e2e` を継承、`build/execute` は when なし
- `.belt/runs/` リテラル + `{run_id}` template の non-existence

**Cross-coupling** (C):

- `pipeline_split_refresh.rs` — stage 内部 shape (phase 順序 / narrative / criteria) は stage 側で lock
- `bug_fix_refresh.rs` — bug-fix 合成 shape (同 tuple pattern で parallel)
- `review_skills_refresh.rs` / `shared_filter_parity.rs` — 従来どおり
```

(注: 上の ` ```yaml ` ブロックはこの plan の入れ子表現のためゼロ幅文字を含めている。実ファイルには通常の fenced code block として書くこと)

- [ ] **Step 7: 検証する**

Run: `cargo fmt --package belt-core && cargo clippy --package belt-core -- -D warnings && cargo test -p belt-core`
Expected: 全 PASS (feature_dev_refresh 8 本が新 shape で green)

Run: `cargo test -p belt-agent --test cli_test feature_dev_migrated_pipeline_boots`
Expected: PASS (`design/design` が先頭 leaf)

Run: `cargo run -p belt -- lint plugins/belt/skills/feature-dev/pipeline.yml && echo LINT-OK`
Expected: `LINT-OK` (Task 2 以降の transient FAIL がここで解消)

- [ ] **Step 8: Commit**

```bash
git add plugins/belt/skills/feature-dev/ crates/belt-core/tests/feature_dev_refresh.rs crates/belt-agent/tests/cli_test.rs docs/testing/lock-ledger.md
git commit -m "feat(plugins): compose feature-dev from design + checkpoint + build"
```

---

### Task 6: bug-fix 合成 cutover

**Files:**
- Modify (全面書換): `plugins/belt/skills/bug-fix/pipeline.yml`
- Modify (全面書換): `plugins/belt/skills/bug-fix/SKILL.md`
- Delete: `plugins/belt/skills/bug-fix/criteria/` 全 8 ファイル (rca, fix-plan, fix-plan-review, execute, code-review, monkey-test, dogfood, integrate)
- Delete: `plugins/belt/skills/bug-fix/references/` 全 7 ファイル (rca-supplement, fix-plan-supplement, monkey-test-supplement, dogfood-supplement, worktrunk-supplement, evidence-catalog, path-convention)
- Modify (全面書換): `crates/belt-core/tests/bug_fix_refresh.rs`
- Modify: `docs/testing/lock-ledger.md` (`## bug_fix_refresh.rs` 節を全面書換)
- 無変更: `plugins/belt/skills/bug-fix/belt.toml`

**Interfaces:**
- Consumes: Tasks 2/4 の `../build/pipeline.yml` / `../diagnose/pipeline.yml`
- Produces: 合成 bug-fix — 展開 9 leaves (File Structure 節の展開形)

- [ ] **Step 1: pipeline.yml を合成形に全面置換する**

`plugins/belt/skills/bug-fix/pipeline.yml`:

```yaml
name: bug-fix
version: 1
description: "Quality-gated debugging pipeline (composed: diagnose -> checkpoint -> build)"

args:
  e2e:
    type: bool
    default: false
    description: "Enable E2E verification (monkey-test, dogfood)"
  codex:
    type: bool
    default: false
    description: "Enable Codex parallel review in fix-plan-review and code-review"

phases:
  - id: diagnose
    invoke:
      pipeline: ../diagnose/pipeline.yml
      with:
        e2e: "args.e2e"
        codex: "args.codex"

  - id: pre-execute-handover
    invoke:
      pipeline: ../handover/checkpoint.yml

  - id: build
    invoke:
      pipeline: ../build/pipeline.yml
      with:
        e2e: "args.e2e"
        codex: "args.codex"
```

- [ ] **Step 2: SKILL.md を stage-map 形に全面置換する**

`plugins/belt/skills/bug-fix/SKILL.md`:

```markdown
---
name: bug-fix
description: >-
  Runs a quality-gated debugging pipeline with root-cause analysis, fix
  planning, code review, and regression verification. Use when a bug needs
  structured diagnosis and verified repair. --e2e adds browser-based
  regression tests; --codex enables adversarial review.
user-invocable: true
argument-hint: "[--e2e] [--codex]"
---

# bug-fix

Composed belt pipeline: the diagnose stage, a context-reset checkpoint, and
the shared build stage. `pipeline.yml` declares three `invoke.pipeline`
references; `belt-agent init` expands them inline, so `next` returns
namespaced leaf phases (`diagnose/rca`, `build/execute`,
`build/verify/monkey-test`, ...) in a single run — status, resume, and
narrative notes work exactly as in a flat pipeline.

## Stage Skills

When `next` returns a phase, load the owning stage's SKILL.md before
executing it — the supplement loading contracts, entry checks, and red flags
live there:

| Phase id prefix | Stage skill |
|---|---|
| `diagnose/` | `plugins/belt/skills/diagnose/SKILL.md` |
| `pre-execute-handover/` | (none — follow the phase description: `/belt:handover`, `/clear`, `/belt:resume`) |
| `build/verify/` | `plugins/belt/skills/verify/SKILL.md` |
| `build/` (other) | `plugins/belt/skills/build/SKILL.md` |

Smaller runs are available directly: `/belt:diagnose` for diagnosis-only
work, `/belt:build` when a fix plan already exists, `/belt:verify` for
browser verification alone.

## Red Flags

- **Never execute a stage phase without loading its stage SKILL.md**: supplement contracts are defined per stage, not here.
- **Never skip diagnose**: root cause must precede fix. "Fix first" is the anti-pattern (enforced by the diagnose stage's own red flags).
- **Never bypass the pre-execute-handover checkpoint**: the context reset before execute is the pipeline's core ergonomics.

## References

- `plugins/belt/skills/diagnose/SKILL.md` — diagnose stage contract
- `plugins/belt/skills/build/SKILL.md` — build stage contract
- `plugins/belt/skills/verify/SKILL.md` — verify stage contract (when `--e2e`)
- `plugins/belt/skills/design/references/path-convention.md` — `docs/features/<YYYY-MM-DD-topic>/` naming rules (SSOT)
```

- [ ] **Step 3: 旧 criteria / references を削除する**

```bash
git rm plugins/belt/skills/bug-fix/criteria/rca.md \
       plugins/belt/skills/bug-fix/criteria/fix-plan.md \
       plugins/belt/skills/bug-fix/criteria/fix-plan-review.md \
       plugins/belt/skills/bug-fix/criteria/execute.md \
       plugins/belt/skills/bug-fix/criteria/code-review.md \
       plugins/belt/skills/bug-fix/criteria/monkey-test.md \
       plugins/belt/skills/bug-fix/criteria/dogfood.md \
       plugins/belt/skills/bug-fix/criteria/integrate.md \
       plugins/belt/skills/bug-fix/references/rca-supplement.md \
       plugins/belt/skills/bug-fix/references/fix-plan-supplement.md \
       plugins/belt/skills/bug-fix/references/monkey-test-supplement.md \
       plugins/belt/skills/bug-fix/references/dogfood-supplement.md \
       plugins/belt/skills/bug-fix/references/worktrunk-supplement.md \
       plugins/belt/skills/bug-fix/references/evidence-catalog.md \
       plugins/belt/skills/bug-fix/references/path-convention.md
```

bug-fix 配下に残るのは `pipeline.yml` / `SKILL.md` / `belt.toml` のみ。

- [ ] **Step 4: bug_fix_refresh.rs を合成 shape lock に全面置換する**

`crates/belt-core/tests/bug_fix_refresh.rs`:

```rust
//! Integration tests for the composed bug-fix pipeline (2026-07-02 pipeline
//! split): diagnose(sub) + pre-execute-handover(sub) + build(sub).
//!
//! Shape contract (spec docs/specs/2026-07-02-pipeline-split-design.md):
//! - args = { e2e: bool, codex: bool } only (legacy args stay removed)
//! - 3 top-level phases, all Invoker::Pipeline delegations:
//!   diagnose -> ../diagnose/pipeline.yml,
//!   pre-execute-handover -> ../handover/checkpoint.yml,
//!   build -> ../build/pipeline.yml
//! - diagnose/build receive with = { e2e: "args.e2e", codex: "args.codex" }
//! - expansion flattens to exactly 9 namespaced leaves
//! - stage-internal regate expands namespaced; verify leaves inherit
//!   when: "args.e2e"
//!
//! Stage-internal shape (phase order, docs/features artifact paths,
//! narrative notes, criteria files) is locked per stage in
//! `pipeline_split_refresh.rs`.

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
    "build/verify/monkey-test",
    "build/verify/dogfood",
    "build/integrate",
];

#[test]
fn bug_fix_composes_three_stages() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&bug_fix_pipeline_path())?;
    let got: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        got,
        vec!["diagnose", "pre-execute-handover", "build"],
        "top-level composition must be diagnose -> checkpoint -> build"
    );
    Ok(())
}

#[test]
fn stages_delegate_with_e2e_and_codex_passthrough() {
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
        let mut keys: Vec<&str> = with.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec!["codex", "e2e"],
            "phase '{phase_id}' must pass exactly {{codex, e2e}}"
        );
        assert_eq!(
            with.get("e2e").and_then(|v| v.as_str()),
            Some("args.e2e"),
            "phase '{phase_id}' e2e must be the bare full-string form"
        );
        assert_eq!(
            with.get("codex").and_then(|v| v.as_str()),
            Some("args.codex"),
            "phase '{phase_id}' codex must be the bare full-string form"
        );
    }
}

#[test]
fn checkpoint_delegates_with_no_args() {
    let pipeline = parse_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must parse");
    let phase = pipeline
        .phases
        .iter()
        .find(|p| p.id == "pre-execute-handover")
        .expect("pre-execute-handover phase must exist");
    match phase.invoke.as_ref() {
        Some(Invoker::Pipeline {
            pipeline: sub_path,
            with,
        }) => {
            assert_eq!(
                sub_path, "../handover/checkpoint.yml",
                "pre-execute-handover must delegate to ../handover/checkpoint.yml"
            );
            assert!(
                with.is_empty(),
                "pre-execute-handover delegation must not pass any `with` args"
            );
        }
        other => panic!("pre-execute-handover must use Invoker::Pipeline, got {other:?}"),
    }
}

#[test]
fn args_are_e2e_and_codex_only() {
    let pipeline = parse_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must parse");
    let mut keys: Vec<&str> = pipeline.args.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["codex", "e2e"]);

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
    for legacy in ["iterations", "swarm", "ui", "smoke"] {
        assert!(
            !pipeline.args.contains_key(legacy),
            "legacy arg '{legacy}' must be removed"
        );
    }
}

#[test]
fn bug_fix_expands_to_nine_namespaced_leaves() {
    let expanded = expand_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must expand");
    let ids: Vec<&str> = expanded.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, EXPECTED_LEAVES, "expanded leaf ids + order must match");
}

#[test]
fn expanded_regate_targets_are_namespaced() {
    let expanded = expand_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must expand");
    let code_review = expanded
        .iter()
        .find(|p| p.id == "build/code-review")
        .expect("build/code-review leaf must exist");
    assert_eq!(
        code_review.regate,
        vec!["build/execute".to_string()],
        "stage-internal regate must expand into the stage namespace"
    );
    // diagnose declares no regate; every other leaf must have none.
    for leaf in &expanded {
        if leaf.id != "build/code-review" {
            assert!(
                leaf.regate.is_empty(),
                "leaf '{}' must have empty regate, got {:?}",
                leaf.id,
                leaf.regate
            );
        }
    }
}

#[test]
fn expanded_verify_leaves_inherit_e2e_when() {
    let expanded = expand_pipeline(&bug_fix_pipeline_path()).expect("bug-fix pipeline must expand");
    for id in ["build/verify/monkey-test", "build/verify/dogfood"] {
        let leaf = expanded
            .iter()
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("leaf '{id}' must exist"));
        assert_eq!(
            leaf.when.as_deref(),
            Some("args.e2e"),
            "leaf '{id}' must inherit when: args.e2e from build's verify phase"
        );
    }
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

- [ ] **Step 5: lock-ledger.md の bug_fix_refresh 節を書き換える**

`## bug_fix_refresh.rs` 節全体を以下で置換 (fenced block の注意は Task 5 Step 6 と同じ):

```markdown
## bug_fix_refresh.rs

​```yaml
locks-file: crates/belt-core/tests/bug_fix_refresh.rs
pipeline: plugins/belt/skills/bug-fix/pipeline.yml
test-fn-count: 9
cross-coupling:
  - crates/belt-core/tests/feature_dev_refresh.rs
  - crates/belt-core/tests/pipeline_split_refresh.rs
​```

**9 test fn 名** (A):

- `bug_fix_composes_three_stages`
- `stages_delegate_with_e2e_and_codex_passthrough`
- `checkpoint_delegates_with_no_args`
- `args_are_e2e_and_codex_only`
- `no_legacy_args`
- `bug_fix_expands_to_nine_namespaced_leaves`
- `expanded_regate_targets_are_namespaced`
- `expanded_verify_leaves_inherit_e2e_when`
- `bug_fix_pipeline_has_no_run_id_template`

**pipeline.yml shape dimensions locked** (B):

- `args` set が exactly `{codex, e2e}` + 全 arg が `ArgType::Bool` / default false、legacy args (`iterations` / `swarm` / `ui` / `smoke`) の non-existence
- 3 top-level phase の順序 (`diagnose → pre-execute-handover → build`)、全て `Invoker::Pipeline`
- diagnose/build の `with` が exactly `{e2e: "args.e2e", codex: "args.codex"}`、checkpoint は `with` 空
- 展開 leaf ids が exactly 9 件 (`diagnose/rca → ... → build/integrate`)
- regate は `build/code-review → [build/execute]` のみ、他 leaf は空
- verify leaves が `when: args.e2e` を継承
- `.belt/runs/` リテラル + `{run_id}` template の non-existence

**Cross-coupling** (C):

- `pipeline_split_refresh.rs` — stage 内部 shape (diagnose の docs/features path 含む) は stage 側で lock
- `feature_dev_refresh.rs` — feature-dev 合成 shape (同 tuple pattern で parallel)
```

- [ ] **Step 6: 検証する**

Run: `cargo fmt --package belt-core && cargo clippy --package belt-core -- -D warnings && cargo test -p belt-core`
Expected: 全 PASS

Run: `cargo run -p belt -- lint plugins/belt/skills/bug-fix/pipeline.yml && echo LINT-OK`
Expected: `LINT-OK`

Run: `grep -rn "docs/plans" plugins/belt/skills/ ; echo "exit=$?"`
Expected: `exit=1` (旧 path 規約への言及が plugins/belt/skills 配下からゼロ)

- [ ] **Step 7: Commit**

```bash
git add plugins/belt/skills/bug-fix/ crates/belt-core/tests/bug_fix_refresh.rs docs/testing/lock-ledger.md
git commit -m "feat(plugins): compose bug-fix from diagnose + checkpoint + build"
```

---

### Task 7: pipeline_split_refresh.rs 新設 (4 stage の shape lock 統合)

**Files:**
- Create: `crates/belt-core/tests/pipeline_split_refresh.rs`
- Modify: `docs/testing/lock-ledger.md` (新節を `## bug_fix_refresh.rs` 節の直後に追加)

**Interfaces:**
- Consumes: Tasks 1–6 の成果物一式 (4 stage pipeline + 合成後のファイル配置)
- Produces: stage 内部 shape の唯一の lock。`common::narrative` helpers を再利用 (`assert_narrative_produce_paths(&Pipeline, &[(&str,&str,&str)])` ほか、helper シグネチャは `crates/belt-core/tests/common/narrative.rs` 準拠)

- [ ] **Step 1: pipeline_split_refresh.rs を書く**

```rust
//! Integration tests locking the four stage pipelines introduced by the
//! 2026-07-02 pipeline split (spec docs/specs/2026-07-02-pipeline-split-design.md).
//!
//! Shape contract:
//! - design (design -> test-scenarios -> spec-review -> plan),
//!   diagnose (rca -> fix-plan -> fix-plan-review),
//!   build (execute -> code-review -> verify -> integrate),
//!   verify (monkey-test -> dogfood)
//! - design/diagnose/build declare args {e2e, codex}; verify declares none
//! - regate stays closed inside each pipeline file:
//!   design: spec-review -> [test-scenarios]; build: code-review -> [execute]
//! - build.verify delegates to ../verify/pipeline.yml gated by when: args.e2e
//! - build declares no upstream consumes (upstream artifacts are located by
//!   SKILL.md entry checks, not by the artifact graph)
//! - diagnose + verify domain artifact paths use the unified docs/features/
//!   scheme (docs/plans/ is retired)
//! - criteria are structured with no ordinal `# Phase N` headings; audit
//!   strictness: execute/code-review = required, all others = lite
//! - each stage skill ships SKILL.md (user-invocable) + belt.toml, and the
//!   old feature-dev/bug-fix criteria+references directories are gone

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::match_wildcard_for_single_variants,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use std::path::PathBuf;

use belt_core::{
    expander::expand_pipeline,
    model::{ArgType, ArtifactRef, Invoker, Pipeline},
    parser::parse_pipeline,
};

mod common;
use common::helpers::repo_root;
use common::narrative::{
    assert_narrative_accumulating_consumes, assert_narrative_gate_paths,
    assert_narrative_produce_paths, assert_non_narrative_phases_have_no_notes,
};

fn stage_dir(stage: &str) -> PathBuf {
    repo_root().join("plugins/belt/skills").join(stage)
}

fn stage_pipeline(stage: &str) -> Pipeline {
    parse_pipeline(&stage_dir(stage).join("pipeline.yml"))
        .unwrap_or_else(|e| panic!("{stage} pipeline.yml must parse: {e:?}"))
}

fn named_consumes(pipeline: &Pipeline, phase_id: &str) -> Vec<String> {
    let phase = pipeline
        .phases
        .iter()
        .find(|p| p.id == phase_id)
        .unwrap_or_else(|| panic!("phase '{phase_id}' must exist"));
    phase
        .consumes
        .iter()
        .filter_map(|r| match r {
            ArtifactRef::Named(n) => Some(n.clone()),
            _ => None,
        })
        .collect()
}

fn artifact_path(pipeline: &Pipeline, phase_id: &str, artifact: &str) -> String {
    let phase = pipeline
        .phases
        .iter()
        .find(|p| p.id == phase_id)
        .unwrap_or_else(|| panic!("phase '{phase_id}' must exist"));
    phase
        .produces
        .iter()
        .find(|a| a.name == artifact)
        .unwrap_or_else(|| panic!("phase '{phase_id}' must produce '{artifact}'"))
        .path
        .clone()
}

#[test]
fn stage_phase_orders() {
    let expected: &[(&str, &[&str])] = &[
        ("design", &["design", "test-scenarios", "spec-review", "plan"]),
        ("diagnose", &["rca", "fix-plan", "fix-plan-review"]),
        ("build", &["execute", "code-review", "verify", "integrate"]),
        ("verify", &["monkey-test", "dogfood"]),
    ];
    for (stage, phases) in expected {
        let pipeline = stage_pipeline(stage);
        let got: Vec<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(&got, phases, "{stage} phase order must match");
    }
}

#[test]
fn design_diagnose_build_args_are_e2e_and_codex_only() {
    for stage in ["design", "diagnose", "build"] {
        let pipeline = stage_pipeline(stage);
        let mut keys: Vec<&str> = pipeline.args.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["codex", "e2e"], "{stage} args must be {{codex, e2e}}");
        for (name, def) in &pipeline.args {
            assert!(
                matches!(def.arg_type, ArgType::Bool),
                "{stage} arg '{name}' must be bool"
            );
            assert_eq!(
                def.default.as_ref().and_then(serde_json::Value::as_bool),
                Some(false),
                "{stage} arg '{name}' default must be false"
            );
        }
    }
}

#[test]
fn verify_declares_no_args() {
    let pipeline = stage_pipeline("verify");
    assert!(
        pipeline.args.is_empty(),
        "verify must declare no args (e2e gating lives on the composing side)"
    );
}

#[test]
fn design_spec_review_regates_test_scenarios_only() {
    let pipeline = stage_pipeline("design");
    for phase in &pipeline.phases {
        if phase.id == "spec-review" {
            assert_eq!(phase.regate, vec!["test-scenarios".to_string()]);
        } else {
            assert!(
                phase.regate.is_empty(),
                "design phase '{}' must have empty regate",
                phase.id
            );
        }
    }
}

#[test]
fn build_code_review_regates_execute_only() {
    let pipeline = stage_pipeline("build");
    for phase in &pipeline.phases {
        if phase.id == "code-review" {
            assert_eq!(phase.regate, vec!["execute".to_string()]);
        } else {
            assert!(
                phase.regate.is_empty(),
                "build phase '{}' must have empty regate",
                phase.id
            );
        }
    }
}

#[test]
fn build_verify_delegates_to_verify_pipeline_gated_by_e2e() {
    let pipeline = stage_pipeline("build");
    let verify = pipeline
        .phases
        .iter()
        .find(|p| p.id == "verify")
        .expect("build must have a verify phase");
    assert_eq!(
        verify.when.as_deref(),
        Some("args.e2e"),
        "build.verify must be gated by args.e2e"
    );
    match verify.invoke.as_ref() {
        Some(Invoker::Pipeline {
            pipeline: sub_path,
            with,
        }) => {
            assert_eq!(sub_path, "../verify/pipeline.yml");
            assert!(with.is_empty(), "build.verify must not pass `with` args");
        }
        other => panic!("build.verify must use Invoker::Pipeline, got {other:?}"),
    }
}

#[test]
fn build_declares_no_upstream_consumes() {
    let pipeline = stage_pipeline("build");
    assert!(
        named_consumes(&pipeline, "execute").is_empty(),
        "build.execute must not consume upstream artifacts"
    );
    assert_eq!(
        named_consumes(&pipeline, "code-review"),
        vec!["execute_notes".to_string()],
        "build.code-review must consume execute_notes only"
    );
    assert!(
        named_consumes(&pipeline, "integrate").is_empty(),
        "build.integrate must not consume upstream artifacts"
    );
}

#[test]
fn verify_dogfood_consumes_monkey_artifacts_only() {
    let pipeline = stage_pipeline("verify");
    assert_eq!(
        named_consumes(&pipeline, "dogfood"),
        vec![
            "monkey_test_report".to_string(),
            "monkey_test_results".to_string(),
            "monkey_test_notes".to_string(),
        ],
        "verify.dogfood consumes must stay inside the verify file"
    );
}

#[test]
fn diagnose_artifact_paths_use_docs_features() {
    let pipeline = stage_pipeline("diagnose");
    assert_eq!(
        artifact_path(&pipeline, "rca", "rca_report"),
        "docs/features/*/rca-report.md"
    );
    assert_eq!(
        artifact_path(&pipeline, "rca", "rca_scenarios"),
        "docs/features/*/rca-scenarios.yml"
    );
    assert_eq!(
        artifact_path(&pipeline, "fix-plan", "fix_plan_doc"),
        "docs/features/*/fix-plan.md"
    );
}

#[test]
fn diagnose_rca_scenarios_when_is_typed() {
    let pipeline = stage_pipeline("diagnose");
    let rca = pipeline
        .phases
        .iter()
        .find(|p| p.id == "rca")
        .expect("rca phase must exist");
    let scenarios = rca
        .produces
        .iter()
        .find(|a| a.name == "rca_scenarios")
        .expect("rca_scenarios artifact must exist");
    assert_eq!(
        scenarios.when,
        Some("args.e2e".to_string()),
        "rca_scenarios.when must parse as a typed field"
    );
}

#[test]
fn verify_artifact_paths_use_docs_features() {
    let pipeline = stage_pipeline("verify");
    assert_eq!(
        artifact_path(&pipeline, "monkey-test", "monkey_test_report"),
        "docs/features/*/monkey-test-report.md"
    );
    assert_eq!(
        artifact_path(&pipeline, "monkey-test", "monkey_test_results"),
        "docs/features/*/monkey-test-results.json"
    );
    assert_eq!(
        artifact_path(&pipeline, "dogfood", "dogfood_report"),
        "docs/features/*/dogfood-report/report.md"
    );
}

#[test]
fn stage_narrative_shapes() {
    // design
    let design = stage_pipeline("design");
    let design_rows: &[(&str, &str, &str)] = &[
        ("design", "design_notes", "belt://current/notes/phase-design.md"),
        ("plan", "plan_notes", "belt://current/notes/phase-plan.md"),
    ];
    assert_narrative_produce_paths(&design, design_rows);
    assert_narrative_gate_paths(&design, design_rows);
    assert_narrative_accumulating_consumes(
        &design,
        &[("design", &[]), ("plan", &["design_notes"])],
    );
    assert_non_narrative_phases_have_no_notes(&design, &["test-scenarios", "spec-review"]);

    // diagnose
    let diagnose = stage_pipeline("diagnose");
    let diagnose_rows: &[(&str, &str, &str)] = &[
        ("rca", "rca_notes", "belt://current/notes/phase-rca.md"),
        ("fix-plan", "fix_plan_notes", "belt://current/notes/phase-fix-plan.md"),
    ];
    assert_narrative_produce_paths(&diagnose, diagnose_rows);
    assert_narrative_gate_paths(&diagnose, diagnose_rows);
    assert_narrative_accumulating_consumes(
        &diagnose,
        &[("rca", &[]), ("fix-plan", &["rca_notes"])],
    );
    assert_non_narrative_phases_have_no_notes(&diagnose, &["fix-plan-review"]);

    // build
    let build = stage_pipeline("build");
    let build_rows: &[(&str, &str, &str)] = &[
        ("execute", "execute_notes", "belt://current/notes/phase-execute.md"),
        (
            "code-review",
            "code_review_notes",
            "belt://current/notes/phase-code-review.md",
        ),
    ];
    assert_narrative_produce_paths(&build, build_rows);
    assert_narrative_gate_paths(&build, build_rows);
    assert_narrative_accumulating_consumes(
        &build,
        &[("execute", &[]), ("code-review", &["execute_notes"])],
    );
    assert_non_narrative_phases_have_no_notes(&build, &["verify", "integrate"]);

    // verify
    let verify = stage_pipeline("verify");
    let verify_rows: &[(&str, &str, &str)] = &[
        (
            "monkey-test",
            "monkey_test_notes",
            "belt://current/notes/phase-monkey-test.md",
        ),
        ("dogfood", "dogfood_notes", "belt://current/notes/phase-dogfood.md"),
    ];
    assert_narrative_produce_paths(&verify, verify_rows);
    assert_narrative_gate_paths(&verify, verify_rows);
    assert_narrative_accumulating_consumes(
        &verify,
        &[("monkey-test", &[]), ("dogfood", &["monkey_test_notes"])],
    );
}

#[test]
fn stage_criteria_files_exist() {
    let expected: &[(&str, &[&str])] = &[
        ("design", &["design.md", "test-scenarios.md", "spec-review.md", "plan.md"]),
        ("diagnose", &["rca.md", "fix-plan.md", "fix-plan-review.md"]),
        ("build", &["execute.md", "code-review.md", "integrate.md"]),
        ("verify", &["monkey-test.md", "dogfood.md"]),
    ];
    for (stage, files) in expected {
        for name in *files {
            assert!(
                stage_dir(stage).join("criteria").join(name).exists(),
                "{stage} criteria '{name}' must exist"
            );
        }
    }
}

#[test]
fn stage_reference_files_exist() {
    let expected: &[(&str, &[&str])] = &[
        (
            "design",
            &[
                "brainstorming-supplement.md",
                "writing-plans-supplement.md",
                "path-convention.md",
            ],
        ),
        ("diagnose", &["rca-supplement.md", "fix-plan-supplement.md"]),
        ("build", &["worktrunk-supplement.md", "evidence-catalog.md"]),
        ("verify", &["monkey-test-supplement.md", "dogfood-supplement.md"]),
    ];
    for (stage, files) in expected {
        for name in *files {
            assert!(
                stage_dir(stage).join("references").join(name).exists(),
                "{stage} reference '{name}' must exist"
            );
        }
    }
}

#[test]
fn stage_skill_md_and_belt_toml_exist() {
    for stage in ["design", "diagnose", "build", "verify"] {
        let skill_md = stage_dir(stage).join("SKILL.md");
        let content = std::fs::read_to_string(&skill_md)
            .unwrap_or_else(|_| panic!("{stage} SKILL.md must exist"));
        assert!(
            content.contains("user-invocable: true"),
            "{stage} SKILL.md must be user-invocable"
        );
        assert!(
            stage_dir(stage).join("belt.toml").exists(),
            "{stage} belt.toml must exist"
        );
    }
}

#[test]
fn criteria_have_no_ordinal_headings() {
    for stage in ["design", "diagnose", "build", "verify"] {
        let criteria_dir = stage_dir(stage).join("criteria");
        for entry in std::fs::read_dir(&criteria_dir).expect("criteria dir must exist") {
            let path = entry.expect("dir entry").path();
            let content = std::fs::read_to_string(&path).expect("criteria file readable");
            assert!(
                !content.contains("# Phase "),
                "{} must not contain an ordinal `# Phase N` heading",
                path.display()
            );
        }
    }
}

#[test]
fn criteria_audit_strictness() {
    let required = [
        stage_dir("build").join("criteria/execute.md"),
        stage_dir("build").join("criteria/code-review.md"),
    ];
    for path in &required {
        let content = std::fs::read_to_string(path).expect("criteria file readable");
        assert!(
            content.contains("audit: required"),
            "{} must keep audit: required",
            path.display()
        );
    }
    let lite: &[(&str, &str)] = &[
        ("design", "design.md"),
        ("design", "test-scenarios.md"),
        ("design", "spec-review.md"),
        ("design", "plan.md"),
        ("diagnose", "rca.md"),
        ("diagnose", "fix-plan.md"),
        ("diagnose", "fix-plan-review.md"),
        ("build", "integrate.md"),
        ("verify", "monkey-test.md"),
        ("verify", "dogfood.md"),
    ];
    for (stage, name) in lite {
        let path = stage_dir(stage).join("criteria").join(name);
        let content = std::fs::read_to_string(&path).expect("criteria file readable");
        assert!(
            content.contains("audit: lite"),
            "{} must declare audit: lite",
            path.display()
        );
    }
}

#[test]
fn stage_pipelines_expand_cleanly() {
    let expected: &[(&str, &[&str])] = &[
        ("design", &["design", "test-scenarios", "spec-review", "plan"]),
        ("diagnose", &["rca", "fix-plan", "fix-plan-review"]),
        (
            "build",
            &[
                "execute",
                "code-review",
                "verify/monkey-test",
                "verify/dogfood",
                "integrate",
            ],
        ),
        ("verify", &["monkey-test", "dogfood"]),
    ];
    for (stage, leaves) in expected {
        let expanded = expand_pipeline(&stage_dir(stage).join("pipeline.yml"))
            .unwrap_or_else(|e| panic!("{stage} must expand: {e:?}"));
        let ids: Vec<&str> = expanded.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(&ids, leaves, "{stage} expanded leaves must match");
    }
}

#[test]
fn old_stage_files_are_removed() {
    for skill in ["feature-dev", "bug-fix"] {
        for dir in ["criteria", "references"] {
            let path = stage_dir(skill).join(dir);
            assert!(
                !path.exists(),
                "{skill}/{dir}/ must be gone after the split, found {}",
                path.display()
            );
        }
    }
}
```

- [ ] **Step 2: RED/GREEN を確認する**

Run: `cargo test -p belt-core --test pipeline_split_refresh`
Expected: 全 19 fn PASS。FAIL した場合はテストではなく Tasks 1–6 の成果物のずれを疑い、該当 Task の仕様 (この plan の YAML / criteria 記載) に合わせて成果物側を直す

- [ ] **Step 3: lock-ledger.md に新節を追加する**

`## bug_fix_refresh.rs` 節の直後 (`---` 区切りの後) に追加 (fenced block の注意は Task 5 Step 6 と同じ):

```markdown
## pipeline_split_refresh.rs

​```yaml
locks-file: crates/belt-core/tests/pipeline_split_refresh.rs
pipelines:
  - plugins/belt/skills/design/pipeline.yml
  - plugins/belt/skills/diagnose/pipeline.yml
  - plugins/belt/skills/build/pipeline.yml
  - plugins/belt/skills/verify/pipeline.yml
test-fn-count: 19
cross-coupling:
  - crates/belt-core/tests/feature_dev_refresh.rs
  - crates/belt-core/tests/bug_fix_refresh.rs
​```

**19 test fn 名** (A):

- `stage_phase_orders`
- `design_diagnose_build_args_are_e2e_and_codex_only`
- `verify_declares_no_args`
- `design_spec_review_regates_test_scenarios_only`
- `build_code_review_regates_execute_only`
- `build_verify_delegates_to_verify_pipeline_gated_by_e2e`
- `build_declares_no_upstream_consumes`
- `verify_dogfood_consumes_monkey_artifacts_only`
- `diagnose_artifact_paths_use_docs_features`
- `diagnose_rca_scenarios_when_is_typed`
- `verify_artifact_paths_use_docs_features`
- `stage_narrative_shapes`
- `stage_criteria_files_exist`
- `stage_reference_files_exist`
- `stage_skill_md_and_belt_toml_exist`
- `criteria_have_no_ordinal_headings`
- `criteria_audit_strictness`
- `stage_pipelines_expand_cleanly`
- `old_stage_files_are_removed`

**locked shape dimensions** (B):

- 4 stage の phase 順序 / args set (design・diagnose・build = `{e2e, codex}` bool default false、verify = args なし)
- regate topology が各 pipeline ファイル内で閉じる (design: `spec-review → [test-scenarios]`、build: `code-review → [execute]`、他は空)
- `build.verify` が `../verify/pipeline.yml` へ delegate + `when: args.e2e` + `with` 空
- build / verify の上流 consumes 非宣言 (artifact graph は stage 内に閉じる)
- diagnose / verify の domain artifact path が `docs/features/*/` 統一形 (rca_scenarios は typed when)
- narrative shape (produce/gate/accumulating consumes/non-narrative) per stage
- criteria 12 本の物理存在 + 序数見出しゼロ + audit 厳格度 (execute/code-review = required、他 = lite)
- references 9 本 + SKILL.md (user-invocable) + belt.toml の物理存在
- 4 stage の展開形 (build は verify nested 込みで 5 leaves)
- 旧 feature-dev / bug-fix の criteria/ references/ ディレクトリ非存在

**Cross-coupling** (C):

- `feature_dev_refresh.rs` / `bug_fix_refresh.rs` — 合成 surface (top-level with / 展開 leaf ids) は合成側で lock
```

- [ ] **Step 4: 検証して Commit**

Run: `cargo fmt --package belt-core && cargo clippy --package belt-core -- -D warnings && cargo test -p belt-core && cargo test -p belt-core --test scenarios_contract`
Expected: 全 PASS

```bash
git add crates/belt-core/tests/pipeline_split_refresh.rs docs/testing/lock-ledger.md
git commit -m "test(belt-core): lock the four stage pipelines in pipeline_split_refresh"
```

---

### Task 8: protocol SKILL.md の invoke.pipeline drift 修正 + criteria-template 序数除去

**Files:**
- Modify: `plugins/belt-agent/skills/protocol/SKILL.md`
- Modify: `plugins/belt-agent/references/criteria-template.md`

**Interfaces:**
- Produces: protocol の `pipeline` invoke 記述が実装 (init 時 inline 展開) と一致。Commands 表に `locate` が載る。criteria-template の ID convention が実在 criteria (`EXECUTE-01` 形式) と一致

- [ ] **Step 1: Commands 表に locate を追加する**

`plugins/belt-agent/skills/protocol/SKILL.md` の Commands ブロック、`belt-agent status` 行の直後に追加:

old:
```
belt-agent status [--run <id>]                          # Inspect full run state (enriched)
```

new:
```
belt-agent status [--run <id>]                          # Inspect full run state (enriched)
belt-agent locate <uri> [--run <id>]                    # Resolve a belt:// URI to a physical path
```

- [ ] **Step 2: `pipeline` variant 行を inline 展開の実態に置換する**

old (Reading `phase.invoke` 表の 2 行目):
```
| `pipeline` | `{ pipeline: "./path.yml", with: { ... } }` | Initialise a nested `belt-agent` run on the referenced sub-pipeline. Pass `with` as args. Treat the nested run as a black-box until it reports `completed`. |
```

new:
```
| `pipeline` | `{ pipeline: "./path.yml", with: { ... } }` | None at runtime. `belt-agent init` expands pipeline references inline (recursively, depth <= 4) into `{parent}/{sub}` namespaced leaf phases, so `next` only ever returns leaf phases — this variant never reaches the orchestrator. |
```

- [ ] **Step 3: `with` 解決の段落を expander 側の実態に置換する**

old (表の直後の段落全体):
```
**`pipeline` invoke — `with` template resolution.** When a `with` entry's
value is a string of the form `"args.X"` (literal prefix `args.` followed by
a single arg identifier — no nested dotted paths), resolve it against the
parent run's `args` before calling `belt-agent init --arg X=<value>`. Literal
values (bool, number, non-template string) are passed through verbatim. If
`args.X` is absent in the parent, omit the `--arg` instead of passing `null`;
the sub-pipeline's declared default applies.
```

new:
```
**`pipeline` references — `with` template resolution.** The expander resolves
`with` at `init` time: an entry whose value is the bare full string `"args.X"`
is substituted against the composing pipeline's arg scope (interpolated forms
like `"prefix args.X"` are NOT substituted and pass through verbatim, as do
literal bool/number/string values). Substitution happens level by level
inside the expander; the orchestrator never initialises nested runs and never
passes `--arg` for sub-pipelines.
```

- [ ] **Step 4: Status Output の例から nested-pipeline 表現を除去する**

old (Status Output JSON 例内):
```
      "invoke": { "pipeline": "./nested-pipeline.yml", "with": {} },
```

new:
```
      "invoke": { "skill": "/belt:code-review" },
```

(run state は展開済み leaf を保持するため、status に `pipeline` invoke が現れることはない)

- [ ] **Step 5: criteria-template.md の序数を除去する**

`plugins/belt-agent/references/criteria-template.md` の編集 (exact match):

old (File Format 内 frontmatter):
```
---
phase: {N}
name: {phase_name}
max_retries: 3
audit: required
---
```

new:
```
---
name: {phase-id}
max_retries: 3
audit: required | lite
---
```

old (ID Convention 節):
```
- Phase N criteria: `DN-01`, `DN-02`, ...
- Evidence-derived criteria (synthesized dynamically): `DN-E1`, `DN-E2`, ...
```

new:
```
- Criteria IDs derive from the phase id in uppercase kebab-case:
  `{PHASE-ID}-01`, `{PHASE-ID}-02`, ... (e.g., `EXECUTE-01`, `MONKEY-TEST-03`)
- Evidence-derived criteria (synthesized dynamically): `{PHASE-ID}-E1`, `{PHASE-ID}-E2`, ...
```

- [ ] **Step 6: 検証して Commit**

Run: `grep -n "nested" plugins/belt-agent/skills/protocol/SKILL.md`
Expected: black-box としての nested run 記述が残っていない (Step 2/3 の新文言のみ)

```bash
git add plugins/belt-agent/skills/protocol/SKILL.md plugins/belt-agent/references/criteria-template.md
git commit -m "docs(belt-agent): align protocol invoke.pipeline semantics with inline expansion"
```

---

### Task 9: codex no-response skip 規則の追記 + 序数残渣の除去

**Files:**
- Modify: `plugins/belt/skills/code-review/SKILL.md`
- Modify: `plugins/belt/skills/spec-review/SKILL.md`
- Modify: `plugins/belt/skills/monkey-test/SKILL.md`

**Interfaces:**
- Produces: 実測で常態化していた「codex 無応答 → 手動 skip」摩擦の解消規則 (時計を持たない LLM 向けに他 reviewer 完了を基準とする)

- [ ] **Step 1: code-review SKILL.md に skip 規則を追記する**

`## Parallel Dispatch` 節内、`If `--codex` is set, also invoke `/codex:rescue` ...` の段落の直後 (`Each finding artifact is independent ...` の前) に追加:

```markdown
If `--codex` is set and Codex has not responded by the time every other
reviewer in the parallel batch has completed, skip the Codex pass and record
a single entry in the merged findings:
`{"observation": "codex", "severity": "low", "description": "codex adversarial pass skipped (no response)"}`.
Orchestrators have no wall clock; completion of the other reviewers is the
timeout baseline.
```

- [ ] **Step 2: spec-review SKILL.md に同規則を追記する**

`If `--codex` is set, also invoke `/codex:rescue` in the same parallel batch with a review-specific prompt: supply the spec, expected findings format, and the resolved `output_path` (from `belt-agent status` `findings-codex` artifact).` の段落直後に、Step 1 と同一の文面を追加する (byte-identical に保つ)。

- [ ] **Step 3: monkey-test SKILL.md の序数を除去する**

old:
```
- Never skip writing `results.json` — downstream Phase 7 (dogfood) depends on it.
```

new:
```
- Never skip writing `results.json` — the downstream dogfood phase depends on it.
```

- [ ] **Step 4: 検証して Commit**

Run: `grep -rn "Phase [0-9]" plugins/ ; echo "exit=$?"`
Expected: `exit=1` (plugins 配下の序数言及ゼロ — Tasks 3/4/8 と合わせて全滅)

Run: `cargo test -p belt-core --test review_skills_refresh --test shared_filter_parity`
Expected: PASS (追記は Filtering 節・dispatch 構造 lock に非干渉)

```bash
git add plugins/belt/skills/code-review/SKILL.md plugins/belt/skills/spec-review/SKILL.md plugins/belt/skills/monkey-test/SKILL.md
git commit -m "docs(plugins): add codex no-response skip rule; drop last phase ordinals"
```

---

### Task 10: AGENTS.md 更新 (uses: 記述除去 + CLI/skill 一覧刷新)

**Files:**
- Modify: `AGENTS.md` (CLAUDE.md は symlink — 編集はどちらからでも透過だが **`git add AGENTS.md`** が必須)

**Interfaces:**
- Produces: AGENTS.md が Plan A の再帰 expander + Plan B の stage 構成と一致する

以下は全て exact match の置換。**注意**: Non-Goals 節の `uses:` gate の実行時解決 (MVP では passthrough、将来実装)` は gate 定義の `uses:` (実在概念、`check_gate_uses_exist`) を指すため**触らない**。

- [ ] **Step 1: CLI コマンド一覧を更新する (3 箇所)**

| old | new |
|---|---|
| `- **CLI 体系**: `belt lint` (静的検証) / `belt-agent init\|next\|verify\|step\|status` (runtime)` | `- **CLI 体系**: `belt lint` (静的検証) / `belt-agent init\|next\|verify\|regate\|step\|status\|locate` (runtime)` |
| `\| 主コマンド \| `lint` \| `init`, `next`, `verify`, `step`, `status` \|` | `\| 主コマンド \| `lint` \| `init`, `next`, `verify`, `regate`, `step`, `status`, `locate` \|` |
| `│   └── belt-agent/   # 🤖 Agent runtime CLI binary (belt-agent init/next/verify/step/status)` | `│   └── belt-agent/   # 🤖 Agent runtime CLI binary (belt-agent init/next/verify/regate/step/status/locate)` |

同様に Crate 構成表の belt-agent 行 `` `init/next/verify/step/status` で belt-core Engine + gate executor を駆動 `` → `` `init/next/verify/regate/step/status/locate` で belt-core Engine + gate executor を駆動 ``。

- [ ] **Step 2: expander モジュール行を再帰展開の実態に更新する**

old:
```
| `expander` | `expand_pipeline()` — `uses:` 参照を flat namespace に展開。親の gate/regate/when 継承 |
```

new:
```
| `expander` | `expand_pipeline()` — `invoke.pipeline` 参照を再帰的に flat namespace に展開 (深さ上限 4、循環検出、sub 内 regate リネーム)。親の gate/regate/when 継承 |
```

- [ ] **Step 3: YAML 例の uses: を invoke.pipeline に更新する**

old (YAML パイプライン構造 例内):
```yaml
  - id: review
    uses: ./pipelines/review-cycle.yml    # sub-pipeline 参照
    with: { skill: "/code-review" }
    when: "args.smoke"
```

new:
```yaml
  - id: review
    invoke:
      pipeline: ./pipelines/review-cycle.yml    # sub-pipeline 参照
      with: { codex: "args.codex" }
    when: "args.smoke"
```

- [ ] **Step 4: Sub-Pipeline 展開 節を更新する**

old:
```
`uses:` で参照された sub-pipeline の phases は `{parent_id}/{sub_phase_id}` にリネームされる。最後の sub-phase が親の gate/regate/validate/config を継承する。親の `when:` は全 sub-phase に伝播する。
```

new:
```
`invoke.pipeline` で参照された sub-pipeline の phases は `{parent_id}/{sub_phase_id}` にリネームされる (再帰参照は `{a}/{b}/{c}` 連結、深さ上限 4、循環は InvalidPipeline)。最後の sub-phase が親の gate/regate/validate/config を継承し、親の `when:` は全 sub-phase に伝播する。sub 内で sibling を指す `regate` は namespace リネームされる (別ファイルの phase を指す regate は解決されない — regate は各 pipeline ファイル内で閉じること)。`with:` は bare `args.<name>` full-string form のみ置換される。
```

- [ ] **Step 5: リモート uses: の 2 箇所を更新する**

| old | new |
|---|---|
| `将来的に、pipeline / gate / sub-pipeline を Web で誰でも公開/取得できるエコシステムを構築する。リモート `uses:` は git clone でキャッシュ (HTTP lib 不要)。` | `将来的に、pipeline / gate / sub-pipeline を Web で誰でも公開/取得できるエコシステムを構築する。リモート `invoke.pipeline` 参照は git clone でキャッシュ (HTTP lib 不要)。` |
| `- **リモート `uses:`**: git-based sub-pipeline/gate 参照のキャッシュ解決` | `- **リモート `invoke.pipeline`**: git-based sub-pipeline/gate 参照のキャッシュ解決` |

- [ ] **Step 6: Plugin Architecture の belt 行に stage skill を反映する**

old:
```
| `belt` | user-invocable skills + それに紐づく reviewer agents | `/belt:<skill>`, `belt:<reviewer>` |
```

new:
```
| `belt` | user-invocable skills (stage pipelines: design / diagnose / build / verify、合成: feature-dev / bug-fix、ほか review 系) + それに紐づく reviewer agents | `/belt:<skill>`, `belt:<reviewer>` |
```

- [ ] **Step 7: 検証して Commit**

Run: `grep -n "uses:" AGENTS.md`
Expected: Non-Goals の gate `uses:` 行のみが残る (1 箇所)

```bash
git add AGENTS.md
git commit -m "docs: refresh AGENTS.md for recursive invoke.pipeline and stage skills"
```

---

### Task 11: plugin version 0.3.0 + description / README 同期

**Files:**
- Modify: `plugins/belt/.claude-plugin/plugin.json`
- Modify: `plugins/belt-agent/.claude-plugin/plugin.json`
- Modify: `.claude-plugin/marketplace.json`
- Modify: `README.md` (2 行のみ — Spec Gap Note 1)

**Interfaces:**
- Produces: plugin.json と marketplace.json の description が byte-identical (drift 防止は同一 commit 原則)

- [ ] **Step 1: belt plugin.json を更新する**

`plugins/belt/.claude-plugin/plugin.json` — `version` を `"0.3.0"` へ、`description` を以下へ:

```
User-invocable skills and their reviewer agents: /belt:feature-dev, /belt:bug-fix (composed), /belt:design, /belt:diagnose, /belt:build, /belt:verify, /belt:code-review (4 reviewers), /belt:spec-review (3 reviewers), /belt:monkey-test, /belt:test-scenarios, /belt:handover, /belt:resume. Requires belt-agent plugin
```

- [ ] **Step 2: belt-agent plugin.json の version を 0.3.0 に更新する**

`plugins/belt-agent/.claude-plugin/plugin.json` — `version": "0.2.0"` → `"0.3.0"` (description は不変 — protocol 修正は記述内容に影響しない)。

- [ ] **Step 3: marketplace.json の belt description を Step 1 と同一文字列に更新する**

`.claude-plugin/marketplace.json` の belt plugin エントリの `description` を Step 1 の文字列に置換 (belt-agent エントリは不変)。

- [ ] **Step 4: README.md の 2 行を補正する**

| old | new |
|---|---|
| `structural errors — missing phase IDs, invalid gate checks, broken `uses:`` (行 17–18 の `uses:`) | `structural errors — missing phase IDs, invalid gate checks, broken `invoke.pipeline`` |
| `\| `belt` \| User-invocable pipelines and reviewer agents: `/belt:feature-dev`, `/belt:bug-fix`, `/belt:code-review` (4 observation reviewers), ...` (行 271) | 同行の `/belt:bug-fix`,` の後に `` `/belt:design`, `/belt:diagnose`, `/belt:build`, `/belt:verify`, `` を挿入 |

README の全面 refresh (Quick Start の pipeline 例など) は follow-up (out of scope)。

- [ ] **Step 5: 検証して Commit**

Run: `diff <(python3 -c "import json;print(json.load(open('plugins/belt/.claude-plugin/plugin.json'))['description'])") <(python3 -c "import json;print([p for p in json.load(open('.claude-plugin/marketplace.json'))['plugins'] if p['name']=='belt'][0]['description'])") && echo DESC-SYNCED`
Expected: `DESC-SYNCED`

```bash
git add plugins/belt/.claude-plugin/plugin.json plugins/belt-agent/.claude-plugin/plugin.json .claude-plugin/marketplace.json README.md
git commit -m "chore(plugins): bump belt plugins to 0.3.0 for the pipeline split"
```

---

### Task 12: 最終検証 (workspace + 実 pipeline 回帰 + CI)

**Files:** なし (検証のみ。失敗時は該当 Task に戻る)

- [ ] **Step 1: CI と同一コマンドをローカル実行する**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --locked -- -D warnings && cargo test --workspace --locked`
Expected: fmt OK / No issues / 全テスト PASS (Plan A 完了時点 462 + pipeline_split 19 − shared_criteria_parity 2 − 旧 refresh 削減分。正確な数は実測で記録)

- [ ] **Step 2: 実 pipeline 6 本を belt lint で確認する**

Run:
```bash
for p in design diagnose build verify feature-dev bug-fix; do
  cargo run -p belt -- lint plugins/belt/skills/$p/pipeline.yml || exit 1
done && echo ALL-LINT-OK
```
Expected: `ALL-LINT-OK`

- [ ] **Step 3: 合成 run の boot を adversarial probe する**

scratch dir で実際に init/next/status を駆動し、展開・when 伝播・regate namespace を runtime で観測する:

```bash
SCRATCH=$(mktemp -d)
REPO=$(pwd)
cd "$SCRATCH"
# (a) feature-dev: 先頭 leaf が design/design で /brainstorming を指す
cargo run --manifest-path "$REPO/Cargo.toml" -p belt-agent -- init "$REPO/plugins/belt/skills/feature-dev/pipeline.yml" --arg e2e=true --arg codex=true
cargo run --manifest-path "$REPO/Cargo.toml" -p belt-agent -- next
# (b) bug-fix: status が 9 leaf (verify 2 leaf 含む) を列挙する
cargo run --manifest-path "$REPO/Cargo.toml" -p belt-agent -- init "$REPO/plugins/belt/skills/bug-fix/pipeline.yml"
cargo run --manifest-path "$REPO/Cargo.toml" -p belt-agent -- status
# (c) 単体 build: e2e=false で verify leaves が skip 対象になる
cargo run --manifest-path "$REPO/Cargo.toml" -p belt-agent -- init "$REPO/plugins/belt/skills/build/pipeline.yml"
cargo run --manifest-path "$REPO/Cargo.toml" -p belt-agent -- status
cd "$REPO"
```

Expected: (a) next の `phase.id == "design/design"`、`invoke.skill == "/brainstorming"`。(b) status の phases が 9 件で `build/verify/monkey-test` / `build/verify/dogfood` を含む。(c) status の phases が 5 件 (`execute` / `code-review` / `verify/monkey-test` / `verify/dogfood` / `integrate`)。いずれも JSON が エラーなく返ること

- [ ] **Step 4: 残渣 grep を流す**

Run: `grep -rn "docs/plans" plugins/belt/skills/; grep -rn "Phase [0-9]" plugins/; grep -rn "# Phase " plugins/belt/skills/*/criteria/ 2>/dev/null; echo "sweeps done"`
Expected: 3 つの grep すべて no match (`sweeps done` のみ出力)

- [ ] **Step 5: push して CI green を確認する**

```bash
git push origin main
gh run list --workflow=ci.yml --limit 1   # conclusion: success を確認
```

注意: CI 結果は `gh run view <id>` の conclusion で必ず直接確認する (watch コマンドの exit code は信用しない)。

---

## 完了条件 (Plan B)

1. `cargo test --workspace --locked` 全 PASS + CI green
2. 実 pipeline 6 本の `belt lint` が exit 0、合成 feature-dev/bug-fix の init/next/status が展開形どおりに応答 (Task 12 Step 3 の実測 3 点)
3. 序数言及・`docs/plans` 参照・`uses:` phase 参照が plugins/ から消滅 (Non-Goals の gate `uses:` のみ許容)
4. lock-ledger.md と実テストファイルの整合 (scenarios_contract PASS)
5. plugin.json / marketplace.json が 0.3.0 + description byte-identical

## Out of Scope (follow-up)

- README.md の全面 refresh (Quick Start 例・アーキテクチャ図の stage 反映)
- handover SKILL.md の stale な `debug-flow` 言及の整理
- plan 粒度ガード / grill-me ラウンド上限 / `--inherits-from` の protocol 教育 (spec の follow-up 節)
- 旧 run (.belt/runs) の migration (spec: 実害なし)
