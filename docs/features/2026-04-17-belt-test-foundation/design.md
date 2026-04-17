# belt-test-foundation (F1) — Design

F1 は「belt の test 資産を audit する多段 feature 群 (F1 → F2 → F3)」の**第 1 段**。目的は、(1) 既存 387 test を照合するための north star (scenarios.yml + lock 台帳 + audit template) を構築、(2) audit の methodology を pilot で検証、(3) F2/F3 が Rust コード変更中心で進められる土台を完成させる。F1 は **SSOT 構築 + 機械検証 + 最小 pilot** に集中し、実際の削除・統合・抽象化コード変更は F2 以降で行う。

## Architecture

3 層構造:

```
Layer 1: SSOT (docs/testing/)
    - docs/testing/README.md          目的・境界宣言
    - docs/testing/cli-behavior/*.yml    CLI behavioral SSOT (3 crate 分割)
    - docs/testing/lock-ledger.md     plugin shape lock test 台帳
    - docs/testing/audit-template.md  F2/F3 用 audit 判定手順

Layer 2: Binding (crates/belt-core/tests/scenarios_contract.rs)
    - scenarios.yml ↔ Rust doc-comment `/// scenario: <id>` の機械検証
      (walk scope = `crates/belt/tests/`, `crates/belt-agent/tests/`,
       `crates/belt-core/tests/` の全 `.rs` を recursive walk)
    - lock-ledger.md の `locks-file:` frontmatter と実ファイル存在検証

Layer 3: Pilot (3 test files + flaky fix)
    - crates/belt/tests/cli_test.rs        (doc-comment 付与のみ)
    - crates/belt-core/tests/config_test.rs (doc-comment 付与のみ)
    - crates/belt-core/tests/feature_dev_refresh.rs (lock-ledger 記録のみ、コード変更なし)
    - crates/belt-core/tests/view_test.rs  (thread::sleep → filetime 置換、flaky 先潰し)
