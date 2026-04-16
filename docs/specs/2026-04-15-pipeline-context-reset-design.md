# feature-dev / bug-fix への Context Reset 対応

**Linear**: BELT-TBD (to be created)
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)
**Status**: Draft
**Date**: 2026-04-15

## Summary

既存の `plugins/feature-dev` (9 phase) / `plugins/bug-fix` (8 phase) pipeline に、**phase-scoped narrative artifact** を組み込む。user が任意の phase 境界で `/clear` しても narrative から context を復元できる状態にする。

2026-04-14 の context-neutral narrative artifact spec ([`docs/specs/2026-04-14-belt-context-neutral-narrative-artifact.md`](./2026-04-14-belt-context-neutral-narrative-artifact.md)) で belt-core / belt-agent 側に実装済みの機構（`{run_id}` template、`.belt/runs/{run_id}/notes/` directory、unprotected produces lint、`ArtifactRef`）を、**pipeline YAML 側で初めて actively 利用する**変更。本 spec の scope は **within-run のみ** で、cross-pipeline (feature-dev → bug-fix) 引き継ぎは別 follow-up とする。

## Background

### Problem

[`2026-04-14-belt-context-neutral-narrative-artifact.md`](./2026-04-14-belt-context-neutral-narrative-artifact.md) で context neutrality を実現する機構（`belt://` URI, `ArtifactRef::External`, `.belt/runs/{run_id}/notes/`, `{run_id}` template, unprotected produces lint）が belt-core / belt-agent に実装された。しかし現状の `feature-dev` / `bug-fix` pipeline YAML は domain artifact（`docs/features/*/design.md`, `docs/plans/*-rca-report.md` 等）のみを produce しており、**narrative note (phase-scoped decisions/concerns/directives/observations) を一切 produce していない**。

結果として:

1. `/clear` 後に後続 phase を進めると、LLM は domain artifact から断片的にしか context を復元できない（plan.md には何を決めたかあるが、なぜそう決めたかの narrative が落ちる）
2. BELT-24 context neutrality 原則が pipeline level で未充足のまま
3. 既存 `lint warn on unprotected produces` (commit aacbbca) の対象となる narrative produces が無いため、lint 自体が動作検証されていない

### Design Constraints

- **BELT-24 Context Neutrality**: pipeline は context 戦略（single / multi）に中立。別 context から呼んでも動作すること
- **SKILL.md Authoring Principle**: pipeline.yml / belt-agent SKILL.md が表現可能なものを SKILL.md で再掲しない
- **belt is content-neutral**: belt-core / belt-agent は note の frontmatter / body を parse しない。規約は SKILL 層責務
- **Claude Code runtime limit**: programmatic `/clear` 不可。user が手動で実施する前提

### Non-Goals

- `/clear` の自動化（Claude Code runtime 制約、対応不能）
- cross-pipeline 引き継ぎ（feature-dev → bug-fix、将来 spec）
- 軽量 phase（`test-scenarios`, `spec-review`, `fix-plan-review`, `integrate`）での narrative produce
- belt-core / belt-agent への新機能追加（既存機構のみで完結）
- note 内容の semantic validation を belt が担う拡張（SKILL 層責務のまま）
- Session-level narrative（active_tasks, recent_decisions 等）の吸収（skill 責務のまま）

## Goals

1. `feature-dev` / `bug-fix` の主要 phase で narrative note を **決定論的に (gate 強制)** produce する
2. 後続の narrative phase が **accumulating** で prior narrative を consume し、`/clear` 後の復元に耐える
3. note 規約を skill 層の convention として文書化する（belt は content 中立）
4. 既存 domain artifact / gate / validate を破壊しない additive な変更とする

## Design

### 1. Narrative Note 規約（SKILL 層 convention）

#### 1.1 Path

```
.belt/runs/{run_id}/notes/phase-{phase_id}.md
```

- belt-core の `{run_id}` template 展開（commit `2c4f603`）と `<run_dir>/notes/` 自動作成（commit `7e992f9`）を利用
- domain artifact（`docs/features/*/design.md` 等）とは別 directory で管理
- phase_id が複合語（`monkey-test`, `code-review` 等）の場合はそのまま hyphen 保持

