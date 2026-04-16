# plugins/ English Conversion + Frontmatter Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/specs/2026-04-16-plugins-english-frontmatter-design.md`

**Goal:** Convert all Japanese content in `plugins/` to English and align every frontmatter `description` with Anthropic's canonical skill pattern (`<capability>. Use when <trigger>. <flag clause>.`).

**Architecture:** Documentation-only change across 19 unique files. Each task focuses on one file or a tight pair of related files. Bodies use idiomatic English; descriptions use the Anthropic canonical pattern with 20-35 word targets. Cross-plugin Phase-N hardcodes are replaced with phase-name references to remove brittle coupling.

**Tech Stack:** Markdown, YAML frontmatter. No code changes. Verification uses `rg` (ripgrep via `Grep`) and wc.

---

## Translation Style Guide (shared across tasks)

Apply consistently across every body translation task (Tasks 2-9).

### Principles

1. **Idiomatic English, not literal translation** — rewrite for natural English flow; preserve meaning, not word order.
2. **Claude is already smart** — cut redundant self-explanation that restates what the heading already says.
3. **Third person** — never "I" / "You" / "We" in descriptions; body text may use imperative ("Do not..." / "Report...").
4. **Consistent terminology** — use the glossary below uniformly within a file.

### Glossary (Japanese → English)

| Japanese | English |
|---|---|
| 確信度 | confidence |
| 報告しない | do not report |
| 観点 | observation / dimension (use `observation` to match existing code-reviewer terminology) |
| 基準 | criterion / criteria |
| 該当する | applies / matches |
| 判定 | verdict |
| 軽微 | minor |
| 動く | works |
| 後で直せる | can be fixed later |
| 回避の根拠 | grounds to avoid |
| グレーゾーン | gray area |
| 設計判断 | design decision |
| 設計書 | design document / spec |
| 境界条件 | boundary condition |
| 例外 | exception |
| 暗黙の前提 | implicit assumption |
| 具体的 | concrete / specific |
| 検証可能 | verifiable |
| 完了条件 | completion condition / done condition |
| 影響範囲 | impact / blast radius |
| 呼び出し元 | caller |
| 命名規則 | naming convention |
| 矛盾 | contradiction / inconsistency |
| 未解決マーカー | unresolved markers |
| 状態遷移 | state transition |
| 監査 | audit |
| 延期 | deferred |
| 根本原因 | root cause |
| 再実行 | re-run |
| 直接参照する | directly reference |
| 誤解 | misinterpretation |
| 既存コードベース | existing codebase |
| 成果物 | deliverable / artifact |
| 規約 | convention |
| 下流 | downstream |
| 上流 | upstream |
| 冪等性 / idempotency | idempotency |
| 副作用 | side effect |
| 偏り / バイアス | bias |
| 所見 | observation (the JSON field) |
| 改善余地 | room for improvement |
| 流用 | reuse |
| 設計判断は rca / fix-plan で決定済み | design decisions are already settled in rca / fix-plan |
| 上流の見直しサイン | signal that upstream phases need revisiting |

### Stock phrase translations

These phrases repeat across `code-reviewer.md` / `spec-reviewer.md` / similar review files. Use the same English everywhere.

| Japanese | English |
|---|---|
| 確信度 80% 未満の問題は報告しない | Do not report issues with confidence below 80%. |
| 同一パターンの問題が複数箇所にある場合、1 件の finding にまとめ、件数と代表箇所を記載する | When the same pattern appears in multiple locations, consolidate into one finding with the occurrence count and a representative location. |
| スタイル好みや主観的な「こう書いた方がきれい」は報告しない。プロジェクト規約違反のみ報告する | Do not report stylistic preferences or subjective "this looks nicer" opinions. Only report project convention violations. |
| 観点間で同じ問題が見つかったら、最も本質的な観点 1 箇所のみに置く（self-dedup） | If the same issue is found across observations, keep it under the most essential one (self-dedup). |
| 判定を甘くする方向への rationalization を禁止する。「軽微だから問題ない」「動くから良い」「後で直せる」は REJECT 回避の根拠にならない。基準に該当するなら REJECT する。該当しないなら APPROVE する。グレーゾーンは WARNING とする。 | Do not rationalize your way to a softer verdict. "Minor, so it's fine", "It works, so it's good", "Can be fixed later" are not valid grounds to avoid REJECT. REJECT when a criterion matches; APPROVE when none match; use WARNING for the gray area. |
| 以下の条件に該当する場合、findings の severity を対応するレベルに設定すること。 | When any of the following conditions apply, set the finding's severity to the corresponding level. |
| REJECT 基準（1つでも該当すれば REJECT を推奨） | REJECT criteria (recommend REJECT if any match) |
| WARNING 基準 | WARNING criteria |

### Terminology preservation

Keep as-is (do not translate):

- Code identifiers (`belt-agent`, `belt-core`, `feature-implementer`, `phase-auditor`, etc.)
- Technical terms already in English use (`gate`, `regate`, `artifact`, `narrative note`, `run_id`, `phase_id`, `verdict`, `finding`, `severity`, `observation`, `criterion`, `evidence`, `escalation`)
- File paths, YAML keys, JSON keys
- Status labels (PASS, FAIL, REJECT, APPROVE, WARNING, CRITICAL, HIGH, MEDIUM, LOW)
- Anthropic product names
- git / shell commands

---

## Task 1: Rewrite all frontmatter descriptions + add Pipeline Overview sections

**Files:**
- Modify: `plugins/bug-fix/skills/bug-fix/SKILL.md` (description + add Pipeline Overview section)
- Modify: `plugins/feature-dev/skills/feature-dev/SKILL.md` (description + add Pipeline Overview section)
- Modify: `plugins/code-review/skills/code-review/SKILL.md` (description only)
- Modify: `plugins/spec-review/skills/spec-review/SKILL.md` (description only)
- Modify: `plugins/monkey-test/skills/monkey-test/SKILL.md` (description only)
- Modify: `plugins/test-scenarios/skills/test-scenarios/SKILL.md` (description only)
- Modify: `plugins/belt-agents/agents/code-architect.md` (description only)
- Modify: `plugins/belt-agents/agents/code-explorer.md` (description only)
- Modify: `plugins/belt-agents/agents/impact-analyzer.md` (description only — body translated in Task 4)
- Modify: `plugins/belt-agents/agents/feature-implementer.md` (description only)
- Modify: `plugins/belt-agents/agents/phase-auditor.md` (description only — body translated in Task 4)

- [ ] **Step 1.1: Rewrite `bug-fix/SKILL.md` description**

Replace the existing `description: >- ...` block (lines 3-6) with:

```yaml
description: >-
  Runs a quality-gated debugging pipeline with root-cause analysis, fix planning,
  code review, and regression verification. Use when a bug needs structured
  diagnosis and verified repair. --e2e adds browser-based regression tests;
  --codex enables adversarial review.
```

- [ ] **Step 1.2: Add `## Pipeline Overview` section to `bug-fix/SKILL.md` body**

Insert between `# bug-fix` (line 11) and `## Args` (line 15). Add:

```markdown
## Pipeline Overview

```
rca → fix-plan → fix-plan-review → execute → code-review → monkey-test → dogfood → integrate
```

`monkey-test` and `dogfood` run only when `--e2e` is set.

```

- [ ] **Step 1.3: Rewrite `feature-dev/SKILL.md` description**

Replace existing `description: >- ...` block with:

```yaml
description: >-
  Runs a quality-gated feature-development pipeline spanning design, test
  strategy, spec review, implementation, and regression verification. Use when
  building a new feature that needs a structured design-to-integration flow.
  --e2e enables browser-based verification; --codex enables adversarial review.
```

- [ ] **Step 1.4: Add `## Pipeline Overview` section to `feature-dev/SKILL.md` body**

Insert between `# feature-dev` and `## Args` (match the structure of Step 1.2). Add:

```markdown
## Pipeline Overview

```
design → test-scenarios → spec-review → plan → execute → code-review → monkey-test → dogfood → integrate
```

`monkey-test` and `dogfood` run only when `--e2e` is set.

```

- [ ] **Step 1.5: Rewrite `code-review/SKILL.md` description**

Replace existing `description: >- ...` with:

```yaml
description: >-
  Reviews a diff across seven dimensions: quality, security, performance, tests,
  AI anti-patterns, impact, and simplification. Use when code changes need
  multi-perspective critique before merging. --codex adds an adversarial pass.
```

- [ ] **Step 1.6: Rewrite `spec-review/SKILL.md` description**

Replace existing `description: >- ...` with:

```yaml
description: >-
  Reviews a spec across five dimensions: requirements, design judgment,
  feasibility, consistency, and UI. Use when a design doc needs
  multi-perspective critique before implementation, with findings driving a
  grill-me dialogue.
```

- [ ] **Step 1.7: Rewrite `monkey-test/SKILL.md` description**

Replace existing `description: >- ...` with:

```yaml
description: >-
  Replays Given/When/Then scenarios via agent-browser and emits human- plus
  machine-readable reports. Use when running scripted E2E regression on a web
  UI with pre-defined scenarios.
```

(Removes the "Designed for feature-dev Phase 6" Phase-N coupling.)

- [ ] **Step 1.8: Rewrite `test-scenarios/SKILL.md` description**

Replace existing `description: >- ...` with:

```yaml
description: >-
  Generates an ISTQB + ISO 25010 test strategy, plus an agent-browser-replayable
  Given/When/Then scenarios.yml when --e2e is set. Use when a feature design
  needs a test plan or replayable E2E scenarios.
```

(Removes the "Designed for feature-dev Phase 2" Phase-N coupling.)

- [ ] **Step 1.9: Rewrite `belt-agents/agents/code-architect.md` description**

Replace existing `description: >- ...` with:

```yaml
description: >-
  Extracts architectural patterns, conventions, and design decisions from an
  existing codebase. Use during parallel design exploration to inform a new
  feature's architecture.
```

(Removes the "feature-dev Phase 1" hardcode.)

- [ ] **Step 1.10: Rewrite `belt-agents/agents/code-explorer.md` description**

Replace existing `description: >- ...` with:

```yaml
description: >-
  Traces code flow end-to-end from entry point to data layer, summarizing
  dependencies, patterns, and constraints. Use during parallel design
  exploration to understand an existing feature.
```

- [ ] **Step 1.11: Rewrite `belt-agents/agents/impact-analyzer.md` description**

Replace existing `description: >- ...` with:

```yaml
description: >-
  Traces reverse dependencies from changed code to map blast radius, implicit
  constraints, and side-effect risks. Use during parallel design exploration
  before modifying established code paths.
```

- [ ] **Step 1.12: Rewrite `belt-agents/agents/feature-implementer.md` description**

Replace existing `description: ...` line with:

```yaml
description: >-
  Executes implementation tasks via test-driven development. Use proactively for
  pipeline implementation and fix dispatch tasks where each task has
  self-contained specs.
```

Preserve the `skills:` block and other frontmatter fields unchanged.

- [ ] **Step 1.13: Rewrite `belt-agents/agents/phase-auditor.md` description**

Replace existing `description: ...` line with:

```yaml
description: >-
  Verifies pipeline deliverables against Done Criteria and Evidence Plan,
  emitting structured diagnostics with fix instructions. Use when a phase needs
  independent verification. Verification-only; never edits files.
```

- [ ] **Step 1.14: Verify all 11 descriptions pass the shape checks**

Run:

```bash
for f in \
  plugins/bug-fix/skills/bug-fix/SKILL.md \
  plugins/feature-dev/skills/feature-dev/SKILL.md \
  plugins/code-review/skills/code-review/SKILL.md \
  plugins/spec-review/skills/spec-review/SKILL.md \
  plugins/monkey-test/skills/monkey-test/SKILL.md \
  plugins/test-scenarios/skills/test-scenarios/SKILL.md \
  plugins/belt-agents/agents/code-architect.md \
  plugins/belt-agents/agents/code-explorer.md \
  plugins/belt-agents/agents/impact-analyzer.md \
  plugins/belt-agents/agents/feature-implementer.md \
  plugins/belt-agents/agents/phase-auditor.md; do
  echo "=== $f ==="
  awk '/^---$/{c++; next} c==1' "$f" | awk '/^description:/,/^[a-z_-]+:/' | head -10
done
```

Expected: every description contains `Use when`; none contains `Phase 1`/`Phase 2`/`Phase 5`/`Phase 6` phrase referring to a sibling skill.

Then verify no Japanese remains in any frontmatter (first 10 lines of each file):

Run (use Grep tool):

```
pattern: [ぁ-んァ-ヶー一-龯]
path: plugins/bug-fix/skills/bug-fix/SKILL.md
output_mode: content
head_limit: 1
```

Repeat for the other 10 files. Expected: no matches in any of the 11 files' first 10 lines (frontmatter scope).

- [ ] **Step 1.15: Commit**