```

### Data Flow

```
docs/testing/cli-behavior/{belt,belt-agent,belt-core}.yml
             │
             ├──(`/// scenario: <id>` doc-comment)──▶ tests/*.rs
             │
             └──(scenarios_contract.rs が両者を grep 照合)──▶ CI gate

docs/testing/lock-ledger.md ──(同 contract test で `locks-file:` 実在確認)──▶ CI gate
docs/testing/audit-template.md ──(人間参照、F2/F3 が consume)
```

### Deliverables

| # | Path | Type | Purpose |
|---|------|------|---------|
| 1 | `docs/testing/README.md` | new md | 目的 / `docs/features/` との境界 / 運用 entry point |
| 2 | `docs/testing/cli-behavior/belt.yml` | new yaml | `belt lint` CLI scenarios (pilot 範囲 = 5 test 相当) |
| 3 | `docs/testing/cli-behavior/belt-agent.yml` | new yaml | stub (F3 で拡充)、schema 準拠 + `scope:` top-level field 宣言 |
| 4 | `docs/testing/cli-behavior/belt-core.yml` | new yaml | `config` module scenarios (pilot 範囲 = 6 test 相当) + 他 module は `scope:` 宣言のみ + `scenarios: []` |

### Stub YAML skeleton (Deliverables #3 / #4 の他 module 分)

```yaml
# docs/testing/cli-behavior/belt-agent.yml — F1 stub
scope: "F3 で拡充予定。対象 = belt-agent CLI 全 subcommand (init, next, verify, regate, step, status) の JSON contract + state.json shape"
scenarios: []
```

```yaml
# docs/testing/cli-behavior/belt-core.yml — config module 本格 + 他 module stub
scope: "F2 で拡充予定。F1 では config module のみ本格 (6 scenarios)。F2 対象 = engine / view / lint / model / parser / expander / gate / error / uri の 9 module 公開 API"
scenarios:
  - id: belt-core-config-valid-toml-parse
    category: config
    severity: high
    technique: equivalence-partition
    given: "a valid belt.toml with pipeline_file field"
    when: "parse_config() is called"
    then: "returns Config with resolved pipeline path"
  # … config module の 6 scenario (F1 で実列挙)
```

`scope: <string>` は本 F1 で introduce する optional top-level field。既存 scenarios.yml schema に additive で追加 (`technique:` と同じく既存 consumer は ignore)。
| 5 | `docs/testing/lock-ledger.md` | new md | 5 既存 lock test + 1 新規 (scenarios_contract) の台帳 |
| 6 | `docs/testing/audit-template.md` | new md | F2/F3 用 audit 判定手順・decision tree・reason label 列挙 |
| 7 | `crates/belt-core/tests/scenarios_contract.rs` | new rs | binding 機械検証 lock test |
| 8a | `crates/belt/tests/cli_test.rs` | edit | `/// scenario: <id>` doc-comment 付与 |
| 8b | `crates/belt-core/tests/config_test.rs` | edit | `/// scenario: <id>` doc-comment 付与 |
| 9 | `crates/belt-core/tests/view_test.rs` | edit | `thread::sleep(20ms)` × 6 → `filetime::set_file_mtime` 置換 |

**+ feature-dev pipeline 通常出力:**
- `docs/features/2026-04-17-belt-test-foundation/design.md` (本 doc)
- `docs/features/.../test-strategy.md` (Phase 2)
- `docs/features/.../plan.md` (Phase 4)
- `docs/features/.../audit-report.md` (Phase 5 execute の産物、pilot 判定詳細)
- `.belt/runs/{run_id}/notes/phase-*.md` (narrative notes 6 phase)

### Non-Goals (F1 でやらないこと)

- belt-core の残り 9 モジュール (engine / view / lint / model / parser / expander / gate / error / uri) の scenarios 本格列挙 → F2
- belt-agent CLI の全 subcommand scenarios 列挙 → F3
- 実際の削除・統合・抽象化コード変更 (pilot 対象含む) → F2/F3
- 未使用 dep (`insta` / `pretty_assertions` / `rstest`) の活用 → 将来 feature
- coverage tool (`tarpaulin` / `cargo-mutants`) 導入 → 将来 feature
- `docs/superpowers/specs/` の 30+ verbatim file-path 参照の機械化 → 将来 doc-audit 拡張
- `tests/common/mod.rs` の新設 (helper 統合) → audit-template.md に候補記録、実施は F2 以降

## Prerequisites

1. **serde-saphyr `=0.0.23` ピン**: scenarios.yml の parse に使用 (workspace dep のまま、新規追加なし)
2. **workspace dev-dep 宣言済み**: `filetime` (未使用宣言あり) を view_test.rs で活用、`Cargo.toml` 編集不要
3. **Rust integration test 規約**: test ファイル冒頭の `#![allow(clippy::unwrap_used, expect_used, panic, ...)] reason = "..."` preamble を新規 `scenarios_contract.rs` にも付与
4. **`CARGO_MANIFEST_DIR + ../..` repo_root 慣習**: scenarios_contract.rs 内でも踏襲 (helper コピペは S2-Q3 方針で許容)
5. **`cargo test` 単独完結**: scenarios.yml ファイル読込は fs::read_to_string、外部 dep 不要
6. **Panic-on-mismatch**: scenarios_contract.rs の assertion は panic + clear message で統一
7. **scenarios.yml schema の additive 拡張**: 既存 required field (id/category/severity/given/when/then) + optional `preconditions` / `postconditions` は維持、**`technique: <istqb>` optional field + `scope: <string>` optional top-level field を追加**。`/belt:monkey-test` / `/dogfood` 既存 consumer は required field のみ読むので後方互換。`scope` は belt-agent.yml / belt-core.yml の stub module 用 self-documenting key
8. **feature_dev_refresh.rs pattern の踏襲**: scenarios_contract.rs は `REVIEW_SKILLS` 風の const table + for-loop assertion 構造を模倣
9. **Lock test の belt-core/tests/ 寄生配置維持**: scenarios_contract.rs も belt-core/tests/ 下に配置 (pragmatic、依存関係上自然)
10. **worktree `feature/2026-04-17-belt-test-foundation`**: `/worktrunk` で作成済、baseline `cargo test --workspace` = 387 pass 確認済
11. **`/belt:feature-dev` pipeline の phase sequence**: design → test-scenarios → spec-review → plan → execute → code-review → (monkey-test skip, --e2e=false) → (dogfood skip) → integrate

## Impact Scope

### File-level (write/edit)

- **新規**:
  - `docs/testing/README.md`
  - `docs/testing/cli-behavior/belt.yml`
  - `docs/testing/cli-behavior/belt-agent.yml`
  - `docs/testing/cli-behavior/belt-core.yml`
  - `docs/testing/lock-ledger.md`
  - `docs/testing/audit-template.md`
  - `crates/belt-core/tests/scenarios_contract.rs`
- **edit (doc-comment 追記のみ、behavior 不変)**:
  - `crates/belt/tests/cli_test.rs`
  - `crates/belt-core/tests/config_test.rs`
- **edit (sleep → filetime 置換、behavior 不変)**:
  - `crates/belt-core/tests/view_test.rs`
- **feature-dev 産物**: `docs/features/2026-04-17-belt-test-foundation/` 配下 + `.belt/runs/*/notes/*` narrative notes

### Module-level (behavior / contract 変化)

- **No production code change**: `crates/*/src/**` の全 module に変更なし (test-only feature)
- **Test surface 追加**: `scenarios_contract.rs` 1 file、4+ test fn 程度 (scenarios.yml parse / 全 scenario ID が doc-comment 参照先に存在 / doc-comment の ID が scenarios.yml に存在 / lock-ledger entry の実ファイル存在)
- **Test surface 改変**: view_test.rs は 6 箇所の内部 timing 実装変更のみ、assertion は変更なし

### Non-impacted (明示)

- `crates/belt-agent/` 以下: F1 scope 外、doc-comment も付与しない
- `plugins/belt/` 以下: scenarios.yml schema 理解のみ、SKILL.md 編集なし
- `.github/workflows/`: test name hard-code ゼロ、CI 変更なし
- `README.md` / `CHANGELOG.md` / `AGENTS.md`: test count / 名称言及ゼロ、更新不要
- `Cargo.toml` / `Cargo.lock`: dep 追加なし

## Impact Analysis

### Reverse Dependencies

| caller | target | coupling | reason |
|---|---|---|---|
| `crates/belt-core/tests/shared_filter_parity.rs:78-89` | `plugins/belt/agents/*.md` | strong | 同 plugin tree を読む。feature_dev_refresh 系と同時メンテ必要 |
| `crates/belt-core/tests/shared_criteria_parity.rs:1-53` | `plugins/belt/skills/{feature-dev,bug-fix}/criteria/{execute,code-review}.md` | strong | plugin directory tree 共有 = 同時改変検知 |
| `crates/belt-core/tests/bug_fix_refresh.rs:327-386` | `feature_dev_refresh.rs:239-386` の tuple pattern | weak (structural) | helper 名 (`find_phase`/`find_produce`/`has_file_exists_gate`/`has_named_consume`) byte-identical コピペ |
| `crates/belt-core/tests/review_skills_refresh.rs:32-50` | 上記 tuple pattern | weak (structural) | parallel structure family |
| `crates/belt-core/src/config.rs:35 resolve_pipeline_path` | `config_test.rs:8,60,71` + `belt/src/main.rs:36` + `belt-agent/src/main.rs:136` | direct | config_test 削除で production caller の behavior lock が外れる (F1 では削除せず doc-comment 付与のみ) |
| `crates/belt/src/main.rs` (lint CLI) | `crates/belt/tests/cli_test.rs:5 tests` | direct | lint CLI contract lock の唯一点 (F1 では削除せず doc-comment 付与のみ) |
| `docs/superpowers/plans/2026-04-1X-*.md` 30+ 箇所 | 3 pilot file path (verbatim) | weak (historical) | rename で stale link 発生 (F1 では rename なし = 無影響) |

### Shared State

| kind | constraint | usage |
|---|---|---|
| FS (plugin tree) | `plugins/belt/skills/feature-dev/pipeline.yml` + parallel | feature_dev_refresh / bug_fix_refresh / shared_criteria_parity 同時読込 |
| FS (agent bundle) | `plugins/belt/agents/*.md` | shared_filter_parity (byte-identity) + review_skills_refresh (存在性) |
| Workspace dev-deps | `filetime` / `insta` / `pretty_assertions` / `rstest` 宣言済み・未使用 | F1 は `filetime` のみ活用、他は未来 |
| Test helpers | `CARGO_MANIFEST_DIR + ../..` = repo_root | 5+ file で byte-identical コピペ (F1 では scenarios_contract.rs も踏襲 = 6 コピー化、台帳に記録) |
| Config contract | `BeltError::FileNotFound` / `ConfigParse` variant 区別 | `config_test.rs` で lock、production は `belt/src/main.rs:36` / `belt-agent/src/main.rs:136` で消費 |
| CI | `.github/workflows/release.yml` は `cargo test` を呼ばない (cargo-dist 専用) | test name 変更耐性あり、F1 pilot の doc-comment 追加は無影響 |

### Implicit Contracts

| file:line | dependency | violation impact |
|---|---|---|
| `crates/belt/tests/cli_test.rs:40,66,106` | `belt lint` stderr = "ok" / "duplicate" 含む | CLI format 変更で 5 test 全滅 (F1 で lock 明示化) |
| `crates/belt/tests/cli_test.rs:65,76,121` | invalid / nonexistent / config+positional で exit code = 1 (特定 1、2 以外) | 一般 non-zero 化で fail、CI / LLM runner にも影響 |
| `crates/belt-core/tests/config_test.rs:27,40,50` | `BeltError::FileNotFound` / `ConfigParse` variant match | error enum 変形で全滅 |
| `crates/belt-core/tests/feature_dev_refresh.rs:243-269` | 6 narrative phase id + artifact_name + path pattern | pipeline.yml narrative path 1 箇所の変更で fail |
| `crates/belt-core/tests/feature_dev_refresh.rs:175` | `args = {codex, e2e}` exactly | args 追加で即 fail |
| `plugins/belt/skills/test-scenarios/SKILL.md:51-63` scenarios.yml schema | required field (given/when/then/id/category/severity) | schema 削減で既存 consumer (`/belt:monkey-test`) 破綻 (F1 は additive `technique` 追加のみで回避) |
| `plugins/belt/skills/monkey-test/SKILL.md:38-39` | scenarios.yml 存在で screenshots 生成前提 | F1 の CLI scenarios.yml を monkey-test に流すと全 SKIP (運用上は「CLI scenarios は monkey-test の対象外」を README で宣言) |

### Side Effect Risks

| severity | trigger | impact |
|---|---|---|
| high | CLI scenarios.yml を書いたが `docs/testing/README.md` で「monkey-test 対象外」を宣言し忘れ | F2/F3 で誰かが `--e2e` + monkey-test 流し、全 SKIP を「bug」として処理し無駄な debug 発生 |
| high | `scenarios_contract.rs` が grep ベース → doc-comment の誤字 (`/// senario:`) を検知できない可能性 | drift が silent pass。対策: scenarios_contract.rs で `/// scenario: ` 正確一致を正規表現で強制 + scenarios.yml ID と完全一致 test |
| high | `filetime::set_file_mtime` 置換でシステム依存挙動 (Windows / ネット FS / tmpfs) | F1 MVP は Unix 前提 (CLAUDE.md 明示)、Windows CI 対象外。ネット FS 利用ケース発生時は test 側で skip 判定を追加 (将来) |
| medium | pilot 3 ファイルに doc-comment 付与する際、既存 Rust doc-comment (あれば) と衝突 | 付与前に `grep '///' cli_test.rs` 等で既存 comment 確認、重複しない |
| medium | `docs/testing/lock-ledger.md` と Rust の実 test の binding が md なので mechanical check なし | scenarios_contract.rs で台帳 entry 毎の Rust file path 存在確認 test を 1 個入れる (台帳側に `locks-file: <relative-path>` frontmatter 追加) |
| medium | `docs/testing/` と `docs/features/<topic>/` の責務 drift | README.md で明示境界宣言 (CLI behavioral SSOT / lock meta は `docs/testing/`、feature scope の scenarios は `docs/features/<topic>/scenarios.yml`) |
| low | `scenarios_contract.rs` の lock test が `serde_saphyr::from_str` で scenarios.yml を parse → schema 要件と実態差 | F1 では minimal schema + `technique` optional でシンプル、scope creep 抑制 |
| low | view_test.rs `filetime` 置換時の既存 assertion への副作用 | 置換前後で `cargo test -p belt-core --test view_test` green を CI blocker 扱い |

## Must-Verify Checklist

### 基本構造 (deliverable 存在と shape)

- [ ] **MV-01**: `docs/testing/README.md` 存在、`docs/testing/` の責務 (CLI behavioral SSOT / lock meta) と `docs/features/<topic>/` との境界を明記
- [ ] **MV-02**: `docs/testing/cli-behavior/belt.yml` 存在、scenarios.yml schema に準拠 (id/category/severity/given/when/then 必須 + optional technique)
- [ ] **MV-03**: `docs/testing/cli-behavior/belt-agent.yml` 存在 (F1 では stub + scope 宣言のみ許容、schema 準拠)
- [ ] **MV-04**: `docs/testing/cli-behavior/belt-core.yml` 存在、`config` module scenarios 本格 + 他 module stub、schema 準拠
- [ ] **MV-05**: `docs/testing/lock-ledger.md` 存在、5 既存 lock test + 1 新規 (scenarios_contract) の entry、各 entry に `locks-file:` frontmatter
- [ ] **MV-06**: `docs/testing/audit-template.md` 存在、decision tree + reason label 列挙 + duplication 候補表
- [ ] **MV-07**: `crates/belt-core/tests/scenarios_contract.rs` 存在、workspace 全体の `cargo test` で 4+ test pass

### SSOT ↔ Rust binding 機械検証

- [ ] **MV-08**: `scenarios_contract.rs` が 3 scenarios.yml を全て parse 成功 (serde_saphyr)
- [ ] **MV-09**: scenarios.yml 内の全 scenario ID が Rust source `/// scenario: <id>` doc-comment で参照されている (正規表現 `^\s*///\s+scenario:\s+\S+\s*$` で正確一致)
- [ ] **MV-10**: Rust source の `/// scenario: <id>` が全て scenarios.yml 内に対応 entry を持つ (逆方向)
- [ ] **MV-11**: lock-ledger.md の各 entry の `locks-file:` frontmatter で指定された test ファイルが実在
- [ ] **MV-12**: scenarios_contract.rs 自身が誤 typo (`senario` / `scenraio` 等) を catch する test を持つ (grep 結果と scenarios.yml の完全一致で間接的に検知)

