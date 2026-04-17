# belt-test-foundation (F2b) — Test Strategy

F2b は test-only feature のため "テスト戦略" は二層構造: (1) **F2b 自身の成果物 (helper 抽出 / scenario 追加 / lock-ledger expansion / Decision Tree 適用) が正しく動作することを検証する戦略**、(2) **F2b が追加/変更する test 資産の品質 (ISTQB / ISO 25010 観点)**。本 doc は両層を扱う。

Design source: `docs/features/2026-04-17-belt-test-foundation-f2b/design.md`

## Test Design Techniques (ISTQB)

### 1. Equivalence Partitioning

**Application target**: helper signature / uri variant / GateCheck kind / BeltError variant

| 適用箇所 | Partition 分類 | 代表値 |
|---|---|---|
| `common/helpers.rs::write_yaml(dir, name, content)` | valid content / empty content / overflow content | `"name: x\n"` / `""` / 1MB string |
| `BeltUri::parse` | Run variant / Latest variant / WorkspaceLatest variant / invalid variant | `belt://run/<uuid>/path` / `belt://latest/pipeline/path` / `belt://workspace/branch/latest/pipeline/path` / `http://example.com` |
| `GateCheck::GitClean` XNOR | clean+expect_clean / dirty+expect_dirty / clean+expect_dirty / dirty+expect_clean / git CLI error | `(true, true, Ok)` / `(false, false, Ok)` / `(true, false, Ok)` / `(false, true, Ok)` / `(_, _, Err)` |
| `BeltError::VerifyRequired` Display | valid phase_id ASCII / empty string / Unicode / max length | `"build"` / `""` / `"構築"` / 255-char string |

**Expected test count**: P1 (helper) 3 × 4 partition = 12 cases、P3 (uri) 12 case、P4 (git_clean) 5 case、P5 (Display) 8 case

### 2. Boundary-Value Analysis

**Application target**: helper size / uri path length / git_clean edge / Display numeric

| Boundary | 対象 | 境界値 |
|---|---|---|
| `write_yaml` content size | empty / 1 byte / normal / 4KB / 1MB | 0, 1, 100, 4096, 1_048_576 bytes |
| `BeltUri::parse` path length | empty / 1 char / typical / 255 chars | `""` / `"x"` / `"notes/phase-design.md"` / 255-char |
| `GateCheck::GitClean` dirty file count | 0 (clean) / 1 / 100 / 1000 | `git status --porcelain` output lines |
| `MaxRetriesExceeded` ratio | `0/0` / `1/1` / `3/3` / `u32::MAX/u32::MAX` | attempts × max_retries |
| `belt-core.yml` scenario count | 5 (F2a uri baseline) / 12 (F2b target) | uri category size |

**Expected test count**: P3 境界 3 case、P4 境界 2 case、P5 境界 4 case

### 3. Decision Tables

**Application target**: git_clean XNOR / uri error matrix / F2b Decision Tree label application

**Table 1: `GateCheck::GitClean` → `GateResult.passed` / `detail`** (※ detail は production contract (gate.rs:272-277) で `is_clean` のみで分岐、`expect_clean` に依存しない)

| is_clean | expect_clean | git CLI | passed | detail |
|---|---|---|---|---|
| T | T | Ok | T | `"working tree clean"` |
| T | F | Ok | F | `"working tree clean"` (passed=false だが detail は clean) |
| F | T | Ok | F | `"N file(s) with uncommitted changes"` |
| F | F | Ok | T | `"N file(s) with uncommitted changes"` (passed=true だが detail は uncommitted) |
| - | - | Err (spawn failure via **non-existent work_dir**) | F | `"failed to run git: <os error>"` |

Note: Row 5 の `Err` trigger は **non-existent work_dir** (tempdir.close() 後 or 存在しない path)。非 git tempdir は `git status` exit=128 だが `Command::output()` は `Ok(output)` を返し、stdout=empty → is_clean=true 判定 → row 1 に fall through (production contract)。

**Table 2: `BeltUri::parse` → `Result<BeltUri, UriParseError>`**

