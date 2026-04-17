# belt-test-foundation (F2a) — Design

F2a は「belt の test 資産を audit する多段 feature 群 (F1 → F2a → F2b → F3)」の**第 2 段 (前半)**。目的は、(1) F1 で完成した 3 層構造を belt-core 全 behavior module に拡張、(2) `docs/testing/cli-behavior/belt-core.yml` に 10 module の scenarios を M2 normalize で列挙、(3) 既存 test に doc-comment を X2 policy で付与して `scenarios_contract.rs` の symmetric diff 機構で binding を確定、(4) 復帰 2 scenario を対応 test と同時に追加、(5) `strip_string_literals` の multiline raw string 対応で binding 機構を strengthen する。F2a は **SSOT 拡充 + binding realization + 軽量復帰** に集中し、behavior-less test の削除・Duplication 統合・shape-lock entry 追加などの実 audit は F2b で行う。

## Brainstorming 決議の総括 (6 axes)

| Axis | 決議 | 含意 |
|---|---|---|
| Scope | **A** | belt-core 全 behavior module + 復帰 2 + strip fix、duplication 統合は F2b |
| Split | **A2** | F2a (scenarios + strip fix + 復帰) → F2b (audit + duplication) に分割 |
| Normalization | **M2** | 1 scenario ↔ N test を normalize-up-front、合計目安 ~77-107 scenarios |
| Doc-comment | **X2** | map できる全 test に付与、behavior-less test は F2b へ forward |
| Restoration | **R1** | 復帰 2 scenario は F2a 内で test 追加と同時 |
| Workflow | **W1** | `/belt:feature-dev` pipeline (monkey-test/dogfood skip) |

## Architecture

3 層構造 (F1 から継承、belt-core 全域に拡張):

```
Layer 1: SSOT (docs/testing/cli-behavior/)
    - belt-core.yml: 既存 config 6 scenario + 10 module scenarios 追加 (engine/view/lint/model/parser/expander/gate/uri/error/artifact_when)
                     + 復帰 belt-core-config-preserves-absolute-pipeline-path
    - belt.yml: 既存 5 scenario + 復帰 belt-lint-invalid-yaml-rejected

Layer 2: Binding (crates/belt-core/tests/scenarios_contract.rs)
    - 既存 symmetric diff assert + lock_ledger assert は F2a の yml/doc 追加を自動検証 (実装変更は set-based 化のみ、必要時)
    + strip_string_literals の multiline raw string 対応 (fix + drift test 2-3 本追加)

Layer 3: Binding realization (Rust test doc-comment X2 付与)
    - 既存 belt-core behavior test file 9 本 (engine/view/lint/model/parser/expander/gate/error/artifact_when_field) に doc-comment 付与
    - 新規 uri_test.rs (3-5 test、belt://{Run,Latest,WorkspaceLatest} parser 検証) 作成 + doc-comment
    - 復帰用新規 test 2 本 (cli_test.rs の lint_invalid_yaml_*, config_test.rs の preserves_absolute_pipeline_path)
```

### Data Flow

```
docs/testing/cli-behavior/{belt,belt-core}.yml
             │
             ├──(`/// scenario: <id>` doc-comment)──▶ tests/*.rs
             │
             └──(scenarios_contract.rs が両者を grep 照合)──▶ CI gate (各 commit で atomic)

docs/testing/audit-template.md v1
             └─ clarification patch (新規 fn 追加は re-audit trigger 対象外)、version unchanged
```

### Scope Boundary (F2b/F3 との境界)

**F2a 内 (本 feature)**:
- `docs/testing/cli-behavior/belt-core.yml` の 10 module 分 scenarios 追加 (M2 normalize、復帰 1 含む)
- `docs/testing/cli-behavior/belt.yml` に `belt-lint-invalid-yaml-rejected` 追加
- `crates/belt-core/tests/{engine,view,lint,model,parser,expander,gate,error,artifact_when_field}_test.rs` への doc-comment 付与 (X2 policy)
- `crates/belt-core/tests/uri_test.rs` 新規作成 + scenarios + doc-comment
- `crates/belt/tests/cli_test.rs` への `belt-lint-invalid-yaml-rejected` test 追加 + doc-comment
- `crates/belt-core/tests/config_test.rs` への `belt-core-config-preserves-absolute-pipeline-path` test 追加 + doc-comment
- `crates/belt-core/tests/scenarios_contract.rs` の `strip_string_literals` fix + drift test 2-3 本追加
- `docs/testing/audit-template.md` clarification patch (v1 unchanged、"新規 fn 追加は re-audit 対象外" 追記)
- F2a deliverable 全体の audit-report.md (F1 pilot 踏襲 format、kept 主体 + F2b forward list)

**F2a 外 (F2b 送り)**:
- doc-comment なし test (M2 normalize で scenario 化しなかった behavior-less 疑い、redundancy 疑い) の `redundant-with-X` / `implementation-coupling` 判定と削除
- Duplication Candidates 7 組の実統合コード変更 (helper 抽出 `tests/common/mod.rs` 含む)
- shape-lock 4 本 (bug_fix_refresh / review_skills_refresh / shared_criteria_parity / shared_filter_parity) の lock-ledger.md entry 追加
- `expander_with_test.rs` 0 test 状態の解消

**F3 scope**:
- belt-agent/tests/ (cli_test.rs 40 + e2e_test.rs 8) の audit
- belt-agent.yml 拡充 (6 subcommand: init/next/verify/regate/step/status の JSON contract)

### artifact_when 帰属

`artifact_when_field.rs` (5 tests) の scenario は **`model` category** ではなく独立 `artifact_when` category で列挙。理由: Artifact.when は `when: "args.e2e"` の expression 評価 + produce filtering の two-concern で、model の型 shape lock とは分離した方が audit 粒度が合う。belt-core.yml の `scope:` 宣言も `model / artifact_when` を並置記述。

## Deliverables

| # | Path | Type | Purpose |
|---|------|------|---------|
| 1 | `docs/testing/cli-behavior/belt-core.yml` | edit | 10 module scenarios 追加 + 復帰 1 + `scope:` 更新 |
| 2 | `docs/testing/cli-behavior/belt.yml` | edit | `belt-lint-invalid-yaml-rejected` 追加 |
| 3 | `crates/belt-core/tests/scenarios_contract.rs` | edit | `strip_string_literals` multiline raw 対応 + drift test 2-3 本追加 (set-based 化は必要時のみ) |
| 4 | `crates/belt-core/tests/engine_test.rs` | edit | doc-comment 付与 (X2) |
| 5 | `crates/belt-core/tests/view_test.rs` | edit | doc-comment 付与 |
| 6 | `crates/belt-core/tests/lint_test.rs` | edit | doc-comment 付与 |
| 7 | `crates/belt-core/tests/model_test.rs` | edit | doc-comment 付与 |
| 8 | `crates/belt-core/tests/parser_test.rs` | edit | doc-comment 付与 |
| 9 | `crates/belt-core/tests/expander_test.rs` | edit | doc-comment 付与 |
| 10 | `crates/belt-core/tests/gate_test.rs` | edit | doc-comment 付与 |
| 11 | `crates/belt-core/tests/error_test.rs` | edit | doc-comment 付与 |
| 12 | `crates/belt-core/tests/artifact_when_field.rs` | edit | doc-comment 付与 |
| 13 | `crates/belt-core/tests/config_test.rs` | edit | `preserves_absolute_pipeline_path` test 1 本追加 + doc-comment |
| 14 | `crates/belt/tests/cli_test.rs` | edit | `lint_invalid_yaml_*` test 1 本追加 + doc-comment |
| 15 | `crates/belt-core/tests/uri_test.rs` | **new** | `belt://{Run,Latest,WorkspaceLatest}` parser の 3-5 test + scenarios |
| 16 | `docs/testing/audit-template.md` | edit | clarification patch (v1 unchanged) |
| 17 | `docs/features/2026-04-17-belt-test-foundation-f2a/design.md` | new | 本 doc |
| 18 | `docs/features/2026-04-17-belt-test-foundation-f2a/test-strategy.md` | new | Phase 2 の産物 |
| 19 | `docs/features/2026-04-17-belt-test-foundation-f2a/plan.md` | new | Phase 4 の産物 |
| 20 | `docs/features/2026-04-17-belt-test-foundation-f2a/audit-report.md` | new | Phase 5 execute の産物、kept 主体 + F2b forward list |
| 21 | `.belt/runs/{run_id}/notes/phase-*.md` | new | 6 phase narrative notes |

