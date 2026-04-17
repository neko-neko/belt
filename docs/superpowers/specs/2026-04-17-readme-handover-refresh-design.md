---
title: README Refresh — Continuity 2 Axes with Handover/Resume Surfacing
date: 2026-04-17
status: draft
scope:
  - README.md
related:
  - docs/superpowers/specs/2026-04-16-readme-refresh-design.md
  - plugins/belt/skills/handover/SKILL.md
  - plugins/belt/skills/resume/SKILL.md
  - .claude-plugin/marketplace.json
---

# README Refresh — Continuity 2 Axes with Handover/Resume Surfacing

## 1. Overview

2026-04-17 に handover/resume skill (`/belt:handover`, `/belt:resume`) が
追加された (commits 59ce220, 4ecae43, 435fd04, f973a0b)。`marketplace.json`
は skill list を更新済みだが、`README.md` は 2026-04-16 refresh
(engine-first positioning) 時点のまま drift している。

本 spec は `README.md` の 3 箇所を対象に最小改定を行い、belt の continuity
モデルを「intra-run (handover/resume)」と「cross-run (`--inherits-from`)」
の 2 軸に再構成する。

## 2. Scope / Non-goals

**In-scope**:

- `README.md` Continuity セクション (126-157 行) の 2 階層再構成
- `README.md` Plugins テーブル (239 行) の skill list に
  `/belt:handover`, `/belt:resume` を末尾追加
- `README.md` Usage (284-288 行) を 2 ブロック分離
  (`# Start a pipeline` / `# Pause & resume an in-progress run`)

**Non-goals**:

- "Why belt?" セクションの書き換え (handover/resume を価値訴求の中核には
  出さない)
- Example / CLI / Install / Key Concepts / External skill dependencies 表
  の改変
- SKILL.md / pipeline.yml / CHANGELOG.md の改変
- `handover.md` schema や resume preconditions の README への詳述
  (SKILL.md に委譲)
- 日本語版 README 追加 (前 spec で不採用維持)
- `plugins/README.md` 新規作成 (前 spec 同上)

## 3. Decisions

| #  | Question                       | Decision                                                           | Rationale                                                                                   |
|----|--------------------------------|--------------------------------------------------------------------|---------------------------------------------------------------------------------------------|
| Q1 | handover/resume の配置         | Continuity セクション再構成 (新設でも追記でもなく)                 | 現状の Continuity は cross-run 偏重で粒度が不揃い。2 軸対比として統合するのが自然           |
| Q2 | Continuity 内部構造            | 2 階層 (cold-start 原則 → 2 レジューム機能対比)                    | フラット 4 項目より、原則と機能の階層が belt の実態 (cold-start 前提 + 付加レジューム) に即する |
| Q3 | 改定範囲                       | Continuity + Plugins + Usage の 3 箇所                             | marketplace.json との drift を同時解消。"Why belt?" 改定は overscope                        |
| Q4 | 対比の見せ方                   | 対比テーブル + 各 1 段落 + ワークフロー例                          | テーブルで差分一目、散文で補強。テーブルのみは使い分け判断に不親切                          |
| Q5 | handover フロー提示            | 3 ステップコードブロック (`/belt:handover` → `/clear` → `/belt:resume`) | 体験フローが handover の核心価値。ファイルパス等は実装詳細として SKILL.md に委譲              |

## 4. Target sections

### 4.1 Continuity セクション (126-157 行) — 再構成

**構造**:

1. 導入段落 — 既存の "Long LLM sessions accumulate context..." を少し
   短縮して流用。動機 (文脈汚染と /clear したい衝動) を提示
2. Part 1: **Cold-start principles** — 箇条書き 2 項目
   - Per-command neutrality (既存文面)
   - Narrative artifacts (既存文面)
3. Part 2: **Two resumption modes** — 以下 3 構成要素
   - 対比テーブル (When / What is carried / Command / Typical use)
   - Intra-run handover 段落 (3 ステップコードブロック付き)
   - Cross-run inheritance 段落 (既存 `--inherits-from` + `belt://` URI 3 種を保持)
4. 結びの一行

**対比テーブル (ドラフト)**:

```markdown
|                 | Intra-run handover                           | Cross-run inheritance                   |
|-----------------|----------------------------------------------|-----------------------------------------|
| When            | Same run, new session                        | New run, reads prior artifacts          |
| What is carried | Resume hint + existing state.json            | Gated artifacts via `belt://` URIs      |
| Command         | `/belt:handover` → `/clear` → `/belt:resume` | `belt-agent init --inherits-from <run>` |
| Typical use     | Context bloat mid-pipeline                   | Fresh run consumes prior conclusions    |
```

**Intra-run handover 段落 (ドラフト)**:

> When a pipeline run is mid-flight and the session's context has grown
> polluted, `/belt:handover` writes a short Resume hint (pause reason,
> first action, transient context) under the current run directory.
> After `/clear`, `/belt:resume` reads the hint and `state.json` and the
> next session picks up exactly where it left off:
>
> ```
> /belt:handover
> /clear
> /belt:resume
> ```
>
> The pipeline is never re-initialized; the resumed session continues the
> current phase with a fresh context but the same run.

**Cross-run inheritance 段落**:

既存の Cross-run inheritance 説明 (導入 1 文 + `belt://` URI 3 種 bullet +
"A typical use case: a long bug investigation..." の用例段落 +
`belt-agent init --inherits-from <prior-run-id>` コードブロック) を
そのまま保持し、Part 2 の 2 つ目として配置する。

**結びの一行 (ドラフト)**:

> Both are `/clear` that keeps what matters — handover keeps the run,
> inheritance keeps the conclusions.

(現行 "This is `/clear` that keeps the conclusions." は削除し、上記の
対比版で置換する。)

### 4.2 Plugins テーブル (239 行) — 末尾追加

現状:

```markdown
| `belt` | User-invocable pipelines and reviewer agents: ..., `/belt:monkey-test`, `/belt:test-scenarios`. Requires `belt-agent` |
```

改定後:

```markdown
| `belt` | User-invocable pipelines and reviewer agents: ..., `/belt:monkey-test`, `/belt:test-scenarios`, `/belt:handover`, `/belt:resume`. Requires `belt-agent` |
```

README の現行表記 `(4 observation reviewers)` / `(3 observation reviewers)`
は維持し、末尾に `/belt:handover`, `/belt:resume` のみを追加する。
`marketplace.json` 側は `(4 reviewers)` / `(3 reviewers)` と短く記述して
いるが、この括弧内注釈の表記差は本 spec のスコープ外 (必要なら別 PR で
一元化)。skill 列挙の有無の一致のみを本改定で確保する。

### 4.3 Usage (284-288 行) — 2 ブロック分離

現状:

```
/belt:feature-dev             # start a new feature
/belt:bug-fix                 # start a bug investigation
/belt:code-review             # standalone code review
/belt:spec-review             # standalone spec review
```

改定後:

```
# Start a pipeline
/belt:feature-dev
/belt:bug-fix
/belt:code-review
/belt:spec-review

# Pause & resume an in-progress run
/belt:handover
/belt:resume
```

行末コメント (`# start a new feature` 等) は削除し、見出しコメント 2 行
で役割を示す。

## 5. Verification

spec 実装後、以下がすべて満たされていることを確認する:

- [ ] Continuity セクションが 2 階層 (cold-start principles / Two resumption modes) になっている
- [ ] Continuity 内に対比テーブル (4 行: When / What / Command / Typical use) が存在する
- [ ] Continuity 内に 3 ステップコードブロック (`/belt:handover` → `/clear` → `/belt:resume`) が存在する
- [ ] Continuity の結びに 2 機能を対比する一行 (`handover keeps the run, inheritance keeps the conclusions`) が存在する
- [ ] Plugins テーブル belt 行末尾に `/belt:handover`, `/belt:resume` が追加されている
- [ ] Usage が 2 ブロック (`# Start a pipeline` / `# Pause & resume an in-progress run`) に分かれている
- [ ] "Why belt?" / Example / CLI / Install / Key Concepts / External skill dependencies 表が未変更
- [ ] `.belt/runs/<run_id>/handover.md` のフルパス言及が README に登場しない (SKILL.md 委譲)

## 6. Out-of-scope follow-ups

- CHANGELOG.md への handover/resume エントリ追加 (未確認、別途判断)
- plugins/README.md 新規作成 (前 spec Q7=C で不採用維持)
- 日本語版 README 追加 (同上)
- "Why belt?" への handover/resume 言及 (今回 overscope、必要が顕在化
  したら別 spec で扱う)