| scheme | selector | pipeline/branch | path | result variant |
|---|---|---|---|---|
| belt | run | `<uuid>` | `notes/x.md` | `Ok(Run)` |
| belt | latest | `<pipeline>` | `notes/x.md` | `Ok(Latest)` |
| belt | workspace | `<branch>/latest/<pipeline>` | `x.md` | `Ok(WorkspaceLatest)` |
| https | * | * | * | `Err(MissingScheme)` |
| belt | unknown | * | * | `Err(UnknownSelector)` |
| belt | latest | "" | `x.md` | `Err(EmptyPipeline)` |
| belt | run | "" | `x.md` | `Err(EmptyRunId)` |
| belt | latest | `p` | "" | `Err(EmptyPath)` |
| belt | latest | `p` | `../x.md` | `Err(PathTraversal)` |
| belt | workspace | `b/not-latest/p` | `x.md` | `Err(Malformed)` |

**Table 3: Decision Tree (F2b label application)**

| Q1 (scenario 登録済) | Q2 (他 test 存在) | Q3 (behavior/shape/internal) | Q4 (trivial/tautology) | 結果 |
|---|---|---|---|---|
| yes | - | - | - | kept + `/// scenario:` |
| no | yes | - | - | `redundant-with-X` (delete) |
| no | no | shape lock | - | kept without scenario ID |
| no | no | internal (non-shape) | - | `implementation-coupling` (delete) |
| no | no | behavior | yes | `trivial-default-assertion` / `tautology` (delete) |
| no | no | behavior | no | Q5 → scenario 追加 + kept |

### 4. State-Transition Testing

**Application target**: git_clean 状態遷移 / uri roundtrip / helper re-invocation / F2b pipeline phase 進行

**Transition 1: git repo state transition (gate git_clean)**
```
clean → [touch file] → dirty → [git checkout --] → clean
```
Expected: gate evaluation が各状態で正しい (clean=pass when expect_clean=true, dirty=pass when expect_clean=false)

**Transition 2: uri parse-stringify roundtrip**
```
parse(s1) → BeltUri → to_string() → s2 → parse(s2) → BeltUri'
```
Invariant: `BeltUri == BeltUri'`, `s1 == s2` (idempotent)

**Transition 3: helper overwrite (write_yaml)**
```
write_yaml(dir, "p.yml", "v1") → write_yaml(dir, "p.yml", "v2")
```
Expected: 2 回目の call で content overwrite、file 1 個のみ存在

**Transition 4: F2b pipeline phase advance**
```
design → test-scenarios → spec-review → plan → pre-execute-handover → execute → code-review → integrate
```
Expected: 各 phase gate PASS 後に advance、narrative notes が design/plan/execute/code-review の 4 phase で生成

## Quality Characteristics (ISO 25010)

| Characteristic | Relevance | Reason |
|---|---|---|
| Functional Suitability | **in-scope** | F2b core: helper 抽出の correctness、uri 移植の coverage parity、git_clean coverage 充足、Decision Tree 適用精度 |
| Performance Efficiency | **in-scope** | scenarios_contract.rs 全 test < 2 秒、git_clean test × 5 で < 5 秒 |
| Compatibility | **in-scope** | Rust 1.94.1 toolchain、Cargo tests/common convention、workspace dep pin 維持 |
| Usability | **out-of-scope (reason: no UI, no CLI API change, test infrastructure only)** | — |
| Reliability | **in-scope** | 追加 test 全 deterministic、bash loop 50 回 pass 必須 |
| Security | **in-scope** | 新 dep 0、production code touch 0、test-only で injection surface ゼロ |
| Maintainability | **in-scope** | F3 作業者が audit-report.md + lock-ledger expanded entries を referable、`tests/common/*` pattern を拡張可能 |
| Portability | **in-scope** | x86_64 × aarch64 × linux × macOS cross-build、Windows MVP 非対象 |

## Priority Matrix

| Characteristic | Criticality | 根拠 |
|---|---|---|
| Functional Suitability | **critical** | F2b 全 item の correctness 担保、failure は手戻り直撃 |
| Reliability | **critical** | git_clean integration test が flaky 化すると CI fail で blocker |
| Maintainability | **high** | F3 handoff の base input、F2b audit-report.md 品質で F3 効率決定 |
| Compatibility | **high** | Cargo tests/common 慣例準拠、`#![allow(dead_code)]` 必須 |
| Performance Efficiency | **medium** | scenarios_contract 2 秒境界は余裕、現状 sub-second |
| Security | **medium** | new dep 0 で CVE 新規流入なし、test-only で surface 限定 |
| Portability | **low** | 既存 matrix 固定、追加 test は標準 library のみ |
| Usability | **out-of-scope** | — |

