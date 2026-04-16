---
depends-on:
  - examples/skills
  - examples/criteria
  - examples/references
  - .claude/agents
  - README.md
  - crates/belt-core/tests/feature_dev_refresh.rs
  - crates/belt-core/tests/bug_fix_refresh.rs
  - crates/belt-core/tests/review_skills_refresh.rs
  - crates/belt-agent/tests/cli_test.rs
  - CLAUDE.md
---

# examples/skills の Claude Code plugin 化と dotfiles agents 移植 設計

## 背景

`examples/skills/` 配下の 6 skill (feature-dev / bug-fix / code-review / spec-review / monkey-test / test-scenarios) は、pipeline.yml / SKILL.md / criteria / supplement で以下の subagent 群に暗黙依存している：

- `belt/.claude/agents/` (repo-local): `code-reviewer`, `spec-reviewer`
- `~/.dotfiles/claude/agents/` (user global): `phase-auditor`, `feature-implementer`, `code-explorer`, `code-architect`, `impact-analyzer`, ほか doc-audit Layer 2 の 9 agents

このため、他ユーザーが `examples/skills/` を clone しただけではスキルは動作せず、手動で subagent 定義を配置する必要がある。加えて memory 記録 (`dotfiles_skill_md_drift_2026_04_15`) にあるように、belt repo と dotfiles の間で同名 agent の drift が実際に発生した経緯がある。