**expander_with_test.rs 除外**: F1 baseline 時点で 0 test (17 行の skeleton のみ)。F2a では touch しない。belt-core.yml の scope 記述で "F2b で expander_with normalize 検討" を明記。

### Stub YAML skeleton (M2 normalize 例 — engine module)

```yaml
  # engine module — 67 test を ~15-20 scenario に normalize する例
  - id: belt-core-engine-init-creates-run-directory
    category: engine
    severity: high
    technique: equivalence-partition
    given: "a valid pipeline with no existing runs"
    when: "Engine::init() is called"
    then: "returns run_id and creates .belt/runs/{run_id}/ with initial state.json"

  - id: belt-core-engine-regate-resets-downstream-verify-verdict
    category: engine
    severity: high
    technique: state-transition
    given: "a run that has passed verify on phase X with regate=[Y]"
    when: "regate returns control to phase Y"
    then: "phase X verify_verdict is cleared; phase X gate must be re-evaluated"

  - id: belt-core-engine-verify-records-per-check-verdict
    category: engine
    severity: high
    technique: equivalence-partition
    given: "a phase with multiple gate checks"
    when: "Engine::verify() is invoked"
    then: "each check's verdict (pass/fail) is persisted individually in state.json"
```

### M2 normalize 粒度目安 (total ~77-107 scenarios)

| module | test 数 | scenario 目安 | 集約率 |
|---|---|---|---|
| engine | 67 | 15-20 | ~4:1 |
| view | 41 | 10-15 | ~3:1 |
| model | 39 | 15-20 | ~2:1 |
| lint | 29 | 10-15 | ~2:1 |
| gate | 22 | 5-10 | ~3:1 |
| error | 6 | 3-5 | ~1.5:1 |
| artifact_when | 5 | 3-5 | ~1:1 |
| expander | 5 | 3-5 | ~1:1 |
| parser | 4 | 3-4 | ~1:1 |
| uri (new) | 3-5 (new test file) | 3-5 | ~1:1 |
| 復帰 | — | +2 | — |

## Prerequisites

1. **F1 deliverables が stable**: merge 済 (`3ca76a8`)、audit-template.md v1 凍結、scenarios_contract.rs 機構稼働
2. **serde-saphyr `=0.0.23`** (workspace dep、新規追加なし)
3. **Rust integration test 規約**: 新規 test file (`uri_test.rs`) は preamble (`#![allow(clippy::unwrap_used, expect_used, panic, ...)] reason = "..."`) 必須、既存 file は preamble 既存
4. **scenarios_contract.rs の symmetric diff assert が F2a 中の CI signal**: yml 追加 → 対応 doc-comment 追加が同一 commit、逸脱は即 CI fail
5. **audit-template.md v1 の "Pilot Audit の再実施 trigger" 非発動**: F2a で F1 pilot file (cli_test.rs / config_test.rs) に new test 追加するが、既存 test は unchanged → re-audit 不要。clarification patch で明示
6. **worktree 作成**: `feature/2026-04-17-belt-test-foundation-f2a` (`/worktrunk` で `.claude/worktrees/f2a` 配置想定)
7. **baseline `cargo test --workspace` = 397 pass** (F1 merge 時点、F2a 開始前に再確認)
8. **feature-dev pipeline args**: `{codex: false, e2e: false}`、monkey-test / dogfood skip (CLI behavior test 増加は monkey-test 対象外)
9. **`uri_test.rs` 不在の現実**: F2a baseline 時点で `crates/belt-core/tests/uri_test.rs` は存在しない → F2a で新規作成、uri module の behavior を M2 normalize で 3-5 scenario 列挙 + 対応 test を同時作成 (symmetric diff CI 制約上、yml と test file は同 commit)
10. **`filetime` dev-dep** (F1 で view_test.rs に導入済、追加なし)
11. **existing F1 pilot files の既存 doc-comment 保存**: cli_test.rs / config_test.rs に追加する新規 test のみ doc-comment 付与、既存 5+6 test の doc-comment は unchanged
12. **symmetric diff CI の multi-match 許容**: M2 で 1 scenario を 1 module 内 3-5 test に付与する場合、yml 側 ID と rust 側 ID の "集合" 一致 (multiplicity 不問) が必須。現状実装が set-based でない場合は Phase B1 で set-based 化