#### 1.2 File 構造（D2: minimal frontmatter + 4 sections）

```markdown
---
phase: <phase_id>
run_id: <run_id>
---

## Decisions
<この phase で確定した設計判断 / 方針>

## Concerns
<未解決の懸念 / リスク / 下流で注意すべき事項>

## Directives
<次以降の phase への指示 / 前提条件>

## Observations
<事実記録 / 探索で判明した事項 / テスト結果など>
```

- frontmatter は `phase` / `run_id` の 2 field のみ。belt は parse しない
- 4 section (`## Decisions`, `## Concerns`, `## Directives`, `## Observations`) は全て必須。空でも heading のみ残すこと（下流 consumer が section 欠落で混乱しないため）
- `run_id` は `belt-agent step` / `belt-agent status` output の `run_id` 値を LLM が書き写す

### 2. feature-dev Pipeline 変更（9 phase → narrative 付与 6 phase）

#### 2.1 narrative-producing phase

| # | Phase | produces name | path |
|---|---|---|---|
| 1 | design | `design_notes` | `.belt/runs/{run_id}/notes/phase-design.md` |
| 4 | plan | `plan_notes` | `.belt/runs/{run_id}/notes/phase-plan.md` |
| 5 | execute | `execute_notes` | `.belt/runs/{run_id}/notes/phase-execute.md` |
| 6 | code-review | `code_review_notes` | `.belt/runs/{run_id}/notes/phase-code-review.md` |
| 7 | monkey-test (when e2e) | `monkey_test_notes` | `.belt/runs/{run_id}/notes/phase-monkey-test.md` |
| 8 | dogfood (when e2e) | `dogfood_notes` | `.belt/runs/{run_id}/notes/phase-dogfood.md` |

非 narrative phase（`test-scenarios`, `spec-review`, `integrate`）は note 非生成。

#### 2.2 consume 拡張（accumulating）

各 narrative phase は prior narrative を全て consume する:

| Phase | 追加 consume (narrative のみ、既存 domain consume は保持) |
|---|---|
| design | （なし、最上流） |
| plan | `design_notes` |
| execute | `design_notes`, `plan_notes` |
| code-review | `design_notes`, `plan_notes`, `execute_notes` |
| monkey-test | `design_notes`, `plan_notes`, `execute_notes`, `code_review_notes` |
| dogfood | `design_notes`, `plan_notes`, `execute_notes`, `code_review_notes`, `monkey_test_notes` |

#### 2.3 gate 強制（E1）

narrative-producing phase の `gate:` に `file_exists: .belt/runs/{run_id}/notes/phase-{id}.md` を追加。note 未提出で step 進行不可。

#### 2.4 YAML diff 例（design / plan）

```yaml
- id: design
  description: "Generate design document via interactive brainstorming"
  invoke:
    skill: /brainstorming
  produces:
    - name: design_doc
      path: "docs/features/*/design.md"
      description: "Design document with explored context and test perspectives"
    - name: design_notes                                   # 追加
      path: ".belt/runs/{run_id}/notes/phase-design.md"
      description: "Phase narrative: decisions, concerns, directives, observations"
  gate:
    - file_exists: "docs/features/*/design.md"
    - file_exists: ".belt/runs/{run_id}/notes/phase-design.md"   # 追加
  validate: ./criteria/design.md
  confirm: true
  max_retries: 3

- id: plan
  description: "Generate implementation plan from design and test strategy"
  invoke:
    skill: /writing-plans
  consumes:
    - design_doc
    - test_strategy
    - design_notes                                         # 追加
  produces:
    - name: plan_doc
      path: "docs/features/*/plan.md"
      description: "Task-level implementation plan (TDD)"
    - name: plan_notes                                     # 追加
      path: ".belt/runs/{run_id}/notes/phase-plan.md"
      description: "Phase narrative"
  gate:
    - file_exists: "docs/features/*/plan.md"
    - file_exists: ".belt/runs/{run_id}/notes/phase-plan.md"   # 追加
  validate: ./criteria/plan.md
  confirm: true
  max_retries: 3
```

`when: args.e2e` phase（monkey-test / dogfood）では phase 全体が skip 時に produces も評価されないため、`Artifact.when` 追加は不要。

