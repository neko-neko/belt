# Narrative Note Convention

Phase-scoped narrative note の規約。`feature-dev` / `bug-fix` の narrative-producing phase から produce される。belt は content を parse しないため、本 convention は SKILL 層の責務。

## Purpose

User が `/clear` で session context をリセットした後、narrative note を読むことで各 phase の判断・懸念・指示・観察を復元できるようにする。domain artifact（`design.md` / `plan.md` / `rca-report.md` 等）が **何を作ったか** を記録するのに対し、narrative note は **なぜそう判断したか / 何が未解決か / 次 phase が守るべき前提は何か** を記録する。

## Path

```
.belt/runs/{run_id}/notes/phase-{phase_id}.md
```

- `{run_id}` は belt-core が template 展開する（Engine が init 時に `<run_dir>/notes/` directory を作成）
- `{phase_id}` は pipeline.yml の `phases[].id` と同一（hyphen 保持: `monkey-test` → `phase-monkey-test.md`）
- Domain artifact (`docs/features/*`, `docs/plans/*`) とは別 directory

## File Schema

```markdown
---
phase: <phase_id>
run_id: <run_id>
---

## Decisions

<この phase で確定した設計判断・方針>

## Concerns

<未解決の懸念・リスク・下流で注意すべき事項>

## Directives

<次以降の phase への指示・前提条件>

## Observations

<事実記録・探索で判明した事項・テスト結果など>
```

## Rules

1. **Frontmatter は必須 2 field のみ**: `phase`, `run_id`。belt は parse しないが、下流 consumer (skill / LLM) が出自を追跡できるようにする。`run_id` は `belt-agent step` / `belt-agent status` 出力の `run_id` 値を LLM が書き写す
2. **4 section は全て必須**: `## Decisions` / `## Concerns` / `## Directives` / `## Observations`。空でも heading のみ残すこと（下流 consumer が section 欠落で混乱しないため）
3. **Section 順序は固定**: Decisions → Concerns → Directives → Observations
4. **各 section は簡潔に**: `/clear` 後の LLM が判断を再構成できる最低限の情報を含む。Domain artifact に書いた内容の複写は避ける（path 参照で十分）
5. **Code block / link は自由**: Markdown 規約内であれば自由。ただし冗長な再説明は避ける

## Section 別記入指針

### Decisions
- 何を決めたか。代替案を捨てた理由
- 下流 phase が「なぜその選択か」を問う時に答えになる情報
- 例: "NoSQL 候補を捨てて PostgreSQL 採用。理由: 既存 schema migration infra を流用できるため"

### Concerns
- 未解決の risk / 仮定 / 検証不足事項
- 下流が注意すべき時限爆弾
- 例: "monkey-test で E2E 未検証。dogfood 時に手動確認が必須"

### Directives
- 次以降の phase が守るべき制約・前提
- 例: "plan phase で task granularity を 30 分以下に保つこと（design で合意）"

### Observations
- 探索で判明した事実（特に domain artifact に書ききれないもの）
- 将来の調査で有用な context
- 例: "既存 `FooService` は actually Bar interface を実装していない（lint warn を確認）"

## Example: feature-dev design phase

```markdown
---
phase: design
run_id: 01947abc-1234-7890-def0-123456789abc
---

## Decisions

- Context reset mechanism は既存 belt-core narrative 機構 (2026-04-14 spec) を再利用する。belt-core への新規追加はしない
- narrative note は 6 phases のみで produce（軽量 phase は除外、user 合意済み）

## Concerns

- `/clear` は user 手動操作に依存。SKILL.md に "reset するタイミングの目安" を記述しない限り、note が参照されない可能性あり

## Directives

- plan phase: 実装タスクは 30 分以下の粒度に保つこと
- execute phase: narrative の Decisions を commit message に引用しないこと（noise になる）

## Observations

- narrative-convention.md は `plugins/belt-agents/references/` に既存 reference と並列配置
- plugin 移行で criteria は各 plugin に per-plugin 化済み（parity test で drift 検出）
```
