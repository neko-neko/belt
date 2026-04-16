---
title: README Refresh — Engine-First Positioning with Inline Lint Demo
date: 2026-04-16
status: approved
scope:
  - README.md
related:
  - docs/superpowers/specs/2026-04-16-plugin-skill-md-refresh-design.md
  - docs/superpowers/specs/2026-04-15-belt-plugins-migration-design.md
  - MEMORY: project_skill_md_authoring_principle.md
  - MEMORY: feedback_subagent_prompt_verbatim_spec.md
---

# README Refresh — Engine-First Positioning with Inline Lint Demo

## 1. Overview

現行 README.md (209 行) は workflow engine (`belt-core` / `belt-agent`) と
Claude Code plugin suite (7 plugin) を対等に扱う「総合カタログ型」。Plugins
セクションが 78 行 (全体の 37%) を占め、engine の姿勢がぼやけている。

本 spec は以下の 2 点を軸に README を engine 特化型に再整理する:

1. **Agent Skill 向け workflow engine としての位置づけ**:
   Claude Code 特化でも LLM agent 一般向けでもなく、Anthropic 公式の
   "Agent Skill" 文脈に合わせ、`belt` を「LLM-driven Agent Skills 向けの
   deterministic workflow engine」として語る。
2. **事前検査 (`belt lint`) の可視化**:
   `belt lint` が LLM 実行前に構造的バグを検出する価値を、Why の 1 文と
   Example 直後の実行例、CLI セクションの効能説明の 3 箇所で強調する。

Plugins セクションの情報量 (install 手順・external deps 表・internal
dependencies・usage) は現行維持する。engine 特化は章立ての順序と
見出し命名 ("Working Examples" の付与) で構成的に表現する。

## 2. Scope / Non-goals

**In-scope**:

- `README.md` の全面リライト (全章を retain / delete / new に分解)
- Plugins 表内の phase 列挙削除 (SSOT 原則: pipeline.yml 側へ委譲)
- Plugins セクション見出しに "(Working Examples)" 付与
- Why への "Agent Skill workflow engine" と "事前検査" 1 文追加
- Example 直後に `belt lint` 実行例を code block で追加
- CLI セクションに `belt lint` の効能説明を追加

**Non-goals**:

- `CLAUDE.md` の改変 (責務分離方針の明示は本刷新のスコープ外)
- `plugins/README.md` の新規作成 (案 β 採用のため不要)
- `belt-agent status` の出力フォーマット変更 (README 内 JSON example は
  現行仕様のまま)
- 新規 screenshot / badge / logo の追加
- `CONTRIBUTING.md` の新規作成
- 日本語版 README の追加
- Linux 哲学 / Tiny by Constraint の README 記載 (Q7 = C で不採用)
- 章立て順序の大規模変更 (現行順を維持、engine 主軸は見出し変更で表現)

## 3. Decisions (Q1–Q7 + Approach + β 調整)

本 spec に至るまでの brainstorming 決定事項:

| # | 選択肢 | 決定 |
|---|--------|------|
| Q1 | 主要読者像 | B: engine 特化型 (workflow engine 主軸、plugins は実例として後半) |
| Q2 | 技術詳細の粒度 | B: Light reference (行数目標は β 調整で 200-220 行に緩和) |
| Q3 | Why の表現方針 | A ベース (LLM context saving) + "Agent Skill workflow engine" と "事前検査" の 2 点を強調 |
| Q4 | "skill" 用語の範囲 | "Agent Skill" 文脈 (Claude Code 特化より一段抽象、Anthropic 公式概念) |
| Q5 | Plugins セクションの扱い | C: 現行情報量維持、engine 主軸の後半に配置 |
| Q6 | `belt lint` の紹介位置 | D: Why に 1 文 pitch + CLI セクションに実行例と効能 |
| Q7 | Linux 哲学 / Tiny by Constraint | C: 記載しない |
| Approach | 刷新戦略 | B: 章立て再編 + inline lint demo |
| β 調整 | Q2 × Q5 の整合性 | β: Plugins 現行維持を優先、行数目標を 200-220 行に緩和 |

不採用案:

- **Approach A (minimal tweak)**: 差分が小さく「刷新」としての意味が薄い
- **Q5 案 α (plugins/README.md 新規作成)**: 刷新スコープが README 単体を超える
- **Q2 行数厳守 (150-180)**: Q5 の現行維持と矛盾、engine 特化は行数でなく構成で表現
- **Q3-B (Linux 哲学前面)**: Q7 = C と整合しないため不採用
- **Q7-A (1 文溶け込み)**: Q7 = C で明示的に不採用

## 4. 行数見積もり

現行 209 行 → 新版 **225-235 行 (実見積もり 232 行)**。