### 3. bug-fix Pipeline 変更（8 phase → narrative 付与 6 phase）

#### 3.1 narrative-producing phase

| # | Phase | produces name | path |
|---|---|---|---|
| 1 | rca | `rca_notes` | `.belt/runs/{run_id}/notes/phase-rca.md` |
| 2 | fix-plan | `fix_plan_notes` | `.belt/runs/{run_id}/notes/phase-fix-plan.md` |
| 4 | execute | `execute_notes` | `.belt/runs/{run_id}/notes/phase-execute.md` |
| 5 | code-review | `code_review_notes` | `.belt/runs/{run_id}/notes/phase-code-review.md` |
| 6 | monkey-test (when e2e) | `monkey_test_notes` | `.belt/runs/{run_id}/notes/phase-monkey-test.md` |
| 7 | dogfood (when e2e) | `dogfood_notes` | `.belt/runs/{run_id}/notes/phase-dogfood.md` |

非 narrative phase（`fix-plan-review`, `integrate`）は note 非生成。

#### 3.2 consume 拡張（accumulating）

feature-dev と同じ構造:

| Phase | 追加 consume (narrative のみ) |
|---|---|
| rca | （なし、最上流） |
| fix-plan | `rca_notes` |
| execute | `rca_notes`, `fix_plan_notes` |
| code-review | `rca_notes`, `fix_plan_notes`, `execute_notes` |
| monkey-test | `rca_notes`, `fix_plan_notes`, `execute_notes`, `code_review_notes` |
| dogfood | 上記 + `monkey_test_notes` |

#### 3.3 gate 強制

feature-dev と同じく各 narrative phase で `file_exists` を追加。

### 4. criteria/ 更新

各 narrative phase の `validate:` が指す criteria file に narrative note の品質基準を追加する。

#### 4.1 追加章（全 narrative criteria で共通）

```markdown
### <N>. Narrative Note

- `.belt/runs/<run_id>/notes/phase-<phase_id>.md` が存在すること（gate で検出済みだが validate でも再確認）
- frontmatter に `phase`, `run_id` の 2 field が記載されていること
- 4 section (`## Decisions` / `## Concerns` / `## Directives` / `## Observations`) がすべて存在すること
- 各 section は空でも heading を保持すること（下流 consumer が section 欠落で混乱しないため）
- section 内容は `/clear` 後の LLM が当該 phase の判断を再構成できる最低限の情報を含むこと
  - Decisions: 何をなぜ決めたか
  - Concerns: 未解決の risk / 仮定
  - Directives: 次 phase が守るべき制約
  - Observations: 探索で得た事実（特に domain artifact に書ききれないもの）
```

#### 4.2 更新対象 file

**feature-dev plugin** (`plugins/feature-dev/skills/feature-dev/criteria/`):
- `design.md`
- `plan.md`
- `execute.md`（plugin 移行で per-plugin 配置に変更済み）
- `code-review.md`（同上）
- `monkey-test.md`
- `dogfood.md`

**bug-fix plugin** (`plugins/bug-fix/skills/bug-fix/criteria/`):
- `rca.md`
- `fix-plan.md`
- `execute.md`（per-plugin 配置）
- `code-review.md`（同上）
- `monkey-test.md`
- `dogfood.md`

plugin 移行により execute.md / code-review.md は各 plugin に複製されている。`crates/belt-core/tests/shared_criteria_parity.rs` が内容 drift を検出する。narrative 章は両 plugin の copy に同一内容を追加すること。

### 5. SKILL.md 更新

`feature-dev/SKILL.md` / `bug-fix/SKILL.md` に以下追加。

#### 5.1 新設 Narrative Notes セクション

SKILL.md Authoring Principle に従い、**pipeline.yml で既に宣言された内容は再掲しない**。以下 3 点のみ記載:

1. 「context reset 後の復元のため、指定 phase で narrative note を produce する」と明示
2. path / frontmatter / 4 section convention への pointer（`plugins/belt-agents/references/narrative-convention.md`）
3. `/clear` は user 判断（自動化不可）の旨を明示

例（feature-dev/SKILL.md への追加）:

```markdown
## Narrative Notes

