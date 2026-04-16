# belt-agent Skill Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move `skills/belt-agent/SKILL.md` to `plugins/belt-agents/skills/belt-agent/SKILL.md`, delete the project-root `skills/` directory, and update README / marketplace.json / plugin.json descriptions to reflect "Base layer: 5 analysis agents + Belt Protocol skill + references".

**Architecture:** Single file move via `git mv`, one empty-directory deletion, three string edits across three files. Verification is a plugin-loader smoke test: after the move, the Claude Code Skill tool must resolve `belt-agent` from within the belt-agents plugin, not from a project-root skill.

**Tech Stack:** Markdown (SKILL.md), JSON (plugin manifests), git mv, Claude Code plugin loader.

**Spec:** `docs/superpowers/specs/2026-04-16-cargo-dist-release-and-skill-consolidation-design.md` — Part B (§3)

**Related context:**

- MEMORY `project_skill_supplement_override_pattern.md` — belt の skill 差し替え慣行
- MEMORY `project_feature_dev_refresh_2026_04_14.md` — feature-dev / bug-fix が既に `plugins/<plugin>/skills/<slug>/SKILL.md` 構造で稼働中なので同形を踏襲
- MEMORY `project_parallel_session_worktree_isolation.md` — 別セッションが動いている場合は worktree 隔離推奨

**Prerequisite:** Spec commit (`dcc31f6`) が main に到達していること。実装は Part A (cargo-dist release) より先に行う (spec §5)。Worktree 推奨:

```bash
wt switch --create belt-agent-skill-consolidation
# or
git switch -c belt-agent-skill-consolidation
```

以降のタスクは全てこの worktree / branch 上で実行する。

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `skills/belt-agent/SKILL.md` | **Move** → `plugins/belt-agents/skills/belt-agent/SKILL.md` | Belt Protocol skill (user-invocable: false)。内容不変 |
| `skills/` (project root) | **Delete** (empty after move) | `rmdir skills/` |
| `.claude-plugin/marketplace.json` | Edit line 11 | belt-agents description に "+ Belt Protocol skill" を追加 |
| `plugins/belt-agents/.claude-plugin/plugin.json` | Edit line 3 | 同上 (marketplace.json と同一文面) |
| `README.md` | Edit 1 行 (plugins table の belt-agents 行) | 同上 |

**Untouched (guard):**

- `plugins/belt-agents/agents/**` — 5 agent は一切変更しない
- `plugins/belt-agents/references/**` — 未修正 (SKILL.md 本文中の `plugins/belt-agents/references/audit-protocol.md` 参照は絶対 path なので壊れない)
- `plugins/feature-dev/**` / `plugins/bug-fix/**` — 内部 skill 参照は name-based で path 依存しない想定だが Task 1 で audit
- `AGENTS.md` (= `CLAUDE.md` symlink) — Part B では変更なし (Part A で "Release Process" 節を追加)
- `crates/**`, `Cargo.toml`, `release.toml`, `dist-workspace.toml` — Part A の範疇

---

## Task 1: Pre-audit — `skills/belt-agent` path の hardcode 検出

**Rationale:** spec §3.5 open point #3。Claude Code plugin の skill invoke は name-based (`/belt-agent`) のはずだが、pipeline.yml / SKILL.md 本文 / plugin README / references に `skills/belt-agent/...` という**path hardcode** があれば移動時に壊れる。移動前に全件洗い出して plan に反映する。

**Files:**
- Read-only: repo 全体

- [ ] **Step 1: `skills/belt-agent` の path hardcode 検出**

Run: `grep -rn "skills/belt-agent" --include="*.md" --include="*.yml" --include="*.yaml" --include="*.json" --include="*.toml" --include="*.rs" .`
Expected: 0 件 (plugin loader は name-based のため path hardcode は原則無いはず)。

もし 1 件以上ヒットした場合:
- Markdown (`.md`) 内の documentation としての言及 → Task 6 以降で該当箇所を更新する list に追加
- Code / YAML / JSON (`.rs`, `.yml`, `.json`) 内の実行時 reference → **STOP**。計画を変更し、該当 file も edit list に追加してから進む

- [ ] **Step 2: `/belt-agent` skill invoke の呼び出し元確認 (name-based)**

Run: `grep -rn "/belt-agent" --include="*.md" --include="*.yml" --include="*.yaml" .`
Expected: 複数ヒット可。name-based 参照なので移動後も**壊れない**。移動対象外であることを確認するだけ。

- [ ] **Step 3: `plugins/belt-agents/skills/` ディレクトリが事前に存在しないことを確認**

Run: `ls plugins/belt-agents/skills/ 2>&1`
Expected: `No such file or directory` または空。既に存在していれば **STOP** し、状況を確認 (別作業が先行していないか)。

