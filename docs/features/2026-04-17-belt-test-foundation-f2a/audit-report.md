---
audited_at: 2026-04-17T10:20:34Z
audited_commit: 2698dd161cc243e94ad005e936bf21a44a931691
audit_template_version: v1
---

# belt-test-foundation F2a Audit Report

F2a audit 結果。F1 pilot で確立した methodology (`docs/testing/audit-template.md` v1) を belt-core 全 behavior module + uri 新規 + 2 復帰 scenario に適用、M2 normalize + X2 doc-comment policy 下で judgment を記録。

## Methodology

- **Decision Tree Q1** で "scenario 登録済" 判定: F2a で scenarios.yml に追加した scenario に doc-comment 付与する test fn は kept
- **Q2-Q5 は F2b で適用**: F2a では "scenario 化できなかった test" は Forward-to-F2b list に送り、F2b が 8 label 適用する
- **Label 使用**: `kept` のみ (F2a)

## Judgment summary

### Phase A モジュール別 (9 module)

| module | test count | scenarios | annotated (kept) | forward-to-F2b | forward reasons |
|---|---|---|---|---|---|
| engine       | 67 | 19 | 65 | 2 | brittle-format-match × 2 (Display assertions) |
| view         | 41 | 15 | 41 | 0 | — |
| lint         | 29 | 15 | 29 | 0 | — |
| model        | 39 | 22 | 39 | 0 | (band 15-20 exceeded to 22: natural seams) |
| gate         | 22 |  9 | 22 | 0 | (git_clean: 0 tests = coverage gap, F2b track) |
| error        |  6 |  3 |  6 | 0 | — |
| expander     |  5 |  4 |  5 | 0 | — |
| parser       |  4 |  4 |  4 | 0 | — |
| artifact_when|  5 |  5 |  5 | 0 | — |
| **subtotal** | **218** | **96** | **216** | **2** | |

### Phase B3 — uri_test.rs (新規 integration file)

- 新規 test 追加: 5 (tests/uri_test.rs)
- scenario 追加: 5 (uri category)
- 全 kept
- **duplication 判断は F2b 送り**: src/uri.rs #[cfg(test)] mod tests の 13 tests (inline unit tests) は F2a の walk scope 外。F2b で inline vs integration duplication audit を実施

### Phase B4-B5 復帰 (2 test + 2 scenario)

- `belt-lint-invalid-yaml-rejected` (crates/belt/tests/cli_test.rs): kept (1 test + 1 scenario, assert は exit=1 + stderr loose match、miette/serde-saphyr format drift 耐性)
- `belt-core-config-preserves-absolute-pipeline-path` (crates/belt-core/tests/config_test.rs): kept (1 test + 1 scenario、production code touch 不要、Rust Path::join(absolute) semantics により現状 implementation で pass)

### Phase B2 — strip_string_literals raw-string fix

- 3 drift tests 追加 (scenarios_contract.rs): `drift_multiline_raw_string_is_stripped`, `drift_raw_string_with_hash_is_stripped`, `drift_doc_comment_outside_string_still_matches_after_fix`
- これらは binding infrastructure の regression guard であり、scenarios_yml との 1:1 対応 test ではない (scenarios_contract.rs 自身の self-test)

### Phase B2b — strip order fix (view apply 副次発見)

- view Phase A 実施時に scenarios_contract.rs の latent bug 発見: `strip_block_comments` が `strip_string_literals` より先に呼ばれる順序で、文字列リテラル内の `/*` が phantom block comment を EOF まで開いて `/// scenario:` を erase していた
- 修正: 11 call sites で `strip_block_comments(&strip_string_literals(src))` に swap + drift test `drift_string_with_slash_star_does_not_swallow_later_doc_comment` 追加
- view commit (d4d759e) scope に fold-in

### Phase B6 — audit-template.md clarification

- "Pilot Audit の再実施 trigger" section に "Trigger 対象外" subsection 追記: 新規 test fn 追加 / doc-comment 付与 / preamble 更新は "touch" 非該当
- audit_template_version: v1 unchanged (clarification、SemVer bump 不要)

## Forward-to-F2b list

F2b が Decision Tree Q2-Q5 + 8 reason label を適用する対象:

### engine module — Display format tests (brittle-format-match)

- `error_verify_required_message` — reason: brittle-format-match. Asserts Display string contains `"verify required for phase"` literal phrase. Format is presentation contract, not engine state transition. F2b decide: keep (format-lock scenario) or delete (rely on Display impl stability convention)
- `error_max_retries_exceeded_message` — reason: brittle-format-match. Asserts Display contains `"max retries exceeded"` and `"3/3"` format. Same rationale

### uri module — inline vs integration duplication

`crates/belt-core/src/uri.rs` #[cfg(test)] mod tests (13 tests) vs `crates/belt-core/tests/uri_test.rs` (F2a, 5 tests):

Overlapping behavior:
- parse_latest_happy_path (inline) ↔ latest_selector_parses_with_pipeline_and_path (integration)
- parse_workspace_latest_happy_path (inline) ↔ workspace_latest_selector_parses_with_branch_pipeline_path (integration)
- parse_run_happy_path (inline) ↔ run_selector_parses_with_run_id_and_path (integration)
- parse_missing_scheme (inline) ↔ non_belt_scheme_is_rejected (integration)
- parse_path_traversal_rejected (inline) ↔ path_traversal_segment_is_rejected (integration)

Unique to inline (not yet scenario-mapped): parse_unknown_selector / parse_empty_pipeline / parse_empty_run_id / parse_empty_path / parse_absolute_path_rejected / parse_workspace_missing_latest / to_string_roundtrip_all_variants

F2b decision options:
- (a) delete inline tests once integration covers all (add scenarios for 7 remaining edge cases)
- (b) keep inline as white-box companion, label as implementation-coupling exemption

### gate module — git_clean coverage gap

`gate_test.rs` has 0 tests for `GateCheck::git_clean` kind. 4 kinds are total (cmd, file_exists, git_clean, has_output) but only 3 are exercised. Not a drift issue (no test to judge); F2b track: add new tests to close the gap.

## Summary

- Total F2a-audited behavior tests: 223 (218 Phase A + 5 Phase B3 uri new)
- kept: 221 (all annotated + 2 復帰 in pilot files + 0 deleted in F2a)
- forward-to-F2b: 2 (engine Display tests)
- deleted/merged/abstracted in F2a: 0 (Phase A is additive-only)

Cumulative scenarios (F1 + F2a): 114 (11 F1 + 103 F2a)
Cumulative workspace tests: 408 (397 F1 baseline + 11 F2a: 4 scenarios_contract drift tests + 5 uri integration + 2 restoration)

## Cross-reference

- Template: `docs/testing/audit-template.md` (v1 with 2026-04-17 "Trigger 対象外" clarification)
- Scenarios: `docs/testing/cli-behavior/{belt,belt-core}.yml`
- Lock ledger: `docs/testing/lock-ledger.md` (unchanged in F2a)
- Design: `docs/features/2026-04-17-belt-test-foundation-f2a/design.md`
- Plan: `docs/features/2026-04-17-belt-test-foundation-f2a/plan.md`
- F1 audit report (pilot 22 tests): `docs/features/2026-04-17-belt-test-foundation/audit-report.md`