## Impact Scope

### File-level

**edit (既存ファイル更新)**:
- `docs/testing/cli-behavior/belt-core.yml` (10 module scenarios + `scope:` 更新 + 復帰 1)
- `docs/testing/cli-behavior/belt.yml` (復帰 1 scenario)
- `crates/belt-core/tests/scenarios_contract.rs` (strip fix + drift test 2-3 本)
- `crates/belt-core/tests/{engine,view,lint,model,parser,expander,gate,error,artifact_when_field}_test.rs` (doc-comment 付与のみ、behavior 不変)
- `crates/belt-core/tests/config_test.rs` (new test 1 本 + doc-comment)
- `crates/belt/tests/cli_test.rs` (new test 1 本 + doc-comment)
- `docs/testing/audit-template.md` (clarification patch、v1 unchanged)

**new (新規作成)**:
- `crates/belt-core/tests/uri_test.rs` (uri module behavior scenarios と対応 test)
- `docs/features/2026-04-17-belt-test-foundation-f2a/{design,test-strategy,plan,audit-report}.md`
- `.belt/runs/{run_id}/notes/phase-*.md` (narrative notes)

**non-impacted (明示)**:
- `crates/belt-core/tests/expander_with_test.rs` (0 test、F2a では touch せず、F2b で再検討)
- `crates/belt-core/tests/{bug_fix,review_skills}_refresh.rs`, `shared_{criteria,filter}_parity.rs` (shape-lock、F2a では doc-comment 付与しない → F2b で lock-ledger.md entry 追加)
- `crates/belt-agent/tests/**` (F3 scope)
- `crates/*/src/**` (production code 全 module unchanged)
- `Cargo.toml` / `Cargo.lock` (dep 追加なし)
- `.github/workflows/**` (CI 変更なし)

### Module-level (behavior / contract 変化)

- **No production code change**: `crates/*/src/**` の全 module に変更なし (test-only feature)
- **Test surface 追加**: 2 復帰 + uri_test.rs 3-5 + scenarios_contract.rs drift 2-3 = 7-10 test fn
- **Test surface 不変**: 既存 test (397) の behavior は unchanged、doc-comment 追加のみ

## Impact Analysis

### Reverse Dependencies

| caller | target | coupling | reason |
|---|---|---|---|
| `scenarios_contract.rs::scenarios_yml_and_rust_docs_match` | 全 `docs/testing/cli-behavior/*.yml` + 全 `crates/*/tests/*.rs` | direct | symmetric diff が F2a の CI gate 本体。各 commit で atomic yml+doc-comment stage 必須 |
| `scenarios_contract.rs::lock_ledger_locks_files_exist` | `docs/testing/lock-ledger.md` | direct | F2a では lock-ledger.md unchanged、breakage なし |
| `scenarios_contract.rs::strip_string_literals` | `scenarios_contract.rs` 自身の 7 drift injection test | self (lock) | F2a で fix する際、既存 7 drift test を壊さずに multiline raw 対応追加 |
| `crates/belt/tests/cli_test.rs::lint_invalid_yaml_*` (new) | `crates/belt/src/main.rs` の `belt lint` + belt-core `parse_pipeline` の YAML error path | direct | F1 existing 5 test と同 surface。miette diagnostic format 変更で brittle 化懸念 |
| `crates/belt-core/tests/config_test.rs::preserves_absolute_pipeline_path` (new) | `belt-core/src/config.rs::resolve_pipeline_path` | direct | 絶対パス入力時の behavior lock。実装が preserve せず canonicalize/join する bug があった場合は F2a scope 越境 |
| `crates/belt-core/tests/uri_test.rs` (new) | `belt-core/src/uri.rs` の `Run` / `Latest` / `WorkspaceLatest` parser | direct | F2a で初の uri module behavior lock。`belt://` schema 変更耐性 |
| `docs/superpowers/specs/**` の 30+ verbatim path 参照 | `cli_test.rs`, `config_test.rs`, `feature_dev_refresh.rs` | weak (historical) | F2a で rename なし、新 file `uri_test.rs` のみ (既存 link に影響なし) |

### Shared State

| kind | constraint | usage |
|---|---|---|
| FS (yml 1 file) | `belt-core.yml` が 10 module 分の scenarios を 1 file に集積 | 並列 subagent 時の write 統合責任は main agent (controller) |
| FS (test file family) | F1 で import 済の serde / thiserror / miette pattern を踏襲 | 13 既存 file edit + 1 new (uri_test.rs) で新規 dep ゼロ |
| scenarios_contract.rs の symmetric diff | yml ID 集合と rust doc ID 集合が完全一致 | 各 commit で両方更新、diverge は即 CI fail |
| audit-template.md v1 | reason label enumeration 固定 | F2a では label 9 個を使わない (kept / kept-without-scenario-id のみ)、次 version bump 不要 |
| workspace dev-deps | `filetime` / `insta` / `pretty_assertions` / `rstest` — F1 で filetime のみ活用、残 3 は未使用 | F2a も同条件、新規活用なし |
| F1 audit-report frontmatter | `audit_template_version: v1` | F2a audit-report.md も v1 を frontmatter に記載 (version unchanged) |

### Implicit Contracts

