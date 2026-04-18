---
audited_at: 2026-04-18T00:00:00Z
audited_commit: 2625fbb
audit_template_version: v1
---

# belt-test-foundation F2b Audit Report

F2b audit 結果。F2a で確立した SSOT + binding realization の上に、Forward-to-F2b 7 items を Decision Tree + label で処理し、belt-core 内 helper consolidation と coverage gap を解消した。

## Methodology

- Forward list (F2a audit-report.md `Forward-to-F2b list`) を Decision Tree Q2-Q5 + 9 reason labels で judged
- Q3-B の副次発見は Side Findings section に集計
- engine Display 2 test の `brittle-format-match` label (F2a) は parameterize (literal → dynamic format) により解消 → Q3 behavior → Q5 kept に routing

## Label Frequency Summary (F2b)

| Label | 使用回数 | 対象 |
|---|---|---|
| `redundant-with-<X>` | 5 | uri inline 5 overlap tests |
| `kept (Q5 promoted)` | 2 | engine Display 2 tests (with parameterize) |
| `kept-without-scenario-id` | 0 | (F2b では shape-lock 追加変更なし、既存 4 file 維持) |
| `brittle-format-match` | 0 | F2a forward の 2 test を F2b で resolved |
| `implementation-coupling` | 0 | (F2b 対象なし) |
| `trivial-default-assertion` | 0 | (F2b 対象なし) |
| `tautology` | 0 | (F2b 対象なし) |
| `state-transition-overlap-with-<X>` | 0 | (F2b 対象なし) |
| `dead-fixture` | 0 | (F2b 対象なし) |
| `unreachable-guard` | 0 | (F2b 対象なし) |
| `obsolete-spec` | 0 | (F2b 対象なし) |

**Unused labels**: 8 labels unused in F2b. v2 bump は future signal、F2b で新 label 要求なし。

## Judgment per item

### Item 1: engine Display 2 tests

- `error_verify_required_message`: **kept (Q5)**. Scenario `belt-core-error-display-verify-required-preserves-phase-id` を追加、assertion parameterize (`starts_with` → `contains(&format!("for phase '{phase_id}'"))`) で 動的 phase_id lock
- `error_max_retries_exceeded_message`: **kept (Q5)**. Scenario `belt-core-error-display-max-retries-preserves-phase-id-and-counter` を追加、assertion parameterize で phase_id + ratio 両 lock
- F2a forward label `brittle-format-match` → F2b parameterize により解消、Decision Tree Q3 behavior branch に routing

### Item 2: uri inline 12 → integration 12 (migration)

- Inline 5 overlap tests (`parse_latest_happy_path`, `parse_workspace_latest_happy_path`, `parse_run_happy_path`, `parse_missing_scheme`, `parse_path_traversal_rejected`): **`redundant-with-<integration-id>`** + delete。同等 behavior を integration 側が cover
- Inline 7 edge case tests (`parse_unknown_selector`, `parse_empty_pipeline`, `parse_empty_run_id`, `parse_empty_path`, `parse_absolute_path_rejected`, `parse_workspace_missing_latest`, `to_string_roundtrip_all_variants`): **kept via Q5** (scenario 追加 + integration 移植完了後に inline delete)。label なし、behavior は新 integration 7 tests で lock
- `src/uri.rs` line 174-310 の `#[cfg(test)] mod tests` ブロック全削除、production code unchanged

### Item 3: gate git_clean coverage restoration

- 既存 test ゼロ → **new tests** 5 本追加 (clean/expect_clean、dirty/expect_dirty、clean/expect_dirty、dirty/expect_clean、missing-work_dir spawn failure)
- Scenario 5 本追加、category `gate` が 9 → 14 scenarios
- Judgment: **additive only** (既存 test 未 audit、新設 coverage)

### Item 4: Duplication Candidates 統合 (belt-core 内)

- `write_yaml` / `repo_root` / `fixture_path` 3 helpers を `tests/common/helpers.rs` に集約 (9 file で import 書き換え)
- narrative helpers (`find_phase` / `find_produce` / `has_file_exists_gate` / `has_named_consume` + 4 assert_*) を `tests/common/narrative.rs` に集約 (feature_dev_refresh / bug_fix_refresh の 2 file で使用)
- Parity helper (`read_workspace_file`) 抽出: **skipped** (scope 縮小、shared_filter_parity / shared_criteria_parity は独立保持で十分と判断)
- parser_test vs model_test: audit-template.md の Duplication Candidates 記述 "parser_test.rs::parse_minimal_pipeline" は誤記 (実 fn は `parse_pipeline_from_file`)。F2b で audit-template wording correction patch を適用、両 test は layer 分離された complementary test として **keep both**、`redundant` 判定せず
- cross-crate duplication (`engine_test regate_* vs belt-agent cli regate_*` 等) は F3 送り

### Item 5: lock-ledger bug_fix_refresh entry expansion

- lock-ledger.md の bug_fix_refresh.rs entry が F1 で stub 状態 ("F2/F3 で同様の shape dimension 列挙を行う") だったのを feature_dev_refresh.rs template 並みに expansion (+35-40 行、19 test-fn names + 17 shape dimensions + 2 cross-coupling)
- `scenarios_contract::lock_ledger_locks_files_exist` は `locks-file:` field のみ machine-check、shape dimensions / test-fn-count / cross-coupling は human-review content

