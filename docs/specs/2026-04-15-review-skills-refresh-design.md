# Review Skills Refresh (code / spec / test / implementation-review) + grill-me Integration

**Linear**: BELT-TBD (to be created)
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)
**Status**: Draft
**Date**: 2026-04-15

## Summary

4 つの review skill（`/code-review`, `/spec-review`, `/test-review`, `/implementation-review`）を同一構造に刷新する。並列 multi-agent dispatch と N-way voting を廃止し、各 skill は **単一 reviewer subagent** に集約する。既存の 17 + 1 個の個別 agent 定義ファイルは、4 つの統合 reviewer agent ファイルに章立てとして merge する。args は `codex` のみに最小化する。`/spec-review` に限り、review 後の dialogue を `/grill-me` エッセンス（unlimited rounds、recommended answer 必須、codebase exploration、one-at-a-time、decision-tree dependency）で強化する。

これにより、feature-dev refresh（2026-04-15 `fa04895` merged）で顕在化した矛盾 —— 「`/code-review` が internal default `iterations: 3` を持ち、外から `--iterations 1` を渡さない限り常に 3-way voting になる」 —— を根本対応する。

## Background

### Problem

現行 4 skill は共通して以下の複雑性を抱える:

1. **N-way voting の default 3**: `args.iterations: { type: number, default: 3 }` を全 skill が持ち、feature-dev 側の Red Flag「Never pass `--iterations` to /code-review」は default 3 を打ち消せない。feature-dev からの呼び出しで必ず 3-way voting が走る矛盾
2. **並列 multi-agent dispatch の重装備**: code-review は 6 agents × 3 iterations = 18 dispatch。親 context への findings 集約、voting ロジック（semantic similarity、base selection、majority threshold）、dedup、codex findings の別扱い、simplify findings の非対称扱い、が orchestrator に集中
3. **`swarm` / `ui` args の decorative 化**: 各 skill に `swarm: bool` があるが invoke 側の使い方が統一されていない。`ui` も spec/impl のみに存在し非対称
4. **agent ファイル 18 個の maintenance コスト**: `.claude/agents/` に各観点ごとのファイルが散在し、観点追加・変更時に pipeline.yml と agent ファイル両方を触る必要がある
5. **simplify の非対称**: `/simplify` skill は free-text output で、voting 対象外、dedup 後に直接統合される。構造的 outlier
6. **impl-review と spec-review の dialogue 仕様不統一**: spec-review は `requirements/design-judgment` の high/medium、impl-review は `impl-clarity/impl-feasibility` と範囲が異なり、いずれも max 3 rounds、recommended answer なし、codebase exploration なし

### Design Constraints（確定済み憲法）

- **BELT-21: CLI is deterministic, skill is protocol** — belt-agent は JSON 事実報告のみ、protocol 層は skill 責務
- **BELT-32: Invoker + Artifact first-class** — `invoke.agents` と `produces`/`consumes` の既存機構を維持、Artifact 体系を活用
- **Context Neutrality (BELT-24)**: skill は multi-context / single-context で中立に動作する
- **Tiny by Constraint**: agent 定義の複雑性を pipeline.yml と SKILL.md で抑える、冗長な args を作らない
- **File-based data flow (BELT-30)**: phase 間は `.belt/runs/*/review/findings.json` で接続

### `/grill-me` の本質

`~/.claude/skills/grill-me/SKILL.md` より:

> Interview me relentlessly about every aspect of this plan until we reach a shared understanding. Walk down each branch of the design tree, resolving dependencies between decisions one-by-one. For each question, provide your recommended answer.
>
> Ask the questions one at a time.
>
> If a question can be answered by exploring the codebase, explore the codebase instead.

これを spec-review の dialogue group（設計判断の曖昧さを user と詰める場面）に注入する。

## Goals

1. 4 skill の pipeline.yml を同型に統一（単一 reviewer subagent、`codex` のみ args、review → fix の 2 phase）
2. 既存 17 + 1 agent 定義ファイルを 4 つの統合 reviewer agent に集約
3. `/spec-review` の review 後 dialogue を grill-me 化
4. `/implementation-review` の既存 dialogue group を廃止（全 findings を selection）
5. `/code-review` の `/simplify` 独立呼び出しを廃止し、code-reviewer agent の prompt に吸収
6. SKILL.md から N-way voting / swarm / iterations の記述を全削除
7. feature-dev refresh の Red Flag 矛盾を解消

