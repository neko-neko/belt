---
title: cargo-dist Release Automation and belt-agent Skill Consolidation
date: 2026-04-16
status: approved
scope:
  - dist-workspace.toml
  - release.toml
  - CHANGELOG.md
  - Cargo.toml
  - crates/belt-core/Cargo.toml
  - crates/belt/Cargo.toml
  - crates/belt-agent/Cargo.toml
  - .github/workflows/release.yml
  - README.md
  - AGENTS.md
  - plugins/belt-agents/skills/belt-agent/SKILL.md
  - plugins/belt-agents/.claude-plugin/plugin.json
  - .claude-plugin/marketplace.json
related:
  - MEMORY: project_belt_architecture.md
  - MEMORY: project_claude_md_symlink.md
---

# cargo-dist Release Automation and belt-agent Skill Consolidation

## 1. Overview

本 spec は 2 つの独立した改善を同日 spec として束ねる。

- **Part A** — `cargo-dist 0.31.0` + `cargo-release 1.1.2` による GitHub Release 自動化。tag push を trigger に 4 target (macOS x86_64/aarch64、Linux x86_64/aarch64) の build / shell installer / tar アーカイブ / supply chain attestation を生成し、README に `curl` インストール手順を追記する。Windows は対象外。
- **Part B** — `skills/belt-agent/SKILL.md` を `plugins/belt-agents/skills/belt-agent/SKILL.md` に移動し、project 直下の `skills/` ディレクトリを削除する。belt-agents plugin の自然な構成に揃える。

Part A と Part B は独立に merge / deploy 可能だが、`Implementation order` で Part B を先行させる方針 (§5)。

## 2. Part A: cargo-dist Release Automation

### 2.1 Architecture

| Tool | Role | Trigger |
|------|------|---------|
| `cargo-release` | ローカルで version bump + CHANGELOG placeholder 置換 + commit + tag + push | 開発者手動 |
| `cargo-dist` | CI で multi-target build + release asset 生成 + GitHub Release 作成 | `v*` tag push |
| `CHANGELOG.md` | 人間向け履歴 + cargo-dist が release notes として抽出する source | `cargo-release` が自動更新 |
| GitHub Release | `curl` 配布のホスティング。`/releases/latest/download/` を利用 | 自動 |

### 2.2 Files created / modified (Part A 範囲)

Part B の file 変更は §3.4 に別表で記載。以下は Part A 固有:

| File | 種別 | 説明 |
|------|------|------|
| `dist-workspace.toml` | 新規 | cargo-dist 設定 |
| `release.toml` | 新規 | cargo-release 設定 (workspace root、section header なし) |
| `.github/workflows/release.yml` | 新規 (`dist init --yes` が自動生成) | 人間は直接編集しない |
| `CHANGELOG.md` | 新規 | keepachangelog 形式、`<!-- next-header -->` / `<!-- next-url -->` マーカー |
| `Cargo.toml` | 編集 | `[workspace.package]` に `description` / `readme` 追加 |
| `crates/belt-core/Cargo.toml` | 編集 | `[package] publish = false` + `[package.metadata.release] release = false` + crate 独自 `description` + `readme.workspace = true` |
| `crates/belt/Cargo.toml` | 編集 | `[package] publish = false` + crate 独自 `description` + `readme.workspace = true` |
| `crates/belt-agent/Cargo.toml` | 編集 | `[package] publish = false` + crate 独自 `description` + `readme.workspace = true` |
| `README.md` | 編集 | **`## Build` を `## Install` に rename & restructure** (§2.9)。Part B の plugins table 更新は別節 (`## Claude Code Plugins`) |
| `AGENTS.md` (= `CLAUDE.md` symlink) | 編集 | "Release Process" 新セクション追加 |

`AGENTS.md` が symlink 経路の正とする (`git add CLAUDE.md` は symlink のみ stage するため `git add AGENTS.md` を使用、MEMORY `project_claude_md_symlink.md` 参照)。

### 2.3 `dist-workspace.toml`

