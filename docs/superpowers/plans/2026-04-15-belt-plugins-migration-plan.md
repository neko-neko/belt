---
depends-on:
  - docs/superpowers/specs/2026-04-15-belt-plugins-migration-design.md
  - examples/skills
  - examples/criteria
  - examples/references
  - .claude/agents
  - README.md
  - crates/belt-core/tests/feature_dev_refresh.rs
  - crates/belt-core/tests/bug_fix_refresh.rs
  - crates/belt-core/tests/review_skills_refresh.rs
  - crates/belt-agent/tests/cli_test.rs
  - skills/belt-agent/SKILL.md
---

# belt Claude Code plugin 化 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `examples/skills/` の 6 skill と依存 subagents を 7 つの Claude Code plugin (`plugins/` 配下) として自己完結配布単位に再構築する。

**Architecture:** 2 層構造 — 基盤層 `belt-agents` (5 analysis agents + 4 references) と skill 層 6 plugin (feature-dev / bug-fix / code-review / spec-review / monkey-test / test-scenarios)。`vercel-labs/skills` CLI が root `.claude-plugin/marketplace.json` で discovery する。Migration は cp (複製) 戦略で、C1〜C6 の 6 commit 全てで `cargo test --workspace` が pass する。

**Tech Stack:** Claude Code plugin format (`.claude-plugin/plugin.json` + `marketplace.json`) / `vercel-labs/skills` CLI / Rust cargo workspace / belt-core + belt-agent integration tests

---

## File Structure (concept)

```
belt/
├── .claude-plugin/
│   └── marketplace.json                    # 新規 (7 plugins discovery)
├── plugins/                                # 新規ディレクトリ
│   ├── belt-agents/
│   │   ├── .claude-plugin/plugin.json
│   │   ├── agents/ {phase-auditor,feature-implementer,code-explorer,code-architect,impact-analyzer}.md
│   │   └── references/ {_schema,audit-protocol,evidence-catalog,criteria-template}.md
│   ├── feature-dev/
│   │   ├── .claude-plugin/plugin.json
│   │   └── skills/feature-dev/ {SKILL.md,pipeline.yml,belt.toml,criteria/,references/}
│   ├── bug-fix/
│   │   ├── .claude-plugin/plugin.json
│   │   └── skills/bug-fix/ {SKILL.md,pipeline.yml,belt.toml,criteria/,references/}
│   ├── code-review/
│   │   ├── .claude-plugin/plugin.json
│   │   ├── agents/code-reviewer.md
│   │   └── skills/code-review/ {SKILL.md,pipeline.yml,belt.toml}
│   ├── spec-review/
│   │   ├── .claude-plugin/plugin.json
│   │   ├── agents/spec-reviewer.md
│   │   └── skills/spec-review/ {SKILL.md,pipeline.yml,belt.toml}
│   ├── monkey-test/
│   │   ├── .claude-plugin/plugin.json
│   │   └── skills/monkey-test/SKILL.md
│   └── test-scenarios/
│       ├── .claude-plugin/plugin.json
│       └── skills/test-scenarios/SKILL.md
├── crates/belt-core/tests/shared_criteria_parity.rs   # 新規
└── README.md                                          # 編集 (Plugins セクション追加)
```

削除対象 (C5):
- `examples/` ディレクトリ全体
- `.claude/agents/{code-reviewer,spec-reviewer}.md`
- `~/.dotfiles/claude/agents/{phase-auditor,feature-implementer,code-explorer,code-architect,impact-analyzer}.md`
- `~/.dotfiles/claude/agents/references/{criteria-template,evidence-catalog}.md`

---

## Rewrite Rules (全タスク共通リファレンス)

以下の書き換えルールを各 Task で適用する。rewrite 対象は配布される artifact (`plugins/` 配下) のみ。`examples/` 側 (削除予定) は触らない。

### R1. 内部 skill 呼び出しの fully-qualified 化

| 書き換え前 | 書き換え後 | 対象ファイル |
|---|---|---|
| `skill: /code-review` | `skill: /code-review:code-review` | pipeline.yml, SKILL.md |
| `skill: /spec-review` | `skill: /spec-review:spec-review` | pipeline.yml, SKILL.md |
| `skill: /monkey-test` | `skill: /monkey-test:monkey-test` | pipeline.yml, SKILL.md |
| `skill: /test-scenarios` | `skill: /test-scenarios:test-scenarios` | pipeline.yml, SKILL.md |

**書き換え対象外 (外部依存のまま)**: `/brainstorming`, `/writing-plans`, `/subagent-driven-development`, `/systematic-debugging`, `/dogfood`, `/worktrunk`, `/agent-browser`

### R2. agent 呼び出しの fully-qualified 化

| 書き換え前 | 書き換え後 | 備考 |
|---|---|---|
| `- code-reviewer` (pipeline.yml の invoke.agents) | `- code-review:code-reviewer` | code-review plugin 内 |
| `- spec-reviewer` (pipeline.yml の invoke.agents) | `- spec-review:spec-reviewer` | spec-review plugin 内 |
| 本文中の `phase-auditor` | `belt-agents:phase-auditor` | criteria/references/SKILL.md の散文 |
| 本文中の `code-explorer` | `belt-agents:code-explorer` | 同上 |
| 本文中の `code-architect` | `belt-agents:code-architect` | 同上 |
| 本文中の `impact-analyzer` | `belt-agents:impact-analyzer` | 同上 |
| 本文中の `feature-implementer` | `belt-agents:feature-implementer` | 同上 |

### R3. 共有 criteria の相対 path 書き換え

feature-dev と bug-fix の pipeline.yml は現状 `validate: ../../criteria/execute.md` / `validate: ../../criteria/code-review.md` で `examples/criteria/` を参照している。plugin 化後は同 plugin 内の criteria/ に物理複製する：

| 書き換え前 | 書き換え後 |
|---|---|
| `validate: ../../criteria/execute.md` | `validate: ./criteria/execute.md` |
| `validate: ../../criteria/code-review.md` | `validate: ./criteria/code-review.md` |

### R4. ソース/宛先 path マッピング (cp 時参照)

