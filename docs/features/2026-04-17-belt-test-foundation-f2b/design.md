# belt-test-foundation (F2b) — Design

F2b は「belt の test 資産を audit する多段 feature 群 (F1 → F2a → F2b → F3)」の**第 2 段 (後半)**。F2a で完了した SSOT 拡充 + binding realization の上に、(1) Forward-to-F2b list の Decision Tree 適用 + label 確定、(2) belt-core 内 helper の `tests/common/mod.rs` 集約、(3) gate `git_clean` coverage gap の解消、(4) `expander_with_test.rs` tombstone 状態の integration test 化、(5) shape-lock entry `bug_fix_refresh.rs` の shape dimensions 本文化、(6) uri module の layered test duplication 解消、(7) engine Display 2 test の scenario 化 + parameterize を実施する。test-only feature、production code (`crates/*/src/**`) は全 module unchanged。

## Brainstorming 決議の総括

### 6 axes

| Axis | 決議 | 含意 |
|---|---|---|
| Scope | **Q1-A** | F2a forward 7 項目全て F2b で処理、split なし |
| Split | **Q2-A** | 単一 feature / 1 branch / 1 PR (`feature/2026-04-17-belt-test-foundation-f2b`) |
| Label-depth | **Q3-B** | forward list + F2b 作業中の副次発見に Decision Tree Q2-Q5 + 9 label 適用 |
| Helper-abstraction-timing | **Q4-A** | helper-first: `tests/common/mod.rs` 抽出を Duplication dedup に先行 |
| Shape-lock-batching | **Q5-A** | bug_fix_refresh.rs stub expansion は single commit |
| Restoration | **Q6-B** | git_clean coverage + expander_with integration test 2-3 本 + 副次発見吸収 |

### 5 implicit rules (S2 extraction)

| Rule | 決議 | 根拠 |
|---|---|---|
| 2-A (lock-ledger scope) | **bug_fix_refresh stub 本文化のみ** | 実態として 6 entries 既存、他 3 (review_skills / shared_criteria / shared_filter) は complete |
| 2-B (binary crate helper) | **F3 送り、belt-core/tests のみ対象** | F2a `belt-agent/tests/** = F3 scope` の延長、Cargo の tests/common/mod.rs は同 crate 内 only |
| 2-C (engine Display 2 test) | **keep + scenario 追加 + parameterize** | spec engine-guards.md:43 が `attempts/max_retries` ratio format を reference、CLI JSON reason は Display 層 regression 感度を cover せず |
| 2-D (uri inline 12 test) | **全量 integration 移植 + inline 全削除** | inline は `super::*` で public API のみ叩く、Q3 white-box exemption 成立せず、SSOT 化 |
| 2-E (parser_test vs model_test) | **layer 分離で両保持** | audit-template.md:52 記述の fn 名誤り、実 fn は layer 異なる complementary test |

### Execute approach: A (F2a-mirror)

F2a の Phase B (infrastructure-first) → Phase A (item work) → Phase C (audit-report) を踏襲。11 commit 想定 (Phase B0-B2 → A1-A5 → C1)。

## Architecture

F2a で確立した 3 層構造を継承、test-only 拡張:

```
Layer 1: SSOT (docs/testing/cli-behavior/)
    - belt-core.yml: 既存 scenarios + 拡張
      - uri category: 5 → 12 scenarios (2-D-a)
      - gate category: 9 → 13-14 scenarios (+4-5 git_clean)
      - error category: 3 → 4-5 scenarios (+1-2 Display format)
      - expander category: 4 → 6-7 scenarios (+2-3 with-substitution integration)

Layer 2: Binding (crates/belt-core/tests/scenarios_contract.rs)
    - F2a で set-based assertion + multiline raw string strip 済
    - F2b では mechanism 変更なし (binding consumer 側で scenario 追加/削除)

Layer 3: Binding realization + Helper consolidation
    - crates/belt-core/tests/common/mod.rs (new, re-export hub)
    - crates/belt-core/tests/common/helpers.rs (new, write_yaml/repo_root/fixture_path)
    - crates/belt-core/tests/common/narrative.rs (new, find_phase/find_produce/has_file_exists_gate/has_named_consume + 4 assert_* helpers)
    - crates/belt-core/tests/common/parity.rs (new, optional)
    - crates/belt-core/tests/uri_test.rs edit (+7 edge case tests)
    - crates/belt-core/src/uri.rs edit (inline #[cfg(test)] mod tests ブロック全削除)
    - crates/belt-core/tests/gate_test.rs edit (+4-5 git_clean tests)
    - crates/belt-core/tests/engine_test.rs edit (2 Display test に scenario + parameterize)
    - crates/belt-core/tests/expander_with_test.rs edit (+2-3 integration tests)
    - crates/belt-core/tests/{engine,view,lint,model,expander,bug_fix_refresh,review_skills_refresh,shared_filter_parity,artifact_when_field,scenarios_contract}_test.rs / feature_dev_refresh.rs edit (helper import 書き換え)

Layer 4: Documentation
    - docs/testing/lock-ledger.md: bug_fix_refresh.rs entry stub を feature_dev_refresh template 並みに expand (+35-40 lines)
    - docs/testing/audit-template.md: Duplication Candidates wording correction (parser_test.rs::parse_minimal_pipeline 誤記訂正、v1 unchanged patch)
    - docs/features/2026-04-17-belt-test-foundation-f2b/audit-report.md: F2b final judgment 集計
```

### Scope Boundary

**F2b 内**:
- engine_test.rs の 2 Display test scenario 化 + parameterize (item 1)
- uri integration 拡充 + inline 全削除 (item 2)
- gate git_clean coverage 追加 (item 3)
- tests/common/mod.rs + 4 helper module (item 4a, 4b, 4c)
- lock-ledger.md bug_fix_refresh.rs stub expansion (item 5)
- expander_with_test.rs integration test 2-3 本追加 (item 6)
- Decision Tree Q2-Q5 + 9 label 適用 + audit-report.md (item 7)
- audit-template.md Duplication Candidates wording correction (副次 patch)

**F2b 外 (F3 送り)**:
- belt-agent/tests/ (cli_test.rs 40 + e2e_test.rs 8) の audit
- belt/cli_test.rs + belt-agent/cli_test.rs の write_yaml variant B 統合 (cross-crate、Cargo tests/common 限界)
- engine_test regate_* vs belt-agent cli regate_* の cross-crate duplication
- engine_test verify_verdict_* vs belt-agent cli verify_* の同上
- view_test engine_enriched_status_* vs belt-agent cli status_* の同上