```bash
git add \
  plugins/bug-fix/skills/bug-fix/SKILL.md \
  plugins/feature-dev/skills/feature-dev/SKILL.md \
  plugins/code-review/skills/code-review/SKILL.md \
  plugins/spec-review/skills/spec-review/SKILL.md \
  plugins/monkey-test/skills/monkey-test/SKILL.md \
  plugins/test-scenarios/skills/test-scenarios/SKILL.md \
  plugins/belt-agents/agents/code-architect.md \
  plugins/belt-agents/agents/code-explorer.md \
  plugins/belt-agents/agents/impact-analyzer.md \
  plugins/belt-agents/agents/feature-implementer.md \
  plugins/belt-agents/agents/phase-auditor.md

git -c commit.gpgsign=false commit -m "docs(plugins): rewrite frontmatter descriptions per Anthropic best practices

Adopt the canonical '<capability>. Use when <trigger>. <flag>.' pattern
(target 20-35 words) across 11 skill/agent descriptions. Move phase-flow
enumeration from descriptions to new Pipeline Overview body sections in
bug-fix and feature-dev SKILL.md. Drop cross-plugin Phase-N hardcodes in
favor of phase-name references.

Refs: docs/specs/2026-04-16-plugins-english-frontmatter-design.md"
```

---

## Task 2: Translate `code-review/agents/code-reviewer.md` body to English

**Files:**
- Modify: `plugins/code-review/agents/code-reviewer.md` (body only; frontmatter already English)

This file has 139 Japanese occurrences across 7 observation sections. The structure is uniform across observations; use the Translation Style Guide glossary and stock phrases.

- [ ] **Step 2.1: Apply stock-phrase translations to `## Filtering` section (lines 16-22)**

Replace the 4 bullets under `## Filtering (applies to all observations)` with their stock-phrase translations from the Style Guide (see "Stock phrase translations" table).

- [ ] **Step 2.2: Translate Observation 1 (Quality) body**

Sections to translate (lines 27-55):
- `### Review Checklist` items 1-8 (line 29-36): translate each bullet. Examples:
  - `1. **Duplication** — 同一ロジックの繰り返し、コピペコード` →
    `1. **Duplication** — Repeated identical logic, copy-pasted code`
  - `2. **Anti-patterns** — God object, shotgun surgery, feature envy, primitive obsession` → already English, leave as-is
  - `3. **Convention violations** — プロジェクトの CLAUDE.md に定義された規約違反` →
    `3. **Convention violations** — Violations of conventions defined in the project's CLAUDE.md`
  - `4. **Naming** — 命名規約違反（camelCase/snake_case の混在、曖昧な名前）` →
    `4. **Naming** — Naming convention violations (mixed camelCase/snake_case, ambiguous names)`
  - `5. **Consistency** — 既存コードベースのパターンとの不整合` →
    `5. **Consistency** — Mismatches with existing codebase patterns`
  - `6. **Structural complexity** — 関数 >50行、ファイル >800行、ネスト >4レベル` →
    `6. **Structural complexity** — Functions >50 lines, files >800 lines, nesting >4 levels`
  - `7. **Debug artifacts** — console.log, print, debugger 文の残存` →
    `7. **Debug artifacts** — Leftover console.log, print, or debugger statements`
  - `8. **Untracked TODO** — TODO/FIXME にイシュー番号・チケット参照がないもの` →
    `8. **Untracked TODO** — TODO/FIXME lines without an issue number or ticket reference`
- `### Policy` preamble (line 40): use stock-phrase translation
- REJECT criteria (lines 43-46): translate each bullet
- WARNING criteria (lines 48-51): translate each bullet
- Closing paragraph (lines 53-55): use stock-phrase translation

- [ ] **Step 2.3: Translate Observation 2 (Security) body**

Apply the same pattern to lines 57-112 (`## Observation 2: Security`):
- Translate `### Filtering` preamble, the "False Positive" bullets (lines 64-68), and closing instruction (line 69)
- Translate Review Checklist items 2, 3, 4, 5, 7, 8, 9, 10, 11 (items 1, 6 are already English-heavy)
- Translate `### Principles` bullets
- Translate `### Policy` section (REJECT + WARNING criteria and closing paragraph) using stock phrases

Sample translations:
- `2. **Authentication/Authorization** — 認証チェック漏れ、権限昇格の可能性、平文パスワード比較、脆弱なハッシュアルゴリズム` →
  `2. **Authentication/Authorization** — Missing authentication checks, privilege escalation paths, plaintext password comparison, weak hash algorithms`
- `3. **Secret leakage** — ハードコードされた API キー、トークン、パスワード` →
  `3. **Secret leakage** — Hardcoded API keys, tokens, or passwords`
- `5. **Data exposure** — ログへの機密情報出力、エラーメッセージでの内部情報漏洩` →
  `5. **Data exposure** — Sensitive data written to logs; internal details leaked in error messages`

- [ ] **Step 2.4: Translate Observation 3 (Performance) body**

Apply the pattern to lines 114-149. Translate Review Checklist items 1-7 and the Policy section using stock phrases.

Sample:
- `1. **N+1 queries** — ループ内のDB/APIクエリ、eager loading の欠如` →
  `1. **N+1 queries** — Database or API calls inside loops; missing eager loading`

- [ ] **Step 2.5: Translate Observation 4 (Test) body**

Apply the pattern to lines 151-194. Translate Review Checklist items 1-7 and Policy.

Sample:
- `1. **Coverage gaps** — 変更された実装コードに対するテストが存在するか。新規関数・分岐にテストがあるか` →
  `1. **Coverage gaps** — Whether tests cover changed implementation code; whether new functions and branches have tests`

- [ ] **Step 2.6: Translate Observation 5 (AI-antipattern) body**

Apply the pattern to lines 196-234. Translate Review Checklist items 1-9, Policy section, and the `### Self-bias check` paragraph (line 234):

```
あなたの判定が「問題ない」方向に偏っていないか常に自己検証せよ。AI が生成したコードを AI がレビューする構造上、同じバイアスを共有するリスクがある。「なぜこのコードが正しいか」ではなく「このコードが間違っている可能性はないか」の視点でレビューせよ。
```

→

```
Always self-check whether your verdict is biased toward "no issue." When AI reviews AI-generated code, there is a structural risk of sharing the same bias. Review from the angle of "might this code be wrong?" rather than "why is this code correct?"
```

- [ ] **Step 2.7: Translate Observation 6 (Impact) body**

Apply the pattern to lines 236-279. Translate the Policy section using stock phrases.

- [ ] **Step 2.8: Translate Observation 7 (Simplification) body**

Apply the pattern to lines 281-300. Translate Review Checklist items 1-3 and Policy.

Sample:
- `1. **Reuse** — 既存の関数・ユーティリティで置き換え可能な自作ロジックがないか` →
  `1. **Reuse** — Custom logic that could be replaced by existing functions or utilities`
- `同一パターンの問題を他観点 (Quality / Performance) で既に報告済みなら、この観点では再度報告しない。` →
  `If the same pattern was already reported under another observation (Quality / Performance), do not re-report it here.`

- [ ] **Step 2.9: Verify no Japanese remains**

Run (Grep tool):