| file:line (推定) | dependency | violation impact |
|---|---|---|
| `scenarios_contract.rs::scenarios_yml_and_rust_docs_match` | yml ID と rust doc ID の完全集合一致 | F2a 中の commit で yml 先行 / doc 先行を許容しない (同 commit で両方更新必須) |
| `scenarios_contract.rs::strip_string_literals` 既存 drift test | single-line string literal を正しく除去 | F2a の multiline raw fix 実装が既存 single-line behavior を壊さない |
| `audit-template.md "Pilot Audit の再実施 trigger"` | `git log --since="$AUDITED_AT"` で touch 検知 | F2a で cli_test.rs / config_test.rs に新規 fn 追加 → touch 発生 → F2b 着手時に re-audit 誤判定。対策: audit-template.md に "新規 fn 追加は re-audit 対象外" clarification patch |
| M2 normalize 粒度目安 (design.md) | engine 15-20 / view 10-15 / model 15-20 / lint 10-15 / gate 5-10 | ±30% 以内を spec-review の許容 band、外れた module は reviewer が指摘 |
| `belt lint` stderr format | 現状の miette diagnostic format で "parse error" / "duplicate" / "ok" の literal substring | F2a 新規 `lint_invalid_yaml_*` test は "contains YAML parse error indication" レベルの抽象 assertion、exact match 回避 |
| `resolve_pipeline_path` の absolute path preservation | `if path.is_absolute() { path } else { config_dir.join(path) }` 型の早期 return behavior | 実装が別 path (canonicalize / strip_prefix 等) を通している場合、復帰 test が発覚 bug → F2a scope 越境 |

### Side Effect Risks

| severity | trigger | impact / 対策 |
|---|---|---|
| high | `scenarios_contract.rs` の symmetric diff assert が同一 scenario ID の多重 doc-comment match を許容しない実装 | X2 policy 破綻、全 commit CI fail。**対策**: F2a Phase B1 で contract.rs 現状を read、必要なら set-based assertion に書き換え |
| high | M2 normalize 粒度が subagent 間で不統一 (engine を 20 scenario / view を 5 scenario など乖離) | SSOT 一貫性崩壊、F2b audit で base がブレる。**対策**: design.md に module 毎目安を記載 (済)、spec-review で normalize 粒度 sanity check、execute 中 subagent prompt に module 目安 verbatim 転記 |
| high | `lint_invalid_yaml` 復帰 test の assertion が miette format に brittle | miette 7.6 → 7.x bump で fail、CVE-2026 patch 連鎖。**対策**: assertion は `stderr.contains("parse")` ± `exit_code == 1` level、literal phrase 回避 |
| high | `resolve_pipeline_path` の absolute path 実装が preserve ではなく canonicalize している場合、復帰 test が bug 発覚 = production code touch 必要 | F2a scope 越境、別 issue 分離。**対策**: F2a design phase で resolve_pipeline_path 実装を事前読み、behavior 確定。乖離あれば復帰 scenario を design から除外、別 mini feature で fix |
| medium | `audit-template.md` patch の scope (v1 unchanged で clarification 追記のみ) | SemVer bump しない = version 確認 assert は unchanged。**対策**: patch は純粋 clarification で semantics 変更なし、scenarios_contract.rs の version check test は unchanged |
| medium | 並列 subagent が belt-core.yml に同時書き込み merge conflict | **対策**: main agent が subagent 成果を sequential merge する workflow、subagent は belt-core.yml を write しない。Module section 境界をコメントで明示 |
| medium | uri_test.rs 新規作成で既存 pattern (parser_test.rs / error_test.rs) と UT convention 乖離 | 一貫性問題のみ。spec-review で指摘 |
| medium | 新規 uri_test.rs の scenarios 列挙で uri.rs の public API 全 surface カバーに scope 肥大 | belt-core.yml の uri category は M2 normalize 目安 3-5 scenario を strict に守る、uri.rs 内部 helper は除外 (public API のみ) |
| low | expander_with_test.rs 空状態の理由不明のまま F2a 通過 | design.md に記述: "expander_with の test は expander_test.rs 内の `expand_pipeline_with_nested_uses` 系に統合済 (F1 baseline 時点)、expander_with_test.rs は将来 separate モジュール化時のプレースホルダー。F2a では touch しない" を Phase 1 design で事実確認 |
| low | strip_string_literals fix で false-negative 導入 (multiline raw string 修正が厳しすぎて真の scenario 行をフィルタ) | drift test で positive case (multiline raw + scenario ID 1 個) も追加、symmetric diff で orphan rust doc 検知 |
| low | F2a scope creep (F2b work を誤って F2a 内で実行) | spec-review reviewer が "F2b マター" を指摘できる criteria 明文化 (execute/design の `/belt:feature-dev` criteria に依存) |

### F2a baseline sanity check

- `cargo test --workspace` = 397 pass (F1 merge 時点、F2a worktree 作成時に再確認)
- `cargo clippy --workspace -- -D warnings` = clean
- `cargo fmt --all -- --check` = clean
- `git log --since="2026-04-17T00:00:00Z" -- crates/belt/tests/cli_test.rs crates/belt-core/tests/config_test.rs crates/belt-core/tests/feature_dev_refresh.rs` = empty (pilot re-audit trigger 非発動、F2a 開始前に再確認)

## Must-Verify Checklist

### 基本構造 (deliverable 存在と shape)

- [ ] **MV-01**: `docs/testing/cli-behavior/belt-core.yml` の `scope:` が「10 module 全列挙済」に更新
- [ ] **MV-02**: belt-core.yml に engine / view / lint / model / parser / expander / gate / uri / error / artifact_when の 10 category の scenarios が追加、合計 77-107 scenarios (M2 normalize 目安 band 内)
- [ ] **MV-03**: belt-core.yml に `belt-core-config-preserves-absolute-pipeline-path` scenario が追加、category = config
- [ ] **MV-04**: belt.yml に `belt-lint-invalid-yaml-rejected` scenario が追加、category = lint
- [ ] **MV-05**: `crates/belt-core/tests/uri_test.rs` が新規作成、3-5 `#[test]` fn + preamble (`#![allow(...)] reason = "..."`)
- [ ] **MV-06**: `crates/belt/tests/cli_test.rs` に `lint_invalid_yaml_*` fn 1 本追加、既存 5 test unchanged
- [ ] **MV-07**: `crates/belt-core/tests/config_test.rs` に `preserves_absolute_pipeline_path` fn 1 本追加、既存 6 test unchanged
- [ ] **MV-08**: `docs/testing/audit-template.md` の Pilot Re-audit trigger 節に "新規 fn 追加は re-audit 対象外" clarification が追記、`audit_template_version` は v1 unchanged
- [ ] **MV-09**: `docs/features/2026-04-17-belt-test-foundation-f2a/audit-report.md` が新規作成、frontmatter に `audited_at / audited_commit / audit_template_version: v1` 記載