### Item 6: expander_with_test.rs 0 test 解消

- 既存 17 行 tombstone preamble 保持、**3 integration tests** 追加:
  - `expand_pipeline_with_string_substitution_end_to_end` (string value propagation)
  - `expand_pipeline_with_bool_and_null_substitution_preserves_types` (type preservation)
  - `expand_pipeline_parent_scope_not_rewritten_by_sub_substitution` (parent-scope isolation、memory `feedback_expander_parent_scope_rule.md` rule lock)
- Public API `expand_pipeline` 経由の end-to-end、src/expander.rs inline unit test (26 本) と scope 分離
- Scenario 3 本追加、category `expander` が 4 → 7 scenarios

### Item 7: Decision Tree + 9 label application

- engine Display 2 tests / uri inline 5 overlap / uri inline 7 edge case / parser_test vs model_test (keep both) 全てに Decision Tree 適用、label 付与 or Q5 経由 kept 判定
- Side Findings section: (none discovered — 副次 test で delete 判定を要するものなし)

## Forward-to-F3 list

F3 (belt-agent behavior SSOT + cross-crate duplication) で扱う項目:

### F3 scope (belt-agent)

- `crates/belt-agent/tests/cli_test.rs` (40 test) の audit
- `crates/belt-agent/tests/e2e_test.rs` (8 test) の audit
- `docs/testing/cli-behavior/belt-agent.yml` 拡充 (6 subcommand JSON contract scenarios 30-40)

### F3 scope (cross-crate duplication)

- `engine_test regate_*` (14) vs `belt-agent cli regate_*` (11) — API layer vs CLI JSON layer
- `engine_test verify_verdict_*` vs `belt-agent cli verify_*` — verify pass/fail semantics
- `view_test engine_enriched_status_*` vs `belt-agent cli status_*` — view module API vs CLI

### F3 scope (binary crate helper unification)

- `belt/cli_test.rs` + `belt-agent/cli_test.rs` の `write_yaml` variant B (fs::write 2-line form) — Cargo cross-crate `tests/common` 制約を考慮した処置判断

### F3 scope (post-audit findings, surfaced during F2b code-review)

以下 4 項目は `2625fbb` audit point の後、F2b code-review phase で `/belt:code-review --codex` および `belt-agent:phase-auditor` が surface したもの。F2b では out-of-scope deferred として処理:

- **codex finding**: `crates/belt-agent/src/git.rs::tests::current_branch_returns_name_for_git_dir` lacks hermetic git config isolation. Inherits user's `commit.gpgsign` + `gpg.format=ssh`, which fails in env without an SSH-signing agent. Fix: thread `-c commit.gpgsign=false` into the `git commit` invocation, or `git -c gpg.format=openpgp -c commit.gpgsign=false`. F2b で再現せず (signing path 通過)、env-dependent。F3 belt-agent test audit 配下で扱う
- **cross-cutting finding**: `crates/belt-core/tests/shared_criteria_parity.rs::workspace_path` helper が `repo_root().join(rel)` の lone holdout (F2b で 4 sibling 移植済の唯一未移植 file)。F3 belt-agent test audit が同 file 周辺を touch する際にまとめて移植
- **phase-auditor finding (CODE-REVIEW-07)**: `/belt:code-review` が出力する merged `findings.json` envelope が per-finding `disposition` / `disposition_rationale` field を持たない。triage state は narrative の Decisions section の自然言語のみで保持され、自動 audit が natural-language cross-referencing 必須 — brittle。`plugins/belt/skills/code-review/SKILL.md` の merge schema 拡張を検討
- **phase-auditor finding (CODE-REVIEW-01)**: merged `findings.json` が個別 finding の `observation` token を ai-antipattern / codex source で `null` のままにする (per-reviewer file には top-level `observation: <name>` あり)。7-perspective coverage を `findings.json` 単独で機械検証不能。merge step で source reviewer の perspective を carry-forward する正規化が必要

## Test count / scenario count delta

| metric | F2a merge (baseline) | F2b completion | delta |
|---|---|---|---|
| workspace tests | 408 | 411 | +3 (Item 2 -5 + Item 3 +5 + Item 6 +3, Item 1 ±0, Item 4/5/7 ±0) |
| belt-core scenarios | 108 | 125 | +17 (uri +7, gate +5, error +2, expander +3) |
| new files | — | 3 | common/{mod,helpers,narrative}.rs |
| deleted files | — | 0 | (tombstone保持、inline削除は削除ではなく移植) |

## Cross-reference

- Template: `docs/testing/audit-template.md` v1 (2026-04-18 Duplication Candidates wording correction)
- Scenarios: `docs/testing/cli-behavior/{belt,belt-core}.yml`
- Lock ledger: `docs/testing/lock-ledger.md` (2026-04-18 bug_fix_refresh entry expanded)
- Design: `docs/features/2026-04-17-belt-test-foundation-f2b/design.md`
- Plan: `docs/features/2026-04-17-belt-test-foundation-f2b/plan.md`
- Test Strategy: `docs/features/2026-04-17-belt-test-foundation-f2b/test-strategy.md`
- F2a audit report: `docs/features/2026-04-17-belt-test-foundation-f2a/audit-report.md`
- F1 audit report (pilot 22 tests): `docs/features/2026-04-17-belt-test-foundation/audit-report.md`
