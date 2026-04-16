# plugins/ English Conversion + Frontmatter Optimization per Anthropic Best Practices

**Linear**: BELT-TBD (to be created)
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)
**Status**: Draft
**Date**: 2026-04-16

## Summary

`plugins/` 配下の全公開コンテンツ（7 plugins / 51 files）を英語化し、併せて frontmatter `description` を Anthropic 公式 Skill 著作ベストプラクティスに準拠させる。対象は日本語含有 15 files の英訳、11 files の frontmatter `description` 書換、および他プラグインの phase 番号に依存する brittle な cross-plugin reference の除去。

主目的は公開コンテンツ英語化ポリシー（memory 記載、2026-04-14 確立）の履行。副次目的は `description` が毎セッションの system prompt に常駐するコストの削減と、トリガー精度の向上。

## Background

### Problem

1. **日本語の残存**: `plugins/` 配下に日本語 417 occurrences / 15 files が残存。ポリシー違反
2. **description が公式パターン不準拠**:
   - 全 11 skill/agent description に `Use when <trigger>` 節が欠落
   - `feature-dev` (40 words) / `monkey-test` (45 words) は phase enumeration が description の過半を占め、実装詳細である phase flow が triggering に寄与していない
   - `code-review` / `spec-review` は "via a single consolidated reviewer subagent" のような実装メカニズムが description に混入
3. **Phase 番号による cross-plugin coupling**: 6 description が `feature-dev Phase 1/2/5/6` のように他プラグインの phase 番号を直接参照。feature-dev pipeline は 2026-04-14 に 10→8 phase 刷新の前例があり、今後の再構成で rot するリスク

### Anthropic Best Practices (canonical rules)

`https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices` より:

- **Description は critical for skill selection**。metadata は startup 時に system prompt へ pre-load される
- **Third person 厳守**: "I can" / "You can" 禁止
- **What + When の両方を含む**: 公式パターンは `<capability>. Use when <trigger condition>.`
- **Be specific and include key terms**
- **Avoid vague descriptions** (`"Helps with documents"`)
- **Max 1024 chars** (hard limit)
- **SKILL.md body 500 行未満**
- **References one level deep from SKILL.md**
- **Avoid time-sensitive information**

公式例 (PDF skill): `"Extract text and tables from PDF files, fill forms, merge documents. Use when working with PDF files or when the user mentions PDFs, forms, or document extraction."` — 24 words

## Goals

1. `plugins/` 配下の日本語を 0 occurrences にする
2. 全 11 skill/agent description を Anthropic 公式パターン `<capability>. Use when <trigger>. <flag/variant>.` に準拠させる (target 20-35 words)
3. Cross-plugin の phase 番号ハードコード 6 箇所を phase 名参照に置換
4. 副次ルール（500-line / nested references / time-sensitive）の遵守を監査・維持

## Non-Goals

- `docs/` 配下の日本語文書の英訳（ポリシー上、日本語許容）
- plugin.json description の書換（既に英語、適切な長さ）
- SKILL.md 本文の構造再設計（Phase 列挙の本文セクションは local anchor として維持）
- 新機能追加 / pipeline の挙動変更
- 翻訳精度の評価メトリクス作成（skill-creator evaluation は本スコープ外）

## Audit Findings

### 500-line rule ✅ 全 files 通過
最大: `code-reviewer.md` 331 行。`phase-auditor.md`/`spec-reviewer.md` 約 200 行。他は全て 130 行未満。追加 split 不要。

### Nested references ✅ 違反なし
SKILL.md → ref.md の markdown link は 2 箇所のみ、いずれも 1-level:
- `feature-dev/skills/feature-dev/SKILL.md:92` → `plugins/belt-agents/references/narrative-convention.md`
- `bug-fix/skills/bug-fix/SKILL.md:81` → 同上

`narrative-convention.md` 内部に MD link 無し（chain 継続なし）。cross-plugin だが belt monorepo 内で resolve 可能。

### Time-sensitive / coupling ⚠️ 6 件の修正必要

Phase 番号ハードコードを含む description:

| file | 問題箇所 |
|---|---|
| `belt-agents/agents/code-architect.md` | `feature-dev Phase 1 の並列探索で使用` |
| `belt-agents/agents/code-explorer.md` | 同上 |
| `belt-agents/agents/impact-analyzer.md` | 同上 |
| `belt-agents/agents/feature-implementer.md` | `feature-dev の Phase 5 実装` |
| `monkey-test/skills/monkey-test/SKILL.md` | `Designed for feature-dev Phase 6` |
| `test-scenarios/skills/test-scenarios/SKILL.md` | `Designed for feature-dev Phase 2` |

方針: description では phase **番号**ではなく phase **名**で参照する。SKILL.md 本文内の `### Phase N: name` local anchor はそのまま（phase 追加/削除しない限り invariant）。

## Design Rules

### Rule 1: Translation style