- [ ] **Step 4: 記録**

Task 1 の grep 結果を手元メモに残し、該当箇所があれば後続 Task の edit 対象に追加する。この Task ではファイル変更は行わないため commit 不要。

---

## Task 2: SKILL.md を `git mv` で移動

**Rationale:** ファイル内容は変更せず、履歴を保ったまま移動する。`git mv` を使うことで blame が辿れる状態を維持。

**Files:**
- Move: `skills/belt-agent/SKILL.md` → `plugins/belt-agents/skills/belt-agent/SKILL.md`

- [ ] **Step 1: 目的ディレクトリを作成**

Run: `mkdir -p plugins/belt-agents/skills/belt-agent`
Expected: 成功 (エラーなし)。

- [ ] **Step 2: `git mv` で move**

Run: `git mv skills/belt-agent/SKILL.md plugins/belt-agents/skills/belt-agent/SKILL.md`
Expected: 成功、`git status` で `renamed: skills/belt-agent/SKILL.md -> plugins/belt-agents/skills/belt-agent/SKILL.md` が表示される。

- [ ] **Step 3: 内容が変わっていないことを確認**

Run: `git diff --cached --stat -M plugins/belt-agents/skills/belt-agent/SKILL.md`
Expected: `1 file changed, 0 insertions(+), 0 deletions(-)` (rename のみ、内容変更なし)。

もし `+N/-N` が表示されたら **STOP**。内容 drift が発生している。`git diff --cached plugins/belt-agents/skills/belt-agent/SKILL.md` で差分を確認し、意図しない変更なら `git restore --staged` で取り消して再実行。

- [ ] **Step 4: SKILL.md の frontmatter `name` を確認**

Run: `head -5 plugins/belt-agents/skills/belt-agent/SKILL.md`
Expected: `name: belt-agent` のまま (変更なし)。

---

## Task 3: 空になった `skills/` ディレクトリを削除

**Rationale:** Task 2 で `skills/belt-agent/` は空になるが、git は空ディレクトリを追跡しない。`rmdir` で手元の空ディレクトリを削除する。

**Files:**
- Delete: `skills/belt-agent/` (空)
- Delete: `skills/` (空)

- [ ] **Step 1: `skills/belt-agent/` が空であることを確認**

Run: `ls -A skills/belt-agent/ 2>&1`
Expected: 出力なし (空)。もしファイル/サブディレクトリが残っていれば **STOP**、原因調査。

- [ ] **Step 2: `skills/belt-agent/` を削除**

Run: `rmdir skills/belt-agent/`
Expected: 成功。

- [ ] **Step 3: `skills/` が空であることを確認**

Run: `ls -A skills/ 2>&1`
Expected: 出力なし。

- [ ] **Step 4: `skills/` を削除**

Run: `rmdir skills/`
Expected: 成功。

- [ ] **Step 5: `.gitignore` に `skills/` が残っていないかチェック**

Run: `grep -n "skills" .gitignore 2>&1`
Expected: ヒットなし (あっても belt-agent 関係なければ無視)。もし `skills/` のエントリがあれば、履歴的な意図を判断し不要なら削除。

---

## Task 4: `.claude-plugin/marketplace.json` の belt-agents description 更新

**Rationale:** spec §3.4。marketplace.json は Claude Code plugin discovery の一次情報。belt-agents が Belt Protocol skill を抱えることを description に明記する。

**Files:**
- Modify: `.claude-plugin/marketplace.json:11` (belt-agents 行の `description`)

- [ ] **Step 1: 現在の description を確認**

Run: `grep -A1 '"name": "belt-agents"' .claude-plugin/marketplace.json`
Expected:
```
"name": "belt-agents",
"description": "Base analysis agents for belt-based quality-gated development pipelines (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer)",
```

- [ ] **Step 2: description を更新**

Use the Edit tool with these exact strings:

`old_string`:
```
"description": "Base analysis agents for belt-based quality-gated development pipelines (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer)",
```

`new_string`:
```
"description": "Base analysis agents + Belt Protocol skill for belt-based quality-gated development pipelines (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer)",
```

- [ ] **Step 3: JSON 妥当性の確認**

Run: `python3 -c "import json; json.load(open('.claude-plugin/marketplace.json')); print('valid')"`
Expected: `valid`。パースエラーが出れば **STOP**、手動で修正。

- [ ] **Step 4: 変更 diff の確認**

Run: `git diff .claude-plugin/marketplace.json`
Expected: 1 行のみ変更、他は touch されていない。

---

## Task 5: `plugins/belt-agents/.claude-plugin/plugin.json` の description 更新

**Rationale:** marketplace.json と plugin.json で description が乖離すると混乱する。spec §3.4 の指示通り両者を同一文面に揃える。