| ソース (変更前、削除予定) | 宛先 (変更後) |
|---|---|
| `examples/skills/feature-dev/` | `plugins/feature-dev/skills/feature-dev/` |
| `examples/skills/bug-fix/` | `plugins/bug-fix/skills/bug-fix/` |
| `examples/skills/code-review/` | `plugins/code-review/skills/code-review/` |
| `examples/skills/spec-review/` | `plugins/spec-review/skills/spec-review/` |
| `examples/skills/monkey-test/` | `plugins/monkey-test/skills/monkey-test/` |
| `examples/skills/test-scenarios/` | `plugins/test-scenarios/skills/test-scenarios/` |
| `examples/criteria/execute.md` | `plugins/feature-dev/skills/feature-dev/criteria/execute.md` + `plugins/bug-fix/skills/bug-fix/criteria/execute.md` (複製) |
| `examples/criteria/code-review.md` | `plugins/feature-dev/skills/feature-dev/criteria/code-review.md` + `plugins/bug-fix/skills/bug-fix/criteria/code-review.md` (複製) |
| `examples/criteria/_schema.md` | `plugins/belt-agents/references/_schema.md` |
| `examples/references/audit-protocol.md` | `plugins/belt-agents/references/audit-protocol.md` |
| `~/.dotfiles/claude/agents/{5 files}.md` | `plugins/belt-agents/agents/{5 files}.md` |
| `~/.dotfiles/claude/agents/references/criteria-template.md` | `plugins/belt-agents/references/criteria-template.md` |
| `~/.dotfiles/claude/agents/references/evidence-catalog.md` | `plugins/belt-agents/references/evidence-catalog.md` |
| `.claude/agents/code-reviewer.md` | `plugins/code-review/agents/code-reviewer.md` |
| `.claude/agents/spec-reviewer.md` | `plugins/spec-review/agents/spec-reviewer.md` |

---

## Task 1 (C1): Plugin skeleton + marketplace manifest + README Plugins セクション

**Files:**
- Create: `.claude-plugin/marketplace.json`
- Create: `plugins/belt-agents/.claude-plugin/plugin.json`
- Create: `plugins/feature-dev/.claude-plugin/plugin.json`
- Create: `plugins/bug-fix/.claude-plugin/plugin.json`
- Create: `plugins/code-review/.claude-plugin/plugin.json`
- Create: `plugins/spec-review/.claude-plugin/plugin.json`
- Create: `plugins/monkey-test/.claude-plugin/plugin.json`
- Create: `plugins/test-scenarios/.claude-plugin/plugin.json`
- Modify: `README.md` (末尾 `## License` 直前に Plugins セクション追加)

- [ ] **Step 1.1: `.claude-plugin/marketplace.json` を新規作成**

Path: `/Users/nishikataseiichi/go/src/github.com/neko-neko/belt/.claude-plugin/marketplace.json`

```json
{
  "metadata": {
    "pluginRoot": "./plugins"
  },
  "plugins": [
    { "name": "belt-agents",    "source": "belt-agents" },
    { "name": "feature-dev",    "source": "feature-dev",    "skills": ["./skills/feature-dev"] },
    { "name": "bug-fix",        "source": "bug-fix",        "skills": ["./skills/bug-fix"] },
    { "name": "code-review",    "source": "code-review",    "skills": ["./skills/code-review"] },
    { "name": "spec-review",    "source": "spec-review",    "skills": ["./skills/spec-review"] },
    { "name": "monkey-test",    "source": "monkey-test",    "skills": ["./skills/monkey-test"] },
    { "name": "test-scenarios", "source": "test-scenarios", "skills": ["./skills/test-scenarios"] }
  ]
}
```

- [ ] **Step 1.2: `plugins/belt-agents/.claude-plugin/plugin.json` を新規作成**

```json
{
  "name": "belt-agents",
  "description": "Base analysis agents for belt-based quality-gated development pipelines (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer)",
  "version": "0.1.0",
  "author": "neko-neko"
}
```

- [ ] **Step 1.3: `plugins/feature-dev/.claude-plugin/plugin.json` を新規作成**

```json
{
  "name": "feature-dev",
  "description": "Quality-gated 9-phase development pipeline (design → test-scenarios → spec-review → plan → execute → code-review → monkey-test → dogfood → integrate)",
  "version": "0.1.0",
  "author": "neko-neko"
}
```

- [ ] **Step 1.4: `plugins/bug-fix/.claude-plugin/plugin.json` を新規作成**

```json
{
  "name": "bug-fix",
  "description": "Quality-gated 8-phase debugging pipeline (rca → fix-plan → fix-plan-review → execute → code-review → monkey-test → dogfood → integrate)",
  "version": "0.1.0",
  "author": "neko-neko"
}
```

- [ ] **Step 1.5: `plugins/code-review/.claude-plugin/plugin.json` を新規作成**

```json
{
  "name": "code-review",
  "description": "Multi-perspective code review (7 observations: quality, security, performance, test, ai-antipattern, impact, simplification) via consolidated code-reviewer agent",
  "version": "0.1.0",
  "author": "neko-neko"
}
```

- [ ] **Step 1.6: `plugins/spec-review/.claude-plugin/plugin.json` を新規作成**

```json
{
  "name": "spec-review",
  "description": "Multi-perspective spec review (5 observations: requirements, design-judgment, feasibility, consistency, ui-design) + grill-me dialogue via consolidated spec-reviewer agent",
  "version": "0.1.0",
  "author": "neko-neko"
}
```

- [ ] **Step 1.7: `plugins/monkey-test/.claude-plugin/plugin.json` を新規作成**

```json
{
  "name": "monkey-test",
  "description": "Scripted E2E regression via agent-browser. Replays Given/When/Then scenarios and emits human + machine-readable reports",
  "version": "0.1.0",
  "author": "neko-neko"
}
```

- [ ] **Step 1.8: `plugins/test-scenarios/.claude-plugin/plugin.json` を新規作成**

```json
{
  "name": "test-scenarios",
  "description": "Generate ISTQB + ISO 25010 test strategy and optional agent-browser scenarios.yml from a design document",
  "version": "0.1.0",
  "author": "neko-neko"
}
```

- [ ] **Step 1.9: `README.md` の末尾 `## License` 行の直前に Plugins セクションを追加**

File: `/Users/nishikataseiichi/go/src/github.com/neko-neko/belt/README.md`