## Non-Goals

- **agent の review 品質向上そのもの** は対象外（既存観点の内容は可能な限り保持）
- **独立 `/simplify` skill の廃止** は対象外（user が直接呼ぶ機能としては残す、code-review 内呼び出しのみ統合）
- **新しい観点 (observation) の追加** は対象外
- **feature-dev 側の invoke 記述変更** は followup の minor 調整範囲（iterations flag が存在しなくなるので pass する記述自体を削除するのみ）
- **swarm 再導入機構**（将来必要なら別 skill として `/code-review-deep` 等を新設）
- **BELT-TBD Linear 登録の詳細**（parent: BELT-20、本 spec 着手前に作成）

## Design

### Core Principle

4 skill すべてが同一の骨格に従う:

```
┌──────────────────────────────────────────────────┐
│ Orchestrator (親 context, light)                 │
│   - dispatch 1 reviewer subagent                 │
│   - parse findings.json                          │
│   - triage (user selection)                      │
│   - [spec-review only] grill-me dialogue         │
│   - dispatch fix (subagent or self)              │
└──────────────────────────────────────────────────┘
                    │
                    │ invoke 1 agent
                    ▼
┌──────────────────────────────────────────────────┐
│ Reviewer Subagent (独立 context, heavy)           │
│   - N observations as prompt sections            │
│   - read target files                            │
│   - detect diff / design doc (as needed)         │
│   - produce findings.json                        │
└──────────────────────────────────────────────────┘
```

### Common Args (全 skill)

```yaml
args:
  codex: { type: bool, default: false }
```

廃止: `iterations`, `swarm`, `ui`

### Common Pipeline Structure (全 skill)

```yaml
phases:
  - id: review
    description: "Multi-perspective <target> review"
    invoke:
      agents:
        - <skill>-reviewer
      args:
        codex: "args.codex"
    produces:
      - name: review_findings
        path: ".belt/runs/*/review/findings.json"
        description: "<observation list>"
    confirm: true

  - id: fix
    description: "Fix accepted findings"
    consumes:
      - review_findings
    gate:
      - has_output: true
```

### Skill Specifics

#### 2.1 `/code-review` (7 observations)

**pipeline.yml**:

```yaml
name: code-review
version: 1
args:
  codex: { type: bool, default: false }

phases:
  - id: review
    description: "Multi-perspective code review (7 observations)"
    invoke:
      agents:
        - code-reviewer
      args:
        codex: "args.codex"
    produces:
      - name: review_findings
        path: ".belt/runs/*/review/findings.json"
        description: "Deduplicated findings across 7 observations (quality, security, performance, test, ai-antipattern, impact, simplification)"
    confirm: true

  - id: fix
    description: "Fix accepted findings"
    consumes:
      - review_findings
    gate:
      - has_output: true
```

**Agent**: `code-reviewer` prompt に以下を章立てとして含める:

| # | Observation | 由来 agent |
|---|-------------|-----------|
| 1 | Quality | code-review-quality |
| 2 | Security | code-review-security |
| 3 | Performance | code-review-performance |
| 4 | Test | code-review-test |
| 5 | AI-antipattern | code-review-ai-antipattern |
| 6 | Impact | code-review-impact |
| 7 | Simplification | `/simplify` skill の core 観点（reuse, quality, efficiency）を agent prompt に吸収 |

**Triage**: 全 findings を selection group（番号で accept/reject）。dialogue なし。

**SKILL.md の変更点**:
- `Voting Protocol` 節削除
- `/simplify Handling` 節削除（agent 内統合）
- Red Flags から「Wait for all parallel agents」「Announce iterations」「Ignore consensus vote results」削除
- `argument-hint`: `[--codex]`

#### 2.2 `/spec-review` (5 observations + grill-me dialogue)

**pipeline.yml**:

```yaml
name: spec-review
version: 1
args:
  codex: { type: bool, default: false }

phases:
  - id: review
    description: "Multi-perspective spec review (5 observations) + grill-me dialogue"
    invoke:
      agents:
        - spec-reviewer
      args:
        codex: "args.codex"
    produces:
      - name: review_findings
        path: ".belt/runs/*/review/findings.json"
        description: "Deduplicated findings across 5 observations (requirements, design-judgment, feasibility, consistency, ui-design)"
    confirm: true

  - id: fix
    description: "Fix accepted findings"
    consumes:
      - review_findings
    gate:
      - has_output: true
```