## Non-Functional Requirements

以下は F2b 完了 gate の測定可能 acceptance criteria:

1. **NFR-01 Performance**: `cargo test -p belt-core --test scenarios_contract --no-run` 後、compiled binary wall time < 2 秒 (warm-cache test runtime; cold-cache CI wall time は別概念で NFR-10 で扱う)
2. **NFR-02 Performance**: `cargo test -p belt-core --test gate_test git_clean -- --test-threads=1` が 4-5 test 合計 < 5 秒 (libtest filter は `--` より前、runner flag は `--` より後)
3. **NFR-03 Reliability**: `for i in {1..50}; do cargo test --workspace || exit 1; done; echo OK` exit 0
4. **NFR-04 Functional**: baseline 408 → F2b 完了時 workspace test count が **[409, 411]** レンジに収まる (arithmetic: Item 1 ±0 + Item 2 -5 + Item 3 +4〜5 + Item 6 +2〜3 = +1〜+3)
5. **NFR-05 Functional**: scenarios_contract の symmetric diff (yml ID set == rust doc-comment ID set) が全 commit で pass (category/file 配置は global set 照合のみ、category 対応は human-review)
6. **NFR-06 Maintainability**: F2b audit-report.md に 9 reason label 各使用頻度 + Side Findings section (Q3-B 副次発見) + Forward-to-F3 list が 3 category (belt-agent / cross-crate / binary helper) で cover
7. **NFR-07 Compatibility**: `cargo clippy --workspace -- -D warnings` clean (全 commit、common/mod.rs preamble で clippy::unwrap_used / expect_used / panic / dead_code を allow)
8. **NFR-08 Compatibility**: `cargo fmt --all -- --check` clean (全 commit)
9. **NFR-09 Security**: `cargo audit` / `cargo deny` 差分なし (F2a merge 基準)
10. **NFR-10 Portability**: CI runner (`ubuntu-latest` × `macos-latest`) で全 test green、`aarch64-unknown-linux-gnu` cross build pass、cold-cache wall time は measurement 対象外
11. **NFR-11 Integrity**: production code diff `git diff main -- 'crates/*/src/**' ':(exclude)crates/belt-core/src/uri.rs'` が empty (pathspec 除外、`grep -v` の文字列 match fragility 回避)。別途 `git diff main -- crates/belt-core/src/uri.rs` が `#[cfg(test)] mod tests` 削除 hunk のみであることを人間 review
12. **NFR-12 Integrity**: pilot 3 file (`cli_test.rs` / `config_test.rs` / `feature_dev_refresh.rs`) の assertion body 不変 (`git diff main -- <pilot files>` の test body diff が helper import と narrative refresh のみ)

## Must-Verify Mapping

design.md の 46 Must-Verify items (MV-01 〜 MV-46) を以下 10 Test Entries (TE) に分類:

### TE-A: Helper extraction (common/mod.rs + helpers.rs + narrative.rs [+ parity.rs])

**Covers**: MV-01, MV-02, MV-03, MV-04, MV-19, MV-20, MV-21, MV-22  
**Test Perspective**: P1 (3 helper × 4 観点 = 12 case)、P2 (narrative × 4 観点 = 16 case)  
**Technique**: Equivalence Partitioning + State-transition (helper overwrite)  
**Entry point**: 抽出後の既存 9+ file 全 test が pass、assertion body 不変 (`git log --oneline -p <file>` で helper import 差分のみ確認)  
**Pass criteria**:
- `common/mod.rs`, `common/helpers.rs`, `common/narrative.rs` が新規作成 (MV-01, MV-02, MV-03)
- 9+ 対応 file が `mod common;` + `use common::helpers::write_yaml;` 等の import 経由 (MV-04)
- `write_yaml` / `repo_root` / `fixture_path` の caller behavior 完全不変 (MV-19, MV-20, MV-21)
- `feature_dev_refresh.rs` / `bug_fix_refresh.rs` が narrative 4 helpers + 4 assert_* を使用、test count 不変 (MV-22)

### TE-B: uri integration migration + inline deletion

