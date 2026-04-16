# Review Skills Subagent Boundary + Observation Decomposition

**Linear**: BELT-TBD (to be created)
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)
**Status**: Draft
**Date**: 2026-04-16

## Summary

2 つの方針を同一 spec で遂行する:

1. **belt-core から agent 概念を完全除去** — `Invoker` enum を `Skill` / `Pipeline` の 2 variant に縮小し、`Agent` / `Agents` / `IterationsSpec` を削除。`pipeline.yml` から `agent:` / `agents:` / `iterations:` が露出しなくなる。副次的に `code-review` / `spec-review` plugin の `pipeline.yml` を削除し、両 review skill は単体 skill (orchestrator) + 観点別 subagent 群の構造に刷新する。

2. **Review 観点の subagent 分解 (hybrid)** — 現行 1 agent × N observation 構造を、独立系観点は並列 subagent、横断系観点は統合 subagent に分解する。`--codex` は追加の independent observation 扱いで、既存 `/codex:rescue` skill を review-specific prompt で invoke する (新 `/codex:review` は作らない)。Cross-agent dedup は severity-first + observation priority (actionability 順) で parent が実行し、Codex finding は dedup 対象外 (別 source 保持)。

これにより「review skill = subagent 境界」という philosophy が各 reviewer agent 層で成立し、belt-core のモデルからは agent 概念が完全に消える。

## Background

### Problem

2026-04-15 `fa04895` で完了した review-skills-refresh により 4 review skill は「1 reviewer subagent × N observation 章立て」構造に統合済み。しかし 2 点の design pressure が残る:

1. **belt-core surface に agent 概念が露出** — `pipeline.yml` の `invoke.agents: [<agent-name>]` は agent dispatch を belt の state machine model に露出させる。ユーザーの設計哲学「subagent 概念は skill 層に閉じ込めて制御」と整合しない
2. **単一 reviewer agent の long system prompt による attention 分散** — code-reviewer は 7 observation 分の (checklist + policy + filtering) を 1 prompt に持ち、観点ごとの attention が希薄化する構造的リスク。観点専用 prompt で specialization を深めたい

また、現状 `code-review:code-reviewer` / `spec-review:spec-reviewer` の 2 つだけが `invoke.agents` を使い、他 5 plugin の 17 phase 全てが `invoke.skill` で済んでいる。belt-core から agent variant を削除する移行コストは限定的 (pipeline 2 本 + agent 定義 2 本)。

並列 N-way voting (`iterations: N`) は 2026-04-15 refresh で明示的に廃止済み (`project_review_skills_refresh_2026_04_15.md`)。IterationsSpec 型も未使用となり、削除可能。

### Design Constraints (確定済み憲法)

- **BELT-21: CLI is deterministic, skill is protocol** — belt-agent は JSON 事実報告のみ、protocol 層は skill 責務
- **BELT-32: Invoker + Artifact first-class** — `produces` / `consumes` の Artifact 体系は維持。**partial revert** として `Agent` / `Agents` variant のみ削除、Invoker 自体の first-class 化は保持
- **Context Neutrality (BELT-24)**: skill は multi-context / single-context で中立
- **Tiny by Constraint**: belt-core の model は必要最小限
- **File-based data flow (BELT-30)**: phase 間は `.belt/runs/*/review/findings.json` で接続

### 用語

- **Parent** / **parent skill**: `/code-review` のような user entry point の SKILL.md。`context: fork` を持たず、orchestration を main context で実行
- **Observation agent** / **reviewer agent**: `.claude/agents/*.md` で定義される subagent。parent が Task tool で dispatch
- **Independent observation**: 検査項目が self-contained で他観点の finding を参照不要 (例: Security, Test)
- **Cross-cutting observation**: 他観点と overlap があり、同一 context での self-dedup が価値を持つ (例: Quality, Impact)

## Goals