### Pilot audit 判定 (doc-comment 付与)

- [ ] **MV-13**: `crates/belt/tests/cli_test.rs` の 5 test 全てに `/// scenario: <belt-id>` doc-comment 付与
- [ ] **MV-14**: `crates/belt-core/tests/config_test.rs` の 6 test 全てに `/// scenario: <belt-core-id>` doc-comment 付与
- [ ] **MV-15**: pilot 判定結果が `docs/features/2026-04-17-belt-test-foundation/audit-report.md` に記載 (各 test に kept / deleted / merged / abstracted + reason)
- [ ] **MV-16**: 「kept」判定 test は対応 scenario ID を持ち、「deleted」判定なら named reason (redundant-with-X / trivial-default / tautology 等) が audit-template.md の許容ラベル集と一致

### Lock pilot (feature_dev_refresh.rs)

- [ ] **MV-17**: `feature_dev_refresh.rs` は**コード変更せず、lock-ledger.md に entry 追加のみ** (S2-Q5 方針)
- [ ] **MV-18**: lock-ledger.md の `feature_dev_refresh.rs` entry に (A) **`feature_dev_refresh.rs` の 11 個 `#[test]` 関数名** (`feature_dev_pipeline_parses_successfully`, `feature_dev_args_are_exactly_codex_and_e2e`, `feature_dev_narrative_phases_produce_notes`, …) を列挙、かつ (B) それら test fn が lock する pipeline.yml の側面 (args {codex,e2e} set / narrative 6 phase id / max_retries / `scenarios.when = args.e2e` typed / regate=[execute]) を dimension name で列挙、(C) cross-file coupling = shared_criteria_parity / shared_filter_parity / bug_fix_refresh / review_skills_refresh を明記