```
pattern: [ぁ-んァ-ヶー一-龯]
path: plugins/code-review/agents/code-reviewer.md
output_mode: count
```

Expected: 0 matches.

- [ ] **Step 2.10: Commit**

```bash
git add plugins/code-review/agents/code-reviewer.md

git -c commit.gpgsign=false commit -m "docs(plugins): translate code-reviewer agent body to English

Translates all 7 observation bodies (quality, security, performance, test,
ai-antipattern, impact, simplification) using the per-spec glossary and
stock-phrase translations. Frontmatter already English; no structural
changes.

Refs: docs/specs/2026-04-16-plugins-english-frontmatter-design.md"
```

---

## Task 3: Translate `spec-review/agents/spec-reviewer.md` body to English

**Files:**
- Modify: `plugins/spec-review/agents/spec-reviewer.md` (body only; frontmatter already English)

81 Japanese occurrences across 5 observations. Structure mirrors code-reviewer.md — same stock phrases apply.

- [ ] **Step 3.1: Translate `## Filtering` section (lines 14-18)**

3 bullets — use stock-phrase translations from the Style Guide.

- [ ] **Step 3.2: Translate Observation 1 (Requirements) body**

Lines 20-48. Translate:
- `### Review Checklist` items 1 and 2 (lines 26-27)
- `### Investigation Method` bullets (lines 31-32)
- `### Policy` REJECT/WARNING criteria and closing paragraph

Sample:
- `1. **Requirements clarity** — 要件・ゴールが実装可能かつ検証可能なレベルまで具体化されているか。「適切に処理する」「パフォーマンスを改善する」のような曖昧な要件がないか。具体的な数値・条件・振る舞いが定義されているか` →
  `1. **Requirements clarity** — Whether requirements and goals are concrete enough to be implementable and verifiable. Watch for vague phrasing like "handle appropriately" or "improve performance." Concrete numbers, conditions, and behaviors must be defined.`

- [ ] **Step 3.3: Translate Observation 2 (Design judgment) body**

Lines 50-73. Same pattern.

- [ ] **Step 3.4: Translate Observation 3 (Feasibility) body**

Lines 75-102. Same pattern.

- [ ] **Step 3.5: Translate Observation 4 (Consistency) body**

Lines 104-136. Same pattern. Item 7 (Impact Analysis section completeness, line 116) is particularly long — translate carefully:

```
7. **Impact Analysis section completeness** — 設計書に Impact Analysis セクション（Reverse Dependencies, Shared State, Implicit Contracts, Side Effect Risks）が存在し、各項目が具体的に記述されているか。抽象的な記述（「他モジュールに影響する可能性がある」等）ではなく、具体的なファイル:行番号・リソース名・シナリオが含まれているか。Must-Verify Checklist が存在し、実装・テスト時に検証可能な具体的項目が列挙されているか。各項目について実際にコードを Grep/Read して記述の正確性を検証する。前提条件セクションと Implicit Contracts に矛盾がないか確認する
```

→

```
7. **Impact Analysis section completeness** — Whether the spec includes an Impact Analysis section (Reverse Dependencies, Shared State, Implicit Contracts, Side Effect Risks) with each item described concretely. Entries must include specific file:line references, resource names, and scenarios — not abstract phrases like "may affect other modules." A Must-Verify Checklist must exist and enumerate items that are verifiable during implementation and testing. Use Grep/Read against the code to confirm each item's accuracy. Verify the Assumptions section does not contradict Implicit Contracts.
```

- [ ] **Step 3.6: Translate Observation 5 (UI design) body**

Lines 138-169. Same pattern.

- [ ] **Step 3.7: Verify no Japanese remains**

Run (Grep tool):

```
pattern: [ぁ-んァ-ヶー一-龯]
path: plugins/spec-review/agents/spec-reviewer.md
output_mode: count
```

Expected: 0 matches.

- [ ] **Step 3.8: Commit**

```bash
git add plugins/spec-review/agents/spec-reviewer.md

git -c commit.gpgsign=false commit -m "docs(plugins): translate spec-reviewer agent body to English

Translates all 5 observation bodies (requirements, design-judgment,
feasibility, consistency, ui-design) using the per-spec glossary and
stock phrases shared with code-reviewer.

Refs: docs/specs/2026-04-16-plugins-english-frontmatter-design.md"
```

---

## Task 4: Translate `phase-auditor.md` + `impact-analyzer.md` bodies

**Files:**
- Modify: `plugins/belt-agents/agents/phase-auditor.md` (body only; frontmatter done in Task 1)
- Modify: `plugins/belt-agents/agents/impact-analyzer.md` (body only; frontmatter done in Task 1)

- [ ] **Step 4.1: Translate `phase-auditor.md` body — Step 2b and Step 6.5 are the main Japanese sections**

Japanese content lives in:
- `### Step 2b: Deferred Impact Verification` (lines 63-68)
- `### Step 6.5: Observation Collection` (lines 100-112)
- `## Verdict Rules` final bullet (line 188)

Translations:

Step 2b (lines 63-68):

```
### Step 2b: Deferred Impact Verification
Evidence Plan に E-DEFERRED-IMPACT が有効で、かつ review-fix アクティビティの場合:
1. レビュー結果から deferred な impact findings を抽出する
2. E-DEFERRED-IMPACT の claimed ファイルを読み取る
3. 各延期 finding について、検証結果が「不一致」であれば severity: blocker の動的基準として追加する
4. claimed ファイルが存在しない場合、blocker FAIL（collection 漏れ）として報告する
```

→

```
### Step 2b: Deferred Impact Verification
When E-DEFERRED-IMPACT is enabled in the Evidence Plan and the activity is review-fix:
1. Extract the deferred impact findings from the review results.
2. Read the file claimed by E-DEFERRED-IMPACT.
3. For each deferred finding, if the verification result does not match, add it as a dynamic severity: blocker criterion.
4. If the claimed file does not exist, report a blocker FAIL (missed collection).
```

Step 6.5 (lines 100-112):

```
### Step 6.5: Observation Collection

Verdict 出力時に `observations[]` を必ず含めること。以下のルールに従う:

1. **PASS だが懸念あり**: `criteria_results[].status` が PASS でも、diagnosis に改善余地がある場合 → `observations[]` に記録
2. **FAIL → Fix → PASS**: 修正で PASS になったが根本的な設計懸念が残る場合 → observation として残す
3. **severity**:
   - `quality`: 動作するが品質上の改善余地（テスト薄い、カバー間接的）
   - `warning`: 下流フェーズで問題化するリスク（トレーサビリティ弱い、境界条件未検証）
4. `observations` が空の場合は `"observations": []`（フィールド常時出力）
5. フェーズあたり最大5件。超過時は severity: warning を quality より優先して保持

オーケストレーターがこの `observations` を `project-state.json` の `phase_observations[]` に蓄積する。phase-auditor 自身は project-state.json を直接書き換えない。
```

