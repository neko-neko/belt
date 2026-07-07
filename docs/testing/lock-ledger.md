# Lock Ledger — plugin shape + cross-crate 契約 lock tests

belt-core/tests/ 配下の shape lock tests の台帳。各 entry は `locks-file:` frontmatter で実 file を参照。scenarios_contract.rs が台帳の `locks-file:` フィールドと実ファイル存在を機械照合。

---

## feature_dev_refresh.rs

```yaml
locks-file: crates/belt-core/tests/feature_dev_refresh.rs
pipeline: plugins/belt/skills/feature-dev/pipeline.yml
test-fn-count: 9
cross-coupling:
  - crates/belt-core/tests/bug_fix_refresh.rs
  - crates/belt-core/tests/review_skills_refresh.rs
  - crates/belt-core/tests/shared_filter_parity.rs
```

**9 test fn 名** (A):

- `feature_dev_composes_six_stages`
- `stages_delegate_with_codex_passthrough`
- `checkpoint_and_qa_delegate_with_no_args`
- `top_level_args_are_codex_only`
- `feature_dev_expands_to_eight_namespaced_leaves`
- `no_leaf_declares_regate_or_when`
- `confirm_leaves_match_the_four_touchpoints`
- `integrate_leaf_identical_across_orchestrators`
- `feature_dev_pipeline_has_no_run_id_template`

**pipeline.yml shape dimensions locked** (B):

- `args` set が exactly `{codex}` + `ArgType::Bool` / default false (2026-07-07 four-stage rewrite で e2e 廃止 — QA mandatory, D2)
- 6 top-level phase の順序 (`design → plan → pre-execute-handover → build → qa → integrate`)
- design/plan/build は `Invoker::Pipeline` + `with` exactly `{codex: "args.codex"}` (bare full-string form)、pre-execute-handover / qa は `with` 空
- integrate は inline leaf (`Invoker::Skill /worktrunk`)
- 展開 leaf ids が exactly 8 件 (`design/intake → design/design → plan/plan → pre-execute-handover/checkpoint → build/execute → build/code-review → qa/qa → integrate`)
- regate は全 leaf で空、phase-level `when` も全 leaf で non-existence (e2e opt-in 全廃)
- confirm leaves は exactly 4 件 (`design/design` / `plan/plan` / `pre-execute-handover/checkpoint` / `integrate`) (D4)
- integrate leaf は bug-fix と serde_json::Value 同値 (D14 inline duplication + identity lock)
- `.belt/runs/` リテラル + `{run_id}` template の non-existence

**Cross-coupling** (C):

- `bug_fix_refresh.rs` — bug-fix 合成 shape (同 tuple pattern で parallel)、integrate leaf 同値 lock の相手側
- `review_skills_refresh.rs` / `shared_filter_parity.rs` — 従来どおり

---

## bug_fix_refresh.rs

```yaml
locks-file: crates/belt-core/tests/bug_fix_refresh.rs
pipeline: plugins/belt/skills/bug-fix/pipeline.yml
test-fn-count: 9
cross-coupling:
  - crates/belt-core/tests/feature_dev_refresh.rs
```

**9 test fn 名** (A):

- `bug_fix_composes_five_stages`
- `stages_delegate_with_codex_passthrough`
- `checkpoint_and_qa_delegate_with_no_args`
- `args_are_codex_only`
- `no_legacy_args`
- `bug_fix_expands_to_eight_namespaced_leaves`
- `no_leaf_declares_regate_or_when`
- `confirm_leaves_match_the_three_touchpoints`
- `bug_fix_pipeline_has_no_run_id_template`

**pipeline.yml shape dimensions locked** (B):

- `args` set が exactly `{codex}` + `ArgType::Bool` / default false、legacy args (`iterations` / `swarm` / `ui` / `smoke` / `e2e`) の non-existence (2026-07-07 four-stage rewrite で e2e も legacy 化)
- 5 top-level phase の順序 (`diagnose → pre-execute-handover → build → qa → integrate`)
- diagnose/build は `Invoker::Pipeline` + `with` exactly `{codex: "args.codex"}` (bare full-string form)、pre-execute-handover / qa は `with` 空
- integrate は inline leaf (`Invoker::Skill /worktrunk`)
- 展開 leaf ids が exactly 8 件 (`diagnose/rca → diagnose/fix-plan → diagnose/fix-plan-review → pre-execute-handover/checkpoint → build/execute → build/code-review → qa/qa → integrate`)
- regate は全 leaf で空、phase-level `when` も全 leaf で non-existence (e2e opt-in 全廃)
- confirm leaves は exactly 3 件 (`diagnose/fix-plan-review` / `pre-execute-handover/checkpoint` / `integrate`) (D4: 診断承認は 1 点)
- integrate leaf の feature-dev との同値は `feature_dev_refresh.rs::integrate_leaf_identical_across_orchestrators` が lock
- `.belt/runs/` リテラル + `{run_id}` template の non-existence

**Cross-coupling** (C):

- `feature_dev_refresh.rs` — feature-dev 合成 shape (同 tuple pattern で parallel)、integrate leaf 同値 lock を保持

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
pipeline-agent-bundle:
  - belt-agent: explorer.md / implementer.md
  - belt: qa-verifier.md (+ consolidated reviewers)
retired-belt-agent-agents-deleted:
  - code-explorer.md
  - code-architect.md
  - impact-analyzer.md
  - feature-implementer.md
  - phase-auditor.md
test-fn-count: 10
```

**10 test fn 名** (A):

- `review_skills_pipeline_yml_is_deleted`
- `review_skills_belt_toml_is_deleted`
- `consolidated_reviewer_agents_exist`
- `per_observation_agent_files_are_deleted`
- `review_skills_parent_skill_md_references_parallel_dispatch`
- `consolidated_agents_use_output_path_arg_pattern`
- `pipeline_agent_bundle_exists`
- `retired_belt_agent_agents_are_deleted`
- `verify_skill_is_replaced_by_qa`
- `qa_verifier_uses_evidence_dir_arg_pattern`

**locked shape dimensions** (B):

- 2026-07-05 sonnet-lean 統合 (7→3) 後の shape: consolidated agents (`spec-reviewer.md`, `code-reviewer.md`, `quality-reviewer.md`) が存在し、旧 per-observation agent 7 file・review skills の pipeline.yml / belt.toml が non-existence、SKILL.md が Task dispatch + `findings-` を記述
- consolidated agents は `output_path` を参照し（lock は file 全体への `contains("output_path")` assertion）、`.belt/runs/` リテラルを hardcode しない
- 2026-07-07 four-stage rewrite の agent bundle lock: belt = {code-reviewer, quality-reviewer, spec-reviewer, qa-verifier}、belt-agent = {explorer, implementer} が存在し、旧 belt-agent agent 5 file (`code-explorer.md` / `code-architect.md` / `impact-analyzer.md` / `feature-implementer.md` / `phase-auditor.md`) と `audit-protocol.md` が non-existence
- `plugins/belt/skills/verify/` は non-existence、代替の `plugins/belt/skills/qa/SKILL.md` が存在。`qa-verifier.md` は `evidence_dir` を参照し `.belt/runs/` リテラルを hardcode しない

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