```toml
[dist]
cargo-dist-version = "0.31.0"  # 0.x なので完全ピン、bump は init 再実行セット

# CI / hosting
ci = ["github"]
hosting = ["github"]

# Install 方法 (Windows 非対応なので powershell 不要)
installers = ["shell"]

# Target platforms
targets = [
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
]

# V2 Balanced shape
github-attestations = true
cache-builds = true
fail-fast = false
pr-run-mode = "plan"
checksum = "sha256"
```

`aarch64-unknown-linux-gnu` は cargo-dist default runner (`ubuntu-22.04` 系) で `cross` を用いて cross-compile。専用 ARM runner (buildjet 等) は不採用。

### 2.4 `release.toml` (workspace root、section header なし)

```toml
# Workspace-wide policy
shared-version = true
consolidate-commits = true
allow-branch = ["main"]                         # main 以外からの release を禁止

# Tag
tag-name = "v{{version}}"
tag-prefix = ""                                 # workspace で統一
tag-message = "chore: Release v{{version}}"

# Commit
pre-release-commit-message = "chore: Release v{{version}}"

# crates.io publish は今回のスコープ外
publish = false

# verify (cargo build) は保持 (pre-release 時に破綻検出)

# CHANGELOG.md 置換
pre-release-replacements = [
    { file = "CHANGELOG.md", search = "Unreleased",  replace = "{{version}}" },
    { file = "CHANGELOG.md", search = "ReleaseDate", replace = "{{date}}" },
    { file = "CHANGELOG.md", search = "<!-- next-header -->",
      replace = "<!-- next-header -->\n\n## [Unreleased] - ReleaseDate", exactly = 1 },
    { file = "CHANGELOG.md", search = "<!-- next-url -->",
      replace = "<!-- next-url -->\n[Unreleased]: https://github.com/neko-neko/belt/compare/{{tag_name}}...HEAD",
      exactly = 1 },
    { file = "CHANGELOG.md", search = "\\.\\.\\.HEAD", replace = "...{{tag_name}}", exactly = 1 },
]
```

### 2.5 `Cargo.toml` additions

```toml
[workspace.package]
# ...既存 version / edition / rust-version / authors / license / repository
description = "Workspace root. See crate-level Cargo.toml for per-crate description."
readme = "README.md"
```

workspace.package.description は GitHub Release notes や install landing には直接露出しないが、`cargo metadata` や `cargo publish` (将来) で参照される。**crate ごとに独自 description を書く**方針を採り、workspace レベルは metadata placeholder のみとする:

| Crate | description 案 |
|---|---|
| `belt-core` | `Core library for belt — pipeline model, parser, expander, engine, gates, linter` |
| `belt` | `belt lint — static validator for belt YAML pipelines` |
| `belt-agent` | `belt-agent — runtime CLI for driving belt YAML pipelines from LLMs or scripts` |

各 crate の `Cargo.toml`:

```toml
[package]
description = "<crate-specific description from table above>"
readme.workspace = true
# description.workspace = true は使わない (crate ごと独自文言のため)
```

README 冒頭の現行文 ("A workflow engine for LLM-driven Agent Skills. Declare deterministic state machines in YAML, ...") と crate description が表現軸を共有することを重視し、cargo-dist が生成する installer ページ (`/releases/latest/download/belt-installer.sh` を案内する landing) で読者が同じ phrasing に触れられる状態にする。

### 2.6 `CHANGELOG.md` initial form

README.md 現行 (2026-04-16 版) に記載済みの機能 (Continuity / `belt://` URI 3 selector / `--inherits-from` 等) をすべて初回エントリで触れる。

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - ReleaseDate

