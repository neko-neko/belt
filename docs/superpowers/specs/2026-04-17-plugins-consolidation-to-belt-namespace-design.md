---
title: Plugin Consolidation to `belt:*` / `belt-agent:*` Namespace
date: 2026-04-17
status: approved
scope:
  - .claude-plugin/marketplace.json
  - plugins/belt/**
  - plugins/belt-agent/**
  - plugins/belt-agents/** (deleted after migration)
  - plugins/feature-dev/** (deleted after migration)
  - plugins/bug-fix/** (deleted after migration)
  - plugins/code-review/** (deleted after migration)
  - plugins/spec-review/** (deleted after migration)
  - plugins/monkey-test/** (deleted after migration)
  - plugins/test-scenarios/** (deleted after migration)
  - crates/belt-core/tests/review_skills_refresh.rs
  - crates/belt-core/tests/shared_filter_parity.rs
  - crates/belt-core/tests/shared_criteria_parity.rs
  - crates/belt-core/tests/feature_dev_refresh.rs
  - crates/belt-core/tests/bug_fix_refresh.rs
  - crates/belt-agent/tests/cli_test.rs
  - README.md
  - CHANGELOG.md
  - AGENTS.md
related:
  - docs/superpowers/specs/2026-04-15-belt-plugins-migration-design.md (supersedes layout decisions)
  - docs/superpowers/specs/2026-04-16-cargo-dist-release-and-skill-consolidation-design.md (Part B preceded this)
  - MEMORY: project_belt_architecture.md
  - MEMORY: project_claude_md_symlink.md
  - MEMORY: project_skill_supplement_override_pattern.md
---

# Plugin Consolidation to `belt:*` / `belt-agent:*` Namespace

## 1. 概要

現行の 7 plugin 構成 (`belt-agents`, `feature-dev`, `bug-fix`, `code-review`, `spec-review`, `monkey-test`, `test-scenarios`) を **2 plugin 構成** (`belt` + `belt-agent`) に統合し、Claude Code の skill/agent 呼び出し namespace を `belt:*` / `belt-agent:*` に統一する。

目的は Linux 哲学 "Do One Thing and Do It Well" に基づく **観点別プラグイン分離**: 利用者 (user-invocable skill + それに紐づく reviewer agents) と基盤 (Belt Protocol driver skill + 汎用 analysis agents + shared references) を 1 plugin ずつに集約する。

本変更は 2026-04-15 spec (`docs/superpowers/specs/2026-04-15-belt-plugins-migration-design.md`) の「7 plugin + fully-qualified 参照」方針を置き換える破壊的再構成である。v0.1.0 release 後の最初の BREAKING change として v0.2.0 で配布する。

## 2. Motivation

### 2.1 Linux 哲学による観点分離

現行 7 plugin は機能単位で細分化されているが、`feature-dev` / `bug-fix` は `code-review` / `spec-review` / `monkey-test` / `test-scenarios` に依存し、`belt-agents` を共有し、reviewer agents は各 skill plugin が個別保有、という網状依存になっている。利用者視点では「どれを install すれば動くか」が自明でない。

"observation-level separation" の軸で再整理すると:

- **利用者が発動する skill (pipeline 含む)** と **それが呼び出す reviewer agents** は**機能的凝集**を持つ → 1 plugin 化が自然
- **belt-agent CLI を駆動する protocol skill** と **汎用 analysis agents (phase-auditor / feature-implementer / 3 explorer 系)** と **shared references** は**基盤機能の凝集**を持つ → 1 plugin 化が自然

### 2.2 Install UX の単純化

7 `/install-plugin` コマンドが 2 に集約される。marketplace.json entries も 7 → 2。

### 2.3 Namespace の一貫性

現状:
- user skill: `/feature-dev:feature-dev` (plugin 名と skill 名が重複して冗長)
- reviewer agent: `code-review:security-reviewer` (skill と同一 plugin 内に同居)
- base agent: `belt-agents:phase-auditor`

移行後:
- user skill: `/belt:feature-dev`, `/belt:code-review` 等
- reviewer agent: `belt:security-reviewer` 等 (skill と同じ namespace を共有)
- base agent: `belt-agent:phase-auditor` 等 (plugin 名を単数化し CLI binary 名と揃える)

### 2.4 Protocol skill の slug rename

現行 `belt-agents:belt-agent` は plugin 名 + skill 名の冗長。slug を `belt-agent` → `protocol` に変更し、`belt-agent:protocol` とする。user-invocable: false のため user が直接目にする場面はないが、Claude Code の skill list や内部 Skill tool invoke で参照される際に冗長が解消される。

## 3. Target Architecture

### 3.1 2 plugin 構成

| Plugin | 責務 | 依存 |
|---|---|---|
| `belt` | user-invocable pipeline + それに紐づく reviewer agents | `belt-agent` |
| `belt-agent` | Belt Protocol driver skill + 汎用 analysis agents + shared references | なし |

Marketplace entries: 7 → 2。

### 3.2 呼び出し名前空間

| カテゴリ | Plugin | 呼び出し形式 | 例 |
|---|---|---|---|
| User-invocable skill | `belt` | `/belt:<skill>` | `/belt:feature-dev`, `/belt:code-review` |
| Reviewer agent (観点別) | `belt` | `belt:<reviewer>` | `belt:security-reviewer`, `belt:feasibility-reviewer` |
| Belt Protocol skill | `belt-agent` | `belt-agent:protocol` (user-invocable: false) | — |
| Base analysis agent | `belt-agent` | `belt-agent:<agent>` | `belt-agent:phase-auditor`, `belt-agent:code-explorer` |

### 3.3 命名決定の整合性

- **CLI binary `belt-agent` と plugin `belt-agent` が同名**: 意図的。"belt-agent 生態系は 1 plugin で完結" の統合性を優先。context で区別可能 (CLI は executable、plugin は Claude Code config)
- **Skill slug `protocol`**: `belt-agent:belt-agent` の冗長を回避
- **Reviewer agent を `belt` plugin に配置**: `code-review` / `spec-review` skill とそれらが dispatch する reviewer が同じ namespace (`belt:*`) を共有。SKILL.md 内の `Task(subagent_type: belt:security-reviewer, ...)` が自己完結する

### 3.4 呼び出し名前空間の移行対応表

| 旧 | 新 |
|---|---|
| `/feature-dev:feature-dev` | `/belt:feature-dev` |
| `/bug-fix:bug-fix` | `/belt:bug-fix` |
| `/code-review:code-review` | `/belt:code-review` |
| `/spec-review:spec-review` | `/belt:spec-review` |
| `/monkey-test:monkey-test` | `/belt:monkey-test` |
| `/test-scenarios:test-scenarios` | `/belt:test-scenarios` |
| `code-review:security-reviewer` | `belt:security-reviewer` |
| `code-review:test-reviewer` | `belt:test-reviewer` |
| `code-review:ai-antipattern-reviewer` | `belt:ai-antipattern-reviewer` |
| `code-review:cross-cutting-reviewer` | `belt:cross-cutting-reviewer` |
| `spec-review:feasibility-reviewer` | `belt:feasibility-reviewer` |
| `spec-review:ui-design-reviewer` | `belt:ui-design-reviewer` |
| `spec-review:cross-cutting-spec-reviewer` | `belt:cross-cutting-spec-reviewer` |
| `belt-agents:phase-auditor` | `belt-agent:phase-auditor` |
| `belt-agents:feature-implementer` | `belt-agent:feature-implementer` |
| `belt-agents:code-explorer` | `belt-agent:code-explorer` |
| `belt-agents:code-architect` | `belt-agent:code-architect` |
| `belt-agents:impact-analyzer` | `belt-agent:impact-analyzer` |
| `belt-agents:belt-agent` (Protocol skill) | `belt-agent:protocol` |

## 4. File Layout

### 4.1 新構造

```
belt/
├── .claude-plugin/
│   └── marketplace.json                      # 7 entries → 2 entries
├── plugins/
│   ├── belt/
│   │   ├── .claude-plugin/plugin.json        # "name": "belt"
│   │   ├── skills/
│   │   │   ├── feature-dev/
│   │   │   │   ├── SKILL.md
│   │   │   │   ├── pipeline.yml
│   │   │   │   ├── belt.toml
│   │   │   │   ├── criteria/*.md             # 9 files
│   │   │   │   └── references/*.md           # 6 supplements + path-convention
│   │   │   ├── bug-fix/
│   │   │   │   ├── SKILL.md
│   │   │   │   ├── pipeline.yml
│   │   │   │   ├── belt.toml
│   │   │   │   ├── criteria/*.md             # 8 files
│   │   │   │   └── references/*.md           # 6 supplements + path-convention
│   │   │   ├── code-review/SKILL.md
│   │   │   ├── spec-review/SKILL.md
│   │   │   ├── monkey-test/SKILL.md
│   │   │   └── test-scenarios/SKILL.md
│   │   └── agents/                           # 7 reviewer agents (flat)
│   │       ├── security-reviewer.md
│   │       ├── test-reviewer.md
│   │       ├── ai-antipattern-reviewer.md
│   │       ├── cross-cutting-reviewer.md
│   │       ├── feasibility-reviewer.md
│   │       ├── ui-design-reviewer.md
│   │       └── cross-cutting-spec-reviewer.md
│   │
│   └── belt-agent/
│       ├── .claude-plugin/plugin.json        # "name": "belt-agent"
│       ├── skills/protocol/
│       │   └── SKILL.md                      # name: protocol, user-invocable: false
│       ├── agents/                           # 5 base analysis agents
│       │   ├── phase-auditor.md
│       │   ├── feature-implementer.md
│       │   ├── code-explorer.md
│       │   ├── code-architect.md
│       │   └── impact-analyzer.md
│       └── references/                       # 5 shared reference docs
│           ├── _schema.md
│           ├── audit-protocol.md
│           ├── evidence-catalog.md
│           ├── criteria-template.md
│           └── narrative-convention.md
```

### 4.2 旧 → 新のディレクトリ mapping

| 旧 path | 新 path | 備考 |
|---|---|---|
| `plugins/feature-dev/skills/feature-dev/` | `plugins/belt/skills/feature-dev/` | 子孫ファイル全部 |
| `plugins/bug-fix/skills/bug-fix/` | `plugins/belt/skills/bug-fix/` | 子孫ファイル全部 |
| `plugins/code-review/skills/code-review/` | `plugins/belt/skills/code-review/` | SKILL.md のみ |
| `plugins/code-review/agents/*.md` | `plugins/belt/agents/*.md` | 4 files |
| `plugins/spec-review/skills/spec-review/` | `plugins/belt/skills/spec-review/` | SKILL.md のみ |
| `plugins/spec-review/agents/*.md` | `plugins/belt/agents/*.md` | 3 files |
| `plugins/monkey-test/skills/monkey-test/` | `plugins/belt/skills/monkey-test/` | SKILL.md のみ |
| `plugins/test-scenarios/skills/test-scenarios/` | `plugins/belt/skills/test-scenarios/` | SKILL.md のみ |
| `plugins/belt-agents/agents/*.md` | `plugins/belt-agent/agents/*.md` | 5 files |
| `plugins/belt-agents/references/*.md` | `plugins/belt-agent/references/*.md` | 5 files |
| `plugins/belt-agents/skills/belt-agent/SKILL.md` | `plugins/belt-agent/skills/protocol/SKILL.md` | **skill slug rename + frontmatter `name: protocol`** |

削除対象の空ディレクトリ (7): `plugins/feature-dev/`, `plugins/bug-fix/`, `plugins/code-review/`, `plugins/spec-review/`, `plugins/monkey-test/`, `plugins/test-scenarios/`, `plugins/belt-agents/`

### 4.3 設計上の決定

1. **reviewer agents は `plugins/belt/agents/` にフラット配置**: Claude Code の plugin agent loader は `plugins/<plugin>/agents/*.md` をフラットに読むため、サブディレクトリは namespace に影響しない。観点 (code-review 向け / spec-review 向け) による細分化は不要。
2. **skill 固有 supplement は `plugins/belt/skills/<skill>/references/`**: 例 `feature-dev/references/brainstorming-supplement.md`
3. **共通 references は `plugins/belt-agent/references/`**: 全 skill の criteria から `plugins/belt-agent/references/narrative-convention.md` として参照される絶対 path
4. **`plugins/belt/skills/<skill>/belt.toml`** は現状維持: pipeline.yml の相対 path 解決用

### 4.4 新 `marketplace.json`

```json
{
  "$schema": "https://anthropic.com/claude-code/marketplace.schema.json",
  "name": "belt",
  "description": "Quality-gated AI development pipeline plugins built on the belt workflow engine",
  "owner": { "name": "neko-neko" },
  "plugins": [
    {
      "name": "belt-agent",
      "description": "Foundation: Belt Protocol skill (driver for belt-agent CLI) + 5 analysis agents (phase-auditor, feature-implementer, code-explorer, code-architect, impact-analyzer) + shared references",
      "source": "./plugins/belt-agent",
      "category": "development"
    },
    {
      "name": "belt",
      "description": "User-invocable skills and their reviewer agents: /belt:feature-dev, /belt:bug-fix, /belt:code-review (4 reviewers), /belt:spec-review (3 reviewers), /belt:monkey-test, /belt:test-scenarios. Requires belt-agent plugin",
      "source": "./plugins/belt",
      "category": "development"
    }
  ]
}
```

### 4.5 Install フロー

```
# In Claude Code:

# 1. External deps (unchanged)
/install-plugin obra/superpowers-marketplace superpowers
/install-plugin max-sixty/worktrunk worktrunk
/install-plugin vercel-labs/agent-browser agent-browser

# 2. belt (2 plugins only)
/install-plugin neko-neko/belt belt-agent
/install-plugin neko-neko/belt belt
```

## 5. 影響範囲

### 5.1 ディレクトリ移動 (git mv)

合計 55 files (§ 4.2 mapping)。`git mv -k` で内容 drift ゼロを保証し、`git diff --cached --stat -M` で rename 判定されることを verify。

### 5.2 新規/削除される設定ファイル

- **新規**: `plugins/belt/.claude-plugin/plugin.json`、`plugins/belt-agent/.claude-plugin/plugin.json`
- **削除**: 旧 7 個の `plugins/<plugin>/.claude-plugin/plugin.json`
- **全面書き換え**: `.claude-plugin/marketplace.json` (7 entries → 2 entries)

### 5.3 テキスト置換 (files after git mv を対象)

#### 5.3.1 `belt-agents:` → `belt-agent:` (base agent namespace rename)

置換対象: `belt-agents:phase-auditor`, `belt-agents:feature-implementer`, `belt-agents:code-explorer`, `belt-agents:code-architect`, `belt-agents:impact-analyzer`

| File (移動後の新 path) | 出現数 |
|---|---|
| `plugins/belt/skills/bug-fix/SKILL.md` | 2 |
| `plugins/belt/skills/bug-fix/criteria/execute.md` | 2 |
| `plugins/belt/skills/bug-fix/criteria/dogfood.md` | 1 |
| `plugins/belt/skills/bug-fix/criteria/integrate.md` | 1 |
| `plugins/belt/skills/bug-fix/criteria/rca.md` | 5 |
| `plugins/belt/skills/bug-fix/criteria/code-review.md` | 1 |
| `plugins/belt/skills/bug-fix/criteria/fix-plan-review.md` | 1 |
| `plugins/belt/skills/bug-fix/criteria/fix-plan.md` | 1 |
| `plugins/belt/skills/bug-fix/criteria/monkey-test.md` | 1 |
| `plugins/belt/skills/bug-fix/references/rca-supplement.md` | 3 |
| `plugins/belt/skills/feature-dev/SKILL.md` | 1 |
| `plugins/belt/skills/feature-dev/criteria/execute.md` | 2 |
| `plugins/belt/skills/feature-dev/criteria/code-review.md` | 1 |
| `plugins/belt/skills/feature-dev/references/brainstorming-supplement.md` | 3 |
| **Total** | **25** |

**制約**: `shared_criteria_parity.rs` が feature-dev と bug-fix の `execute.md` / `code-review.md` を byte-identical に lock しているため、両者を**同一 commit で同一差分**で更新必須。

#### 5.3.2 `code-review:<reviewer>` / `spec-review:<reviewer>` → `belt:<reviewer>`

| File | 出現数 |
|---|---|
| `plugins/belt/skills/code-review/SKILL.md` (`Task(subagent_type: ...)` 記述内) | 4 |
| `plugins/belt/skills/spec-review/SKILL.md` | 3 |

#### 5.3.3 `/<skill>:<skill>` → `/belt:<skill>`

| File | 対象文字列 | 出現数 |
|---|---|---|
| `plugins/belt/skills/feature-dev/pipeline.yml` | `/test-scenarios:...`, `/spec-review:...`, `/code-review:...`, `/monkey-test:...` | 4 |
| `plugins/belt/skills/feature-dev/criteria/code-review.md` | `/code-review:...` | 1 |
| `plugins/belt/skills/bug-fix/pipeline.yml` | `/spec-review:...`, `/code-review:...`, `/monkey-test:...` | 3 |
| `plugins/belt/skills/bug-fix/SKILL.md` | `/spec-review:...` ×2, `/code-review:...` ×1 | 3 |
| `plugins/belt/skills/bug-fix/criteria/code-review.md` | `/code-review:...` | 1 |
| `plugins/belt/skills/bug-fix/criteria/fix-plan-review.md` | `/spec-review:...` | 2 |
| `plugins/belt/skills/bug-fix/criteria/monkey-test.md` | `/monkey-test:...` | 1 |
| `plugins/belt/skills/bug-fix/references/monkey-test-supplement.md` | `/monkey-test:...` | 3 |
| `README.md` (Usage セクション L301-304) | 4 skill invocations | 4 |
| **Total** | | **22** |

#### 5.3.4 `plugins/belt-agents/references/` → `plugins/belt-agent/references/`

| File | 出現数 |
|---|---|
| `plugins/belt/skills/feature-dev/SKILL.md` | 2 |
| `plugins/belt/skills/feature-dev/criteria/{design,plan,code-review,execute,monkey-test,dogfood}.md` | 6 |
| `plugins/belt/skills/bug-fix/SKILL.md` | 2 |
| `plugins/belt/skills/bug-fix/criteria/{rca,fix-plan,execute,code-review,monkey-test,dogfood}.md` | 6 |
| `plugins/belt-agent/skills/protocol/SKILL.md` (line 98 `audit-protocol.md` 参照) | 1 |
| `plugins/belt-agent/references/narrative-convention.md` (line 97 自己言及) | 1 |
| **Total** | **18** |

### 5.4 Pre-existing stale references の修正 (ついで修正)

今回の refactor とは別の既存 bug だが、対象ファイルを touch するため同時に修正:

1. `plugins/belt-agent/agents/phase-auditor.md:43`: `claude/agents/references/evidence-catalog.md` は dotfiles 時代の残存 path。新 path は `./references/evidence-catalog.md` (agent と references が同一 plugin 内のため相対参照で十分)
2. `plugins/belt-agent/skills/protocol/SKILL.md:150`: JSON example で `"pipeline": "../spec-review/pipeline.yml"` を参照しているが、spec-review の pipeline.yml は 2026-04-16 refactor で削除済み。**抽象化**を採用し、`"pipeline": "./nested-pipeline.yml"` のようなプレースホルダに差し替える (具体 path 埋め込みは将来の構造変更で再度 stale 化するため)

### 5.5 Rust integration test の更新 (path lock)

| Test file | 変更内容 |
|---|---|
| `crates/belt-core/tests/review_skills_refresh.rs` | `REVIEW_PLUGINS` const の構造変更。旧 `(plugin, [agents])` tuple を新構造 `(skill_name, [agents])` + base path `plugins/belt/` に書き直し。`review_plugins_pipeline_yml_is_deleted` / `review_plugins_belt_toml_is_deleted` / `review_plugins_legacy_consolidated_agent_is_deleted` / `review_plugins_new_observation_agents_exist` / `review_plugins_parent_skill_md_references_parallel_dispatch` の path 組み立てロジックを新 layout に合わせる |
| `crates/belt-core/tests/shared_filter_parity.rs` | `CODE_REVIEW_AGENTS` / `SPEC_REVIEW_AGENTS` 2 const の 7 path を `plugins/belt/agents/*.md` に更新 |
| `crates/belt-core/tests/shared_criteria_parity.rs` | 4 path を `plugins/belt/skills/<skill>/criteria/*.md` に更新 |
| `crates/belt-core/tests/feature_dev_refresh.rs` | `feature_dev_pipeline_path()` 内の 1 path 更新 |
| `crates/belt-core/tests/bug_fix_refresh.rs` | `bug_fix_dir()` 内の 1 path 更新 |
| `crates/belt-agent/tests/cli_test.rs:1659` | コメント (`real plugins/feature-dev/skills/feature-dev tree`) の path を新構造に更新 |

### 5.6 README.md の書き換え

- **Plugins table** (L234-244): 7 行 → 2 行 (`belt-agent`, `belt`)
- **External skill dependencies table** (L252-260): `Required by` 列を `/belt:<skill>` 形式に変更
- **Install コマンドリスト** (L271-283): 7 行 → 2 行
- **Internal dependencies** (L289-294): 2 plugin 化で plugin 間 internal dep が消滅 (同一 plugin 内になるため)。この節は全面書き直し
- **Usage section** (L301-304): `/belt:<skill>` 形式に
- **L185**: "7 Claude Code plugins" → "2 Claude Code plugins"
- 他セクションの旧 path mention を最終 grep で洗い出し

### 5.7 CHANGELOG.md

`[Unreleased]` セクションに v0.2.0 の BREAKING note を追加:

```markdown
## [Unreleased] - ReleaseDate

### Changed (BREAKING)
- Plugin consolidation: 7 plugins (`belt-agents`, `feature-dev`, `bug-fix`, `code-review`, `spec-review`, `monkey-test`, `test-scenarios`) → 2 plugins (`belt`, `belt-agent`).
- Skill invocation renamed:
  - `/feature-dev:feature-dev` → `/belt:feature-dev`
  - `/bug-fix:bug-fix` → `/belt:bug-fix`
  - `/code-review:code-review` → `/belt:code-review`
  - `/spec-review:spec-review` → `/belt:spec-review`
  - `/monkey-test:monkey-test` → `/belt:monkey-test`
  - `/test-scenarios:test-scenarios` → `/belt:test-scenarios`
- Agent namespace renamed:
  - `belt-agents:<agent>` → `belt-agent:<agent>` (base 5 agents)
  - `code-review:<reviewer>` → `belt:<reviewer>` (4 reviewers)
  - `spec-review:<reviewer>` → `belt:<reviewer>` (3 reviewers)
- Belt Protocol skill slug: `belt-agent:belt-agent` → `belt-agent:protocol`
- Installation: `/install-plugin neko-neko/belt <plugin>` takes 2 plugin names now (`belt-agent` and `belt`) instead of 7.
```

### 5.8 AGENTS.md (= CLAUDE.md via symlink)

- **L92** の illustrative example (`with: { skill: "/code-review" }`) を `/belt:code-review` に統一するか判断 (optional)
- **新セクション追加**: 「Plugin Architecture」を Non-Goals の前 (または `CLI 命名` 節の直下) に追加し、2 plugin 方針と namespace 規則を宣言:

```markdown
## Plugin Architecture

belt は Claude Code plugin として **2 plugin 構成**で配布する:

| Plugin | 責務 | 呼び出し namespace |
|---|---|---|
| `belt` | user-invocable skills + それに紐づく reviewer agents | `belt:<skill>`, `belt:<reviewer>` |
| `belt-agent` | Belt Protocol driver skill + 汎用 analysis agents + shared references | `belt-agent:protocol`, `belt-agent:<agent>` |

- `belt` は `belt-agent` を依存として要求する (Claude Code plugin manifest に hard dependency field が無いため、README / CHANGELOG で明示)
- Skill tool invoke および agent reference は常に fully-qualified (`/belt:code-review`, `belt-agent:phase-auditor`) で記述する。Shorthand (`/code-review`) は使用禁止
- CLI binary `belt-agent` と plugin `belt-agent` が同名だが、前者は executable、後者は Claude Code config。context で区別する
```

`CLAUDE.md` は `AGENTS.md` への symlink のため、実装時は `git add AGENTS.md` を用いる (MEMORY `project_claude_md_symlink.md` 参照)。

### 5.9 docs/ 配下の historical docs

`docs/specs/` / `docs/plans/` 内の過去 spec / plan には旧 path / 旧 namespace の言及が大量にあるが、**これらは過去時点の記録**であり書き換えない。ただし本 spec の `related` frontmatter で `2026-04-15-belt-plugins-migration-design.md` を "supersedes" として明示する。

### 5.10 影響規模の集計

| カテゴリ | ファイル数 |
|---|---|
| git mv (内容不変) | 55 |
| 新規 plugin.json | 2 |
| 削除 plugin.json | 7 |
| marketplace.json 全面書き換え | 1 |
| 本文テキスト書き換え (union of 5.3.1-5.3.4) | ~29 |
| pre-existing stale 修正 | 2 |
| Rust test 更新 | 6 |
| README.md | 1 |
| CHANGELOG.md | 1 |
| AGENTS.md | 1 |
| **合計 file touches** | **約 101** |

## 6. Migration Plan

### 6.1 実施戦略

- **Worktree 分離**: `.claude/worktrees/<name>/` に隔離作業
- **単一 PR**、内部は **3-4 atomic commits** に分割 (review 可能粒度 + bisect 有効)
- **subagent-driven-development 推奨**: 書き換えが機械的で自己完結なためタスク分割 → verification → commit ループに適合
- **version bump**: v0.1.0 → **v0.2.0** (BREAKING)。PR merge 後に `cargo release minor -x` で別 commit

### 6.2 Commit 分割

#### Commit 1: `belt-agent` plugin への rename (単数化) + skill slug rename

**変更**:
- `git mv plugins/belt-agents/` → `plugins/belt-agent/`
- `git mv plugins/belt-agent/skills/belt-agent/SKILL.md` → `plugins/belt-agent/skills/protocol/SKILL.md`
- `plugins/belt-agent/skills/protocol/SKILL.md` frontmatter `name: belt-agent` → `name: protocol`
- 新規 `plugins/belt-agent/.claude-plugin/plugin.json`
- 全 `belt-agents:<agent>` → `belt-agent:<agent>` 置換 (25 箇所 / 14 files、旧 path のまま)
- 全 `plugins/belt-agents/references/` → `plugins/belt-agent/references/` 置換 (18 箇所 / 15 files)
- Pre-existing stale 修正:
  - `phase-auditor.md:43`: `claude/agents/references/evidence-catalog.md` → `./references/evidence-catalog.md`
  - `protocol/SKILL.md:150`: stale pipeline example を差し替え
- `marketplace.json`: belt-agents entry の name/source/description を belt-agent に更新 (belt entry は Commit 2 で追加)

**Done criteria**:
- `cargo test --workspace` green
- `grep -rn "belt-agents" plugins/ .claude-plugin/ crates/` が 0 件
- `plugins/belt-agents/` ディレクトリ存在しない

**Commit message**: `refactor(plugins): rename belt-agents → belt-agent, skill belt-agent → protocol`

#### Commit 2: 6 user-facing plugins を `belt` plugin に統合

**変更**:
- `git mv plugins/feature-dev/skills/feature-dev/` → `plugins/belt/skills/feature-dev/`
- 同様に bug-fix, code-review, spec-review, monkey-test, test-scenarios
- `git mv plugins/code-review/agents/*.md` → `plugins/belt/agents/*.md` (4 files)
- `git mv plugins/spec-review/agents/*.md` → `plugins/belt/agents/*.md` (3 files)
- 新規 `plugins/belt/.claude-plugin/plugin.json`
- 旧 6 plugin の `plugin.json` 削除
- `marketplace.json`: belt entry を追加 (最終 2 entries)
- 全 `/<skill>:<skill>` → `/belt:<skill>` 置換 (22 箇所 / 9 files)
- 全 `code-review:<reviewer>` / `spec-review:<reviewer>` → `belt:<reviewer>` 置換 (7 箇所 / 2 files)
- Rust test 更新 (6 files):
  - `review_skills_refresh.rs`: 新 layout に対応 (§ 5.5)
  - `shared_filter_parity.rs`: 7 path 更新
  - `shared_criteria_parity.rs`: 4 path 更新
  - `feature_dev_refresh.rs`: 1 path 更新
  - `bug_fix_refresh.rs`: 1 path 更新
  - `cli_test.rs:1659`: コメント更新
- 旧 6 plugin 空ディレクトリ削除

**Done criteria**:
- `cargo test --workspace` green (特に 5 refactor-affected tests)
- `cargo run -p belt -- lint plugins/belt/skills/feature-dev/pipeline.yml` が ok
- `cargo run -p belt -- lint plugins/belt/skills/bug-fix/pipeline.yml` が ok
- `grep -rn "plugins/feature-dev\|plugins/bug-fix\|plugins/code-review\|plugins/spec-review\|plugins/monkey-test\|plugins/test-scenarios" plugins/ crates/ .claude-plugin/` が 0 件

**Commit message**: `refactor(plugins): consolidate 6 user-facing plugins into single "belt" plugin`

#### Commit 3: README / CHANGELOG / AGENTS.md 更新

**変更**:
- `README.md`: § 5.6 の 6 項目
- `CHANGELOG.md`: § 5.7 の BREAKING note
- `AGENTS.md`: § 5.8 の新セクション + optional edit

**Done criteria**:
- `grep -rn "/feature-dev:\|/bug-fix:\|/code-review:\|/spec-review:\|/monkey-test:\|/test-scenarios:" README.md CHANGELOG.md AGENTS.md` が 0 件
- README の Plugins table が 2 行
- CHANGELOG `[Unreleased]` に BREAKING マーカー存在

**Commit message**: `docs: update README/CHANGELOG/AGENTS.md for belt + belt-agent 2-plugin layout`

#### Commit 4: Manual dogfood verification (optional)

最終 grep sweep で取りこぼし検出、Manual local install test 実施。commit 不要なら省略。

```bash
claude --plugin-dir ./plugins/belt-agent --plugin-dir ./plugins/belt
# 起動後: /belt:feature-dev を呼び出し、belt-agent:phase-auditor, belt:security-reviewer が resolve 確認
```

### 6.3 Verification チェックポイント

各 commit 後に実行:

```bash
# (1) Lint
cargo run -p belt -- lint plugins/belt/skills/feature-dev/pipeline.yml
cargo run -p belt -- lint plugins/belt/skills/bug-fix/pipeline.yml

# (2) Test
cargo test --workspace

# (3) Clippy
cargo clippy --workspace -- -D warnings

# (4) Stale reference sweep
grep -rn "belt-agents\|/feature-dev:\|/bug-fix:\|/code-review:\|/spec-review:\|/monkey-test:\|/test-scenarios:\|plugins/feature-dev\|plugins/bug-fix\|plugins/code-review\|plugins/spec-review\|plugins/monkey-test\|plugins/test-scenarios" plugins/ crates/ .claude-plugin/ README.md CHANGELOG.md AGENTS.md
# 期待: 0 件

# (5) Manual plugin loader test (Commit 4 または final check で実施)
claude --plugin-dir ./plugins/belt-agent --plugin-dir ./plugins/belt
```

### 6.4 subagent-driven-development 分割案

もし `/subagent-driven-development` で実行する場合のタスク分割 (14 tasks):

| Task | Scope | Dep | 並列可否 |
|---|---|---|---|
| T1 | Create `plugins/belt-agent/.claude-plugin/plugin.json` + `plugins/belt/.claude-plugin/plugin.json` | — | Yes |
| T2 | `git mv` belt-agents → belt-agent + skill slug rename (protocol) | — | Serial |
| T3 | `git mv` 6 plugins into belt + consolidate agents/ | T2 完了後 (git status clean) | Serial |
| T4 | Text: `belt-agents:` → `belt-agent:` (25 occ, 14 files) | T2, T3 | Yes (異 file) |
| T5 | Text: `plugins/belt-agents/references/` → `plugins/belt-agent/references/` (18 occ, 15 files) | T2, T3 | Yes |
| T6 | Text: `/<skill>:<skill>` → `/belt:<skill>` (22 occ, 9 files) | T3 | Yes |
| T7 | Text: `code-review:<reviewer>` / `spec-review:<reviewer>` → `belt:<reviewer>` (7 occ, 2 files) | T3 | Yes |
| T8 | Rust tests path update (6 files) | T2, T3 | Yes |
| T9 | marketplace.json rewrite (2 entries) | T1 | Yes |
| T10 | Pre-existing stale fixes (phase-auditor.md:43, protocol/SKILL.md:150) | T2 | Yes |
| T11 | README.md rewrite | T4-T7 | Serial (単独 file) |
| T12 | CHANGELOG.md BREAKING note | T11 | Serial |
| T13 | AGENTS.md Plugin Architecture section | T11 | Serial |
| T14 | Manual dogfood + final sweep | T1-T13 all | Serial |

T2 と T3 は git 状態の複雑化を避けるため serial 実行。T4-T10 は独立 file を touch するため並列 subagent 可能。

### 6.5 失敗時のロールバック

- Commit 1, 2 のいずれかで test fail が残存: worktree で `git reset --hard HEAD~1` (main push 前)
- Manual dogfood で plugin loader が resolve しない: `.claude-plugin/marketplace.json` の `source` path と `name` の typo を第一に疑う
- `belt-agent:protocol` が Claude Code で認識されない: SKILL.md frontmatter `name:` (`protocol`) と skill ディレクトリ名 (`protocol/`) の一致を再確認

## 7. Version / Release

- 本 PR は v0.2.0 minor bump (BREAKING)
- PR merge 後、main branch で `cargo release minor -x` 実行 → CHANGELOG placeholder 置換 + tag push
- cargo-dist CI が `v0.2.0` tag を trigger に release artifact 生成
- README の Install 例に含まれる version (v0.1.0) も追随更新 (ただし `/releases/latest/download/` URL を使うため shell installer は URL 固定で済む。tarball 例の `v0.1.0` 記述のみ更新)

## 8. Non-Goals

- plugin 名のさらなる変更 (例: `belt` を `belt-skills` にする等)。`belt` / `belt-agent` 2 plugin で確定
- Claude Code marketplace への登録 (`/install-plugin` は GitHub URL 直接指定で動作)
- `belt-agent` plugin と `belt-agent` CLI binary の rename (両者同名を意図的に維持)
- reviewer agents のさらなる細分化 (観点別プラグイン化)
- historical docs (`docs/specs/`, `docs/plans/`) の retroactive 書き換え
- v0.1.0 利用者向けの backward compatibility alias / deprecation warning (v0.2.0 は clean break)

## 9. Open Questions / Risks

### 9.1 Risks

1. **`shared_criteria_parity` test との衝突**: feature-dev / bug-fix の `execute.md` / `code-review.md` は byte-identical 必須。§ 5.3.1 の置換時に両者を同一 commit / 同一差分で更新必須。
2. **skill slug `protocol` rename の影響範囲**: grep 上では呼び出し参照ゼロ (`belt-agents:belt-agent` 文字列が存在しない) のため、frontmatter `name:` + ディレクトリ rename のみで完結。ただし Claude Code 内部で skill 名を参照している箇所 (skill list UI など) は verify 必要。
3. **移行順序の atomicity**: 段階的 commit にすると中間状態 (例: Commit 1 後で marketplace.json が belt-agent しか宣言していない間) で Claude Code の plugin loader が動作しない。ただし **PR 全体で atomic であれば merge 後に壊れない**ので問題なし。bisect 時のみ中間 commit で loader が半壊する可能性あり、その場合は直前の green commit に戻せばよい。
4. **v0.1.0 released の migration**: 既に v0.1.0 を install した利用者は clean uninstall + reinstall が必要。README に migration guide を記載するか、CHANGELOG BREAKING note のみで済ますか要判断。**CHANGELOG のみで済ます方針を採用** (v0.2.0 pre-GA なので利用者母数が限定的)。
5. **進行中の `.belt/runs/<run_id>/` との互換性**: run state JSON には pipeline 参照が記録されている可能性。進行中 run がある場合は完了してから移行を推奨。

### 9.2 Open Questions

なし。以下は implementation 時の minor 判断で解決可能:

- `README.md` の External skill dependencies table で `Required by` 列の表記粒度 (`/belt:feature-dev design phase` 等) → implementation 時に既存記述スタイルを踏襲

## 10. Open Points to Verify During Implementation

1. **Plugin loader が `plugins/belt/agents/*.md` をフラットに agent として認識するか**: `/install-plugin` による実地確認。`feature-dev` / `bug-fix` で既に `plugins/<plugin>/agents/*.md` 構造が稼働中のため OK のはず。
2. **`plugins/belt/skills/<skill>/` の複数 skill 共存**: `belt-agents` plugin 内に `agents/` と `skills/belt-agent/` が共存する既存例があるため、`belt` plugin 内の `skills/feature-dev/`, `skills/bug-fix/`, ..., `agents/*.md` 共存も問題ない想定。Manual dogfood で verify。
3. **`belt-agent:protocol` の Claude Code 認識**: user-invocable: false の skill が plugin 内部の他 skill から内部参照される仕組みの確認。現状 `belt-agents:belt-agent` として稼働しているため、slug rename 後も同じ機構が働くことを verify。
