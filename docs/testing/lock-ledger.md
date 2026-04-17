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

- `feature_dev_has_nine_phases`
- `feature_dev_expands_cleanly`
- `monkey_test_and_dogfood_are_conditional_on_e2e`
- `scenarios_produce_is_conditional_on_e2e`
- `code_review_regates_execute_only`
- `top_level_args_are_e2e_and_codex_only`
- `feature_dev_scenarios_artifact_has_typed_when_field`
- `feature_dev_narrative_phases_produce_notes`
- `feature_dev_narrative_phases_gate_notes`
- `feature_dev_narrative_accumulating_consumes`
- `feature_dev_non_narrative_phases_have_no_notes`

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
test-fn-count: 10 (1 positive + 1 ledger_locks_files_exist + 1 audit_template_version + 7 drift injection)
```
