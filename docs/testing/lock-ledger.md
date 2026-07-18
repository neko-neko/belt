# Lock Ledger — plugin shape + cross-crate 契約 lock tests

(2026-07-18: feature_dev_refresh.rs / bug_fix_refresh.rs / review_skills_refresh.rs は pipeline/plugin 構造の二重管理負債として削除。pipeline.yml の静的検証は `belt lint` が担う。)

belt-core/tests/ 配下の shape lock tests の台帳。各 entry は `locks-file:` frontmatter で実 file を参照。scenarios_contract.rs が台帳の `locks-file:` フィールドと実ファイル存在を機械照合。

---

## shared_filter_parity.rs

```yaml
locks-file: crates/belt-core/tests/shared_filter_parity.rs
parity-groups:
  - code-review-agents: code-reviewer / quality-reviewer の ## Filtering 先頭 3 bullet byte-identical (spec-reviewer は diff-scope agent でないため対象外)
test-fn-count: 1
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
test-fn-count: 14 (1 positive + 1 ledger_locks_files_exist + 1 audit_template_version + 7 drift injection + 4 raw-string drift: multiline raw string, hashed raw string, outside-string preservation, `/*` inside string literal)
```
