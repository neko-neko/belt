# pre-execute-handover Sub-pipeline — Design

**Status**: Draft
**Date**: 2026-04-18
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)

## Summary

feature-dev / bug-fix pipeline 両方で完全同一 (byte-identical 8 行) となっている `pre-execute-handover` phase を、`plugins/belt/skills/handover/checkpoint.yml` に抽出した **sub-pipeline** へ移し、parent phase から `invoke: { pipeline: ... }` で delegate する形に変更する。あわせて既存の 2 つの lock contract (「skill-invoke-only」「Pure-checkpoint-has-no-invoke」) を sub-pipeline delegation を許可する方向に緩和し、`docs/testing/lock-ledger.md` の pre-execute-handover 関連 drift を同時修正する。

## Goals

- `pre-execute-handover` phase の定義を **1 箇所 (sub-pipeline) に集約** し、feature-dev / bug-fix 間の byte-identical 重複を排除
- sub-pipeline の owner を `/belt:handover` スキル (gate の `handover.md` producer) に一致させる
- lock test の対象範囲を変えずに既存契約のみ narrow に緩和 (`Invoker::Cmd` 禁止等は維持)
- `docs/testing/lock-ledger.md` の phase list drift を修正し、sub-pipeline delegation を反映

## Non-Goals

- execute / code-review / monkey-test / dogfood / integrate 等の他 phase の共通化 (belt-core expander の substitute が `consumes` / `produces.path` を rewrite できないため、DRY メリットが現時点では薄い — pain-driven first-class principle に従い今回は対象外)
- `belt-core` の `expand_pipeline` / `substitute_*` / model 拡張
- `plugins/belt/pipelines/` 等の新規共通ディレクトリ導入 (sub-pipeline は handover スキル配下に凝集させる)
- `/belt:handover` / `/belt:resume` SKILL.md の変更 (sub-pipeline 導入は checkpoint の owner 関係を文書化するだけで workflow は不変)
- `plugin.json` / `marketplace.json` の update (skill 列挙 / phase count 表記に影響なし)
- `docs/testing/cli-behavior/belt-core.yml` への新規 scenario 追加 (既存 `expander_test.rs` / `expander_with_test.rs` が sub-pipeline 展開を十分カバー)
- debug-flow pipeline の新規追加 (handover SKILL.md に言及があるが現行未実装、本 design とは独立)
- feature-dev / bug-fix の phase 追加・削除・並べ替え

## Background

### 重複の実態

`plugins/belt/skills/feature-dev/pipeline.yml` と `plugins/belt/skills/bug-fix/pipeline.yml` の `pre-execute-handover` phase は **8 行が完全に同一** で、`id` / `description` (multiline) / `confirm: true` / `gate: file_exists: ".belt/runs/{run_id}/handover.md"` 以外の field を一切持たない Pure-checkpoint:

```yaml
  - id: pre-execute-handover
    description: >-
      Context checkpoint before execute. Run `/belt:handover`, then `/clear`,
      then `/belt:resume` in a new session. The gate passes once the handover
      note exists under `.belt/runs/{run_id}/`.
    confirm: true
    gate:
      - file_exists: ".belt/runs/{run_id}/handover.md"
```

この重複は 2026-04-17 の `/belt:handover` / `/belt:resume` skill 導入 (merge `efc0a4d`) に伴って両 pipeline に同時追加されたもので、設計的には 1 つの shared checkpoint を 2 箇所に物理的に複製している状態。

### 他 phase との比較