1. belt-core `Invoker` enum から `Agent` / `Agents` variant を削除、`IterationsSpec` 型を削除
2. `code-review` / `spec-review` plugin の `pipeline.yml` と `belt.toml` を削除し、両 skill を単体 skill + observation subagent 群に刷新
3. 単一 reviewer agent を観点クラスタに分解 (code-review: 4 agents、spec-review: 3 agents)
4. Parent parallel dispatch + merged findings + cross-agent dedup を parent 責務として明示
5. `--codex` は独立 observation として `/codex:rescue` を追加 invoke する構造に揃える
6. feature-dev / bug-fix pipeline からの `invoke.skill: /code-review:code-review` invariant を維持 (呼び出し側 breaking を最小化)
7. `belt-agent/SKILL.md` の invoke 変換表を 4 variant → 2 variant (skill / pipeline) に更新

## Non-Goals

- **review 品質向上そのもの** は対象外 (既存観点の checklist / policy 文面は原則移植、必要最小限の整合性調整のみ)
- **軽量版 review skill の新設** は out-of-scope (別 brainstorm)
- **feature-dev / bug-fix pipeline の構造刷新** は out-of-scope (criteria/code-review.md の文言調整のみ)
- **BELT-32 Invoker / Artifact first-class 化の完全 revert** は対象外 (`Skill` / `Pipeline` variant と Artifact 体系は保持)
- **N-way voting / iterations の再導入** は明示的に放棄 (将来の必要性は project_review_skills_refresh_2026_04_15 で既に negate 済み)
- **Codex 専用 reviewer agent の新設** は不要 (既存 `/codex:rescue` skill を使う)
- **observation 観点そのものの追加・削減** は対象外 (code-review = 7、spec-review = 5 を維持)

## Design

### Core Principle

```
┌─────────────────────────────────────────────────────────────────┐
│ Parent SKILL.md (/code-review)  — main context, dispatcher      │
│   1. Scope Detection                                            │
│   2. Parallel Dispatch (Task tool)                              │
│      ├─ Independent obs × N (agents/*-reviewer.md)              │
│      ├─ Cross-cutting obs × 1 (agents/cross-cutting-reviewer.md)│
│      └─ Codex (if --codex) → /codex:rescue skill invoke         │
│   3. Merge findings-*.json → cross-agent dedup                  │
│   4. Triage (user selection)                                    │
│   5. Fix apply + Verify                                         │
└─────────────────────────────────────────────────────────────────┘
```

4 つの原則:

1. **Pipeline は skill または sub-pipeline のみ invoke できる** — belt-core の Invoker から agent 概念を排除
2. **Review skill の parent は main-context dispatcher** (`context: fork` なし) — triage と user dialogue を main context で実行するため
3. **Review 実行 = subagent (各 observation agent)** — skill = subagent 境界の philosophy は各 reviewer agent 層で成立
4. **観点分類による並列度制御** — independent は並列 subagent で specialization、cross-cutting は統合 subagent で self-dedup を保持

### Observation Classification

#### code-review

| 種別 | 観点 | agent file | 並列 |
|---|---|---|---|
| Independent | Security | `agents/security-reviewer.md` | yes |
| Independent | Test | `agents/test-reviewer.md` | yes |
| Independent | AI-antipattern | `agents/ai-antipattern-reviewer.md` | yes |
| Cross-cutting | Quality + Impact + Performance + Simplification | `agents/cross-cutting-reviewer.md` | yes (並列に 1 agent) |
| Optional | Codex | 既存 `/codex:rescue` skill invoke | yes, `--codex` 時のみ |

Cross-cutting を 1 agent に統合する根拠:
- **Quality (DRY) ↔ Simplification (reuse opportunity)** は同一現象の別表現
- **Quality (naming consistency) ↔ Impact (caller integrity)** は pattern compliance の帰結と実害
- **Performance (N+1) ↔ Simplification (unnecessary computation)** は non-functional 改善と duplication 除去
- 同一 context で見ると self-dedup が機能し、4 観点 × 類似箇所の report を 1 finding に consolidation できる

#### spec-review

