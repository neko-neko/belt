# flowrail — Agent Workflow Engine

`flowrail` は LLM エージェント向けの軽量ワークフローエンジン CLI。YAML で宣言された決定論的 state machine を、冪等かつ拡張可能に駆動する。

## プロジェクト概要

- **言語**: Rust
- **MSRV**: 1.86.0
- **推奨開発版 / CI toolchain**: 1.94.1 以上 (CVE-2026-33055 / CVE-2026-33056 回避)
- **Edition**: 2024
- **アーキテクチャ**: Cargo workspace (3 crates)
- **設計思想**: Linux 哲学 "Do One Thing and Do It Well" / Tiny by Constraint
- **CLI 体系**: kubectl/helm 風の リソース+動詞 (5 リソース: pipeline / run / state / snapshot / help)
- **Linear tracking**: [CLA-5](https://linear.app/neko-neko/issue/CLA-5)

## Binary Separation (原則 8: Separation by Audience)

flowrail は 2 種類の利用者を想定し、それぞれに最適化された**別バイナリ**を提供する:

| | Agent CLI (`flowrail`) | Human CLI (`flowrail-tui`) |
|---|---|---|
| 重視 | security, minimal deps, determinism, scriptability | UX, interactivity, visualization |
| 依存 | flowrail-core + clap のみ | flowrail-core + ratatui + crossterm |
| I/O | JSON / plain text (パイプ前提) | rich terminal UI |
| 起動 | per-command, fast startup | long-running session |

**ビルド時の crate 分離**で保証される (feature flag ではなく独立 crate / 独立 binary)。これにより agent CLI の supply chain risk が tui 依存の影響を受けない。

## Crate 構成

```
flowrail/
├── crates/
│   ├── flowrail-core/    # 📦 library: state machine / rule set / primitives / hooks
│   ├── flowrail/         # 🤖 agent CLI binary (binary name: flowrail)
│   └── flowrail-tui/     # 👤 human CLI binary (binary name: flowrail-tui) — Phase 3
```

| Crate | 役割 | Phase |
|-------|------|-------|
| `flowrail-core` | pure library。state machine, rule set resolver, 4 primitive checks, hook executor, 8 directives, YAML 抽象層 (`src/yaml/`) | Phase 1 |
| `flowrail` | agent 用 CLI binary。automation-first、security-first、minimal dependencies | Phase 1 |
| `flowrail-tui` | human 用 TUI binary。ratatui-based、interactive 監視・デバッグ | Phase 3 |

### 依存関係フロー

```
flowrail-core (pure library)
     │
     ├──▶ flowrail (agent bin, minimal deps)
     │
     └──▶ flowrail-tui (human bin, UI deps isolated)
```

`flowrail` と `flowrail-tui` の間には依存関係がない。両者は独立して `flowrail-core` を参照する。

## flowrail Core が知る 5 概念

1. **State Machine** — phase 遷移の決定論的駆動
2. **Artifact Lifecycle** — verification/validation 契約
3. **Rule Set Resolver** — YAML 宣言の解決と評価
4. **4 Core Primitive Checks** — `file_exists` / `cmd_exit` / `regex_match` / `git_status`
5. **Hook Executor** — lifecycle event に基づく外部コマンド実行

これ以外の概念 (audit, regate, inner-loop, autonomy, phase-summary, linear-sync 等) はすべて rule set 側に外出しする。

## 8 Built-in Directives / 4 LLM Response Types

Core が rule set 評価エンジンとして必要な最小語彙:

- **Directives**: `confirm` / `classify_then_regate` / `produce_artifact` / `validation_question` / `regate_action` / `hook_command` / `phase_confirm` / `classifier_question`
- **Response types**: verdict / check result / classifier response / validation result

## 原則 (8 つ)

詳細は `docs/specs/` を参照。特に重要なのは:

- **原則 2 (精密化済み)**: 高次 workflow semantics (audit/regate/inner-loop/autonomy) は知らない。ただし低次語彙 (8 directive + 4 response type) は知る
- **原則 7 (Tiny by Constraint)**: Phase 1 は ~3,500-3,800 LOC、Phase 2 Testing Primitives で ~4,400 LOC
- **原則 8 (Separation by Audience)**: agent CLI と human CLI を別 crate / 別 binary に分離、依存関係を物理的に分離

## Technology Stack

### Workspace `Cargo.toml` (SSOT)

`[workspace.dependencies]` で全依存を一元管理する。詳細は `Cargo.toml` を参照。主要な crate と version:

| crate | version | 備考 |
|-------|---------|------|
| `clap` | `4.6` | agent CLI 用 |
| `serde` | `1.0.228` | derive feature |
| `serde-saphyr` | **`=0.0.23`** | 0.0.x 完全ピン必須 (pre-1.0) |
| `jsonschema` | **`=0.45.0`** | 0.x 完全ピン、HTTPS 解決は disable |
| `minijinja` | `2.19` | default-features=false + 必要 features のみ |
| `miette` | `7.6` | core は plain、binary は `fancy-no-backtrace` |
| `thiserror` | `2.0.18` | |
| `notify` | `8.2` | v9 rc は保留 (MSRV 1.88 要求) |
| `uuid` | `1.23` | v4 / v7 / v5 / serde features |
| `regex` | `1.12` | |
| `ratatui` | **`=0.30.0`** | TUI 用、0.x 完全ピン |
| `crossterm` | **`=0.29.0`** | TUI 用、0.x 完全ピン |

### 各 crate の依存

**`flowrail-core`** (library):
- `serde`, `serde-saphyr`, `jsonschema`, `minijinja`, `miette` (plain), `thiserror`, `notify`, `uuid`, `regex`, `glob`

**`flowrail`** (agent CLI):
- `flowrail-core` (path dependency)
- `clap`
- `miette` (with `fancy-no-backtrace` feature)
- ⚠ **TUI 系依存は一切追加禁止** (原則 8)

**`flowrail-tui`** (human CLI, Phase 3):
- `flowrail-core` (path dependency)
- `ratatui`, `crossterm`
- `miette` (with `fancy-no-backtrace` feature)

## 依存管理ポリシー

### 追加制限 (agent CLI `flowrail`)

- **TUI/GUI ライブラリは追加禁止** (ratatui, crossterm, tui-rs, iced, egui)
- **ネットワーク通信ライブラリは原則禁止** (reqwest, hyper, tonic)。必要時は spec に justification を記載
- **非同期ランタイムは原則禁止** (tokio, async-std, smol)。同期実行を基本とする
- **unsafe code は原則禁止** (`#![forbid(unsafe_code)]` をクレート root に)
- 新規依存追加時は `cargo audit` と `cargo deny` を通す

### バージョン指定ポリシー

| crate 種別 | 指定方式 | 理由 |
|-----------|---------|------|
| 0.x crates | `=X.Y.Z` 完全ピン | SemVer 前、patch でも breaking 可 |
| 1.x crates | `"X.Y"` caret 許容 | SemVer 準拠で安全 |

**完全ピン対象 (0.x)**: `serde-saphyr`, `jsonschema`, `ratatui`, `crossterm`
**caret 許容 (1.x+)**: `clap`, `serde`, `minijinja`, `miette`, `thiserror`, `uuid`, `regex`, `notify`

### Cargo.lock

- **コミット対象**: flowrail は binary project のため `Cargo.lock` をコミットする
- 再現可能なビルドを保証

## Non-Goals (やらないこと)

- 高次 workflow semantics (audit, regate, inner-loop, autonomy) の core 組み込み
- **Rule Set の `layer` metadata 必須化** (2026-04-05 撤回、YAML Universe 対応のため)
- **層間逆依存の enforcement、cycle detection の実装** (rule set 作者の自己責任、別の静的解析ツールに委ねる)
- 複雑な assertion DSL / N 回実行統計 / 可視化ダッシュボード
- Web UI、リアルタイム協調編集
- `git2` / `gix` 等の C 依存ライブラリ (git CLI 直接呼び出しで統一)
- async runtime 必須化 (`tokio` / `async-std` を core に持ち込まない)
- agent CLI binary への TUI/GUI 依存の混入

## YAML Universe (Future Vision)

将来的に、rule set を Web で誰でも公開/取得できるエコシステムを構築する。flowrail core が layer/structure 知識を持たないことがエコシステム成立の前提である。詳細は `docs/specs/` 内の "YAML Universe (Future)" セクションを参照。

## Phase 1 スコープ (Pipeline Lint/Fmt MVP)

- 24 タスク、TDD ベース
- spec: `docs/specs/2026-04-05-flowrail-cli-rule-set-architecture-design.md`
- plan: `docs/plans/2026-04-05-flowrail-phase1-pipeline-lint-fmt.md`
- 実装対象: `crates/flowrail-core/` + `crates/flowrail/`
- `crates/flowrail-tui/` は Phase 1 では placeholder (空 `main.rs` のみ)
- **Phase A 着手前 gate**: Feasibility Mapping + Impact Analysis の完成 (現在 spec には骨子のみ、plan フェーズで完成予定)

## Phase 2 / Phase 3 予定

- **Phase 2**: Testing Framework (Recording / Replay / Assertion / State Diff / Pipeline Test Runner)。追加 ~800 LOC、累計 ~4,400 LOC
- **Phase 3**: `flowrail-tui` 本体実装。ratatui-based TUI、リアルタイム state 可視化

## 歴史的コンテキスト

- **2026-04-05**: プロジェクト発足 (旧名 `jig` = 治具)
- **2026-04-05**: spec-review 完了、169 raw findings → 42 consensus → 15 適用
- **2026-04-05**: `jig` → `flowrail` に rename (丸パクリ回避、agent workflow らしさ重視)
- **2026-04-05**: Rule Set `layer` 機構を撤回 (YAML Universe 構想と衝突、tiny 原則に反する)
- **2026-04-05**: dotfiles から独立レポジトリへ分離 (OSS 公開を見据えた自己完結化)
- **2026-04-05**: Cargo workspace 化、agent CLI と TUI CLI を別 crate に分離 (原則 8 追加)
- **2026-04-05**: Technology Stack を Rust 1.94.1 stable 時代に再調査、MSRV 1.86 に統一

元々は dotfiles の `claude/skills/workflow-engine/` で実装していた workflow engine を、独立 Rust CLI として再実装するもの。

## 関連リソース

- **Linear**: [CLA-5 flowrail Phase 1 Pipeline Lint/Fmt MVP](https://linear.app/neko-neko/issue/CLA-5)
- **Workflow Report Document**: https://linear.app/neko-neko/document/workflow-report-cla-5-b50b8db30faf
- **Upstream inspiration**: dotfiles `claude/skills/workflow-engine/` (https://github.com/neko-neko/dotfiles)
- **本 rename/repo split/layer retraction の design doc**: dotfiles 側 `docs/superpowers/specs/2026-04-05-flowrail-rename-and-repo-split-design.md` (commit 409a4f5)

## 開発規約 (継承)

### コミュニケーション

- 技術的に誤った意見には根拠を示して反論する
- 曖昧な指示に対しては推測で進めず、具体的に確認する
- 設計・レビュー・分析は分割確認せず、完全な形で一度に出力する

### 実装規律

- 初回の実装パスでバリデーション (範囲制約、境界値、型チェック) を含める
- コミット前に `cargo fmt` / `cargo clippy -- -D warnings` / `cargo test` を実行 (変更ファイルのみスコープ)
- LSP (rust-analyzer) が利用可能な場合はシンボル調査・定義元・参照箇所の特定に Grep/Glob より優先
- GPG 署名エラー時は `-c commit.gpgsign=false` で再試行 OK

### CLI 命名

- Linux 哲学準拠、kubectl/helm/aws-cli 風のリソース+動詞で統一
- 動詞は意図が明確 (例: `step` (advance ではない), `prune` (clean ではない), `reset` は `--to-phase` 必須)

### Verification Contract

- 非 trivial な変更 (3 ファイル以上 / backend / infrastructure) は独立 verification 必須
- `verified` / `PASS` 主張は実コマンドと出力を伴う
- 境界値・異常系・idempotency の adversarial probe を少なくとも 1 つ含める

## Known Risks (要監視)

- **`miette`**: 7.6.0 以降 1 年更新停滞 (2025-04-27 → 現在)。代替候補は `annotate-snippets`。maintainer 活動を定期確認
- **`notify v9`**: rc.2 段階、v9 stable で MSRV 1.88 要求。v8.2.0 で当面維持、v9 stable 時に再評価
- **`serde-saphyr 0.0.x`**: SemVer 前。patch でも breaking 可能性、`cargo update` 前に changelog 確認必須
- **Rust 1.94.0 CVE (CVE-2026-33055/33056)**: Cargo 同梱 `tar` crate の脆弱性、1.94.1 で修正済み。toolchain は 1.94.1+ を使用 (`rust-toolchain.toml` で固定済み)
- **`jsonschema` default HTTPS fetch**: `default-features = false` + `resolve-file` のみに絞ることで不要な TLS stack を除外済み