**Agent**: `spec-reviewer` prompt に以下を章立てとして含める:

| # | Observation | 由来 agent |
|---|-------------|-----------|
| 1 | Requirements | spec-review-requirements |
| 2 | Design judgment | spec-review-design-judgment |
| 3 | Feasibility | spec-review-feasibility |
| 4 | Consistency | spec-review-consistency |
| 5 | UI design | spec-review-ui-design（常時内包、spec に UI 記述なければ empty findings） |

**Triage**:

- **Grill-me Dialogue group**: `requirements` / `design-judgment` の severity **high/medium**
  - One question at a time（並列提示禁止、順次 resolve）
  - Orchestrator が recommended answer を必ず付与
  - codebase で答えられる question は user に聞かず自分で explore (Read/Grep)
  - Rounds unlimited（user が「十分」「納得した」等で stop、明示要求なしに中断しない）
  - Decision tree dependency: 先行 finding の決定が後続 finding の提案に影響する場合、orchestrator が順序調整
  - Dialogue の結果として finding の suggestion は revise される（user 承認後 fix phase に引き継ぐ）
- **Selection group**: `feasibility` / `consistency` / `ui-design` / severity low / codex findings
  - 直接 accept/reject（番号選択）

**SKILL.md の変更点**:
- `Voting Protocol` 節削除
- `Triage` 節を「Grill-me Dialogue + Selection」に書き換え
- Red Flags から「Wait for all parallel agents」「Announce iterations」「Ignore consensus vote results」削除
- `argument-hint`: `[--codex]`

#### 2.3 `/test-review` (3 observations + requirement map)

**pipeline.yml**:

```yaml
name: test-review
version: 1
args:
  codex: { type: bool, default: false }

phases:
  - id: review
    description: "Multi-perspective test review (3 observations) + requirement map"
    invoke:
      agents:
        - test-reviewer
      args:
        codex: "args.codex"
    produces:
      - name: review_findings
        path: ".belt/runs/*/review/findings.json"
        description: "Deduplicated findings (coverage, quality, design-alignment) + informational requirement_map"
    confirm: true

  - id: fix
    description: "Fix accepted findings"
    consumes:
      - review_findings
    gate:
      - has_output: true
```

**Agent**: `test-reviewer` prompt に以下を章立てとして含める:

| # | Observation | 由来 agent |
|---|-------------|-----------|
| 1 | Coverage | test-review-coverage |
| 2 | Quality | test-review-quality |
| 3 | Design alignment | test-review-design-alignment |

**Design Spec Resolution**: subagent 内で実施（prompt に手順を記述）。以下の優先順で path 解決:

1. Output directory の `*-design.md`
2. `docs/plans/*-design.md` の日付 prefix 一致
3. 見つからなければ design-alignment 観点を reduced coverage で実行（warning を findings に含める）

**Requirement Map**: findings.json と並ぶ informational artifact として出力、voting 対象外（元々非対称扱い）。

**Triage**: 全 findings を selection group（番号で accept/reject）。dialogue なし。

**SKILL.md の変更点**:
- `Voting Protocol` 節削除
- `Design Spec Resolution` は「agent 内で実施」と簡潔化
- Red Flags から「Wait for all parallel agents」「Announce iterations」「Ignore consensus vote results」削除
- `argument-hint`: `[--codex]`

#### 2.4 `/implementation-review` (4 observations)

**pipeline.yml**:

```yaml
name: implementation-review
version: 1
args:
  codex: { type: bool, default: false }

phases:
  - id: review
    description: "Multi-perspective plan review (4 observations)"
    invoke:
      agents:
        - implementation-reviewer
      args:
        codex: "args.codex"
    produces:
      - name: review_findings
        path: ".belt/runs/*/review/findings.json"
        description: "Deduplicated findings across 4 observations (clarity, feasibility, consistency, ui-spec)"
    confirm: true

  - id: fix
    description: "Fix accepted findings"
    consumes:
      - review_findings
    gate:
      - has_output: true
```

**Agent**: `implementation-reviewer` prompt に以下を章立てとして含める:

| # | Observation | 由来 agent |
|---|-------------|-----------|
| 1 | Clarity | implementation-review-clarity |
| 2 | Feasibility | implementation-review-feasibility |
| 3 | Consistency | implementation-review-consistency |
| 4 | UI spec | implementation-review-ui-spec（常時内包、plan に UI task なければ empty findings） |