| 種別 | 観点 | agent file | 並列 |
|---|---|---|---|
| Independent | Feasibility | `agents/feasibility-reviewer.md` | yes |
| Independent | UI design | `agents/ui-design-reviewer.md` | yes (UI 無しなら 0 finding で early-exit) |
| Cross-cutting | Requirements + Design-judgment + Consistency | `agents/cross-cutting-spec-reviewer.md` | yes |
| Optional | Codex | 既存 `/codex:rescue` skill invoke | yes, `--codex` 時のみ |

Cross-cutting 統合根拠:
- **Requirements (implicit assumption) ↔ Consistency (codebase alignment)** は spec の前提と codebase の entry point の整合確認
- **Design-judgment (alternatives evaluation) ↔ Consistency (Impact Analysis completeness)** は decision rationale と影響範囲の同一事象
- **Requirements (vague phrasing) ↔ Design-judgment (rationale)** は spec の抽象度問題

### Parent SKILL.md の責務

Parent `/code-review` SKILL.md (main-context で実行、`context: fork` なし):

1. **Scope Detection**
   - branch != main → `git diff main...HEAD`
   - staged changes あり → `git diff --staged`
   - どちらでもない → "no diff detected" で exit

2. **Impact Observation Context** (code-review のみ)
   - design doc (`*-design.md`) が run output directory にあれば Impact Analysis 節を cross-cutting-reviewer に追加 context として渡す

3. **Parallel Dispatch**
   - Task tool で observation agents を並列 dispatch
   - 各 agent への prompt に `findings-{observation}.json` の write path を指定
   - `--codex` 時、`/codex:rescue` を review-specific prompt で invoke (後述)

4. **Findings Merge + Cross-agent Dedup**
   - `.belt/runs/{run_id}/review/findings-*.json` を全 read
   - Dedup ロジック (下記「Dedup rule」)
   - `findings.json` に合算、20 件上限 + truncate note 維持

5. **Triage** (現行仕様を維持)
   - code-review: numbered list sorted by severity desc → user pick by number
   - spec-review: grill-me group (requirements/design-judgment の high/medium) + selection group (残り) に partition

6. **Fix apply + Verify**
   - Parent が Edit tool で適用
   - Linter / test 実行 (現行 `code-review/SKILL.md` の Verify 節を踏襲)

### Observation Agent body

各 agent の body は**現行 `code-reviewer.md` / `spec-reviewer.md` の該当 observation 節 + 共通 Filtering rules を移植 (transcription)** して構成する。新規に書き下ろさない。

- `security-reviewer.md` ← 現 Observation 2 (Security) 節 + 共通 Filtering (ただし cross-observation self-dedup 項は削除 — 他 agent の finding を知らないため機能しない)
- `test-reviewer.md` ← Observation 4 (Test)
- `ai-antipattern-reviewer.md` ← Observation 5 (AI-antipattern)
- `cross-cutting-reviewer.md` ← Observation 1 (Quality) + 3 (Performance) + 6 (Impact) + 7 (Simplification) + 共通 Filtering (self-dedup は**内部 4 観点間で保持**)

各 agent の Output Format は `.belt/runs/{run_id}/review/findings-{observation}.json` に書き出し、agent 固有 observation label を設定。Integrated agent は複数 observation label を同一ファイルに出す:

```json
// findings-security.json
{
  "observation": "security",
  "findings": [ { "id": "...", "severity": "...", ... } ]
}

// findings-cross-cutting.json
{
  "observations": ["quality", "performance", "impact", "simplification"],
  "findings": [ { "id": "...", "observation": "quality", ... } ]
}

// merged findings.json (parent 出力)
{
  "findings": [ ... ]  // 20 件上限、observation field で区別
}
```

### `--codex` 統合

- 親 skill が `--codex` を検出した場合、他 observation agent と並列に `/codex:rescue` skill を invoke
- Invoke 時 prompt に (a) 対象 diff/spec、(b) findings.json の format spec、(c) 出力先 path (`findings-codex.json`) を渡す
- 出力 findings は `source: "codex"` tag を持つ
- **新 `/codex:review` skill は作成しない** — 既存 `/codex:rescue` で足る (memory `project_review_skills_refresh_2026_04_15` で既決済の review-context 用途)

### Dedup Rule (Cross-agent)