**将来 feature (F2b/F3 外)**:
- audit-template.md v1 → v2 bump (新 label 必要時)
- coverage tool (`tarpaulin` / `cargo-mutants`) 導入
- 未使用 dev-dep (`insta` / `pretty_assertions` / `rstest`) の活用

## Deliverables

| # | Path | Type | Purpose |
|---|------|------|---------|
| 1 | `crates/belt-core/tests/common/mod.rs` | **new** | helper re-export hub (`pub mod helpers; pub mod narrative;`), `#![allow(dead_code)]` preamble |
| 2 | `crates/belt-core/tests/common/helpers.rs` | **new** | `write_yaml` / `repo_root` / `fixture_path` 3 helpers (variant A 統一、signature 固定) |
| 3 | `crates/belt-core/tests/common/narrative.rs` | **new** | `find_phase` / `find_produce` / `has_file_exists_gate` / `has_named_consume` 4 helpers + `assert_narrative_produce_paths` / `assert_narrative_gate_paths` / `assert_narrative_accumulating_consumes` / `assert_non_narrative_phases_have_no_notes` |
| 4 | `crates/belt-core/tests/common/parity.rs` | **new, optional** | `read_workspace_file` helper (extract section logic は shared_filter_parity 側に閉じる) |
| 5 | `crates/belt-core/tests/engine_test.rs` | edit | write_yaml/fixture_path を common 経由に、Display 2 test に scenario doc-comment + parameterize |
| 6 | `crates/belt-core/tests/view_test.rs` | edit | fixture_path を common 経由に |
| 7 | `crates/belt-core/tests/lint_test.rs` | edit | write_yaml を common 経由に |
| 8 | `crates/belt-core/tests/expander_test.rs` | edit | write_yaml を common 経由に |
| 9 | `crates/belt-core/tests/expander_with_test.rs` | edit | integration test 2-3 本追加 (preamble 保持) |
| 10 | `crates/belt-core/tests/gate_test.rs` | edit | git_clean test 4-5 本追加 (scenarios 対応 doc-comment 付与) |
| 11 | `crates/belt-core/tests/model_test.rs` | edit (optional) | common import (helper 未使用なら touchless) |
| 12 | `crates/belt-core/tests/parser_test.rs` | edit (optional) | 同上 |
| 13 | `crates/belt-core/tests/artifact_when_field.rs` | edit | fixture_path を common 経由に |
| 14 | `crates/belt-core/tests/scenarios_contract.rs` | edit | repo_root を common 経由に |
| 15 | `crates/belt-core/tests/bug_fix_refresh.rs` | edit | repo_root + narrative helpers を common 経由に |
| 16 | `crates/belt-core/tests/feature_dev_refresh.rs` | edit | narrative helpers を common 経由に |
| 17 | `crates/belt-core/tests/review_skills_refresh.rs` | edit | repo_root を common 経由に |
| 18 | `crates/belt-core/tests/shared_filter_parity.rs` | edit | repo_root を common 経由に (optional: read_workspace_file) |
| 19 | `crates/belt-core/tests/shared_criteria_parity.rs` | edit (optional) | workspace_path を common 経由に |
| 20 | `crates/belt-core/tests/uri_test.rs` | edit | 5 → 12 test (7 edge case 追加) |
| 21 | `crates/belt-core/src/uri.rs` | edit | `#[cfg(test)] mod tests` (line 174-310) 全削除 |
| 22 | `docs/testing/cli-behavior/belt-core.yml` | edit | uri 5→12、gate 9→13-14、error 3→4-5、expander 4→6-7 scenarios |
| 23 | `docs/testing/lock-ledger.md` | edit | bug_fix_refresh.rs entry stub expansion (+35-40 行) |
| 24 | `docs/testing/audit-template.md` | edit | Duplication Candidates wording correction (v1 patch unchanged) |
| 25 | `docs/features/2026-04-17-belt-test-foundation-f2b/design.md` | new | 本 doc |
| 26 | `docs/features/2026-04-17-belt-test-foundation-f2b/test-strategy.md` | new | Phase 2 産物 |
| 27 | `docs/features/2026-04-17-belt-test-foundation-f2b/plan.md` | new | Phase 4 産物 |
| 28 | `docs/features/2026-04-17-belt-test-foundation-f2b/audit-report.md` | new | Phase 6 execute 産物、label 集計 + Q2-Q5 適用結果 |
| 29 | `.belt/runs/{run_id}/notes/phase-*.md` | new | 4 phase narrative notes (design / plan / execute / code-review) |

## Prerequisites

1. **F2a deliverables が stable**: merge 済 (`9f08676`)、216/218 Phase A test annotated、cumulative 408 workspace tests / 114 scenarios
2. **current branch**: `feature/2026-04-17-belt-test-foundation-f2b` (既存、F2a merge から派生)
3. **baseline `cargo test --workspace` = 408 pass**: F2a merge 時点、F2b 開始直前に再確認 (本 design commit 前)
4. **Rust toolchain**: `rust-toolchain.toml` で 1.94.1 固定済 (F2a と同)
5. **新規 dep ゼロ**: `Cargo.toml` / `Cargo.lock` unchanged、`tempfile` / `serde` / `thiserror` / `miette` / `serde-saphyr` 既存のみ使用
6. **scenarios_contract.rs の symmetric diff 機構 stable**: set-based assertion + raw-string strip + strip order invariant 全て F2a で fix 済、F2b で mechanism 変更不要
7. **audit-template.md v1 凍結**: reason label 9 固定、Decision Tree Q1-Q5 固定。F2b は wording correction (Duplication Candidates) のみ patch、v1 unchanged
8. **Cargo `tests/common/mod.rs` 慣例**: integration test file 扱いされず module として shared、ただし `#![allow(dead_code)]` で unused 警告抑制必須
9. **pilot file immutability 維持**: cli_test.rs / config_test.rs / feature_dev_refresh.rs の既存 test unchanged、F1 pilot 判定 stale 化させない (feature_dev_refresh.rs は narrative helper import のみ書き換え、既存 assertion body 不変)
10. **production code touch ゼロ**: `crates/*/src/**` 全 module unchanged、但し `src/uri.rs` の `#[cfg(test)] mod tests` ブロック削除は test コードなので `src/` 配下だが**実質 test 変更**、production runtime path には影響なし

## Impact Scope

### File-level

