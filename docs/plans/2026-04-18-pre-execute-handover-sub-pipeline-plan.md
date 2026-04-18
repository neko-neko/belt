# pre-execute-handover Sub-pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** feature-dev / bug-fix pipeline 両方の `pre-execute-handover` phase を `plugins/belt/skills/handover/checkpoint.yml` sub-pipeline へ抽出し、parent phase から delegate する形に変更する。合わせて既存 lock contract 2 つを narrow に緩和し、`docs/testing/lock-ledger.md` の関連 drift を同時修正する。

**Architecture:** handover スキル配下に sub-pipeline を 1 ファイル新設。feature-dev / bug-fix の parent phase (id は `pre-execute-handover` のまま) は `invoke: { pipeline: ../handover/checkpoint.yml }` で委譲。展開後の phase id は `pre-execute-handover/checkpoint`。`Invoker::Skill` / `Invoker::Pipeline` 両対応に lock test を緩和 (`Invoker::Cmd` は引き続き禁止)。

**Tech Stack:** Rust 1.94.1, belt-core (serde-saphyr YAML parser / `expand_pipeline`), YAML pipeline definitions, markdown (docs).

**Spec:** `docs/specs/2026-04-18-pre-execute-handover-sub-pipeline-design.md`

---

## File Structure

**Create:**
- `plugins/belt/skills/handover/checkpoint.yml` — sub-pipeline SSOT (11 行)

**Modify:**
- `plugins/belt/skills/feature-dev/pipeline.yml` — pre-execute-handover phase を 8 行 → 3 行 に書き換え
- `plugins/belt/skills/bug-fix/pipeline.yml` — 同上
- `crates/belt-core/tests/feature_dev_refresh.rs` — new test 2 個追加 + `feature_dev_expands_cleanly` コメント更新
- `crates/belt-core/tests/bug_fix_refresh.rs` — new test 2 個追加 + 既存 2 test (fn 名 1 個改名含む) を sub-pipeline 許容に緩和
- `docs/testing/lock-ledger.md` — feature_dev_refresh / bug_fix_refresh entry の既存 drift 修正 + sub-pipeline delegation note 追記

**Unchanged (spec Non-Goals):**
- `plugin.json` / `marketplace.json`
- feature-dev / bug-fix / handover / resume SKILL.md
- `docs/testing/cli-behavior/*.yml`
- belt-core `model.rs` / `parser.rs` / `expander.rs` / `lint.rs`
- `review_skills_refresh.rs` / `shared_criteria_parity.rs` / `shared_filter_parity.rs`

---

## Task 1: 新規 lock test 追加 (delegation と expansion の shape を red-first で lock)

**Files:**
- Modify: `crates/belt-core/tests/feature_dev_refresh.rs` (2 test fn 追加、末尾に)
- Modify: `crates/belt-core/tests/bug_fix_refresh.rs` (2 test fn 追加、末尾に)

**Target behavior:** 各 pipeline の `pre-execute-handover` phase が (a) `Invoker::Pipeline` で `../handover/checkpoint.yml` に delegate、(b) `expand_pipeline` 結果に `pre-execute-handover/checkpoint` phase id が含まれる、ことを lock する。

- [ ] **Step 1.1: feature_dev_refresh.rs の import を拡張して Invoker を追加**

`crates/belt-core/tests/feature_dev_refresh.rs` の冒頭 import ブロック (line 11-13 周辺) を次に書き換える:

```rust
use belt_core::{
    error::BeltError,
    expander::expand_pipeline,
    model::{ArgType, Invoker},
    parser::parse_pipeline,
};
```

(既存: `model::ArgType` → 追加で `Invoker` を同居)

- [ ] **Step 1.2: feature_dev_refresh.rs の末尾に 2 つの新規 test fn を追加**

ファイル末尾 (line 332 以降) に追加:

```rust
// --- pre-execute-handover sub-pipeline delegation (spec 2026-04-18) ---

#[test]
fn pre_execute_handover_delegates_to_sub_pipeline() {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())
        .expect("feature-dev pipeline must parse");
    let phase = pipeline
        .phases
        .iter()
        .find(|p| p.id == "pre-execute-handover")
        .expect("pre-execute-handover phase must exist");
    match phase.invoke.as_ref() {
        Some(Invoker::Pipeline {
            pipeline: sub_path,
            with,
        }) => {
            assert_eq!(
                sub_path, "../handover/checkpoint.yml",
                "pre-execute-handover must delegate to ../handover/checkpoint.yml"
            );
            assert!(
                with.is_empty(),
                "pre-execute-handover delegation must not pass any `with` args"
            );
        }
        other => panic!(
            "pre-execute-handover must use Invoker::Pipeline, got {other:?}"
        ),
    }
}

#[test]
fn pre_execute_handover_expands_to_namespaced_checkpoint() {
    let expanded = expand_pipeline(&feature_dev_pipeline_path())
        .expect("feature-dev pipeline must expand");
    let ids: Vec<&str> = expanded.iter().map(|p| p.id.as_str()).collect();
    assert!(
        ids.contains(&"pre-execute-handover/checkpoint"),
        "expanded pipeline must contain phase id 'pre-execute-handover/checkpoint', got: {ids:?}"
    );
}
```

- [ ] **Step 1.3: bug_fix_refresh.rs の末尾に同じ形の 2 つの新規 test fn を追加**

`crates/belt-core/tests/bug_fix_refresh.rs` の末尾 (line 425 以降) に追加:

```rust
// --- pre-execute-handover sub-pipeline delegation (spec 2026-04-18) ---

#[test]
fn pre_execute_handover_delegates_to_sub_pipeline() {
    let pipeline = bug_fix_pipeline();
    let phase = pipeline
        .phases
        .iter()
        .find(|p| p.id == "pre-execute-handover")
        .expect("pre-execute-handover phase must exist");
    match phase.invoke.as_ref() {
        Some(Invoker::Pipeline {
            pipeline: sub_path,
            with,
        }) => {
            assert_eq!(
                sub_path, "../handover/checkpoint.yml",
                "pre-execute-handover must delegate to ../handover/checkpoint.yml"
            );
            assert!(
                with.is_empty(),
                "pre-execute-handover delegation must not pass any `with` args"
            );
        }
        other => panic!(
            "pre-execute-handover must use Invoker::Pipeline, got {other:?}"
        ),
    }
}

#[test]
fn pre_execute_handover_expands_to_namespaced_checkpoint() {
    let expanded = expand_pipeline(&bug_fix_pipeline_path())
        .expect("bug-fix pipeline must expand");
    let ids: Vec<&str> = expanded.iter().map(|p| p.id.as_str()).collect();
    assert!(
        ids.contains(&"pre-execute-handover/checkpoint"),
        "expanded pipeline must contain phase id 'pre-execute-handover/checkpoint', got: {ids:?}"
    );
}
```

- [ ] **Step 1.4: 新規 test を実行し fail を確認**

実行: `cargo test -p belt-core --test feature_dev_refresh pre_execute_handover && cargo test -p belt-core --test bug_fix_refresh pre_execute_handover`

期待される結果 (全 4 test が fail):

- `pre_execute_handover_delegates_to_sub_pipeline` → `panicked at '... must use Invoker::Pipeline, got None'` (現時点で phase.invoke は None)
- `pre_execute_handover_expands_to_namespaced_checkpoint` → `panicked at '... must contain phase id "pre-execute-handover/checkpoint"...'` (現時点で expand は top-level id のまま)

- [ ] **Step 1.5: red 状態を WIP commit**

```bash
git add crates/belt-core/tests/feature_dev_refresh.rs \
        crates/belt-core/tests/bug_fix_refresh.rs
git commit -m "test(belt-core): add pre-execute-handover sub-pipeline delegation lock tests (red)"
```

