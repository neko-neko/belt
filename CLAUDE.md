# belt — Agent Workflow Engine

`belt` は LLM エージェント向けの軽量ワークフローエンジン CLI スイート。YAML で宣言された決定論的 state machine を、冪等かつ拡張可能に駆動する。

## プロジェクト概要

- **言語**: Rust
- **MSRV**: 1.86.0
- **推奨開発版 / CI toolchain**: 1.94.1 以上 (CVE-2026-33055 / CVE-2026-33056 回避)
- **Edition**: 2024
- **アーキテクチャ**: Cargo workspace (**3 crates**)
- **設計思想**: Linux 哲学 "Do One Thing and Do It Well" / Tiny by Constraint
- **CLI 体系**: `belt lint` (静的検証) / `belt-agent init|next|verify|step|status` (runtime)
- **Linear tracking**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)

## Binary Separation (原則 8: Separation by Audience)

belt は **2 種類の利用者** (Developer / Agent) に最適化された**別バイナリ**を提供する。

| | Human CLI (`belt`) | Agent CLI (`belt-agent`) |
|---|---|---|
| 対象 | pipeline 作者 | LLM / CI/CD / script |
| 主コマンド | `lint` | `init`, `next`, `verify`, `step`, `status` |
| 重視 | authoring UX, fast feedback | determinism, scriptability, JSON output |
| 依存 | belt-core + clap + miette[fancy] | belt-core + clap + serde_json + miette[fancy] |
| I/O | 人向け (stderr diagnostics) | JSON stdout (パイプ前提) |
| Phase | **MVP (実装済み)** | **MVP (実装済み)** |

**ビルド時の crate 分離**で保証される (feature flag ではなく独立 crate / 独立 binary)。これにより:
- agent CLI (`belt-agent`) の supply chain risk が lint コードの影響を受けない
- `cargo build -p belt-agent` で agent runtime のみビルド
- `cargo tree -p <crate>` で各 audience の依存グラフを独立に audit 可能

## Crate 構成

```
belt/
├── crates/
│   ├── belt-core/    # 📦 library: model, parser, expander, engine, gate, lint
│   ├── belt/         # 🛠  Human CLI binary (belt lint)
│   └── belt-agent/   # 🤖 Agent runtime CLI binary (belt-agent init/next/verify/step/status)
```

| Crate | 役割 | Audience |
|-------|------|----------|
| `belt-core` | pure library。model types, YAML parser, sub-pipeline expander, state machine engine, gate executor, lint validator | 内部のみ、全 binary が依存 |
| `belt` | human 用 CLI binary。`lint` コマンドで belt-core の lint_pipeline を呼び出す | Developer (pipeline 作者) |
| `belt-agent` | agent 用 runtime CLI binary。`init/next/verify/step/status` で belt-core Engine + gate executor を駆動 | LLM, CI/CD, script |

### 依存関係フロー

```
belt-core (pure library)
     │
     ├──▶ belt        (human bin)  ─ lint のみ
     └──▶ belt-agent  (agent bin)  ─ engine + gate 実行
```

`belt` と `belt-agent` の間には依存関係がない。

## belt-core の 7 モジュール

| モジュール | 責務 |
|-----------|------|
| `model` | Pipeline, Phase, GateCheck (untagged enum), GateDefinition, SubPipeline, RunState, ExpandedPhase の serde 型 |
| `parser` | `parse_pipeline()`, `parse_gate_definition()`, `parse_sub_pipeline()` — YAML ファイルからモデルへのデシリアライズ |
| `expander` | `expand_pipeline()` — `uses:` 参照を flat namespace に展開。親の gate/regate/when 継承 |
| `engine` | `Engine` — init/step/verify_verdict/next_phase_info/status/latest_run_id。RunState 永続化、when: 評価 |
| `gate` | `execute_gate()`, `execute_gates()`, `all_passed()` — cmd, file_exists, git_clean, has_output の 4 実装 |
| `lint` | `lint_pipeline()` — 7 静的検証 (duplicate IDs, regate, args, description, uses references) + expansion 試行 |
| `error` | `BeltError` (thiserror + miette) — YamlParse, FileNotFound, InvalidPipeline, GateFailed, State, Io |