**Files:**
- Modify: `plugins/belt-agents/.claude-plugin/plugin.json:3`

- [ ] **Step 1: 現在の description を確認**

Run: `cat plugins/belt-agents/.claude-plugin/plugin.json`
Expected:
```json
{
  "name": "belt-agents",
  "description": "Base analysis agents for belt-based quality-gated development pipelines (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer)",
  "version": "0.1.0",
  "author": { "name": "neko-neko" }
}
```

- [ ] **Step 2: description を更新**

Use the Edit tool with these exact strings:

`old_string`:
```
  "description": "Base analysis agents for belt-based quality-gated development pipelines (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer)",
```

`new_string`:
```
  "description": "Base analysis agents + Belt Protocol skill for belt-based quality-gated development pipelines (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer)",
```

- [ ] **Step 3: JSON 妥当性の確認**

Run: `python3 -c "import json; json.load(open('plugins/belt-agents/.claude-plugin/plugin.json')); print('valid')"`
Expected: `valid`。

- [ ] **Step 4: marketplace.json との文面同一性を検証**

Run:
```bash
diff \
  <(python3 -c "import json; print(json.load(open('.claude-plugin/marketplace.json'))['plugins'][0]['description'])") \
  <(python3 -c "import json; print(json.load(open('plugins/belt-agents/.claude-plugin/plugin.json'))['description'])")
```
Expected: 出力なし (完全一致)。差分があれば **STOP** して揃える。

---

## Task 6: `README.md` plugins table の belt-agents 行を更新

**Rationale:** spec §3.4。README の "Plugins in this repo" table が最も利用者の目に触れる箇所。skill 追加を反映する。

**Files:**
- Modify: `README.md:195` (belt-agents 行)

- [ ] **Step 1: 現在の行を確認**

Run: `grep -n "belt-agents" README.md | head -5`
Expected: `195:| \`belt-agents\` | Base layer: 5 analysis agents (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer) + references |` (line 番号は若干ズレ可)。

- [ ] **Step 2: 行を更新**

Use the Edit tool with these exact strings:

`old_string`:
```
| `belt-agents` | Base layer: 5 analysis agents (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer) + references |
```

`new_string`:
```
| `belt-agents` | Base layer: 5 analysis agents (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer) + Belt Protocol skill + references |
```

- [ ] **Step 3: diff 確認**

Run: `git diff README.md`
Expected: 1 行のみ変更、他は touch されていない。

---

## Task 7: Plugin loader smoke test — 移動後も `/belt-agent` skill が invoke できることを確認

**Rationale:** spec §3.5 open point #1-#2。`feature-dev` / `bug-fix` 等で既に同構造が稼働しているため動くはずだが、user-invocable: false の skill が plugin 配下で正しく登録されるかを実地で確認。

**Files:**
- Read-only verification

- [ ] **Step 1: Claude Code session を再起動 (skill cache 再読み込み)**

このセッション自体は継続。別ターミナル / 別 Claude Code session で plugin を再 load するか、IDE で Claude Code を restart する。

もし Claude Code の skill cache がプロセス起動時に固定化される環境では、新しい session で以下の Step 2-3 を実行する。

- [ ] **Step 2: `/belt-agent` skill が discovery される**

New session で:
```
/belt-agent
```

Expected: skill content がロードされる (frontmatter `user-invocable: false` が現実にどう扱われるかは implementation detail だが、少なくとも plugin loader が認識していることを確認)。

Alternative: `Skill` tool (agentic context 内) で `belt-agent` を指定できることを `Skill` tool の skill list に belt-agent が含まれていることで確認。

- [ ] **Step 3: belt-agents plugin が同 skill を抱えていることを marketplace view で確認**

```
/plugin-list
```

もしくは Claude Code の plugin 画面で `belt-agents` plugin の詳細を開き、提供 skill に `belt-agent` が列挙されていること。列挙されない場合、skill がまだ discoverable でない可能性がある。

- [ ] **Step 4: feature-dev / bug-fix の invoke で壊れないか**

Run (dry-run 相当、何かしらの軽い起動で skill 参照が機能することを確認):
```
# 例: feature-dev pipeline の help 相当を load
# plugins/feature-dev/skills/feature-dev/pipeline.yml に `/belt-agent` 参照がないか確認
grep -n "belt-agent" plugins/feature-dev/skills/feature-dev/pipeline.yml
grep -n "belt-agent" plugins/bug-fix/skills/bug-fix/pipeline.yml
```
Expected: `belt-agent` CLI binary の言及 (cmd など) はヒット可だが、**skill reference** (`invoke.skill: /belt-agent` 等) は基本存在しないはず。存在すれば Task 1 で拾えていた grep と重複する。

もし skill reference があり、かつ Task 1 で壊れない invoke と判断していた場合も、現実に動くか新 session で確認。