---

## Task 2: sub-pipeline file (`handover/checkpoint.yml`) を新規作成

**Files:**
- Create: `plugins/belt/skills/handover/checkpoint.yml`

- [ ] **Step 2.1: checkpoint.yml を作成**

`plugins/belt/skills/handover/checkpoint.yml` に以下を書く:

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

- [ ] **Step 2.2: parser の smoke test (standalone)**

実行: `cargo test -p belt-core --test parser_test 2>&1 | tail -5`

期待: 既存 parser test は全 pass (新規 file は parse されない、ただ存在するだけ)。

- [ ] **Step 2.3: Task 1 の 4 test はまだ fail のまま (parent phase 未修正)**

実行: `cargo test -p belt-core --test feature_dev_refresh pre_execute_handover 2>&1 | tail -10`

期待: 引き続き fail (`Invoker::Pipeline, got None`)。これは正しい状態 — sub-pipeline は作ったが parent 側から delegation していない。

- [ ] **Step 2.4: commit**

```bash
git add plugins/belt/skills/handover/checkpoint.yml
git commit -m "feat(plugins): add pre-execute-handover checkpoint sub-pipeline"
```

---

## Task 3: feature-dev/pipeline.yml の pre-execute-handover を delegation 化 + expands_cleanly コメント更新

**Files:**
- Modify: `plugins/belt/skills/feature-dev/pipeline.yml` (line 91-98 書き換え)
- Modify: `crates/belt-core/tests/feature_dev_refresh.rs` (`feature_dev_expands_cleanly` コメント)

- [ ] **Step 3.1: feature-dev/pipeline.yml の pre-execute-handover phase を書き換え**

`plugins/belt/skills/feature-dev/pipeline.yml` の該当 phase (line 91-98) を次に置換:

変更前:
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

変更後:
```yaml
  - id: pre-execute-handover
    invoke:
      pipeline: ../handover/checkpoint.yml
```

- [ ] **Step 3.2: feature-dev 向け Task 1 の 2 test が pass することを確認**

実行: `cargo test -p belt-core --test feature_dev_refresh pre_execute_handover`

期待: 2 test が PASS:
- `pre_execute_handover_delegates_to_sub_pipeline` ✓
- `pre_execute_handover_expands_to_namespaced_checkpoint` ✓

(bug-fix 側はまだ fail)

- [ ] **Step 3.3: feature_dev_expands_cleanly のコメントを sub-pipeline 反映に更新**

`crates/belt-core/tests/feature_dev_refresh.rs` の該当 test (line 48-56) のコメントを書き換え:

変更前:
```rust
#[test]
fn feature_dev_expands_cleanly() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;
    // Refresh deletes all `uses:`/`invoke.pipeline:` references; the expanded
    // phases must equal the top-level phases 1:1.
    let expanded = expand_pipeline(&feature_dev_pipeline_path())?;
    assert_eq!(expanded.len(), pipeline.phases.len());
    Ok(())
}
```

変更後:
```rust
#[test]
fn feature_dev_expands_cleanly() -> Result<(), BeltError> {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path())?;
    // The feature-dev pipeline delegates `pre-execute-handover` to the shared
    // sub-pipeline `../handover/checkpoint.yml`. That single sub-pipeline
    // contains exactly one leaf phase (`checkpoint`), so expansion replaces
    // one top-level phase with one namespaced sub-phase and the total count
    // remains equal. If additional leaves are added to the sub-pipeline in
    // the future, this assertion must be updated (the right form is then
    // `expanded.len() == pipeline.phases.len() + (sub_leaves - 1)`).
    let expanded = expand_pipeline(&feature_dev_pipeline_path())?;
    assert_eq!(expanded.len(), pipeline.phases.len());
    Ok(())
}
```

- [ ] **Step 3.4: feature-dev 関連 test が全 pass することを確認**