## YAML パイプライン構造

```yaml
name: my-pipeline
version: 1
args:
  smoke: { type: bool, default: false }
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "cargo build"
      - file_exists: "target/debug/belt"
    regate: [design]
  - id: review
    uses: ./pipelines/review-cycle.yml    # sub-pipeline 参照
    with: { skill: "/code-review" }
    when: "args.smoke"
  - id: done
    description: "Complete"
    confirm: true
    validate: ["All checks pass"]
```

### GateCheck 4 種類

| 種類 | YAML | 実行方法 |
|------|------|---------|
| `cmd` | `cmd: "cargo test"` | `sh -c` で実行、exit code で判定 |
| `file_exists` | `file_exists: "docs/*.md"` | glob パターンマッチ |
| `git_clean` | `git_clean: true` | `git status --porcelain` が空か |
| `has_output` | `has_output: true` | output_dir にファイルがあるか |

### Sub-Pipeline 展開

`uses:` で参照された sub-pipeline の phases は `{parent_id}/{sub_phase_id}` にリネームされる。最後の sub-phase が親の gate/regate/validate/config を継承する。親の `when:` は全 sub-phase に伝播する。

## Technology Stack

### Workspace `Cargo.toml` (SSOT)

`[workspace.dependencies]` で全依存を一元管理する。詳細は `Cargo.toml` を参照。

| crate | version | 備考 |
|-------|---------|------|
| `clap` | `4.6` | belt / belt-agent binary 用 |
| `serde` | `1.0.228` | derive feature |
| `serde-saphyr` | **`=0.0.23`** | 0.0.x 完全ピン必須 (pre-1.0) |
| `miette` | `7.6` | core は plain、binary は `fancy-no-backtrace` |
| `thiserror` | `2.0.18` | |
| `uuid` | `1.23` | v7 (time-ordered run ID) |
| `glob` | `0.3` | file_exists gate 用 |

### Workspace Lints (clippy + rust-lint)

`Cargo.toml` の `[workspace.lints.clippy]` と `[workspace.lints.rust]` で workspace 共通の lint policy を宣言的に強制する。各 crate は `[lints] workspace = true` で継承する。

- `clippy::all` + `clippy::pedantic` を warn
- `clippy::unwrap_used` / `expect_used` / `panic` を warn (tiny + fail-loud 方針)
- `unsafe_code` を forbid (rust-lint レベル)
- `unreachable_pub`, `missing_debug_implementations` を warn

CI + pre-commit で `cargo clippy --workspace -- -D warnings` を実行すれば、上記 warn は全て error として検出される。`rust-toolchain.toml` の `components = ["rustfmt", "clippy", "rust-src"]` で toolchain レベルでも clippy を強制。

## 依存管理ポリシー

### 追加制限 (agent CLI `belt-agent`)

- **TUI/GUI ライブラリは追加禁止** (ratatui, crossterm, tui-rs, iced, egui)
- **ネットワーク通信ライブラリは原則禁止** (reqwest, hyper, tonic)。必要時は spec に justification を記載
- **非同期ランタイムは原則禁止** (tokio, async-std, smol)。同期実行を基本とする
- **unsafe code は原則禁止** (`workspace.lints.rust` で `unsafe_code = "forbid"` 宣言済み)
- 新規依存追加時は `cargo audit` と `cargo deny` を通す

### 追加制限 (human CLI `belt`)

- **TUI/GUI ライブラリは追加禁止** (原則 8)
- **HTTP/async runtime は原則禁止** (authoring-time ツールでの spawn は避ける)

### バージョン指定ポリシー