**Covers**: MV-05, MV-06, MV-14, MV-26, MV-30, MV-31  
**Test Perspective**: P3 (uri 12 variant × 4 観点)  
**Technique**: Equivalence Partitioning (10 variant) + Boundary-value (path length) + Decision Table (Table 2) + State-transition (Transition 2)  
**Entry point**: `cargo test -p belt-core --test uri_test` で 12 test pass + `cargo test -p belt-core --lib uri` で 0 test (inline 削除確認)  
**Pass criteria**:
- `src/uri.rs` line 174-310 の `#[cfg(test)] mod tests` 全削除、production code 不変 (MV-05)
- `uri_test.rs` が 5 → 12 test、7 edge case (parse_unknown_selector 等) 追加 (MV-06)
- `belt-core.yml` uri category が 5 → 12 scenarios (MV-14)
- 1:1 移植表が audit-report.md に記録、各 inline test の behavior が integration test で再現 (MV-26)
- inline 5 overlap = `redundant-with-<integration-id>` label (MV-30)
- inline 7 edge case = Q5 経由 kept、scenario 付き (MV-31)

### TE-C: gate git_clean coverage restoration

**Covers**: MV-07, MV-15, MV-27  
**Test Perspective**: P4 (5 variant × 4 観点)  
**Technique**: Equivalence Partitioning (5 variant) + Decision Table (Table 1) + State-transition (Transition 1)  
**Entry point**: `cargo test -p belt-core --test gate_test git_clean` で 4-5 test pass  
**Pass criteria**:
- `gate_test.rs` に git_clean test 4-5 本追加 (MV-07)
- `belt-core.yml` gate category が 9 → 13-14 scenarios (MV-15)
- clean+expect_clean / dirty+expect_dirty / clean+expect_dirty / dirty+expect_clean / git error の 5 variant cover (MV-27)

### TE-D: engine Display scenario promotion + parameterize

**Covers**: MV-08, MV-16, MV-24, MV-25, MV-29  
**Test Perspective**: P5 (Display 2 test × 4 観点)  
**Technique**: Equivalence Partitioning + Boundary-value (phase_id length, attempts ratio) + State-transition (format rewording)  
**Entry point**: `cargo test -p belt-core --test engine_test -- error_verify_required_message error_max_retries_exceeded_message`  
**Pass criteria**:
- 2 Display test に `/// scenario:` doc-comment 付与 (MV-08)
- `belt-core.yml` error category が 3 → 4-5 scenarios (MV-16)
- `error_verify_required_message`: `msg.starts_with("verify required for phase '")` 以上の semantic assertion、literal `"verify required"` 固定依存除去 (MV-24)
- `error_max_retries_exceeded_message`: `msg.contains(&format!("{attempts}/{max_retries}"))` で動的 format、literal `"3/3"` 固定依存除去 (MV-25)
- 2 test judgment = kept (Q5)、label なし (MV-29)

### TE-E: expander_with integration test addition

**Covers**: MV-09, MV-17, MV-28  
**Test Perspective**: P6 (expander with × 4 観点、non-overlap matrix あり)  
**Technique**: Equivalence Partitioning (string / bool / null 値) + Boundary-value (1 sub-phase / 10 sub-phase)  
**Entry point**: `cargo test -p belt-core --test expander_with_test`  
**Pass criteria**:
- `expander_with_test.rs` に integration test 2-3 本追加、既存 17 行 preamble 保持 (MV-09)
- `belt-core.yml` expander category が 4 → 6-7 scenarios (MV-17)
- public API `expand_pipeline` 経由の end-to-end、private helper 直接 call なし (MV-28)
- **Non-overlap matrix** (design.md P6 参照): 各新 test が inline 26 unit test の「対応 semantic」と「public boundary で新規 cover する内容」を明示。plan phase で table を確定

### TE-F: lock-ledger bug_fix_refresh stub expansion