実行: `cargo test -p belt-core --test feature_dev_refresh`

期待: 全 13 test (既存 11 + 新規 2) が PASS。

- [ ] **Step 3.5: lint pass を確認**

実行: `cargo run -p belt -- lint plugins/belt/skills/feature-dev/pipeline.yml`

期待: 0 diagnostics (path `../handover/checkpoint.yml` が `base_dir.join(sub_path)` で解決される)。

- [ ] **Step 3.6: commit**

```bash
git add plugins/belt/skills/feature-dev/pipeline.yml \
        crates/belt-core/tests/feature_dev_refresh.rs
git commit -m "feat(plugins/feature-dev): delegate pre-execute-handover to sub-pipeline"
```

---

## Task 4: bug-fix/pipeline.yml の pre-execute-handover を delegation 化 + 既存 lock test 緩和

**Files:**
- Modify: `plugins/belt/skills/bug-fix/pipeline.yml` (line 71-78 書き換え)
- Modify: `crates/belt-core/tests/bug_fix_refresh.rs` (2 既存 test 緩和 + 1 fn 改名)

- [ ] **Step 4.1: bug-fix/pipeline.yml の pre-execute-handover phase を書き換え**

`plugins/belt/skills/bug-fix/pipeline.yml` の該当 phase (line 71-78) を次に置換:

変更前:
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

変更後:
```yaml
  - id: pre-execute-handover
    invoke:
      pipeline: ../handover/checkpoint.yml
```

- [ ] **Step 4.2: bug-fix 向け Task 1 の 2 test が pass、既存 2 test が fail することを確認**

実行: `cargo test -p belt-core --test bug_fix_refresh 2>&1 | tail -40`

期待:
- `pre_execute_handover_delegates_to_sub_pipeline` ✓
- `pre_execute_handover_expands_to_namespaced_checkpoint` ✓
- `all_phases_use_skill_invoke` ✗ (panicked — `Invoker::Pipeline` variant が未対応の `_` arm に落ちる)
- `all_phases_have_max_retries_3_and_confirm_true` ✗ (panicked — `pre-execute-handover` の `confirm` が false、parent phase に confirm を書いていないため)

- [ ] **Step 4.3: `all_phases_use_skill_invoke` を `all_phases_use_skill_or_pipeline_invoke` に改名し中身を緩和**

`crates/belt-core/tests/bug_fix_refresh.rs` の line 96-124 を次に置換:

変更前:
```rust
#[test]
fn all_phases_use_skill_invoke() {
    // Pure-checkpoint phases (phase.invoke.is_none()) are exempt: they carry
    // no implementation work, only a file_exists gate. E.g. `pre-execute-handover`
    // is a context-reset barrier between plan and execute. The contract under
    // test is: *if* a phase invokes anything, it must be a /-prefixed skill
    // (not a sub-pipeline, not a cmd). Pure checkpoints bypass this check by
    // design.
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
            _ => panic!(
                "phase '{}' must use Invoker::Skill variant, got {invoker:?}",
                phase.id
            ),
        }
    }
}
```

変更後:
```rust
#[test]
fn all_phases_use_skill_or_pipeline_invoke() {
    // Contract: if a phase invokes anything, it must be either a /-prefixed
    // skill (Invoker::Skill) or a sub-pipeline (Invoker::Pipeline). Pure
    // checkpoints (phase.invoke.is_none()) continue to bypass this check by
    // design. Invoker::Cmd is not a defined variant in belt-core model —
    // match is exhaustive on the two variants; no wildcard arm is required.
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
            Invoker::Pipeline {
                pipeline: sub_path,
                ..
            } => {
                assert!(
                    !sub_path.is_empty(),
                    "phase '{}' sub-pipeline path must be non-empty",
                    phase.id
                );
            }
        }
    }
}
```

