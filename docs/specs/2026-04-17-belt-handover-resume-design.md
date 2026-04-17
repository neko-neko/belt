# belt Handover & Resume Skills — Design

**Status**: Draft
**Date**: 2026-04-17
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)

## Summary

belt pipeline (feature-dev / bug-fix / debug-flow 等の multi-phase pipeline) の実行中に session を跨いで pause / resume するための **2 skills + 1 reference + 2 pipeline 拡張** を追加する:

- **`/belt:handover`** (新 skill): 現 run の下に `handover.md` (Resume hint のみ) を書き出す
- **`/belt:resume`** (新 skill): current worktree の最新 run を検出し、`handover.md` と既存 `state.json` から context を復元、owning pipeline skill を resume mode で再 invoke
- **`belt-agent:protocol/references/resume-mode.md`** (新 reference): protocol driver が resume args を受けた時の手順 SSOT
- **feature-dev / bug-fix `pipeline.yml`**: execute phase の前に `pre-execute-handover` checkpoint phase を追加

現 `/handover` `/continue` (dotfiles) は session-level state のみ扱い、belt runtime state を保存しない。結果、`/continue` 後に belt が再開されず別 skill が誤発動する問題が起きている。本設計は belt 専用に並立させ、**新規 artifact を handover.md 1 本のみ** に最小化して解消する。

## Goals

- belt pipeline run を session を跨いで 2 コマンド (`/belt:handover` → 次 session で `/belt:resume`) で pause / resume
- pipeline 種別を問わず動作 (belt-agent は既に pipeline-agnostic)
- 既存 `/handover` `/continue` を壊さず並立、役割を明確分離
- pipeline 側で `pre-execute-handover` phase を導入し、heavy phase 前の context リセットを belt 状態機械で強制
- **新規 artifact を 1 本に限定し、既存 `state.json` との二重管理を避ける**
- Anthropic Agent Skills best practices に準拠

## Non-Goals

- session-level state (active_tasks, recent_decisions, resume_hint) の複製 — `/handover` と機能分離
- belt-agent CLI への `resume` subcommand 追加 — 当面 skill のみ (将来検討)
- cross-worktree resume サポート — 常に current worktree の `.belt/runs/` を見る前提、worktree 切替は user 責務
- global index ファイル (`~/.belt/state/...` 等) — `state.json` に既存情報と重複のため作らない
- `/belt:handover` / `/belt:resume` 概念の pipeline SKILL.md への浸透 — SSOT を新 skill 側に限定
- pipeline.yml への新 field 追加 — 既存 `confirm` + `file_exists` gate で checkpoint phase 表現

## Background

### 現 skill の gap

- **`/handover`** (dotfiles): `project-state.json` に session-level state を保存するが、belt 実行中検知なし、`.belt/runs/<id>/state.json` の `run_id` / `current_phase` / `pipeline_file` を保存しない
- **`/continue`** (dotfiles): `project-state.json` の `pipeline` field で `/feature-dev` を invoke するロジックはあるが run_id を渡す contract がなく fresh init に流れうる
- **belt 側の素材**: `RunState`, `phase-*.md` narrative notes, `belt-agent status --run <id>` は既に揃っている

### 観察された問題

feature-dev pipeline を narrative phase 付近まで進めて `/handover` → `/clear` → `/continue` した際、belt が再開されず別 skill が発動する。根本原因は handover が belt state を記録していないこと。

### 情報の所在 (設計の核)

belt run を resume するために必要な情報は **既に** 以下に分散して存在する:

| 情報 | 所在 |
|---|---|
| `run_id` | `.belt/runs/<uuid>/` の directory 名 (latest は UUIDv7 lex max) |
| `pipeline` / `pipeline_file` (abs path) | `.belt/runs/<id>/state.json` (既存、BELT-22) |
| `branch` | 同上 (narrative artifact spec で追加済) |
| `current_phase` | 同上 |
| `worktree_path` | current `pwd` (handover/resume は同一 worktree 前提) |
| latest run 選択 | UUIDv7 lex max で `belt-agent status` auto-detect |

よって **新規に保存すべき情報は LLM が書く Resume hint のみ**。pipeline state の冗長な index を別途作らない。

### Claude Code runtime 制約

`/clear` の programmatic 発火は不可 (memory `project_claude_code_runtime_limits.md`)。resume は user 手動呼び出し前提。

## Architecture