他の構造類似 phase (execute / code-review / monkey-test / dogfood / integrate / spec-review-vs-fix-plan-review) は `consumes` の artifact 名セットや `produces.path` (docs/features/* vs docs/plans/*) が pipeline-specific に異なり、belt-core 現状の substitute (`expander.rs::substitute_in_value_map` は string value のみ `"args.<name>"` 形式で rewrite) では sub-pipeline 経由で吸収できない。従って本 design では `pre-execute-handover` (差分ゼロの完全一致) のみを対象とする。

### 既存 sub-pipeline 契約

2026-04-14 feature-dev refresh (merge `fa04895`) と 2026-04-15 debug-flow refresh の方針で、belt skill pipeline は **sub-pipeline を一切使わない** 方向に統一されていた。現行 `plugins/belt/` 内の pipeline.yml で `invoke: { pipeline: ... }` を使うものは存在しない (Grep で確認済み)。これを受けて以下 2 つの lock contract が書かれている:

1. `crates/belt-core/tests/bug_fix_refresh.rs:97-103` `all_phases_use_skill_invoke` のコメント:
   > If a phase invokes anything, it must be a /-prefixed skill (**not a sub-pipeline**, not a cmd). Pure checkpoints bypass this check by design.
2. `crates/belt-core/tests/feature_dev_refresh.rs:51-52` `feature_dev_expands_cleanly` のコメント:
   > Refresh deletes all `uses:`/`invoke.pipeline:` references; the expanded phases must equal the top-level phases 1:1.

本 design はこの 2 契約を「`Invoker::Pipeline` を許可、`Invoker::Cmd` は引き続き禁止」という狭い範囲で緩和する。

### F2b 後のテスト構造

2026-04-17 の F2b refactor (merge `e3583ec` 等) で `crates/belt-core/tests/common/helpers.rs` (`repo_root` / `write_yaml` / `fixture_path`) と `common/narrative.rs` (`assert_narrative_*`) に共通 helper が抽出された。feature_dev_refresh / bug_fix_refresh の **lock contract 本文は変わっていない** が、本 design の実装は `common/helpers::repo_root` を使う前提で書く。

### lock-ledger.md の既存 drift

`docs/testing/lock-ledger.md` が実テストとずれている箇所を本 design 着手時に発見:

- feature_dev_refresh.rs entry:
  - 記述: `feature_dev_has_nine_phases` → 実体: `feature_dev_has_ten_phases`
  - 記述: "9 phase の順序 (design → test-scenarios → spec-review → plan → execute → ...)" — `pre-execute-handover` が欠落、実体は 10 phases
- bug_fix_refresh.rs entry:
  - 記述: "8 phase の順序 (rca → fix-plan → fix-plan-review → execute → ...)" — `pre-execute-handover` が欠落、実体は 9 phases

`scenarios_contract.rs::lock_ledger_locks_files_exist` は `locks-file:` のパス存在のみ照合し、phase list 本文までは機械照合していないため drift が蓄積。本 design は sub-pipeline delegation 反映のついでにこの既存 drift も併せて修正する。

## Design

### File layout

```
plugins/belt/skills/
├── handover/
│   ├── SKILL.md              # 既存 — 無変更
│   └── checkpoint.yml        # 新規 — sub-pipeline SSOT
├── feature-dev/
│   └── pipeline.yml          # 修正 — pre-execute-handover を delegation 化
└── bug-fix/
    └── pipeline.yml          # 修正 — 同上
```

配置理由:

- `checkpoint.yml` の gate が要求する `handover.md` の **producer は `/belt:handover` スキル自身** → checkpoint phase は handover スキルの semantic な責務延長であり、同一ディレクトリ凝集が自然
- 新規トップレベルディレクトリ (`plugins/belt/pipelines/` 等) を作らず YAGNI を維持
- 将来 handover 挙動が変わる際、checkpoint 内容と SKILL.md を 1 ディレクトリ内で同時更新できる
- file 名 `checkpoint.yml` は phase 本質 (context reset barrier) を表現、parent phase id `pre-execute-handover` との重複を避ける

### Sub-pipeline content (`plugins/belt/skills/handover/checkpoint.yml`)

```yaml
name: pre-execute-handover-checkpoint
version: 1
description: "Context reset checkpoint shared by pipelines that require /belt:handover + /clear + /belt:resume before entering the execute phase"

phases:
  - id: checkpoint
    description: >-
      Context checkpoint before execute. Run `/belt:handover`, then `/clear`,
      then `/belt:resume` in a new session. The gate passes once the handover
      note exists under `.belt/runs/{run_id}/`.
    confirm: true
    gate:
      - file_exists: ".belt/runs/{run_id}/handover.md"
```

- `inputs:` なし (parent から値を渡す必要がない)
- `version: 1` は `SubPipeline` struct の required field
- `description` (pipeline-level) は human-readable 用途、lint に影響なし

### Parent phase の書き換え (feature-dev / bug-fix 共通)

変更前 (両 pipeline 同一 8 行):

```yaml
  - id: pre-execute-handover
    description: >-
      Context checkpoint before execute. Run `/belt:handover`, then `/clear`,
      then `/belt:resume` in a new session. The gate passes once the handover
      note exists under `.belt/runs/{run_id}/`.
    confirm: true
    gate:
      - file_exists: ".belt/runs/{run_id}/handover.md"
```

変更後 (両 pipeline 3 行):

```yaml
  - id: pre-execute-handover
    invoke:
      pipeline: ../handover/checkpoint.yml
```

- parent phase id は `pre-execute-handover` を維持 (pipeline flow のラベルを保存、lock test の `EXPECTED_PHASES` 順序を変えない)
- `description` 不要: `lint.rs::lint_pipeline` は `Invoker::Pipeline` variant の parent phase を leaf-description 要件から exempt (line 114-127)
- `confirm` / `gate` は sub-phase 側に集約 — expander の `expand_sub_pipeline` は最後の sub-phase に parent の gate/regate/validate/config を append するが、本 design では parent に何も置かないため sub-phase の値が runtime の真実
- path `../handover/checkpoint.yml` は `expand_pipeline` が `pipeline_file.parent().unwrap_or_else(|| Path::new("."))` を base_dir にして `base_dir.join(sub_path)` で解決 — feature-dev/pipeline.yml からも bug-fix/pipeline.yml からも同じ文字列で plugins/belt/skills/handover/checkpoint.yml に到達する

### Expander 展開結果

`expand_pipeline("plugins/belt/skills/feature-dev/pipeline.yml")` 実行後:

- parse 結果 (10 top-level phases, id 不変): `design`, `test-scenarios`, `spec-review`, `plan`, `pre-execute-handover`, `execute`, `code-review`, `monkey-test`, `dogfood`, `integrate`
- expand 結果 (10 ExpandedPhase, pre-execute-handover が namespaced): `design`, ..., `plan`, **`pre-execute-handover/checkpoint`**, `execute`, ...

`expanded.len() == pipeline.phases.len()` (10 == 10) は偶然維持される (1 parent → 1 sub-phase)。bug-fix 側も同様に 9 → 9。

### Lint 通過性

- `check_invoke_pipeline_exists`: `base_dir.join("../handover/checkpoint.yml")` が実在 → OK
- `check_empty_phase`: parent phase は `invoke.is_some()` で `has_action = true` → OK
- `check_leaf_description`: parent は `Invoker::Pipeline` で exempt、sub-phase は description あり → OK
- `check_produces_protected_by_gate`: sub-phase は `produces` なし、対象外 → OK
- Phase 2 expansion check: `expand_pipeline` を実行して sub-pipeline の解決とマージを確認 → OK

## Contract changes

### Relaxed: "belt skill pipeline is skill-invoke-only"

既存 (`bug_fix_refresh.rs::all_phases_use_skill_invoke`):
> If a phase invokes anything, it must be a /-prefixed skill (not a sub-pipeline, not a cmd). Pure checkpoints bypass this check by design.

新:
> If a phase invokes anything, it must be either (a) a /-prefixed skill (`Invoker::Skill`) or (b) a sub-pipeline (`Invoker::Pipeline`). Pure checkpoints (invoke is None) continue to bypass this check. `Invoker::Cmd` remains prohibited.

### Relaxed: "Pure-checkpoint phases have no invoke"

既存 (`bug_fix_refresh.rs::all_phases_have_max_retries_3_and_confirm_true` のコメント):
> Pure-checkpoint phases (phase.invoke.is_none()) are exempt from the max_retries invariant.

新:
> Pure-checkpoint semantics survive at the sub-pipeline leaf level. The parent phase that delegates via `Invoker::Pipeline` carries no confirm/gate/max_retries of its own — it is a thin delegation stub whose invoke is Some but whose body is defined elsewhere. Top-level assertions (confirm/max_retries) skip sub-pipeline delegation parents; the checkpoint leaf sub-phase remains `invoke.is_none()` with only `confirm: true` + `file_exists` gate.

### Unchanged

- `Invoker::Cmd` は引き続き禁止 (pipeline.yml 内で shell command 直接 invoke は許可しない)
- `confirm: true` は全 Pure-checkpoint で維持 (sub-phase 側に保存)
- `max_retries: 3` は retry-able な skill invoke phase で要求 (sub-pipeline delegation parent と sub-checkpoint は両方 exempt)
- phase order / count (`EXPECTED_PHASES`) は不変 (parent id `pre-execute-handover` を維持)
- `code-review.regate = [execute]` 契約は不変
- narrative artifact 契約は不変 (pre-execute-handover は narrative phase ではない)

## Impact analysis

| File | 変更内容 |
|---|---|
| `plugins/belt/skills/handover/checkpoint.yml` | **新規作成** (11 行) |
| `plugins/belt/skills/feature-dev/pipeline.yml` | `pre-execute-handover` phase を 8 行 → 3 行に書き換え |
| `plugins/belt/skills/bug-fix/pipeline.yml` | 同上 |
| `crates/belt-core/tests/feature_dev_refresh.rs` | `feature_dev_expands_cleanly` のコメント更新 (uses:/invoke.pipeline: 復活を反映)。assert は数的に通る (10 == 10) |
| `crates/belt-core/tests/bug_fix_refresh.rs` | (1) `all_phases_use_skill_invoke`: `Invoker::Pipeline` を許可する分岐を追加 + コメント更新 / (2) `all_phases_have_max_retries_3_and_confirm_true`: sub-pipeline delegation parent を skip する条件追加 + コメント更新 |
| `docs/testing/lock-ledger.md` | (a) feature_dev_refresh entry: `feature_dev_has_ten_phases` に修正、phase list に `pre-execute-handover` を追加し sub-pipeline delegation note を追記 / (b) bug_fix_refresh entry: "9 phase" に修正、phase list に `pre-execute-handover` を追加し sub-pipeline delegation note を追記、`all_phases_use_skill_invoke` / `all_phases_have_max_retries_3_and_confirm_true` の shape dimension 表現を新契約に合わせて書き直す |
| `plugin.json` / `marketplace.json` | 変更なし |
| feature-dev / bug-fix / handover / resume SKILL.md | 変更なし (Grep で pre-execute-handover 直接言及なし確認済み) |
| `docs/testing/cli-behavior/*.yml` | 変更なし (`expander_test.rs` / `expander_with_test.rs` が sub-pipeline 展開動作を既にカバー) |
| review_skills_refresh.rs / shared_criteria_parity.rs / shared_filter_parity.rs | 変更なし (feature-dev/bug-fix pipeline 構造に直接依存しない) |

### lock test 修正の形

`bug_fix_refresh.rs::all_phases_use_skill_invoke` (line 97-124) 修正例:

```rust
#[test]
fn all_phases_use_skill_invoke() {
    // Contract: if a phase invokes anything, it must be either a /-prefixed
    // skill (Invoker::Skill) or a sub-pipeline (Invoker::Pipeline). Pure
    // checkpoints (phase.invoke.is_none()) and Invoker::Cmd are both handled
    // separately: pure checkpoints bypass this check by design, and Cmd
    // invocation is prohibited (no test path constructs it, so it would
    // panic through the `_` arm if ever introduced).
    let pipeline = bug_fix_pipeline();
    for phase in pipeline.phases.iter().filter(|p| p.invoke.is_some()) {
        let invoker = phase
            .invoke
            .as_ref()
            .expect("filter guarantees invoke.is_some()");
        match invoker {
            Invoker::Skill { skill, .. } => {
                assert!(
                    skill.starts_with('/'),
                    "phase '{}' skill must start with '/', got '{skill}'",
                    phase.id
                );
            }
            Invoker::Pipeline { pipeline: sub_path, .. } => {
                assert!(
                    !sub_path.is_empty(),
                    "phase '{}' sub-pipeline path must be non-empty",
                    phase.id
                );
                // Path existence is already asserted at lint-time by
                // `lint.rs::check_invoke_pipeline_exists`; no need to re-check
                // filesystem state here.
            }
            _ => panic!(
                "phase '{}' must use Invoker::Skill or Invoker::Pipeline variant, got {invoker:?}",
                phase.id
            ),
        }
    }
}
```

`bug_fix_refresh.rs::all_phases_have_max_retries_3_and_confirm_true` (line 222-242) 修正例:

```rust
#[test]
fn all_phases_have_max_retries_3_and_confirm_true() {
    // Sub-pipeline delegation phases (Invoker::Pipeline) are thin stubs: the
    // real confirm/gate/max_retries live on the sub-phase. Top-level shape
    // assertions skip them. Pure-checkpoint phases (invoke.is_none()) are
    // also exempt from max_retries — retry has no meaning without a
    // retry-able body. All non-delegation phases still require confirm: true
    // and, when they carry an Invoker::Skill, max_retries == 3.
    let pipeline = bug_fix_pipeline();
    for phase in &pipeline.phases {
        if matches!(phase.invoke, Some(Invoker::Pipeline { .. })) {
            continue;
        }
        assert!(phase.confirm, "phase '{}' confirm must be true", phase.id);
        if phase.invoke.is_some() {
            assert_eq!(
                phase.max_retries, 3,
                "phase '{}' max_retries must be 3",
                phase.id
            );
        }
    }
}
```

`feature_dev_refresh.rs::feature_dev_expands_cleanly` (line 48-56) 修正例:

```rust
#[test]
fn feature_dev_expands_cleanly() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;
    // The feature-dev pipeline delegates `pre-execute-handover` to the shared
    // sub-pipeline `../handover/checkpoint.yml`. That single sub-pipeline
    // contains exactly one leaf phase (`checkpoint`), so expansion replaces
    // one top-level phase with one namespaced sub-phase and the total count
    // remains equal. If additional leaves are added to the sub-pipeline in
    // the future, this assertion will need to be updated (the right form is
    // then `expanded.len() == pipeline.phases.len() + (sub_leaves - 1)`).
    let expanded = expand_pipeline(&feature_dev_pipeline_path())?;
    assert_eq!(expanded.len(), pipeline.phases.len());
    Ok(())
}
```

### lock-ledger.md 修正の形

feature_dev_refresh.rs entry の "11 test fn 名" は実体と一致 (既に OK) だが、`feature_dev_has_nine_phases` の記述を `feature_dev_has_ten_phases` に修正、phase list を "10 phase の順序 (`design → test-scenarios → spec-review → plan → pre-execute-handover → execute → code-review → monkey-test → dogfood → integrate`)" に修正、新たに "**sub-pipeline delegation**: `pre-execute-handover` は `../handover/checkpoint.yml` に delegate (展開後 id: `pre-execute-handover/checkpoint`)" を追記する。

bug_fix_refresh.rs entry は "8 phase の順序" を "9 phase の順序 (`rca → fix-plan → fix-plan-review → pre-execute-handover → execute → code-review → monkey-test → dogfood → integrate`)" に修正、`all_phases_use_skill_invoke` の記述を「全 phase が `Invoker::Skill` または `Invoker::Pipeline` variant を取り、`Invoker::Cmd` は不在」に書き換え、`all_phases_have_max_retries_3_and_confirm_true` を「sub-pipeline delegation parent を skip、それ以外は `confirm == true`、skill invoke phase は `max_retries == 3`」に書き換え、"**sub-pipeline delegation**: `pre-execute-handover` は `../handover/checkpoint.yml` に delegate" を追記する。

## Migration sequence

1. `plugins/belt/skills/handover/checkpoint.yml` を作成
2. `plugins/belt/skills/feature-dev/pipeline.yml` の pre-execute-handover phase を delegation 化
3. `plugins/belt/skills/bug-fix/pipeline.yml` の pre-execute-handover phase を delegation 化
4. `crates/belt-core/tests/bug_fix_refresh.rs` の 2 test + コメントを修正
5. `crates/belt-core/tests/feature_dev_refresh.rs` の 1 コメントを修正
6. `docs/testing/lock-ledger.md` の feature_dev_refresh / bug_fix_refresh entry 修正 (既存 drift + sub-pipeline delegation 反映)
7. `cargo test -p belt-core` で全 lock test が pass することを確認
8. `cargo clippy -p belt-core -- -D warnings` / `cargo clippy -p belt-agent -- -D warnings` で warning ゼロ確認
9. `cargo run -p belt -- lint plugins/belt/skills/feature-dev/pipeline.yml` / `bug-fix/pipeline.yml` で lint 通過確認
10. `cargo run -p belt-agent -- init plugins/belt/skills/feature-dev/pipeline.yml` + `cargo run -p belt-agent -- next --run <id>` で展開後 phase id `pre-execute-handover/checkpoint` が表示されることを目視確認

## Open questions

- なし。Q1..Q7 は brainstorming で合意済み。

## References

- `crates/belt-core/src/expander.rs` — sub-pipeline 展開と substitute ルール
- `crates/belt-core/src/lint.rs` — 静的検証 + Phase 2 expansion check
- `crates/belt-core/src/model.rs` — `Phase` / `SubPipeline` / `Invoker` / `ExpandedPhase` 型定義
- `crates/belt-core/tests/feature_dev_refresh.rs` / `bug_fix_refresh.rs` — lock contract の本文
- `crates/belt-core/tests/common/helpers.rs` — `repo_root()` 他の shared test helper (F2b で抽出)
- `docs/testing/lock-ledger.md` — lock test の台帳
- `docs/specs/2026-04-17-belt-handover-resume-design.md` — `/belt:handover` / `/belt:resume` skill + pre-execute-handover phase 新設の経緯
- `docs/specs/2026-04-14-feature-dev-refresh-design.md` — 「sub-pipeline を使わない」方針が確立された spec
- `docs/specs/2026-04-15-debug-flow-refresh-design.md` — bug-fix (旧 debug-flow) pipeline の shape 契約