案 β の「行数目標を 200-220 行に緩和」は Q2 × Q5 衝突の判断軸に関する
決定 (Plugins 現行維持を優先) であり、実行数はその帰結として案 β の名目
目標を上回る。engine 特化は構成 (Plugins 見出しへの "(Working Examples)"
付与と章順序) で表現し、行数制約は副次的とする。

| セクション | 現行 行数 | 新版 行数 | 差分要因 |
|-----------|---------|---------|----------|
| Intro (title + 1-2 文) | 4 | 5 | +1 行: "Agent Skill" 明記 + 1 文追加 |
| Why belt? | 8 | 12 | +4 行: 事前検査の段落追加 (空行含む) |
| Example + lint demo | 34 | 44 | +10 行: lint demo code block + 補足段落 (空行・fence 含む) |
| CLI (Binary Separation + Agent loop + Status) | 49 | 57 | +8 行: `belt lint` 効能段落 (空行含む) |
| Key Concepts | 13 | 13 | 変更なし |
| Build | 12 | 12 | 変更なし |
| Claude Code Plugins (Working Examples) | 79 | 79 | ±0 行: 見出し変更 + phase 列挙 2 行削除 + 代替文言 2 行追加 |
| License | 3 | 3 | 変更なし |
| 合計 | 209 | 232 | +23 行 |

(各セクションの行数は README.md 上の実行数を wc 相当でカウントした値。
追加段落に必要な前後の空行 1 行を含む)

## 5. Section-by-Section retain / delete / new (逐語列挙)

plan 段階での paraphrase を防ぐため (MEMORY
`feedback_subagent_prompt_verbatim_spec.md`)、retain / delete / new を行番号と
逐語で明示する。plan controller prompt は本 section の逐語列挙を
verbatim 転記する。paraphrase 禁止。

### 5.1 Intro (line 1-4 書き換え)

**削除 (逐語)**:

```
# belt

A lightweight workflow engine for AI agents. Define deterministic state machines
in YAML, drive them idempotently from any LLM.
```

**追加 (逐語 draft)**:

```
# belt

A workflow engine for LLM-driven Agent Skills. Declare deterministic state
machines in YAML, drive them idempotently from any LLM, and lint them
statically before they ever reach execution.
```

### 5.2 Why belt? (line 6-13 保持 + 1 文追加)

**保持 (逐語、変更なし、現行 6-13 行)**:

```
## Why belt?

When LLM agents control entire workflows — phase transitions, gate checks,
retry loops — they burn context on bookkeeping instead of reasoning. A 10-phase
pipeline can cost ~900 lines of prompt just to maintain structure. belt moves
the deterministic control plane into YAML: the agent calls `belt-agent next`
to receive one phase at a time, executes it, and calls `belt-agent verify`
to check gates. The pipeline definition never enters the context window.
```

**追加 (逐語 draft、上記 block の末尾に 1 段落挿入)**:

```
Pipelines are statically linted with `belt lint` before any LLM run, so
structural errors — missing phase IDs, invalid gate checks, broken `uses:`
references — never reach execution.
```

### 5.3 Example + Inline Lint Demo (line 15-48 保持 + 直後に追加)

**保持 (逐語、変更なし、現行 15-48 行)**: YAML Example 全体 (heading
`## Example` と fenced code block `name: review-and-ship` 〜 `confirm: true`)
を変更しない。

**追加 (逐語 draft、Example fenced block の直後に挿入)**:

````
Lint it before handing it to the agent:

```
$ belt lint review-and-ship.yml
ok: review-and-ship.yml
```

If any phase id is duplicated, a gate is malformed, or a `uses:` reference is
unresolvable, lint exits non-zero with a descriptive diagnostic and the agent
is never invoked.
````

(実出力フォーマット: success は `ok: <path>`、warning あり成功は
`ok (with warnings): <path>`、error は `error: <message>` + exit 1。
本 spec 作成時点で `cargo run -p belt -- lint plugins/feature-dev/skills/feature-dev/pipeline.yml`
で確認済み)

### 5.4 CLI (line 50-98 保持 + lint 効能追加)

**保持 (逐語、変更なし、現行 50-98 行)**:

- `## CLI` 見出しと intro "belt ships two binaries, separated by audience:"
- Binary Separation 表 (現行 54-57 行)
- `### Agent loop` 見出しとその下の code block (現行 59-69 行)
- "All `belt-agent` output is JSON..." 段落 (現行 71-72 行)
- `### Status` 見出しとその下の code block / JSON example / 説明段落
  (現行 74-98 行)

**追加 (逐語 draft、Binary Separation 表の直後、`### Agent loop` 見出しの前に
挿入)**:

```
`belt lint` is the pipeline author's fast feedback loop: it runs in
milliseconds, catches structural errors (duplicate phase IDs, unknown `regate`
targets, undefined args referenced from `when:`, missing descriptions,
unresolvable `uses:` / `invoke.pipeline:` references, artifact flow
violations, and sub-pipeline expansion failures), and exits non-zero on any
finding — ideal for pre-commit hooks and CI.
```