Parent の merge 段階で同一 issue と判定された finding 群を 1 件に集約する。

#### 軸 A: Severity-first + observation priority tie-break

1. 同一 issue 候補の中から最大 severity の finding を保持対象とする
2. 同 severity 内では observation priority (軸 B) を tie-break に使う

#### 軸 B: Observation priority (actionability 順)

**code-review**: `Security > Impact > Quality > Test > AI-antipattern > Performance > Simplification`

**spec-review**: `Feasibility > Requirements > Design-judgment > Consistency > UI-design`

根拠: 集約先の fix suggestion が最も具体的・明確な観点を優先 (user が "これどう直す?" と問うた際に最も actionable な observation に集約する)。

#### 軸 C: Codex finding の扱い

Codex finding は **dedup 対象外**。他観点と同一 issue を指摘しても統合せず別 finding として保持し、`source: "codex"` tag を維持する。

根拠: Codex は別 provider による adversarial second opinion が目的。通常観点に集約すると「別 signal source」という価値が消える。

### 同一 issue の判定

LLM (parent) が `description` / `file` / `line` / `section` を参照して同一性判定する。厳密な rule は置かない (LLM judgment に委ねる、ただし「file + line が同一で description に重複語彙があれば同一候補」は guideline として SKILL.md に記載)。

### belt-core の変更

#### `crates/belt-core/src/model.rs`

```rust
// 変更前
pub enum Invoker {
    Skill { skill: String, args: HashMap<String, serde_json::Value> },
    Agent { agent: String, args: HashMap<String, serde_json::Value> },
    Agents { agents: Vec<String>, iterations: IterationsSpec, args: HashMap<String, serde_json::Value> },
    Pipeline { pipeline: String, with: HashMap<String, serde_json::Value> },
}

// 変更後
pub enum Invoker {
    Skill { skill: String, args: HashMap<String, serde_json::Value> },
    Pipeline { pipeline: String, with: HashMap<String, serde_json::Value> },
}

// 削除: IterationsSpec enum 全体
```

#### `crates/belt-core/src/lint.rs`

以下の追加 lint rule:
- `invoke:` 直下に `agent` / `agents` / `iterations` キーがある場合、`InvalidPipeline` error として診断
- メッセージ: "`invoke.agent` and `invoke.agents` are no longer supported. Use `invoke.skill` with a skill that forks a subagent."

#### `crates/belt-core/src/{engine,view,expander}.rs`

- `Invoker::Agent` / `Invoker::Agents` への match arm を削除
- `expander.rs` の parent-merge / substitute 処理で Agent/Agents 分岐を除去
- `view.rs` の status JSON 生成で Agent/Agents 出力分岐を除去

#### `crates/belt-core/tests/` + fixtures

- Invoker parsing unit test を skill/pipeline のみに更新
- `fixtures/*.yml` 内に `invoke.agents` を使っているものを (本 spec の実装時点で) 全撤去
- 新規 test case: `invoke.agents` を含む YAML を parse するとパース error (untagged enum の variant 不一致) になることを確認
- 新規 lint test: `invoke.agent:` などのキーが含まれる pipeline.yml を lint で reject

### Plugin 構造の変更

#### `plugins/code-review/`

**追加**:
- `agents/security-reviewer.md`
- `agents/test-reviewer.md`
- `agents/ai-antipattern-reviewer.md`
- `agents/cross-cutting-reviewer.md`

**削除**:
- `agents/code-reviewer.md`
- `skills/code-review/pipeline.yml`
- `skills/code-review/belt.toml`

**変更**:
- `skills/code-review/SKILL.md` を parent dispatcher として書き換え (parallel dispatch + merge + triage + verify)

#### `plugins/spec-review/`

**追加**:
- `agents/feasibility-reviewer.md`
- `agents/ui-design-reviewer.md`
- `agents/cross-cutting-spec-reviewer.md`

**削除**:
- `agents/spec-reviewer.md`
- `skills/spec-review/pipeline.yml`
- `skills/spec-review/belt.toml`