### SSOT ↔ Rust binding 機械検証

- [ ] **MV-10**: `cargo test -p belt-core --test scenarios_contract` が全 pass (10 既存 test + strip fix drift test 2-3 本)
- [ ] **MV-11**: `scenarios_contract.rs::strip_string_literals` が multiline raw string (`r#"\n/// scenario: X\n"#`) を正しく除去する positive drift test 追加
- [ ] **MV-12**: 同一 scenario ID が複数 Rust test の doc-comment に付与されたケース (X2 policy) で `scenarios_yml_and_rust_docs_match` が pass する (set-based assertion)。現状実装が set-based でない場合は Phase B1 で修正
- [ ] **MV-13**: 新規 drift test — yml に書いた scenario ID に対応する doc-comment が Rust 側ゼロの場合、明確な panic message
- [ ] **MV-14**: 新規 drift test — Rust doc-comment の ID が yml 側にない場合、明確な panic message

### Module-level coverage (M2 normalize band)

- [ ] **MV-15**: engine category = 15-20 scenarios、doc-comment 対応 test が engine_test.rs 内に存在
- [ ] **MV-16**: view category = 10-15 scenarios、doc-comment 対応が view_test.rs 内に存在
- [ ] **MV-17**: model category = 15-20 scenarios、doc-comment 対応が model_test.rs 内に存在
- [ ] **MV-18**: lint category = 10-15 scenarios、doc-comment 対応が lint_test.rs 内に存在
- [ ] **MV-19**: gate category = 5-10 scenarios、doc-comment 対応が gate_test.rs 内に存在
- [ ] **MV-20**: error category = 3-5 scenarios、doc-comment 対応が error_test.rs 内に存在
- [ ] **MV-21**: expander category = 3-5 scenarios、doc-comment 対応が expander_test.rs 内に存在
- [ ] **MV-22**: parser category = 3-4 scenarios、doc-comment 対応が parser_test.rs 内に存在
- [ ] **MV-23**: uri category = 3-5 scenarios、doc-comment 対応が uri_test.rs (新規) 内に存在
- [ ] **MV-24**: artifact_when category = 3-5 scenarios、doc-comment 対応が artifact_when_field.rs 内に存在

### 復帰 2 scenario の behavior correctness

- [ ] **MV-25**: `lint_invalid_yaml_*` test: invalid YAML を入力した belt lint が exit=1 かつ stderr に parse error を含む (literal phrase 依存せず、`contains("parse")` または miette diagnostic severity check で抽象化)
- [ ] **MV-26**: `preserves_absolute_pipeline_path` test: absolute path を `resolve_pipeline_path` に渡すと入力そのものが返る (`assert_eq!(resolved, input)`、canonicalize されない)。実装乖離時は F2a 停止 + 別 issue 分離
- [ ] **MV-27**: 新規 2 test は F1 pilot file の既存 test 集合を変更しない (modify 0、add only)

### strip_string_literals multiline raw string fix

- [ ] **MV-28**: 既存 7 drift injection test (`typo senario` / `single-slash` / `block comment stripping /* */` / `block doc-comment /** */` / `string literal stripping` / `inner doc-comment //!` / `positive match case`) が unchanged で全 pass
- [ ] **MV-29**: 新規 drift test で multiline raw string (`r#"\n/// scenario: foo\n"#`) 内の `/// scenario:` 行を false-positive として拾わないことを lock
- [ ] **MV-30**: 新規 drift test で multiline raw string を含む test ファイル全体で正しい `/// scenario:` doc-comment (関数直上) が依然 match することを lock (false-negative 防止)

### 一貫性・regression

- [ ] **MV-31**: `cargo test --workspace` 全 pass (397 + 新規 `uri_test.rs` 3-5 + `lint_invalid_yaml` 1 + `preserves_absolute_pipeline_path` 1 + `scenarios_contract` drift 2-3 = 404-407 前後)
- [ ] **MV-32**: `cargo clippy --workspace -- -D warnings` clean
- [ ] **MV-33**: `cargo fmt --all -- --check` clean
- [ ] **MV-34**: F1 pilot file (cli_test.rs 既存 5 / config_test.rs 既存 6 / feature_dev_refresh.rs 全 11) の既存 test が unchanged (Phase 5 execute 最初に `git diff` empty 確認)
- [ ] **MV-35**: shape-lock 4 file (bug_fix_refresh / review_skills_refresh / shared_criteria_parity / shared_filter_parity) が unchanged

### Narrative notes

- [ ] **MV-36**: `.belt/runs/{run_id}/notes/phase-{design,test-scenarios,spec-review,plan,execute,code-review}.md` が各 phase 完了時に作成、frontmatter (`phase:` / `run_id:`) + 4 節 (Decisions / Concerns / Directives / Observations)

### worktree

- [ ] **MV-37**: `feature/2026-04-17-belt-test-foundation-f2a` branch 存在 (`git branch --list`)
- [ ] **MV-38**: baseline `cargo test --workspace` = 397 pass、design.md commit 前に確認済

### doc-drift 予防

- [ ] **MV-39**: `docs/testing/README.md` の「docs/ 構造」宣言が F2a 変更 (belt-core.yml 10 module 列挙済) と矛盾しない、必要なら README 側も同 commit で更新
- [ ] **MV-40**: `docs/superpowers/specs/**` の 30+ verbatim path 参照に F2a で rename ゼロ (既存 link 維持)。F2a で追加する `uri_test.rs` は既存 doc の参照対象外
- [ ] **MV-41**: audit-template.md の patch がクロス doc (F1 design.md / F1 audit-report.md / lock-ledger.md) と矛盾しない (Decision Tree の Q1-Q5 + reason label 9 + shape-lock exception は unchanged)