### Added
- Initial public release of belt workflow engine for LLM-driven Agent Skills.
- `belt lint` CLI: static validator for belt YAML pipelines. Detects duplicate phase IDs, unknown `regate` targets, undefined args in `when:`, missing descriptions, unresolvable `uses:` / `invoke.pipeline:` references, artifact flow violations, and sub-pipeline expansion failures.
- `belt-agent` CLI (`init` / `next` / `verify` / `regate` / `step` / `status`): runtime for agent-driven pipeline execution. All output is JSON.
- Sub-pipeline `uses:` composition with flat namespace expansion (`{parent_id}/{sub_phase_id}`).
- GateCheck (4 variants): `cmd`, `file_exists`, `git_clean`, `has_output`.
- Invoker + Artifact first-class model (BELT-32) with `invoke.skill` / `invoke.pipeline` dispatch.
- Narrative artifacts with `belt://` URI scheme (3 selectors: `belt://run/<id>/<path>`, `belt://latest/<pipeline>/<path>`, `belt://workspace/<branch>/latest/<pipeline>/<path>`).
- Cross-run inheritance via `belt-agent init --inherits-from <run_id>` — a fresh agent picks up a prior run's gated outputs without inheriting its trial-and-error trace.
- `belt.toml` config file for path resolution (BELT-22).
- Enriched `status` output (BELT-29): query-time assembly from run state, pipeline YAML, and output directories.
- 7 Claude Code plugins under `plugins/` (belt-agents, feature-dev, bug-fix, code-review, spec-review, monkey-test, test-scenarios) as working examples of belt-driven quality-gated AI development.

<!-- next-url -->
[Unreleased]: https://github.com/neko-neko/belt/compare/v0.1.0...HEAD
```

初回 v0.1.0 は手動で `[Unreleased]` を `[0.1.0] - 2026-04-XX` に書き換える。2 回目以降は cargo-release が自動置換。

### 2.7 Release commands

```bash
# Bump levels
cargo release patch -x          # 0.1.0 → 0.1.1
cargo release minor -x          # 0.1.0 → 0.2.0
cargo release major -x          # 0.1.0 → 1.0.0
cargo release 0.2.3 -x          # 明示 version
cargo release rc -x             # pre-release (0.2.0-rc.1)
cargo release release -x        # pre-release extension 除去

# CHANGELOG 執筆補助
cargo release changes           # 前 tag 以降の commit を印字
```

`-x` は `--execute` の短縮形。dry-run が default (`cargo release minor` だけなら計画表示のみ)。

### 2.8 Initial v0.1.0 release

`workspace.package.version` が既に `0.1.0` の状態では cargo-release が「次 version への bump」を前提とするため、初回は手動 tag で発火する:

```bash
cargo install cargo-dist@0.31.0 --locked
cargo install cargo-release@1.1.2 --locked

dist init --yes                              # .github/workflows/release.yml 生成
# CHANGELOG.md の [Unreleased] を [0.1.0] - YYYY-MM-DD に手で書き換える