**Covers**: MV-10, MV-18  
**Test Perspective**: P7 (ledger entry × 4 観点)  
**Technique**: Equivalence Partitioning (stub vs populated) + State-transition (F1 stub → F2b populated)  
**Entry point (machine)**: `cargo test -p belt-core --test scenarios_contract lock_ledger_locks_files_exist` — **locks-file: 行の file 存在のみ検証**  
**Entry point (human-review)**: test-fn-count / 19 named test-fn / 9 shape dimensions / cross-coupling — 機械検証対象外、reviewer が feature_dev_refresh template と同粒度か目視確認  
**Pass criteria**:
- [machine] `lock_ledger_locks_files_exist` が全 `locks-file:` 行で pass (MV-18)
- [human-review] `lock-ledger.md` の bug_fix_refresh.rs entry が stub "F2/F3 で同様の shape dimension 列挙" を削除、feature_dev_refresh template 並みに 19 test-fn-names + 9 shape dimensions 列挙 (MV-10)
- **Note**: test-fn-count の実値 drift (bug_fix_refresh.rs に test 追加されたのに ledger 未更新) は CI で検出されない、contributor rule として管理

### TE-G: audit-template wording correction

**Covers**: MV-11, MV-33  
**Test Perspective**: documentation only  
**Technique**: Equivalence Partitioning (valid wording vs stale fn name)  
**Entry point**: `cargo test audit_template_version_v1_matches_expected` + human read of audit-template.md  
**Pass criteria**:
- `audit-template.md` の Duplication Candidates 節で "parser_test.rs::parse_minimal_pipeline" 誤記を "parse_pipeline_from_file" 訂正、Decision Tree / reason labels 不変 (MV-11)
- `audit_template_version: v1` unchanged (scenarios_contract version assert pass)
- parser_test vs model_test の layer 分離根拠を audit-report.md に記録、keep-both 判定 (MV-33)

### TE-H: audit-report.md creation

**Covers**: MV-12, MV-32  
**Test Perspective**: P8 (label 集計 × 4 観点)  
**Technique**: Equivalence Partitioning (label frequency distribution)  
**Entry point**: 人間 review + frontmatter parse  
**Pass criteria**:
- `audit-report.md` frontmatter: `audited_at` / `audited_commit` / `audit_template_version: v1` (MV-12)
- 9 label 使用頻度 summary: 各 label の F2b 内 使用回数、使用されなかった label の enumeration (MV-32)
- Forward-to-F3 list: belt-agent / cross-crate / binary helper unification の 3 category (NFR-06 と同期)

### TE-I: Infrastructure correctness (cross-cutting)

**Covers**: MV-13, MV-23, MV-34, MV-35, MV-36, MV-37, MV-38, MV-39, MV-42, MV-44, MV-45, MV-46, MV-47, MV-48  
**Test Perspective**: NFR 全項  
**Technique**: State-transition (baseline → commit 1 → ... → final) + Decision Table (各 commit の green 条件)  
**Entry point**: 各 commit で `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --all -- --check`  
**Pass criteria**:
- `scenarios_contract` 全 test pass (MV-13)
- shape-lock 4 file の test fn 名 + `#[test]` 数が F2a merge 時点と同数、helper import 以外は touchless (MV-23)
- `cargo test --workspace` 全 pass、count **409-411** (MV-34、NFR-04)
- `cargo clippy --workspace -- -D warnings` clean (common/mod.rs preamble で test helper 由来 warnings 抑止) (MV-35、NFR-07)
- `cargo fmt --all -- --check` clean (MV-36、NFR-08)
- pilot 3 file の assertion body 不変 (MV-37、NFR-12)
- shape-lock 4 file の test count / assertion body 不変 (MV-38)
- production code diff zero with pathspec exclude (MV-39、NFR-11)
- baseline 408 pass を design commit 前に再確認 (MV-42)
- `docs/testing/README.md` 整合 (MV-44)
- `docs/superpowers/specs/**` verbatim path 維持 (MV-45)
- audit-template wording patch が cross doc と矛盾なし (MV-46)
- common/mod.rs preamble に test helper 由来 clippy lint allowances 宣言 (MV-47)
- git_clean spawn-failure variant が non-existent work_dir で trigger (MV-48)

### TE-J: Narrative notes + worktree + pipeline args

**Covers**: MV-40, MV-41, MV-43  
**Test Perspective**: workflow 整備  
**Technique**: Equivalence Partitioning (4 narrative phase) + State-transition (pipeline phase advance)  
**Entry point**: `ls .belt/runs/<run_id>/notes/` + `git branch --list` + `belt-agent status`  
**Pass criteria**:
- 4 narrative notes: phase-{design,plan,execute,code-review}.md 各 frontmatter + 4 節 (MV-40)
- `feature/2026-04-17-belt-test-foundation-f2b` branch current (MV-41)
- pipeline args `{codex: true, e2e: false}`、monkey-test / dogfood skip (MV-43)