### F2a 特有 risk 由来

- [ ] **MV-42**: M2 normalize 粒度の spec-review 承認が phase-spec-review.md narrative notes に記録される
- [ ] **MV-43**: `resolve_pipeline_path` の absolute path 実装が "preserve" であることを design phase で事前確認、実装読みの根拠 (file:line) を design.md に記載
- [ ] **MV-44**: 並列 subagent 実装時、各 subagent は 1 module / 1 test file のみ担当、main agent (controller) が belt-core.yml merge を sequential に担当、conflict ゼロ
- [ ] **MV-45**: `uri_test.rs` の test 数が M2 目安 3-5 を逸脱しない (scope 肥大防止)

## Test Perspectives

F2a deliverable を input parameter として Normal / Boundary / Abnormal / State-transition の 4 観点で test 対象を列挙。Phase 2 (test-scenarios) で `test-strategy.md` に expand、Phase 5 execute 中の `scenarios_contract.rs` drift test がこれら観点を assertion 化する。

### P1: belt-core.yml 10 module scenarios (M2 normalize)

| 観点 | 対象 case |
|---|---|
| Normal | 各 module の happy path scenario (engine init、view status、lint valid、model serde roundtrip 等)、required field (id/category/severity/given/when/then) 全充足 |
| Boundary | M2 normalize 目安 band の下限 / 上限、scenario 1 entry の category、ID 最大長 kebab-case |
| Abnormal | scope に書いた module と yml 内 category に不一致 (e.g., `scope: "engine ..."` だが `category: enigne` typo) / category が未宣言モジュール |
| State-transition | scenario A の category = engine → yml 編集で category = view に変更 (module 移動)、doc-comment 側の同期漏れ |

### P2: belt.yml への 1 scenario 追加

| 観点 | 対象 case |
|---|---|
| Normal | `belt-lint-invalid-yaml-rejected` が schema 準拠、既存 5 scenario (belt-lint-valid-pipeline-ok 等) と non-collision |
| Boundary | belt.yml scenarios 配列が 5 → 6 entry に増加のみ (順序 preservation) |
| Abnormal | 既存 scenario の ID を誤って複製 (duplicate ID 検出) |
| State-transition | belt.yml の既存 5 scenario と新 1 scenario で cli_test.rs 側 doc-comment の分布が 5:1 対応維持 |

### P3: Rust test doc-comment X2 多重付与

| 観点 | 対象 case |
|---|---|
| Normal | 1 scenario ↔ 1 test、1 scenario ↔ 3 test (engine regate 等)、1 test ↔ 2 scenarios (複合 behavior test) |
| Boundary | 最小 = 1 scenario ↔ 1 test、最大 = 1 scenario ↔ 全 scenario 対応 test (理論上 67 まで) |
| Abnormal | doc-comment typo (`/// senario:`) / yml にない ID を doc-comment で参照 (orphan rust) / yml に書いたが Rust 側ゼロ (orphan scenario) |
| State-transition | doc-comment 追加 → yml 削除でも symmetric diff fail (commit atomicity 確保) / doc-comment 削除 → yml 追加でも同じ |

### P4: strip_string_literals multiline raw string 対応

| 観点 | 対象 case |
|---|---|
| Normal | 通常の `"..."` / raw string `r"..."` / raw string with hash `r#"..."#` 内の `/// scenario:` を除去 |
| Boundary | 複数行に渡る raw string、string literal の先頭行と末尾行にまたがる doc-comment-like テキスト、最大 hash count (`r##########"..."##########`) |
| Abnormal | 非閉 raw string (syntax error ファイル) での無限ループ / string literal 内の `/// scenario: X\n` が実 doc-comment と誤分類 |
| State-transition | raw string を closing tag (`"#`) の前に追加/削除した時の scenarios_contract CI 挙動 |

### P5: uri_test.rs (新規 behavior test)

| 観点 | 対象 case |
|---|---|
| Normal | `belt://run/<uuid>` / `belt://latest` / `belt://<workspace>/latest` の正常 parse、3 selector 全種 |
| Boundary | uuid v7 の最大/最小表記、workspace 名の kebab-case edge (single char、100 char 等) |
| Abnormal | schema mismatch (`http://...`) / path 欠落 (`belt://`) / uuid format 不正 / workspace 名に禁止文字 |
| State-transition | Run parser が成功 → Latest に fallback (順序依存) が現状実装に存在するか否か、scenarios で明示 |

### P6: 復帰 2 new test

| 観点 | 対象 case (lint_invalid_yaml) |
|---|---|
| Normal | invalid YAML 入力で exit=1、stderr に YAML parse error 指示 (literal 非依存) |
| Boundary | empty file / 1 byte / YAML 構文の edge (tab indent 禁止 等) |
| Abnormal | binary file 入力、permission denied ファイル、symlink loop |
| State-transition | 同 file を valid → invalid に書き換えて連続 `belt lint` 呼び出し、2 回目は必ず fail |

| 観点 | 対象 case (preserves_absolute_pipeline_path) |
|---|---|
| Normal | `/tmp/pipeline.yml` absolute 入力 → そのまま返る |
| Boundary | absolute path of length 1 (`/`), pipeline_file に絶対パス + `../` 混在 (canonicalize されない確認) |
| Abnormal | absolute path 存在しない file、permission denied dir |
| State-transition | relative pipeline_file → absolute pipeline_file に書き換えた config 再読み込み時、resolve が入力毎に correct |

### P7: audit-template.md micro-patch

| 観点 | 対象 case |
|---|---|
| Normal | "新規 fn 追加は re-audit 対象外" clarification が Pilot Re-audit trigger 節に追記、既存 reason label / Decision Tree unchanged |
| Boundary | patch 前後の version 比較 — `audit_template_version: v1` unchanged、scenarios_contract.rs の version check が pass |
| Abnormal | clarification が Decision Tree Q1-Q5 の semantics に干渉 (例: Q2 "同等 behavior を検証する他 test が存在するか" の "他 test" に新規 fn を含むか否か) |
| State-transition | F2a → F2b 着手時に audit 作業者が clarification を読み、pilot file の新規 fn 追加を re-audit 対象外と正しく判定 |

