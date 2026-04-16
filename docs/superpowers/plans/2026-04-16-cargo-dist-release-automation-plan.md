# cargo-dist Release Automation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Install and configure `cargo-dist 0.31.0` + `cargo-release 1.1.2` so that a `v*` tag push automatically produces a GitHub Release with 4-target binaries (macOS x86_64/aarch64, Linux x86_64/aarch64), a shell installer, tar archives, sigstore attestations, and keepachangelog-formatted release notes. Document install in `README.md` (`## Build` → `## Install`) and add a "Release Process" section to `AGENTS.md`. Final milestone: cut **v0.1.0** as the first release.

**Architecture:** Two hand-written config files (`dist-workspace.toml`, `release.toml`) drive two binaries: `cargo-dist` (CI side, emits and runs `.github/workflows/release.yml`) and `cargo-release` (dev side, bumps versions / updates CHANGELOG / tags / pushes). `Cargo.toml` gets metadata (`description`, `readme`); each crate's `Cargo.toml` adds `publish = false` + an independent `description`. `belt-core` also gets `[package.metadata.release] release = false` so the library crate is excluded from the release set but follows the shared version. `CHANGELOG.md` uses keepachangelog with `<!-- next-header -->` / `<!-- next-url -->` markers for cargo-release placeholder replacement. The initial v0.1.0 is cut manually (cargo-release presumes a bump from current version, so the first release is an edge case).

**Tech Stack:** Rust (Cargo workspace), cargo-dist 0.31.0, cargo-release 1.1.2, GitHub Actions, `cross` (automatic for aarch64-unknown-linux-gnu), sigstore attestations via `gh attestation verify`.

**Spec:** `docs/superpowers/specs/2026-04-16-cargo-dist-release-and-skill-consolidation-design.md` — Part A (§2)

**Related context:**

- MEMORY `project_belt_architecture.md` — workspace shape (belt-core / belt / belt-agent)
- MEMORY `project_claude_md_symlink.md` — `CLAUDE.md` は `AGENTS.md` への symlink。`git add CLAUDE.md` は symlink のみ stage する。本 plan は `AGENTS.md` を直接指す
- MEMORY `project_parallel_session_worktree_isolation.md` — 別セッションとの branch-switch race 回避のため worktree 推奨

**Prerequisites:**

1. Spec commit (`dcc31f6`) が main に到達していること
2. **Part B (`belt-agent-skill-consolidation`) が main に merge 済みであること** — Part B → Part A の順で実施 (spec §5)。Part B PR が open のままなら先にそちらを merge
3. Worktree 推奨:
   ```bash
   wt switch --create cargo-dist-release-automation
   # or
   git switch -c cargo-dist-release-automation
   ```

以降のタスクは全てこの worktree / branch 上で実行する。

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `Cargo.toml` | Edit | `[workspace.package]` に `description` + `readme` を追加 |
| `crates/belt-core/Cargo.toml` | Edit | `[package]` に `publish = false` + crate 独自 `description` + `readme.workspace = true` + `[package.metadata.release] release = false` |
| `crates/belt/Cargo.toml` | Edit | `[package]` に `publish = false` + crate 独自 `description` + `readme.workspace = true` |
| `crates/belt-agent/Cargo.toml` | Edit | `[package]` に `publish = false` + crate 独自 `description` + `readme.workspace = true` |
| `dist-workspace.toml` | Create | cargo-dist 設定 (4 targets / shell installer / attestations / cache-builds / pr-run-mode = plan) |
| `release.toml` | Create | cargo-release 設定 (shared-version / tag-name v{{version}} / allow-branch = ["main"] / publish = false / pre-release-replacements) |
| `CHANGELOG.md` | Create | keepachangelog 初期形、`<!-- next-header -->` / `<!-- next-url -->` マーカー必須 |
| `.github/workflows/release.yml` | **cargo-dist が auto-generate** | 人間は直接書かない。Task 9 の `dist init --yes` で出る。以後の手動編集は禁止 |
| `README.md` | Edit | `## Build` (L173-184) を `## Install` に rename & restructure。`### Shell installer` / `### Manual install` / `### Verify` / `### From source` 4 subsection |
| `AGENTS.md` | Edit | `## Non-Goals` の直前に `## Release Process` 節を新設 |

**Untouched (guard):**

- `crates/*/src/**`, `crates/*/tests/**` — コード変更なし。Cargo.toml 変更で build が通ることは Task 6 で verify
- `plugins/**`, `skills/**` — Part B の領域
- `pipelines/**`, `docs/**` (spec / plan を除く) — 無関係
- `.github/workflows/` の他の既存 workflow (あれば) — 新規 `release.yml` のみ追加

---

## Task 1: Pre-flight — tooling install + version verify

**Rationale:** cargo-dist と cargo-release をローカルにインストールし、version が spec の pin (`0.31.0` / `1.1.2`) と一致することを確認する。後続で `dist init --yes` / `cargo release` を使うので、事前に pin version で入れる。

**Files:**
- Read-only: cargo binstall / cargo install のローカル実行

- [ ] **Step 1: 既に入っていないか確認**

Run:
```bash
cargo dist --version 2>/dev/null || echo "not installed"
cargo release --version 2>/dev/null || echo "not installed"
```

Expected 以下のどちらか:
- `not installed` (初回): 次 Step でインストール
- `cargo-dist 0.31.0` と `cargo-release 1.1.2`: Step 3 に skip
- 別 version が入っている: Step 2 で再インストール (pin version に揃える)

- [ ] **Step 2: pin version でインストール**

Run:
```bash
cargo install cargo-dist@0.31.0 --locked
cargo install cargo-release@1.1.2 --locked
```

Expected: `Installed package cargo-dist v0.31.0` / `Installed package cargo-release v1.1.2` 相当のメッセージ。`--locked` で Cargo.lock の再現性を保証。

もし installation が失敗する場合 (rust-toolchain 不一致 / ネットワークエラー) は **STOP** し、`rustup show` で toolchain を確認。belt は MSRV 1.86 / 推奨 1.94.1+ (CLAUDE.md)。

- [ ] **Step 3: version 再確認**

Run:
```bash
cargo dist --version
cargo release --version
```