### Flaky 先潰し (view_test.rs)

- [ ] **MV-19**: `view_test.rs` の `thread::sleep(Duration::from_millis(20))` 6 箇所を `filetime::set_file_mtime` で置換。strict ordering を仮定する assertion 箇所は **mtime delta 2 秒以上** に設定し、macOS HFS+ の 1 秒 granularity でも順序保証 (weak ordering 箇所も decisive に differ する値を設定)
- [ ] **MV-20**: 置換後 `for i in {1..100}; do cargo test -p belt-core --test view_test --quiet || exit 1; done; echo OK` が 100 回連続 pass (fail 検知は `|| exit 1` で即時 propagate、silent pass なし)

### 一貫性・regression

- [ ] **MV-21**: `cargo test --workspace` が全 green
- [ ] **MV-22**: `cargo clippy --workspace -- -D warnings` が clean
- [ ] **MV-23**: `cargo fmt --all -- --check` が clean
- [ ] **MV-24**: `feature_dev_refresh.rs` / `bug_fix_refresh.rs` / `review_skills_refresh.rs` / `shared_criteria_parity.rs` / `shared_filter_parity.rs` を未改変、これら test は全 green

### Narrative notes

- [ ] **MV-25**: `.belt/runs/{run_id}/notes/phase-design.md` 存在、frontmatter (phase: design / run_id: ...) + 4 節 (Decisions / Concerns / Directives / Observations)
- [ ] **MV-26**: plan / execute / code-review phase 完了時に各 note 作成される (Phase 3/5/6 の責務、F1 design gate ではまだ未検証、参考として列挙)