**new**:
- `crates/belt-core/tests/common/mod.rs` (hub, `pub mod helpers; pub mod narrative; [pub mod parity;]`)
- `crates/belt-core/tests/common/helpers.rs` (write_yaml + repo_root + fixture_path)
- `crates/belt-core/tests/common/narrative.rs` (4 helper fns + 4 assert_* fns)
- `crates/belt-core/tests/common/parity.rs` (optional, read_workspace_file)
- `docs/features/2026-04-17-belt-test-foundation-f2b/{design,test-strategy,plan,audit-report}.md`
- `.belt/runs/{run_id}/notes/phase-*.md`

**edit (test file, helper import 書き換え)**:
- `crates/belt-core/tests/engine_test.rs` (write_yaml + fixture_path + Display 2 test edit)
- `crates/belt-core/tests/view_test.rs` (fixture_path)
- `crates/belt-core/tests/lint_test.rs` (write_yaml)
- `crates/belt-core/tests/expander_test.rs` (write_yaml)
- `crates/belt-core/tests/artifact_when_field.rs` (fixture_path)
- `crates/belt-core/tests/scenarios_contract.rs` (repo_root)
- `crates/belt-core/tests/bug_fix_refresh.rs` (repo_root + 4 narrative helpers)
- `crates/belt-core/tests/feature_dev_refresh.rs` (4 narrative helpers, assertion body 不変)
- `crates/belt-core/tests/review_skills_refresh.rs` (repo_root)
- `crates/belt-core/tests/shared_filter_parity.rs` (repo_root + optional read_workspace_file)
- `crates/belt-core/tests/shared_criteria_parity.rs` (optional workspace_path)

**edit (test file, behavior 追加)**:
- `crates/belt-core/tests/uri_test.rs` (+7 edge case tests)
- `crates/belt-core/tests/gate_test.rs` (+4-5 git_clean tests)
- `crates/belt-core/tests/expander_with_test.rs` (+2-3 integration tests)

**edit (src 削除、production runtime 不変)**:
- `crates/belt-core/src/uri.rs` (`#[cfg(test)] mod tests` ブロック line 174-310 全削除)

**edit (docs/)**:
- `docs/testing/cli-behavior/belt-core.yml` (scenarios 拡張)
- `docs/testing/lock-ledger.md` (bug_fix_refresh entry expansion)
- `docs/testing/audit-template.md` (Duplication Candidates wording patch)

**non-impacted (明示)**:
- `crates/belt-agent/tests/**` (F3 scope)
- `crates/belt/tests/**` (F3 scope、belt binary cli_test.rs の variant B helper は touchless)
- `crates/*/src/**` excluding `src/uri.rs` test mod (production runtime unchanged)
- `Cargo.toml` / `Cargo.lock` / `.github/workflows/**`
- `docs/testing/README.md`
- `docs/superpowers/specs/**` の verbatim path 参照 (既存 link 維持、helper 抽出で file rename ゼロ)

### Module-level (behavior / contract 変化)

- **No production code change**: `src/uri.rs` の `#[cfg(test)] mod tests` 削除は cfg-gated、production binary に残留ゼロ
- **Test surface delta**: baseline 408 test → F2b 完了時点予測 **411-417** test
  - Item 1: ±0 (keep)
  - Item 2: -12 (inline delete) + 7 (integration 追加) = **-5**
  - Item 3: +4-5 (git_clean)
  - Item 4a/b/c: ±0 (helper dedup、test count 不変)
  - Item 5: ±0 (docs)
  - Item 6: +2-3 (integration)
  - Item 7: ±0 (Item 1/2/3/6 で吸収)