**変更**:
- `skills/spec-review/SKILL.md` を parent dispatcher として書き換え (parallel dispatch + merge + grill-me + selection + verify)

### `belt-agent/SKILL.md` の更新

"Reading `phase.invoke`" 変換表を 2 variant に:

| Variant | Shape | Orchestrator action |
|---|---|---|
| `skill` | `{ skill: "/name", args: { ... } }` | Invoke the slash-command skill, passing `invoke.args`. |
| `pipeline` | `{ pipeline: "./path.yml", with: { ... } }` | Initialise a nested `belt-agent` run. Treat as black-box until `completed`. |

"Well-known Config Keys" 節で obsolete 記述 (`config.agents` 等) を更新。

### 呼び出し側 (feature-dev / bug-fix) の扱い

`feature-dev/pipeline.yml` の `code-review` phase:

```yaml
- id: code-review
  invoke:
    skill: /code-review:code-review        # 変更なし
    args:
      codex: "args.codex"
  consumes: [...]                          # 変更なし
  produces:
    - name: code_review_findings
      path: ".belt/runs/{run_id}/review/findings.json"     # 追加
      description: "Merged cross-agent review findings"
    - name: code_review_notes
      path: ".belt/runs/{run_id}/notes/phase-code-review.md"
  validate: ./criteria/code-review.md      # 文言更新
  regate: [execute]                        # 変更なし
  confirm: true
```

`criteria/code-review.md` (feature-dev / bug-fix 両方) に追記する検証項目:
- Findings.json の triage が完了し user selection が記録されていること
- Selected findings が実装に反映されていること (git diff で確認)
- Linter / test が pass していること
- Phase narrative note が書かれていること

Spec-review phase 側も同様に `spec-review/criteria/*.md` を反映させる (feature-dev の `spec-review` phase、bug-fix の `fix-plan-review` phase)。

### Data Flow

```
User が /code-review を実行 (or feature-dev pipeline が review phase で invoke)
    │
    ▼
Parent SKILL.md (main context)
    │
    ├── Scope Detection
    │
    ├── Parallel dispatch (Task tool)
    │     ├─ Task(subagent_type: security-reviewer, ...)
    │     ├─ Task(subagent_type: test-reviewer, ...)
    │     ├─ Task(subagent_type: ai-antipattern-reviewer, ...)
    │     ├─ Task(subagent_type: cross-cutting-reviewer, ...)
    │     └─ Skill(/codex:rescue)    [--codex 時のみ]
    │          ↓ 各 subagent/skill が findings-*.json を write
    │
    ├── Merge: findings-*.json 読み込み
    ├── Cross-agent Dedup: severity-first + observation priority
    │     (Codex finding は dedup 対象外、保持)
    ├── Write: findings.json (20 件上限)
    │
    ├── Triage: numbered list → user selection
    │
    ├── Fix apply (parent が Edit tool で適用)
    ├── Verify (lint / test)
    │
    ▼
(呼び出し元への復帰 — skill summary として findings summary + apply 結果)
```

## Breaking Changes

1. **`invoke.agent` / `invoke.agents` / `iterations` を含む pipeline.yml は lint error**
   - 影響 pipeline: `code-review/pipeline.yml`, `spec-review/pipeline.yml` の 2 本のみ (削除対象なので同 commit で解消)

2. **BELT-32 の `Invoker::Agent` / `Invoker::Agents` / `IterationsSpec` の partial revert**
   - Invoker first-class 化自体は継続、Skill / Pipeline variant は保持

3. **`agents/code-reviewer.md` / `agents/spec-reviewer.md` の削除**
   - Task tool で `subagent_type: code-reviewer` を直接指定している外部コードがあれば破壊的変更
   - 2026-04-16 時点では該当なし (memory `project_belt_agent_ownership_2026_04_15` で belt-specific reviewer agents の所在を確認済み)

4. **`.belt/runs/*/run-state.json` の JSON 互換性**
   - 進行中の run で `invoke.agents` variant が persist されていた場合、新 Invoker enum での deserialize に失敗
   - **方針**: 新 version 以降の新規 run でのみ動作、旧 run は持ち越さない (broken のまま untouched)