**Note:** `#![allow(clippy::match_wildcard_for_single_variants, ...)]` のうち `match_wildcard_for_single_variants` が不要になるが、他 test で使われている可能性があるため allow 属性は触らない (削除は separate follow-up の判断)。

- [ ] **Step 4.4: `all_phases_have_max_retries_3_and_confirm_true` を sub-pipeline delegation 対応に緩和**

`crates/belt-core/tests/bug_fix_refresh.rs` の line 222-242 を次に置換:

変更前:
```rust
#[test]
fn all_phases_have_max_retries_3_and_confirm_true() {
    // Pure-checkpoint phases (phase.invoke.is_none()) are exempt from the
    // max_retries invariant: `max_retries` makes sense only when there is
    // implementation work to retry. Pure checkpoints (e.g. `pre-execute-handover`)
    // have no invoke and no retry-able body; their `max_retries` stays at the
    // serde default (0). The `confirm: true` invariant still applies to all
    // phases because a checkpoint phase is exactly where we want a human
    // confirm beat.
    let pipeline = bug_fix_pipeline();
    for phase in &pipeline.phases {
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

変更後:
```rust
#[test]
fn all_phases_have_max_retries_3_and_confirm_true() {
    // Sub-pipeline delegation phases (Invoker::Pipeline) are thin stubs: the
    // real confirm/gate/max_retries live on the sub-phase. Top-level shape
    // assertions skip them. Pure-checkpoint phases (invoke.is_none()) are
    // also exempt from max_retries — retry has no meaning without a
    // retry-able body. All non-delegation phases still require confirm: true;
    // skill-invoke phases additionally require max_retries == 3.
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

- [ ] **Step 4.5: bug-fix 関連 test が全 pass することを確認**

実行: `cargo test -p belt-core --test bug_fix_refresh`

期待: 全 21 test (既存 19 + 新規 2) が PASS。fn 名変更で `all_phases_use_skill_or_pipeline_invoke` が表示される。

- [ ] **Step 4.6: lint pass を確認**

実行: `cargo run -p belt -- lint plugins/belt/skills/bug-fix/pipeline.yml`

期待: 0 diagnostics。

- [ ] **Step 4.7: commit**

```bash
git add plugins/belt/skills/bug-fix/pipeline.yml \
        crates/belt-core/tests/bug_fix_refresh.rs
git commit -m "feat(plugins/bug-fix): delegate pre-execute-handover to sub-pipeline, relax lock tests"
```

---

## Task 5: docs/testing/lock-ledger.md の update (既存 drift 修正 + sub-pipeline delegation 反映)

**Files:**
- Modify: `docs/testing/lock-ledger.md` (feature_dev_refresh / bug_fix_refresh entry)

- [ ] **Step 5.1: feature_dev_refresh.rs entry の更新**

`docs/testing/lock-ledger.md` の line 9-51 付近 (feature_dev_refresh.rs セクション) の以下 4 点を修正:

(a) `test-fn-count: 11` → `test-fn-count: 13` (pre_execute_handover_delegates_to_sub_pipeline と pre_execute_handover_expands_to_namespaced_checkpoint を追加)

(b) **11 test fn 名** リスト:
- `feature_dev_has_nine_phases` を `feature_dev_has_ten_phases` に修正 (既存 drift)
- リスト末尾に 2 行追加:
  - `pre_execute_handover_delegates_to_sub_pipeline`
  - `pre_execute_handover_expands_to_namespaced_checkpoint`
- リスト見出しを **11 test fn 名** → **13 test fn 名** に

(c) **pipeline.yml shape dimensions locked** リスト:
- "9 phase の順序 (`design → test-scenarios → spec-review → plan → execute → code-review → monkey-test → dogfood → integrate`)" を次に修正:
  - "10 phase の順序 (`design → test-scenarios → spec-review → plan → pre-execute-handover → execute → code-review → monkey-test → dogfood → integrate`)"
- リスト末尾に追加:
  - "`pre-execute-handover` が `../handover/checkpoint.yml` sub-pipeline に delegate (展開後 phase id: `pre-execute-handover/checkpoint`)"

(d) その他の dimensions 記述は変更なし。

- [ ] **Step 5.2: bug_fix_refresh.rs entry の更新**

`docs/testing/lock-ledger.md` の line 55-112 付近 (bug_fix_refresh.rs セクション) の以下 4 点を修正:

(a) `test-fn-count: 19` → `test-fn-count: 21`

(b) **19 test fn 名** リスト:
- `all_phases_use_skill_invoke` を `all_phases_use_skill_or_pipeline_invoke` に改名 (Task 4 で fn 名変更)
- リスト末尾に 2 行追加:
  - `pre_execute_handover_delegates_to_sub_pipeline`
  - `pre_execute_handover_expands_to_namespaced_checkpoint`
- リスト見出しを **19 test fn 名** → **21 test fn 名** に

(c) **pipeline.yml shape dimensions locked** リスト:
- "8 phase の順序 (`rca → fix-plan → fix-plan-review → execute → code-review → monkey-test → dogfood → integrate`)" を次に修正:
  - "9 phase の順序 (`rca → fix-plan → fix-plan-review → pre-execute-handover → execute → code-review → monkey-test → dogfood → integrate`)"
- "全 phase が `Invoker::Skill` variant で `skill` が leading slash 付き (`bug_fix_refresh.rs:96-117`)" を次に修正:
  - "skill-invoke phase は `Invoker::Skill` variant で leading slash 付き、sub-pipeline delegation phase は `Invoker::Pipeline` variant で `pipeline` path が非空 (`bug_fix_refresh.rs:96-129`)"
- "全 phase の `max_retries == 3` + `confirm == true` blanket (`bug_fix_refresh.rs:216-226`)" を次に修正:
  - "skill-invoke phase は `max_retries == 3` + `confirm == true`、sub-pipeline delegation parent (`Invoker::Pipeline`) は top-level assertion を skip (confirm / max_retries は sub-phase 側で管理) (`bug_fix_refresh.rs:222-244`)"
- リスト末尾に追加:
  - "`pre-execute-handover` が `../handover/checkpoint.yml` sub-pipeline に delegate (展開後 phase id: `pre-execute-handover/checkpoint`)"

(d) Cross-coupling 節はそのまま。

- [ ] **Step 5.3: lock-ledger.md の自己整合性を scenarios_contract で検証**

実行: `cargo test -p belt-core --test scenarios_contract lock_ledger_locks_files_exist`

期待: PASS (`locks-file:` パスは実在ファイルに変わらず指しているため)。

- [ ] **Step 5.4: commit**

```bash
git add docs/testing/lock-ledger.md
git commit -m "docs(testing): reflect pre-execute-handover sub-pipeline in lock-ledger"
```

---

## Task 6: 統合 verification (no commit)

**Files:** なし (観測のみ)

- [ ] **Step 6.1: workspace 全体の clippy 確認**

実行: `cargo clippy --workspace -- -D warnings 2>&1 | tail -10`

期待: 0 warnings, 0 errors.

- [ ] **Step 6.2: workspace 全体の test 確認**

実行: `cargo test --workspace 2>&1 | tail -20`

期待: 全 test PASS (既存 397 + 新規 4 = 401 test 予想、F2b 以降の数と一致するか確認)。

- [ ] **Step 6.3: belt lint の feature-dev / bug-fix 両 pipeline**

実行:
```bash
cargo run -p belt -- lint plugins/belt/skills/feature-dev/pipeline.yml
cargo run -p belt -- lint plugins/belt/skills/bug-fix/pipeline.yml
```

期待: 両方とも 0 diagnostics.

- [ ] **Step 6.4: belt-agent で展開後 phase id を目視確認**

実行 (一時 run を init してすぐに status を見る):
```bash
cd /tmp && mkdir -p belt-verify && cd belt-verify && git init -q
cp -R <repo>/plugins .
<repo>/target/debug/belt-agent init plugins/belt/skills/feature-dev/pipeline.yml \
    --arg e2e=false --arg codex=false 2>&1 | head -5
```

期待出力に phase id `pre-execute-handover/checkpoint` が現れる (例えば `next_phase: pre-execute-handover/checkpoint` の形で初回 next 時)。

`<repo>` は `/Users/nishikataseiichi/go/src/github.com/neko-neko/belt` 等、実リポジトリ絶対パスに置換。

- [ ] **Step 6.5: git log を確認して commit 5 個が順に積まれていることを確認**

実行: `git log --oneline -6`

期待 (新しい順):
```
<hash> docs(testing): reflect pre-execute-handover sub-pipeline in lock-ledger
<hash> feat(plugins/bug-fix): delegate pre-execute-handover to sub-pipeline, relax lock tests
<hash> feat(plugins/feature-dev): delegate pre-execute-handover to sub-pipeline
<hash> feat(plugins): add pre-execute-handover checkpoint sub-pipeline
<hash> test(belt-core): add pre-execute-handover sub-pipeline delegation lock tests (red)
<previous commit — spec>
```

---

## Summary

5 commit + 1 verification:

1. **Task 1** → `test(belt-core): add pre-execute-handover sub-pipeline delegation lock tests (red)` (test-only WIP, 4 new test fns が red)
2. **Task 2** → `feat(plugins): add pre-execute-handover checkpoint sub-pipeline` (sub-pipeline 新設)
3. **Task 3** → `feat(plugins/feature-dev): delegate pre-execute-handover to sub-pipeline` (feature-dev green + expands_cleanly コメント)
4. **Task 4** → `feat(plugins/bug-fix): delegate pre-execute-handover to sub-pipeline, relax lock tests` (bug-fix green + 既存 2 test 緩和 + fn 改名)
5. **Task 5** → `docs(testing): reflect pre-execute-handover sub-pipeline in lock-ledger` (drift 修正 + sub-pipeline 反映)
6. **Task 6** → verification のみ (no commit)

---

## Rollback strategy

実装中に想定外の問題が出た場合:

- **Task 3 / Task 4 で parent phase delegation が lint に弾かれる**: `cargo run -p belt -- lint <path>` の出力を見て、path resolution が失敗しているか確認。`../handover/checkpoint.yml` の相対参照は `base_dir = plugins/belt/skills/{feature-dev,bug-fix}/` から解決される想定。
- **Task 4 で `all_phases_have_max_retries_3_and_confirm_true` が sub-pipeline 除外後も fail**: parent phase の body 記述誤り。`invoke: pipeline:` のみの 3 行構造になっているか再確認。
- **Task 5 の lock-ledger.md drift 修正で対応しない箇所が見つかった**: 別 commit で separate に追加 (plan の scope 外扱い)。
- **roll-back が必要**: `git reset --hard HEAD~N` は `docs(specs):` commit を保持する位置 (N = 失敗時点の本 plan commits 数) まで戻す。リモート push 前のみ許可。

## Known hazards

- **`match_wildcard_for_single_variants` allow 属性**: Task 4 で fn 中身から wildcard arm を外すため不要化するが、他 test で使用中のため `#![allow(...)]` 行全体は残す。削除は follow-up PR で。
- **Task 3 の feature_dev_expands_cleanly assert**: `expanded.len() == pipeline.phases.len()` は偶然通るだけで、将来 checkpoint sub-pipeline に 2 つ目 leaf phase を追加した時点で破綻する。コメントにその旨明記済み。
- **既存 lock-ledger.md drift の二次発見**: 実修正中に他の drift が見つかった場合 (例: shared_criteria_parity.rs entry の内容違い)、plan scope 外として別 PR で対応する。