### Non-Functional Requirements

| 特性 | 要件 | 計測 |
|---|---|---|
| Performance | `scenarios_contract.rs` 全 test が < 2 秒 (lint スケール + 新規 drift test 加算後) | `cargo test --test scenarios_contract -- --nocapture` 実時間 |
| Maintainability | F2b 作業者が F2a audit-report.md の "forward-to-F2b" list を read して Decision Tree 適用可能 | handover + F2b Phase 1 design で referrable |
| Determinism | F2a 追加 test 全て deterministic (時間依存なし)、`for i in {1..50}; do cargo test --workspace \|\| exit 1; done; echo OK` 50 回 pass | bash loop で実測 |
| Portability | F1 と同条件 (x86_64/aarch64 × linux/macOS)、Windows MVP scope 外 | release.yml cross build + local test |
| Security | docs + test-only、production code unchanged、新 dep ゼロ | `cargo audit` / `cargo deny` 差分なし |

### Quality Bar 適合チェック

- 全 input parameter (P1-P7) に Normal / Boundary / Abnormal / State-transition 各 1 件以上 cover
- M2 normalize 粒度の定量基準 (module 毎 scenario 数目安) を spec-review phase で enforce
- 復帰 test の brittle 耐性 (miette format 依存回避) を design level で言明

## Execute Strategy

### Subagent 並列 dispatch + controller merge

F2a は module 単位で独立性が高い (各 module の test file は他 module に touch せず)。主 agent を controller、subagent を worker として以下の体制で実行:

**subagent の scope (1 module につき 1 subagent)**:
- 担当 module の test file のみ read
- M2 normalize で scenarios を列挙 (design.md の目安 band 遵守)
- doc-comment diff を構築 (X2 policy、behavior-less 疑い test は skip)
- **subagent は `docs/testing/cli-behavior/belt-core.yml` に直接書き込まない**、output を structured format で返す

**subagent output format (verbatim prompt で強制)**:
```
## Scenarios (to append to belt-core.yml)
<yml block: 該当 category の scenarios のみ>

## Doc-comment patches (for <test_file_path>)
<unified diff>

## Forward-to-F2b candidates
<list: scenario に map できなかった test の fn 名 + 疑い label>
```

**controller (main agent) の責務**:
- subagent output を受領、belt-core.yml に section merge (category 境界をコメントで明示)
- `cargo test -p belt-core --test scenarios_contract` で symmetric diff CI pass 確認
- 1 module 毎に atomic commit
- Forward-to-F2b list を audit-report.md に集約

### Phase 分割 (execute phase 内 tactical sequence)

**Phase A (parallel, 9 subagent dispatch)**:
- engine / view / lint / model / gate / error / expander / parser / artifact_when の 9 module を同時投入
- 各 subagent は独立、controller が merge 順序を決定

**Phase B (sequential, main agent)**:
- **B1**: `scenarios_contract.rs` の現状実装を read、symmetric diff が set-based (multi-match 許容) を確認。set-based でなければ先に set-based 化 + drift test を追加 (1 commit)
- **B2**: `scenarios_contract.rs` の `strip_string_literals` multiline raw string fix + 対応 drift test 2-3 本 (1 commit)
- **B3**: `uri_test.rs` 新規作成 + uri scenarios 追加 + doc-comment (1 commit)
- **B4**: 復帰 `belt-lint-invalid-yaml-rejected` — cli_test.rs に new test 追加 + belt.yml 更新 + doc-comment (1 commit)
- **B5**: 復帰 `belt-core-config-preserves-absolute-pipeline-path` — config_test.rs に new test 追加 + belt-core.yml 更新 + doc-comment。事前に `resolve_pipeline_path` 実装を read して preserve behavior を確認、乖離あれば F2a 停止 + 別 issue 分離 (1 commit)
- **B6**: `audit-template.md` micro-patch — "新規 fn 追加は re-audit trigger 対象外" clarification 追記 (1 commit)

**Phase C (post-work)**:
- audit-report.md 作成: kept 主体判定 + F2b forward list (subagent が Phase A で出した候補を集約) + frontmatter
- `docs/testing/README.md` の整合性チェック、scope 記述が 10 module 列挙済 reality に合っていれば unchanged

### commit 粒度 (目安 14-16 commits)

| # | commit scope | subject 例 |
|---|---|---|
| 1 | Phase B1 (必要時) | `test(belt-core): make scenarios_contract symmetric diff set-based` |
| 2 | Phase B2 | `test(belt-core): fix strip_string_literals for multiline raw strings` |
| 3 | Phase A/engine | `test(belt-core): add engine scenarios + doc-comments (67 tests → 18 scenarios)` |
| 4 | Phase A/view | `test(belt-core): add view scenarios + doc-comments (41 tests → 12 scenarios)` |
| 5 | Phase A/lint | `test(belt-core): add lint scenarios + doc-comments (29 tests → 12 scenarios)` |
| 6 | Phase A/model | `test(belt-core): add model scenarios + doc-comments (39 tests → 17 scenarios)` |
| 7 | Phase A/gate | `test(belt-core): add gate scenarios + doc-comments (22 tests → 7 scenarios)` |
| 8 | Phase A/error | `test(belt-core): add error scenarios + doc-comments (6 tests → 4 scenarios)` |
| 9 | Phase A/expander | `test(belt-core): add expander scenarios + doc-comments (5 tests → 4 scenarios)` |
| 10 | Phase A/parser | `test(belt-core): add parser scenarios + doc-comments (4 tests → 4 scenarios)` |
| 11 | Phase A/artifact_when | `test(belt-core): add artifact_when scenarios + doc-comments (5 tests → 4 scenarios)` |
| 12 | Phase B3 | `test(belt-core): add uri_test.rs + scenarios (new file, 4 tests → 4 scenarios)` |
| 13 | Phase B4 | `test(belt): restore belt-lint-invalid-yaml-rejected scenario + test` |
| 14 | Phase B5 | `test(belt-core): restore belt-core-config-preserves-absolute-pipeline-path scenario + test` |
| 15 | Phase B6 | `docs(testing): clarify re-audit trigger excludes new fn addition` |
| 16 | Phase C | `docs(features): add F2a audit report` |