**Related Design Doc Detection**: subagent 内で実施（prompt に手順を記述）。plan filename の日付 prefix → `docs/plans/<prefix>*-design.md` を resolve。

**Triage**: 全 findings を selection group（番号で accept/reject）。dialogue group は **廃止**。

**SKILL.md の変更点**:
- `Voting Protocol` 節削除
- `Dialogue group` 節削除（全 selection）
- `Related Design Doc Detection` は「agent 内で実施」と簡潔化
- Red Flags から「Wait for all parallel agents」「Announce iterations」「Ignore consensus vote results」削除
- `argument-hint`: `[--codex]`

### Agent File Migration

#### 削除対象 (18 ファイル, `.claude/agents/`)

- `code-review-quality.md`
- `code-review-security.md`
- `code-review-performance.md`
- `code-review-test.md`
- `code-review-ai-antipattern.md`
- `code-review-impact.md`
- `spec-review-requirements.md`
- `spec-review-design-judgment.md`
- `spec-review-feasibility.md`
- `spec-review-consistency.md`
- `spec-review-ui-design.md`
- `test-review-coverage.md`
- `test-review-quality.md`
- `test-review-design-alignment.md`
- `implementation-review-clarity.md`
- `implementation-review-feasibility.md`
- `implementation-review-consistency.md`
- `implementation-review-ui-spec.md`

#### 新規作成 (4 ファイル, `.claude/agents/`)

各統合 agent は以下の構造を持つ:

```markdown
---
name: <skill>-reviewer
description: <one-line summary>
---

# <Skill> Review Agent

## Mission
...

## Input Resolution
（diff / target file / design doc path の解決手順）

## Observations

### Observation 1: <name>
（旧 agent ファイルから転記した観点説明 + check items + output rules）

### Observation 2: <name>
...

## Output Format
（共通 findings.json schema）

## Guardrails
...
```

- `code-reviewer.md`: 7 observations
- `spec-reviewer.md`: 5 observations
- `test-reviewer.md`: 3 observations + requirement map output
- `implementation-reviewer.md`: 4 observations

### Findings JSON Schema (共通)

現行 schema をそのまま踏襲。voting metadata フィールド（`iteration_count`, `consensus_votes` 等）は存在しないはずだが、存在するなら削除:

```json
{
  "findings": [
    {
      "id": "<uuid>",
      "observation": "<observation name e.g. 'quality', 'requirements'>",
      "severity": "high | medium | low",
      "file": "<path>",         // code/test-review
      "section": "<heading>",    // spec/impl-review
      "description": "...",
      "suggestion": "...",
      "source": "agent | codex"
    }
  ],
  "requirement_map": [           // test-review only
    { "number": 1, "requirement": "...", "source": "...", "test": "...", "gap": "..." }
  ]
}
```

### Orchestrator Protocol（SKILL.md 再構成の共通パターン）

各 SKILL.md は以下の節に再構成する:

1. **Scope Detection / Target Resolution** — 何を review するかの決定（diff / target file / design doc）
2. **Invoke** — 単一 reviewer agent の dispatch と context 準備
3. **Triage** — findings 提示形式、accept/reject ルール、spec-review のみ grill-me dialogue rules を追記
4. **Fix** — fix phase の実行方法
5. **Verify** — 修正後の確認手順（linter, tests, markdown check など現行のもの）
6. **Red Flags** — 禁止事項・必須事項（voting 関連は削除）

### Grill-me Dialogue Protocol (spec-review 専用詳細)

Triage phase の順序:

1. Reviewer subagent が返した findings.json を parse
2. `requirements` と `design-judgment` の severity high/medium を抽出 → grill group
3. それ以外を selection group
4. Grill group が空でなければ、findings を **decision-tree の依存順に並べ替え** (先行決定が後続提案に影響するもの先)
5. **One-at-a-time loop**:
   ```
   for each finding in grill_group:
     a. finding 内容を user に提示
     b. codebase で answer 可能か判定 → 可能なら Read/Grep で自己解決、suggestion を revise
     c. 不可能なら recommended answer 付きで user に質問
     d. user response を受けて suggestion を revise
     e. user が「OK」「accept」等で明示承認 or「reject」するまで c-d を繰り返す
        （unlimited rounds）
     f. user が「もう次へ」「十分」等で stop した場合、現在の revised suggestion を finding に保存
   ```