```
<worktree>/.belt/runs/<run_id>/
  handover.md                 ← ★新規 artifact (唯一の新規、per-run)
  state.json                  ← belt-agent 既存、resume 時の source of truth
  notes/phase-*.md            ← 既存 narrative notes

plugins/belt/skills/
  handover/SKILL.md           ← /belt:handover (user-facing)
  resume/SKILL.md             ← /belt:resume   (user-facing)

plugins/belt-agent/skills/protocol/
  SKILL.md                    ← 既存、1 行 reference pointer を追加
  references/
    resume-mode.md            ← 新規、resume args 受け手順の SSOT

plugins/belt/skills/feature-dev/pipeline.yml  ← pre-execute-handover phase 追加
plugins/belt/skills/bug-fix/pipeline.yml      ← pre-execute-handover phase 追加
```

### 責務の SSOT マトリクス

| 責務 | SSOT |
|---|---|
| handover 概念 / 手順 / 利用タイミング | `plugins/belt/skills/handover/SKILL.md` |
| resume 概念 / precondition / recovery | `plugins/belt/skills/resume/SKILL.md` |
| protocol driver が resume args を受けた時の分岐 | `plugins/belt-agent/skills/protocol/references/resume-mode.md` |
| checkpoint phase の挙動 | `pipeline.yml` の phase `description` (belt-agent next JSON 経由で LLM に届く) |
| `handover.md` schema | 本 design doc (spec) |
| belt run state 全般 | `.belt/runs/<id>/state.json` (既存、belt-agent 管轄) |

pipeline SKILL.md (feature-dev / bug-fix / debug-flow) には handover / resume 概念を書かない。

## Skills

### 命名と frontmatter

Anthropic best practices (lowercase + hyphens only, no reserved words, third-person, what + when 両明示, ≤1024 chars):

```yaml
# plugins/belt/skills/handover/SKILL.md
---
name: handover
description: >-
  Writes a handover note (Resume hint) under the current belt run directory
  so a later session can pick up where the pipeline was paused. Use when
  pausing a multi-phase belt pipeline (feature-dev, bug-fix, debug-flow)
  before /clear or session end, or when the user invokes /belt:handover.
---
```

```yaml
# plugins/belt/skills/resume/SKILL.md
---
name: resume
description: >-
  Resumes a previously handed-over belt pipeline run by reading handover.md
  and state.json from the current worktree, verifying preconditions, and
  invoking the owning pipeline skill in resume mode via Skill tool args.
  Use when continuing belt pipeline work after /belt:handover and /clear,
  when the user invokes /belt:resume, or after a session restart to pick up
  an in-progress run in the current worktree.
---
```

### SKILL.md 本文構成 (≤500 行、progressive disclosure)

各 SKILL.md の節順:

1. `## Overview` (3〜5 行、何をする skill か)
2. `## Workflow` (番号付きチェックリスト)
3. `## Schema` (handover.md のみ)
4. `## Preconditions` (resume のみ、5 check の表)
5. `## Error Recovery` (resume のみ、precondition 失敗時の選択肢)
6. `## References` (`./references/...` への link、一段階のみ深)

各 100〜150 行見込み。500 行超過時のみ references/ に切り出す。

### `/belt:handover` のワークフロー

```
Handover Progress:
- [ ] Step 1: Verify belt-agent is on PATH and cwd is inside a git worktree
- [ ] Step 2: Query `belt-agent status` to capture run_id / pipeline / current_phase / branch
- [ ] Step 3: Draft Resume hint (Pause reason / First action / Transient context)
- [ ] Step 4: Overwrite .belt/runs/<run_id>/handover.md with frontmatter + hint
- [ ] Step 5: Inform user "Handover written. Run /clear then /belt:resume to continue."
```

### `/belt:resume` のワークフロー

```
Resume Progress:
- [ ] Step 1: Precondition checks #1..#5 (short-circuit on first failure)
- [ ] Step 2: Read .belt/runs/<run_id>/handover.md and incorporate Resume hint into context
- [ ] Step 3: Invoke Skill(skill="belt:<pipeline>", args="resume run_id=<id>")
- [ ] Step 4: Protocol driver follows resume-mode.md, skips init, uses `status --run <id>`
```

## Schema

### `handover.md` (唯一の新規 artifact)

場所: `<worktree>/.belt/runs/<run_id>/handover.md`

```markdown
---
run_id: 01JBC5R1MZQFVY9T8H0K3P7Q2N
branch: feature/2026-04-17-belt-handover-resume
created_at: 2026-04-17T18:23:44Z
---

## Resume hint

- **Pause reason**: <なぜここで止めたか (例: context が肥大化、EOD、設計判断待ち)>
- **First action on resume**: <最初の具体的動作 (例: `belt-agent status` 確認後、phase-plan.md を読み execute phase の Task 1 から着手)>
- **Transient context**: <belt state / phase-*.md どちらにも落ちていない文脈 (例: user 口頭で "API の error handling は後回し" と合意)>
```

