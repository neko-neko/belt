---
audited_at: 2026-04-17T06:28:55Z
audited_commit: 2386abbaafbcccf650e0ae6822ec71051af6ff58
audit_template_version: v1
---

# belt-test-foundation F1 Pilot Audit Report

F1 pilot audit 結果。methodology (`docs/testing/audit-template.md` v1) を 3 pilot file に適用した judgement。

## crates/belt/tests/cli_test.rs (5 tests, all kept)

### lint_valid_pipeline_exits_zero
- judgement: kept
- scenario: belt-lint-valid-pipeline-ok
- rationale: happy path lint、scenario covers exit 0 + stderr "ok"

### lint_invalid_pipeline_exits_one
- judgement: kept
- scenario: belt-lint-duplicate-phase-id-detected
- rationale: duplicate phase ID 検知は lint の主要機能

### lint_nonexistent_file_exits_one
- judgement: kept
- scenario: belt-lint-nonexistent-file-rejected
- rationale: file I/O error の CLI level lock

### lint_with_config_resolves_pipeline
- judgement: kept
- scenario: belt-lint-config-resolves-pipeline-file
- rationale: --config flag 経由での pipeline path 解決 happy path

### lint_config_and_positional_file_errors
- judgement: kept
- scenario: belt-lint-config-and-positional-mutually-exclusive
- rationale: argument mutual exclusion は breaking change 検知の sentinel

## crates/belt-core/tests/config_test.rs (6 tests, all kept)

### parse_valid_config
- judgement: kept
- scenario: belt-core-config-valid-toml-parses
- rationale: happy path toml parse

### parse_config_missing_file
- judgement: kept
- scenario: belt-core-config-missing-file-yields-file-not-found
- rationale: BeltError::FileNotFound variant lock

### parse_config_invalid_toml
- judgement: kept
- scenario: belt-core-config-invalid-toml-yields-config-parse
- rationale: BeltError::ConfigParse variant lock (malformed TOML syntax)

### parse_config_missing_pipeline_field
- judgement: kept
- scenario: belt-core-config-missing-pipeline-file-field-yields-config-parse
- rationale: BeltError::ConfigParse variant lock (missing required field)

### resolve_pipeline_path_relative_to_config
- judgement: kept
- scenario: belt-core-config-resolves-relative-pipeline-path
- rationale: relative pipeline_file path resolution to config_dir

### resolve_pipeline_path_with_subdirectory
- judgement: kept
- scenario: belt-core-config-resolves-subdirectory-pipeline-path
- rationale: subdirectory-qualified relative path resolution

## crates/belt-core/tests/feature_dev_refresh.rs (11 tests, all kept)

Lock pilot。本 file は shape lock test の代表として `docs/testing/lock-ledger.md` に entry 移送済。

- judgement (全 11 tests): kept
- rationale: lock test は behavior scenario ではなく shape を固定する特殊役割で、audit-template.md v1 で "judgement = kept without scenario ID" を許容 (implementation-coupling label でも obsolete-spec label でもない)

本 F1 では `feature_dev_refresh.rs` 自体に変更を加えない (MV-17)。lock-ledger.md entry で 11 test fn 名 + pipeline.yml shape dimensions + cross-coupling が記録される (MV-18)。

## Summary

- Total pilot audited: 22 test (5 + 6 + 11)
- kept: 22
- deleted: 0
- merged: 0
- abstracted: 0

F1 scope では全 kept。F2/F3 で本格 audit (他 belt-core / belt-agent test 全体) を実施する際は `docs/testing/audit-template.md` の Decision Tree + v1 9 reason label 集を使用する。pilot file が F1 → F2 間に touch されていれば re-audit (audit-template.md の "Pilot Audit の再実施 trigger" section 参照)。

## Cross-reference

- Template: `docs/testing/audit-template.md` (v1)
- Scenarios: `docs/testing/cli-behavior/{belt,belt-core}.yml`
- Lock ledger: `docs/testing/lock-ledger.md`
- Design: `docs/features/2026-04-17-belt-test-foundation/design.md`
- Test strategy: `docs/features/2026-04-17-belt-test-foundation/test-strategy.md`