6. Grill 完了後、selection group を numbered list で提示 → accept/reject
7. Fix phase に accepted findings を引き継ぐ

### Impact on Other Skills / Code

#### feature-dev skill

- `examples/skills/feature-dev/pipeline.yml` で `/code-review` / `/spec-review` / `/test-review` / `/implementation-review` を invoke する箇所で `iterations` / `swarm` / `ui` 引数を渡していれば削除（feature-dev refresh では渡していない想定だが要確認）
- Red Flag「Never pass `--iterations` to /code-review」は削除可能
- SKILL.md の argument-hint で `[--e2e] [--smoke] [--doc] [--codex] [--iterations N] [--swarm] [--linear]` となっていれば、`[--iterations N]` と `[--swarm]` を削除

#### belt-core tests

- `crates/belt-core/tests/feature_dev_refresh.rs` が 4 review skill の args 形状を直接 assert していれば更新（`iterations`/`swarm` expectation 削除）
- 当該テストが feature-dev の invoke フィールドをスキャンして `iterations`/`swarm` の presence を assert していれば削除

#### 既存 `.belt/runs/*/review/findings.json`

- 旧 voting metadata を含む findings.json は、新 fix phase から consume すると schema 不整合になる可能性
- 軽減策: 旧 findings を consume する code path がないことを確認（実行中の run がなければ新規 run から自動的に新 schema）

## Migration Plan

実装は別途 plan doc (`docs/plans/2026-04-15-review-skills-refresh-plan.md`) で `/superpowers:writing-plans` により詳細化するが、ざっくり phase:

1. **Phase A: 統合 agent ファイル作成** (4 skill × 1 agent = 4 ファイル)
   - 旧 18 ファイルの内容を転記・merge
   - findings.json schema と output rules を統一
2. **Phase B: pipeline.yml 刷新** (4 skill)
   - args 削減、agents 配列 1 要素化
3. **Phase C: SKILL.md 刷新** (4 skill)
   - voting 節削除、triage 再構成、spec-review のみ grill-me dialogue 追記
4. **Phase D: 旧 agent ファイル削除** (18 ファイル)
5. **Phase E: feature-dev / belt-core tests の調整**
   - iterations/swarm/ui 引数削除
   - feature_dev_refresh.rs の expectation 更新
6. **Phase F: Dogfood**
   - 4 skill を各自自己 review（`/spec-review` で本 spec review、`/code-review` で本 plan 実装後 review 等）

## Risks

| Risk | Severity | Mitigation |
|------|----------|-----------|
| 単一 agent prompt の長大化で review 品質低下 | medium | 旧 agent ファイルの観点 content は保持、重複 guardrail は 1 箇所に統合して冗長性排除。初回 dogfood で品質 regression を検出 |
| N-way voting の信頼性を失う | medium | `codex` opt-in で独立 provider 検証を維持。false-positive が課題化したら将来 `/code-review-deep` 別 skill 新設 |
| grill-me の unlimited rounds で user が疲弊 | low | 「もう十分」等で明示 stop できる rule を SKILL.md に明記。recommended answer を必ず提示するので最短 1 round で済むケースも多い |
| 6 parallel context 相当の情報密度喪失 | medium | 単一 subagent は独立 context なので prompt 長は増やせる。重い読み込み（Read/Grep）は subagent 側で完結 |
| 旧 findings.json の schema 不整合 | low | 移行期間中の既存 run は fix phase で consume する必要があれば手動対応。通常は新規 run から自然に新 schema |
| feature-dev の既存 iterations/swarm/ui 参照見落とし | medium | Phase E で feature-dev/pipeline.yml と SKILL.md を grep で全件確認、belt-core test が args 形状を検査していればそこで検出 |

## Open Questions

- **Observation count の将来拡張**: 新観点追加時、単一 agent の prompt 章立てに追記する運用で良いか（現状 OK の想定）
- **codex の invoke 経路**: 既存 `companion.mjs` 経由で `args.codex` 受け取る flow は維持か更新か（plan 段階で確認）
- **Findings の dedup ロジック**: 旧 voting は semantic similarity で dedup していたが、単一 agent でも同観点内の重複 findings は agent prompt 内で self-dedup する指示にするか、orchestrator が post-process するか（plan 段階で確認）

## Approval

この design で `/superpowers:writing-plans` による実装計画作成へ進む。
