# feature-dev belt pipeline 移植設計

## 概要

feature-dev スキル (品質ゲート付き 10 フェーズ開発オーケストレーター) を belt pipeline として `examples/skills/feature-dev/` に移植する。belt の dogfooding として、フェーズ遷移・gate・regate・conditional skip・confirm を belt-agent CLI に委譲し、オーケストレーターの context overhead を **76% 削減** する。

## 動機

- **Dogfooding**: belt が複雑な実世界パイプラインを駆動できることを実証する
- **Context 削減**: 現行 SKILL.md (35.7KB) をフェーズ遷移ロジック除去で ~8.5KB に縮小
- **Audit 品質向上**: audit を独立フェーズ化し、work フェーズとの context 分離を構造的に保証
- **Example 充実**: 既存 examples (linear-refresh, smoke-test) に加え、大規模パイプラインの模範例を提供

## スコープ

### In Scope

- feature-dev 本体の belt pipeline 化 (pipeline.yml + SKILL.md + references/)
- done-criteria の移植 (セマンティック名への変更含む)
- audit-protocol, evidence-plan-protocol, fix-dispatch-strategy の reference 化
- pipeline.yml の config passthrough 設計 (子スキル自体の修正は不要。オーケストレーターが config 値を invoke 時に渡す)

### Out of Scope

- Resume / Handover 機構 (belt state.json で代替、将来対応)
- Linear sync (--linear フラグ)
- 子スキル (code-review, spec-review 等) 自体の belt pipeline 化 (Phase C で段階的に対応)

## 分解戦略: B → C

**Phase B (本設計)**: feature-dev の pipeline.yml を作成。子スキルは `config: { skill: "/code-review" }` で既存スキル invoke に委ねる。belt が制御するのはフェーズ遷移・gate・regate・max_retries のみ。

**Phase C (将来)**: 利用頻度の高い子スキル (例: code-review) を後から belt pipeline 化し、`uses:` で差し替え。

## ディレクトリ構造

```
examples/skills/feature-dev/
├── pipeline.yml
├── belt.toml
├── SKILL.md
└── references/
    ├── done-criteria/
    │   ├── design.md
    │   ├── spec-review.md
    │   ├── plan.md
    │   ├── plan-review.md
    │   ├── execute.md
    │   ├── doc-audit.md          # 新規作成
    │   ├── smoke-test.md
    │   ├── code-review.md
    │   └── test-review.md
    ├── audit-protocol.md
    ├── evidence-plan-protocol.md
    └── fix-dispatch-strategy.md
```

## Pipeline 設計

### Args

```yaml
args:
  e2e:        { type: bool, default: false }    # Phase 9 (test-review) 有効化
  smoke:      { type: bool, default: false }    # Phase 7 (smoke-test) 有効化
  doc:        { type: bool, default: false }    # Phase 6 (doc-audit) 有効化
  codex:      { type: bool, default: false }    # config passthrough → 子スキル
  ui:         { type: bool, default: false }    # config passthrough → 子スキル
  iterations: { type: number, default: 3 }      # config passthrough → 子スキル
  swarm:      { type: bool, default: false }    # config passthrough → 子スキル
```

- `when:` 制御: `e2e`, `smoke`, `doc` の 3 つ
- Config passthrough: `codex`, `ui`, `iterations`, `swarm`

### フェーズ一覧 (19 フェーズ、最小構成 13)