git add dist-workspace.toml release.toml CHANGELOG.md Cargo.toml crates/*/Cargo.toml \
        .github/workflows/release.yml AGENTS.md README.md
git commit -m "chore: prepare v0.1.0 release"

git tag v0.1.0
git push origin main --tags
# → GitHub Actions が trigger、Release v0.1.0 を作成
```

### 2.9 `README.md` install section

**方針**: 現行 README (2026-04-16 版) の `## Build` (L173-184) を **`## Install` に rename & restructure** する。既存の `cargo build` コマンドは `### From source` サブセクションに吸収。位置は現 `## Build` と同じ (`## Key Concepts` と `## Claude Code Plugins` の間)。

新規 `## Install` セクションの全文:

```markdown
## Install

### Shell installer (recommended)

Installs `belt` and `belt-agent` to `$HOME/.cargo/bin` (or configurable),
auto-detects platform.

\`\`\`bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/neko-neko/belt/releases/latest/download/belt-installer.sh | sh
\`\`\`

### Manual install (CI / Docker)

Pick a tarball matching your platform from the
[releases page](https://github.com/neko-neko/belt/releases):

\`\`\`bash
# Example: Linux x86_64
curl -L https://github.com/neko-neko/belt/releases/latest/download/belt-x86_64-unknown-linux-gnu.tar.xz \
  | tar -xJ -C /usr/local/bin belt belt-agent
\`\`\`

Replace the triple to match your platform:

| OS | Arch | Triple |
|---|---|---|
| macOS | Intel | `x86_64-apple-darwin` |
| macOS | Apple Silicon | `aarch64-apple-darwin` |
| Linux | x86_64 | `x86_64-unknown-linux-gnu` |
| Linux | aarch64 | `aarch64-unknown-linux-gnu` |

### Verify (optional)

\`\`\`bash
gh release download v0.1.0 --repo neko-neko/belt --pattern '*.tar.xz'
gh attestation verify belt-x86_64-unknown-linux-gnu.tar.xz --repo neko-neko/belt
\`\`\`

### From source

\`\`\`bash
git clone https://github.com/neko-neko/belt.git && cd belt
cargo build --release --workspace
\`\`\`

Build only what you need:

\`\`\`bash
cargo build -p belt          # lint tool
cargo build -p belt-agent    # agent runtime
\`\`\`
```

既存 `## Build` の `cargo build -p belt / belt-agent` の抜粋も `### From source` 配下に移す (現行 README L180-184 の情報損失を避ける)。

Archive 拡張子 (`.tar.xz` vs `.tar.gz`) は cargo-dist 0.31.0 の default を初回 `dist plan` で確認し、実物に合わせて `curl ... | tar -xJ ...` (tar.xz の場合) または `curl ... | tar -xz ...` (tar.gz の場合) に最終調整する。

### 2.10 PR check behavior

`pr-run-mode = "plan"` により PR では `dist plan` job のみ走る。build は起動せず ~30 秒で完了。設定変更 PR で早期に問題検出可能。毎 PR で full build を走らせる `upload` mode は採用しない (CI 時間コスト > 価値)。

### 2.11 Supply chain verification

`github-attestations = true` により release asset は sigstore 準拠で署名される。permissions (`attestations: write`, `id-token: write`) は `dist init` が生成する `release.yml` に自動埋め込みされ、人間は touch しない。

利用者検証例:

```bash
gh attestation verify <asset>.tar.xz --repo neko-neko/belt
```

README の Install section に optional step として記載 (§2.9)。

### 2.12 aarch64-linux cross-compile

cargo-dist default Linux runner (GitHub-hosted Ubuntu 系 LTS、具体 version は 0.31.0 の選択に従う) 上で `cross` crate を自動利用してクロスコンパイル。buildjet 等の ARM 専用 runner は**不採用** (月額費用、現段階で release 頻度が低いため不要)。`cross` default image は古めの glibc を採用するため、後方互換性の観点でも無難。将来 build 時間が問題になれば `[dist.github-custom-runners]` で有料 runner に切り替え可能。

具体 runner OS version (`ubuntu-22.04` or `ubuntu-24.04` 等) は cargo-dist 0.31.0 generator の出力に依存。初回 `dist init --yes` 実行後に `.github/workflows/release.yml` の `runs-on:` を確認し、予期せぬ LTS 切替があれば `[dist.github-custom-runners].global` で pin する。

### 2.13 Required permissions / secrets

- `GITHUB_TOKEN` — 自動提供。release 作成 + attestation に使用
- 追加 secrets は不要 (macOS notarization / crates.io publish を後追で導入する場合のみ必要)

## 3. Part B: belt-agent skill consolidation

### 3.1 Motivation

現状 `skills/` 直下には `belt-agent` が 1 つだけ存在し、`plugins/belt-agents` と役割が分離している。`feature-dev` / `bug-fix` 等の plugin が "Belt Protocol" を依存として扱う際、共通依存の belt-agents plugin に集約される方が整合的。Anthropic Claude Code plugin の `plugins/<plugin>/skills/<slug>/SKILL.md` 慣行にも揃う。

### 3.2 Move

| From | To |
|------|------|
| `skills/belt-agent/SKILL.md` | `plugins/belt-agents/skills/belt-agent/SKILL.md` |
| `skills/` (project 直下、belt-agent 削除後空になる) | **ディレクトリ削除** |

skill slug (`belt-agent` 単数) は保持、plugin 名 (`belt-agents` 複数) も既存のまま。両者は独立した slug として衝突しない。

### 3.3 Path references in `SKILL.md`

SKILL.md 本文内の既存参照:

```
See `plugins/belt-agents/references/audit-protocol.md` for the expected
criteria file format.
```

この path は **project-root 起点の絶対的表記** で、移動後も LLM による `Read` 解釈で正しく解決される (project root からの相対)。**本文変更は不要**。

### 3.4 Integrity updates (Part B 範囲)

| File | 変更 |
|------|------|
| `README.md` | "Plugins in this repo" table の belt-agents 行を `Base layer: 5 analysis agents (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer) + Belt Protocol skill + references` に更新 |
| `.claude-plugin/marketplace.json` | `plugins[].description` (belt-agents エントリ) を `Base analysis agents + Belt Protocol skill for belt-based quality-gated development pipelines (phase-auditor, code-explorer, code-architect, impact-analyzer, feature-implementer)` に更新 |
| `plugins/belt-agents/.claude-plugin/plugin.json` | `description` フィールドを上記 marketplace.json と同一文面に更新 |
| `AGENTS.md` (= `CLAUDE.md` symlink) | Part B としては**変更不要** (skill 配置を明示した節は存在しない)。Part A での "Release Process" 節追加と commit は分離する (PR 分割の観点) |

README.md は Part A でも "Install" section を追加するため両 Part で変更が発生するが、**節が異なる**ため行レベルの conflict は起きない。Part B を先に merge、Part A を後に merge する実施順 (§5) で問題は発生しない。

### 3.5 Open points to verify during implementation

1. **Plugin loader が `plugins/belt-agents/skills/belt-agent/SKILL.md` を自動認識するか** — `feature-dev` / `bug-fix` で既に同構造が稼働中なので OK のはずだが、実装時に `Skill` tool invoke テストで確認
2. **`user-invocable: false` の skill が plugin loader に登録されるか** — 既存の belt-agent が `user-invocable: false` で動作中なので問題ないはずだが、plugin 配下に移動後にも invoke 経路 (`/belt-agent` ではなく internal reference から) が保たれるか確認
3. **他 plugin (`feature-dev` / `bug-fix` 等) 内部で skill を参照する path** — skill は name ベースで invoke されるため path 依存しないはずだが、`plugins/<other>/skills/<slug>/SKILL.md` / `plugin.json` / `references/*.md` 内に `skills/belt-agent/` ハードコードがないか grep で確認

## 4. Non-goals

### Part A

- Homebrew tap / npm wrapper / msi installer (V3 で別 spec として追加検討)
- crates.io publish (`publish = false` 明示)
- macOS code signing / notarization (Apple Developer 登録必要、後回し)
- Windows release
- Docker image の prebuilt 配布
- stable / nightly 等の複数 release channel

### Part B

- plugin 名 (`belt-agents` 複数) / skill slug (`belt-agent` 単数) のリネーム
- SKILL.md 本文の内容変更 (path reference は絶対的表記を維持)
- 他 plugin の skill 再配置

## 5. Implementation order

Part B → Part A の順で実施する。

| Step | Part | 説明 |
|------|------|------|
| 1 | B | skill file 移動 + `skills/` 削除 + integrity updates + PR merge |
| 2 | A | 設定ファイル追加 + CHANGELOG 初期化 + `dist init --yes` 実行 + Cargo.toml edit + AGENTS.md 追記 + PR merge |
| 3 | A | 初回 v0.1.0 tag 作成 + push (GitHub Actions が release 生成) |
| 4 | A | README の archive 拡張子を `dist plan` 結果に合わせて fix (Step 3 の前に spike でも OK) |

Part B が Part A より先の理由:
1. Part B は shape 変更で build / release artifact (binary tarball) に影響しない。rollback も `git mv` の reverse のみで済む
2. **GitHub が tag 時点で自動生成するソース tarball** (`Source code (tar.gz)` / `Source code (zip)`) には repo 全スナップショットが含まれる。Part A を先に走らせると v0.1.0 ソース tarball に `skills/` 旧ディレクトリが記録され、直後の Part B 適用後に main と v0.1.0 ソース tarball で skill 配置が乖離する。cargo-dist が生成する binary tarball はこの影響を受けないが、ソース tarball 経由で入手する利用者には混乱要因
3. Part B → Part A の順なら v0.1.0 が clean shape で最初から配布される。以降の release も一貫した shape を保つ

両者は独立 PR として分けて作ることを推奨 (review focus が異なる)。

## 6. Version matrix

| Tool | Pinned version | Release date | Upgrade policy |
|------|----------------|--------------|----------------|
| `cargo-dist` | `=0.31.0` | 2026-02-23 | 0.x なので完全ピン。bump は `dist-workspace.toml` 更新 + `dist init --yes` 再実行 + 生成 workflow を再 commit の三点セット必須 |
| `cargo-release` | `1.1.2` | 2026-03-24 | 1.x SemVer 安定版。major 互換内で追従可、patch bump 時は release notes を確認 |

両者とも `--locked` で install して依存解決を再現可能にする。

## 7. `AGENTS.md` (`CLAUDE.md`) additions

`## Technology Stack` / `## 依存管理ポリシー` の流れを継ぐ形で、`## Non-Goals` の直前に新セクションを追加:

```markdown
## Release Process

belt は `cargo-dist` + `cargo-release` による 2 段構成でリリースする。version bump と tag 作成は開発者ローカル、multi-target build と GitHub Release 作成は CI が担う。

### Tooling

| Tool | Pinned version | Role | Upgrade policy |
|------|----------------|------|----------------|
| `cargo-dist` | `=0.31.0` | CI で multi-target build、GitHub Release 作成、shell installer / tar アーカイブ / attestation 生成 | 0.x 完全ピン。bump 時は `dist-workspace.toml` 更新 + `dist init --yes` 再実行 + 生成された `.github/workflows/release.yml` の再 commit の三点セットを必須 |
| `cargo-release` | `1.1.2` | ローカルで version bump + `CHANGELOG.md` placeholder 置換 + commit + tag + push | 1.x SemVer 安定版。major 互換内で追従可 |

### 配置ファイル

| File | 所有 | 役割 |
|------|------|------|
| `dist-workspace.toml` | 人間 | cargo-dist 設定 (targets / installers / attestations / cache-builds) |
| `release.toml` | 人間 | cargo-release 設定 (`shared-version = true` / `tag-name = "v{{version}}"` / `pre-release-replacements` / `allow-branch = ["main"]`) |
| `.github/workflows/release.yml` | cargo-dist が自動生成 | 人間は直接編集しない。再生成は `dist init --yes` |
| `CHANGELOG.md` | 人間 | keepachangelog 形式、`<!-- next-header -->` / `<!-- next-url -->` マーカー必須 |

### Target platforms

- `x86_64-apple-darwin` / `aarch64-apple-darwin` / `x86_64-unknown-linux-gnu` / `aarch64-unknown-linux-gnu`
- Windows は対象外 (MVP scope、将来必要なら `dist-workspace.toml` に追加)
- `aarch64-unknown-linux-gnu` は default runner 上で `cross` を用いて cross-compile

### Release commands

\`\`\`bash
cargo release patch -x          # 0.1.0 → 0.1.1
cargo release minor -x          # 0.1.0 → 0.2.0
cargo release major -x          # 0.1.0 → 1.0.0
cargo release 0.2.3 -x          # 明示 version
cargo release rc -x             # pre-release (0.2.0-rc.1)
cargo release release -x        # pre-release extension 除去
cargo release changes           # 前 tag 以降の commit 一覧 (CHANGELOG 執筆補助)
\`\`\`

Bump は `allow-branch = ["main"]` により main からのみ実行可能。CI workflow (`release.yml`) は `v*` tag push で自動発火する。

### 初回 release の特殊ケース

`workspace.package.version` が既に `0.1.0` の状態では cargo-release は「次 version への bump」を前提とするため、初回 v0.1.0 は手動 tag 作成で発火する (`git tag v0.1.0 && git push --tags`)。初回前に `CHANGELOG.md` の `[Unreleased]` 節を `[0.1.0] - YYYY-MM-DD` に手で書き換える。2 回目以降は `cargo release minor -x` 等で CHANGELOG placeholder 置換を含めて完全自動化される。

### crates.io

**publish 対象外** (MVP)。`release.toml` で `publish = false`、各 crate の `Cargo.toml` にも `publish = false` を冗長記述。将来 crates.io 対応時は両方を外す + `publish-jobs = ["crates-io"]` を `dist-workspace.toml` に追加。

### 供給チェーン検証

`github-attestations = true` により release asset は sigstore 署名される。利用者検証:

\`\`\`bash
gh attestation verify <asset>.tar.xz --repo neko-neko/belt
\`\`\`

README の Install section に optional step として記載。

### 依存管理ポリシーとの整合

Release tooling は既存の「依存管理ポリシー > バージョン指定ポリシー」に従う。`cargo-dist` は 0.x ゆえ完全ピン、`cargo-release` は 1.x ゆえ caret 許容の対象だが、release 再現性を重視して patch-level まで明示する。
```

`CLAUDE.md` は `AGENTS.md` への symlink のため、実装時は `git add AGENTS.md` を用いる (symlink のみが stage される罠を回避、MEMORY `project_claude_md_symlink.md` 参照)。

## 8. Tooling install commands

```bash
cargo install cargo-dist@0.31.0 --locked
cargo install cargo-release@1.1.2 --locked
```

`dist-workspace.toml` の `cargo-dist-version` と install した CLI の version は**完全一致させる**。CI 側もこの値に従って dist binary を fetch するため、version drift を防ぐには手元と設定を揃える。

## 9. Known risks / Open points

1. **Archive 圧縮形式の確定** — cargo-dist 0.31.0 の default が `.tar.xz` か `.tar.gz` か、初回 `dist plan` の出力で確認後に README install example を fix する
2. **`dist init --yes` の Cargo.toml 副作用** — `[workspace.metadata.dist]` を書き込もうとする場合があるので、init 後に `git diff Cargo.toml` を確認し、設定は `dist-workspace.toml` にのみ残す
3. **`cross` image の glibc version** — `cross` default image は古めの glibc で build されるため後方互換性は高い。新しい glibc 要求が発生したら `Cross.toml` で image を差し替える
4. **CI 実行時間** — 4 targets 並列 build は数分〜10 分程度。`cache-builds = true` で 2 回目以降は短縮
5. **`cargo release --no-verify` の誘惑** — `verify = true` default を保持することで pre-release の build 破綻を local で検出。verify を disable すると CI まで failure が遅延する
6. **Part B: plugin loader 実地検証** — §3.5 の 3 点をテストで確認
7. **Part A: 初回 v0.1.0 の CHANGELOG 手動編集** — cargo-release の自動置換は使えないため、手順書 (§2.8) に従って人間が `[Unreleased]` → `[0.1.0] - YYYY-MM-DD` に書き換える必要がある

## 10. Future extensions (V3 への path)

V2 の設定は additive で V3 に拡張可能:

- `installers += ["homebrew"]` + `publish-jobs += ["homebrew"]` + tap repo (`neko-neko/homebrew-tap`)
- `installers += ["npm"]` + `@neko-neko/belt` package
- `macos-sign` + Apple Developer 証明書で notarization
- `publish = true` + `publish-jobs += ["crates-io"]` + Cargo.toml から `publish = false` 削除
- Windows target 追加 (`x86_64-pc-windows-msvc`) + `installers += ["powershell"]`

これらは本 spec の範囲外。それぞれ別 spec として brainstorm する。
