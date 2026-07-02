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
test-fn-count: 25
cross-coupling:
  - crates/belt-core/tests/feature_dev_refresh.rs
```

**25 test fn 名** (A):

- `args_are_e2e_and_codex_only`
- `no_legacy_args`
- `phase_count_and_order`
- `all_phases_use_skill_or_pipeline_invoke`
- `review_phases_pass_codex_only`
- `only_code_review_has_regate`
- `rca_scenarios_when_is_typed`
- `rca_scenarios_filtered_when_e2e_false`
- `rca_scenarios_present_when_e2e_true`
- `all_phases_have_max_retries_3_and_confirm_true`
- `supplement_files_exist`
- `dead_letter_references_removed`
- `criteria_files_exist`
- `skill_md_has_expected_sections`
- `skill_md_declares_supplement_injection_per_phase`
- `bug_fix_narrative_phases_produce_notes`
- `bug_fix_narrative_phases_gate_notes`
- `bug_fix_narrative_accumulating_consumes`
- `bug_fix_non_narrative_phases_have_no_notes`
- `pre_execute_handover_delegates_to_sub_pipeline`
- `pre_execute_handover_expands_to_namespaced_checkpoint`
- `bug_fix_produces_use_belt_current_uri`
- `bug_fix_pipeline_has_no_run_id_template`
- `bug_fix_code_review_produces_seven_artifacts`
- `bug_fix_fix_plan_review_produces_spec_findings`

**pipeline.yml shape dimensions locked** (B):

- `args` set が exactly `{codex, e2e}` + 全 arg が `ArgType::Bool` (`bug_fix_refresh.rs:63-75`)
- legacy `args` (`iterations` / `swarm` / `ui` / `smoke`) の non-existence (`bug_fix_refresh.rs:78-86`)
- 9 phase の順序 (`rca → fix-plan → fix-plan-review → pre-execute-handover → execute → code-review → monkey-test → dogfood → integrate`) (`bug_fix_refresh.rs:89-93`)
- skill-invoke phase は `Invoker::Skill` variant で leading slash 付き、sub-pipeline delegation phase は `Invoker::Pipeline` variant で `pipeline` path が非空 (`bug_fix_refresh.rs:96-128`)
- review phases (`fix-plan-review` / `code-review`) の invoke args が exactly `{codex: "args.codex"}` (`bug_fix_refresh.rs:120-144`)
- `code-review.regate = [execute]`、他 phase は `regate` 空 (`bug_fix_refresh.rs:147-165`)
- `rca_scenarios.when = "args.e2e"` typed enum field (string ではなく ArtifactWhen) (`bug_fix_refresh.rs:168-185`)
- `view::active_produces` の条件付き filtering (`args.e2e=false` で `rca_scenarios` 除外、`rca_report` は常時) (`bug_fix_refresh.rs:188-213`)
- skill-invoke phase は `max_retries == 3` + `confirm == true`、sub-pipeline delegation parent (`Invoker::Pipeline`) は top-level assertion を skip (confirm / max_retries は sub-phase 側で管理) (`bug_fix_refresh.rs:226-248`)
- 5 supplement file (`rca-supplement.md`, `fix-plan-supplement.md`, `monkey-test-supplement.md`, `dogfood-supplement.md`, `worktrunk-supplement.md` + `path-convention.md`) の physical existence (`bug_fix_refresh.rs:229-244`)
- dead-letter reference (`evidence-plan-protocol.md`, `fix-dispatch-strategy.md`) の physical non-existence (`bug_fix_refresh.rs:247-255`)
- 6 skill-local criteria + 2 duplicated shared criteria (`execute.md`, `code-review.md`) の physical existence (`bug_fix_refresh.rs:258-285`)
- SKILL.md の expected section substring lock (`## Supplement Loading` / `## Phase-specific Runtime Notes` / `## Red Flags` / `## References` / `argument-hint:`) (`bug_fix_refresh.rs:288-303`)
- SKILL.md が 5 phase supplement 名を Supplement Loading table に列挙 (`bug_fix_refresh.rs:306-323`)
- `consumes` の accumulating narrative pattern (phase N は phase 1..N-1 の notes を `Named(n)` で consume) (`bug_fix_refresh.rs:369-402`)
- non-narrative phase (`fix-plan-review` / `integrate`) が notes artifact を持たない (`bug_fix_refresh.rs:405-408`)
- `pre-execute-handover` が `../handover/checkpoint.yml` sub-pipeline に delegate (展開後 phase id: `pre-execute-handover/checkpoint`)
- narrative artifact 6 phase (`rca` / `fix-plan` / `execute` / `code-review` / `monkey-test` / `dogfood`) の path が exactly `belt://current/notes/phase-<id>.md` URI 形式
- code-review.produces が 7 entries (findings-security / findings-test / findings-ai-antipattern / findings-cross-cutting / findings-codex with `when: args.codex` / findings (merged) / code_review_notes)
- fix-plan-review.produces が 5 entries (findings-feasibility / findings-cross-cutting-spec / findings-ui-design / findings-codex with `when: args.codex` / findings (merged))
- 全 phase の `produces[].path` および `gate.file_exists` が `belt://...` URI または `docs/`/`src/` raw path のみ (`.belt/runs/` リテラル + `{run_id}` template の non-existence)

**Cross-coupling** (C):

- `feature_dev_refresh.rs` — feature-dev pipeline shape (同 tuple pattern で parallel)

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