5. **Findings.json の structure 変更**
   - 現状: 1 agent が全 observation の finding を 1 file に出力
   - 新: observation 別 file + parent merge の 2 段階
   - `findings.json` の最終 shape は現行と同じ (`findings[]` 配列、各 finding に `observation` field)、parent が生成
   - 中間 `findings-*.json` は新規

## Test Strategy

### belt-core unit tests

- `Invoker` parse: `skill` / `pipeline` キーを含む YAML のみ Ok、`agent:` / `agents:` / `iterations:` を含む YAML は parse Err
- `IterationsSpec` 削除後の compile 成功
- `lint_pipeline`: `invoke.agent` / `invoke.agents` / `invoke.iterations` キーを持つ pipeline を reject、エラーメッセージに migration hint を含む

### Adversarial probes

- **Race condition**: 各 observation agent が別 path に write するため race なし、ただし 2 つの observation が偶然同じ path に書くような regressions を unit test で guard
- **Empty findings**: UI-design reviewer が 0 finding を返す場合も `findings-ui-design.json: {"observation":"ui-design","findings":[]}` を作成すること (parent merge が file 不在で crash しない contract)
- **Codex skipped**: `--codex` が false の場合、parent が `/codex:rescue` を invoke しないこと
- **Idempotency**: 同一 run で parent が 2 度目の dispatch を試みた場合、findings-*.json が overwrite されて最新 findings.json が正しく生成されること
- **Dedup tie-break**: 同一 severity の findings 複数で observation priority が正しく tie-break として機能すること (decision table test)

### Integration tests (plugins)

- `/code-review` 単体 (CLI from test fixture) で 4 + optional Codex の並列 dispatch が走り、merged `findings.json` が produce されること
- `/spec-review` で grill-me group partition が parent で正しく動作すること
- feature-dev pipeline の `code-review` phase が新 criteria で validate を pass して step できること (criteria 強化後)

### Parent merge logic decision table

`docs/specs/` 内に補足表として dedup decision table を残す (実装時に detailed table を spec に添付):

| Finding A (obs=Security, sev=critical) | Finding B (obs=Quality, sev=high) | 同一 issue? | 集約先 |
|---|---|---|---|
| SQL injection in 3 files | DRY violation in 3 files | yes | Security (severity 優先) |
| SQL injection in file X | naming inconsistency in file Y | no | 両方 keep |
| Impact: caller change → N+1 | Performance: N+1 query | yes | Impact (severity 同じ、actionability で Impact > Performance... や、むしろ Performance が fix 方針 owner なので Performance) |
| (Codex) Hardcoded secret | Security: Hardcoded secret | — (Codex は dedup 対象外) | 両方 keep |

(最後の Impact vs Performance の例は actionability 解釈の grey zone、実装時に re-examine)

## Migration Order

1. **belt-core の Invoker 縮小** — model.rs / lint.rs / engine.rs / view.rs / expander.rs / tests 更新。fixtures の `invoke.agents` 使用を修正
2. **`belt-agent/SKILL.md` の invoke 変換表 4→2 variant 更新**
3. **code-review 観点 agent 分解** — 新 4 agent file を transcription で作成、`code-reviewer.md` はこの時点では残す (新構造が機能してから削除)
4. **code-review parent SKILL.md 書き換え** — parallel dispatch + merge + dedup + triage + verify
5. **code-review pipeline.yml / belt.toml 削除、旧 `code-reviewer.md` 削除**
6. **spec-review 側 (3, 4, 5 を mirror)**
7. **`criteria/code-review.md` + `criteria/spec-review.md` + `criteria/fix-plan-review.md` を feature-dev / bug-fix 側で triage+fix 要求に更新**
8. **Documentation** — `docs/specs/2026-04-06-belt-redesign.md` の Invoker 節に partial revert memo、memory 4 件 update (`project_belt32_invoker_artifact.md`, `project_belt_core_model_shapes_2026_04_14.md`, `project_review_skills_refresh_2026_04_15.md`, `project_belt_agent_cli_shape.md`)