挿入位置: `## License` 行の直前。既存の `## Build` セクションの後、`## License` の前。

挿入内容 (そのまま追加):

````markdown
## Claude Code Plugins

belt ships 7 Claude Code plugins under `plugins/` — working examples and
production tooling for quality-gated AI-driven development.

### Plugins in this repo

| Plugin | Purpose |
|---|---|
| `belt-agents` | Base layer: 5 analysis agents (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer) + references |
| `feature-dev` | 9-phase development pipeline (design → review → plan → execute → review → e2e → integrate) |
| `bug-fix` | 8-phase debugging pipeline (rca → fix-plan → review → execute → review → e2e → integrate) |
| `code-review` | Multi-perspective code review (7 observations: quality / security / perf / test / ai-antipattern / impact / simplification) |
| `spec-review` | Multi-perspective spec review (5 observations: requirements / design-judgment / feasibility / consistency / ui-design) |
| `monkey-test` | Scripted E2E regression via agent-browser (Given/When/Then replay) |
| `test-scenarios` | Test strategy (ISTQB + ISO 25010) + Given/When/Then scenarios |

### External skill dependencies

`feature-dev` and `bug-fix` invoke skills from other plugins. `monkey-test`
requires the `agent-browser` CLI. Install them before the belt plugins that
use them:

| Dependency | Source | Required by |
|---|---|---|
| `/brainstorming` | [obra/superpowers](https://github.com/obra/superpowers) | feature-dev Phase 1 |
| `/writing-plans` | obra/superpowers | feature-dev Phase 4, bug-fix Phase 2 |
| `/subagent-driven-development` | obra/superpowers | feature-dev Phase 5, bug-fix Phase 4 |
| `/systematic-debugging` | obra/superpowers | bug-fix Phase 1 |
| `/worktrunk` | [max-sixty/worktrunk](https://github.com/max-sixty/worktrunk) | feature-dev Phase 9, bug-fix Phase 8 (integrate) |
| `agent-browser` CLI + `/agent-browser` skill | [vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser) | monkey-test (always), feature-dev Phase 7, bug-fix Phase 6 (when `--e2e`) |
| `/dogfood` | vercel-labs/agent-browser | feature-dev Phase 8, bug-fix Phase 7 (when `--e2e`) |

### Install

Install external dependencies first, then belt plugins. `skills add` supports
both GitHub shorthand (`owner/repo`) and full URLs. Use `-g` for global
install (all projects) or omit for project-local.

```bash
# 1. superpowers (/brainstorming, /writing-plans, /subagent-driven-development, /systematic-debugging)
npx skills add obra/superpowers -g -y

# 2. worktrunk (/worktrunk)
npx skills add max-sixty/worktrunk -g -y

# 3. agent-browser plugin (agent-browser CLI + /dogfood skill)
npx skills add vercel-labs/agent-browser --skill agent-browser --skill dogfood -g -y

# 4. belt plugins (all 7)
npx skills add neko-neko/belt -g -y
```

Selective install (only some belt plugins):

```bash
# Example: code-review only (no external deps)
npx skills add neko-neko/belt --skill code-review -g -y
```

Plugin discovery uses `.claude-plugin/marketplace.json` at belt repo root.

### Internal dependencies (plugin-to-plugin)

- `feature-dev` invokes `spec-review`, `code-review`, `test-scenarios`, `monkey-test`
- `bug-fix` invokes `spec-review`, `code-review`, `monkey-test`
- `feature-dev`, `bug-fix` require `belt-agents` (analysis agents referenced by criteria and supplements)
- `code-review`, `spec-review`, `monkey-test`, `test-scenarios`, `belt-agents` are standalone

### Usage

After install:

```
/feature-dev:feature-dev         # start a new feature
/bug-fix:bug-fix                 # start a bug investigation
/code-review:code-review         # standalone code review
/spec-review:spec-review         # standalone spec review
```

See each plugin's `SKILL.md` for phase details and arg reference.

````

- [ ] **Step 1.10: cargo test を実行して pass を確認**

Run: `cargo test --workspace --no-fail-fast`
Expected: all tests pass (既存テストは `examples/` を引き続き参照しており、新規 `plugins/` は未使用)

- [ ] **Step 1.11: Commit**

```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
git add .claude-plugin/ plugins/ README.md
git commit -m "$(cat <<'EOF'
chore(plugins): add plugin skeleton, marketplace manifest, README section

C1 of the Claude Code plugin migration: create empty plugin scaffolding
(7 plugin.json + marketplace.json) and document plugins in README.
Plugin bodies (agents, skills) arrive in C2/C3.

See docs/superpowers/specs/2026-04-15-belt-plugins-migration-design.md
EOF
)"
```

---

## Task 2 (C2): belt-agents plugin の agents / references を配置

**Files:**
- Create: `plugins/belt-agents/agents/phase-auditor.md` (cp from `~/.dotfiles/claude/agents/phase-auditor.md`)
- Create: `plugins/belt-agents/agents/feature-implementer.md` (cp from `~/.dotfiles/claude/agents/feature-implementer.md`)
- Create: `plugins/belt-agents/agents/code-explorer.md` (cp from `~/.dotfiles/claude/agents/code-explorer.md`)
- Create: `plugins/belt-agents/agents/code-architect.md` (cp from `~/.dotfiles/claude/agents/code-architect.md`)
- Create: `plugins/belt-agents/agents/impact-analyzer.md` (cp from `~/.dotfiles/claude/agents/impact-analyzer.md`)
- Create: `plugins/belt-agents/references/_schema.md` (cp from `examples/criteria/_schema.md`)
- Create: `plugins/belt-agents/references/audit-protocol.md` (cp from `examples/references/audit-protocol.md`)
- Create: `plugins/belt-agents/references/evidence-catalog.md` (cp from `~/.dotfiles/claude/agents/references/evidence-catalog.md`)
- Create: `plugins/belt-agents/references/criteria-template.md` (cp from `~/.dotfiles/claude/agents/references/criteria-template.md`)

- [ ] **Step 2.1: 5 agents を cp**

```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
cp ~/.dotfiles/claude/agents/phase-auditor.md       plugins/belt-agents/agents/phase-auditor.md
cp ~/.dotfiles/claude/agents/feature-implementer.md plugins/belt-agents/agents/feature-implementer.md
cp ~/.dotfiles/claude/agents/code-explorer.md       plugins/belt-agents/agents/code-explorer.md
cp ~/.dotfiles/claude/agents/code-architect.md      plugins/belt-agents/agents/code-architect.md
cp ~/.dotfiles/claude/agents/impact-analyzer.md     plugins/belt-agents/agents/impact-analyzer.md
```

- [ ] **Step 2.2: 4 references を cp**

```bash
cp examples/criteria/_schema.md                              plugins/belt-agents/references/_schema.md
cp examples/references/audit-protocol.md                     plugins/belt-agents/references/audit-protocol.md
cp ~/.dotfiles/claude/agents/references/evidence-catalog.md  plugins/belt-agents/references/evidence-catalog.md
cp ~/.dotfiles/claude/agents/references/criteria-template.md plugins/belt-agents/references/criteria-template.md
```

- [ ] **Step 2.3: 配置確認**

Run:
```bash
ls plugins/belt-agents/agents/ plugins/belt-agents/references/
```

Expected:
```
plugins/belt-agents/agents/:
code-architect.md  code-explorer.md  feature-implementer.md  impact-analyzer.md  phase-auditor.md

plugins/belt-agents/references/:
_schema.md  audit-protocol.md  criteria-template.md  evidence-catalog.md
```

- [ ] **Step 2.4: cargo test を実行して pass を確認**

Run: `cargo test --workspace --no-fail-fast`
Expected: all tests pass (新規配置は既存テストに無影響)

- [ ] **Step 2.5: Commit**

```bash
git add plugins/belt-agents/
git commit -m "$(cat <<'EOF'
chore(plugins): populate belt-agents with 5 agents + 4 references

C2 of the Claude Code plugin migration: cp existing agent/reference
files into the belt-agents plugin. Old copies remain in
~/.dotfiles/claude/agents and examples/ until C5 deletes them, so tests
stay green throughout the migration.

Agents (from ~/.dotfiles/claude/agents/):
- phase-auditor, feature-implementer, code-explorer, code-architect,
  impact-analyzer

References:
- _schema.md, audit-protocol.md (from examples/)
- evidence-catalog.md, criteria-template.md (from ~/.dotfiles/claude/agents/references/)
EOF
)"
```

---

## Task 3 (C3): 各 skill plugin に skill 本体と reviewer agent を配置 + rewrite

最大のコミット。6 skill を cp し、shared criteria を複製し、全 agent/skill 参照を fully-qualified に書き換える。

**Files:**
- Create: `plugins/feature-dev/skills/feature-dev/{SKILL.md,pipeline.yml,belt.toml,criteria/*.md,references/*.md}` (cp + rewrite)
- Create: `plugins/feature-dev/skills/feature-dev/criteria/execute.md` (cp from `examples/criteria/execute.md`)
- Create: `plugins/feature-dev/skills/feature-dev/criteria/code-review.md` (cp from `examples/criteria/code-review.md`)
- Create: `plugins/bug-fix/skills/bug-fix/{SKILL.md,pipeline.yml,belt.toml,criteria/*.md,references/*.md}` (cp + rewrite)
- Create: `plugins/bug-fix/skills/bug-fix/criteria/execute.md` (cp from `examples/criteria/execute.md`)
- Create: `plugins/bug-fix/skills/bug-fix/criteria/code-review.md` (cp from `examples/criteria/code-review.md`)
- Create: `plugins/code-review/skills/code-review/{SKILL.md,pipeline.yml,belt.toml}` (cp + rewrite)
- Create: `plugins/code-review/agents/code-reviewer.md` (cp from `.claude/agents/code-reviewer.md`)
- Create: `plugins/spec-review/skills/spec-review/{SKILL.md,pipeline.yml,belt.toml}` (cp + rewrite)
- Create: `plugins/spec-review/agents/spec-reviewer.md` (cp from `.claude/agents/spec-reviewer.md`)
- Create: `plugins/monkey-test/skills/monkey-test/SKILL.md` (cp + rewrite)
- Create: `plugins/test-scenarios/skills/test-scenarios/SKILL.md` (cp from `examples/skills/test-scenarios/SKILL.md` — 変更不要)

- [ ] **Step 3.1: 6 skill のディレクトリ構造を cp で複製**

```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt

# feature-dev: SKILL.md + pipeline.yml + belt.toml + criteria/ + references/
mkdir -p plugins/feature-dev/skills/feature-dev/criteria plugins/feature-dev/skills/feature-dev/references
cp examples/skills/feature-dev/SKILL.md     plugins/feature-dev/skills/feature-dev/SKILL.md
cp examples/skills/feature-dev/pipeline.yml plugins/feature-dev/skills/feature-dev/pipeline.yml
cp examples/skills/feature-dev/belt.toml    plugins/feature-dev/skills/feature-dev/belt.toml
cp examples/skills/feature-dev/criteria/*.md   plugins/feature-dev/skills/feature-dev/criteria/
cp examples/skills/feature-dev/references/*.md plugins/feature-dev/skills/feature-dev/references/

# bug-fix: 同様
mkdir -p plugins/bug-fix/skills/bug-fix/criteria plugins/bug-fix/skills/bug-fix/references
cp examples/skills/bug-fix/SKILL.md     plugins/bug-fix/skills/bug-fix/SKILL.md
cp examples/skills/bug-fix/pipeline.yml plugins/bug-fix/skills/bug-fix/pipeline.yml
cp examples/skills/bug-fix/belt.toml    plugins/bug-fix/skills/bug-fix/belt.toml
cp examples/skills/bug-fix/criteria/*.md   plugins/bug-fix/skills/bug-fix/criteria/
cp examples/skills/bug-fix/references/*.md plugins/bug-fix/skills/bug-fix/references/

# code-review: SKILL.md + pipeline.yml + belt.toml
mkdir -p plugins/code-review/skills/code-review plugins/code-review/agents
cp examples/skills/code-review/SKILL.md     plugins/code-review/skills/code-review/SKILL.md
cp examples/skills/code-review/pipeline.yml plugins/code-review/skills/code-review/pipeline.yml
cp examples/skills/code-review/belt.toml    plugins/code-review/skills/code-review/belt.toml

# spec-review: 同様
mkdir -p plugins/spec-review/skills/spec-review plugins/spec-review/agents
cp examples/skills/spec-review/SKILL.md     plugins/spec-review/skills/spec-review/SKILL.md
cp examples/skills/spec-review/pipeline.yml plugins/spec-review/skills/spec-review/pipeline.yml
cp examples/skills/spec-review/belt.toml    plugins/spec-review/skills/spec-review/belt.toml

# monkey-test: SKILL.md のみ
mkdir -p plugins/monkey-test/skills/monkey-test
cp examples/skills/monkey-test/SKILL.md plugins/monkey-test/skills/monkey-test/SKILL.md

# test-scenarios: SKILL.md のみ (書き換え不要)
mkdir -p plugins/test-scenarios/skills/test-scenarios
cp examples/skills/test-scenarios/SKILL.md plugins/test-scenarios/skills/test-scenarios/SKILL.md
```

- [ ] **Step 3.2: shared criteria を feature-dev と bug-fix の criteria/ に複製**

```bash
cp examples/criteria/execute.md      plugins/feature-dev/skills/feature-dev/criteria/execute.md
cp examples/criteria/code-review.md  plugins/feature-dev/skills/feature-dev/criteria/code-review.md
cp examples/criteria/execute.md      plugins/bug-fix/skills/bug-fix/criteria/execute.md
cp examples/criteria/code-review.md  plugins/bug-fix/skills/bug-fix/criteria/code-review.md
```

- [ ] **Step 3.3: reviewer agents を各 review plugin に cp**

```bash
cp .claude/agents/code-reviewer.md plugins/code-review/agents/code-reviewer.md
cp .claude/agents/spec-reviewer.md plugins/spec-review/agents/spec-reviewer.md
```

- [ ] **Step 3.4: `plugins/feature-dev/skills/feature-dev/pipeline.yml` を書き換え**

Rewrite R1 (内部 skill fully-qualified) と R3 (共有 criteria path):

Path: `plugins/feature-dev/skills/feature-dev/pipeline.yml`

以下 5 箇所の edit:

1. `skill: /test-scenarios` → `skill: /test-scenarios:test-scenarios`
2. `skill: /spec-review` → `skill: /spec-review:spec-review`
3. `skill: /code-review` → `skill: /code-review:code-review`
4. `skill: /monkey-test` → `skill: /monkey-test:monkey-test`
5. `validate: ../../criteria/execute.md` → `validate: ./criteria/execute.md`
6. `validate: ../../criteria/code-review.md` → `validate: ./criteria/code-review.md`

(`skill: /brainstorming`, `skill: /writing-plans`, `skill: /subagent-driven-development`, `skill: /dogfood`, `skill: /worktrunk` は外部依存なので書き換え対象外)

- [ ] **Step 3.5: `plugins/bug-fix/skills/bug-fix/pipeline.yml` を書き換え**

Path: `plugins/bug-fix/skills/bug-fix/pipeline.yml`

以下 4 箇所の edit:

1. `skill: /spec-review` → `skill: /spec-review:spec-review`
2. `skill: /code-review` → `skill: /code-review:code-review`
3. `skill: /monkey-test` → `skill: /monkey-test:monkey-test`
4. `validate: ../../criteria/execute.md` → `validate: ./criteria/execute.md`
5. `validate: ../../criteria/code-review.md` → `validate: ./criteria/code-review.md`

(`skill: /systematic-debugging`, `skill: /writing-plans`, `skill: /subagent-driven-development`, `skill: /dogfood`, `skill: /worktrunk` は外部依存)

- [ ] **Step 3.6: `plugins/code-review/skills/code-review/pipeline.yml` を書き換え (R2)**

Path: `plugins/code-review/skills/code-review/pipeline.yml`

Edit 1 箇所:

```yaml
# 変更前:
      agents:
        - code-reviewer
# 変更後:
      agents:
        - code-review:code-reviewer
```

- [ ] **Step 3.7: `plugins/spec-review/skills/spec-review/pipeline.yml` を書き換え (R2)**

Path: `plugins/spec-review/skills/spec-review/pipeline.yml`

Edit 1 箇所:

```yaml
# 変更前:
      agents:
        - spec-reviewer
# 変更後:
      agents:
        - spec-review:spec-reviewer
```

- [ ] **Step 3.8: feature-dev の SKILL.md で内部 skill 呼び出しを fully-qualified 化**

Path: `plugins/feature-dev/skills/feature-dev/SKILL.md`

Edit 対象の INVOKE 行 (`/brainstorming`, `/writing-plans`, `/subagent-driven-development`, `/dogfood`, `/worktrunk` は対象外):

- `Skill tool /test-scenarios with` → `Skill tool /test-scenarios:test-scenarios with`
- `Skill tool /spec-review with` → `Skill tool /spec-review:spec-review with`
- `Skill tool /code-review with` → `Skill tool /code-review:code-review with`
- `Skill tool /monkey-test.` → `Skill tool /monkey-test:monkey-test.`

正確な箇所は Grep で再確認してから適用：
```bash
grep -n "Skill tool /" plugins/feature-dev/skills/feature-dev/SKILL.md
```

- [ ] **Step 3.9: feature-dev の criteria/*.md と references/*.md で agent 本文参照を fully-qualified 化 (R2)**

対象ディレクトリ: `plugins/feature-dev/skills/feature-dev/criteria/`, `plugins/feature-dev/skills/feature-dev/references/`

以下の文字列をそれぞれ `belt-agents:<agent>` に置換。置換対象は Markdown 本文のみ (コードブロック内の shell 出力例などは文脈判断)。:
- `phase-auditor` → `belt-agents:phase-auditor` (ただし既に `belt-agents:` 付きの行は除く)
- `code-explorer` → `belt-agents:code-explorer`
- `code-architect` → `belt-agents:code-architect`
- `impact-analyzer` → `belt-agents:impact-analyzer`
- `feature-implementer` → `belt-agents:feature-implementer`

```bash
# 確認:
grep -nE "\b(phase-auditor|code-explorer|code-architect|impact-analyzer|feature-implementer)\b" \
  plugins/feature-dev/skills/feature-dev/criteria/*.md \
  plugins/feature-dev/skills/feature-dev/references/*.md
```

Edit tool で 1 箇所ずつ置換し、`belt-agents:` が二重に付かないよう注意する。

- [ ] **Step 3.10: bug-fix の SKILL.md / criteria/*.md / references/*.md で同様の書き換え**

対象: `plugins/bug-fix/skills/bug-fix/{SKILL.md,criteria/*.md,references/*.md}`

適用ルール:
- Step 3.5 と同じ内部 skill rewrite (`/spec-review`, `/code-review`, `/monkey-test`)
- Step 3.9 と同じ agent 本文参照書き換え

```bash
# 事前確認:
grep -nE "/spec-review|/code-review|/monkey-test|/test-scenarios|\b(phase-auditor|code-explorer|code-architect|impact-analyzer|feature-implementer)\b" \
  plugins/bug-fix/skills/bug-fix/SKILL.md \
  plugins/bug-fix/skills/bug-fix/criteria/*.md \
  plugins/bug-fix/skills/bug-fix/references/*.md
```

- [ ] **Step 3.11: code-review / spec-review の SKILL.md で agent 本文参照を fully-qualified 化**

対象: `plugins/code-review/skills/code-review/SKILL.md`, `plugins/spec-review/skills/spec-review/SKILL.md`

`code-reviewer` / `spec-reviewer` の本文言及を各々 `code-review:code-reviewer` / `spec-review:spec-reviewer` に置換。

```bash
grep -nE "\bcode-reviewer\b" plugins/code-review/skills/code-review/SKILL.md
grep -nE "\bspec-reviewer\b" plugins/spec-review/skills/spec-review/SKILL.md
```

- [ ] **Step 3.12: monkey-test SKILL.md で `/dogfood` 本文参照はそのまま、内部 skill 参照があれば書き換え**

対象: `plugins/monkey-test/skills/monkey-test/SKILL.md`

`/dogfood` は外部 skill (vercel-labs/agent-browser) なのでそのまま。Red Flags / References の中で `/monkey-test`, `/test-scenarios` 言及があれば fully-qualified 化。

```bash
grep -nE "/monkey-test|/test-scenarios|/code-review|/spec-review" plugins/monkey-test/skills/monkey-test/SKILL.md
```

該当箇所のみ手動 edit。

- [ ] **Step 3.13: test-scenarios SKILL.md (変更なし確認)**

Path: `plugins/test-scenarios/skills/test-scenarios/SKILL.md`

確認:
```bash
grep -nE "/monkey-test|/test-scenarios|/code-review|/spec-review|\b(phase-auditor|code-explorer|code-architect|impact-analyzer|feature-implementer|code-reviewer|spec-reviewer)\b" \
  plugins/test-scenarios/skills/test-scenarios/SKILL.md
```

Expected: 自己言及 (`/test-scenarios`) のみ。必要なら fully-qualified 化。

- [ ] **Step 3.14: 全体再確認 — skill 参照と agent 参照の drift チェック**

```bash
# 内部 skill 呼び出しで fully-qualified 化されていない箇所が残っていないか
grep -rnE "skill: */(code-review|spec-review|monkey-test|test-scenarios)[^:]" plugins/

# agent 呼び出し (invoke.agents) で shorthand が残っていないか
grep -rnE "^\s*-\s+(code-reviewer|spec-reviewer|phase-auditor|code-explorer|code-architect|impact-analyzer|feature-implementer)\s*$" plugins/

# 本文中の agent 言及で shorthand が残っていないか (false positive はレビューで許容)
grep -rnE "\b(phase-auditor|code-explorer|code-architect|impact-analyzer|feature-implementer|code-reviewer|spec-reviewer)\b" plugins/ \
  | grep -v "belt-agents:" | grep -v "code-review:" | grep -v "spec-review:" | grep -v "^Binary "
```

Expected: 最初の 2 つは empty。3 つめは自己 plugin 定義行 (`name: code-reviewer` の frontmatter など) のみ残る。

- [ ] **Step 3.15: cargo test を実行**

Run: `cargo test --workspace --no-fail-fast`
Expected: all tests pass (既存テストはまだ `examples/` を参照)

- [ ] **Step 3.16: Commit**

```bash
git add plugins/
git commit -m "$(cat <<'EOF'
chore(plugins): populate skill plugins + rewrite agent/skill references

C3 of the Claude Code plugin migration: cp the 6 skills into plugins/,
duplicate shared criteria into feature-dev and bug-fix, cp reviewer
agents into code-review and spec-review plugins, and rewrite all
intra-artifact references to fully-qualified form.

- internal skill calls: /code-review → /code-review:code-review, etc.
- invoke.agents: code-reviewer → code-review:code-reviewer, etc.
- narrative agent mentions: phase-auditor → belt-agents:phase-auditor, etc.
- shared criteria path: ../../criteria/... → ./criteria/...

External skill calls (/brainstorming, /writing-plans,
/subagent-driven-development, /systematic-debugging, /dogfood,
/worktrunk, /agent-browser) stay unqualified.

Old examples/ and .claude/agents/ copies remain until C5.
EOF
)"
```

---

## Task 4 (C4): test path を `plugins/` に切替 + drift parity test 追加

**Files:**
- Modify: `crates/belt-core/tests/feature_dev_refresh.rs:15`
- Modify: `crates/belt-core/tests/bug_fix_refresh.rs:32,267-274`
- Modify: `crates/belt-core/tests/review_skills_refresh.rs:21`
- Modify: `crates/belt-agent/tests/cli_test.rs:1652-1665`
- Create: `crates/belt-core/tests/shared_criteria_parity.rs`

- [ ] **Step 4.1: `crates/belt-core/tests/feature_dev_refresh.rs` の path を更新**

Edit 対象: line 15

```rust
// 変更前:
    path.push("examples/skills/feature-dev/pipeline.yml");
// 変更後:
    path.push("plugins/feature-dev/skills/feature-dev/pipeline.yml");
```

- [ ] **Step 4.2: `crates/belt-core/tests/bug_fix_refresh.rs` の path を更新**

Edit 対象: line 32, 266-273

```rust
// line 32 変更前:
    repo_root().join("examples/skills/bug-fix")
// 変更後:
    repo_root().join("plugins/bug-fix/skills/bug-fix")
```

line 266-274 付近 (`// Shared criteria:` 以下) のブロックを plugin 内 criteria/ 確認に書き換える：

```rust
// 変更前:
    // Shared criteria: pipeline.yml uses `../../criteria/` relative to
    // `examples/skills/bug-fix/`, resolving to `examples/criteria/`.
    let shared = repo_root().join("examples/criteria");
    for name in ["execute.md", "code-review.md"] {
        assert!(
            shared.join(name).exists(),
            "shared criteria '{name}' must exist at examples/criteria/"
        );
    }
// 変更後:
    // Shared criteria: after plugin migration, pipeline.yml uses `./criteria/`
    // and the shared files are physically duplicated into each plugin.
    // Drift between feature-dev and bug-fix copies is checked in
    // `shared_criteria_parity.rs`.
    for name in ["execute.md", "code-review.md"] {
        assert!(
            criteria_dir.join(name).exists(),
            "duplicated shared criteria '{name}' must exist at {}",
            criteria_dir.display()
        );
    }
```

- [ ] **Step 4.3: `crates/belt-core/tests/review_skills_refresh.rs` の path を更新**

Edit 対象: line 21

```rust
// 変更前:
    repo_root().join(format!("examples/skills/{skill}/pipeline.yml"))
// 変更後:
    repo_root().join(format!("plugins/{skill}/skills/{skill}/pipeline.yml"))
```

- [ ] **Step 4.4: `crates/belt-agent/tests/cli_test.rs` の path を更新**

Edit 対象: line 1660-1665

```rust
// 変更前 (line 1660-1665):
    let pipeline = workspace
        .join("examples")
        .join("skills")
        .join("feature-dev")
        .join("pipeline.yml");
// 変更後:
    let pipeline = workspace
        .join("plugins")
        .join("feature-dev")
        .join("skills")
        .join("feature-dev")
        .join("pipeline.yml");
```

また line 1651-1653 付近のコメントも整合化:

```rust
// 変更前:
/// End-to-end walk through the migrated feature-dev pipeline using the
/// real examples/skills/feature-dev tree. This test is not meant to simulate
// 変更後:
/// End-to-end walk through the migrated feature-dev pipeline using the
/// real plugins/feature-dev/skills/feature-dev tree. This test is not meant to simulate
```

- [ ] **Step 4.5: `crates/belt-core/tests/shared_criteria_parity.rs` を新規作成**

Path: `/Users/nishikataseiichi/go/src/github.com/neko-neko/belt/crates/belt-core/tests/shared_criteria_parity.rs`

```rust
//! Integration test to detect drift between feature-dev and bug-fix shared
//! criteria files (execute.md, code-review.md).
//!
//! After the plugin migration belt-agent cannot resolve cross-plugin paths,
//! so the two files are physically duplicated. This parity test fails fast
//! if they ever diverge.

use std::fs;
use std::path::PathBuf;

fn workspace_path(rel: &str) -> PathBuf {
    // CARGO_MANIFEST_DIR points at crates/belt-core; walk two levels up to
    // reach the workspace root, then join the plugin-relative path.
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path.push(rel);
    path
}

#[test]
fn execute_criteria_identical_across_feature_dev_and_bug_fix() {
    let fd = fs::read_to_string(workspace_path(
        "plugins/feature-dev/skills/feature-dev/criteria/execute.md",
    ))
    .expect("feature-dev execute.md missing");
    let bf = fs::read_to_string(workspace_path(
        "plugins/bug-fix/skills/bug-fix/criteria/execute.md",
    ))
    .expect("bug-fix execute.md missing");
    assert_eq!(
        fd, bf,
        "execute.md drift: feature-dev and bug-fix must be byte-identical"
    );
}

#[test]
fn code_review_criteria_identical_across_feature_dev_and_bug_fix() {
    let fd = fs::read_to_string(workspace_path(
        "plugins/feature-dev/skills/feature-dev/criteria/code-review.md",
    ))
    .expect("feature-dev code-review.md missing");
    let bf = fs::read_to_string(workspace_path(
        "plugins/bug-fix/skills/bug-fix/criteria/code-review.md",
    ))
    .expect("bug-fix code-review.md missing");
    assert_eq!(
        fd, bf,
        "code-review.md drift: feature-dev and bug-fix must be byte-identical"
    );
}
```

- [ ] **Step 4.6: cargo test を実行して pass を確認**

Run: `cargo test --workspace --no-fail-fast`
Expected: all tests pass (tests が新しい `plugins/` path を参照。C3 で配置済みのため resolve 可能)

Parity test 2 件も含めて pass すること:
```
test execute_criteria_identical_across_feature_dev_and_bug_fix ... ok
test code_review_criteria_identical_across_feature_dev_and_bug_fix ... ok
```

- [ ] **Step 4.7: `belt lint` で各 pipeline を静的検証**

Run:
```bash
cargo run -p belt -- lint plugins/feature-dev/skills/feature-dev/pipeline.yml
cargo run -p belt -- lint plugins/bug-fix/skills/bug-fix/pipeline.yml
cargo run -p belt -- lint plugins/code-review/skills/code-review/pipeline.yml
cargo run -p belt -- lint plugins/spec-review/skills/spec-review/pipeline.yml
```

Expected: 全て `OK` (belt lint の validation rules をパス)

- [ ] **Step 4.8: Commit**

```bash
git add crates/belt-core/tests/ crates/belt-agent/tests/
git commit -m "$(cat <<'EOF'
test: switch pipeline/skill tests to plugins/ paths + parity test

C4 of the Claude Code plugin migration: update the four integration
test files that hard-code examples/skills paths to point at the new
plugins/ layout, and add shared_criteria_parity.rs to detect drift
between the duplicated execute.md / code-review.md copies.

Tests continue to pass because C3 already populated plugins/.
examples/ is still around for safety until C5 removes it.
EOF
)"
```

---

## Task 5 (C5): 旧ファイル削除 (belt repo)

**Files:**
- Delete: `examples/` ディレクトリ全体 (`examples/skills/`, `examples/criteria/`, `examples/references/`, `examples/.gitkeep`)
- Delete: `.claude/agents/code-reviewer.md`
- Delete: `.claude/agents/spec-reviewer.md`

**注記:** `~/.dotfiles/claude/agents/` 配下の削除は belt repo と別リポジトリなので**この commit には含めない**。Task 6 の後 (belt repo PR マージ後) に dotfiles 側で別 commit を打つ。

- [ ] **Step 5.1: `examples/` を削除**

```bash
cd /Users/nishikataseiichi/go/src/github.com/neko-neko/belt
rm -rf examples/
```

- [ ] **Step 5.2: belt repo `.claude/agents/{code-reviewer,spec-reviewer}.md` を削除**

```bash
rm .claude/agents/code-reviewer.md
rm .claude/agents/spec-reviewer.md
```

- [ ] **Step 5.3: 残存チェック**

Run:
```bash
ls examples/ 2>/dev/null && echo "FAIL: examples/ still exists" || echo "OK: examples/ removed"
ls .claude/agents/ | grep -E "code-reviewer|spec-reviewer" && echo "FAIL: reviewer agents still present" || echo "OK: reviewer agents removed"
```

Expected:
```
OK: examples/ removed
OK: reviewer agents removed
```

- [ ] **Step 5.4: cargo test を実行**

Run: `cargo test --workspace --no-fail-fast`
Expected: all tests pass (tests は既に `plugins/` を参照しており `examples/` 不要)

- [ ] **Step 5.5: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
chore(plugins): remove legacy examples/ and .claude/agents/ reviewers

C5 of the Claude Code plugin migration: delete the old directories now
that plugins/ is the source of truth and all tests + pipeline lints
pass against the new paths.

Removed:
- examples/ (skills/, criteria/, references/, .gitkeep)
- .claude/agents/code-reviewer.md (moved to plugins/code-review/agents/)
- .claude/agents/spec-reviewer.md (moved to plugins/spec-review/agents/)

Dotfiles cleanup (~/.dotfiles/claude/agents/{5 agents, 2 references}) is
tracked out-of-band and will be removed after this PR is merged.
EOF
)"
```

---

## Task 6 (C6): `skills/belt-agent/SKILL.md` の reference path 更新

**Files:**
- Modify: `skills/belt-agent/SKILL.md:100`

- [ ] **Step 6.1: `skills/belt-agent/SKILL.md` の reference path を更新**

Path: `/Users/nishikataseiichi/go/src/github.com/neko-neko/belt/skills/belt-agent/SKILL.md`

Edit 対象: line 100

```markdown
# 変更前:
When a validate entry is a file reference, the orchestrator MUST read the file before running `step --confirm`. The file contains the actual criteria; the scalar/struct in `pipeline.yml` is just the pointer. See `examples/references/audit-protocol.md` for the expected criteria file format.