#### Frontmatter 最小化の根拠

保持する 3 フィールド:

| フィールド | 保持する理由 |
|---|---|
| `run_id` | `state.json` から得られるが、handover.md 単体での ID 照合・破損検知に使う |
| `branch` | handover 時点の branch。resume 時に branch 変化を検知する唯一の source |
| `created_at` | stale 警告 (将来機能) / 人間が目視で鮮度判定するための情報 |

保持しないフィールド (既存情報源に委譲):

- `pipeline_file`, `current_phase` — `.belt/runs/<id>/state.json` から取得
- `worktree_path` — current `pwd` で取得 (handover / resume は同一 worktree 前提)
- `pipeline` — `state.json` から取得 (convenience 情報、二重管理不要)

#### 厳守制約

- 本文の自由記述は `## Resume hint` の 3 項目のみ。他の節を追加しない
- **phase 進行の叙述を書かない**。phase 単位の記録は `notes/phase-<id>.md` の役割であり、重複させない

## Resume Mode Reference

`plugins/belt-agent/skills/protocol/references/resume-mode.md` (新規):

```markdown
# Resume Mode

/belt:resume からの invoke 時、以下の手順で init を skip する。

## Detection

Skill invoke args に `resume run_id=<id>` 形式のヒントが含まれる場合、resume mode と判定する。

## Steps

1. `belt-agent init` を実行せず、`belt-agent status --run <id>` で現 phase を確認する
2. `current_phase == "COMPLETED"` の場合、resume skill 側に報告して停止する (precondition で事前弾きされる想定)
3. それ以外は通常プロトコル (`belt-agent next` / `verify` / `step`) に遷移する
4. `.belt/runs/<id>/handover.md` が存在する場合、`## Resume hint` 節を読み込んで LLM 文脈に取り込む
```

`plugins/belt-agent/skills/protocol/SKILL.md` には 1 行のみ追加 (本文は reference 側 SSOT):

```markdown
When invoked with `resume run_id=<id>` args, follow `./references/resume-mode.md`.
```

## Preconditions (resume, fail-loud)

| # | Check | Failure message | Recovery options |
|---|---|---|---|
| 1 | `belt-agent` CLI が PATH 上に存在 | "belt-agent CLI not installed or not on PATH" | install / fix PATH / abort |
| 2 | `belt-agent status` が exit 0、latest run を返す | "No belt runs found in current directory" | cd to correct worktree / abort |
| 3 | `.belt/runs/<run_id>/handover.md` 存在 | "No handover note for latest run. Run /belt:handover first." | run `/belt:handover` / abort |
| 4 | `current_phase != "COMPLETED"` | "Last run already completed" | 情報出力 + 新規 run 促し |
| 5 | 現 branch == handover.md frontmatter `branch` | "Branch changed A→B since handover" | proceed anyway (y/N) |

- check は順次短絡、最初の失敗で停止
- #4 は abort ではなく情報通知 (COMPLETED run を resume できないのは仕様であり例外的失敗ではない)
- #5 は warning のみ、user 判断で継続可

## Pipeline Extension: `pre-execute-handover` phase

### 目的

execute phase は context 重量が最大の phase となりやすい。その前に structured な checkpoint を入れ、user に `/belt:handover` → `/clear` → `/belt:resume` のサイクルを踏ませることで fresh context で execute に入れる。

### 追加 phase 定義

```yaml
- id: pre-execute-handover
  description: >-
    Context checkpoint before execute. Run `/belt:handover`, then `/clear`,
    then `/belt:resume` in a new session. The gate passes once the handover
    note exists under `.belt/runs/{run_id}/`.
  confirm: true
  gate:
    - file_exists: ".belt/runs/{run_id}/handover.md"