| # | ID | 種別 | when | gate | confirm | regate | max_retries |
|---|-----|------|------|------|---------|--------|-------------|
| 1 | design | work | - | file_exists | - | - | - |
| 2 | design-audit | audit | - | has_output | yes | - | 3 |
| 3 | spec-review | work | - | - | - | - | - |
| 4 | spec-review-audit | audit | - | has_output | yes | - | 3 |
| 5 | plan | work | - | file_exists ×2 | - | - | - |
| 6 | plan-audit | audit | - | has_output | yes | - | 3 |
| 7 | plan-review | work | - | - | - | - | - |
| 8 | plan-review-audit | audit | - | has_output | yes | - | 3 |
| 9 | execute | work | - | cmd | - | - | 3 |
| 10 | execute-audit | audit | - | has_output | yes | - | 3 |
| 11 | doc-audit | work | args.doc | - | - | - | - |
| 12 | doc-audit-audit | audit | args.doc | has_output | yes | - | 3 |
| 13 | smoke-test | work | args.smoke | file_exists | - | - | - |
| 14 | smoke-test-audit | audit | args.smoke | has_output | yes | - | 3 |
| 15 | code-review | work | - | - | - | - | - |
| 16 | code-review-audit | audit | - | has_output | yes | [execute, smoke-test, doc-audit] | 3 |
| 17 | test-review | work | args.e2e | - | - | - | - |
| 18 | test-review-audit | audit | args.e2e | has_output | yes | [execute] | 3 |
| 19 | integrate | lite-audit | - | - | yes | - | - |

### フェーズ種別の設計パターン

**Work フェーズ**: config.skill で子スキルを指定。gate は成果物の存在確認 (file_exists) またはテスト実行 (cmd)。review 系 work フェーズは gate なし (成果物が会話内のため)。

**Audit フェーズ**: `config: { audit: required, criteria: "<phase-id>" }` で done-criteria を参照。`has_output: true` で verdict.json の存在を検証。`validate:` で LLM 判断を要求。`confirm: true` で明示的確認を強制。`max_retries: 3` で再試行制御。

**Lite-audit フェーズ (integrate のみ)**: 独立 audit フェーズ不要。`validate:` + `confirm: true` で直接判断。

### Regate トポロジー

```
code-review-audit ──regate──→ execute (gate 再実行: cmd)
                  ──regate──→ smoke-test (skipped → auto-pass)
                  ──regate──→ doc-audit (skipped → auto-pass)

test-review-audit ──regate──→ execute (gate 再実行: cmd)
```

regate は gate のみ再実行 (validate は対象外)。skipped phase は belt が自動 PASS する (belt-agent main.rs:422-428 で実装済み)。

### pipeline.yml 全文