→

```
### Step 6.5: Observation Collection

Always include `observations[]` in the verdict output, following these rules:

1. **PASS with concerns**: If `criteria_results[].status` is PASS but the diagnosis notes room for improvement, record it in `observations[]`.
2. **FAIL → Fix → PASS**: If a fix resulted in PASS but a fundamental design concern remains, keep it as an observation.
3. **severity**:
   - `quality`: Works but has room for improvement (thin tests, indirect coverage).
   - `warning`: Risk that will surface in downstream phases (weak traceability, unverified boundary conditions).
4. When there are no observations, emit `"observations": []` — always include the field.
5. Maximum five per phase. When over the limit, keep severity: warning entries in preference to quality entries.

The orchestrator accumulates `observations` into `phase_observations[]` in `project-state.json`. The phase-auditor itself never writes to `project-state.json` directly.
```

Verdict Rules final bullet (line 188):

```
- `observations`: verdict に影響しない。PASS/FAIL いずれの場合も、品質所見があれば記録する。
```

→

```
- `observations`: Do not affect the verdict. Record quality observations regardless of PASS or FAIL.
```

- [ ] **Step 4.2: Translate `impact-analyzer.md` body Japanese portions**

Read the file first to locate Japanese occurrences (4 body occurrences). Translate each inline, preserving structure and code blocks. Apply the glossary.

- [ ] **Step 4.3: Verify no Japanese remains in either file**

Run (Grep tool) for each of the two files:

```
pattern: [ぁ-んァ-ヶー一-龯]
path: plugins/belt-agents/agents/phase-auditor.md
output_mode: count
```

```
pattern: [ぁ-んァ-ヶー一-龯]
path: plugins/belt-agents/agents/impact-analyzer.md
output_mode: count
```

Expected: 0 matches in each.

- [ ] **Step 4.4: Commit**

```bash
git add \
  plugins/belt-agents/agents/phase-auditor.md \
  plugins/belt-agents/agents/impact-analyzer.md

git -c commit.gpgsign=false commit -m "docs(plugins): translate phase-auditor and impact-analyzer bodies

Translates Step 2b (Deferred Impact Verification), Step 6.5 (Observation
Collection), and Verdict Rules tail in phase-auditor, plus residual
Japanese in impact-analyzer body.

Refs: docs/specs/2026-04-16-plugins-english-frontmatter-design.md"
```

---

## Task 5: Translate `belt-agents/references/evidence-catalog.md`

**Files:**
- Modify: `plugins/belt-agents/references/evidence-catalog.md`

83 Japanese occurrences across 218 lines. This is the largest references file.

- [ ] **Step 5.1: Read the full file**

Use Read tool on `plugins/belt-agents/references/evidence-catalog.md` (218 lines — fits in one read).

- [ ] **Step 5.2: Translate section by section**

For each Japanese line or phrase:
1. Apply the glossary
2. Preserve all YAML / JSON / code blocks exactly
3. Keep IDs (`E-IMPL-01`, etc.) and severity labels as-is
4. Keep frontmatter structure unchanged (translate only the Japanese text within value fields)

Special attention:
- Section headings with Japanese: translate the heading and any introductory paragraph
- Inline comments in code blocks that are Japanese: translate them
- YAML `claimed:`, `verified:`, `required_capabilities:`, `condition:` field descriptions: translate prose values; keep keys and structural markers

- [ ] **Step 5.3: Verify no Japanese remains**

Run (Grep tool):

```
pattern: [ぁ-んァ-ヶー一-龯]
path: plugins/belt-agents/references/evidence-catalog.md
output_mode: count
```

Expected: 0 matches.

- [ ] **Step 5.4: Commit**

```bash
git add plugins/belt-agents/references/evidence-catalog.md

git -c commit.gpgsign=false commit -m "docs(plugins): translate evidence-catalog reference to English

Refs: docs/specs/2026-04-16-plugins-english-frontmatter-design.md"
```

---

## Task 6: Translate `belt-agents/references/narrative-convention.md`

**Files:**
- Modify: `plugins/belt-agents/references/narrative-convention.md`

33 Japanese occurrences across 99 lines.

- [ ] **Step 6.1: Translate the body**

Apply glossary. Key sections:

Line 1-7 (Purpose):

```
# Narrative Note Convention

Phase-scoped narrative note の規約。`feature-dev` / `bug-fix` の narrative-producing phase から produce される。belt は content を parse しないため、本 convention は SKILL 層の責務。

## Purpose

User が `/clear` で session context をリセットした後、narrative note を読むことで各 phase の判断・懸念・指示・観察を復元できるようにする。domain artifact（`design.md` / `plan.md` / `rca-report.md` 等）が **何を作ったか** を記録するのに対し、narrative note は **なぜそう判断したか / 何が未解決か / 次 phase が守るべき前提は何か** を記録する。
```

→

```
# Narrative Note Convention

Convention for phase-scoped narrative notes produced by narrative-producing phases in `feature-dev` and `bug-fix`. belt does not parse note content, so this convention is owned by the SKILL layer.

## Purpose

After the user resets session context with `/clear`, reading the narrative note restores each phase's decisions, concerns, directives, and observations. Domain artifacts (`design.md`, `plan.md`, `rca-report.md`, etc.) record **what was produced**, while narrative notes record **why the call was made, what remains unresolved, and what the next phase must assume**.
```

Other sections (File Schema prose, Rules 1-5, Section guidance, Example comments) follow the same pattern. Translate all Japanese prose; preserve code blocks, YAML frontmatter examples, file path strings.

- [ ] **Step 6.2: Verify no Japanese remains**

Run (Grep tool):

```
pattern: [ぁ-んァ-ヶー一-龯]
path: plugins/belt-agents/references/narrative-convention.md
output_mode: count
```

Expected: 0 matches.

Note: Lines 83 (`Context reset mechanism は既存 belt-core narrative 機構 (2026-04-14 spec) を再利用する。...`) and surrounding example notes are inside a `## Example: feature-dev design phase` code block. Translate these too — they serve as examples and must be readable to an English-only reader.

- [ ] **Step 6.3: Commit**

```bash
git add plugins/belt-agents/references/narrative-convention.md

git -c commit.gpgsign=false commit -m "docs(plugins): translate narrative-convention reference to English

Refs: docs/specs/2026-04-16-plugins-english-frontmatter-design.md"
```

---

## Task 7: Translate `belt-agents/references/criteria-template.md`

**Files:**
- Modify: `plugins/belt-agents/references/criteria-template.md`

20 Japanese occurrences across 55 lines.

- [ ] **Step 7.1: Translate the body**

Key sections:

Line 3 (frontmatter description):