各 step で `cargo fmt -p <changed> && cargo clippy -p <changed> -- -D warnings && cargo test -p <changed>` を実行。Workspace 全体は最終 step で `cargo clippy --workspace -- -D warnings && cargo test --workspace`。

## Out of Scope

- 軽量版 review skill の新設 (別 brainstorm で扱う)
- feature-dev / bug-fix pipeline の phase 構造全面刷新 (criteria 文言調整のみ)
- Review 以外の pipeline.yml 変更 (monkey-test, bug-fix の integrate 等は保持)
- BELT-32 Invoker / Artifact first-class 化の完全 revert
- N-way voting / iterations の再導入
- Codex 専用 reviewer agent 定義の追加 (既存 `/codex:rescue` で済ませる)
- 観点自体の追加・削減 (code-review = 7、spec-review = 5 を維持)
- review 本体の品質向上 (観点 checklist / policy 文面は移植のみ)
- `agents/code-reviewer.md` / `agents/spec-reviewer.md` 以外の agent 定義の変更 (belt-agents/agents の 5 agent は touch しない)

## Resolved Open Questions (from brainstorming)

| Question | Resolution |
|---|---|
| 並列複数 subagent の capability は将来的に必要か | 不要。`invoke.agents` / `iterations` 削除確定 |
| Agent 定義ファイルの配置 | agents/ を残す (subagent の system prompt source として機能)。skill 化による body 統合は行わない |
| Triage / fix の責務 | 親 SKILL.md (main context) が triage、Edit tool で fix。skill 化しない |
| Observation 分散の粒度 | Independent (並列) / Integrated (統合) の hybrid。全 N 分散は cross-observation self-dedup を壊すため不採用 |
| Codex dispatch 方式 | 既存 `/codex:rescue` を review-specific prompt で invoke。新 `/codex:review` 不要 |
| Dedup priority 軸 | Severity-first + observation priority tie-break (actionability 順) |
| Codex finding の dedup | 対象外 (別 source として保持) |

## Open Questions (awaiting implementation)

1. **findings.json の最終 shape 最適化** — 現状 `findings[]` 配列に `observation` field を付ける scheme。複数 observation が同一 issue を指した場合の付加 metadata (`dedup_from: ["quality", "impact"]` など) は必要か。**推奨**: 現時点では不要。実装時に必要性を再判断
2. **Integrated agent の system prompt 長さ** — 4 観点 (code-review) + 共通 filtering の prompt は現状の code-reviewer.md の該当節の合計より若干短くなる (cross-cutting 以外の 3 観点を切り出すため)。attention 分散の懸念は緩和される想定、実機計測は実装後
3. **Dedup logic の decision table を SKILL.md に embed するか、reference file に分離するか** — SKILL.md inline で済むなら inline、長くなれば `references/dedup-priority.md` に分離。実装時判断

## Memory Updates (after implementation)

| Memory | 更新内容 |
|---|---|
| `project_belt32_invoker_artifact.md` | Agent/Agents variant の partial revert、IterationsSpec 削除を追記 |
| `project_belt_core_model_shapes_2026_04_14.md` | Invoker enum の新 shape (Skill/Pipeline のみ) に更新 |
| `project_review_skills_refresh_2026_04_15.md` | Subagent decomposition (independent/integrated) 方針への進化と 2026-04-16 spec への pointer |
| `project_belt_agent_cli_shape.md` | invoke 変換表 2 variant 反映 |

## References

- `docs/specs/2026-04-15-review-skills-refresh-design.md` — 直前の refresh (single-agent consolidation)
- `docs/specs/2026-04-13-belt32-followup-design.md` — Invoker first-class 化の前提
- `docs/specs/2026-04-06-belt-redesign.md` — belt 再設計全体
- `plugins/code-review/agents/code-reviewer.md` — 観点分解の source (移植元)
- `plugins/spec-review/agents/spec-reviewer.md` — 同上
- `skills/belt-agent/SKILL.md` — invoke 変換表の更新対象
- `.claude/plugins/cache/codex/skills/codex-rescue/` — `--codex` 時の dispatch 先