```yaml
name: feature-dev
description: "Quality-gated development orchestrator"
version: 1
args:
  e2e:        { type: bool, default: false }
  smoke:      { type: bool, default: false }
  doc:        { type: bool, default: false }
  codex:      { type: bool, default: false }
  ui:         { type: bool, default: false }
  iterations: { type: number, default: 3 }
  swarm:      { type: bool, default: false }

phases:
  # ─── Design ───
  - id: design
    description: "Create design spec via brainstorming"
    config:
      skill: "/brainstorming"
      swarm: "args.swarm"
    gate:
      - file_exists: "docs/plans/*-design.md"

  - id: design-audit
    description: "Audit design spec against done-criteria"
    config:
      audit: required
      criteria: "design"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/design.md pass"
    confirm: true
    max_retries: 3

  # ─── Spec Review ───
  - id: spec-review
    description: "4-perspective spec review"
    config:
      skill: "/spec-review"
      codex: "args.codex"
      iterations: "args.iterations"
      swarm: "args.swarm"

  - id: spec-review-audit
    description: "Audit spec review completion"
    config:
      audit: required
      criteria: "spec-review"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/spec-review.md pass"
    confirm: true
    max_retries: 3

  # ─── Plan ───
  - id: plan
    description: "Create implementation plan and test cases"
    config:
      skill: "/writing-plans"
    gate:
      - file_exists: "docs/plans/*-plan.md"
      - file_exists: "docs/plans/*-test-cases.md"

  - id: plan-audit
    description: "Audit implementation plan"
    config:
      audit: required
      criteria: "plan"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/plan.md pass"
    confirm: true
    max_retries: 3

  # ─── Plan Review ───
  - id: plan-review
    description: "3-perspective implementation plan review"
    config:
      skill: "/implementation-review"
      codex: "args.codex"
      iterations: "args.iterations"
      ui: "args.ui"

  - id: plan-review-audit
    description: "Audit plan review completion"
    config:
      audit: required
      criteria: "plan-review"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/plan-review.md pass"
    confirm: true
    max_retries: 3

  # ─── Execute ───
  - id: execute
    description: "TDD implementation following the plan"
    config:
      skill: "/subagent-driven-development"
    gate:
      - cmd: "make test"
    max_retries: 3

  - id: execute-audit
    description: "Audit implementation against plan"
    config:
      audit: required
      criteria: "execute"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/execute.md pass"
    confirm: true
    max_retries: 3

  # ─── Doc Audit (conditional) ───
  - id: doc-audit
    description: "4-layer document audit"
    when: "args.doc"
    config:
      skill: "/doc-audit"

  - id: doc-audit-audit
    description: "Audit doc audit completion"
    when: "args.doc"
    config:
      audit: required
      criteria: "doc-audit"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/doc-audit.md pass"
    confirm: true
    max_retries: 3

  # ─── Smoke Test (conditional) ───
  - id: smoke-test
    description: "Local smoke test with browser verification"
    when: "args.smoke"
    config:
      skill: "/smoke-test"
    gate:
      - file_exists: "smoke-test-report.md"

  - id: smoke-test-audit
    description: "Audit smoke test results"
    when: "args.smoke"
    config:
      audit: required
      criteria: "smoke-test"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/smoke-test.md pass"
    confirm: true
    max_retries: 3

  # ─── Code Review ───
  - id: code-review
    description: "7-perspective code review"
    config:
      skill: "/code-review"
      codex: "args.codex"
      iterations: "args.iterations"
      swarm: "args.swarm"

  - id: code-review-audit
    description: "Audit code review and verify regression"
    config:
      audit: required
      criteria: "code-review"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/code-review.md pass"
    confirm: true
    max_retries: 3
    regate: [execute, smoke-test, doc-audit]

  # ─── Test Review (conditional) ───
  - id: test-review
    description: "3-perspective test review"
    when: "args.e2e"
    config:
      skill: "/test-review"
      codex: "args.codex"
      iterations: "args.iterations"

  - id: test-review-audit
    description: "Audit test review and verify regression"
    when: "args.e2e"
    config:
      audit: required
      criteria: "test-review"
    gate:
      - has_output: true
    validate:
      - "All criteria in references/done-criteria/test-review.md pass"
    confirm: true
    max_retries: 3
    regate: [execute]

  # ─── Integrate ───
  - id: integrate
    description: "Merge, PR, or branch management"
    config:
      skill: "/worktrunk"
    validate:
      - "Integration method chosen and executed"
      - "All pre-merge checks pass"
    confirm: true
```

## SKILL.md 設計

### 役割

belt がフェーズ遷移を管理するため、SKILL.md は以下のみ記述:

1. **belt-agent 駆動ループ**: `init → next → [dispatch] → verify → regate → step --confirm → next`
2. **フェーズ別 dispatch ルール**: config.skill に基づくスキル invoke 方法
3. **Audit dispatch ルール**: `config.audit == "required"` 時の phase-auditor agent 起動手順
4. **Red Flags**: 絶対禁止事項 (design スキップ禁止、review 結果フィルタ禁止等)

### 推定サイズ

~4KB (現行 35.7KB から **89% 削減**)

### 記述しない内容

- フェーズ順序・遷移ロジック (pipeline.yml が SSOT)
- gate/validate/confirm の定義 (pipeline.yml)
- conditional フェーズのスキップ条件 (when: が SSOT)
- regate トポロジー (pipeline.yml)
- max_retries 制御 (pipeline.yml)

## References 設計

### done-criteria/ (8 ファイル + 1 新規)

元の `feature-dev/done-criteria/` から移植。変更点:

| 変更 | 理由 |
|------|------|
| ファイル名: `phase-N-xxx.md` → `xxx.md` | セマンティック名。フェーズ順序変更への耐性 |
| Criteria ID: `D1-01` → `DESIGN-01` | 番号排除。同上 |
| `doc-audit.md` 新規作成 | 元の feature-dev に Phase 6 の done-criteria がなかった |
| 内容構造は維持 | severity, verify_type, pass_condition 等 |

### audit-protocol.md