# 変更後:
When a validate entry is a file reference, the orchestrator MUST read the file before running `step --confirm`. The file contains the actual criteria; the scalar/struct in `pipeline.yml` is just the pointer. See `plugins/belt-agents/references/audit-protocol.md` for the expected criteria file format.
```

- [ ] **Step 6.2: 残存チェック — 他に `examples/` 言及が belt-agent skill に残っていないか**

```bash
grep -n "examples/" skills/belt-agent/SKILL.md
```

Expected: empty

- [ ] **Step 6.3: repo 全体で `examples/` の残存チェック (docs/ は除外)**

```bash
grep -rn "examples/skills\|examples/criteria\|examples/references" \
  --include='*.md' --include='*.yml' --include='*.rs' --include='*.toml' \
  --exclude-dir=docs --exclude-dir=target .
```

Expected: no hits (docs/specs/, docs/plans/, memory/ は historical なので対象外)

- [ ] **Step 6.4: cargo test を実行**

Run: `cargo test --workspace --no-fail-fast`
Expected: all tests pass

- [ ] **Step 6.5: Commit**

```bash
git add skills/belt-agent/SKILL.md
git commit -m "$(cat <<'EOF'
docs(belt-agent): repoint audit-protocol reference to plugins/belt-agents

C6 of the Claude Code plugin migration: the belt-agent SKILL.md linked
to examples/references/audit-protocol.md, which C5 deleted. Repoint to
the plugin-local copy at plugins/belt-agents/references/audit-protocol.md.
EOF
)"
```

---

## Post-migration: dotfiles cleanup (別リポジトリ)

belt repo の PR がマージされたあと、`~/.dotfiles` 側で以下の cleanup commit を打つ：

- [ ] **Step 7.1: dotfiles で 5 agents + 2 references を削除**

```bash
cd ~/.dotfiles
rm claude/agents/phase-auditor.md
rm claude/agents/feature-implementer.md
rm claude/agents/code-explorer.md
rm claude/agents/code-architect.md
rm claude/agents/impact-analyzer.md
rm claude/agents/references/criteria-template.md
rm claude/agents/references/evidence-catalog.md

