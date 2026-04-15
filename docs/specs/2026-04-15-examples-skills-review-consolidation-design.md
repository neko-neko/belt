---
depends-on:
  - examples/skills/feature-dev/pipeline.yml
  - examples/skills/feature-dev/SKILL.md
  - examples/skills/feature-dev/criteria
  - examples/skills/bug-fix/pipeline.yml
  - examples/skills/bug-fix/SKILL.md
  - examples/skills/spec-review/SKILL.md
  - examples/skills/code-review/SKILL.md
  - examples/skills/test-review
  - examples/skills/implementation-review
---

# examples/skills Review Consolidation 設計

## 概要

`examples/skills/` 配下の review 系スキルから観点重複している 2 スキル (`test-review` / `implementation-review`) を削除し、`spec-review` を feature-dev の Phase 3 として組み込む。bug-fix (旧 debug-flow、commit 9af77cc でリネーム済み) の Phase 3 `fix-plan-review` は invoke 先を `/implementation-review` から `/spec-review` に切り替える。

Reviewer skill の最終形は **code-review + spec-review** の 2 種類に集約される。

## 動機

### 観点重複の解消

**test-review** (3 observations: `coverage` / `quality` / `design-alignment`):

- `coverage` と `quality` は code-review の `test` observation (7 observations 中の一つ) と実質重複
- 固有価値の `design-alignment` + `requirement_map` (design-spec → test file:line マッピング) は、test-strategy.md を spec-review に通すことで上流担保可能

**implementation-review** (4 observations: `clarity` / `feasibility` / `consistency` / `ui-spec`):

- `feasibility` / `consistency` / `ui-spec` は spec-review の同名 (または `ui-design` として近接) 観点と重複
- 固有の `clarity` (タスクが TDD 実行可能か) は spec-review の `consistency` + `feasibility` でほぼ吸収
- 対象ドキュメントが plan の場合、spec-review の grill-me triage (`design-judgment` / `requirements` の high/medium 発動) は低頻度にしか作動しない

### 最小セット原則 (A 方針)

`examples/skills/` は feature-dev / bug-fix を動かすのに必要なスキルだけ置く方針。孤立した未参照スキル (`test-review` / `spec-review`) は削除 or 組み込み判断が必要。

### feature-dev のレビュー配置不足

現行 feature-dev の phase 構成 (8 phases):

```
design → test-scenarios → plan → execute → code-review → monkey-test → dogfood → integrate
```

Phase 2 `test-scenarios` の output (`test-strategy.md` / `scenarios.yml`) は Phase 3 `plan` へ consumes されるが、その間にレビュー gate がなく、誤った戦略がそのまま plan に伝播するリスクがある。

## スコープ

### In Scope

- `examples/skills/test-review/` 丸ごと削除
- `examples/skills/implementation-review/` 丸ごと削除
- feature-dev の 8→9 phase 化 (Phase 3 `spec-review` 追加)
- `examples/skills/feature-dev/criteria/spec-review.md` 新規作成
- `examples/skills/feature-dev/pipeline.yml` 更新 (spec-review phase 追加)
- `examples/skills/feature-dev/SKILL.md` 更新 (description / phase rules)
- `examples/skills/bug-fix/pipeline.yml` 更新 (Phase 3 invoke 切替)
- `examples/skills/bug-fix/SKILL.md` 更新 (Phase 3 記述 + Red Flags)

### Out of Scope

- `examples/skills/spec-review/SKILL.md` 本体の修正 (汎用性を保つ)
- `examples/skills/code-review/` スキルの観点追加
- belt-core / belt-agent / その他 crate 側の変更
- feature-dev / bug-fix 以外の examples/skills 配下スキル (code-review / monkey-test / test-scenarios / spec-review) の修正

## 設計詳細

### feature-dev Phase 構成 (9 phases)

| # | id | 変更 | produces | consumes (hard) |
|---|---|---|---|---|
| 1 | design | 据置 | design_doc | — |
| 2 | test-scenarios | 据置 | test_strategy, scenarios (when e2e) | design_doc |
| **3** | **spec-review** | **新規** | — | test_strategy |
| 4 | plan | 据置 | plan_doc | design_doc, test_strategy |
| 5 | execute | 据置 | — | design_doc, plan_doc |
| 6 | code-review | 据置 | — | design_doc, plan_doc |
| 7 | monkey-test (when e2e) | 据置 | monkey_test_report, monkey_test_results | design_doc, test_strategy, scenarios, plan_doc |
| 8 | dogfood (when e2e) | 据置 | dogfood_report | design_doc, test_strategy, scenarios, monkey_test_report, monkey_test_results, plan_doc |
| 9 | integrate | 据置 | — | design_doc, plan_doc |

phase id は全て不変。配列順序への `spec-review` 挿入のみ。

### Phase 3 spec-review 定義 (pipeline.yml)

```yaml
  - id: spec-review
    description: "Review test strategy (and scenarios if --e2e) via spec-review"
    invoke:
      skill: /spec-review
      args:
        codex: "args.codex"
    consumes:
      - test_strategy
    validate: ./criteria/spec-review.md
    regate: [test-scenarios]
    confirm: true
    max_retries: 3
```

**consumes 設計判断**: `scenarios` artifact は e2e 時のみ produce される。belt-core の `Artifact.when` は produces のみで動作するため (`consumes` 側の when は未対応)、hard consume には含めない。scenarios.yml の存在時扱いは SKILL.md の Phase 3 rule で「同じ出力ディレクトリに存在すればレビュー対象に含める」と規定し、spec-review スキル内で file 実在確認させる。