Expected:
```
cargo-dist 0.31.0
cargo-release 1.1.2
```

不一致なら **STOP**。

- [ ] **Step 4: ネットワーク前提の確認**

Run: `gh auth status`
Expected: 認証済み (`Logged in to github.com as <username>`)。Task 18-19 で `gh release download` / `gh attestation verify` を使うため。

未認証なら `gh auth login` を実施。

この Task ではファイル変更なし。commit 不要。

---

## Task 2: `Cargo.toml` (workspace root) に `description` + `readme` を追加

**Rationale:** spec §2.5。cargo-dist / crates.io / `cargo metadata` が workspace-wide の metadata を期待する。description は placeholder (crate ごとに独自文言を書くため)、readme は workspace の README を指す。

**Files:**
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: 現状の `[workspace.package]` を確認**

Run: `sed -n '9,16p' Cargo.toml`
Expected (現状):
```toml
[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.86"
authors = ["neko-neko"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/neko-neko/belt"
```

- [ ] **Step 2: description + readme を追加**

Use the Edit tool.

`old_string`:
```
[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.86"
authors = ["neko-neko"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/neko-neko/belt"
```

`new_string`:
```
[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.86"
authors = ["neko-neko"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/neko-neko/belt"
description = "Workspace root. See crate-level Cargo.toml for per-crate description."
readme = "README.md"
```

- [ ] **Step 3: `cargo metadata` で parse 成功を確認**

Run: `cargo metadata --format-version 1 --no-deps > /dev/null && echo OK`
Expected: `OK`。parse エラーなら **STOP**。

- [ ] **Step 4: diff 確認**

Run: `git diff Cargo.toml`
Expected: 2 行追加のみ (`description` + `readme`)、他は touch されていない。

この Task の commit は Task 6 と合わせて後で一括。ここでは個別 commit しない。

---

## Task 3: `crates/belt-core/Cargo.toml` の更新

**Rationale:** spec §2.5 table。belt-core は library crate として独自 description を持つ。`publish = false` で crates.io への誤 publish を防止。`[package.metadata.release] release = false` で cargo-release の release 対象から除外 (shared-version の制約だけ受ける)。

**Files:**
- Modify: `crates/belt-core/Cargo.toml`

- [ ] **Step 1: 現状を確認**

Run: `cat crates/belt-core/Cargo.toml`
Expected (先頭):
```toml
[package]
name = "belt-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
```

- [ ] **Step 2: description + publish + readme + [package.metadata.release] を追加**

Use the Edit tool.

`old_string`:
```
[package]
name = "belt-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
```

`new_string`:
```
[package]
name = "belt-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Core library for belt — pipeline model, parser, expander, engine, gates, linter"
readme.workspace = true
publish = false

[package.metadata.release]
release = false
```

- [ ] **Step 3: parse 成功を確認**

Run: `cargo metadata --format-version 1 --no-deps > /dev/null && echo OK`
Expected: `OK`。

- [ ] **Step 4: diff 確認**

Run: `git diff crates/belt-core/Cargo.toml`
Expected: 追加のみ、既存行は touch されていない。

---

## Task 4: `crates/belt/Cargo.toml` の更新

**Rationale:** spec §2.5 table。`belt` は human CLI で lint 用。crate 独自 description + `publish = false`。release 対象なので `[package.metadata.release]` は記載しない (default の release = true が適用される)。

**Files:**
- Modify: `crates/belt/Cargo.toml`

- [ ] **Step 1: 現状を確認**

Run: `cat crates/belt/Cargo.toml`
Expected (先頭 8 行):
```toml
[package]
name = "belt"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
```

- [ ] **Step 2: description + publish + readme を追加**

Use the Edit tool.

`old_string`:
```
[package]
name = "belt"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
```

`new_string`:
```
[package]
name = "belt"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "belt lint — static validator for belt YAML pipelines"
readme.workspace = true
publish = false
```

- [ ] **Step 3: parse 成功を確認**

Run: `cargo metadata --format-version 1 --no-deps > /dev/null && echo OK`
Expected: `OK`。

- [ ] **Step 4: diff 確認**

Run: `git diff crates/belt/Cargo.toml`
Expected: 3 行追加のみ。

---

## Task 5: `crates/belt-agent/Cargo.toml` の更新

**Rationale:** spec §2.5 table。`belt-agent` は agent runtime CLI。crate 独自 description + `publish = false`。release 対象。

**Files:**
- Modify: `crates/belt-agent/Cargo.toml`

- [ ] **Step 1: 現状を確認**

Run: `cat crates/belt-agent/Cargo.toml`
Expected (先頭):
```toml
[package]
name = "belt-agent"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
```

- [ ] **Step 2: description + publish + readme を追加**

Use the Edit tool.

`old_string`:
```
[package]
name = "belt-agent"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
```

`new_string`:
```
[package]
name = "belt-agent"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "belt-agent — runtime CLI for driving belt YAML pipelines from LLMs or scripts"
readme.workspace = true
publish = false
```

- [ ] **Step 3: parse 成功を確認**

Run: `cargo metadata --format-version 1 --no-deps > /dev/null && echo OK`
Expected: `OK`。

- [ ] **Step 4: diff 確認**

Run: `git diff crates/belt-agent/Cargo.toml`
Expected: 3 行追加のみ。

---

## Task 6: `cargo build --workspace` / `cargo test -p belt-core` で breakage が無いことを確認

**Rationale:** Cargo.toml 変更だけで code は触っていないが、workspace.package.description / readme.workspace = true などの metadata 依存を cargo が正しく解決できるか確認する。CLAUDE.md「コミット前に linter・型チェッカー・フォーマッター・テストを実行すること (変更 crate のみスコープ)」に従う。

**Files:**
- Read-only verification via cargo

- [ ] **Step 1: fmt / clippy / build が通ることを確認**

Run (変更したのは Cargo.toml だけなので `--all-targets` は不要、workspace 全体の metadata 整合だけ見る):
```bash
cargo fmt --check && \
cargo clippy --workspace -- -D warnings && \
cargo build --workspace
```

Expected: いずれも成功 (exit 0)。clippy が既存の warn を拾う場合は **無関係な drift** なので個別対処 (本 plan のスコープ外)。