- [ ] **Step 5: 記録**

verification が success であれば Task 7 は done。failure が出た場合は原因を切り分け、場合によっては rollback (`git restore` + `git mv` の逆転) を実施。

---

## Task 8: Commit

**Rationale:** Part B の変更を 1 commit にまとめる。PR 分割は後続で実施 (main へは別 PR として merge)。

**Files:** all changes from Task 2-6.

- [ ] **Step 1: 変更状態の最終確認**

Run: `git status`
Expected: 以下が staged / modified で、他は touch されていない:
- `renamed: skills/belt-agent/SKILL.md -> plugins/belt-agents/skills/belt-agent/SKILL.md`
- `modified: .claude-plugin/marketplace.json`
- `modified: plugins/belt-agents/.claude-plugin/plugin.json`
- `modified: README.md`

`skills/` ディレクトリ自体は git の追跡対象外なので status には出ないが、作業ツリーから削除済みであれば OK。

- [ ] **Step 2: stage all**

Run:
```bash
git add plugins/belt-agents/skills/belt-agent/SKILL.md \
        .claude-plugin/marketplace.json \
        plugins/belt-agents/.claude-plugin/plugin.json \
        README.md
```

(`git mv` は自動で stage する。念のため上記で stage 完了を保証)

- [ ] **Step 3: commit**

Run:
```bash
git commit -m "$(cat <<'EOF'
refactor(plugins): consolidate belt-agent skill into belt-agents plugin

Move skills/belt-agent/SKILL.md to plugins/belt-agents/skills/belt-agent/
SKILL.md so that installing the belt-agents plugin also brings the Belt
Protocol skill. Drop the empty project-root skills/ directory.

Update marketplace.json / plugin.json / README descriptions to advertise
"Base analysis agents + Belt Protocol skill".

Per spec: docs/superpowers/specs/2026-04-16-cargo-dist-release-and-skill-consolidation-design.md Part B (§3).
EOF
)"
```

Expected: commit 成功。pre-commit hook が無ければそのまま完了。

もし GPG 署名エラーが出た場合は `-c commit.gpgsign=false` を前置して再試行 (CLAUDE.md 指示に従う)。

- [ ] **Step 4: commit 内容の post-verify**

Run: `git log -1 --stat`
Expected: 4 file 変更 (rename + 3 modified)、README.md / marketplace.json / plugin.json の行数変化は小さい (各 1 行)、SKILL.md は 0 行差分で rename。

- [ ] **Step 5: PR 作成 (optional、ローカル検証だけなら skip)**

Run:
```bash
git push -u origin belt-agent-skill-consolidation
gh pr create --title "Consolidate belt-agent skill into belt-agents plugin" --body "$(cat <<'EOF'
## Summary

- Move `skills/belt-agent/SKILL.md` → `plugins/belt-agents/skills/belt-agent/SKILL.md`
- Delete empty project-root `skills/` directory
- Update `marketplace.json` / `plugin.json` / `README.md` descriptions to reflect the consolidated plugin shape

## Why

`skills/` held a single entry (`belt-agent`) and duplicated the responsibility of the `belt-agents` plugin. Consolidating avoids a stray top-level directory and makes the `belt-agents` plugin self-contained (users installing it now get the Belt Protocol skill automatically).

## Test plan

- [x] `git diff --stat -M` shows a pure rename for `SKILL.md` (zero content churn)
- [x] JSON validity: `marketplace.json` and `plugin.json` parse cleanly
- [x] Claude Code plugin loader recognises `/belt-agent` from the new path (smoke test per plan Task 7)

Per spec: `docs/superpowers/specs/2026-04-16-cargo-dist-release-and-skill-consolidation-design.md` Part B (§3). This is the first of two PRs; Part A (cargo-dist release automation) ships separately.
EOF
)"
```

---

## Self-Review Checklist

1. **Spec coverage:**
   - §3.2 Move — Task 2 ✓
   - §3.3 Path references in SKILL.md — 本文変更なし、Task 1 で verify ✓
   - §3.4 Integrity updates (README / marketplace.json / plugin.json) — Task 4, 5, 6 ✓
   - §3.4 Integrity updates (AGENTS.md = 変更不要) — Task non-task で guard 記載 ✓
   - §3.5 Open points (plugin loader / user-invocable / hardcode 参照) — Task 1 (audit), Task 7 (smoke) ✓

2. **No placeholders:** 全 step に具体コマンド + expected output。TBD / 推測なし ✓

3. **Type consistency:** description 文面が Task 4 (marketplace.json) と Task 5 (plugin.json) で diff verify される ✓

4. **Rollback path:** commit 前であれば `git restore` + `git mv` 逆転。commit 後であれば `git revert <hash>` ✓