- **Scenario count delta**: baseline 114 → 127-132 (uri +7, gate +4-5, error +1-2, expander +2-3)
- **File count delta**: +4 new test file (common/ 以下), -0 (uri_test.rs keep, gate_test.rs keep, expander_with_test.rs keep), -1 (src/uri.rs #[cfg(test)] 削除だが file 自体は残る)

## Impact Analysis

### Reverse Dependencies

| caller | target | coupling | reason |
|---|---|---|---|
| `scenarios_contract.rs::scenarios_yml_and_rust_docs_match` | 全 `docs/testing/cli-behavior/*.yml` + 全 `crates/*/tests/*.rs` | direct | F2b で scenario 追加 + test 追加/削除を同一 commit で atomic 実施必須、symmetric diff が CI gate 本体 |
| `scenarios_contract.rs::lock_ledger_locks_files_exist` | `docs/testing/lock-ledger.md` の `locks-file:` 行 | direct | bug_fix_refresh entry expansion で `locks-file:` field 不変、assertion touchless |
| `belt-agent/tests/cli_test.rs::<phase_id/reason assertions>` | `belt-core/src/error.rs` の `#[error(...)]` 文面 | indirect (JSON 経由) | Item 1 Display parameterize で diagnostic code 層は無影響 |
| `crates/belt-core/src/uri.rs::BeltUri::parse/to_string` | `uri_test.rs` 12 integration tests | direct | Item 2 inline 削除後、uri_test.rs のみが uri module の behavior lock、regression 感度集中 |
| `crates/belt-core/src/gate.rs::execute_git_clean` | `gate_test.rs` | direct | Item 3 で初の integration test が追加、以降 gate module 改変時の regression detector |
| `feature_dev_refresh.rs::find_phase/find_produce/has_file_exists_gate` | `tests/common/narrative.rs` | direct | Item 4b 抽出後、narrative 4 helper の唯一の caller が refresh 2 file + future 追加 refresh file |
| `docs/superpowers/specs/**` の 30+ verbatim path 参照 | `cli_test.rs`, `config_test.rs`, `feature_dev_refresh.rs` | weak (historical) | F2b で rename ゼロ、新 file は `tests/common/*` のみ、既存 link 不変 |

### Shared State

| kind | constraint | usage |
|---|---|---|
| FS (scenarios yml) | belt-core.yml が uri/gate/error/expander category の scenarios を集積 | controller が scenario merge 責務、subagent は write しない |
| FS (test common module) | `tests/common/mod.rs` は Cargo integration test 慣例で全 test binary が include、unused helper で warning 出現する file も `#![allow(dead_code)]` で抑止 | common/mod.rs 先頭で `#![allow(dead_code)]` 必須 |
| scenarios_contract symmetric diff | yml ID 集合と rust doc ID 集合の完全一致 (set-based) | uri inline 削除 + integration 追加 + yml 拡張を atomic commit で同期 |
| lock-ledger.md format | `locks-file:` 行だけが機械検証、他 field (`test-fn-count:` etc.) は documentation | bug_fix_refresh entry expansion は documentation-only、assertion touchless |
| git CLI availability | gate `git_clean` integration test が `git status --porcelain` を exec | CI runner で git 既存前提 (production code で既に依存)、probe 不要 |
| Cargo `#[cfg(test)] mod tests` in src/uri.rs | production binary に含まれず test binary のみに include | src/uri.rs edit は production runtime unchanged、削除による production impact ゼロ |

### Implicit Contracts

| file:line (推定) | dependency | violation impact |
|---|---|---|
| `scenarios_contract.rs::strip_string_literals` then `strip_block_comments` 順序 | F2a Phase B2 fix + drift test 4 本で lock 済 | F2b では mechanism 変更なし、test helper 内の raw string 使用は drift test で自動検証 |
| `BeltError::VerifyRequired` / `MaxRetriesExceeded` の `#[error("...")]` 文面 (error.rs:30,34) | spec engine-guards.md:43 の `attempts/max_retries` ratio format | Item 1 で scenario 化 + parameterize assertion (`starts_with` / formatter interpolation) で rewording 耐性 + regression 感度両立 |
| `BeltUri::parse` の 9 error variant (`MissingScheme` 〜 `PathTraversal` 〜 `Malformed`) | 現 inline 12 test が variant 全 cover | Item 2 全量移植後、uri_test.rs が唯一の variant coverage、削除 = 移植漏れ検知 loss リスク |
| `GateCheck::GitClean` XNOR logic (`is_clean == expect_clean`) | `gate.rs:271` の現実装 | Item 3 test で clean/dirty × expect_clean/expect_dirty の 4 組合せ + git error 1 の 5 variant を lock |
| `write_yaml(dir: &TempDir, name: &str, content: &str) -> PathBuf` signature | 3 belt-core file で identical | Item 4a 抽出時、common/helpers.rs の signature を variant A (File::create + write_all) に統一、3 call site 全て動作変更なし |
| `expand_pipeline` public API の with-substitution semantics | memory `feedback_expander_parent_scope_rule.md` の parent-inherit 値 rewrite 禁止 rule | Item 6 integration test で parent-scope isolation を公開 API 経由で lock、inline unit test と重複しない scope |
| `lock-ledger.md` の `locks-file:` field only machine-checked | `scenarios_contract.rs::lock_ledger_locks_files_exist` の trim+strip_prefix | Item 5 expansion は documentation fields のみ追加、machine check 影響ゼロ |

### Side Effect Risks

| severity | trigger | impact / 対策 |
|---|---|---|
| high | `tests/common/mod.rs` 抽出時に `use` path 書き換え漏れで某 test binary が compile fail | Phase B0 commit で **全 9+ file を同一 commit で書き換え**、cargo test --workspace で regression detection。subagent 分担は禁止、controller 単独作業 |
| high | uri integration 移植時に 1-2 test 漏れで edge case coverage silent loss | 移植表 (12 inline → 12 integration) を subagent が作成、controller が 1:1 対応確認。scenarios_contract の symmetric diff は yml ↔ rust doc-comment のみ検証、**test 内容の semantic 検査はしない** → 人間 review 必須 |
| high | `src/uri.rs` の `#[cfg(test)] mod tests` 削除で serde impl (line 161-172) が unit test coverage ゼロ | Item 2 で integration 側に `to_string_roundtrip_all_variants` 対応 scenario + test を必ず追加 |
| high | F2b commit sequence 内で yml-先行 / test-先行 diverge | 各 atomic commit で belt-core.yml + 対応 test file を同時 stage、`cargo test --test scenarios_contract` を commit 直前に実行 |
| medium | Item 3 gate git_clean test で `tempdir + git init` を要するが helper が無い | common/helpers.rs に `fn git_init_tempdir() -> TempDir` を追加、または gate_test.rs 内で local helper (Item 3 commit scope) |
| medium | bug_fix_refresh.rs narrative helper import で narrative helpers が feature_dev_refresh の assertion body と微小 drift を起こす | Phase B1 commit で narrative helpers の signature を feature_dev_refresh 側の既存 inline signature と完全一致、assertion body unchanged を PR review で verify |
| medium | lock-ledger.md bug_fix_refresh entry expansion で shape dimensions enumeration が bug_fix_refresh.rs 実 test fn 集合と drift | expansion は test fn 実集合を列挙する template に従い、bug_fix_refresh.rs の `#[test]` 行 grep 結果をそのまま転記 (F1 feature_dev_refresh entry と同構造) |
| medium | audit-template.md Duplication Candidates wording patch が v1 semantics 変更と誤解される | patch は "parser_test.rs::parse_minimal_pipeline (誤記、実 fn は parse_pipeline_from_file)" 等の訂正注記のみ、Decision Tree / reason labels は完全 unchanged、v1 unchanged |
| low | expander_with_test.rs integration 2-3 test が既存 inline 26 test と内容重複 | Q6-B の design 意図 "unit は per-field semantic、integration は public API end-to-end (YAML file → expand → assert ExpandedPhase shape)" を test 名と doc-comment で明示分離 |
| low | write_yaml variant A と variant B の差分が今後 production binary crate test に影響 | F2b では variant A を belt-core/tests のみに統一、variant B は binary crates 内 local helper のまま保持 (F3 で整理) |
| low | `tests/common/mod.rs` `#![allow(dead_code)]` が新規 helper の dead code 検出を silent に rp | helper 追加時は少なくとも 1 caller を同 commit で用意、CI clippy は caller 側 file で dead code 検出 |
| low | F1 pilot 3 file (cli_test.rs / config_test.rs / feature_dev_refresh.rs) の touch | feature_dev_refresh.rs は narrative helper import のみ書き換え、**assertion body 不変**。cli_test.rs / config_test.rs は F2b で touchless。audit-template.md の "Trigger 対象外" clarification (F2a) により re-audit 不発動 |

### F2b baseline sanity check

- `cargo test --workspace` = 408 pass (F2a merge 時点、本 design commit 前に再確認)
- `cargo clippy --workspace -- -D warnings` = clean
- `cargo fmt --all -- --check` = clean
- `git log --since="2026-04-17T10:20:34Z"` (F2a audit_at) -- pilot 3 file が empty (trigger 非発動確認)

## Must-Verify Checklist

### 基本構造 (deliverable 存在と shape)

- [ ] **MV-01**: `crates/belt-core/tests/common/mod.rs` が新規作成、`#![allow(dead_code)]` preamble + `pub mod helpers; pub mod narrative;` [+ `pub mod parity;`] 記述
- [ ] **MV-02**: `common/helpers.rs` に `write_yaml` / `repo_root` / `fixture_path` 3 helpers が variant A signature で export 済み
- [ ] **MV-03**: `common/narrative.rs` に 4 helper fns (`find_phase` / `find_produce` / `has_file_exists_gate` / `has_named_consume`) + 4 assert_* fns が export
- [ ] **MV-04**: 全 helper caller (belt-core/tests/ 内 9+ file) が common 経由 import に変更
- [ ] **MV-05**: `crates/belt-core/src/uri.rs` line 174-310 の `#[cfg(test)] mod tests` ブロック全削除、production code (line 1-172) unchanged
- [ ] **MV-06**: `crates/belt-core/tests/uri_test.rs` が 5 → 12 test、7 edge case 追加 (parse_unknown_selector / parse_empty_pipeline / parse_empty_run_id / parse_empty_path / parse_absolute_path_rejected / parse_workspace_missing_latest / to_string_roundtrip_all_variants)
- [ ] **MV-07**: `crates/belt-core/tests/gate_test.rs` に git_clean test 4-5 本追加 (clean+expect_clean / dirty+expect_dirty / clean+expect_dirty / dirty+expect_clean / git command error)
- [ ] **MV-08**: `crates/belt-core/tests/engine_test.rs` の `error_verify_required_message` / `error_max_retries_exceeded_message` 2 test に scenario doc-comment 付与 + assertion parameterize (`starts_with` / `format!()` 化)
- [ ] **MV-09**: `crates/belt-core/tests/expander_with_test.rs` に integration test 2-3 本追加 (既存 preamble コメント保持)
- [ ] **MV-10**: `docs/testing/lock-ledger.md` の bug_fix_refresh.rs entry stub が feature_dev_refresh.rs template 並みに expand (+35-40 行、test-fn-count + 9 shape dimensions)
- [ ] **MV-11**: `docs/testing/audit-template.md` の Duplication Candidates 節に wording correction patch (parser_test.rs::parse_minimal_pipeline 誤記を parse_pipeline_from_file に訂正)、audit_template_version: v1 unchanged
- [ ] **MV-12**: `docs/features/2026-04-17-belt-test-foundation-f2b/audit-report.md` frontmatter に `audited_at / audited_commit / audit_template_version: v1` 記載、label 集計 + Decision Tree 適用結果 + Forward-to-F3 list

### SSOT ↔ Rust binding 機械検証

- [ ] **MV-13**: `cargo test -p belt-core --test scenarios_contract` が全 pass (F2a baseline 14 test + F2b 無変更)
- [ ] **MV-14**: belt-core.yml の uri category = 12 scenarios、対応 doc-comment が uri_test.rs 12 test に付与
- [ ] **MV-15**: belt-core.yml の gate category = 13-14 scenarios、対応 doc-comment が gate_test.rs (既存 22 + 新規 4-5) に付与
- [ ] **MV-16**: belt-core.yml の error category = 4-5 scenarios、うち 1-2 が Display format lock (engine_test.rs の 2 Display test)
- [ ] **MV-17**: belt-core.yml の expander category = 6-7 scenarios、うち 2-3 が expander_with_test.rs の新規 integration test 向け
- [ ] **MV-18**: `lock_ledger_locks_files_exist` assert が全 `locks-file:` 行で pass、bug_fix_refresh entry expansion 後も 4 shape-lock file 全実在

### Helper consolidation correctness

- [ ] **MV-19**: `common/helpers.rs::write_yaml` 呼び出し site (engine/lint/expander_test.rs) で既存 assertion body unchanged、test count 不変
- [ ] **MV-20**: `common/helpers.rs::repo_root` 呼び出し site (scenarios_contract / review_skills_refresh / bug_fix_refresh / shared_filter_parity) で変数型 `PathBuf` signature 不変
- [ ] **MV-21**: `common/helpers.rs::fixture_path` 呼び出し site (engine/view/artifact_when_field) で subdirectory `tests/fixtures/<name>` 解決 behavior 不変
- [ ] **MV-22**: `common/narrative.rs` の 4 helpers を feature_dev_refresh / bug_fix_refresh 2 file で共通利用、assertion body unchanged
- [ ] **MV-23**: shape-lock test 4 file (bug_fix_refresh / review_skills_refresh / shared_criteria_parity / shared_filter_parity) の test fn 名 + `#[test]` 数が F2a merge 時点と同数 (helper import 以外は touchless)

### Item-level behavior correctness

- [ ] **MV-24**: Item 1 Display 2 test の parameterize assertion: `error_verify_required_message` が `msg.starts_with("verify required for phase '")` 以上の semantic 検証、literal `"verify required"` 固定依存を除去
- [ ] **MV-25**: Item 1 Display 2 test: `error_max_retries_exceeded_message` が `msg.contains(&format!("{attempts}/{max_retries}"))` で動的 format 検証、literal `"3/3"` 固定依存を除去
- [ ] **MV-26**: Item 2 uri integration 12 test が inline 12 test の semantic coverage を 1:1 再現 (移植表で確認)
- [ ] **MV-27**: Item 3 gate git_clean 4-5 test が clean/dirty 両 branch + expect_clean/expect_dirty XNOR + git command error の 5 variant をカバー
- [ ] **MV-28**: Item 6 expander_with_test.rs integration test が **public API `expand_pipeline`** (parse_pipeline 経由または直接) を entry point とし、`substitute_arg_in_value` 等 private API を直接叩かない (unit test との layering 分離)

### Decision Tree / Label application

- [ ] **MV-29**: engine Display 2 test → kept (Q5 路由)、label なし、新 scenario `belt-core-error-display-preserves-phase-id-and-counter` (or similar) を belt-core.yml に追加
- [ ] **MV-30**: uri inline 5 overlap test → `redundant-with-<integration-test-id>` label + delete
- [ ] **MV-31**: uri inline 7 edge case test → Q5 経由 kept (scenario 追加 + integration 移植後 delete)、label なし
- [ ] **MV-32**: audit-report.md に 9 label の使用頻度 summary、使用された label と使用されなかった label を明記 (v2 bump 必要性の signal)
- [ ] **MV-33**: parser_test.rs::parse_pipeline_from_file vs model_test.rs::parse_minimal_pipeline は **keep both** 判定、audit-report.md に layer 分離根拠記録、audit-template.md wording correction と整合

### 一貫性 / regression

- [ ] **MV-34**: `cargo test --workspace` 全 pass (predicted 411-417)
- [ ] **MV-35**: `cargo clippy --workspace -- -D warnings` clean (`dead_code` 警告は common/mod.rs の preamble で抑止)
- [ ] **MV-36**: `cargo fmt --all -- --check` clean
- [ ] **MV-37**: pilot 3 file (cli_test.rs / config_test.rs / feature_dev_refresh.rs) の既存 test unchanged (Phase B0-C 全 commit で existing test body が touch されていないこと `git diff` で確認)
- [ ] **MV-38**: shape-lock 4 file (bug_fix_refresh / review_skills_refresh / shared_criteria_parity / shared_filter_parity) の test count 不変、assertion body 不変 (helper import のみ書き換え)
- [ ] **MV-39**: production code (`crates/*/src/**` excluding `src/uri.rs` 削除部分) unchanged、`git diff` で新規変更ゼロ

### Narrative notes

- [ ] **MV-40**: `.belt/runs/{run_id}/notes/phase-{design,plan,execute,code-review}.md` が各 phase 完了時作成、frontmatter (`phase:` / `run_id:`) + 4 節 (Decisions / Concerns / Directives / Observations)

### worktree / workflow

- [ ] **MV-41**: `feature/2026-04-17-belt-test-foundation-f2b` branch 存在 (`git branch --list`)、current branch 一致
- [ ] **MV-42**: baseline `cargo test --workspace` = 408 pass を design commit 前に再確認
- [ ] **MV-43**: F2b pipeline args: `{codex: true, e2e: false}`、monkey-test / dogfood skip

### doc-drift 予防

- [ ] **MV-44**: `docs/testing/README.md` の docs/ 構造宣言が F2b 変更 (tests/common/*.rs 新設、audit-template wording patch) と矛盾しない、必要なら README 側も同 commit で更新 (想定: unchanged)
- [ ] **MV-45**: `docs/superpowers/specs/**` の 30+ verbatim path 参照が F2b で破壊されていない (rename ゼロ、新 file は `tests/common/*` のみ)
- [ ] **MV-46**: audit-template.md wording patch がクロス doc (F1/F2a design.md / audit-report.md、lock-ledger.md) と矛盾しない (Decision Tree Q1-Q5 + reason label 9 + shape-lock exception 全 unchanged)

## Test Perspectives

F2b deliverable を input parameter として Normal / Boundary / Abnormal / State-transition の 4 観点で test 対象を列挙。Phase 2 (test-scenarios) で `test-strategy.md` に expand、Phase 6 execute 中の各 test fn が assertion 化する。

### P1: common/helpers.rs 3 helper の抽出

| 観点 | 対象 case |
|---|---|
| Normal | write_yaml(&tempdir, "p.yml", "name: x\nversion: 1") が `<tempdir>/p.yml` を返し内容一致 / repo_root() が CARGO_MANIFEST_DIR/../.. を返す / fixture_path("f.yml") が tests/fixtures/f.yml を返す |
| Boundary | content = "" (empty string) / name = 最大長 255 chars / tempdir が nested subdir |
| Abnormal | write_yaml で dir が非存在 (panic via expect)、repo_root が CARGO_MANIFEST_DIR 未設定 (env var missing、test 環境で発生しない) |
| State-transition | 同 tempdir で write_yaml を 2 回呼ぶと overwrite 動作 |

### P2: common/narrative.rs 4 helper + 4 assert_*

| 観点 | 対象 case |
|---|---|
| Normal | find_phase が存在する phase_id を返す、find_produce が Artifact を返す、has_file_exists_gate が `file_exists` gate を持つ phase で true |
| Boundary | 6 narrative phase を持つ pipeline で全 phase 対応 artifact を全列挙できる / 0 narrative phase (空 pipeline) で assert_narrative_* fn が空集合対応 |
| Abnormal | find_phase が非存在 phase_id で panic (expect) / find_produce が非存在 artifact 名で panic |
| State-transition | assert_narrative_accumulating_consumes で phase 追加 → consumes 集合が単調増加、2 phase 飛ばし (pipeline mutation) で assertion fail |

### P3: uri_test.rs 12 test (5 overlap + 7 edge case)

| 観点 | 対象 case |
|---|---|
| Normal | 3 selector variant happy path (Latest / WorkspaceLatest / Run)、to_string roundtrip 3 variant |
| Boundary | uuid v7 format、workspace 名 single char、path 最短 / 最長 |
| Abnormal | MissingScheme (https://) / UnknownSelector / EmptyPipeline / EmptyRunId / EmptyPath / PathTraversal / AbsolutePath / Malformed WorkspaceLatest |
| State-transition | parse → to_string → parse の 2 回 roundtrip で invariant 維持 |

### P4: gate_test.rs git_clean 4-5 test

| 観点 | 対象 case |
|---|---|
| Normal | clean git repo + expect_clean=true → pass / dirty repo + expect_clean=false → pass |
| Boundary | clean repo 直後に git add した file を untrack (dirty detection precision) / empty repo (no commits) の clean 判定 |
| Abnormal | tempdir が git repo でない → `git status` fail → passed=false + detail "failed to run git" / git コマンド非存在 (PATH 外) も同 fail path |
| State-transition | clean → touch file → dirty の状態遷移を 1 test 内で連続確認 |

### P5: engine Display 2 test scenario 化 + parameterize

| 観点 | 対象 case |
|---|---|
| Normal | `BeltError::VerifyRequired { phase_id: "build" }` → msg starts_with "verify required for phase 'build'" / `MaxRetriesExceeded { phase_id, attempts: 3, max_retries: 3 }` → msg contains `format!("{attempts}/{max_retries}")` |
| Boundary | phase_id 最長 255 chars / attempts/max_retries が `0/0` (境界値) / `u32::MAX/u32::MAX` |
| Abnormal | phase_id が empty string ("") での format、Unicode/emoji 含 phase_id |
| State-transition | Display format 文面変更 (rewording) で test 失敗してはならない semantic invariant: phase_id が msg に含まれる && `{attempts}/{max_retries}` format が含まれる |

### P6: expander_with_test.rs 2-3 integration test

| 観点 | 対象 case |
|---|---|
| Normal | YAML ファイル parse → expand_pipeline → with 値が sub_phase args に正しく propagate |
| Boundary | with 値が bool / null / string 型 / empty string、1 sub-phase / 10 sub-phase |
| Abnormal | with が parent arg 名と collision、循環参照 yml (uses: self) |
| State-transition | parent arg を sub で override (scope isolation)、memory `feedback_expander_parent_scope_rule.md` の rule 遵守 |

### P7: lock-ledger bug_fix_refresh entry expansion

| 観点 | 対象 case |
|---|---|
| Normal | entry が feature_dev_refresh template と同粒度 (test-fn-count + 9 shape dimensions + 2 cross-coupling) |
| Boundary | test-fn-count: 19 と bug_fix_refresh.rs の `#[test]` 実数一致 (実数検証はなし、documentation) |
| Abnormal | entry が stub 状態のまま残存 (F1 placeholder 文言が削除されずに残る) |
| State-transition | 将来 bug_fix_refresh.rs に test 追加時に entry test-fn-count が stale 化、human update needed (machine check 非対象) |

### P8: audit-report.md label 集計

| 観点 | 対象 case |
|---|---|
| Normal | 全 audit 対象 test に label or kept judgment、frontmatter version match |
| Boundary | 9 label 全使用 (または限定使用) の明示、F2a 踏襲の kept 主体 |
| Abnormal | label が v1 enumeration 外の命名 (新 label 必要なら v2 bump signal) |
| State-transition | F2a forward list → F2b judgment → F3 forward list の連鎖整合 |

### Non-Functional Requirements

| 特性 | 要件 | 計測 |
|---|---|---|
| Performance | scenarios_contract.rs 全 test が < 2 秒、gate git_clean test × 5 個で git spawn 合計 < 5 秒 | `cargo test --test scenarios_contract --test gate_test -- --nocapture` 実時間 |
| Maintainability | F3 作業者が F2b audit-report.md + lock-ledger の expanded bug_fix_refresh entry を読んで belt-agent audit を開始可能 | handover で referrable |
| Determinism | F2b 追加 test 全 deterministic (時間/race 依存なし)、git_clean test は tempdir 内で完結、bash loop 50 回 pass | `for i in {1..50}; do cargo test --workspace \|\| exit 1; done; echo OK` |
| Portability | F1/F2a と同条件、git CLI が PATH 上 (CI runner 前提) | release.yml cross build + local test |
| Security | docs + test-only、production code unchanged、新 dep ゼロ | `cargo audit` / `cargo deny` 差分なし |

### Quality Bar 適合チェック

- 全 input parameter (P1-P8) に Normal / Boundary / Abnormal / State-transition 各 1 件以上 cover
- git_clean 4-5 test が clean/dirty × expect × error variant で matrix 充足
- uri integration 12 test が inline 12 test の strict superset (7 edge case 追加)
- helper 抽出後の caller file 全数で既存 assertion body unchanged を確認

## Execute Strategy

### Approach A (F2a-mirror) の 3 phase 構造

**Phase B (sequential, controller): infrastructure-first**

- **B0**: `crates/belt-core/tests/common/mod.rs` + `common/helpers.rs` 新設、`write_yaml` / `repo_root` / `fixture_path` 3 helpers 抽出 + 9+ file で import 書き換え (1 commit)
  - Cargo 慣例確認: `tests/common/mod.rs` は integration test binary ではなく mod として include、`#![allow(dead_code)]` 必須
  - variant A (belt-core 系) に統一、binary crate 系 variant B は touchless
  - touched files: `common/{mod,helpers}.rs` (new) + `engine_test.rs` / `view_test.rs` / `lint_test.rs` / `expander_test.rs` / `artifact_when_field.rs` / `scenarios_contract.rs` / `bug_fix_refresh.rs` / `review_skills_refresh.rs` / `shared_filter_parity.rs` [+ `shared_criteria_parity.rs` optional]

- **B1**: `common/narrative.rs` 新設、4 helpers + 4 assert_* fns export (1 commit)
  - feature_dev_refresh / bug_fix_refresh の narrative helpers を common 経由に書き換え
  - shape-lock test の assertion body 不変、helper import のみ変更
  - touched files: `common/narrative.rs` (new) + `feature_dev_refresh.rs` + `bug_fix_refresh.rs`

- **B2** (optional): `common/parity.rs` 新設、`read_workspace_file` helper 抽出 (1 commit、scope 縮小可)
  - shared_criteria / shared_filter の共通部分
  - extractor logic (section bullet parse) は shared_filter_parity 側 local 保持

**Phase A (sequential or parallel, item work)**

- **A1 (item 3)**: gate git_clean scenario (4-5) + test (4-5) 追加、belt-core.yml update (1 commit)
  - clean+expect_clean / dirty+expect_dirty / clean+expect_dirty / dirty+expect_clean / git command error
  - `tempfile::tempdir + Command::new("git").arg("init")` で git repo tempdir 作成

- **A2 (item 6)**: expander_with_test.rs integration test 2-3 追加、belt-core.yml expander category 拡張 (1 commit)
  - (a) string 値 with-substitution integration、(b) bool/null 値 with-substitution integration、(c) parent-scope isolation (feedback_expander_parent_scope_rule.md rule lock)
  - existing 17-line preamble 保持

- **A3 (item 1)**: engine_test.rs 2 Display test に scenario doc-comment 付与 + assertion parameterize、belt-core.yml error category +1-2 (1 commit)
  - `starts_with` / `format!()` で literal 依存除去
  - scenario id: `belt-core-error-display-verify-required-preserves-phase-id`、`belt-core-error-display-max-retries-preserves-phase-id-and-counter`

- **A4 (item 2)**: uri integration 7 edge case 追加、belt-core.yml uri category 5→12、src/uri.rs inline 12 test 全削除 (1 commit、atomic)
  - 移植表 (12 inline → 12 integration) で 1:1 対応確認
  - scenarios_contract の symmetric diff が自動検証

- **A5 (item 5)**: lock-ledger.md bug_fix_refresh.rs entry stub expansion (1 commit)
  - 19 test-fn-names 列挙 (A block)
  - 9 shape dimensions 列挙 (B block、feature_dev_refresh template 踏襲)
  - cross-coupling 2 bullets 既存維持

**Phase C (post-work)**

- **C1**: audit-report.md 作成、audit-template.md Duplication Candidates wording patch、narrative notes 整備 (1 commit)
  - frontmatter: `audited_at`, `audited_commit`, `audit_template_version: v1`
  - label 集計: Q5 kept (engine Display 2 + uri 7 edge 経由 via integration) + redundant-with-X (uri 5 overlap)
  - Forward-to-F3 list: belt-agent/tests/, cross-crate duplication (regate_*, verify_verdict_*, status_*), binary crate helper unification
  - audit-template.md patch: parser_test.rs 誤記訂正 notation 追記、Decision Tree / labels unchanged (v1 unchanged)

### Commit 粒度 (目安 9-10 commits)

| # | commit scope | subject 例 |
|---|---|---|
| 1 | Phase B0 | `test(belt-core): extract write_yaml/repo_root/fixture_path to tests/common` |
| 2 | Phase B1 | `test(belt-core): extract narrative helpers to tests/common/narrative` |
| 3 | Phase B2 (optional) | `test(belt-core): extract parity helper to tests/common/parity` |
| 4 | Phase A1 | `test(belt-core): add gate git_clean coverage (4-5 tests, 4-5 scenarios)` |
| 5 | Phase A2 | `test(belt-core): add expander_with integration tests (2-3 tests, 2-3 scenarios)` |
| 6 | Phase A3 | `test(belt-core): scenario-promote engine Display tests with parameterized assertions` |
| 7 | Phase A4 | `test(belt-core): migrate uri inline tests to integration (7 edge cases + 5 overlap)` |
| 8 | Phase A5 | `docs(testing): expand lock-ledger.md bug_fix_refresh entry with 9 shape dimensions` |
| 9 | Phase C1 | `docs(features): add F2b audit report + audit-template wording correction` |

Phase B2 skip なら 8 commits。

### Atomicity 原則

- 各 commit で `cargo test --workspace` / `cargo clippy --workspace -- -D warnings` / `cargo fmt --all -- --check` 全 green
- Phase A の scenario 追加 + test 追加 + (Item 2 の inline delete) は各 commit 内で atomic (scenarios_contract.rs symmetric diff が CI gate)
- pilot 3 file (cli_test.rs / config_test.rs / feature_dev_refresh.rs) の既存 test body は全 commit で touchless (helper import のみ feature_dev_refresh が touch)

### Concurrent write 回避

- Phase B0/B1 は controller 単独、subagent 分担禁止 (複数 file 同時書き換えの stage conflict 回避)
- Phase A は item 毎 independent、parallel subagent 可だが F2a の経験から sequential が predictable
- worktree 分離: `feature/2026-04-17-belt-test-foundation-f2b` branch の既存 worktree 内で完結

## Non-Goals

**F3 送り (F2b scope 外、明示的に forward)**:
- `belt-agent/tests/` (cli_test.rs 40 + e2e_test.rs 8) の audit
- `belt-agent.yml` の 6 subcommand (init/next/verify/regate/step/status) JSON contract scenarios 列挙
- cross-crate duplication 統合:
  - `engine_test regate_*` (14 test) vs `belt-agent cli regate_*` (11 test)
  - `engine_test verify_verdict_*` vs `belt-agent cli verify_*`
  - `view_test engine_enriched_status_*` vs `belt-agent cli status_*`
- binary crate helper unification (`belt/cli_test.rs` + `belt-agent/cli_test.rs` の write_yaml variant B)
- Cargo workspace 横断 helper (現状 Cargo の `tests/common/mod.rs` は同 crate 内限定、cross-crate には proc-macro or proto build.rs が必要)

**将来 feature (F2b/F3 全て外)**:
- audit-template.md v1 → v2 bump (新 reason label 必要時)
- coverage tool (`tarpaulin` / `cargo-mutants`) 導入
- 未使用 dev-dep (`insta` / `pretty_assertions` / `rstest`) の活用
- `docs/superpowers/specs/` の 30+ verbatim file-path 参照の機械検証拡張
- monkey-test / dogfood pipeline 実行用の別系統 scenarios (CLI scenarios は monkey-test 対象外)

**F2b で明示的にやらないこと (scope creep 防止)**:
- production code touch (全 `crates/*/src/**` module は unchanged、src/uri.rs の `#[cfg(test)] mod tests` 削除は cfg-gated test コードで production runtime 不変)
- `Cargo.toml` / `Cargo.lock` dep 追加
- CI workflow (`.github/workflows/**`) 変更
- `README.md` / `CHANGELOG.md` / `AGENTS.md` の test count 記載 (total 数は load bearing でない、update 不要)
- monkey-test / dogfood phase の実行 (feature-dev `args: {codex: true, e2e: false}`)
- Pilot Re-audit (F1 pilot 3 file の既存 test body は F2b で touchless、trigger 非発動)
- audit-template.md v1 → v2 bump (F2b patch は wording clarification のみ、v1 unchanged)
- Duplication Candidates の "engine_test regate_* vs belt-agent cli regate_*" 統合 (cross-crate、F3 scope)

## Future Work

### F3 (belt-agent behavior SSOT + cross-crate duplication、次 feature)

**F2b からの input**:
- F2b audit-report.md の Forward-to-F3 list (belt-agent + cross-crate duplication)
- belt-core.yml の確立済 scenarios (uri 12 + gate 13-14 + error 4-5 + expander 6-7 + model/view/lint/parser/artifact_when 既存)
- tests/common/{mod,helpers,narrative}.rs の established pattern (F3 で belt-agent/tests/common へ拡張検討)

**F3 scope**:
- `crates/belt-agent/tests/cli_test.rs` (40 test) の audit
- `crates/belt-agent/tests/e2e_test.rs` (8 test) の audit
- `docs/testing/cli-behavior/belt-agent.yml` 拡充 — 6 subcommand (init/next/verify/regate/step/status) の JSON contract scenarios 列挙 (推定 30-40 scenarios)
- cross-crate duplication の決着:
  - `engine_test regate_* vs belt-agent cli regate_*` の分離判定 (layer 分離 keep or 統合)
  - `engine_test verify_verdict_* vs belt-agent cli verify_*` 同上
  - `view_test engine_enriched_status_* vs belt-agent cli status_*` 同上
- binary crate helper unification 判定 (variant B を統合するか、crate 独立を許容するか)
- F2b で新設した tests/common/*.rs を belt-agent 側にも導入するか判定

**F3 推定規模**:
- audit 対象 test 数: 48 (belt-agent cli + e2e) + cross-crate 40 = 88
- 新規 scenarios: belt-agent.yml で 30-40
- 削除 test: cross-crate duplication で 5-10 程度

### 将来 (F2b/F3 全て外、別 feature)

- audit-template.md v1 → v2 bump (新 reason label 必要時)
- coverage tool 導入 (`tarpaulin` / `cargo-mutants`)
- 未使用 dev-dep 活用 (`insta` snapshot / `pretty_assertions` diff / `rstest` parameterize)
- `docs/superpowers/specs/` 30+ verbatim path 参照の機械検証拡張
- monkey-test / dogfood pipeline 実行用の別系統 scenarios (現状 CLI scenarios は monkey-test 対象外)

### Linear tracking

- F1 は memory に Linear issue 番号記録なし (BELT-20 parent tracking のみ)
- F2a / F2b / F3 を BELT-20 配下の子 issue で切るかは本 feature scope 外、linear-refresh skill の責務
- 本 design は feature-dev pipeline (F2b) の deliverable 作成に集中、Linear 反映は別 operation