- [ ] **Step 2: unit test の sanity**

Run: `cargo test -p belt-core --lib 2>&1 | tail -5`
Expected: `test result: ok.` で終わる行 + 全 pass。

workspace 全体 test を走らせるとインテグレーションが重いので、ここでは belt-core の lib test のみ。

- [ ] **Step 3: `cargo package --no-verify -p belt-agent --dry-run`** (将来 publish する場合の metadata check、`publish = false` でも dry-run は走る)

Run: `cargo package -p belt-agent --no-verify --list 2>&1 | head -10`
Expected: publish = false なので error message `the `publish` field in belt-agent's Cargo.toml is `false`` が出るはず。

もしこの error が出なければ (= publish が false に設定されていない) **STOP** し、Task 5 を見直す。

同じ verify を belt / belt-core にも実施: `cargo package -p belt --no-verify --list 2>&1 | head -3`、`cargo package -p belt-core --no-verify --list 2>&1 | head -3`。それぞれ publish false の error を確認。

この Task ではファイル変更なし。

---

## Task 7: `dist-workspace.toml` を新規作成

**Rationale:** spec §2.3。cargo-dist 設定を `Cargo.toml` の `[workspace.metadata.dist]` に混ぜず独立 file に書く (読みやすさ + 責務分離)。0.31.0 は pin、targets 4 つ、shell installer、attestations + cache-builds + fail-fast=false + pr-run-mode=plan の "V2 Balanced" 構成。

**Files:**
- Create: `dist-workspace.toml`

- [ ] **Step 1: 既存ファイルが無いこと確認**

Run: `ls dist-workspace.toml 2>&1`
Expected: `No such file or directory`。

- [ ] **Step 2: `dist-workspace.toml` を作成**

Use the Write tool to create `dist-workspace.toml` with content:

```toml
[dist]
# 0.x なので完全ピン (bump 時は `dist init --yes` 再実行 + 生成 workflow の再 commit の三点セット)
cargo-dist-version = "0.31.0"

# CI / hosting
ci = ["github"]
hosting = ["github"]

# Install 方法 (Windows 非対応なので powershell 不要)
installers = ["shell"]

# Target platforms (spec Q2 = B: macOS 両 + Linux 両、Windows なし)
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

- [ ] **Step 3: TOML parse 確認**

Run: `python3 -c "import tomllib; tomllib.load(open('dist-workspace.toml','rb')); print('ok')"`
Expected: `ok`。

- [ ] **Step 4: 内容 review**

Run: `cat dist-workspace.toml`
Expected: 上記 content と一致。

この Task の commit は後で Task 15 でまとめて実施。

---

## Task 8: `release.toml` を新規作成

**Rationale:** spec §2.4。cargo-release 設定を workspace root に置く。section header なし (workspace metadata は key 直接記述)。`shared-version = true` で全 crate の version を同期、`allow-branch = ["main"]` で main 以外からの release を禁止、`publish = false` で crates.io publish を全面 disable、`pre-release-replacements` で CHANGELOG.md の keepachangelog placeholder を自動更新。

**Files:**
- Create: `release.toml` (workspace root)

- [ ] **Step 1: 既存ファイルが無いこと確認**

Run: `ls release.toml 2>&1`
Expected: `No such file or directory`。

- [ ] **Step 2: `release.toml` を作成**

Use the Write tool to create `release.toml` with content:

```toml
# Workspace-wide cargo-release policy
# (workspace root の release.toml は section header なしで key を直接記述)

# 全 crate で version を揃える
shared-version = true

# Workspace release で commit を 1 本にまとめる (default だが明示)
consolidate-commits = true

# main からのみ release を許可
allow-branch = ["main"]

# Tag name: workspace 全体で "v{{version}}" を使う
tag-name = "v{{version}}"
tag-prefix = ""
tag-message = "chore: Release v{{version}}"

# Release commit
pre-release-commit-message = "chore: Release v{{version}}"

# crates.io publish は今回のスコープ外
publish = false

# verify (local cargo build) は default の true を保持
# → pre-release 時に build 破綻を catch できる

# CHANGELOG.md placeholder 置換 (keepachangelog)
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

- [ ] **Step 3: TOML parse 確認**

Run: `python3 -c "import tomllib; tomllib.load(open('release.toml','rb')); print('ok')"`
Expected: `ok`。

- [ ] **Step 4: cargo-release の config dump で認識されることを確認**

Run: `cargo release config 2>&1 | head -30`
Expected: `shared-version = true`, `allow-branch = ["main"]`, `tag-name = "v{{version}}"`, `publish = false` が含まれる dump が表示される。

- [ ] **Step 5: 内容 review**

Run: `cat release.toml`
Expected: 上記 content と一致。

---

## Task 9: `CHANGELOG.md` を初期作成 (keepachangelog + placeholder)

**Rationale:** spec §2.6。cargo-release の `pre-release-replacements` が `<!-- next-header -->` / `<!-- next-url -->` / `Unreleased` / `ReleaseDate` / `...HEAD` の 5 箇所を search/replace する。初期 v0.1.0 の時点で該当 marker が正しい位置にあることが必須。

**Files:**
- Create: `CHANGELOG.md` (workspace root)

- [ ] **Step 1: 既存ファイルが無いこと確認**

Run: `ls CHANGELOG.md 2>&1`
Expected: `No such file or directory`。

- [ ] **Step 2: `CHANGELOG.md` を作成**

Use the Write tool to create `CHANGELOG.md` with content:

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

- [ ] **Step 3: Markdown として view できるか確認 (linter 相当)**

Run:
```bash
grep -c "<!-- next-header -->" CHANGELOG.md
grep -c "<!-- next-url -->" CHANGELOG.md
grep -c "## \[Unreleased\] - ReleaseDate" CHANGELOG.md
grep -c "compare/v0\.1\.0\.\.\.HEAD" CHANGELOG.md
```

Expected: 各 `1`。どれかが `0` になれば **STOP**、CHANGELOG の marker が欠落している。

- [ ] **Step 4: cargo-release の dry-run で placeholder が検出されることを確認**

