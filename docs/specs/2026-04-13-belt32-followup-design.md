# BELT-32 Follow-up: SKILL.md drift collapse + args passthrough

> Linear: follow-up to [BELT-32](https://linear.app/neko-neko/issue/BELT-32)
> Date: 2026-04-13

## Context

BELT-32 Plan B (2026-04-13) で belt-core の typed `invoke:` / `produces` / `consumes` / `validate` への移行が完結したが、protocol documents と orchestrator pipeline.yml に 3 種類の残債が残っている。

- **Drift**: 5 つの leaf-skill SKILL.md (`spec-review` / `code-review` / `test-review` / `implementation-review` / `smoke-test`) が legacy `config.*` key を参照する Dispatch Rules 表を抱えたまま。pipeline.yml 側は既に `invoke.agents` / `invoke.args.*` へ移行済みのため記述と実態が食い違う。
- **Functional gap**: feature-dev.yml / debug-flow.yml で `invoke.pipeline` に `with:` 句がなく、親 run の `--iterations N` / `--codex` / `--ui` / `--swarm` が子 sub-pipeline に伝播しない。argument-hint が実質 no-op。
- **Doc bug**: `skills/belt-agent/SKILL.md:78` が `resolved_path: null` を "artifact missing" の signal と記述するが、`crates/belt-core/src/view.rs:94-95` で `Option<String> + skip_serializing_if = "Option::is_none"` のため実際は JSON から omit される (`null` シリアライズされない)。

### 前提

- `Invoker::Pipeline { pipeline, with: HashMap<String, serde_json::Value> }` は belt-core の model 層に既に存在 (`crates/belt-core/src/model.rs:260-264`)。
- belt-core の `expander` は `with` を pass-through するのみで解決しない (`crates/belt-core/src/expander.rs:28`)。template 解決は orchestrator 責務 (既存の `IterationsSpec::Template("args.X")` と同じレイヤー方針)。
- CLAUDE.md の SKILL.md Authoring Principle (2026-04-07): "Protocol is taught by belt-agent SKILL.md. Do not re-state." を leaf-skill SKILL.md に適用する段階が来ている。

## Design

### Architecture

変更はすべて **protocol 文 (1 file) + leaf SKILL.md (5 file) + orchestrator pipeline.yml (2 file)** のドキュメント/YAML 層に閉じる。belt-core / belt-agent / belt binary に Rust コード変更なし。

### Task A — skills/belt-agent/SKILL.md の 2 edit

**A-1. L78 artifact missing signal wording 修正**

現:

> Use `exists: false` and `resolved_path: null` as a signal that the declared artifact is missing.

新:

> Use `exists: false` as the signal that the declared artifact is missing. The `resolved_path` field is omitted from JSON (not serialized as `null`) when unresolved.

**A-2. L58 pipeline invoke 行に template 解決 note 追加**

現 `pipeline` 行末尾 "Treat the nested run as a black-box until it reports `completed`." の後に 1 段落追加:

> When a `with` entry's value is a string of the form `"args.X"`, resolve it against the parent run's `args` before calling `belt-agent init --arg X=<value>`. Literal values (bool, number, non-template string) are passed through verbatim. If `args.X` is absent in the parent, omit the `--arg` instead of passing `null` — the sub-pipeline's declared default applies.

### Task B — leaf SKILL.md の dispatch SSOT 集約 (B3)

対象 5 file:
- `examples/skills/spec-review/SKILL.md`
- `examples/skills/code-review/SKILL.md`
- `examples/skills/test-review/SKILL.md`
- `examples/skills/implementation-review/SKILL.md`
- `examples/skills/smoke-test/SKILL.md`

各 file 共通の改修:

1. `## Dispatch Rules` 見出しと配下の表 / 段落を全削除。
2. ファイル先頭の H1 直下 (または既存の概要段落の末尾) に 1 行 note を追加:
   > Dispatching and `invoke:` semantics follow `skills/belt-agent/SKILL.md`. This document covers only this skill's domain-specific concerns (voting, triage, fix strategy, verify).
3. `## Voting Protocol` 冒頭の "Activated when `config.iterations` > 1." を "Activated when this phase's `invoke.iterations` > 1." に書き換える。
4. その他 `config.*` を含む散発記述は、belt-agent SKILL.md に一本化される意味が消えるため削除、または `invoke.*` に言い換える (残す価値がある文脈のみ)。

保持するセクション (skill 固有の知識として価値があるもの):
- spec-review: Voting Protocol, Triage, Dialogue group / Selection group, Verify, Red Flags
- code-review: Scope Detection, /simplify Handling, code-review-impact Context, Voting, Triage, Verify, Red Flags
- test-review: Design Spec Resolution, Requirement Map, Voting, Triage, Fix Strategy by Category, Verify, Red Flags
- implementation-review: Related Design Doc Detection, Voting, Triage, Verify, Red Flags
- smoke-test: Output, Red Flags

### Task C — invoke.pipeline.with を使った parent args propagation (C-α)

**C-1. `examples/skills/feature-dev/pipeline.yml`**

4 つの `invoke.pipeline` phase に `with:` を追加:

| Phase | pipeline | with entries |
|---|---|---|
| spec-review | ../spec-review/pipeline.yml | iterations, codex, ui, swarm |
| plan-review | ../implementation-review/pipeline.yml | iterations, codex, ui, swarm |
| code-review | ../code-review/pipeline.yml | iterations, codex, swarm |
| test-review | ../test-review/pipeline.yml | iterations, codex, swarm |

形式 (1 phase の例):

```yaml
- id: spec-review
  description: "Multi-perspective spec review via spec-review sub-pipeline"
  invoke:
    pipeline: ../spec-review/pipeline.yml
    with:
      iterations: "args.iterations"
      codex: "args.codex"
      ui: "args.ui"
      swarm: "args.swarm"
  consumes:
    - design_doc
  validate: ./criteria/spec-review.md
  confirm: true
  max_retries: 3
```

**C-2. `examples/skills/debug-flow/pipeline.yml`**

3 つの `invoke.pipeline` phase に `with:` を追加:

| Phase | pipeline | with entries |
|---|---|---|
| fix-plan-review | ../implementation-review/pipeline.yml | iterations, codex, ui, swarm |
| code-review | ../code-review/pipeline.yml | iterations, codex, swarm |
| test-review | ../test-review/pipeline.yml | iterations, codex, swarm |

`ui` を持たない code-review / test-review sub-pipeline には `ui:` を渡さない (sub-pipeline の `args:` 宣言に存在しないため、渡すと `belt-agent init` が unknown arg で reject する)。

### Data flow (Task C)

```
$ belt-agent init feature-dev/pipeline.yml --arg iterations=5 --arg codex=true

  parent RunState.args = { iterations: 5, codex: true, ui: false, swarm: false, ... }

  … design → plan → execute 相当 phase 進行 …

  ▼ phase=spec-review
  belt-agent next → JSON:
      invoke.pipeline = "../spec-review/pipeline.yml"
      invoke.with = {
          iterations: "args.iterations",
          codex:      "args.codex",
          ui:         "args.ui",
          swarm:      "args.swarm"
      }                                     ← declared, raw template

  ▼ orchestrator (Claude) resolves parent args
  resolved_with = { iterations: 5, codex: true, ui: false, swarm: false }

  ▼ nested belt-agent init
  belt-agent init ../spec-review/pipeline.yml \
      --arg iterations=5 --arg codex=true --arg ui=false --arg swarm=false
```

### Error handling

| ケース | 挙動 |
|---|---|
| `"args.X"` の X が parent args に無い | orchestrator は `--arg X=...` を omit。sub-pipeline の declared default が適用される。 |
| sub-pipeline に宣言されていない key を with に書く | belt-agent init が unknown arg で reject。本 design では sub-pipeline 側の `args:` 宣言と整合する entry だけ渡すことで回避。 |
| value が literal (`false`, `3`, `"hello"`) | そのまま `--arg K=V` に翻訳。template resolution なし。 |

### Non-goals

- `expander` / `engine` 層での `with` 解決実装 (C-β)。protocol は orchestrator 責務のまま。
- `smoke-test` phase を `invoke.pipeline` 化すること (現状 `invoke.skill: /smoke-test` で設計通り)。
- legacy YAML key warning lint (BELT-32 follow-up #1、ユーザー判断でスキップ)。
- Plan doc Task 4.5 back-fill / workspace test-clippy debt (follow-up #3, #4、別 session)。

## Verification

1. `cargo run -p belt --quiet -- lint examples/skills/feature-dev/pipeline.yml` → clean
2. `cargo run -p belt --quiet -- lint examples/skills/debug-flow/pipeline.yml` → clean
3. `rg 'config\.(agents|iterations|ui_agent|codex|swarm|skills|reference)' examples/skills -g '*.md'` → 0 hit
4. `rg 'resolved_path: null' skills -g '*.md'` → 0 hit
5. `cargo run -p belt-agent --quiet -- init examples/skills/feature-dev/pipeline.yml --arg iterations=5 --arg codex=true` → exit 0
6. `cargo run -p belt-agent --quiet -- next` で spec-review phase の JSON に `invoke.pipeline.with.iterations == "args.iterations"` が raw template のまま出現
7. `cargo test -p belt-core -p belt-agent` → 既存 test suite pass
8. `cargo clippy --package belt-core --package belt --package belt-agent -- -D warnings` → clean (Rust 変更なしのため baseline と同値であれば OK)

## Commit strategy

3 commits (bisect 容易性のための logical split):

1. `docs(belt-agent): clarify artifact missing signal and with-template resolution` — Task A (skills/belt-agent/SKILL.md の 2 edit)
2. `docs(examples): delegate dispatch semantics to belt-agent SKILL.md` — Task B (5 leaf SKILL.md の表削除 + 参照 note)
3. `feat(examples): propagate parent args via invoke.pipeline.with in feature-dev/debug-flow` — Task C (2 pipeline.yml)

## Execution plan

subagent-driven-development で 3 subagent 並列:

- **SA-A (Task A)**: `skills/belt-agent/SKILL.md` に 2 edit。
- **SA-B (Task B)**: 5 leaf SKILL.md の Dispatch Rules 表削除 + 参照 note 挿入 + `config.iterations` → `invoke.iterations` 書き換え。
- **SA-C (Task C)**: `feature-dev/pipeline.yml` + `debug-flow/pipeline.yml` に `with:` 追加。

全 3 subagent 完了後、controller が Verification ステップ 1-8 を実行し、ステップ毎の commit を順次作成。