### Mapping 網羅性確認

48 MV items が 10 Test Entries で全 cover されている (F2b spec-review で MV-47/48 追加):

| MV | TE | MV | TE | MV | TE |
|---|---|---|---|---|---|
| 01 | A | 17 | E | 33 | G |
| 02 | A | 18 | F | 34 | I |
| 03 | A | 19 | A | 35 | I |
| 04 | A | 20 | A | 36 | I |
| 05 | B | 21 | A | 37 | I |
| 06 | B | 22 | A | 38 | I |
| 07 | C | 23 | I | 39 | I |
| 08 | D | 24 | D | 40 | J |
| 09 | E | 25 | D | 41 | J |
| 10 | F | 26 | B | 42 | I |
| 11 | G | 27 | C | 43 | J |
| 12 | H | 28 | E | 44 | I |
| 13 | I | 29 | D | 45 | I |
| 14 | B | 30 | B | 46 | I |
| 15 | C | 31 | B | 47 | I |
| 16 | D | 32 | H | 48 | C/I |

網羅性: 48 / 48 = 100%

## Risk-based Test Prioritization

F2b の risk (design.md Impact Analysis Side Effect Risks) に基づく test 実行順序:

| Priority | Test Entry | Risk |
|---|---|---|
| P0 (must pass first) | TE-I Infrastructure | baseline + commit green = 全 TE の前提 |
| P0 (must pass first) | TE-A Helper extraction | 後続 TE-B〜F 全てが common/ import に依存、Phase B0 で最初に validate |
| P1 (high risk) | TE-B uri migration | 1:1 移植漏れで silent coverage loss、symmetric diff は semantic 不検証 |
| P1 (high risk) | TE-C git_clean | 新設 coverage gap 解消の core、git spawn flakiness リスク |
| P2 (medium) | TE-D Display promotion | parameterize で brittleness 除去、実装単純 |
| P2 (medium) | TE-E expander_with | layer 分離 (unit vs integration) が scope creep riskの main |
| P2 (medium) | TE-F lock-ledger | doc-only、`locks-file:` 機械照合のみ影響 |
| P3 (low) | TE-G audit-template | wording patch、v1 unchanged |
| P3 (low) | TE-H audit-report.md | 最終 deliverable、他 TE 完了後 |
| P3 (low) | TE-J narrative notes | 各 phase 完了時 by-product、workflow 側 |

## Execution Plan

1. **Phase B0 (commit 1)**: TE-A (helpers.rs), NFR-07/08/09 pre-commit
2. **Phase B1 (commit 2)**: TE-A (narrative.rs), same NFR
3. **Phase B2 (commit 3, optional)**: TE-A (parity.rs)、scope-skip 判断は plan phase
4. **Phase A1 (commit 4)**: TE-C git_clean
5. **Phase A2 (commit 5)**: TE-E expander_with
6. **Phase A3 (commit 6)**: TE-D Display
7. **Phase A4 (commit 7)**: TE-B uri migration (atomic: yml + integration + inline delete)
8. **Phase A5 (commit 8)**: TE-F lock-ledger
9. **Phase C1 (commit 9)**: TE-G + TE-H (audit-template patch + audit-report.md)
10. **Continuous**: TE-I infrastructure + TE-J narrative (各 commit / 各 phase)

**Test execution gating**:
- 各 commit で `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --all -- --check` 必須
- Phase A4 (uri atomic commit) の前後で `cargo test -p belt-core --lib uri` 0 test 確認 (inline 削除 effect)
- NFR-03 determinism test は integrate phase 前に 1 回実行

## Out-of-scope (明示)

- E2E testing (`--e2e=false` で monkey-test / dogfood skip、scenarios.yml 生成なし)
- UI testing (no UI surface)
- Load / stress testing (test infrastructure feature)
- Coverage measurement tool (`tarpaulin` / `cargo-mutants` は future feature)
- Mutation testing (F2b scope 外)
- Property-based testing (`proptest` / `quickcheck`) — 既存 test suite の設計原則 (ISTQB example-based) 踏襲