Run: `cargo release patch 2>&1 | tee /tmp/cargo-release-dry.log | tail -20`
Expected: dry-run 出力に `Replacing in CHANGELOG.md` のメッセージが含まれる。dry-run なので実際の変更はされない。

もし `Replacing in CHANGELOG.md` が一切出ない、またはエラー (e.g., "cannot find search pattern") が出る場合は **STOP** し、Step 2 の marker 位置を見直す。

- [ ] **Step 5: cargo-release の dry-run を undo (念のため)**

cargo-release は dry-run では変更を永続化しないが、念のため:
```bash
git checkout -- CHANGELOG.md Cargo.toml crates/*/Cargo.toml 2>/dev/null || true
# 未 stage の local 変更が誤って書き換わっていないか確認
git diff CHANGELOG.md Cargo.toml crates/*/Cargo.toml
```

Expected: diff は Task 2-5 + 新規 CHANGELOG.md だけで、cargo-release dry-run による drift はない。

---

## Task 10: `dist init --yes` を実行し `.github/workflows/release.yml` を生成、Cargo.toml 副作用を verify

**Rationale:** spec §2.8 / §9 Open point 2。`dist init` は設定を読み取り workflow を auto-generate するが、`Cargo.toml` に `[workspace.metadata.dist]` を書き込もうとする副作用があり得る。本 plan は `dist-workspace.toml` を single source of truth とするため、Cargo.toml に入り込んだら revert する。

**Files:**
- Create (auto): `.github/workflows/release.yml`
- Potential modify (auto, to be reverted): `Cargo.toml`

- [ ] **Step 1: 事前 snapshot**

Run: `git stash push --include-untracked -m "pre-dist-init-snapshot"`

stash を作って pre-state を保存。dist init が想定外のファイルを書き換えた場合、最小限の revert で戻せる。

Expected: `Saved working directory ...`。

- [ ] **Step 2: stash した内容を作業ツリーに戻す**

dist init は未 commit の dist-workspace.toml / release.toml / CHANGELOG.md などを読み取るので、stash から pop して戻す。

Run: `git stash pop`
Expected: 戻った状態。

(※ Step 1-2 は安全のための snapshot だが、Task 6 で既に各 Cargo.toml が OK と verify 済み、Task 7-9 の新規ファイルも valid なので実運用では Step 1-2 を skip して Step 3 に進んでも可。)

- [ ] **Step 3: `dist init --yes` を実行**

Run: `dist init --yes 2>&1 | tee /tmp/dist-init.log`
Expected: `✓` / `Generated` 等の success メッセージ。`.github/workflows/release.yml` が作成される。

エラー (e.g., "cargo-dist-version mismatch", "unsupported target") が出たら **STOP**。

- [ ] **Step 4: 生成物と副作用を確認**

Run: `git status`
Expected:
- `.github/workflows/release.yml` (新規)
- `Cargo.toml` が変更されていれば → Step 5 で revert
- `dist-workspace.toml` が変更されていれば → Step 6 で内容確認、想定外の追記あれば revert

- [ ] **Step 5: `Cargo.toml` に `[workspace.metadata.dist]` が書き込まれていたら revert**

Run: `git diff Cargo.toml`

もし `[workspace.metadata.dist]` セクションが追加されていたら:

Run:
```bash
# 該当セクションだけ手動で削除
# Edit tool を使い、追加された [workspace.metadata.dist] ブロック全体を削除
```

削除後:
```bash
cargo metadata --format-version 1 --no-deps > /dev/null && echo OK
```
Expected: `OK`。parse 破綻していないことを確認。

- [ ] **Step 6: `dist-workspace.toml` が変更されていたら revert**

Run: `git diff dist-workspace.toml`

dist init は既存 dist-workspace.toml を respect するので、通常は変更されない。もし変更があれば Edit tool で Task 7 で書いた内容に戻す。

- [ ] **Step 7: `.github/workflows/release.yml` の構造を確認**

Run: `head -40 .github/workflows/release.yml`
Expected:
- `on:` trigger に `push: tags: 'v[0-9]+.[0-9]+.[0-9]+'` 相当が含まれる
- `jobs:` に `plan`, `build-local-artifacts`, `host`, `announce` 相当
- `runs-on:` が含まれる (Ubuntu 系 LTS、0.31.0 generator の選択に依存)

Run: `grep 'runs-on' .github/workflows/release.yml | sort -u`
Expected: `ubuntu-` か `macos-` を含む runs-on が列挙。予期せぬ OS version (e.g., `ubuntu-18.04` 等の EOL) があれば spec §2.12 に従い `dist-workspace.toml` の `[dist.github-custom-runners].global` で pin を検討。

- [ ] **Step 8: permissions 埋め込みの確認**

Run: `grep -A2 'permissions:' .github/workflows/release.yml | head -20`
Expected: `attestations: write` と `id-token: write` が含まれる (github-attestations = true の効果)。

含まれない場合、dist-workspace.toml の `github-attestations = true` が効いていない可能性 → Task 7 を再 verify。

- [ ] **Step 9: YAML parse 確認**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('ok')"`
Expected: `ok`。YAML 構文エラーがあれば dist init 自体に問題があるので Step 3 を再実行。

---

## Task 11: `dist plan` で archive 拡張子と release shape を確認

**Rationale:** spec §9 Open point 1。cargo-dist 0.31.0 の default 圧縮形式 (`.tar.xz` か `.tar.gz`) を確定させ、後続 Task 13 の README install command を実物に合わせる。

**Files:**
- Read-only: `dist plan --output-format=json`

- [ ] **Step 1: `dist plan` を JSON 出力で実行**

Run: `dist plan --output-format=json 2>/dev/null | python3 -m json.tool > /tmp/dist-plan.json && echo OK`
Expected: `OK`。JSON が生成される。

もし error (e.g., `cargo-dist-version mismatch`) が出たら **STOP**。

- [ ] **Step 2: archive 拡張子を抽出**

Run:
```bash
python3 -c "
import json
plan = json.load(open('/tmp/dist-plan.json'))
archives = [a for r in plan.get('releases', []) for a in r.get('artifacts', []) if 'archive' in a.get('kind', '').lower() or 'executable' in a.get('kind', '').lower()]
for a in archives[:8]:
    print(a.get('name', a))
"
```