### 5.5 Key Concepts (line 100-112 保持)

**保持 (逐語、変更なし、現行 100-112 行)**: `## Key Concepts` 見出しと
gate / validate / uses / regate / config の箇条書き全体を変更しない。

### 5.6 Build (line 114-125 保持)

**保持 (逐語、変更なし、現行 114-125 行)**: `## Build` 見出しと
`cargo build --workspace` 以下の内容全体を変更しない。

### 5.7 Claude Code Plugins (Working Examples) (line 127-205 部分変更)

**見出し変更 (逐語、現行 127 行)**:

削除:

```
## Claude Code Plugins
```

追加:

```
## Claude Code Plugins (Working Examples)
```

**保持 (逐語、変更なし)**:

- Intro 段落 (現行 129-130 行): "belt ships 7 Claude Code plugins under
  `plugins/` — working examples and production tooling for
  quality-gated AI-driven development."
- `### Plugins in this repo` 見出し (現行 132 行)
- Plugins in this repo 表の先頭 2 行 (現行 134-136 行: header と
  `belt-agents` 行)
- Plugins in this repo 表の残りの行 (現行 139-142 行: `code-review`,
  `spec-review`, `monkey-test`, `test-scenarios`)
- `### External skill dependencies` 見出しと本文 (現行 144-158 行)
- `### Install` 見出しと本文 (現行 160-185 行)
- `### Internal dependencies (plugin-to-plugin)` 見出しと本文
  (現行 187-192 行)
- `### Usage` 見出しと本文 (現行 194-205 行)

**削除 (逐語、現行 137-138 行)**:

```
| `feature-dev` | 9-phase development pipeline (design → test-scenarios → spec-review → plan → execute → code-review → monkey-test → dogfood → integrate) |
| `bug-fix` | 8-phase debugging pipeline (rca → fix-plan → fix-plan-review → execute → code-review → monkey-test → dogfood → integrate) |
```

**追加 (逐語 draft、削除行の置き換え)**:

```
| `feature-dev` | Quality-gated feature-development pipeline (design → implementation → review → integration). Phase structure is declared in the plugin's `pipeline.yml` |
| `bug-fix` | Quality-gated debugging pipeline (RCA → fix → review → integration). Phase structure is declared in the plugin's `pipeline.yml` |
```

### 5.8 License (line 207-209 保持)

**保持 (逐語、変更なし、現行 207-209 行)**: `## License` 見出しと "MIT" を
変更しない。

## 6. データフロー / 責務分離

本刷新で README が担う責務:

- **belt のアイデンティティ**: "LLM-driven Agent Skills 向け workflow engine"
- **価値提案**: LLM context saving + 事前検査可能
- **入門的な使い方**: Example / Agent loop / Build
- **概念説明**: gate / validate / uses / regate / config (Key Concepts)
- **具体的な使用例**: Claude Code Plugins (Working Examples)

`CLAUDE.md` が担う責務 (README と重複しない部分):

- **実装詳細**: Crate 構成 / belt-core 10 モジュール / GateCheck 実装
- **依存管理**: Workspace Cargo.toml / Lint policy / バージョン指定
- **運用原則**: 依存制限 / Non-Goals / Known Risks
- **YAML Universe / Future Phases**

本刷新で `CLAUDE.md` は変更しない。責務分離は現状の棲み分けで十分。
将来 `CLAUDE.md` に手を入れる場合は別 spec。

## 7. 検証戦略

### 7.1 行数検証

```bash
wc -l README.md
```

目標: 210-220 行範囲 (案 β 採用)。

### 7.2 削除対象が消えているか

```bash
grep -nE '9-phase development pipeline \(design →' README.md   # expected: no match
grep -nE '8-phase debugging pipeline \(rca →' README.md        # expected: no match
grep -nE '^## Claude Code Plugins$' README.md                  # expected: no match
grep -nE 'A lightweight workflow engine for AI agents' README.md  # expected: no match
```

### 7.3 追加対象が存在するか

```bash
grep -nE 'LLM-driven Agent Skills' README.md                   # expected: 1 match
grep -nE 'Pipelines are statically linted with' README.md      # expected: 1 match
grep -nE '\$ belt lint review-and-ship\.yml' README.md         # expected: 1 match
grep -nE 'ok: review-and-ship\.yml' README.md                  # expected: 1 match
grep -nE 'fast feedback loop' README.md                        # expected: 1 match
grep -nE '## Claude Code Plugins \(Working Examples\)' README.md  # expected: 1 match
```

### 7.4 lint demo の事実性確認