### worktree

- [ ] **MV-27**: `feature/2026-04-17-belt-test-foundation` branch が git 上に存在 (`git branch --list`)
- [ ] **MV-28**: baseline `cargo test --workspace` = 387 pass、design.md commit 前に確認済

### doc-drift 予防

- [ ] **MV-29**: `docs/testing/README.md` が CLAUDE.md / AGENTS.md の「docs/ 構造」節 (存在すれば) と矛盾しない
- [ ] **MV-30**: 30+ spec/plan が verbatim 参照する 3 pilot file path は F1 で改名しない (既存 link 維持)。検証: Phase 5 execute 最初のタスクで `grep -rl "cli_test.rs\|config_test.rs\|feature_dev_refresh.rs" docs/ | sort > .belt/runs/{run_id}/artifacts/mv30-before.txt` を capture、Phase 6 code-review で `grep -rl ... > after.txt && diff mv30-before.txt after.txt` = empty を確認

### 追加 MV (CCS-05 / CCS-06 由来)

- [ ] **MV-31** (CCS-05): `view_test.rs` の assertion 行 (`assert_eq!` / `assert!` / `assert_ne!`) の tokenization が置換前後で diff ゼロ。`filetime::set_file_mtime` call と `thread::sleep` 削除のみが差分で、assertion ロジック (comparison operator / expected field set / target value source) は不変
- [ ] **MV-32** (CCS-06): `audit-report.md` frontmatter に `audited_at: <ISO 8601 UTC>` / `audited_commit: <F1 執筆時 HEAD sha>` / `audit_template_version: v1` が記載。scenarios_contract.rs 側で frontmatter 存在 + `audit_template_version` が audit-template.md の宣言 version と一致するかを assertion
- [ ] **MV-33** (CCS-06): `audit-template.md` に F2 着手時の re-audit trigger 手順 (`git log --since="<audited_at>" -- <pilot_file>` が非空なら pilot audit 再実施) を記載