本設計では、[Claude Code plugin 機構](https://code.claude.com/docs/en/plugins) を使い、skills と agents を自己完結な配布単位として再構築する。配布には [vercel-labs/skills](https://github.com/vercel-labs/skills) の `skills` CLI を使う。

## 目的

- `examples/skills/` の 6 skill + 依存 subagent 群を、plugin として自己完結に配布できる状態にする
- `npx skills add https://github.com/neko-neko/belt` 1 コマンドで全 plugin を install 可能にする
- agent 参照を fully-qualified 形式 (`belt-agents:phase-auditor` 等) に統一し、install 漏れ時のエラーを明示化する
- dotfiles と belt repo の drift を解消し、移植対象 agent の SSOT を belt repo に集約する

## Non-Goals

- belt-core / belt-agent への plugin URI scheme 追加 (`belt-agents://criteria/execute.md` のような cross-plugin path resolution)
- dotfiles の doc-audit Layer 2 agents 9 個 (architecture-analyzer, business-rule-analyzer, claude-md-analyzer, readme-analyzer, coherence-analyzer, coverage-analyzer, deps-analyzer) の plugin 化。これらは dotfiles 側の `/doc-audit` skill が使うため今回移植対象外
- 外部 skill (superpowers / worktrunk / vercel-labs/agent-browser 配下の agent-browser・dogfood) の fork または内製化
- Claude Code marketplace への登録 (`skills` CLI は GitHub URL 直接指定で動作するため必須でない)
- plugin 間 hard dependency 宣言機構 (docs 未記載のため README 案内で代替)
- 既存 memory 記録の path 言及修正 (knowledge capture 時に都度更新)
- Shorthand (`/feature-dev`) vs fully-qualified (`/feature-dev:feature-dev`) の挙動実証テスト。配布 artifact は fully-qualified 固定

## 設計詳細

### 1. Plugin 構成 (7 plugins)

2 層構造。基盤層 1 plugin + skill 層 6 plugins。

| plugin | 責務 | 依存 |
|---|---|---|
| **belt-agents** | 基盤層。5 analysis agents + 4 references | なし |
| **feature-dev** | 9-phase 開発パイプライン | belt-agents, spec-review, code-review, test-scenarios, monkey-test |
| **bug-fix** | 8-phase デバッグパイプライン | belt-agents, spec-review, code-review, monkey-test |
| **code-review** | 7 観点コードレビュー + `code-reviewer` agent | なし |
| **spec-review** | 5 観点スペックレビュー + `spec-reviewer` agent | なし |
| **monkey-test** | agent-browser 駆動の E2E replay | なし |
| **test-scenarios** | Given/When/Then シナリオ生成 | なし |

依存関係は `plugin.json` では宣言しない (Claude Code plugin manifest に hard dependency field が存在しないため)。README の「External skill dependencies」および「Internal dependencies」で案内する。

### 2. ディレクトリ構造

```
belt/
├── .claude-plugin/
│   └── marketplace.json                  # skills CLI 向け plugin 一覧
├── plugins/
│   ├── belt-agents/
│   │   ├── .claude-plugin/plugin.json
│   │   ├── agents/
│   │   │   ├── phase-auditor.md
│   │   │   ├── feature-implementer.md
│   │   │   ├── code-explorer.md
│   │   │   ├── code-architect.md
│   │   │   └── impact-analyzer.md
│   │   └── references/
│   │       ├── _schema.md                # criteria file schema doc
│   │       ├── audit-protocol.md         # phase-auditor dispatch protocol
│   │       ├── evidence-catalog.md       # evidence plan catalog
│   │       └── criteria-template.md      # criteria authoring template
│   ├── feature-dev/
│   │   ├── .claude-plugin/plugin.json
│   │   └── skills/feature-dev/
│   │       ├── SKILL.md
│   │       ├── pipeline.yml
│   │       ├── belt.toml
│   │       ├── criteria/
│   │       │   ├── design.md
│   │       │   ├── test-scenarios.md
│   │       │   ├── spec-review.md
│   │       │   ├── plan.md
│   │       │   ├── execute.md            # ← bug-fix と同一内容 (複製)
│   │       │   ├── code-review.md        # ← bug-fix と同一内容 (複製)
│   │       │   ├── monkey-test.md
│   │       │   ├── dogfood.md
│   │       │   └── integrate.md
│   │       └── references/
│   │           ├── path-convention.md
│   │           ├── brainstorming-supplement.md
│   │           ├── writing-plans-supplement.md
│   │           ├── monkey-test-supplement.md
│   │           ├── dogfood-supplement.md
│   │           └── worktrunk-supplement.md
│   ├── bug-fix/
│   │   ├── .claude-plugin/plugin.json
│   │   └── skills/bug-fix/
│   │       ├── SKILL.md
│   │       ├── pipeline.yml
│   │       ├── belt.toml
│   │       ├── criteria/
│   │       │   ├── rca.md
│   │       │   ├── fix-plan.md
│   │       │   ├── fix-plan-review.md
│   │       │   ├── execute.md            # ← feature-dev と同一内容 (複製)
│   │       │   ├── code-review.md        # ← feature-dev と同一内容 (複製)
│   │       │   ├── monkey-test.md
│   │       │   ├── dogfood.md
│   │       │   └── integrate.md
│   │       └── references/
│   │           ├── path-convention.md
│   │           ├── rca-supplement.md
│   │           ├── fix-plan-supplement.md
│   │           ├── monkey-test-supplement.md
│   │           ├── dogfood-supplement.md
│   │           └── worktrunk-supplement.md
│   ├── code-review/
│   │   ├── .claude-plugin/plugin.json
│   │   ├── agents/code-reviewer.md
│   │   └── skills/code-review/
│   │       ├── SKILL.md
│   │       ├── pipeline.yml
│   │       └── belt.toml
│   ├── spec-review/
│   │   ├── .claude-plugin/plugin.json
│   │   ├── agents/spec-reviewer.md
│   │   └── skills/spec-review/
│   │       ├── SKILL.md
│   │       ├── pipeline.yml
│   │       └── belt.toml
│   ├── monkey-test/
│   │   ├── .claude-plugin/plugin.json
│   │   └── skills/monkey-test/SKILL.md
│   └── test-scenarios/
│       ├── .claude-plugin/plugin.json
│       └── skills/test-scenarios/SKILL.md
├── crates/                                # 既存 (path 更新のみ)
├── docs/                                  # 既存
└── README.md                              # Plugins セクション追加
```

削除対象 (atomic cutover):

- `examples/` ディレクトリ全体 (criteria/, references/, skills/)
- `belt/.claude/agents/code-reviewer.md`, `belt/.claude/agents/spec-reviewer.md`
- `~/.dotfiles/claude/agents/{phase-auditor,feature-implementer,code-explorer,code-architect,impact-analyzer}.md`
- `~/.dotfiles/claude/agents/references/{criteria-template,evidence-catalog}.md`

### 3. Plugin manifest (`plugin.json`)

各 plugin の `.claude-plugin/plugin.json`:

```json
{
  "name": "belt-agents",
  "description": "Base analysis agents for belt-based quality-gated development pipelines",
  "version": "0.1.0",
  "author": "neko-neko"
}
```

- `name`: plugin 識別子。kebab-case。Claude Code namespace 化のキー
- `version`: 初期リリース全て `0.1.0`。以降は plugin 単位で semver 進化
- 他 plugin の name / description は「responsibility」を 1 文で記述

### 4. Marketplace manifest (`.claude-plugin/marketplace.json`)

belt repo root の `.claude-plugin/marketplace.json`:

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

- `belt-agents` は skill を持たない (agent-only plugin) ので `skills` field を省略
- `source` は `pluginRoot` からの相対パス

### 5. Agent 参照の命名規約 (fully-qualified)

全ての agent 参照を `plugin:agent` 形式に統一する。

| 参照元 | 参照先 | 書き方 |
|---|---|---|
| `plugins/code-review/skills/code-review/pipeline.yml` | 同 plugin の code-reviewer | `invoke.agents: [code-review:code-reviewer]` |
| `plugins/spec-review/skills/spec-review/pipeline.yml` | 同 plugin の spec-reviewer | `invoke.agents: [spec-review:spec-reviewer]` |
| `plugins/feature-dev/skills/feature-dev/criteria/*.md` 本文 | belt-agents の phase-auditor | 文中で `belt-agents:phase-auditor` と明記 |
| `plugins/feature-dev/skills/feature-dev/references/brainstorming-supplement.md` | belt-agents の分析 agents | `belt-agents:code-explorer`, `belt-agents:code-architect`, `belt-agents:impact-analyzer` |
| `plugins/bug-fix/skills/bug-fix/criteria/*.md` | belt-agents の phase-auditor | `belt-agents:phase-auditor` |
| `plugins/bug-fix/skills/bug-fix/references/rca-supplement.md` | belt-agents の分析 agents | 同上 |
| 各 SKILL.md の Red Flags / References 節 | cross-plugin agent | 常に `plugin:agent` 形式 |

Skill tool 呼び出し (SKILL.md 内の `/skill-name` 呼び出し) は shorthand を許容せず、常に fully-qualified (`/code-review:code-review` など) で記述する。ユーザー手動呼び出しでは shorthand が通る可能性があるが、配布 artifact 内では install 状態に関わらず動作する fully-qualified に統一する。

### 6. 共有 criteria の drift 防止

`feature-dev/.../criteria/execute.md` と `bug-fix/.../criteria/execute.md` は同一内容で物理複製される (belt-agent の filesystem path resolution 制約により cross-plugin 参照できない)。drift 検出のため `belt-core/tests/` に parity test を追加：

```rust
// crates/belt-core/tests/shared_criteria_parity.rs
//! Integration test to detect drift between feature-dev and bug-fix shared criteria files.

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

CI (`cargo test --workspace`) で drift が即検出される。

### 7. README.md 追加セクション

belt README 末尾 (License の前) に以下を追加する：

```markdown
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

\`\`\`bash
# 1. superpowers (/brainstorming, /writing-plans, /subagent-driven-development, /systematic-debugging)
npx skills add obra/superpowers -g -y

# 2. worktrunk (/worktrunk)
npx skills add max-sixty/worktrunk -g -y

# 3. agent-browser plugin (agent-browser CLI + /dogfood skill)
npx skills add vercel-labs/agent-browser --skill agent-browser --skill dogfood -g -y

# 4. belt plugins (all 7)
npx skills add neko-neko/belt -g -y
\`\`\`

Selective install (only some belt plugins):

\`\`\`bash
# Example: code-review only (no external deps)
npx skills add neko-neko/belt --skill code-review -g -y
\`\`\`

Plugin discovery uses `.claude-plugin/marketplace.json` at belt repo root.

### Internal dependencies (plugin-to-plugin)

- `feature-dev` invokes `spec-review`, `code-review`, `test-scenarios`, `monkey-test`
- `bug-fix` invokes `spec-review`, `code-review`, `monkey-test`
- `feature-dev`, `bug-fix` require `belt-agents` (analysis agents referenced by criteria and supplements)
- `code-review`, `spec-review`, `monkey-test`, `test-scenarios`, `belt-agents` are standalone

### Usage

After install:

\`\`\`
/feature-dev:feature-dev         # start a new feature
/bug-fix:bug-fix                 # start a bug investigation
/code-review:code-review         # standalone code review
/spec-review:spec-review         # standalone spec review
\`\`\`

See each plugin's `SKILL.md` for phase details and arg reference.
```

### 8. Migration 計画 (hard cutover)

論理単位ごとに commit を分割し、各 commit でレポジトリが整合している状態を保つ。

| # | Commit 内容 | cargo test 状態 |
|---|---|---|
| C1 | Plugin 骨格作成: `plugins/<plugin>/` 7 ディレクトリ + 各 `.claude-plugin/plugin.json` + `.claude-plugin/marketplace.json` + README に Plugins セクション追加 | pass (既存 test は `examples/skills/` を継続参照、新 `plugins/` は未使用) |
| C2 | `belt-agents` plugin への agent / references 配置: `~/.dotfiles/claude/agents/{5 agents}.md` + `~/.dotfiles/claude/agents/references/{2 files}.md` を `plugins/belt-agents/agents/` と `plugins/belt-agents/references/` に **cp** で複製 (dotfiles 削除は C5)。`examples/criteria/_schema.md` と `examples/references/audit-protocol.md` を `plugins/belt-agents/references/` に **cp** で複製 (examples/ 削除は C5) | pass (既存 test 継続) |
| C3 | skill 本体と shared criteria を plugin 配下に **cp** で複製配置: `examples/skills/<skill>/` → `plugins/<skill>/skills/<skill>/`。`examples/criteria/{execute,code-review}.md` を feature-dev と bug-fix の criteria/ に複製。全 pipeline.yml / SKILL.md / criteria / supplement 内の agent 参照を fully-qualified (`belt-agents:phase-auditor` 等) に書き換え。`invoke.skill:` も `/plugin:skill` 形式へ。`belt/.claude/agents/{code-reviewer,spec-reviewer}.md` を `plugins/code-review/agents/` と `plugins/spec-review/agents/` に複製 (旧削除は C5) | pass (既存 test は `examples/` を継続参照) |
| C4 | belt-core / belt-agent test の path を `plugins/` 配下へ切替 + drift parity test 追加: `feature_dev_refresh.rs`, `bug_fix_refresh.rs`, `review_skills_refresh.rs`, `belt-agent/tests/cli_test.rs` の `examples/skills/` path を `plugins/<name>/skills/<name>/` に更新。`crates/belt-core/tests/shared_criteria_parity.rs` を新規追加 | pass (plugins/ 配下のファイルは C2+C3 で配置済み、test は新 path を参照) |
| C5 | 旧ファイル削除: `examples/` ディレクトリ全体 + `belt/.claude/agents/{code-reviewer,spec-reviewer}.md` + `~/.dotfiles/claude/agents/{5 agents}.md` + `~/.dotfiles/claude/agents/references/{2 files}.md` | pass (test は既に `plugins/` 参照、examples/ は test 対象外) |
| C6 | CLAUDE.md 更新: `examples/skills/` への言及を `plugins/` 配下のパスに書き換え | pass |

**Commit 整合性の鍵**: C2 / C3 で旧ファイルを **cp (複製) する** ことで、C5 の削除までは旧と新の両方が共存する。C4 で test path を切替えた瞬間に `plugins/` 参照が live になり、C5 で旧を削除しても test は継続 pass する。各 commit 後に `cargo test --workspace` が pass することを CI で確認する。

### 9. 検証戦略

1. `cargo test --workspace` が全 pass (既存 test の path 更新 + 新規 parity test)
2. 新規 `shared_criteria_parity.rs` が `execute.md` / `code-review.md` の byte-identical を検証
3. `belt lint plugins/<plugin>/skills/<skill>/pipeline.yml` が 6 plugin (belt-agents は pipeline なし) で lint 合格
4. 手動確認 (dogfood): `claude --plugin-dir ./plugins/belt-agents --plugin-dir ./plugins/feature-dev --plugin-dir ./plugins/spec-review --plugin-dir ./plugins/code-review --plugin-dir ./plugins/test-scenarios --plugin-dir ./plugins/monkey-test` でローカル install し、`/feature-dev:feature-dev` 起動時に `belt-agents:phase-auditor` などの fully-qualified 参照が resolve できるか確認
5. `npx skills add ./` (ローカル manifest) で marketplace.json 経由の install が成立するか確認

### 10. リスクと対応

| リスク | 影響 | 対応 |
|---|---|---|
| Claude Code の plugin agent shorthand 解決が想定と異なる | `belt-agents:phase-auditor` 呼び出しが失敗 | ローカル `claude --plugin-dir` で事前確認。失敗時は命名規約 A を見直し |
| dotfiles 側 agent 削除直後に global session で旧 agent を参照するコードがある | agent not found エラー | Migration commit 順序で 「belt-agents plugin 作成 → global install 案内 → dotfiles 削除」を直列化 |
| `skills` CLI が `.claude-plugin/marketplace.json` を discovery しない | `npx skills add` で plugin が見つからない | vercel-labs/skills README のフォーマット仕様に準拠。失敗時は各 `plugin.json` を直接 discovery させるべく `pluginRoot` を調整 |
| shared criteria file の drift | feature-dev と bug-fix で挙動齟齬 | `shared_criteria_parity.rs` integration test で CI 検出 |
| 外部 plugin (superpowers / worktrunk) の breaking change | `/brainstorming` 等が失敗 | README で version pin を案内。現状は latest 追従 |

## 代替案と採用理由

Plugin 分割方式の代替：

- **B. skill 単位で完全分割** (14 agent を各 plugin に重複コピー) — 重複メンテ負荷が高い
- **A. 単一巨大 plugin** (1 つに全て詰める) — 粒度が粗く、選択的 install が不可

採用 (C: 基盤 + skill 層) の理由：
1. 14 agent のうち phase-auditor / code-explorer 等は feature-dev と bug-fix の両方で使われ、B では重複必須
2. namespace `belt-agents:phase-auditor` が自己記述的で install 依存を明示できる
3. code-review / spec-review だけ使いたいユーザーは `belt-agents` を入れず済む

belt-agents スコープの代替：

- **B. 全 14 agents 移植** — doc-audit skill 自体が belt 外部なので、Layer 2 agents を belt に持ち込むと責務が広がる
- **C. 2 層分離 (`belt-agents` + `belt-doc-audit-agents`)** — doc-audit skill 自体が plugin 化されていない以上、9 agent を移植しても他利用者の導線がない。YAGNI

採用 (A: 実使用の 5 agents のみ) の理由：
1. このレポジトリの関心は belt とその example skills
2. doc-audit の plugin 化時に `belt-doc-audit-agents` として切り出せば良い

## Out of Scope (再掲)

- belt-core / belt-agent の plugin URI scheme 対応
- doc-audit Layer 2 agents 9 個の plugin 化
- marketplace への登録
- plugin 間 hard dependency 宣言機構
- 既存 memory 記録の path 言及修正
- 外部 skill (superpowers / worktrunk / vercel-labs/agent-browser) の fork または内製化
- Shorthand vs fully-qualified 挙動実証テスト

## 影響範囲

### 変更ファイル

- 新規: `plugins/` ディレクトリ全体 (7 plugin × 複数ファイル)、`.claude-plugin/marketplace.json`
- 変更: `README.md`, `CLAUDE.md`, `crates/belt-core/tests/feature_dev_refresh.rs`, `crates/belt-core/tests/bug_fix_refresh.rs`, `crates/belt-core/tests/review_skills_refresh.rs`, `crates/belt-agent/tests/cli_test.rs`
- 追加: `crates/belt-core/tests/shared_criteria_parity.rs`
- 削除: `examples/` (全削除)、`.claude/agents/code-reviewer.md`, `.claude/agents/spec-reviewer.md`

### 変更ファイル (外部 / dotfiles)

- 削除: `~/.dotfiles/claude/agents/{phase-auditor,feature-implementer,code-explorer,code-architect,impact-analyzer}.md`
- 削除: `~/.dotfiles/claude/agents/references/{criteria-template,evidence-catalog}.md`

### 影響を受ける利用者

- belt repo 開発者: 移行後、pipeline 書き換えと test 更新の都合で必ず `cargo test` が必要
- Claude Code 利用者全体: dotfiles 側の 5 agent 削除後、belt-agents plugin を install しない限り `phase-auditor` などが global から消える。README に install 手順を明記することで誘導

## 成功基準

1. `cargo test --workspace` 全 pass
2. `belt lint plugins/<plugin>/skills/<skill>/pipeline.yml` 全 pass
3. `npx skills add <belt repo URL> --agent claude-code` で 7 plugin が discovery される
4. ローカル `--plugin-dir` install で `/feature-dev:feature-dev` を起動し、Phase 1 で `belt-agents:phase-auditor` が resolve して pipeline が進行する (E2E dogfood)
5. `shared_criteria_parity.rs` が CI で drift を検出する
