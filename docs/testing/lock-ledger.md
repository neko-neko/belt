# Lock Ledger — plugin shape + cross-crate 契約 lock tests

belt-core/tests/ 配下の shape lock tests の台帳。各 entry は `locks-file:` frontmatter で実 file を参照。scenarios_contract.rs が台帳の `locks-file:` フィールドと実ファイル存在を機械照合。

---

## feature_dev_refresh.rs

```yaml
locks-file: crates/belt-core/tests/feature_dev_refresh.rs
pipeline: plugins/belt/skills/feature-dev/pipeline.yml
test-fn-count: 8
cross-coupling:
  - crates/belt-core/tests/bug_fix_refresh.rs
  - crates/belt-core/tests/pipeline_split_refresh.rs
  - crates/belt-core/tests/review_skills_refresh.rs
  - crates/belt-core/tests/shared_filter_parity.rs
```

**8 test fn 名** (A):

- `feature_dev_composes_three_stages`
- `stages_delegate_with_e2e_and_codex_passthrough`
- `checkpoint_delegates_with_no_args`
- `top_level_args_are_e2e_and_codex_only`
- `feature_dev_expands_to_ten_namespaced_leaves`
- `expanded_regate_targets_are_namespaced`
- `expanded_verify_leaves_inherit_e2e_when`
- `feature_dev_pipeline_has_no_run_id_template`

**pipeline.yml shape dimensions locked** (B):

- `args` set が exactly `{codex, e2e}` + 全 arg が `ArgType::Bool` / default false
- 3 top-level phase の順序 (`design → pre-execute-handover → build`)、全て `Invoker::Pipeline`
- design/build の `with` が exactly `{e2e: "args.e2e", codex: "args.codex"}` (bare full-string form)、checkpoint は `with` 空
- 展開 leaf ids が exactly 10 件 (`design/design → ... → build/integrate`、File Structure 節の展開形)
- stage 内 regate の namespace 展開 (`design/spec-review → [design/test-scenarios]`、`build/code-review → [build/execute]`)
- verify leaves (`build/verify/monkey-test` / `build/verify/dogfood`) が `when: args.e2e` を継承、`build/execute` は when なし
- `.belt/runs/` リテラル + `{run_id}` template の non-existence

**Cross-coupling** (C):

- `pipeline_split_refresh.rs` — stage 内部 shape (phase 順序 / narrative / criteria) は stage 側で lock
- `bug_fix_refresh.rs` — bug-fix 合成 shape (同 tuple pattern で parallel)
- `review_skills_refresh.rs` / `shared_filter_parity.rs` — 従来どおり

---

## bug_fix_refresh.rs

```yaml
locks-file: crates/belt-core/tests/bug_fix_refresh.rs
pipeline: plugins/belt/skills/bug-fix/pipeline.yml
test-fn-count: 9
cross-coupling:
  - crates/belt-core/tests/feature_dev_refresh.rs
  - crates/belt-core/tests/pipeline_split_refresh.rs
```

**9 test fn 名** (A):

- `bug_fix_composes_three_stages`
- `stages_delegate_with_e2e_and_codex_passthrough`
- `checkpoint_delegates_with_no_args`
- `args_are_e2e_and_codex_only`
- `no_legacy_args`
- `bug_fix_expands_to_nine_namespaced_leaves`
- `expanded_regate_targets_are_namespaced`
- `expanded_verify_leaves_inherit_e2e_when`
- `bug_fix_pipeline_has_no_run_id_template`

**pipeline.yml shape dimensions locked** (B):

- `args` set が exactly `{codex, e2e}` + 全 arg が `ArgType::Bool` / default false、legacy args (`iterations` / `swarm` / `ui` / `smoke`) の non-existence
- 3 top-level phase の順序 (`diagnose → pre-execute-handover → build`)、全て `Invoker::Pipeline`
- diagnose/build の `with` が exactly `{e2e: "args.e2e", codex: "args.codex"}`、checkpoint は `with` 空
- 展開 leaf ids が exactly 9 件 (`diagnose/rca → ... → build/integrate`)
- regate は `build/code-review → [build/execute]` のみ、他 leaf は空
- verify leaves が `when: args.e2e` を継承
- `.belt/runs/` リテラル + `{run_id}` template の non-existence

**Cross-coupling** (C):

- `pipeline_split_refresh.rs` — stage 内部 shape (diagnose の docs/features path 含む) は stage 側で lock
- `feature_dev_refresh.rs` — feature-dev 合成 shape (同 tuple pattern で parallel)

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
test-fn-count: 7
```

**7 test fn 名** (A):

- `review_skills_pipeline_yml_is_deleted`
- `review_skills_belt_toml_is_deleted`
- `review_skills_legacy_consolidated_agent_is_deleted`
- `review_skills_new_observation_agents_exist`
- `review_skills_parent_skill_md_references_parallel_dispatch`
- `legacy_per_observation_review_agent_files_are_removed`
- `per_observation_agents_use_output_path_arg_pattern`

**locked shape dimensions** (B):

- per-observation agents (`security-reviewer.md`, `test-reviewer.md`, `ai-antipattern-reviewer.md`, `cross-cutting-reviewer.md`, `cross-cutting-spec-reviewer.md`, `feasibility-reviewer.md`, `ui-design-reviewer.md`) reference `output_path` in their Output Format section and do not hardcode `.belt/runs/` literals

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
test-fn-count: 14 (1 positive + 1 ledger_locks_files_exist + 1 audit_template_version + 7 drift injection + 4 raw-string drift: multiline raw string, hashed raw string, outside-string preservation, `/*` inside string literal)
```