Expected 出力例:
```
belt-x86_64-apple-darwin.tar.xz
belt-aarch64-apple-darwin.tar.xz
belt-x86_64-unknown-linux-gnu.tar.xz
belt-aarch64-unknown-linux-gnu.tar.xz
```

(あるいは `.tar.gz`、どちらも OK。plan に書かれた拡張子を Task 13 で採用する)

もし 4 target 分の archive name が全部揃っていなければ **STOP** し、Task 7 の targets を見直す。

- [ ] **Step 3: 拡張子を記録**

手元メモに archive 拡張子 (`tar.xz` or `tar.gz`) を記録。Task 13 でこの値を使う。

- [ ] **Step 4: installer script 名も確認**

Run:
```bash
python3 -c "
import json
plan = json.load(open('/tmp/dist-plan.json'))
installers = [a for r in plan.get('releases', []) for a in r.get('artifacts', []) if 'installer' in a.get('kind', '').lower() or 'sh' in a.get('name', '')]
for a in installers:
    print(a.get('name', a))
" | head -3
```

Expected: `belt-installer.sh` 等。shell installer の正確な filename を Task 13 の README で使う。

---

## Task 12: `README.md` の `## Build` を `## Install` に rename & restructure

**Rationale:** spec §2.9。現行 README の `## Build` (単一 `cargo build --workspace` 小節) を `## Install` に書き換え、shell installer primary + manual install + verify + from source の 4 subsection 構成にする。既存の `cargo build -p belt / belt-agent` 抜粋も `### From source` に吸収して情報損失を防止。

**Files:**
- Modify: `README.md` (L173-184 周辺、`## Build` セクション)

- [ ] **Step 1: 現状の `## Build` セクションを抽出**

Run: `sed -n '/^## Build$/,/^## /p' README.md | head -20`
Expected 出力 (末尾 `## Claude Code Plugins (Working Examples)` の直前まで):
```
## Build

\`\`\`bash
cargo build --workspace
\`\`\`

Build only what you need:

\`\`\`bash
cargo build -p belt          # lint tool
cargo build -p belt-agent    # agent runtime
\`\`\`

## Claude Code Plugins (Working Examples)
```

- [ ] **Step 2: `## Build` セクションを `## Install` に置き換え**

Use the Edit tool.

**NOTE:** `TASK11_EXT` の部分は Task 11 Step 3 で記録した archive 拡張子 (`tar.xz` または `tar.gz`) に置き換える。`TASK11_TAR_FLAG` は `tar.xz` なら `tar -xJ`、`tar.gz` なら `tar -xz` に置き換える。

`old_string`:
```
## Build

```bash
cargo build --workspace
```

Build only what you need:

```bash
cargo build -p belt          # lint tool
cargo build -p belt-agent    # agent runtime
```
```

`new_string`:
```
## Install

### Shell installer (recommended)

Installs `belt` and `belt-agent` to `$HOME/.cargo/bin` (or configurable),
auto-detects platform.

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/neko-neko/belt/releases/latest/download/belt-installer.sh | sh
```

### Manual install (CI / Docker)