## Test Perspectives

F1 deliverables を入力 parameter として、Normal / Boundary / Abnormal / State-transition の 4 観点で test 対象を列挙。Phase 2 (test-scenarios) はこれを元に `test-strategy.md` を書き、Phase 5 execute 中の `scenarios_contract.rs` がこれらを assertion 化する。

### P1: `docs/testing/cli-behavior/*.yml`

| 観点 | 対象 case |
|---|---|
| Normal | 全必須 field (id, category, severity, given, when, then) が valid、optional `technique` あり/なし両方 |
| Boundary | scenarios array が**空 (0 entries) の .yml**、**1 entry のみ**、**ID 最大長 (kebab-case 推奨最長)** |
| Abnormal | `severity` が enum 外 (`trivial` 等) / required field 欠落 / YAML syntax error / unknown top-level key |
| State-transition | scenario A が `.yml` に追加された直後 (Rust doc-comment 未追加状態) / scenario A が `.yml` から削除された直後 (Rust doc-comment 残存状態) |

### P2: Rust doc-comment `/// scenario: <id>`

| 観点 | 対象 case |
|---|---|
| Normal | `/// scenario: belt-lint-valid-pipeline` が `#[test]` fn 直上 |
| Boundary | 複数 scenario を 1 test に紐付け (`/// scenario: X` + `/// scenario: Y`、1 test ↔ N scenarios 許容) / 0 doc-comment (全 scenarios.yml 未整備の test はこの状態で pass 許容) |
| Abnormal | typo (`/// senario:` / `/// scenraio:`) / leading slash 欠 (`scenario: X`) / 余計な prefix (`// /// scenario: X`) |
| State-transition | doc-comment 存在するが scenario ID が `.yml` に存在しない (orphan Rust 参照) / `.yml` に ID あるが Rust 参照ゼロ (orphan scenario) |