```yaml
description: done-criteria ファイルのテンプレートと品質ルール。新規 done-criteria 作成時に参照する。
```

→

```yaml
description: >-
  Template and quality rules for done-criteria files. Reference when creating a
  new done-criteria file.
```

YAML template block (lines 10-29): translate inline Japanese comments:

```
- **verification**:
  {Audit Agent が実行する具体的な手順。inspection の場合は番号付きステップ必須}
- **pass_condition**: {主観語禁止。数値閾値 or パターンマッチで判定可能な条件}
- **fail_diagnosis_hint**: {FAIL 時に何を調べれば解消するかの方向性}
- **depends_on_artifacts**: [{検証に必要な成果物パス}]
- **forward_check**: {次フェーズの入力として十分かの観点。省略可}
```

→

```
- **verification**:
  {Concrete steps for the Audit Agent to execute. For inspection type, numbered steps are required.}
- **pass_condition**: {No subjective terms. Use numeric thresholds or pattern matches that can be judged deterministically.}
- **fail_diagnosis_hint**: {On FAIL, what to investigate to resolve.}
- **depends_on_artifacts**: [{Paths to artifacts required for verification.}]
- **forward_check**: {Whether this is sufficient as input to the next phase. Optional.}
```

Template Rules (lines 31-40):

```
These rules are enforced by the template structure itself:

1. **pass_condition に主観語禁止**: 「適切」「十分」「具体的」「正しい」は使えない。数値閾値（例: "2件以上"）またはパターンマッチ（例: "ファイルパスが含まれる"）で記述する。
2. **inspection 型は番号付きステップ必須**: verification に "1. ... 2. ... 3. ..." の形式で判定手順を列挙する。
3. **fail_diagnosis_hint 必須**: FAIL 時に「何を調べれば解消するか」の方向性を必ず記述する。
4. **severity の使い分け**:
   - `blocker`: 未達なら必ず FAIL。修正→再監査のリトライ対象。
   - `quality`: 未達でも全 blocker が PASS なら警告のみで通過可能。リトライ対象にしない。
```

→

```
These rules are enforced by the template structure itself:

1. **No subjective terms in `pass_condition`**: Words like "appropriate", "sufficient", "concrete", or "correct" are disallowed. Use numeric thresholds (e.g., "2 or more") or pattern matches (e.g., "contains a file path").
2. **Inspection type requires numbered steps**: Enumerate the decision procedure in `verification` as "1. ... 2. ... 3. ...".
3. **`fail_diagnosis_hint` is required**: On FAIL, always state what to investigate to resolve the failure.
4. **Severity semantics**:
   - `blocker`: If unmet, the phase must FAIL. Eligible for the fix-and-re-audit retry loop.
   - `quality`: If unmet while every blocker PASSes, the phase passes with a warning only. Not eligible for the retry loop.
```

Human Review (lines 42-50):

```
## Human Review: 3-Point Scan

done-criteria ファイルの作成・変更時、以下の3点を確認:

1. **pass_condition を読んで、PASS/FAIL が自分でも判断できるか？** → できなければ曖昧
2. **blocker 基準が多すぎないか？** → 全部 blocker だとリトライ地獄になる
3. **このフェーズで本当に検証すべきことが漏れていないか？** → カバレッジ

所要時間の目安: 1ファイルあたり2-3分。
```

→

```
## Human Review: 3-Point Scan

When creating or modifying a done-criteria file, check these three points:

1. **Can you judge PASS/FAIL yourself by reading `pass_condition`?** — If not, it is ambiguous.
2. **Are there too many blocker criteria?** — Making everything a blocker causes retry hell.
3. **Is any truly necessary check for this phase missing?** — Coverage.

Expected time: 2-3 minutes per file.
```

ID Convention (lines 52-55):

```
## ID Convention

- Phase N の基準: `DN-01`, `DN-02`, ...
- Evidence-derived 基準（動的合成）: `DN-E1`, `DN-E2`, ...
```

→

```
## ID Convention

- Phase N criteria: `DN-01`, `DN-02`, ...
- Evidence-derived criteria (synthesized dynamically): `DN-E1`, `DN-E2`, ...
```

- [ ] **Step 7.2: Verify no Japanese remains**

Run (Grep tool):

```
pattern: [ぁ-んァ-ヶー一-龯]
path: plugins/belt-agents/references/criteria-template.md
output_mode: count
```

Expected: 0 matches.

- [ ] **Step 7.3: Commit**

```bash
git add plugins/belt-agents/references/criteria-template.md

git -c commit.gpgsign=false commit -m "docs(plugins): translate criteria-template reference to English

Refs: docs/specs/2026-04-16-plugins-english-frontmatter-design.md"
```

---

## Task 8: Translate `bug-fix/SKILL.md` + `feature-dev/SKILL.md` bodies

**Files:**
- Modify: `plugins/bug-fix/skills/bug-fix/SKILL.md` (body only; description + Pipeline Overview done in Task 1)
- Modify: `plugins/feature-dev/skills/feature-dev/SKILL.md` (body only)

15 + 5 Japanese occurrences. Both have similar Narrative Notes + Red Flags + References sections.

- [ ] **Step 8.1: Translate `bug-fix/SKILL.md` body Japanese regions**

After Task 1, the frontmatter and Pipeline Overview are English. Remaining Japanese:

Phase 3 note (lines 40-42):

```
- Note: spec-review を fix-plan レビューに流用する。`design-judgment` 観点の
  grill-me は原則発動しない (設計判断は rca / fix-plan で決定済みのため)。
  発動した場合は上流 (rca / fix-plan) の見直しサインとして扱う。
```

→

```
- Note: spec-review is reused for fix-plan review. The grill-me prompt under
  the `design-judgment` observation does not fire by default (design decisions
  are already settled in rca / fix-plan). If it does fire, treat it as a signal
  that upstream phases (rca / fix-plan) need to be revisited.
```

Narrative Notes section (lines 72-83):

```
## Narrative Notes

以下 6 phase は `/clear` 後の context 復元のため narrative note を produce する (`.belt/runs/{run_id}/notes/phase-<id>.md`):

- **rca** / **fix-plan** / **execute** / **code-review**
- **monkey-test** (`--e2e`) / **dogfood** (`--e2e`)

各 note は 4 section (`## Decisions` / `## Concerns` / `## Directives` / `## Observations`) と minimal frontmatter (`phase`, `run_id`) を含む。

規約詳細: [`plugins/belt-agents/references/narrative-convention.md`](plugins/belt-agents/references/narrative-convention.md)

`/clear` 自体は user 判断（Claude Code runtime 制約で自動化不可）。重い phase 完了直後（例: rca / execute 後）に context が膨れた場合の選択肢として narrative を活用できる。
```

→

```
## Narrative Notes

