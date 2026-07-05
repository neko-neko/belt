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
- `feature_dev_expands_to_seven_namespaced_leaves`
- `no_leaf_declares_regate`
- `e2e_leaf_carries_when_and_others_do_not`
- `feature_dev_pipeline_has_no_run_id_template`

**pipeline.yml shape dimensions locked** (B):

- `args` set が exactly `{codex, e2e}` + 全 arg が `ArgType::Bool` / default false
- 3 top-level phase の順序 (`design → pre-execute-handover → build`)、全て `Invoker::Pipeline`
- design/build の `with` が exactly `{e2e: "args.e2e", codex: "args.codex"}` (bare full-string form)、checkpoint は `with` 空
- 展開 leaf ids が exactly 7 件 (`design/intake → design/design → pre-execute-handover/checkpoint → build/execute → build/code-review → build/e2e → build/integrate`)
- regate は全 leaf で空 (2026-07-05 sonnet-lean で regate 全廃)
- `build/e2e` のみ `when: args.e2e` を保持、`build/execute` / `build/code-review` / `build/integrate` は when なし
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
- `bug_fix_expands_to_eight_namespaced_leaves`
- `no_leaf_declares_regate`
- `e2e_leaf_carries_when_and_others_do_not`
- `bug_fix_pipeline_has_no_run_id_template`

**pipeline.yml shape dimensions locked** (B):

- `args` set が exactly `{codex, e2e}` + 全 arg が `ArgType::Bool` / default false、legacy args (`iterations` / `swarm` / `ui` / `smoke`) の non-existence
- 3 top-level phase の順序 (`diagnose → pre-execute-handover → build`)、全て `Invoker::Pipeline`
- diagnose/build の `with` が exactly `{e2e: "args.e2e", codex: "args.codex"}`、checkpoint は `with` 空
- 展開 leaf ids が exactly 8 件 (`diagnose/rca → diagnose/fix-plan → diagnose/fix-plan-review → pre-execute-handover/checkpoint → build/execute → build/code-review → build/e2e → build/integrate`)
- regate は全 leaf で空 (2026-07-05 sonnet-lean で regate 全廃)
- `build/e2e` のみ `when: args.e2e` を保持、`build/execute` / `build/code-review` / `build/integrate` は when なし
- `.belt/runs/` リテラル + `{run_id}` template の non-existence

**Cross-coupling** (C):

- `pipeline_split_refresh.rs` — stage 内部 shape (diagnose の docs/features path 含む) は stage 側で lock
- `feature_dev_refresh.rs` — feature-dev 合成 shape (同 tuple pattern で parallel)

---

## review_skills_refresh.rs

```yaml
locks-file: crates/belt-core/tests/review_skills_refresh.rs
consolidated-agents:
  - spec-reviewer.md
  - code-reviewer.md
  - quality-reviewer.md
per-observation-agents-deleted:
  - security-reviewer.md
  - test-reviewer.md
  - ai-antipattern-reviewer.md
  - cross-cutting-reviewer.md
  - feasibility-reviewer.md
  - ui-design-reviewer.md
  - cross-cutting-spec-reviewer.md
test-fn-count: 6
```

**6 test fn 名** (A):

- `review_skills_pipeline_yml_is_deleted`
- `review_skills_belt_toml_is_deleted`
- `consolidated_reviewer_agents_exist`
- `per_observation_agent_files_are_deleted`
- `review_skills_parent_skill_md_references_parallel_dispatch`
- `consolidated_agents_use_output_path_arg_pattern`

**locked shape dimensions** (B):

- 2026-07-05 sonnet-lean 統合 (7→3) 後の shape: consolidated agents (`spec-reviewer.md`, `code-reviewer.md`, `quality-reviewer.md`) が存在し、旧 per-observation agent 7 file・review skills の pipeline.yml / belt.toml が non-existence、SKILL.md が Task dispatch + `findings-` を記述
- consolidated agents は Output Format section で `output_path` を参照し、`.belt/runs/` リテラルを hardcode しない

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