Pick a tarball matching your platform from the
[releases page](https://github.com/neko-neko/belt/releases):

```bash
# Example: Linux x86_64
curl -L https://github.com/neko-neko/belt/releases/latest/download/belt-x86_64-unknown-linux-gnu.TASK11_EXT \
  | TASK11_TAR_FLAG -C /usr/local/bin belt belt-agent
```

Replace the triple to match your platform:

| OS | Arch | Triple |
|---|---|---|
| macOS | Intel | `x86_64-apple-darwin` |
| macOS | Apple Silicon | `aarch64-apple-darwin` |
| Linux | x86_64 | `x86_64-unknown-linux-gnu` |
| Linux | aarch64 | `aarch64-unknown-linux-gnu` |

### Verify (optional)

```bash
gh release download v0.1.0 --repo neko-neko/belt --pattern '*.TASK11_EXT'
gh attestation verify belt-x86_64-unknown-linux-gnu.TASK11_EXT --repo neko-neko/belt
```

### From source

```bash
git clone https://github.com/neko-neko/belt.git && cd belt
cargo build --release --workspace
```

Build only what you need:

```bash
cargo build -p belt          # lint tool
cargo build -p belt-agent    # agent runtime
```
```

- [ ] **Step 3: `TASK11_EXT` / `TASK11_TAR_FLAG` を実値に置換**

If Task 11 で確認した拡張子が `tar.xz`:

Use the Edit tool (replace_all = true):
- `TASK11_EXT` → `tar.xz`
- `TASK11_TAR_FLAG` → `tar -xJ`

(2 回 Edit tool を invoke、それぞれで `replace_all = true` を指定)

If 拡張子が `tar.gz`:
- `TASK11_EXT` → `tar.gz`
- `TASK11_TAR_FLAG` → `tar -xz`

- [ ] **Step 4: 置換漏れのチェック**

Run: `grep -n "TASK11_" README.md 2>&1`
Expected: 0 件ヒット。placeholder が残っていれば **STOP**、Step 3 をやり直し。

- [ ] **Step 5: `## Install` セクションを目視確認**

Run: `sed -n '/^## Install$/,/^## /p' README.md`
Expected: 4 subsection (`### Shell installer (recommended)` / `### Manual install (CI / Docker)` / `### Verify (optional)` / `### From source`) と最後に `## Claude Code Plugins (Working Examples)`。

- [ ] **Step 6: Markdown 構文の安全確認**

Run: `grep -c "^## " README.md`
Expected: 変更前と同数 (Build が Install に rename されただけで section 数は不変)。

---

## Task 13: `AGENTS.md` に `## Release Process` セクション追加

**Rationale:** spec §7。`## Technology Stack` / `## 依存管理ポリシー` の流れを継ぎ、`## Non-Goals` の直前に新セクションを追加。CLAUDE.md は AGENTS.md への symlink なので、MEMORY `project_claude_md_symlink.md` に従い `AGENTS.md` を直接編集し `git add AGENTS.md` で stage する。

**Files:**
- Modify: `AGENTS.md` (L ~? 周辺、`## Non-Goals (やらないこと)` の直前)

- [ ] **Step 1: 挿入位置 (Non-Goals の直前) を確認**

Run: `grep -n "^## " AGENTS.md | head -30`
Expected: section 一覧が表示され、`## Non-Goals (やらないこと)` がある。その直前の section を特定。

- [ ] **Step 2: `## Release Process` 節を `## Non-Goals (やらないこと)` の直前に挿入**

Use the Edit tool.

`old_string`:
```
## Non-Goals (やらないこと)
```

`new_string`:
```
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

```bash
cargo release patch -x          # 0.1.0 → 0.1.1
cargo release minor -x          # 0.1.0 → 0.2.0
cargo release major -x          # 0.1.0 → 1.0.0
cargo release 0.2.3 -x          # 明示 version
cargo release rc -x             # pre-release (0.2.0-rc.1)
cargo release release -x        # pre-release extension 除去
cargo release changes           # 前 tag 以降の commit 一覧 (CHANGELOG 執筆補助)
```

Bump は `allow-branch = ["main"]` により main からのみ実行可能。CI workflow (`release.yml`) は `v*` tag push で自動発火する。

### 初回 release の特殊ケース

`workspace.package.version` が既に `0.1.0` の状態では cargo-release は「次 version への bump」を前提とするため、初回 v0.1.0 は手動 tag 作成で発火する (`git tag v0.1.0 && git push --tags`)。初回前に `CHANGELOG.md` の `[Unreleased]` 節を `[0.1.0] - YYYY-MM-DD` に手で書き換える。2 回目以降は `cargo release minor -x` 等で CHANGELOG placeholder 置換を含めて完全自動化される。

### crates.io

**publish 対象外** (MVP)。`release.toml` で `publish = false`、各 crate の `Cargo.toml` にも `publish = false` を冗長記述。将来 crates.io 対応時は両方を外す + `publish-jobs = ["crates-io"]` を `dist-workspace.toml` に追加。

### 供給チェーン検証

`github-attestations = true` により release asset は sigstore 署名される。利用者検証:

```bash
gh attestation verify <asset>.tar.xz --repo neko-neko/belt
```

README の Install section に optional step として記載。

### 依存管理ポリシーとの整合

Release tooling は既存の「依存管理ポリシー > バージョン指定ポリシー」に従う。`cargo-dist` は 0.x ゆえ完全ピン、`cargo-release` は 1.x ゆえ caret 許容の対象だが、release 再現性を重視して patch-level まで明示する。

## Non-Goals (やらないこと)
```

- [ ] **Step 3: AGENTS.md の整合性確認**

Run: `grep -c "^## Release Process$" AGENTS.md`
Expected: `1`。

Run: `grep -c "^## Non-Goals (やらないこと)$" AGENTS.md`
Expected: `1`。

どちらかが 0 や 2 以上なら Step 2 をやり直す。

- [ ] **Step 4: symlink の整合確認**

Run: `ls -la CLAUDE.md`
Expected: `CLAUDE.md -> AGENTS.md` の symlink。

Run: `diff CLAUDE.md AGENTS.md`
Expected: 差分なし (symlink なので同一 content)。

---

## Task 14: 全体 build / clippy / test の再確認

**Rationale:** Task 2-13 の全変更完了後、workspace 全体が build / clippy / test を通ることを確認。特に Cargo.toml metadata 変更と dist-workspace.toml / release.toml 追加が既存の cargo 挙動を壊していないか。

**Files:**
- Read-only verification

- [ ] **Step 1: fmt / clippy / build**

Run:
```bash
cargo fmt --check && \
cargo clippy --workspace --all-targets -- -D warnings && \
cargo build --workspace
```
Expected: 全 success。

- [ ] **Step 2: 変更 crate の test**

Run: `cargo test --workspace 2>&1 | tail -5`
Expected: `test result: ok.` 全 pass。

- [ ] **Step 3: `dist plan` 再実行して設定変更後も plan が通ることを確認**

Run: `dist plan --output-format=json > /dev/null && echo OK`
Expected: `OK`。

- [ ] **Step 4: `cargo release --dry-run` 相当**

Run: `cargo release 0.1.0 2>&1 | tee /tmp/cargo-release-final-dry.log | tail -30`
Expected: `WARN Releasing 0.1.0 is the same as current version, falling through as no-op` 的なメッセージ、または dry-run が完了 (no error)。

注: 0.1.0 は現在 version と同じ。`cargo release release` なら pre-release extension を除去するが、現 version に extension が無いので no-op。次 Task (PR 準備) の前に dry-run で sanity を取るだけ。

---

## Task 15: Part A 全体を 1 branch に commit + PR 作成

**Rationale:** Part A の全変更を 1 PR にまとめる (Part B とは独立 PR)。`dist init` 生成物 (`.github/workflows/release.yml`) も同 PR に含める。

**Files:** all changes from Task 2-13.

- [ ] **Step 1: 変更状態の最終確認**

Run: `git status`
Expected: 以下の状態:
- Modified: `Cargo.toml`, `crates/belt-core/Cargo.toml`, `crates/belt/Cargo.toml`, `crates/belt-agent/Cargo.toml`, `README.md`, `AGENTS.md`
- New: `dist-workspace.toml`, `release.toml`, `CHANGELOG.md`, `.github/workflows/release.yml`

(`CLAUDE.md` は symlink なので status には出ないが、`AGENTS.md` modify で OK)

- [ ] **Step 2: stage (`git add`)**

Run:
```bash
git add Cargo.toml \
        crates/belt-core/Cargo.toml \
        crates/belt/Cargo.toml \
        crates/belt-agent/Cargo.toml \
        dist-workspace.toml \
        release.toml \
        CHANGELOG.md \
        .github/workflows/release.yml \
        README.md \
        AGENTS.md
```

- [ ] **Step 3: stage 状態を diff 確認**

Run: `git diff --cached --stat`
Expected: 10 file 変更 (6 modified + 4 new)。

- [ ] **Step 4: commit**

Run:
```bash
git commit -m "$(cat <<'EOF'
build: add cargo-dist + cargo-release release automation

Configure cargo-dist 0.31.0 and cargo-release 1.1.2 so that `v*` tag push
produces a multi-platform GitHub Release with a shell installer and
sigstore attestations. 4 targets: macOS x86_64/aarch64 + Linux
x86_64/aarch64. Windows is out of scope.

- `dist-workspace.toml` — cargo-dist config (targets, installers,
  attestations, cache-builds, pr-run-mode=plan)
- `release.toml` — cargo-release config at workspace root (shared-version,
  tag-name v{{version}}, allow-branch=[main], publish=false,
  pre-release-replacements for CHANGELOG placeholders)
- `CHANGELOG.md` — keepachangelog initial form with <!-- next-header --> /
  <!-- next-url --> markers
- `.github/workflows/release.yml` — auto-generated by `dist init --yes`
- `Cargo.toml` + `crates/*/Cargo.toml` — workspace description/readme,
  per-crate descriptions, publish=false, belt-core release=false
- `README.md` — replace `## Build` with `## Install` (shell installer /
  manual / verify / from source)
- `AGENTS.md` — new "Release Process" section

Per spec: docs/superpowers/specs/2026-04-16-cargo-dist-release-and-skill-consolidation-design.md Part A (§2).
EOF
)"
```

GPG 署名エラー時は `-c commit.gpgsign=false` を前置して再試行。

- [ ] **Step 5: push + PR**

Run:
```bash
git push -u origin cargo-dist-release-automation
gh pr create --title "Add cargo-dist + cargo-release release automation" --body "$(cat <<'EOF'
## Summary

- Configure `cargo-dist 0.31.0` + `cargo-release 1.1.2` for GitHub Release automation on `v*` tag push.
- 4 targets: macOS x86_64/aarch64, Linux x86_64/aarch64 (Windows deliberately excluded).
- Shell installer (primary, `curl | sh`) + tar archives + sigstore attestations.
- README `## Build` → `## Install` with 4 subsections (shell installer / manual / verify / from source).
- `AGENTS.md` gets a "Release Process" section documenting the tooling, commands, and initial-release caveat.

## Test plan

- [x] `cargo build --workspace` passes
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [x] `cargo test --workspace` passes
- [x] `dist plan --output-format=json` produces 4 target archives + installer
- [x] `cargo release 0.1.0` dry-run is no-op (current version = 0.1.0)
- [ ] After merge: cut v0.1.0 per the runbook in Task 16 (manual CHANGELOG edit + manual tag)
- [ ] After tag push: verify GitHub Actions release workflow completes and `gh attestation verify` passes on downloaded assets

Per spec: `docs/superpowers/specs/2026-04-16-cargo-dist-release-and-skill-consolidation-design.md` Part A (§2). Part B (skill consolidation) shipped in a separate PR, merged first.
EOF
)"
```

- [ ] **Step 6: PR CI (plan job) の確認**

`gh pr checks` で plan job が GREEN であることを確認。`pr-run-mode = "plan"` なので build は走らず plan のみ ~30秒。

Plan job が FAIL した場合:
- `dist-workspace.toml` の設定ミス → PR 上で fixup commit
- `release.toml` parse エラー → 同上
- `Cargo.toml` metadata 欠損 → Task 2-5 を見直し

FAIL log は GitHub Actions UI で確認。

- [ ] **Step 7: PR review + merge**

PR を self-review + 可能なら third-party review。merge 後、Task 16 (初回 release) に進む。

---

## Task 16: 初回 v0.1.0 release — CHANGELOG 手動編集 + 手動 tag

**Rationale:** spec §2.8 / §9 Open point 7。`workspace.package.version` が既に `0.1.0` のため cargo-release は「次 version への bump」を前提とする → `cargo release release -x` は no-op になる。初回 v0.1.0 は `CHANGELOG.md` を手動編集 + 手動 tag 作成 + push で発火する。**この Task は main branch 上で実施する。**

**Files:**
- Modify: `CHANGELOG.md` (Task 9 で作った内容の `[Unreleased]` 節を `[0.1.0] - 2026-04-XX` に置換)

**Prerequisites:** Task 15 の PR が main に merge 済みであること。

- [ ] **Step 1: main に switch + pull**

```bash
git switch main
git pull origin main
```

Expected: Task 15 の merge commit が local に到着。

- [ ] **Step 2: `CHANGELOG.md` を手動編集**

今日の日付を `date +%Y-%m-%d` で取得し、CHANGELOG を以下のように書き換える。

Use the Edit tool on `CHANGELOG.md`.

`old_string`:
```
<!-- next-header -->

## [Unreleased] - ReleaseDate

### Added
```

`new_string`:
```
<!-- next-header -->

## [Unreleased] - ReleaseDate

## [0.1.0] - YYYY-MM-DD

### Added
```

(`YYYY-MM-DD` を実行日の日付に置き換え。例: `2026-04-17`)

- [ ] **Step 3: URL compare link を追加**

Use the Edit tool on `CHANGELOG.md`.

`old_string`:
```
<!-- next-url -->
[Unreleased]: https://github.com/neko-neko/belt/compare/v0.1.0...HEAD
```

`new_string`:
```
<!-- next-url -->
[Unreleased]: https://github.com/neko-neko/belt/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/neko-neko/belt/releases/tag/v0.1.0
```

- [ ] **Step 4: diff 確認**

Run: `git diff CHANGELOG.md`
Expected: 2 箇所追加のみ (`## [0.1.0] - YYYY-MM-DD` と `[0.1.0]: ...tag/v0.1.0`)。

- [ ] **Step 5: commit**

Run:
```bash
git add CHANGELOG.md
git commit -m "chore: prepare v0.1.0 release"
```

- [ ] **Step 6: tag 作成**

Run:
```bash
git tag v0.1.0 -m "Release v0.1.0"
```

- [ ] **Step 7: push (branch + tag 同時)**

Run:
```bash
git push origin main
git push origin v0.1.0
```

Expected: 両方成功。tag push が GitHub Actions `release.yml` workflow を trigger する。

---

## Task 17: Release workflow の監視 + asset 検証

**Rationale:** Task 16 の tag push が trigger した GitHub Actions workflow を監視し、success を確認。生成された release asset を download して `gh attestation verify` で検証する。

**Files:**
- Read-only: `gh run list`, `gh release view`, `gh attestation verify`

- [ ] **Step 1: workflow 起動を確認**

Run: `gh run list --workflow=release.yml --limit=3`
Expected: `in_progress` or `queued` の run が 1 つ。

- [ ] **Step 2: workflow 完了を待つ**

Run: `gh run watch $(gh run list --workflow=release.yml --limit=1 --json databaseId --jq '.[0].databaseId')`

Expected: 5-15 分程度で complete (4 targets + cross-compile + installer + attestation)。`✓` で終われば success。

FAIL した場合:
- `gh run view --log` で log を確認
- よくある原因: runner OS version mismatch / cross crate fetch 失敗 / attestation permissions 不足
- 修正 → workflow 再実行 or tag 削除 + 再 push (rollback 手順は §13 spec 参照)

- [ ] **Step 3: release が作成されたことを確認**

Run: `gh release view v0.1.0`
Expected: `title: v0.1.0`, `assets:` list に:
- `belt-installer.sh`
- `belt-x86_64-apple-darwin.tar.xz` (or tar.gz)
- `belt-aarch64-apple-darwin.tar.xz`
- `belt-x86_64-unknown-linux-gnu.tar.xz`
- `belt-aarch64-unknown-linux-gnu.tar.xz`
- `*.sha256` / sigstore attestations

8+ asset が揃っていることを確認。

- [ ] **Step 4: asset 1 つを download + attestation verify**

自分の platform に合う asset (Apple Silicon mac なら `aarch64-apple-darwin`) を download:

Run:
```bash
mkdir -p /tmp/belt-v0.1.0
cd /tmp/belt-v0.1.0
gh release download v0.1.0 --repo neko-neko/belt --pattern 'belt-aarch64-apple-darwin.*'
gh attestation verify belt-aarch64-apple-darwin.tar.xz --repo neko-neko/belt
```

Expected: `✓ Verification succeeded!` メッセージ。

FAIL (`no attestations found` 等) した場合、workflow の attestations 生成が欠落している → `dist-workspace.toml` の `github-attestations = true` を再 verify + workflow 再実行。

- [ ] **Step 5: tar archive 展開 + 動作確認**

Run:
```bash
cd /tmp/belt-v0.1.0
tar -xJf belt-aarch64-apple-darwin.tar.xz   # または tar -xzf if tar.gz
./belt-aarch64-apple-darwin/belt --version
./belt-aarch64-apple-darwin/belt-agent --version
```

Expected: どちらも `belt 0.1.0` / `belt-agent 0.1.0` 的な出力。

- [ ] **Step 6: shell installer の smoke test (別 shell で)**

別 shell で:
```bash
curl --proto '=https' --tlsv1.2 -LsSf \
    https://github.com/neko-neko/belt/releases/latest/download/belt-installer.sh \
  | sh
belt --version
belt-agent --version
```

Expected: 自動 download + `$HOME/.cargo/bin/` に配置 (default) + version 出力。

(※ ローカル開発マシンに `cargo install` 版の belt が入っていると install path が衝突するかも。衝突する場合、installer が warning を出す。その場合は PATH 優先度で確認。)

- [ ] **Step 7: README.md の install コマンドを手元で実行してみて documentation が正しいか**

Task 12 で書いた install command をそのまま copy-paste で実行し、同じ結果が得られるか verify。

- [ ] **Step 8: release の成功を報告**

- PR (Task 15) に comment で "Released v0.1.0 successfully, attestations verified, `<platform>` binary smoke tested."
- Slack / internal channel に announce (optional)

---

## Self-Review Checklist

1. **Spec coverage:**
   - §2.3 dist-workspace.toml — Task 7 ✓
   - §2.4 release.toml — Task 8 ✓
   - §2.5 Cargo.toml additions — Task 2-5 ✓
   - §2.6 CHANGELOG.md — Task 9 + Task 16 (手動 edit) ✓
   - §2.7 Release commands — AGENTS.md (Task 13) 経由でドキュメント化、初回 v0.1.0 は Task 16 で実演 ✓
   - §2.8 Initial v0.1.0 release — Task 16 ✓
   - §2.9 README.md install section — Task 12 ✓
   - §2.10 PR check behavior — Task 15 Step 6 ✓
   - §2.11 Supply chain verification — Task 17 Step 4 ✓
   - §2.12 aarch64-linux cross-compile — Task 10 Step 7 で runner OS version 確認 ✓
   - §2.13 Required permissions — Task 10 Step 8 で attestations 権限 verify ✓
   - §7 AGENTS.md additions — Task 13 ✓
   - §8 Tooling install commands — Task 1 ✓
   - §9 Known risks / Open points — 個別 Task に分散 (archive 拡張子=Task 11, Cargo.toml 副作用=Task 10, cross image=非対処 (wait-and-see), CI 時間=Task 15 CI, verify default=Task 14, plugin loader=Part B plan に分離) ✓

2. **No placeholders:**
   - Task 12 Step 3 で `TASK11_EXT` / `TASK11_TAR_FLAG` を Task 11 の結果値で置換する手順を明示 ✓
   - Task 16 Step 2 で日付 `YYYY-MM-DD` を実行日で置換する手順を明示 ✓
   - その他 "TBD" / "TODO" / "implement later" なし ✓

3. **Type consistency:**
   - `dist-workspace.toml` の targets list と README の install table で同一 triple を使用 ✓
   - `release.toml` の tag-name (`v{{version}}`) と Task 16 の `git tag v0.1.0` で format 一致 ✓
   - `cargo-dist-version = "0.31.0"` pin と Task 1 の `cargo install cargo-dist@0.31.0` で version 一致 ✓

4. **Rollback paths:**
   - Task 10 副作用 revert → git diff + Edit ✓
   - Task 15 pre-merge fixup → PR に追加 commit ✓
   - Task 16 tag 誤作成 → `git push --delete origin v0.1.0` + GitHub Release 画面 draft 削除 (spec §13 参照) ✓
   - Task 17 workflow FAIL → rollback せず原因調査、必要なら tag 削除 + 再 push ✓