The following six phases produce a narrative note so context can be restored after `/clear` (`.belt/runs/{run_id}/notes/phase-<id>.md`):

- **rca** / **fix-plan** / **execute** / **code-review**
- **monkey-test** (`--e2e`) / **dogfood** (`--e2e`)

Each note contains four sections (`## Decisions` / `## Concerns` / `## Directives` / `## Observations`) and minimal frontmatter (`phase`, `run_id`).

Full convention: [`plugins/belt-agents/references/narrative-convention.md`](plugins/belt-agents/references/narrative-convention.md)

`/clear` itself is the user's call — Claude Code runtime constraints prevent automation. Use narrative notes as an option when context has grown large after a heavy phase (for example, right after rca or execute).
```

Red Flags (line 88):

```
- **Never skip Phase 1 / 2 / 6 / 7 / 8 の supplement load**: bug-fix 固有 override が inject されず drift 発生.
```

→

```
- **Never skip the supplement load in Phases 1 / 2 / 6 / 7 / 8**: without bug-fix specific overrides injected, behavior drifts.
```

Red Flags (line 89):

```
- **Never delegate root cause synthesis to subagents**: parallel exploration results は orchestrator が再構築.
```

→

```
- **Never delegate root cause synthesis to subagents**: the orchestrator must reconstruct parallel exploration results.
```

Red Flags (line 91):

```
- **Never filter or omit review findings**: `/code-review:code-review`, `/spec-review:spec-review` の triage は user 責務.
```

→

```
- **Never filter or omit review findings**: triage of `/code-review:code-review` and `/spec-review:spec-review` output is the user's responsibility.
```

Red Flags (line 92):

```
- **Never bypass the Phase 8 A/B choice**: merge-vs-PR は user 決定.
```

→

```
- **Never bypass the Phase 8 A/B choice**: the merge-vs-PR decision is always the user's.
```

Red Flags (line 93):

```
- **Never hand-edit files under `docs/plans/<topic>-*`**: phase-produced; manual edits break belt の phase-start mtime filter.
```

→

```
- **Never hand-edit files under `docs/plans/<topic>-*`**: they are phase-produced; manual edits break belt's phase-start mtime filter.
```

Red Flags (line 94):

```
- **Never modify the consumed global skills**: override は `references/*-supplement.md` 経由のみ.
```

→

```
- **Never modify the consumed global skills**: overrides go through `references/*-supplement.md` only.
```

Red Flags (line 95):

```
- **Never leave narrative note 4 sections blank**: gate は file_exists のみで空 section も通過するが、下流 consume で context 復元不能になる。最低限 `(none)` placeholder を置き、heading は必ず保持。
```

→

```
- **Never leave the narrative note's four sections blank**: the gate is `file_exists` only and empty sections still pass, but downstream consumers cannot restore context. Use at least `(none)` as a placeholder and always keep the heading.
```

References (lines 99, 103, 105):

```
- `./references/path-convention.md` — `docs/plans/YYYY-MM-DD-<topic>-*` 命名 SSOT
...
- `./references/dogfood-supplement.md` — Phase 7 override and CLI-only degradation
...
- `plugins/belt-agents/references/narrative-convention.md` — narrative note schema (shared with feature-dev)
```

→

```
- `./references/path-convention.md` — SSOT for `docs/plans/YYYY-MM-DD-<topic>-*` naming
...
- `./references/dogfood-supplement.md` — Phase 7 override and CLI-only degradation
...
- `plugins/belt-agents/references/narrative-convention.md` — narrative note schema (shared with feature-dev)
```

(Line 103 is already mostly English; verify no JP left.)

- [ ] **Step 8.2: Translate `feature-dev/SKILL.md` body Japanese regions**

Read the file to locate the 5 Japanese occurrences. Apply the same translation patterns (Narrative Notes / Red Flags / References sections mirror `bug-fix/SKILL.md`).

- [ ] **Step 8.3: Verify no Japanese remains in either file**

Run (Grep tool) for each:

```
pattern: [ぁ-んァ-ヶー一-龯]
path: plugins/bug-fix/skills/bug-fix/SKILL.md
output_mode: count
```

```
pattern: [ぁ-んァ-ヶー一-龯]
path: plugins/feature-dev/skills/feature-dev/SKILL.md
output_mode: count
```

Expected: 0 matches in each.

- [ ] **Step 8.4: Commit**

```bash
git add \
  plugins/bug-fix/skills/bug-fix/SKILL.md \
  plugins/feature-dev/skills/feature-dev/SKILL.md

git -c commit.gpgsign=false commit -m "docs(plugins): translate SKILL.md bodies for bug-fix and feature-dev

Translates Narrative Notes, Red Flags, and References sections. Frontmatter
and Pipeline Overview were updated in the earlier description-rewrite
commit.

Refs: docs/specs/2026-04-16-plugins-english-frontmatter-design.md"
```

---

## Task 9: Translate remaining small files

**Files:**
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/spec-review.md` (12 occurrences)
- Modify: `plugins/bug-fix/skills/bug-fix/references/fix-plan-supplement.md` (1 occurrence)
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/dogfood.md` (1 occurrence)

- [ ] **Step 9.1: Translate `feature-dev/criteria/spec-review.md`**

Read the file. Translate each of the 12 Japanese occurrences in place, applying the glossary. Preserve front-matter keys, IDs, and `pass_condition` / `verification` structure.

- [ ] **Step 9.2: Translate `bug-fix/references/fix-plan-supplement.md`**

Read the file. Translate the single Japanese occurrence in place.

- [ ] **Step 9.3: Translate `bug-fix/criteria/dogfood.md`**

Read the file. Translate the single Japanese occurrence in place.

- [ ] **Step 9.4: Verify no Japanese remains in any of the three files**

Run (Grep tool) for each:

```
pattern: [ぁ-んァ-ヶー一-龯]
path: plugins/feature-dev/skills/feature-dev/criteria/spec-review.md
output_mode: count
```

```
pattern: [ぁ-んァ-ヶー一-龯]
path: plugins/bug-fix/skills/bug-fix/references/fix-plan-supplement.md
output_mode: count
```

```
pattern: [ぁ-んァ-ヶー一-龯]
path: plugins/bug-fix/skills/bug-fix/criteria/dogfood.md
output_mode: count
```

Expected: 0 matches in each.

- [ ] **Step 9.5: Commit**

```bash
git add \
  plugins/feature-dev/skills/feature-dev/criteria/spec-review.md \
  plugins/bug-fix/skills/bug-fix/references/fix-plan-supplement.md \
  plugins/bug-fix/skills/bug-fix/criteria/dogfood.md

git -c commit.gpgsign=false commit -m "docs(plugins): translate remaining criteria and supplement bodies