| crate 種別 | 指定方式 | 理由 |
|-----------|---------|------|
| 0.x crates | `=X.Y.Z` 完全ピン | SemVer 前、patch でも breaking 可 |
| 1.x crates | `"X.Y"` caret 許容 | SemVer 準拠で安全 |

**完全ピン対象 (0.x)**: `serde-saphyr`
**caret 許容 (1.x+)**: `clap`, `serde`, `miette`, `thiserror`, `uuid`

### Cargo.lock

- **コミット対象**: belt は binary project のため `Cargo.lock` をコミットする
- 再現可能なビルドを保証

## Non-Goals (やらないこと)

- 複雑な assertion DSL / N 回実行統計 / 可視化ダッシュボード
- Web UI、リアルタイム協調編集
- `git2` / `gix` 等の C 依存ライブラリ (git CLI 直接呼び出しで統一)
- async runtime 必須化 (`tokio` / `async-std` を belt-core に持ち込まない)
- agent CLI binary (`belt-agent`) への TUI/GUI 依存または lint コードの混入
- `uses:` gate の実行時解決 (MVP では passthrough、将来実装)

## YAML Universe (Future Vision)

将来的に、pipeline / gate / sub-pipeline を Web で誰でも公開/取得できるエコシステムを構築する。リモート `uses:` は git clone でキャッシュ (HTTP lib 不要)。詳細は `docs/specs/2026-04-06-belt-redesign.md` 内の "YAML Universe (Future)" セクションを参照。

## Future Phases

- **TUI (`belt-tui`)**: ratatui-based interactive 監視・デバッグ UI (独立 crate として追加予定)
- **リモート `uses:`**: git-based sub-pipeline/gate 参照のキャッシュ解決
- **`fmt` コマンド**: pipeline YAML の自動フォーマット

### コミュニケーション

- 技術的に誤った意見には根拠を示して反論する
- 曖昧な指示に対しては推測で進めず、具体的に確認する
- 設計・レビュー・分析は分割確認せず、完全な形で一度に出力する

### 実装規律

- 初回の実装パスでバリデーション (範囲制約、境界値、型チェック) を含める
- コミット前に `cargo fmt --package <pkg>` / `cargo clippy --package <pkg> -- -D warnings` / `cargo test -p <pkg>` を実行 (変更 crate のみスコープ)
- workspace 全体チェックは CI 側で `cargo clippy --workspace -- -D warnings` として実行
- LSP (rust-analyzer) が利用可能な場合はシンボル調査・定義元・参照箇所の特定に Grep/Glob より優先
- GPG 署名エラー時は `-c commit.gpgsign=false` で再試行 OK

### CLI 命名

- Linux 哲学準拠、kubectl/helm/aws-cli 風のリソース+動詞で統一
- 動詞は意図が明確 (例: `step` (advance ではない))
- binary 名は audience を示す (`belt` = human, `belt-agent` = agent runtime)

### Verification Contract

- 非 trivial な変更 (3 ファイル以上 / backend / infrastructure) は独立 verification 必須
- `verified` / `PASS` 主張は実コマンドと出力を伴う
- 境界値・異常系・idempotency の adversarial probe を少なくとも 1 つ含める

## Known Risks (要監視)

- **`miette`**: 7.6.0 以降 1 年更新停滞 (2025-04-27 → 現在)。代替候補は `annotate-snippets`。maintainer 活動を定期確認
- **`serde-saphyr 0.0.x`**: SemVer 前。patch でも breaking 可能性、`cargo update` 前に changelog 確認必須。GateCheck の untagged enum デシリアライズ順序に依存するため特に注意
- **Rust 1.94.0 CVE (CVE-2026-33055/33056)**: Cargo 同梱 `tar` crate の脆弱性、1.94.1 で修正済み。toolchain は 1.94.1+ を使用 (`rust-toolchain.toml` で固定済み)