- **Body** (references / agents / criteria / SKILL.md 本文): idiomatic English
  - "Claude is already smart" 原則に従い、自明な説明は削除
  - 冗長な受動表現は能動に
  - 意味は厳密に保存
- **Frontmatter `description`**: Rule 2 に従う
- **Terminology 一貫性**: skill 内で `validate` / `verify` / `gate` などは 1 語に統一

### Rule 2: Description shape

```
<third-person capability>. Use when <trigger condition>. <optional flag/variant clause>.
```

- **Target**: 20-35 words
- **Hard limit**: 1024 chars (公式制約、全案余裕)
- **Must include**:
  1. 三人称の capability 文（動詞 `reviews` / `extracts` / `generates` / `executes` / ...）
  2. `Use when ...` trigger 節
  3. domain-specific key terms（トリガーに効く語彙）
- **Must NOT include**:
  - Phase flow の `→` 列挙 → SKILL.md 本文 "Pipeline Overview" セクションへ移設
  - 実装メカニズム（`"via a single consolidated reviewer subagent"`）
  - **他プラグインの Phase 番号** → phase 名で参照
- **Keep**: numeric identity (`7 observations`, `9 phases`), 挙動 flag (`--e2e`, `--codex`)

### Rule 3: SKILL.md body

- 500 行未満を維持（現状全通過）
- `description` から削った phase flow は SKILL.md 冒頭の `## Pipeline Overview` に移設
- 日本語セクション（Narrative Notes / Red Flags 等）は idiomatic English に rewrite

### Rule 4: References

- 1-level depth 維持
- file 名は lowercase + hyphen, descriptive（現状全準拠）

## Scope Inventory

Total unique files touched: **19** (15 with Japanese content + 4 English-only SKILL.md that still need description rewrite to match the canonical pattern).

### Group A: Frontmatter description rewrite (11 files)

| file | current lang | action |
|---|---|---|
| `belt-agents/agents/code-architect.md` | JP | JP→EN, add `Use when`, drop phase number |
| `belt-agents/agents/code-explorer.md` | JP | 同上 |
| `belt-agents/agents/impact-analyzer.md` | JP | 同上 |
| `belt-agents/agents/feature-implementer.md` | JP | 同上 |
| `belt-agents/agents/phase-auditor.md` | JP | JP→EN, add `Use when` |
| `monkey-test/skills/monkey-test/SKILL.md` | EN | add `Use when`, drop phase number |
| `test-scenarios/skills/test-scenarios/SKILL.md` | EN | 同上 |
| `bug-fix/skills/bug-fix/SKILL.md` | EN | move phase flow to body, add `Use when` |
| `feature-dev/skills/feature-dev/SKILL.md` | EN | 同上 |
| `code-review/skills/code-review/SKILL.md` | EN | add `Use when`, drop impl detail |
| `spec-review/skills/spec-review/SKILL.md` | EN | 同上 |

### Group B: Body translation (12 files — overlaps with Group A on 4 files: `impact-analyzer`, `phase-auditor`, `bug-fix/SKILL`, `feature-dev/SKILL`)

| file | JP occurrences | priority |
|---|---|---|
| `code-review/agents/code-reviewer.md` | 139 | P0 |
| `belt-agents/references/evidence-catalog.md` | 83 | P0 |
| `spec-review/agents/spec-reviewer.md` | 81 | P0 |
| `belt-agents/references/narrative-convention.md` | 33 | P0 |
| `belt-agents/references/criteria-template.md` | 20 | P0 |
| `belt-agents/agents/phase-auditor.md` | 15 | P0 |
| `bug-fix/skills/bug-fix/SKILL.md` | 15 | P0 |
| `feature-dev/skills/feature-dev/criteria/spec-review.md` | 12 | P0 |
| `belt-agents/agents/impact-analyzer.md` | 6 | P0 |
| `feature-dev/skills/feature-dev/SKILL.md` | 5 | P0 |
| `bug-fix/skills/bug-fix/references/fix-plan-supplement.md` | 1 | P1 |
| `bug-fix/skills/bug-fix/criteria/dogfood.md` | 1 | P1 |

## Target Descriptions

### Skills (6)

```yaml
# bug-fix (30 → 25 words)
description: >-
  Runs a quality-gated debugging pipeline with root-cause analysis, fix planning,
  code review, and regression verification. Use when a bug needs structured
  diagnosis and verified repair. --e2e adds browser-based regression tests;
  --codex enables adversarial review.

# feature-dev (40 → 28 words)
description: >-
  Runs a quality-gated feature-development pipeline spanning design, test
  strategy, spec review, implementation, and regression verification. Use when
  building a new feature that needs a structured design-to-integration flow.
  --e2e enables browser-based verification; --codex enables adversarial review.

# code-review (24 → 28 words)
description: >-
  Reviews a diff across seven dimensions: quality, security, performance, tests,
  AI anti-patterns, impact, and simplification. Use when code changes need
  multi-perspective critique before merging. --codex adds an adversarial pass.

# spec-review (24 → 26 words)
description: >-
  Reviews a spec across five dimensions: requirements, design judgment,
  feasibility, consistency, and UI. Use when a design doc needs
  multi-perspective critique before implementation, with findings driving a
  grill-me dialogue.

# monkey-test (45 → 24 words)
description: >-
  Replays Given/When/Then scenarios via agent-browser and emits human- plus
  machine-readable reports. Use when running scripted E2E regression on a web
  UI with pre-defined scenarios.

# test-scenarios (30 → 28 words)
description: >-
  Generates an ISTQB + ISO 25010 test strategy, plus an agent-browser-replayable
  Given/When/Then scenarios.yml when --e2e is set. Use when a feature design
  needs a test plan or replayable E2E scenarios.
```

