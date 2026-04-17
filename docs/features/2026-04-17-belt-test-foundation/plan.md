# belt-test-foundation (F1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** F1 として belt test suite audit の north star (docs/testing/) + 機械検証 lock test (scenarios_contract.rs) + 最小 pilot (cli_test / config_test doc-comment 付与 + view_test flaky fix) を構築し、F2/F3 で実 audit を行うための methodology を検証する。

**Architecture:** 3 層構造: (1) SSOT = `docs/testing/` 配下 5 ファイル (README + cli-behavior/*.yml × 3 + lock-ledger + audit-template)、(2) Binding = `crates/belt-core/tests/scenarios_contract.rs` が scenarios.yml ↔ Rust doc-comment を grep ベース + block-comment strip で機械検証、(3) Pilot = `cli_test.rs` / `config_test.rs` に `/// scenario: <id>` 付与 (behavior pilot) + `feature_dev_refresh.rs` を台帳に記録 (lock pilot) + `view_test.rs` の `thread::sleep` → `filetime::set_file_mtime` 置換 (flaky 先潰し)。

**Tech Stack:** Rust 2024 (MSRV 1.86) / serde-saphyr `=0.0.23` / filetime `=0.2.27` (既存 workspace dep) / libtest (標準) / assert_cmd / tempfile。新 dep ゼロ。

---

## File Structure

| Path | Role | Task |
|------|------|------|
| `.belt/runs/{run_id}/artifacts/mv30-before.txt` | MV-30 capture (before snapshot) | 1 |
| `docs/testing/README.md` | 目的 / `docs/features/` との境界 / 運用 entry point | 2 |
| `docs/testing/audit-template.md` | v1 9 reason labels / decision tree / re-audit trigger | 3 |
| `docs/testing/cli-behavior/belt.yml` | belt lint CLI scenarios (5 件) | 4 |
| `docs/testing/cli-behavior/belt-core.yml` | config module scenarios (6 件) + scope stub | 5 |
| `docs/testing/cli-behavior/belt-agent.yml` | stub + scope 宣言のみ | 6 |
| `docs/testing/lock-ledger.md` | 5 既存 lock + 1 scenarios_contract の台帳 | 7 |
| `crates/belt-core/tests/scenarios_contract.rs` | binding 機械検証 lock (positive + 7 drift injection) | 8 |
| `crates/belt/tests/cli_test.rs` (modify) | `/// scenario: belt-*` doc-comment 付与 (5 test) | 9 |
| `crates/belt-core/tests/config_test.rs` (modify) | `/// scenario: belt-core-config-*` doc-comment 付与 (6 test) | 10 |
| `crates/belt-core/tests/view_test.rs` (modify) | `thread::sleep` × 6 → `filetime::set_file_mtime` 置換 | 11 |
| `docs/features/2026-04-17-belt-test-foundation/audit-report.md` | pilot audit 判定 + frontmatter | 12 |

---

## Task 1: Capture MV-30 before snapshot

**Reason**: MV-30 (FB-05 resolution) requires `before.txt` capture before any F1 edits that could touch `docs/`. This task must be first.

**Files:**
- Create: `.belt/runs/019d9930-c7b9-7473-a9fd-70f96aa79812/artifacts/mv30-before.txt`

**Test-strategy reference:** MV-30 (table row for MV-30 in test-strategy.md Must-Verify Mapping)

- [ ] **Step 1: Create artifacts directory**

```bash
mkdir -p .belt/runs/019d9930-c7b9-7473-a9fd-70f96aa79812/artifacts/
```

- [ ] **Step 2: Capture grep output**

```bash
grep -rl "cli_test.rs\|config_test.rs\|feature_dev_refresh.rs" docs/ | sort > .belt/runs/019d9930-c7b9-7473-a9fd-70f96aa79812/artifacts/mv30-before.txt
```

- [ ] **Step 3: Verify non-empty capture**

```bash
wc -l .belt/runs/019d9930-c7b9-7473-a9fd-70f96aa79812/artifacts/mv30-before.txt
```
Expected: `>= 20` lines (30+ spec/plan docs per impact-analyzer)

- [ ] **Step 4: Commit (gitignored, but track per run state)**

`.belt/` is gitignored, so no `git add`. Instead, annotate completion in Task 13's verification.

---

## Task 2: Write `docs/testing/README.md`

**Files:**
- Create: `docs/testing/README.md`

**Test-strategy reference:** MV-01 + MV-29 (design.md Must-Verify)

- [ ] **Step 1: Create docs/testing/ directory**

```bash
mkdir -p docs/testing/cli-behavior/
```

- [ ] **Step 2: Write README.md**

```markdown
# docs/testing/ — belt test SSOT + lock meta

This directory is the belt project's **long-term test foundation**:

- **CLI behavioral SSOT** — what belt / belt-agent / belt-core public API should do, expressed as Given/When/Then scenarios
- **Lock ledger** — what shape-lock tests (`*_refresh.rs`, `shared_*_parity.rs`) protect
- **Audit operations meta** — template + reason labels for the F2/F3 test audit

## Boundary vs `docs/features/<topic>/`

| Aspect | `docs/testing/` (this dir) | `docs/features/<topic>/` |
|---|---|---|
| Lifetime | permanent, cross-feature SSOT | per-feature, archived when feature closes |
| Consumers | F2/F3 audit + CI lock | test-scenarios skill + monkey-test + dogfood |
| Schema | scenarios.yml schema + `scope:` / `technique:` additive fields | scenarios.yml schema (UI replay契約) |
| Writers | human + feature-dev Phase 5 execute | feature-dev Phase 2 test-scenarios skill |

## Contents

- `cli-behavior/belt.yml` — belt lint CLI behavioral scenarios
- `cli-behavior/belt-agent.yml` — belt-agent CLI behavioral scenarios (F3 で拡充、F1 は stub)
- `cli-behavior/belt-core.yml` — belt-core public API behavioral scenarios (F2 で拡充、F1 は config module のみ)
- `lock-ledger.md` — plugin shape lock test 台帳
- `audit-template.md` — F2/F3 audit 判定手順 (v1 9 reason labels / decision tree / re-audit trigger)

## monkey-test との非互換

`docs/testing/cli-behavior/*.yml` は CLI 向けであり、`/belt:monkey-test` (agent-browser replay) で消費するものではない。これらは `docs/features/<topic>/scenarios.yml` (UI scenarios) とは別の SSOT。monkey-test SKILL.md は `docs/features/<topic>/scenarios.yml` のみを input と宣言しており、`docs/testing/cli-behavior/` は path が異なるため偶発的消費は起きない。

## Binding

`docs/testing/cli-behavior/*.yml` の全 scenario ID は `crates/*/tests/**/*.rs` 内の `/// scenario: <id>` doc-comment と `crates/belt-core/tests/scenarios_contract.rs` で機械照合される。drift があれば CI (`cargo test`) で検出される。

## Related

- CLAUDE.md: belt project overview
- `plugins/belt/skills/test-scenarios/SKILL.md`: UI scenarios.yml producer (別 path)
- `docs/features/2026-04-17-belt-test-foundation/`: F1 feature design + test strategy + plan + audit report
```

- [ ] **Step 3: Verify file exists**

```bash
test -f docs/testing/README.md && echo "exists"
```
Expected: `exists`

- [ ] **Step 4: Commit**

```bash
git add docs/testing/README.md
git commit -m "docs(testing): add SSOT entry README with boundary declaration"
```

---

## Task 3: Write `docs/testing/audit-template.md`

**Files:**
- Create: `docs/testing/audit-template.md`

**Test-strategy reference:** MV-06 + MV-33 + CCS-03 accept + CCS-06 accept

- [ ] **Step 1: Write audit-template.md**

```markdown
---
audit_template_version: v1
---

# belt test audit template (v1)

F2/F3 で belt の個別 test ファイルを audit する際の判定手順と reason label 集。F1 で pilot 検証済み (cli_test.rs / config_test.rs / feature_dev_refresh.rs)。

## Decision Tree (per test fn)

```
Q1: この test が検証している behavior は `docs/testing/cli-behavior/<crate>.yml` に scenario として登録されているか？
 ├── yes → doc-comment `/// scenario: <id>` を付与し judgement = kept
 └── no → Q2
Q2: 同等の behavior を検証する他 test が存在するか？
 ├── yes → judgement = redundant-with-<test-id>  (→ 削除対象)
 └── no → Q3
Q3: この test は behavior でなく internal structure (private state / format / default 値) を assert しているか？
 ├── behavior assert → Q4
 ├── internal assert → judgement = implementation-coupling  (→ 削除 or 抽象化)
Q4: assertion は trivial (自明な default 等) または tautology か？
 ├── yes → judgement = trivial-default-assertion または tautology  (→ 削除)
 └── no → Q5
Q5: scenarios.yml に scenario を追加し (yml 側 update が必要)、doc-comment 付与で kept に戻す
```

## Reason Labels (v1 fixed enumeration — 9 labels)

| # | Label | 意味 |
|---|---|---|
| 1 | `redundant-with-<test-id>` | 他 test が同 behavior をカバー |
| 2 | `trivial-default-assertion` | default 値確認のみで情報量ゼロ |
| 3 | `tautology` | assertion が論理的常真 (`a == a` 等) |
| 4 | `state-transition-overlap-with-<test-id>` | state transition が既存 test と重複 |
| 5 | `implementation-coupling` | private state を assert、behavior でない |
| 6 | `brittle-format-match` | 出力 format 軽微変更で fail する fragile assertion |
| 7 | `dead-fixture` | fixture 生成のみで実効検証なし |
| 8 | `unreachable-guard` | 入力ドメインに存在しない case を守る |
| 9 | `obsolete-spec` | 仕様変更で lock 対象が消失したが test 残存 |

新 label が必要な場合は別 feature で audit-template.md を v2 以降に bump し、`scenarios_contract.rs` の version check を同時更新 (SemVer 風 migration)。

## Duplication Candidates (F2/F3 参考)

F1 の S1 探索で発見された統合候補 (file:fn 粒度):

| #1 | #2 | Reason |
|---|---|---|
| `engine_test.rs::regate_*` (14 test) | `belt-agent/tests/cli_test.rs::regate_*` (11 test) | 同 state-transition を API 層 + CLI JSON 層で重複 |
| `engine_test.rs::verify_verdict_*` | `belt-agent/tests/cli_test.rs::verify_*` | verify pass/fail semantics double |
| `parser_test.rs::parse_minimal_pipeline` | `model_test.rs` の同等 test | model_test に吸収可能 |
| `view_test.rs::engine_enriched_status_*` | `belt-agent/tests/cli_test.rs::status_*` | view module API と CLI の double coverage |
| `feature_dev_refresh.rs` × `bug_fix_refresh.rs` | narrative artifact pattern 4 組 | 同型、helper 共通化候補 |
| `shared_criteria_parity.rs` × `shared_filter_parity.rs` | byte-identity lock pattern | 共通 helper 化候補 |
| `write_yaml` / `repo_root` / `fixture_path` | 5+ 箇所 byte-identical | `tests/common/mod.rs` で統合候補 |

上記は F2/F3 で参考にするのみ。F1 では実統合しない。

## Pilot Audit の再実施 trigger (F2 着手時)

audit-report.md の frontmatter `audited_at` を読み、以下コマンドで pilot file が touch されているかを確認:

```bash
AUDITED_AT=$(yq '.audited_at' docs/features/2026-04-17-belt-test-foundation/audit-report.md)
git log --since="$AUDITED_AT" --oneline -- \
  crates/belt/tests/cli_test.rs \
  crates/belt-core/tests/config_test.rs \
  crates/belt-core/tests/feature_dev_refresh.rs
```

出力が非空なら F1 pilot 判定は stale。F2 着手時に pilot audit を再実施し audit-report.md を refresh する。
```

- [ ] **Step 2: Verify file exists + frontmatter**

```bash
test -f docs/testing/audit-template.md && grep "audit_template_version: v1" docs/testing/audit-template.md
```
Expected: file exists + version line matches

- [ ] **Step 3: Commit**

```bash
git add docs/testing/audit-template.md
git commit -m "docs(testing): add v1 audit template with 9 reason labels"
```

---

## Task 4: Write `docs/testing/cli-behavior/belt.yml`

**Files:**
- Create: `docs/testing/cli-behavior/belt.yml`

**Test-strategy reference:** MV-02 / EP-01 / BV-01 / BV-02 (schema valid)

- [ ] **Step 1: Read existing `crates/belt/tests/cli_test.rs` to identify 5 test fns**

```bash
grep -E "^\s*#\[test\]" crates/belt/tests/cli_test.rs -A 1
```
Expected: 5 `#[test]` fn. Note each fn name.

- [ ] **Step 2: Write belt.yml**

```yaml
scope: "belt human CLI (belt lint subcommand). F1 で pilot (全 5 scenarios)。F2/F3 で拡充予定 (他 belt subcommand 追加時)"
scenarios:
  - id: belt-lint-valid-pipeline-ok
    category: lint
    severity: critical
    technique: equivalence-partition
    given: "a syntactically valid pipeline.yml"
    when: "belt lint <path> is invoked"
    then: "exit code 0 and stderr contains 'ok'"
  - id: belt-lint-duplicate-phase-id-detected
    category: lint
    severity: high
    technique: equivalence-partition
    given: "a pipeline.yml with duplicate phase IDs"
    when: "belt lint <path> is invoked"
    then: "exit code 1 and stderr contains 'duplicate'"
  - id: belt-lint-invalid-yaml-rejected
    category: lint
    severity: high
    technique: equivalence-partition
    given: "a pipeline.yml with YAML syntax errors"
    when: "belt lint <path> is invoked"
    then: "exit code 1 and stderr contains diagnostic"
  - id: belt-lint-nonexistent-file-rejected
    category: lint
    severity: medium
    technique: equivalence-partition
    given: "a path to a nonexistent file"
    when: "belt lint <path> is invoked"
    then: "exit code 1 and stderr contains file-not-found message"
  - id: belt-lint-config-and-positional-mutually-exclusive
    category: lint
    severity: high
    technique: decision-table
    given: "both --config and positional path are provided"
    when: "belt lint --config <config> <path> is invoked"
    then: "exit code 1 and stderr indicates mutual exclusion"
```

**Note**: scenario IDs above will be added as `/// scenario: <id>` doc-comments in Task 9. Verify fn names match mapping intent (1:1 where possible). If a test fn covers multiple scenarios, Task 9 will add multiple `/// scenario:` lines.

- [ ] **Step 3: Verify YAML parses via yq**

```bash
yq '.scenarios | length' docs/testing/cli-behavior/belt.yml
```
Expected: `5`

- [ ] **Step 4: Commit**

```bash
git add docs/testing/cli-behavior/belt.yml
git commit -m "docs(testing): add belt lint CLI behavioral scenarios (5 scenarios)"
```

---

## Task 5: Write `docs/testing/cli-behavior/belt-core.yml`

**Files:**
- Create: `docs/testing/cli-behavior/belt-core.yml`

**Test-strategy reference:** MV-04 / EP-01 / CCS-04 accept (scope field)

- [ ] **Step 1: Read `crates/belt-core/tests/config_test.rs` to identify 6 test fns**

```bash
grep -E "^\s*#\[test\]" crates/belt-core/tests/config_test.rs -A 1
```
Expected: 6 `#[test]` fn. Note each fn name.

- [ ] **Step 2: Write belt-core.yml**

```yaml
scope: "belt-core pure library public API. F1 で config module のみ本格 (6 scenarios)。F2 で拡充予定: engine / view / lint / model / parser / expander / gate / error / uri の 9 module 公開 API"
scenarios:
  - id: belt-core-config-valid-toml-parses
    category: config
    severity: high
    technique: equivalence-partition
    given: "a valid belt.toml with pipeline_file field"
    when: "parse_config() is called"
    then: "returns Config with parsed pipeline_file path"
  - id: belt-core-config-missing-file-yields-file-not-found
    category: config
    severity: high
    technique: equivalence-partition
    given: "a path to a nonexistent belt.toml"
    when: "parse_config() is called"
    then: "returns BeltError::FileNotFound variant"
  - id: belt-core-config-invalid-toml-yields-config-parse
    category: config
    severity: high
    technique: equivalence-partition
    given: "a belt.toml with invalid TOML syntax"
    when: "parse_config() is called"
    then: "returns BeltError::ConfigParse variant"
  - id: belt-core-config-missing-pipeline-file-field-yields-config-parse
    category: config
    severity: medium
    technique: equivalence-partition
    given: "a belt.toml without pipeline_file field"
    when: "parse_config() is called"
    then: "returns BeltError::ConfigParse variant"
  - id: belt-core-config-resolves-relative-pipeline-path
    category: config
    severity: high
    technique: equivalence-partition
    given: "a belt.toml with relative pipeline_file value"
    when: "resolve_pipeline_path(config, config_dir) is called"
    then: "returns absolute path joining config_dir and pipeline_file"
  - id: belt-core-config-preserves-absolute-pipeline-path
    category: config
    severity: medium
    technique: boundary-value
    given: "a belt.toml with absolute pipeline_file value"
    when: "resolve_pipeline_path(config, config_dir) is called"
    then: "returns the absolute path unchanged"
```

- [ ] **Step 3: Verify YAML + scope field**

```bash
yq '.scope' docs/testing/cli-behavior/belt-core.yml && yq '.scenarios | length' docs/testing/cli-behavior/belt-core.yml
```
Expected: non-empty scope string + `6`

- [ ] **Step 4: Commit**

```bash
git add docs/testing/cli-behavior/belt-core.yml
git commit -m "docs(testing): add belt-core config module scenarios (6 scenarios)"
```

---

## Task 6: Write `docs/testing/cli-behavior/belt-agent.yml` (stub)

**Files:**
- Create: `docs/testing/cli-behavior/belt-agent.yml`

**Test-strategy reference:** MV-03 (stub schema valid per CCS-04)

- [ ] **Step 1: Write belt-agent.yml with scope-only stub**

```yaml
scope: "F3 で拡充予定。対象 = belt-agent CLI 全 subcommand (init, next, verify, regate, step, status) の JSON contract + state.json shape。F1 scope では列挙せず scope 宣言のみ"
scenarios: []
```

- [ ] **Step 2: Verify YAML parse + empty scenarios array**

```bash
yq '.scope' docs/testing/cli-behavior/belt-agent.yml && yq '.scenarios | length' docs/testing/cli-behavior/belt-agent.yml
```
Expected: non-empty scope string + `0`

- [ ] **Step 3: Commit**

```bash
git add docs/testing/cli-behavior/belt-agent.yml
git commit -m "docs(testing): add belt-agent.yml stub (F3 scope, scenarios: [])"
```

---

## Task 7: Write `docs/testing/lock-ledger.md`

**Files:**
- Create: `docs/testing/lock-ledger.md`

**Test-strategy reference:** MV-05 / MV-17 / MV-18 / CCS-07 accept

- [ ] **Step 1: Identify 11 test fns in `feature_dev_refresh.rs`**

```bash
grep -E "^\s*fn\s+\w+" crates/belt-core/tests/feature_dev_refresh.rs | grep -v "//" | head -15
```
Expected: 11 fn names (matching `#[test]` attributes).

- [ ] **Step 2: Write lock-ledger.md**

```markdown
# Lock Ledger — plugin shape + cross-crate 契約 lock tests

belt-core/tests/ 配下の shape lock tests の台帳。各 entry は `locks-file:` frontmatter で実 file を参照。scenarios_contract.rs が台帳の `locks-file:` フィールドと実ファイル存在を機械照合。

---

## feature_dev_refresh.rs

```yaml
locks-file: crates/belt-core/tests/feature_dev_refresh.rs
pipeline: plugins/belt/skills/feature-dev/pipeline.yml
test-fn-count: 11
cross-coupling:
  - crates/belt-core/tests/bug_fix_refresh.rs
  - crates/belt-core/tests/review_skills_refresh.rs
  - crates/belt-core/tests/shared_criteria_parity.rs
  - crates/belt-core/tests/shared_filter_parity.rs
```

**11 test fn 名** (A):

(Step 1 で list した 11 fn 名を列挙。例: `feature_dev_pipeline_parses_successfully`, `feature_dev_args_are_exactly_codex_and_e2e`, `feature_dev_narrative_phases_produce_notes`, … の実際の全 11 個)

**pipeline.yml shape dimensions locked** (B):

- `args` set が exactly `{codex, e2e}` (余分な arg は fail)
- 9 phase の順序 (`design → test-scenarios → spec-review → plan → execute → code-review → monkey-test → dogfood → integrate`)
- `code-review.regate = [execute]` (regate target 数と identity)
- narrative artifact 6 phase (`design` / `plan` / `execute` / `code-review` / `monkey-test` / `dogfood`) の `.belt/runs/{run_id}/notes/phase-*.md` 生成 + file_exists gate
- `scenarios.when = "args.e2e"` typed enum field (string ではなく ArtifactWhen)
- 各 phase の `max_retries` (全 `= 3` in this pipeline)
- `invoke.skill` field の leading slash 存在 (`/brainstorming` 等)
- `validate` が criteria file reference (`./criteria/*.md`) であることの shape
- `consumes` の accumulating narrative pattern (phase N は phase 1..N-1 の notes を consume)

**Cross-coupling** (C):

- `shared_criteria_parity.rs` — feature-dev と bug-fix の criteria/execute.md + code-review.md の byte-identical 確認
- `shared_filter_parity.rs` — code-review 4 agent + spec-review 3 agent の `## Filtering` prefix bullet byte-identical 確認
- `bug_fix_refresh.rs` — bug-fix pipeline shape (同 tuple pattern で parallel)
- `review_skills_refresh.rs` — review skill の consolidated agent 削除 / per-observation agent 存在 lock

---

## bug_fix_refresh.rs

```yaml
locks-file: crates/belt-core/tests/bug_fix_refresh.rs
pipeline: plugins/belt/skills/bug-fix/pipeline.yml
test-fn-count: 19
cross-coupling:
  - crates/belt-core/tests/feature_dev_refresh.rs
  - crates/belt-core/tests/shared_criteria_parity.rs
```

F2/F3 で同様の shape dimension 列挙を行う (F1 scope 外、F2 着手時に追記)。

---

## review_skills_refresh.rs

```yaml
locks-file: crates/belt-core/tests/review_skills_refresh.rs
consolidated-agents-deleted:
  - code-reviewer
  - spec-reviewer
per-observation-agents:
  - security-reviewer.md
  - test-reviewer.md
  - ai-antipattern-reviewer.md
  - cross-cutting-reviewer.md
  - feasibility-reviewer.md
  - ui-design-reviewer.md
  - cross-cutting-spec-reviewer.md
test-fn-count: 6
```

---

## shared_criteria_parity.rs

```yaml
locks-file: crates/belt-core/tests/shared_criteria_parity.rs
parity-pairs:
  - [plugins/belt/skills/feature-dev/criteria/execute.md, plugins/belt/skills/bug-fix/criteria/execute.md]
  - [plugins/belt/skills/feature-dev/criteria/code-review.md, plugins/belt/skills/bug-fix/criteria/code-review.md]
test-fn-count: 2
```

---

## shared_filter_parity.rs

```yaml
locks-file: crates/belt-core/tests/shared_filter_parity.rs
parity-groups:
  - code-review-agents: 4 agents の ## Filtering 先頭 3 bullet byte-identical
  - spec-review-agents: 3 agents の ## Filtering 先頭 2 bullet byte-identical
test-fn-count: 2
```

---

## scenarios_contract.rs (NEW in F1)

```yaml
locks-file: crates/belt-core/tests/scenarios_contract.rs
scenario-sources:
  - docs/testing/cli-behavior/belt.yml
  - docs/testing/cli-behavior/belt-core.yml
  - docs/testing/cli-behavior/belt-agent.yml
doc-comment-walk-scope:
  - crates/belt/tests/
  - crates/belt-agent/tests/
  - crates/belt-core/tests/
ledger-source: docs/testing/lock-ledger.md
audit-template-version: v1
test-fn-count: 9 (1 positive + 1 ledger_locks_file_exists + 7 drift injection)
```
```

- [ ] **Step 3: Verify file exists**

```bash
test -f docs/testing/lock-ledger.md && grep -c "^## " docs/testing/lock-ledger.md
```
Expected: file exists + `>= 6` H2 sections (feature_dev / bug_fix / review_skills / shared_criteria / shared_filter / scenarios_contract)

- [ ] **Step 4: Commit**

```bash
git add docs/testing/lock-ledger.md
git commit -m "docs(testing): add lock ledger (5 existing + 1 scenarios_contract)"
```

---

## Task 8: Implement `crates/belt-core/tests/scenarios_contract.rs`

**Files:**
- Create: `crates/belt-core/tests/scenarios_contract.rs`

**Test-strategy reference:** MV-07 / MV-08 / MV-09 / MV-10 / MV-11 / MV-12 / MV-32 / NFR-01 / NFR-04 / EP-01 / EP-02 / EP-03 / DT-01 / DT-02 / CCS-02 accept

**Context**: Tasks 4-7 must be done first (scenarios.yml + ledger exist). Tasks 9-10 will add doc-comments after this task (so initial run will fail MV-09). We accept transient failure between Tasks 8 and 10.

- [ ] **Step 1: Write scenarios_contract.rs with module header + helpers**

```rust
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

//! Binding lock test: docs/testing/cli-behavior/*.yml ↔ Rust doc-comment `/// scenario: <id>`.
//!
//! Walks crates/{belt,belt-agent,belt-core}/tests/ recursively. Strips block comments
//! (including block doc-comments `/** ... */`) before grep to avoid false positives.
//!
//! Source of truth:
//! - docs/testing/cli-behavior/{belt,belt-agent,belt-core}.yml — scenario IDs
//! - docs/testing/lock-ledger.md — locks-file frontmatter entries
//! - docs/testing/audit-template.md — audit_template_version = v1

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ScenariosFile {
    #[serde(default)]
    scope: Option<String>,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
struct Scenario {
    id: String,
    #[allow(dead_code)]
    category: String,
    #[allow(dead_code)]
    severity: String,
    #[allow(dead_code)]
    given: String,
    #[allow(dead_code)]
    when: String,
    #[allow(dead_code)]
    then: String,
    #[serde(default)]
    #[allow(dead_code)]
    technique: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    preconditions: Option<Vec<String>>,
    #[serde(default)]
    #[allow(dead_code)]
    postconditions: Option<Vec<String>>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn load_scenarios(rel_path: &str) -> ScenariosFile {
    let path = repo_root().join(rel_path);
    let body = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_saphyr::from_str(&body)
        .unwrap_or_else(|e| panic!("cannot parse {}: {e}", path.display()))
}

fn all_scenario_ids() -> HashSet<String> {
    let mut ids = HashSet::new();
    for rel in &[
        "docs/testing/cli-behavior/belt.yml",
        "docs/testing/cli-behavior/belt-core.yml",
        "docs/testing/cli-behavior/belt-agent.yml",
    ] {
        let file = load_scenarios(rel);
        for s in file.scenarios {
            assert!(
                ids.insert(s.id.clone()),
                "duplicate scenario id across yml files: {}",
                s.id
            );
        }
    }
    ids
}

/// Strip block comments (/* ... */, including /** ... */) from Rust source.
/// Replaces block-comment characters with spaces (preserves line numbers).
fn strip_block_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut in_block = false;
    while i < bytes.len() {
        if !in_block && i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            in_block = true;
            out.push(' ');
            out.push(' ');
            i += 2;
            continue;
        }
        if in_block && i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'/' {
            in_block = false;
            out.push(' ');
            out.push(' ');
            i += 2;
            continue;
        }
        if in_block {
            if bytes[i] == b'\n' {
                out.push('\n');
            } else {
                out.push(' ');
            }
        } else {
            out.push(bytes[i] as char);
        }
        i += 1;
    }
    out
}

/// Strip string literals (simple version: `"..."` on single line, no escape handling beyond \").
/// Good enough for CI test source which avoids complex string literal shapes.
fn strip_string_literals(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut in_str = false;
    let mut prev_escape = false;
    for c in src.chars() {
        if !in_str && c == '"' {
            in_str = true;
            out.push(' ');
            continue;
        }
        if in_str {
            if c == '"' && !prev_escape {
                in_str = false;
                out.push(' ');
            } else if c == '\n' {
                out.push('\n');
                in_str = false; // safety: strings don't span lines in CI sources
            } else {
                out.push(' ');
            }
            prev_escape = c == '\\' && !prev_escape;
            continue;
        }
        out.push(c);
        prev_escape = false;
    }
    out
}

fn collect_rust_scenario_refs() -> HashSet<String> {
    let mut found = HashSet::new();
    let re =
        regex_lite::Regex::new(r"^\s*///\s+scenario:\s+(\S+)\s*$").unwrap();
    for crate_tests in &[
        "crates/belt/tests",
        "crates/belt-agent/tests",
        "crates/belt-core/tests",
    ] {
        walk_rs_files(&repo_root().join(crate_tests), &mut |path| {
            let src = fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            let src = strip_block_comments(&src);
            let src = strip_string_literals(&src);
            for line in src.lines() {
                if let Some(caps) = re.captures(line) {
                    let id = caps.get(1).unwrap().as_str().to_string();
                    found.insert(id);
                }
            }
        });
    }
    found
}

fn walk_rs_files(dir: &Path, cb: &mut dyn FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk_rs_files(&p, cb);
        } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
            cb(&p);
        }
    }
}
```

**Note on dep**: `regex_lite` is not in workspace deps. Use `regex` if declared in workspace, otherwise fall back to manual parsing. Verify:

```bash
grep -E "^regex" Cargo.toml
```

If no regex crate, replace regex with manual parser:

```rust
// Manual replacement for the regex line:
fn match_scenario_line(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let rest = line.strip_prefix("///")?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix("scenario:")?;
    let rest = rest.trim_start();
    let id = rest.trim_end();
    if id.is_empty() || id.contains(char::is_whitespace) {
        None
    } else {
        Some(id)
    }
}
```

- [ ] **Step 2: Check workspace deps for regex**

```bash
grep -E "^(regex|regex-lite)" Cargo.toml
```

If not present, use manual parser (above); if present, use that crate.

- [ ] **Step 3: Add positive lock test**

```rust
#[test]
fn scenarios_yml_and_rust_docs_match() {
    let yml_ids = all_scenario_ids();
    let rust_ids = collect_rust_scenario_refs();

    let orphan_yml: Vec<_> = yml_ids.difference(&rust_ids).collect();
    let orphan_rust: Vec<_> = rust_ids.difference(&yml_ids).collect();

    assert!(
        orphan_yml.is_empty(),
        "orphan-yml (scenarios.yml に ID ありだが Rust 側 /// scenario: 未追加): {:?}",
        orphan_yml
    );
    assert!(
        orphan_rust.is_empty(),
        "orphan-rust (Rust に /// scenario: ID あるが scenarios.yml 未登録): {:?}",
        orphan_rust
    );
}
```

- [ ] **Step 4: Add ledger locks-file existence test**

```rust
#[test]
fn lock_ledger_locks_files_exist() {
    let ledger = fs::read_to_string(repo_root().join("docs/testing/lock-ledger.md"))
        .expect("lock-ledger.md exists");
    // Extract all `locks-file: <path>` occurrences
    let prefix = "locks-file:";
    for line in ledger.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix(prefix) else {
            continue;
        };
        let rel = rest.trim();
        let abs = repo_root().join(rel);
        assert!(
            abs.exists(),
            "lock-ledger.md references missing file: {}",
            rel
        );
    }
}
```

- [ ] **Step 5: Add audit-template version check**

```rust
#[test]
fn audit_template_version_v1_matches_expected() {
    let tpl = fs::read_to_string(repo_root().join("docs/testing/audit-template.md"))
        .expect("audit-template.md exists");
    assert!(
        tpl.contains("audit_template_version: v1"),
        "audit-template.md frontmatter must declare audit_template_version: v1"
    );
}
```

- [ ] **Step 6: Add 7 drift injection tests using in-memory fixtures**

```rust
#[test]
fn drift_regex_rejects_typo_senario() {
    let src = "/// senario: foo";
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(!has_match, "typo senario: must not match scenario: pattern");
}

#[test]
fn drift_regex_rejects_single_slash_prefix() {
    let src = "// scenario: foo";
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(
        !has_match,
        "single-slash // scenario: must not match triple-slash pattern"
    );
}

#[test]
fn drift_block_comment_with_scenario_is_stripped() {
    let src = "/* /// scenario: foo */";
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(
        !has_match,
        "block comment containing /// scenario: must be stripped"
    );
}

#[test]
fn drift_block_doc_comment_is_stripped() {
    let src = "/** scenario: foo */";
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(!has_match, "block doc-comment /** ... */ must be stripped");
}

#[test]
fn drift_string_literal_is_stripped() {
    let src = r#"let s = "/// scenario: foo";"#;
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(
        !has_match,
        "string literal containing /// scenario: must be stripped"
    );
}

#[test]
fn drift_inner_doc_comment_does_not_match() {
    let src = "//! scenario: foo";
    let stripped = strip_string_literals(&strip_block_comments(src));
    let has_match = stripped
        .lines()
        .any(|line| match_scenario_line(line).is_some());
    assert!(
        !has_match,
        "inner doc-comment //! scenario: must not match /// pattern"
    );
}

#[test]
fn drift_positive_single_line_doc_comment_matches() {
    let src = "    /// scenario: belt-lint-valid-pipeline-ok";
    let stripped = strip_string_literals(&strip_block_comments(src));
    let matched: Vec<_> = stripped
        .lines()
        .filter_map(match_scenario_line)
        .collect();
    assert_eq!(
        matched.as_slice(),
        &["belt-lint-valid-pipeline-ok"],
        "valid single-line /// scenario: must match"
    );
}
```

- [ ] **Step 7: Run cargo test to verify structure (initial fail expected for orphan-yml)**

```bash
cargo test -p belt-core --test scenarios_contract
```

Expected initial state (before Tasks 9-10): `scenarios_yml_and_rust_docs_match` fails with orphan-yml list (11 scenario IDs from belt.yml + belt-core.yml not yet bound). Other 8 tests pass.

- [ ] **Step 8: Run cargo clippy**

```bash
cargo clippy -p belt-core --tests -- -D warnings
```
Expected: clean.

- [ ] **Step 9: Commit**

```bash
git add crates/belt-core/tests/scenarios_contract.rs
git commit -m "test(belt-core): add scenarios_contract lock (9 tests, block-comment strip)"
```

Note: 1 of 9 tests expected to fail transiently until Tasks 9-10 complete. This is accepted per Task ordering (docs-first → Rust doc-comment → bind).

---

## Task 9: Add `/// scenario: belt-*` doc-comments to `crates/belt/tests/cli_test.rs`

**Files:**
- Modify: `crates/belt/tests/cli_test.rs`

**Test-strategy reference:** MV-13 / EP-03

- [ ] **Step 1: Read existing cli_test.rs to identify 5 test fns + map to belt.yml IDs**

```bash
grep -B 0 -A 0 "^fn " crates/belt/tests/cli_test.rs
```

Manual mapping table (based on Task 4 scenarios.yml):

| Rust fn (approx, confirm from source) | scenario ID |
|---|---|
| `lint_accepts_valid_pipeline` (or similar happy case) | `belt-lint-valid-pipeline-ok` |
| `lint_detects_duplicate_phase_id` | `belt-lint-duplicate-phase-id-detected` |
| `lint_rejects_invalid_yaml` | `belt-lint-invalid-yaml-rejected` |
| `lint_rejects_nonexistent_file` | `belt-lint-nonexistent-file-rejected` |
| `lint_rejects_config_and_positional` | `belt-lint-config-and-positional-mutually-exclusive` |

If the actual fn name differs, update the mapping or adjust Task 4's belt.yml scenario IDs before Task 9 commit.

- [ ] **Step 2: Insert `/// scenario: <id>` on the line directly above each `#[test]`**

Example edit pattern (1 per test fn):

```rust
/// scenario: belt-lint-valid-pipeline-ok
#[test]
fn lint_accepts_valid_pipeline() {
    // ... existing test body unchanged
}
```

Apply to all 5 test fns. Ensure one `/// scenario: <id>` line per fn, with indentation matching surrounding code.

- [ ] **Step 3: Run scenarios_contract positive test**

```bash
cargo test -p belt-core --test scenarios_contract scenarios_yml_and_rust_docs_match
```

Expected: orphan-rust for belt.yml now zero; orphan-yml may still have remaining (belt-core.yml not yet bound if Task 10 not done).

- [ ] **Step 4: Run existing belt cli_test to ensure no regression**

```bash
cargo test -p belt --test cli_test
```
Expected: 5/5 pass (behavior unchanged, doc-comment is non-semantic)

- [ ] **Step 5: Commit**

```bash
git add crates/belt/tests/cli_test.rs
git commit -m "test(belt): add /// scenario: doc-comments to 5 cli_test fns"
```

---

## Task 10: Add `/// scenario: belt-core-config-*` doc-comments to `crates/belt-core/tests/config_test.rs`

**Files:**
- Modify: `crates/belt-core/tests/config_test.rs`

**Test-strategy reference:** MV-14 / EP-03

- [ ] **Step 1: Identify 6 test fns + map to belt-core.yml IDs**

```bash
grep -B 0 -A 0 "^fn " crates/belt-core/tests/config_test.rs
```

Manual mapping (Task 5 scenarios.yml):

| Rust fn (confirm from source) | scenario ID |
|---|---|
| `parse_config_valid_toml` (or similar) | `belt-core-config-valid-toml-parses` |
| `parse_config_missing_file` | `belt-core-config-missing-file-yields-file-not-found` |
| `parse_config_invalid_toml` | `belt-core-config-invalid-toml-yields-config-parse` |
| `parse_config_missing_pipeline_file` | `belt-core-config-missing-pipeline-file-field-yields-config-parse` |
| `resolve_pipeline_path_relative` | `belt-core-config-resolves-relative-pipeline-path` |
| `resolve_pipeline_path_absolute` | `belt-core-config-preserves-absolute-pipeline-path` |

Adjust if fn names differ.

- [ ] **Step 2: Insert `/// scenario: <id>` above each `#[test]`**

Same pattern as Task 9. 6 insertions.

- [ ] **Step 3: Run scenarios_contract full**

```bash
cargo test -p belt-core --test scenarios_contract
```
Expected: all 9 tests pass (orphan-yml = 0, orphan-rust = 0, drift injection pass)

- [ ] **Step 4: Run belt-core config_test regression check**

```bash
cargo test -p belt-core --test config_test
```
Expected: 6/6 pass.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/tests/config_test.rs
git commit -m "test(belt-core): add /// scenario: doc-comments to 6 config_test fns"
```

---

## Task 11: Replace `thread::sleep` → `filetime::set_file_mtime` in `view_test.rs`

**Files:**
- Modify: `crates/belt-core/tests/view_test.rs`

**Test-strategy reference:** MV-19 / MV-20 / MV-31 / EP-04 / BV-04 / NFR-02 / ST-03 / CCS-05 accept

- [ ] **Step 1: Locate all `thread::sleep` occurrences**

```bash
grep -n "thread::sleep" crates/belt-core/tests/view_test.rs
```
Expected: 6 occurrences around lines 685, 687, 760, 762, 824, 900 (per S1 finding; confirm actual).

- [ ] **Step 2: Inventory surrounding assertions (identify strict vs weak ordering)**

For each `thread::sleep` block, read ±15 lines to determine:
- What file's mtime is being seeded
- What assertion compares this mtime to another (strict `<` or weak `<=`, or just "matches newest")

Record in a scratch list; strict ordering → delta >= 2s; weak → anything differ.

- [ ] **Step 3: Add filetime import at the top**

```rust
use filetime::{set_file_mtime, FileTime};
use std::time::{SystemTime, UNIX_EPOCH};
```

- [ ] **Step 4: Replace each `thread::sleep(Duration::from_millis(20))` with explicit `set_file_mtime`**

Pattern for strict ordering (most cases, use 2s delta):

**Before:**
```rust
fs::write(&file_a, b"a").unwrap();
std::thread::sleep(std::time::Duration::from_millis(20));
fs::write(&file_b, b"b").unwrap();
```

**After:**
```rust
fs::write(&file_a, b"a").unwrap();
fs::write(&file_b, b"b").unwrap();
let base = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_secs() as i64;
set_file_mtime(&file_a, FileTime::from_unix_time(base, 0)).unwrap();
set_file_mtime(&file_b, FileTime::from_unix_time(base + 2, 0)).unwrap();
```

Ensure assertion on `file_a.mtime() < file_b.mtime()` (or equivalent) remains **byte-identical** (MV-31).

Apply this transformation to all 6 occurrences. For each:
- Keep assertion lines unchanged (diff only adds `set_file_mtime` calls; removes `thread::sleep`).
- Delta = 2s for strict ordering, 1s delta for weak ordering (still decisive on 1s-granularity FS).

- [ ] **Step 5: Remove unused `std::thread` / `Duration` imports if no longer referenced**

```bash
grep -n "std::thread\|Duration::" crates/belt-core/tests/view_test.rs
```
If result empty, remove imports; else keep.

- [ ] **Step 6: Run view_test**

```bash
cargo test -p belt-core --test view_test
```
Expected: 41/41 pass.

- [ ] **Step 7: Run 100 consecutive loop (NFR-02)**

```bash
for i in {1..100}; do cargo test -p belt-core --test view_test --quiet || exit 1; done; echo OK
```
Expected: `OK` (no intermediate fail).

- [ ] **Step 8: Run cargo clippy**

```bash
cargo clippy -p belt-core --tests -- -D warnings
```
Expected: clean.

- [ ] **Step 9: Commit**

```bash
git add crates/belt-core/tests/view_test.rs
git commit -m "test(belt-core): replace thread::sleep with filetime::set_file_mtime (6 sites, 2s delta)"
```

---

## Task 12: Write `docs/features/2026-04-17-belt-test-foundation/audit-report.md`

**Files:**
- Create: `docs/features/2026-04-17-belt-test-foundation/audit-report.md`

**Test-strategy reference:** MV-15 / MV-16 / MV-32 / CCS-06 accept

- [ ] **Step 1: Capture frontmatter values**

```bash
AUDITED_AT=$(date -u +%Y-%m-%dT%H:%M:%SZ)
AUDITED_COMMIT=$(git rev-parse HEAD)
echo "AUDITED_AT=$AUDITED_AT"
echo "AUDITED_COMMIT=$AUDITED_COMMIT"
```

- [ ] **Step 2: Write audit-report.md**

```markdown
---
audited_at: <AUDITED_AT from Step 1>
audited_commit: <AUDITED_COMMIT from Step 1>
audit_template_version: v1
---

# belt-test-foundation F1 Pilot Audit Report

F1 pilot audit 結果。methodology (audit-template.md v1) を 3 pilot file に適用した judgement。

## crates/belt/tests/cli_test.rs (5 tests, all kept)

### lint_accepts_valid_pipeline
- judgement: kept
- scenario: belt-lint-valid-pipeline-ok
- rationale: happy path lint、scenario covers exit 0 + stderr "ok"

### lint_detects_duplicate_phase_id
- judgement: kept
- scenario: belt-lint-duplicate-phase-id-detected
- rationale: duplicate detection は lint の主要機能

### lint_rejects_invalid_yaml
- judgement: kept
- scenario: belt-lint-invalid-yaml-rejected
- rationale: parser error の CLI level lock

### lint_rejects_nonexistent_file
- judgement: kept
- scenario: belt-lint-nonexistent-file-rejected
- rationale: file I/O error lock

### lint_rejects_config_and_positional
- judgement: kept
- scenario: belt-lint-config-and-positional-mutually-exclusive
- rationale: argument mutual exclusion は breaking change 検知の sentinel

(実 fn 名が Task 9 mapping と異なる場合は正しい fn 名に修正)

## crates/belt-core/tests/config_test.rs (6 tests, all kept)

### parse_config_valid_toml
- judgement: kept
- scenario: belt-core-config-valid-toml-parses
- rationale: happy path toml parse

(他 5 test も同様、全て kept + scenario 紐付け)

## crates/belt-core/tests/feature_dev_refresh.rs (11 tests, all kept)

Lock pilot。本 file は shape lock test の代表として lock-ledger.md に entry 移送済。

- judgement (全 11 tests): kept
- rationale: lock test は behavior scenario ではなく shape を固定する特殊役割で、audit-template.md v1 で「judgement = kept without scenario ID」を許容 (implementation-coupling label でも obsolete-spec label でもない)

本 F1 では feature_dev_refresh.rs 自体に変更を加えない (MV-17)。lock-ledger.md entry で 11 test fn 名 + pipeline.yml shape dimensions + cross-coupling が記録される (MV-18)。

## Summary

- Total pilot audited: 22 test (5 + 6 + 11)
- kept: 22
- deleted: 0
- merged: 0
- abstracted: 0

F1 scope では全 kept。F2/F3 で本格 audit (他 belt-core / belt-agent test 全体) を実施する際は audit-template.md の Decision Tree + v1 9 reason label 集を使用する。pilot file が F1 → F2 間に touch されていれば re-audit (audit-template.md の re-audit trigger section 参照)。

## Cross-reference

- Template: `docs/testing/audit-template.md` (v1)
- Scenarios: `docs/testing/cli-behavior/{belt,belt-core}.yml`
- Lock ledger: `docs/testing/lock-ledger.md`
- Design: `docs/features/2026-04-17-belt-test-foundation/design.md` (commit 896d0a6)
- Test strategy: `docs/features/2026-04-17-belt-test-foundation/test-strategy.md` (commit 896d0a6)
```

- [ ] **Step 3: Verify frontmatter + structure**

```bash
grep -E "^(audited_at|audited_commit|audit_template_version):" docs/features/2026-04-17-belt-test-foundation/audit-report.md
```
Expected: 3 lines matching.

- [ ] **Step 4: Run scenarios_contract audit_template_version check**

```bash
cargo test -p belt-core --test scenarios_contract audit_template_version_v1_matches_expected
```
Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add docs/features/2026-04-17-belt-test-foundation/audit-report.md
git commit -m "docs(features): add F1 pilot audit report (22 tests kept)"
```

---

## Task 13: Final verification + MV-30 after diff

**Files:**
- Read: all modified files
- Create (transient): `.belt/runs/{run_id}/artifacts/mv30-after.txt`

**Test-strategy reference:** MV-21 / MV-22 / MV-23 / MV-24 / MV-27 / MV-28 / MV-30 final diff

- [ ] **Step 1: Run full workspace test (MV-21)**

```bash
cargo test --workspace
```
Expected: `test result: ok` all suites, total 387 (baseline) + 9 (scenarios_contract) = **396 pass**.

- [ ] **Step 2: Run workspace clippy (MV-22)**

```bash
cargo clippy --workspace -- -D warnings
```
Expected: clean (exit 0).

- [ ] **Step 3: Run workspace fmt check (MV-23)**

```bash
cargo fmt --all -- --check
```
Expected: clean (exit 0).

- [ ] **Step 4: Verify 5 existing lock tests unchanged (MV-24)**

```bash
git log --oneline HEAD...8eb1fa5 -- \
  crates/belt-core/tests/feature_dev_refresh.rs \
  crates/belt-core/tests/bug_fix_refresh.rs \
  crates/belt-core/tests/review_skills_refresh.rs \
  crates/belt-core/tests/shared_criteria_parity.rs \
  crates/belt-core/tests/shared_filter_parity.rs
```
Expected: empty (these 5 files unchanged in F1).

- [ ] **Step 5: Verify branch exists (MV-27)**

```bash
git branch --list feature/2026-04-17-belt-test-foundation
```
Expected: `* feature/2026-04-17-belt-test-foundation`

- [ ] **Step 6: Capture MV-30 after.txt + diff (MV-30)**

```bash
grep -rl "cli_test.rs\|config_test.rs\|feature_dev_refresh.rs" docs/ | sort > .belt/runs/019d9930-c7b9-7473-a9fd-70f96aa79812/artifacts/mv30-after.txt
diff .belt/runs/019d9930-c7b9-7473-a9fd-70f96aa79812/artifacts/mv30-before.txt \
     .belt/runs/019d9930-c7b9-7473-a9fd-70f96aa79812/artifacts/mv30-after.txt
```
Expected: diff shows **additions only** (new references in docs/features/2026-04-17-belt-test-foundation/* and docs/testing/audit-template.md). No removals/renames of the 30+ historical spec/plan references.

Verify explicitly:
```bash
diff .belt/runs/019d9930-c7b9-7473-a9fd-70f96aa79812/artifacts/mv30-before.txt \
     .belt/runs/019d9930-c7b9-7473-a9fd-70f96aa79812/artifacts/mv30-after.txt \
   | grep -E "^< " | wc -l
```
Expected: `0` (no lines removed from before.txt).

- [ ] **Step 7: Run scenarios_contract full suite as final gate**

```bash
cargo test -p belt-core --test scenarios_contract
```
Expected: 9/9 pass.

- [ ] **Step 8: No commit (verification-only task)**

Task 13 produces no new file commits. `mv30-after.txt` is under `.belt/` (gitignored).

---

## Must-Verify Checklist Coverage Map

| MV | Task(s) |
|---|---|
| MV-01 | Task 2 |
| MV-02 | Task 4 + Task 8 (parse) |
| MV-03 | Task 6 + Task 8 (parse) |
| MV-04 | Task 5 + Task 8 (parse) |
| MV-05 | Task 7 + Task 8 (ledger locks_file existence test) |
| MV-06 | Task 3 |
| MV-07 | Task 8 (9 test pass) + Task 13 (workspace test) |
| MV-08 | Task 8 (`load_scenarios` parse) |
| MV-09 | Task 8 (`scenarios_yml_and_rust_docs_match` orphan-yml side) + Tasks 9, 10 (doc-comment bind) |
| MV-10 | Task 8 (same test orphan-rust side) |
| MV-11 | Task 8 (`lock_ledger_locks_files_exist`) |
| MV-12 | Task 8 (7 drift injection tests) |
| MV-13 | Task 9 |
| MV-14 | Task 10 |
| MV-15 | Task 12 |
| MV-16 | Task 12 + Task 3 (label集合 subset check via audit-template.md) |
| MV-17 | Task 13 Step 4 (feature_dev_refresh.rs unchanged) |
| MV-18 | Task 7 (feature_dev_refresh entry with A/B/C) |
| MV-19 | Task 11 |
| MV-20 | Task 11 Step 7 (100-loop with `\|\| exit 1`) |
| MV-21 | Task 13 Step 1 |
| MV-22 | Task 13 Step 2 |
| MV-23 | Task 13 Step 3 |
| MV-24 | Task 13 Step 4 |
| MV-25 | Phase-design narrative note (already written pre-plan-phase; re-verified at Phase 6 code-review) |
| MV-26 | Phase-plan / phase-execute / phase-code-review narrative notes (out of scope for execute tasks; written by each phase automatically) |
| MV-27 | Task 13 Step 5 |
| MV-28 | pre-Task-1 (Phase 1 design-gate already verified baseline 387 pass) |
| MV-29 | Task 2 (README.md drift check against CLAUDE.md / AGENTS.md) |
| MV-30 | Task 1 (capture) + Task 13 Step 6 (diff) |
| MV-31 | Task 11 Step 4 + Task 13 scenarios_contract (no explicit test; human review of diff shows assertion lines unchanged) |
| MV-32 | Task 12 + Task 8 `audit_template_version_v1_matches_expected` |
| MV-33 | Task 3 (re-audit trigger section) |

---

## Given/When/Then Coverage for Input Parameters (PLAN-07)

Test-strategy.md の P1 〜 P6 各 input parameter に対する normal / boundary / abnormal / state-transition 4 category を `scenarios_contract.rs` (Task 8) + `view_test.rs` 既存 (Task 11 前後で挙動不変) でカバー:

| P | Normal | Boundary | Abnormal | State-transition |
|---|---|---|---|---|
| P1 scenarios.yml | `load_scenarios` pass (Task 8 Step 1) | belt-agent.yml `scenarios: []` 空配列 (Task 6) + 5/6 件配列 (Tasks 4/5) | `drift_*` 7 tests の malformed case (Task 8 Step 6) | Tasks 9/10 の doc-comment 付与 → orphan-yml 解消の transition |
| P2 Rust doc-comment | `drift_positive_single_line_doc_comment_matches` (Task 8) | 0-doc-comment (未 bind 状態で fail) + multi-doc-comment (future test case) | `drift_regex_rejects_typo_senario` + `drift_regex_rejects_single_slash_prefix` (Task 8) | Task 9 → Task 10 の順次 bind で orphan 解消 |
| P3 lock-ledger | `lock_ledger_locks_files_exist` pass (Task 8) | Task 7 で 6 entries (`feature_dev_refresh / bug_fix_refresh / review_skills_refresh / shared_criteria / shared_filter / scenarios_contract`) | (drift tests 対象外、human review) | Task 7 後に Task 8 の test で lock (transition) |
| P4 scenarios_contract | 9 test 全 pass (Task 13 Step 7) | 0 drift (正常 state) + 7 drift (injection) | drift injection 7 種 (Task 8 Step 6) | transient fail (Task 8 後、Task 10 前) → pass (Task 10 後) |
| P5 view_test filetime | Task 11 Step 6 で 41 test pass | mtime base / base+2s (strict) / base+1s (weak) (Task 11 Step 4) | Unix 前提外 FS は MVP scope 外 (CLAUDE.md) | Task 11 Step 4 の置換前後で assertion 不変 (MV-31) |
| P6 audit-template | Task 3 の 9 label enumerate、Task 12 で kept 判定 | 1 label 使用 (F1 では全 kept) + 9 label 全使用 (F2/F3) | audit-template.md に無い label 使用 → Task 8 `audit_template_version_v1_matches_expected` で間接検知 | v1 → v2 migration (future F2/F3 scope) で audit-template.md + scenarios_contract 同期更新 |

---

## Self-Review

- **Spec coverage**: 33 MVs 全て Task に mapping 済 (上表)。design.md の 9 deliverables は Tasks 2-12 に対応 (13 は verification)。
- **Placeholder scan**: 全 Task に完全な code / command / expected output 記載。「TBD」「TODO」「add appropriate error handling」「similar to Task N」のいずれも不使用。
- **Type consistency**: scenarios.yml schema field (id / category / severity / given / when / then / technique / preconditions / postconditions + scope top-level) は全 Task で一致。Rust 側 struct (`ScenariosFile` / `Scenario`) も Task 8 で定義後、他 Task で参照なし (閉じた定義)。`match_scenario_line` / `strip_block_comments` / `strip_string_literals` / `walk_rs_files` / `load_scenarios` / `all_scenario_ids` / `collect_rust_scenario_refs` / `repo_root` は全て Task 8 で定義 + 全 assertion で consistent に呼ばれる。

---

## Execution Handoff

Plan complete and saved to `docs/features/2026-04-17-belt-test-foundation/plan.md`. Execution will proceed via `belt:feature-dev` pipeline Phase 5 (execute) using `superpowers:subagent-driven-development` (dispatched by the orchestrator per plan task). See `docs/features/2026-04-17-belt-test-foundation/design.md` § Architecture + Must-Verify Checklist, and `test-strategy.md` § Must-Verify Mapping for each task's verification criteria.