### P3: `docs/testing/lock-ledger.md` entries

| 観点 | 対象 case |
|---|---|
| Normal | frontmatter `locks-file: crates/belt-core/tests/feature_dev_refresh.rs` 指定、内容 body に lock 対象列挙 |
| Boundary | ledger に entry 1 個のみ / entry 多数 |
| Abnormal | `locks-file:` で指定したパスの実ファイル不在 / frontmatter 欠落 / 相対パス基準混在 (repo root 基準 vs crate 基準) |
| State-transition | entry 追加されたが対応 Rust test まだ未追加 / Rust test 削除されたが ledger entry 残存 |

### P4: `scenarios_contract.rs` (binding 機械検証 lock)

| 観点 | 対象 case |
|---|---|
| Normal | 3 scenarios.yml 全て valid、doc-comment 全て整合、ledger 全て実在 → 全 test pass |
| Boundary | scenarios.yml がゼロ件 entry でも schema valid なら accept / 数百 scenario でも parse 完走 |
| Abnormal | scenarios.yml parse 失敗時に明確な panic message / grep 正規表現の false positive なし (例: `/* /// scenario: X */` コメントアウト行を拾わない) |
| State-transition | scenario ID 追加 → grep 時に Rust 側未追加を検知、明確な diff message |

### P5: `view_test.rs` flaky fix (thread::sleep → filetime)