Refs: docs/specs/2026-04-16-plugins-english-frontmatter-design.md"
```

---

## Task 10: Final verification

**No file modifications.** This task confirms the whole change set satisfies the spec's Definition of Done.

- [ ] **Step 10.1: Zero Japanese check across all of `plugins/`**

Run (Grep tool):

```
pattern: [ぁ-んァ-ヶー一-龯]
path: plugins
output_mode: count
```

Expected: 0 matches total.

- [ ] **Step 10.2: `Use when` presence check on all 11 descriptions**

Run:

```bash
for f in \
  plugins/bug-fix/skills/bug-fix/SKILL.md \
  plugins/feature-dev/skills/feature-dev/SKILL.md \
  plugins/code-review/skills/code-review/SKILL.md \
  plugins/spec-review/skills/spec-review/SKILL.md \
  plugins/monkey-test/skills/monkey-test/SKILL.md \
  plugins/test-scenarios/skills/test-scenarios/SKILL.md \
  plugins/belt-agents/agents/code-architect.md \
  plugins/belt-agents/agents/code-explorer.md \
  plugins/belt-agents/agents/impact-analyzer.md \
  plugins/belt-agents/agents/feature-implementer.md \
  plugins/belt-agents/agents/phase-auditor.md; do
  if ! awk '/^---$/{c++; next} c==1' "$f" | grep -q "Use when"; then
    echo "FAIL: $f missing 'Use when'"
  fi
done
echo "-- Use when check done --"
```

Expected: no FAIL lines.

- [ ] **Step 10.3: No first-person / second-person in descriptions**

Run:

```bash
for f in \
  plugins/bug-fix/skills/bug-fix/SKILL.md \
  plugins/feature-dev/skills/feature-dev/SKILL.md \
  plugins/code-review/skills/code-review/SKILL.md \
  plugins/spec-review/skills/spec-review/SKILL.md \
  plugins/monkey-test/skills/monkey-test/SKILL.md \
  plugins/test-scenarios/skills/test-scenarios/SKILL.md \
  plugins/belt-agents/agents/code-architect.md \
  plugins/belt-agents/agents/code-explorer.md \
  plugins/belt-agents/agents/impact-analyzer.md \
  plugins/belt-agents/agents/feature-implementer.md \
  plugins/belt-agents/agents/phase-auditor.md; do
  desc=$(awk '/^---$/{c++; next} c==1' "$f")
  if echo "$desc" | grep -Eq '\b(I|You|We|My|Your|Our)\b'; then
    echo "FAIL: $f uses first/second person"
  fi
done
echo "-- Person check done --"
```

Expected: no FAIL lines.

- [ ] **Step 10.4: No cross-plugin `Phase N` hardcode in descriptions**

Run:

```bash
for f in \
  plugins/bug-fix/skills/bug-fix/SKILL.md \
  plugins/feature-dev/skills/feature-dev/SKILL.md \
  plugins/code-review/skills/code-review/SKILL.md \
  plugins/spec-review/skills/spec-review/SKILL.md \
  plugins/monkey-test/skills/monkey-test/SKILL.md \
  plugins/test-scenarios/skills/test-scenarios/SKILL.md \
  plugins/belt-agents/agents/code-architect.md \
  plugins/belt-agents/agents/code-explorer.md \
  plugins/belt-agents/agents/impact-analyzer.md \
  plugins/belt-agents/agents/feature-implementer.md \
  plugins/belt-agents/agents/phase-auditor.md; do
  desc=$(awk '/^---$/{c++; next} c==1' "$f")
  if echo "$desc" | grep -Eq 'Phase [0-9]'; then
    echo "FAIL: $f references hardcoded Phase number"
  fi
done
echo "-- Phase-N check done --"
```

Expected: no FAIL lines.

- [ ] **Step 10.5: Word count range check (20-35)**

Run:

```bash
for f in \
  plugins/bug-fix/skills/bug-fix/SKILL.md \
  plugins/feature-dev/skills/feature-dev/SKILL.md \
  plugins/code-review/skills/code-review/SKILL.md \
  plugins/spec-review/skills/spec-review/SKILL.md \
  plugins/monkey-test/skills/monkey-test/SKILL.md \
  plugins/test-scenarios/skills/test-scenarios/SKILL.md \
  plugins/belt-agents/agents/code-architect.md \
  plugins/belt-agents/agents/code-explorer.md \
  plugins/belt-agents/agents/impact-analyzer.md \
  plugins/belt-agents/agents/feature-implementer.md \
  plugins/belt-agents/agents/phase-auditor.md; do
  desc=$(awk '/^---$/{c++; next} c==1' "$f" | sed -n '/^description:/,/^[a-z_-]\+:/p' | sed 's/^description: *>-//' | sed 's/^[a-z_-]\+:.*$//')
  wc=$(echo "$desc" | wc -w | tr -d ' ')
  echo "$f: $wc words"
done
```

Expected: every count between 20 and 35 inclusive. Flag any outliers and adjust the relevant description if needed (rare — all target drafts in the spec are in-range).

- [ ] **Step 10.6: SKILL.md body under 500 lines (regression check)**

Run:

```bash
wc -l plugins/*/skills/*/SKILL.md | awk '$1 >= 500 { print "FAIL:", $0 }'
echo "-- 500-line check done --"
```

Expected: no FAIL lines.

- [ ] **Step 10.7: Record verification results and push the branch**

Verification summary should confirm all of Steps 10.1–10.6 passed with no failures. If any failure surfaced, stop and open a fix task rather than pushing.

If all pass, the branch is ready for PR. Do not push automatically — leave the push / PR creation to the user unless explicitly instructed otherwise.

---

## Self-Review (already applied)

Spec coverage:
- Goal 1 (0 Japanese) covered by Tasks 2-9 + verification Step 10.1 ✓
- Goal 2 (11 descriptions canonical) covered by Task 1 + Steps 10.2-10.5 ✓
- Goal 3 (Phase-N removal) covered by Task 1 + Step 10.4 ✓
- Goal 4 (secondary rules) — 500-line confirmed in Step 10.6; nested refs and time-sensitive already green in audit, no task needed ✓

Placeholders: none (all descriptions and translation examples are concrete).

Type consistency: no types in scope. Filename and section heading references cross-checked between tasks.

Ambiguity: resolved — every translation task either provides exact target text (for descriptions and key stock phrases) or points to the glossary. For large files (code-reviewer, spec-reviewer, evidence-catalog), the glossary + stock phrases are sufficient for the executing agent to produce idiomatic output.

---

## Execution Handoff

Plan complete and saved to `docs/plans/2026-04-16-plugins-english-plus-frontmatter-plan.md`. Two execution options:

**1. Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — execute tasks in this session using executing-plans, batch with checkpoints.

Which approach?
