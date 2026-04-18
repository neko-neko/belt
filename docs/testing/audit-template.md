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
 ├── internal assert (shape lock: `*_refresh.rs` / `*_parity.rs` / `scenarios_contract.rs`) → judgement = kept without scenario ID (shape lock test は behavior scenario の対象外で、plugin/pipeline の構造自体を固定する役割)
 ├── internal assert (shape lock 以外) → judgement = implementation-coupling  (→ 削除 or 抽象化)
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
| `parser_test.rs::parse_pipeline_from_file` | `model_test.rs::parse_minimal_pipeline` | 誤記訂正 (F2b audit 2026-04-18): 実 fn は `parse_pipeline_from_file` (file-I/O + parse layer) で `model_test::parse_minimal_pipeline` (serde_saphyr 直接、model layer) と layer 分離された complementary test、redundant ではない。F2b では keep-both 判定 |
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

### Trigger 対象外 (F2a で明示)

「touch されている」判定は **既存 test fn の modify** を指す。以下は re-audit trigger の対象外:

- pilot file に**新規 test fn を追加**する変更 (既存 fn の assertion / setup / teardown 変更なし)
- pilot file に doc-comment (`/// scenario: <id>`) を付与する変更 (behavior 不変)
- pilot file の preamble (`#![allow(...)] reason = "..."`) 更新

理由: 新規追加 fn は F2a の「復帰 scenario + 対応 test」pattern であり、F1 pilot 判定済 fn の挙動を変えないため re-audit は不要。逆に、既存 fn の body 変更 (assertion 差し替え / fixture 変更 / 実装 side 変更による意味の shift) のみが audit 結果 stale 化の signal となる。