**gate 省略**: spec-review は既存 .md を in-place 書き換えるため `file_exists` gate は無意味。`validate` criteria が唯一の gate となる。

**regate**: `test-scenarios`。fix で test-strategy.md が書き換わった場合、test-scenarios phase の validate を再検証する (既存パターン `code-review.regate: [execute]` に倣う)。

**max_retries: 3**: 他レビュー系フェーズと同値で fix-loop の暴走を防ぐ。

### criteria/spec-review.md (新規)

既存 criteria (`code-review.md` / `plan.md` 等) のスタイルに揃え、以下を必須項目として宣言する:

1. `test-strategy.md` の必須セクション (ISTQB / ISO 25010 カバレッジ) が維持されていること
2. spec-review findings の triage が完了していること (grill-me group / selection group 両方処理済み)
3. user 承認済み findings のみ反映されていること (無断反映は NG)
4. 未承認 findings は skip 記録に残されていること
5. `scenarios.yml` が存在する場合 (args.e2e)、レビュー対象に含まれていること

構造は `## Must / ## Must Not / ## Evidence` を踏襲。

### feature-dev/SKILL.md 変更

- frontmatter `description` の "8 phases" → "9 phases"、phase list に `spec-review` を追加
- `## Phase-Specific Invocation Rules` に以下を追加 (Phase 2 と Phase 4 の間):

```
### Phase 3: spec-review
- **INVOKE**: Skill tool `/spec-review` with `codex` passed through.
- Targets `test-strategy.md`. If `scenarios.yml` exists (args.e2e), include in review scope.
- grill-me dialogue for requirements/design-judgment findings; direct selection for others.
- regate: test-scenarios; fix loop capped at max_retries=3.
```

- 後続 Phase 4〜9 (旧 3〜8) は **番号のみ繰下げ**。セクション本文は変更なし

### bug-fix Phase 3 変更 (pipeline.yml)

```diff
   - id: fix-plan-review
-    description: "Plan review via implementation-review"
+    description: "Plan review via spec-review"
     invoke:
-      skill: /implementation-review
+      skill: /spec-review
       args:
         codex: "args.codex"
     consumes:
       - fix_plan_doc
     validate: ./criteria/fix-plan-review.md
     confirm: true
     max_retries: 3
```

phase id (`fix-plan-review`) と validate criteria (`fix-plan-review.md`) は据え置き。run state 互換性を保ち criteria 変更コストを避ける。

### bug-fix/SKILL.md 変更

Phase 3 セクションを以下に置換:

```
### Phase 3: fix-plan-review
- **INVOKE**: Skill tool `/spec-review` with `codex` passed through.
- No supplement required; the skill is self-contained.
- Note: spec-review を fix-plan レビューに流用する。`design-judgment` 観点の grill-me は原則発動しない (設計判断は rca / fix-plan で決定済みのため)。発動した場合は上流 (rca / fix-plan) の見直しサインとして扱う。
```

Red Flags の記述:

```diff
- **Never filter or omit review findings**: `/code-review`, `/implementation-review` の triage は user 責務.
+ **Never filter or omit review findings**: `/code-review`, `/spec-review` の triage は user 責務.
```

References セクションは変更なし (bug-fix/references/ 配下に implementation-review-supplement は存在しないため)。

## 影響評価

### 観点の損失

| 失う観点 | 代替策 | 損失度 |
|---|---|---|
| test-review / `requirement_map` (design section → test file:line 表) | spec-review が test-strategy.md 時点で requirements + consistency を検証。粒度は (design → test-strategy section) に緩和 | 中 |
| test-review / `coverage` | code-review / `test` observation | 低 (機能的に重複) |
| test-review / `quality` | code-review / `test` observation | 低 (機能的に重複) |
| implementation-review / `clarity` | spec-review / `consistency` + `feasibility` | 低 (実質吸収) |
| implementation-review / `feasibility` | spec-review / `feasibility` | なし (同一) |
| implementation-review / `consistency` | spec-review / `consistency` | なし (同一) |
| implementation-review / `ui-spec` | spec-review / `ui-design` | 低 (ほぼ同義) |

### grill-me 挙動差

- spec-review の grill-me triage は `requirements` / `design-judgment` の high/medium で発動
- bug-fix で `fix_plan_doc` を spec-review に通す場合、`design-judgment` は rca / fix-plan phase で決定済みのため発動頻度は低い
- 発動時は bug-fix/SKILL.md の note に従い、上流 (rca / fix-plan) の見直しサインとして扱う。reviewer agent の severity 判定が適切であれば実害は小さい

### 非互換

- feature-dev の走行中 run がある場合、新規 run からのみ Phase 3 spec-review が適用される
- 既存 phase id (`design` / `test-scenarios` / `plan` / ...) は変わらず、run state (phase id ベース) の前方互換性は保たれる
- bug-fix の phase id (`fix-plan-review`) と criteria filename も不変のため、bug-fix 側は完全な in-place 切替

### 削除ディレクトリの影響

- `examples/skills/test-review/`: どの pipeline.yml / SKILL.md からも未参照。影響なし
- `examples/skills/implementation-review/`: bug-fix/fix-plan-review からのみ参照。本設計の bug-fix 変更と同時に削除すれば影響なし

## Open Questions

なし。