`cargo run -p belt -- lint <path>` を手元で実行し、Example 直後の demo
文言 (`ok: review-and-ship.yml`) が実際の success 出力フォーマットと一致する
ことを確認 (確認済み: 2026-04-16 時点、`crates/belt/src/main.rs` の
`eprintln!("ok: {display}")` 出力と一致)。

### 7.5 lint error 種類列挙の実装整合

Section 5.4 で列挙した lint error カテゴリ (duplicate phase IDs / unknown
`regate` targets / undefined args from `when:` / missing descriptions /
unresolvable `uses:` / `invoke.pipeline:` / artifact flow violations /
sub-pipeline expansion failures) が `crates/belt-core/src/lint.rs` の実装に
存在することを確認:

- `duplicate phase id` (line 77 付近)
- `regate target '{}' does not exist` (line 89 付近)
- `when references undefined arg` (line 106 付近)
- `leaf phase must have a description` (line 124 付近)
- `gate uses '{}' not found` (line 287 付近、`check_gate_uses_exist`)
- `invoke pipeline '{}' not found` (line 312 付近、`check_invoke_pipeline_exists`)
- `check_artifact_flow` (line 146 呼び出し)
- `expansion error` (line 173 付近、Phase 2)

(確認済み: 2026-04-16 時点、`crates/belt-core/src/lint.rs` を Read で確認)

### 7.6 リンク健全性

External skill dependencies 表の GitHub URL:

- `https://github.com/obra/superpowers`
- `https://github.com/max-sixty/worktrunk`
- `https://github.com/vercel-labs/agent-browser`

本刷新で URL は変更しない。plan 段階で 404 チェックを実施するかは任意。

### 7.7 手動 proof-read

Why / Example + lint demo / CLI / Plugins 表の文言が英語として自然か確認。
Technical writer 観点。

### 7.8 テストへの影響

README.md に関する lock test は現時点で存在しない
(`crates/belt-core/tests/` 配下に README 関連は無し)。`cargo test --workspace`
は pass のまま (無関係)。

## 8. Risks

| Risk | 可能性 | 影響 | 対応 |
|------|-------|------|------|
| lint demo の文言が実際の出力と乖離 | Low | 読者を混乱 | Section 7.4 で確認済み。plan 段階で再確認 |
| lint error 種類列挙が実装と乖離 | Low | 事実誤認 | Section 7.5 で確認済み。plan 段階で再確認 |
| Plugins phase 列挙削除で訴求力が落ちる | Low | 検索流入からの理解負荷増 | 代替文言 "Quality-gated feature-development pipeline" で価値を保持 |
| `CLAUDE.md` と README の責務分離が曖昧 | Low | 将来 CLAUDE.md 改変時に drift | 本刷新では CLAUDE.md に触れない。責務分離方針はコミットメッセージで明記 |
| plan 段階で retain/delete が paraphrase される (MEMORY 既記載の副次バグ) | Medium | spec drift | 本 spec の Section 5 逐語列挙を plan controller prompt に verbatim 転記する指示を明記 |

## 9. Implementation Sequence (writing-plans で詳細化する骨格)

1. **事前確認**: `cargo run -p belt -- lint <sample>.yml` で lint demo 文言の
   実出力フォーマット再確認 (Section 7.4)
2. **事前確認**: `crates/belt-core/src/lint.rs` を読み lint error 種類列挙の
   実装整合再確認 (Section 7.5)
3. **編集 5.1**: Intro (line 1-4) 書き換え
4. **編集 5.2**: Why belt? (line 13 末尾) に段落追加
5. **編集 5.3**: Example fenced block 直後に lint demo code block 挿入
6. **編集 5.4**: CLI Binary Separation 表の直後に `belt lint` 効能段落挿入
7. **編集 5.7**: Claude Code Plugins 見出しに "(Working Examples)" 付与 +
   phase 列挙 2 行を代替文言に置換
8. **検証**: Section 7.1-7.3 の grep 検証
9. **検証**: 手動 proof-read (Section 7.7)
10. **コミット**: `docs(readme): refresh for engine-first positioning with
    inline lint demo`

各ステップの成果物と検証は writing-plans で TDD ベースにブレイクダウンする。

## 10. Related Documents

- MEMORY: `project_skill_md_authoring_principle.md` — SSOT 原則 (phase 列挙の
  README からの削除根拠)
- MEMORY: `feedback_subagent_prompt_verbatim_spec.md` — plan 段階の retain /
  delete 逐語列挙を verbatim 転記する原則
- `docs/superpowers/specs/2026-04-16-plugin-skill-md-refresh-design.md` —
  同日付の plugin SKILL.md refresh (SSOT 原則を plugin 側で実装。本刷新は
  README 側で整合を取る)
- `docs/superpowers/specs/2026-04-15-belt-plugins-migration-design.md` —
  現行 README Plugins セクションの由来 (Section 7 で新規追加された内容)