- verdict.json の形式定義
- phase-auditor agent dispatch 手順 (prompt template)
- PASS/FAIL 判定基準
- max_retries 到達時の cumulative diagnosis 形式
- Evidence Plan 生成トリガー (design-audit 完了後)

元: `feature-dev/references/audit-gate-protocol.md` (10KB) を belt 向けに簡略化

### evidence-plan-protocol.md

- Evidence Plan の生成タイミング (design-audit 完了後)
- 再評価タイミング (plan-review-audit 完了後、設計 hash 変更時)
- 収集対象の定義
- output_dir への書き込みルール
- execute 以降のフェーズへの収集指示注入方法

元: feature-dev SKILL.md の Evidence Plan セクションを抽出

### fix-dispatch-strategy.md

| Work フェーズ | FAIL 時 Executor | 戦略 |
|-------------|-----------------|------|
| design | Orchestrator | 調査記録再読み、再スキャン |
| spec-review | Orchestrator | 設計書直接編集 |
| plan | Orchestrator | 計画書直接編集 |
| plan-review | Orchestrator | 計画書直接編集 |
| execute | feature-implementer agent | TDD タスク再実行 |
| doc-audit | Orchestrator / feature-implementer | depends_on → Edit, content → doc-check |
| smoke-test | feature-implementer agent | バグ修正 |
| code-review | feature-implementer agent | レビュー指摘修正 |
| test-review | feature-implementer agent | テストコード修正 |

元: feature-dev SKILL.md の Fix Dispatch セクションを抽出

### belt.toml

```toml
pipeline_file = "pipeline.yml"
```

## Context Window 分析

### 1 セッションあたりのオーケストレーター overhead

| シナリオ | 現行 | Belt | 削減率 |
|---------|------|------|--------|
| Work フェーズ | 36KB | 8.5KB | **-76%** |
| Audit フェーズ (最大) | 50KB | 20KB | **-60%** |
| Audit フェーズ (最小) | 42KB | 14KB | **-67%** |

子スキルのコンテキストは両モデルで同一。上記はオーケストレーター overhead のみ。

### セッション分離時の優位性

フェーズ単位でセッションを分ける場合:
- 現行: 毎回 35.7KB の SKILL.md リロード
- Belt: base 8.5KB で開始可能

## 検証結果

pipeline.yml を `/tmp/belt-feature-dev-test/` で検証済み:

| テスト項目 | 結果 |
|-----------|------|
| `belt lint` 静的検証 | PASS |
| `belt-agent init` (19 フェーズ, 7 args) | PASS |
| gate: file_exists / has_output / cmd | PASS |
| confirm ガード (--confirm なし → 拒否) | PASS |
| validate / config passthrough | PASS |
| conditional skip (when: args.doc/smoke/e2e) | PASS — 6 フェーズ正常スキップ |
| regate (skipped phase auto-pass) | PASS |
| パイプライン完走 (completed: 13, skipped: 6, total: 19) | PASS |

## 設計判断ログ

| 判断 | 選択 | 理由 |
|------|------|------|
| 分解戦略 | B (top-down) → C (段階的) | 最速で dogfooding 価値。子スキルは後から belt 化可能 |
| Audit 表現 | 明示的 audit フェーズ (パターン A) | context 分離を構造保証。セッション分割に最適 |
| Pipeline / SKILL.md 分担 | Balanced (Approach 2) | done-criteria は md 保守、pipeline は構造制御 |
| Args 戦略 | when: (3) + config passthrough (4) | belt 制御と子スキル制御の明確な分離 |
| done-criteria 命名 | セマンティック名 (番号排除) | フェーズ順序変更への保守性 |
| Audit gate | has_output: true | glob パス不要、output_dir に verdict 書き込み |
| Regate 対象 | execute, smoke-test, doc-audit | skipped phase は belt auto-pass (実装済み) |
| Integrate | audit: lite (独立 audit フェーズなし) | validate + confirm で直接判断 |
| Resume/Handover | スコープアウト | belt state.json で代替、将来対応 |
| Linear sync | スコープアウト | 本設計の目的外 |