```

挿入位置:

- **feature-dev**: `plan` と `execute` の間
- **bug-fix**: `fix-plan` と `execute` の間

### 実行フロー (feature-dev 例)

1. `plan` phase 完了 (`phase-plan.md` 確認)
2. `belt-agent step` → `pre-execute-handover` phase に遷移
3. `belt-agent next` → phase description を JSON で返却
4. LLM が user に伝達: 「`/belt:handover` → `/clear` → `/belt:resume`」
5. user: `/belt:handover` — `handover.md` 書き出し
6. user: `/clear` — context リセット
7. user: `/belt:resume` — `handover.md` 読み込み → `/belt:feature-dev` を `resume run_id=<id>` args で invoke
8. 新 context の LLM: `belt-agent status` で `pre-execute-handover` に居ると認識
9. `belt-agent verify` → gate `file_exists` 通過
10. `belt-agent step --confirm` → `execute` phase へ遷移

### 設計上の美点

- **SKILL.md 汚染ゼロ**: phase description が pipeline.yml に内包、belt-agent next 経由で LLM に届く
- **既存 gate 語彙のみ**: `confirm: true` + `file_exists`、belt-core 機構を一切拡張しない
- **Fallback 安全**: `/clear` を忘れても gate は通過する (context 肥大のみ残るが pipeline は進む)
- **他 pipeline は無関心**: debug-flow 等 checkpoint 不要な pipeline は触らない

### regate topology への影響

既存 regate 宣言への影響なし。`pre-execute-handover` は新規 phase であり、他 phase が regate 対象として指定しない限り自動 regate されない。feature-dev の `code-review.regate` は `design / test-scenarios / plan / execute` のみを対象にし、checkpoint phase は含めない (既存宣言のまま維持)。

## Lifecycle

| Event | handover.md |
|---|---|
| `/belt:handover` 実行 | 同一 run 配下を上書き |
| `/belt:resume` 成功 | 保持 (同一 run の再 resume 許容) |
| run が COMPLETED | 保持 (run 履歴として残存) |
| `belt-agent init` で新 run 開始 | 新 run 配下には生成されない (次 handover で初回生成) |

- run ごとに `.belt/runs/<id>/handover.md` が最大 1 本
- run 切替で自動的に「最新 handover」が変わる (UUIDv7 lex max の latest run が常に追跡対象)
- cleanup ロジック不要 (global index がないため)

## Impact on Existing Docs

### 書き換え必須 (2 ファイル、最小 touch)

1. **`plugins/belt-agent/references/narrative-convention.md`** L7
   - `/clear` 言及を `/clear or /belt:resume` に拡張 (1 行の改変)
   - 他の `/clear` mention (L49 / L88) は canonical example として据え置き
   - 新規節は追加しない、handover 概念は持ち込まない

2. **`plugins/belt/skills/feature-dev/references/brainstorming-supplement.md`** L105
   - `(base = current branch at Phase 1 start; resume from handover if set).` から `; resume from handover if set` を削除
   - 結果: `(base = current branch at Phase 1 start).`
   - generic `/handover` と新 `/belt:handover` の mixed semantics を除去

### 書き換え不要 (scope 外として明示)

| 対象 | 理由 |
|---|---|
| `plugins/belt/skills/feature-dev/SKILL.md` | handover/resume は新 skill 側 SSOT、pipeline SKILL.md に浸透させない |
| `plugins/belt/skills/bug-fix/SKILL.md` | 同上 |
| `plugins/belt/skills/*/criteria/*.md` | `depends_on_artifacts` は既存 path (`.belt/runs/*/notes/*`, `.belt/runs/*/review/*`) のみ、handover.md は別 path |
| `plugins/belt/agents/*-reviewer.md` | `findings-*.json` 出力のみ、handover 非依存 |
| `plugins/belt/skills/feature-dev/references/path-convention.md` | feature-dev phase 産物の SSOT、handover.md は skill 産物で scope 外 |
| `pipeline.yml` の既存 phase 定義 | `pre-execute-handover` の追加以外は無改修 |

## Evaluations

Anthropic best practices の "Build evaluations first" に従い、実装前に以下を eval として固定する:

### EV-1: Happy path resume

- **Given**: feature-dev pipeline を `plan` 完了まで進め、`pre-execute-handover` phase に入っている
- **When**: user が `/belt:handover` → `/clear` → `/belt:resume` を実行
- **Then**:
  - `.belt/runs/<id>/handover.md` が所定 schema (frontmatter 3 field + Resume hint 3 項目) で書かれる
  - resume で `belt-agent status` が `pre-execute-handover` phase を返す
  - `belt-agent verify` で gate 通過
  - `belt-agent step --confirm` で `execute` phase に遷移できる

### EV-2: No runs in current directory (precondition #2)

- **Given**: cwd に `.belt/runs/` が存在しない (worktree 取り違え等)
- **When**: `/belt:resume` 実行
- **Then**:
  - precondition #2 で fail-loud、`"No belt runs found in current directory"` を surface
  - recovery として正しい worktree への cd を提示 (自動 cd しない)

### EV-3: No handover note (precondition #3)

- **Given**: run は存在するが `handover.md` 未作成 (`/belt:handover` を呼んでいない)
- **When**: `/belt:resume` 実行
- **Then**:
  - precondition #3 で fail-loud、`"Run /belt:handover first"` を提示

### EV-4: Branch mismatch (precondition #5)

- **Given**: `/belt:handover` 時 branch A、その後 `git checkout B` で別 branch に切替
- **When**: `/belt:resume` 実行
- **Then**:
  - precondition #5 で `"Branch changed A→B since handover"` warning を表示
  - `proceed anyway (y/N)` の選択肢を提示
  - user が `y` を選んだ場合のみ resume 本体に進む

## Files To Add / Modify

### 新規

1. `plugins/belt/skills/handover/SKILL.md`
2. `plugins/belt/skills/resume/SKILL.md`
3. `plugins/belt-agent/skills/protocol/references/resume-mode.md`

### 更新

4. `plugins/belt-agent/skills/protocol/SKILL.md` — 1 行追加 (resume-mode.md への reference pointer)
5. `plugins/belt-agent/references/narrative-convention.md` — L7 の `/clear` 拡張のみ
6. `plugins/belt/skills/feature-dev/references/brainstorming-supplement.md` — L105 の `; resume from handover if set` 削除
7. `plugins/belt/skills/feature-dev/pipeline.yml` — `pre-execute-handover` phase 追加 (plan → execute の間)
8. `plugins/belt/skills/bug-fix/pipeline.yml` — `pre-execute-handover` phase 追加 (fix-plan → execute の間)

## Known Limitations

### L-1: belt-agent CLI 前提

precondition #1 で `belt-agent` が PATH 上にない場合 abort する。belt エコシステム全体の前提条件であり本 skill 固有の制約ではない。

### L-2: phase id hyphen 互換性

`pre-execute-handover` phase id は hyphen を含む。belt-core は既存 `monkey-test` 等で hyphen preservation を実証済み (memory `project_belt_core_model_shapes_2026_04_14.md`)。実装時に `parser.rs` / `expander.rs` で再確認する。

### L-3: current worktree 限定

`/belt:resume` は常に current cwd の `.belt/runs/` を見る。別 worktree の run は復元対象外。worktree 切替運用は user 側で担保 (`cd <worktree>` → `/belt:resume`)。これは global index を持たない設計判断のトレードオフであり、意図的な制約。

## Open Questions / Follow-ups

- 将来 `belt-agent resume` CLI subcommand を追加する場合、skill は CLI wrapper に退行させるか — MVP では skill のみ
- `pre-execute-handover` phase を `when:` condition で無効化可能にするか (例: `when: "!args.skip_handover"`) — MVP では default-on 固定、発言があれば follow-up
- `handover.md` 書き込み時に phase 未完了 (gate 未到達) のケース向け Resume hint template 強化 — 実装段階で検討
- stale handover 検知 (`created_at` が一定時間以上古い場合の warning 表示) — follow-up
- scenarios_contract 対応: `/belt:handover` / `/belt:resume` の scenario を `docs/testing/cli-behavior/` 下に追加 — F2a 実装完了後の follow-up

## Checklist for Effective Skills (Anthropic compliance)

### Core quality

- [ ] description は specific で key terms を含む
- [ ] description は what + when を両方含む (3 人称)
- [ ] SKILL.md body は 500 行未満
- [ ] references は一段階のみ深 (`./references/...`)
- [ ] 用語統一 (`handover.md` / `handover` / `resume` / `Resume hint`)
- [ ] 時間依存情報なし
- [ ] 具体例は concrete (schema YAML、precondition 表)
- [ ] progressive disclosure 採用 (SKILL.md overview → references)
- [ ] workflow のステップが明確 (チェックリスト形式)

### Code and scripts

- [ ] scripts は punt でなく問題を解く (N/A: skill は LLM-driven、script なし)
- [ ] error handling は explicit (precondition 表)
- [ ] magic number なし (N/A)
- [ ] required packages (N/A: belt-agent CLI 前提のみ)
- [ ] forward slashes のみ (unix path)
- [ ] 重大操作の validation あり (precondition 1-5)
- [ ] feedback loops (precondition fail → recovery → retry)

### Testing

- [ ] 最低 3 件の evaluation (EV-1..EV-4 で 4 件)
- [ ] 将来 Haiku / Sonnet / Opus で test 予定 (実装フェーズ)
- [ ] real 使用シナリオ (EV-1 が happy path)

## References

- Anthropic Agent Skills best practices: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices
- belt narrative artifact design: `docs/specs/2026-04-14-belt-context-neutral-narrative-artifact.md`
- belt narrative follow-up: `docs/specs/2026-04-15-narrative-followup-design.md`
- plugin consolidation: `docs/specs/2026-04-17-plugins-consolidation-design.md`