### Agents (5)

```yaml
# phase-auditor (JP → 25 words EN)
description: >-
  Verifies pipeline deliverables against Done Criteria and Evidence Plan,
  emitting structured diagnostics with fix instructions. Use when a phase needs
  independent verification. Verification-only; never edits files.

# code-architect (JP → 22 words EN)
description: >-
  Extracts architectural patterns, conventions, and design decisions from an
  existing codebase. Use during parallel design exploration to inform a new
  feature's architecture.

# code-explorer (JP → 25 words EN)
description: >-
  Traces code flow end-to-end from entry point to data layer, summarizing
  dependencies, patterns, and constraints. Use during parallel design
  exploration to understand an existing feature.

# impact-analyzer (JP → 24 words EN)
description: >-
  Traces reverse dependencies from changed code to map blast radius, implicit
  constraints, and side-effect risks. Use during parallel design exploration
  before modifying established code paths.

# feature-implementer (mixed → 22 words EN)
description: >-
  Executes implementation tasks via test-driven development. Use proactively for
  pipeline implementation and fix dispatch tasks where each task has
  self-contained specs.
```

## Rollout

Single PR. Commits split by concern for reviewability:

1. `docs(plugins): rewrite frontmatter descriptions per Anthropic best practices` (11 files, description のみ)
2. `docs(plugins): remove cross-plugin phase-number coupling in descriptions` (もし 1 に含まれなければ)
3. `docs(plugins): translate agent bodies to English` (code-reviewer, spec-reviewer, phase-auditor, impact-analyzer)
4. `docs(plugins): translate references to English` (evidence-catalog, narrative-convention, criteria-template)
5. `docs(plugins): translate SKILL.md and criteria bodies` (bug-fix/SKILL, feature-dev/SKILL, spec-review criteria 他)

## Verification (Definition of Done)

- `grep -r '[ぁ-んァ-ヶー一-龯]' plugins/` が 0 件
- 全 11 description に以下を満たす:
  - word count 20-35 範囲内
  - `Use when` 節を含む
  - 三人称（`I` / `You` / `We` 非含有）
  - 他プラグインの `Phase N` ハードコード非含有
- SKILL.md 本文 500 行未満を維持
- `cargo test -p belt-core` / `cargo test -p belt-agent` / `cargo clippy --workspace -- -D warnings` が pass（documentation 変更のみなので影響なし、regression 確認）
- 各 skill を実際にセッションでロードし、description triggering を spot-check
  - `/bug-fix` / `/feature-dev` / `/code-review` / `/spec-review` / `/monkey-test` / `/test-scenarios` それぞれ最低 1 回 invoke
- Agent tool の description menu で 5 agents (phase-auditor, code-architect, code-explorer, impact-analyzer, feature-implementer) が英語で表示されることを確認

## Risks

### R1: Triggering 精度の regression
- **Mitigation**: 既存 description に含まれる key term（`quality-gated`, `pipeline`, `7 observations` 等の識別語）を意図的に保持。`Use when` 節は trigger 増強であり削減ではない

### R2: SKILL.md 本文に phase flow を移設する過程で冗長化
- **Mitigation**: 本文 "Pipeline Overview" は 1 行の ASCII flow (`rca → fix-plan → ... → integrate`) のみ追加。既存 "Phase N: name" セクションは変更しない

### R3: 翻訳時の意味 drift
- **Mitigation**:
  - Body 翻訳は段落単位で原文と対応付け
  - Technical term は belt 全体で使用中の語彙（`gate`, `regate`, `artifact`, `narrative note` 等）に揃える
  - 各 commit を小さく保ち、reviewer が段落ごとに diff 確認可能にする

### R4: Cross-plugin path (`plugins/belt-agents/references/narrative-convention.md`) が plugin 単体配布時に resolve しない
- **Out of scope**: 現状 belt monorepo 配布前提。plugin 単体配布は将来の packaging 設計時に対応（本 spec で touch しない）

## Open Questions

なし（設計に user 合意済み、2026-04-16）。

## Next Steps

1. Spec self-review（本 spec 自体の placeholder / contradiction / ambiguity チェック）
2. User review of this spec
3. `/superpowers:writing-plans` で implementation plan 作成
4. Plan review → execute → PR