| 観点 | 対象 case |
|---|---|
| Normal | 置換後 `cargo test --test view_test` で全 41 test pass |
| Boundary | mtime を epoch 0 / max i64 / 1 ns 差の境界に設定、assertion 挙動保持 |
| Abnormal | `filetime::set_file_mtime` が Unix 以外の FS (NFS / tmpfs) で失敗 → test 側で `#[cfg(unix)]` gate は不要 (workspace MVP が Unix 限定、CLAUDE.md 明示) |
| State-transition | mtime 操作順序 (produce phase 開始 → 書込 → read) が既存 assertion と一致 |

### P6: `audit-template.md` (reason labels — v1 fixed enumeration)

F1 で固定する v1 label 集合 (9 個):

1. `redundant-with-<test-id>` — 他 test が同 behavior をカバー
2. `trivial-default-assertion` — default 値確認のみで情報量ゼロ
3. `tautology` — assertion が論理的常真
4. `state-transition-overlap-with-<test-id>` — state transition が既存 test と重複
5. `implementation-coupling` — private state を assert、behavior でない
6. `brittle-format-match` — 出力 format 軽微変更で fail する fragile assertion
7. `dead-fixture` — fixture 生成のみで実効検証なし
8. `unreachable-guard` — 入力ドメインに存在しない case を守る
9. `obsolete-spec` — 仕様変更で lock 対象が消失したが test 残存

F2/F3 で新 label が必要なら別 feature で audit-template.md を SemVer 風 migration (audit-report frontmatter の `audit_template_version` bump) で update。F1 では v1 固定。

| 観点 | 対象 case |
|---|---|
| Normal | v1 9 label のみが audit-report で使用され、kept 以外の全 test に label 割当 |
| Boundary | 9 label 全てが少なくとも 1 pilot test で使用される (happy case) / 1 label のみ使用 (偏り) |
| Abnormal | audit-report で v1 9 label 以外を使用 → scenarios_contract.rs 補助 test で検出 (`audit_template_version` と label 集合の整合 check) |
| State-transition | v1 → v2 migration (F2/F3 feature) 時に audit-template + audit-report + scenarios_contract.rs lock test が同期更新 |

### Non-Functional Requirements

| 特性 | 要件 | 計測 |
|---|---|---|
| Performance | `scenarios_contract.rs` 単独実行が < 1 秒 (lint スケール) | `cargo test --test scenarios_contract -- --nocapture` 実時間 |
| Maintainability | audit-template.md + lock-ledger.md が F1 作者不在で F2/F3 実行可 | handover + 外部 reviewer (Phase 3 spec-review) で妥当性確認 |
| Determinism | view_test.rs 置換後、100 回連続 `cargo test -p belt-core --test view_test` で 100/100 pass (fail 検知は `\|\| exit 1` で即時 propagate) | bash loop `for i in {1..100}; do cargo test ... \|\| exit 1; done; echo OK` で実測、MV-20 |
| Portability | `x86_64-unknown-linux-gnu` / `aarch64-unknown-linux-gnu` (glibc >= 2.35) + `x86_64-apple-darwin` / `aarch64-apple-darwin` (macOS 14+)。dist-workspace.toml の 4 target triple と粒度整合。Windows は MVP scope 外 | CLAUDE.md 方針継承、release.yml の cross build pass + ローカル `cargo test --workspace` pass を合算 signal (CI test matrix は future work) |
| Security | docs + test-only feature、production code 変更なし、新 dep ゼロ → 供給チェーン / 秘密情報影響なし | `cargo audit` / `cargo deny` 結果に差分なしで足りる |
| Accessibility | internal developer tool、外部非公開 | N/A |

### Quality Bar 適合チェック

- 全 input parameter (P1-P6) に Normal / Boundary / Abnormal / State-transition 各 1 件以上 cover
- Phase 3 test-scenarios expansion 時に Given/When/Then 形式に移植可能