以下 phase は `/clear` 後の context 復元のため narrative note を produce する (`.belt/runs/{run_id}/notes/phase-<id>.md`):

- design / plan / execute / code-review / monkey-test (`--e2e`) / dogfood (`--e2e`)

Note 規約は `plugins/belt-agents/references/narrative-convention.md` を参照。

`/clear` 自体は user 判断。重い phase 完了直後（例: design, execute 後）に context が膨れる場合の選択肢として利用可能。
```

#### 5.2 新設 references/narrative-convention.md（SSOT）

**配置**: `plugins/belt-agents/references/narrative-convention.md`。belt-agents plugin は cross-cutting な共通 reference の配置先（`audit-protocol.md`, `criteria-template.md`, `evidence-catalog.md` 等が既存）。

内容は §1「Narrative Note 規約」の内容を具体例付きで記載:
- path template（`.belt/runs/{run_id}/notes/phase-<id>.md`）
- frontmatter schema（phase, run_id）
- 4 section schema + 各 section に何を書くべきかの例
- 空 section の保持ルール
- run_id を `belt-agent step` JSON から写す方法

#### 5.3 Red Flags 追加

両 SKILL.md の Red Flags セクションに 1 行追加:

- "Never leave narrative 4 sections blank": gate は file_exists のみで空でも通過するが、下流 consume で復元不能になる

### 6. Error Handling

| Scenario | Behavior |
|---|---|
| narrative phase で note 未作成 → step 進行 | `file_exists` gate で fail、belt-agent が明示 error |
| note に 4 section が欠落 | gate 通過するが validate 段階で phase-auditor が検出（criteria 章に基づく audit） |
| `when: args.e2e` phase で e2e=false → note path が評価されない | phase skip で produces 未実行、downstream 非 narrative phase も同 note 非 consume のため影響なし |
| LLM が `run_id` を frontmatter に書き写し忘れ | belt は parse しないので gate 通過。criteria 章で audit 検出 |
| 既存 domain artifact gate は通るが narrative gate が通らない | step 全体 fail、retry 可能 |
| lint warn on unprotected produces | 本設計では全 narrative produces が gate 保護されるため発火しない |

### 7. Backward Compatibility

- **既存 run**: `.belt/runs/<old_run_id>/state.json` の `pipeline_file` が旧 pipeline.yml を指している場合、resume は旧 YAML で継続（belt 既存仕様）。新 YAML は新規 run から適用
- **既存 `docs/features/*` / `docs/plans/*` domain artifact**: 変更なし
- **既存 `criteria/execute.md`, `criteria/code-review.md`**: narrative 章を additive に追加するのみ。既存項目は保持
- **lint**: `feat(belt-core): lint warn on unprotected produces` (commit `aacbbca`) を本変更で実効発火させる（全 narrative produces を gate 保護して warn 0 を維持）

### 8. Testing Strategy

#### 8.1 belt-core integration test 更新

既存 `crates/belt-core/tests/feature_dev_refresh.rs` / `crates/belt-core/tests/bug_fix_refresh.rs` が pipeline shape を lock している。以下 assertion を追加:

- narrative-producing phase の `produces` に `{phase}_notes` entry があること、path が `.belt/runs/{run_id}/notes/phase-<id>.md` であること
- narrative-producing phase の `gate` に `file_exists: .belt/runs/{run_id}/notes/phase-<id>.md` があること
- 後続 narrative phase の `consumes` に prior narrative name が全て含まれること
- 非 narrative phase（test-scenarios / spec-review / fix-plan-review / integrate）は note 非 produce / 非 consume
- monkey-test / dogfood の narrative produces は phase の `when: args.e2e` により conditional skip されること

#### 8.2 Adversarial Probes（Verification Contract 準拠）

- `/clear` 擬似: phase N 完了直後に別 process で `belt-agent next` → consumes JSON に全 prior narrative notes の resolved_path が出現すること
- narrative note file 未作成で `belt-agent step` が fail すること
- e2e=false で monkey-test / dogfood の note path が評価されないこと（phase skip 経路）
- 既存 domain artifact gate と narrative gate の両方が満たされて初めて step 成功すること
- regate で narrative gate も再評価されること（既存 regate 仕様の継承確認）

#### 8.3 validate criteria audit 確認

各 narrative criteria の新章が phase-auditor で正しく評価されることを手動確認:
- note file 存在検出
- 4 section 欠落検出
- frontmatter 2 field 欠落検出

### 9. Impact / Effort 見積もり

| 変更対象 | ファイル数 | 行数目安 |
|---|---|---|
| pipeline.yml (feature-dev, bug-fix) | 2 | +40〜60 行 |
| criteria (各 plugin 内 6 + 6 = 12 files) | 12 | +60〜100 行 |
| SKILL.md (feature-dev, bug-fix) | 2 | +20〜30 行 |
| narrative-convention.md (belt-agents/references/ 新設) | 1 | ~80 行 |
| belt-core test (feature_dev_refresh.rs, bug_fix_refresh.rs) | 2 | +40〜60 行 |
| **合計** | **19** | **~260〜330 行** |

belt-core / belt-agent source への touch は**無し**（spec は既に merge 済み）。本変更は **純粋な YAML + skill convention レイヤの改修**。

### 10. Open Questions（実装前に確認したい点）

- (a) ~~`references/narrative-convention.md` 配置先~~ → **解決済み**: `plugins/belt-agents/references/narrative-convention.md` に配置（G1）
- (b) ~~共通 criteria~~ → **解決済み**: plugin 移行で execute.md / code-review.md は per-plugin 化。両方に同一内容を追加し `shared_criteria_parity.rs` で drift 検出
- (c) criteria audit で 4 section の **内容品質** までを semantic 評価の対象にするか（本 spec では「存在 + 復元可能な最低限の情報」止まりとする）

## Migration

- 既存 feature-dev / bug-fix run は旧 YAML で走り切り、新規 run から新 YAML が適用される（belt 既存動作）
- 既存 docs/features/* / docs/plans/* domain artifact は変更不要
- 新規 `references/narrative-convention.md` は new file のため migration 不要

## Future Work

以下は本 spec の scope 外。別 ticket として追跡する。

- **Cross-pipeline narrative**: feature-dev → bug-fix の narrative 引き継ぎ（`belt://latest/feature-dev/notes/phase-integrate.md`）。bug-fix の hard-require を避けるため optional consume 機構が必要で、belt-core 拡張が scope 膨張する
- **narrative convention の belt-core 化**: 4 section 規約を belt-core が semantic validation する拡張（content 中立原則との兼ね合い要検討）
- **PreCompact hook 連携**: 自動 snapshot trigger（Claude Code hook 側の変更、別 skill spec）
- **narrative に基づく `/continue` 自動化**: handover skill と本 narrative の統合
- **TUI 可視化**: `belt-tui` crate で narrative を browse

## References

- [BELT-20](https://linear.app/neko-neko/issue/BELT-20): master tracking
- [BELT-24](https://linear.app/neko-neko/issue/BELT-24): Context neutrality 原則
- [`docs/specs/2026-04-14-belt-context-neutral-narrative-artifact.md`](./2026-04-14-belt-context-neutral-narrative-artifact.md): 機構側 spec（本 spec の前提）
- [`docs/specs/2026-04-14-feature-dev-refresh-design.md`](./2026-04-14-feature-dev-refresh-design.md): feature-dev 9 phase 構成の根拠
- [`docs/specs/2026-04-15-debug-flow-refresh-design.md`](./2026-04-15-debug-flow-refresh-design.md): bug-fix (旧 debug-flow) 8 phase 構成の根拠
- `plugins/feature-dev/skills/feature-dev/pipeline.yml`: 改修対象
- `plugins/bug-fix/skills/bug-fix/pipeline.yml`: 改修対象
- `plugins/belt-agents/references/narrative-convention.md`: SSOT 新設先
- `crates/belt-core/src/gate.rs`: `file_exists` gate の実装（glob pattern match）
- `crates/belt-core/tests/feature_dev_refresh.rs`: feature-dev pipeline shape test（更新対象）
- `crates/belt-core/tests/bug_fix_refresh.rs`: bug-fix pipeline shape test（更新対象）
- `crates/belt-core/tests/shared_criteria_parity.rs`: criteria drift 検出（既存）