Phase B1 が skipped なら 15 commits。

### Atomicity 原則

- 各 commit で `cargo test --workspace` / `cargo clippy --workspace -- -D warnings` / `cargo fmt --all -- --check` がすべて green
- 1 module 毎に bisect 可能 (commit revert で 1 module 巻き戻し)
- scenarios_contract.rs CI が各 commit の symmetric diff を自動 guard

### Concurrent write 回避

- subagent が belt-core.yml に直接書き込まないことで write conflict 根絶
- controller が sequential に merge = 同一 worktree 内 race condition ゼロ
- worktree 分離も controller 責務 (F2a worktree 1 個で全 work 完結)

## Non-Goals

**F2b 送り (F2a scope 外、明示的に forward)**:
- behavior-less test の削除 (`implementation-coupling` / `trivial-default-assertion` / `tautology` 判定)
- Duplication Candidates 7 組の実統合 (helper 抽出 / 重複 test の削除 / `tests/common/mod.rs` 新設)
- shape-lock 4 file (bug_fix_refresh / review_skills_refresh / shared_criteria / shared_filter) の lock-ledger.md entry 追加
- `expander_with_test.rs` 0 test 状態の解消 (touch せず、現状維持)
- F2a audit-report.md の `Forward-to-F2b candidates` list を input に F2b design 開始

**F3 scope**:
- `belt-agent/tests/` (cli_test.rs 40 test + e2e_test.rs 8 test) の audit
- `belt-agent.yml` の 6 subcommand (init/next/verify/regate/step/status) JSON contract 列挙

**将来 feature (F2a/F2b/F3 全てで未着手)**:
- audit-template.md の Reason Label v1 → v2 拡張
- coverage tool (`tarpaulin` / `cargo-mutants`) 導入
- 未使用 dev-dep (`insta` / `pretty_assertions` / `rstest`) の活用
- `docs/superpowers/specs/` の 30+ verbatim file-path 参照の機械化

**F2a で明示的にやらないこと (scope creep 防止)**:
- production code touch (全 `crates/*/src/**` module は unchanged、test-only feature)
- `Cargo.toml` / `Cargo.lock` dep 追加
- CI workflow (`.github/workflows/**`) 変更
- `README.md` / `CHANGELOG.md` / `AGENTS.md` の test count 記載 (total 数は load bearing でない、update 不要)
- monkey-test / dogfood phase の実行 (feature-dev `args: {codex: false, e2e: false}`)
- Pilot Re-audit (F1 pilot 22 test 既存 fn は modify ゼロ、trigger 非発動)

## Future Work

### F2b (belt-core audit + duplication 統合、次 feature)

**F2a からの input**:
- F2a audit-report.md の `Forward-to-F2b candidates` list (subagent が scenario に map できなかった test の fn 名 + 疑い label 付き)
- belt-core.yml 10 module scenarios (M2 normalize 完了済、F2b での scenario 追加削除は redundancy 判定の副次として発生)

**F2b scope**:
- Forward list test の Decision Tree 適用 (Q1-Q5)、judgement (`redundant-with-X` / `trivial-default-assertion` / `tautology` / `implementation-coupling` / `brittle-format-match` / `dead-fixture` / `unreachable-guard` / `obsolete-spec` の 8 label で分類)
- 判定に応じた test 削除 + scenarios.yml の整合性維持 (削除 test が唯一の map 先だった場合 scenario も削除)
- Duplication Candidates 7 組の統合 (`engine_test regate_* vs belt-agent cli regate_*` は F3 に先送り、belt-core 内 duplication のみ F2b)
- `tests/common/mod.rs` 新設 + `write_yaml` / `repo_root` / `fixture_path` を 5+ 箇所から集約
- shape-lock 4 file (bug_fix_refresh / review_skills_refresh / shared_criteria_parity / shared_filter_parity) の lock-ledger.md entry 追加
- `expander_with_test.rs` 0 test 状態の解消 (統合 or 削除)

**F2b 推定規模**:
- audit 対象 test 数: F2a で doc-comment 付かなかった残 test ≒ 40-80 (M2 normalize 集約率に依存)
- 削除 test 数: Decision Tree の分布次第、推定 20-50
- lock-ledger.md entry 追加: 4 files
- helper 抽出: 1 new file (tests/common/mod.rs)

### F3 (belt-agent behavior SSOT + cross-crate duplication)

**F3 scope**:
- `belt-agent/tests/cli_test.rs` (40 test) + `e2e_test.rs` (8 test) の audit
- `belt-agent.yml` に 6 subcommand (init/next/verify/regate/step/status) の JSON contract scenarios 列挙
- F2b で先送りした cross-crate duplication (`engine_test regate_* vs belt-agent cli regate_*` 等) の統合判断
- `belt-agent` の e2e_test がシェル外依存 (`belt-agent --init` + FS state 作成) を持つため、M2 normalize 粒度は cli_test (unit) と e2e_test (integration) で別設計

**F3 推定規模**: F1 + F2a + F2b 合算と同等 (belt-agent tests が 48 test、subcommand 6 個の JSON contract で scenarios ~30-40 本)

### 将来 (F2a/F2b/F3 全て外、別 feature)

- audit-template.md v1 → v2 bump (新 Reason Label 必要時)
- coverage tool 導入 (`tarpaulin` / `cargo-mutants`)
- 未使用 dev-dep 活用 (`insta` snapshot / `pretty_assertions` diff / `rstest` parameterize)
- `docs/superpowers/specs/` 30+ verbatim path 参照の機械検証拡張
- monkey-test / dogfood pipeline 実行用の別系統 scenarios (現状 CLI scenarios は monkey-test 対象外)

### Linear tracking

- F1 は memory に Linear issue 番号記録なし (BELT-20 は parent tracking の 1 本)
- F2a / F2b / F3 を BELT-20 配下の子 issue で切るかは本 feature の scope 外、linear-refresh skill の責務
- 本 design は feature-dev pipeline (F2a) の deliverable 作成に集中、Linear 反映は別 operation