git add -A
git commit -m "chore(claude): migrate 5 analysis agents + 2 references to neko-neko/belt plugins"
```

残す agent (doc-audit Layer 2): `architecture-analyzer`, `business-rule-analyzer`, `claude-md-analyzer`, `readme-analyzer`, `coherence-analyzer`, `coverage-analyzer`, `deps-analyzer`. これらは `~/.dotfiles/claude/skills/doc-audit` (plugin 化対象外) が使うので残す。

---

## Verification Contract (全体)

- [ ] 各 C1〜C6 commit の直後に `cargo test --workspace --no-fail-fast` が pass することを必ず確認
- [ ] C4 後に `cargo run -p belt -- lint plugins/<plugin>/skills/<skill>/pipeline.yml` が 4 pipeline 全てで pass
- [ ] C5 後に `git grep "examples/skills\|examples/criteria\|examples/references"` の残存を確認 (docs/ 以外)
- [ ] 最終 PR 前に Manual dogfood: `claude --plugin-dir ./plugins/belt-agents --plugin-dir ./plugins/feature-dev --plugin-dir ./plugins/spec-review --plugin-dir ./plugins/code-review --plugin-dir ./plugins/test-scenarios --plugin-dir ./plugins/monkey-test` で起動し、`/feature-dev:feature-dev` を呼び出して `belt-agents:phase-auditor` が resolve するか確認
- [ ] `npx skills add ./` (ローカル manifest) で 7 plugin が discovery されるか確認
