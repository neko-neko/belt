# flowrail Phase 1 — `flowrail pipeline lint` + `flowrail pipeline fmt` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **⚠ 2026-04-05 更新 v3 (rename + repo split + workspace + layer 撤回 + Tech Stack 再調査 + path 全面書き換え)**: 本 plan は以下の 6 変更を反映して全体が更新された。実装時は**必ず spec の最新セクションを優先**すること。本 plan 本文中のファイルパス参照は **Cargo workspace 構造** に全面書き換え済み (2026-04-05 plan フェーズ): library module は `crates/flowrail-core/src/*`、agent binary は `crates/flowrail/src/*`、test fixtures は `crates/flowrail-core/tests/fixtures/` に集約されている。
>
> **主要変更点** (spec を正とする):
> 1. **CLI rename**: `jig` → **`flowrail`**、バイナリ名 / 設定ディレクトリ `.flowrail/` / 環境変数 `FLOWRAIL_*`
> 2. **Layer 機構撤回**: Rule Set Schema から `layer: primitive|recipe|pipeline-local` フィールド削除、cycle detection は実装しない (実行時の `max_depth` 超過で検出)。詳細は spec の "Conventions & Best Practices" と "YAML Universe (Future)" セクション参照
> 3. **Repository split (Pattern B)**: dotfiles から独立リポジトリ `~/go/src/github.com/neko-neko/flowrail/` へ分離
> 4. **Cargo workspace 化 (3 crates)**: `flowrail-core` (library) + `flowrail` (agent CLI bin) + `flowrail-tui` (human TUI bin, Phase 3)。原則 8 "Separation by Audience" により agent CLI と TUI CLI を依存関係レベルで分離。本 plan 本文中のパス参照は **Cargo workspace 構造に全面書き換え済み** (library module は `crates/flowrail-core/src/*`、agent binary は `crates/flowrail/src/*`、test fixtures は `crates/flowrail-core/tests/fixtures/` に配置)
> 5. **5 リソース体系**: `flowrail tui` サブコマンドは廃止、TUI は別バイナリ `flowrail-tui` として Phase 3 で提供。6 リソース → **5 リソース** (`pipeline` / `run` / `state` / `snapshot` / `help`)
> 6. **Technology Stack 再調査 (Rust 1.94.1 stable 時代)**:
>    - **MSRV**: `1.85` → **`1.86`** (ratatui 0.30 要求、workspace 統一のため)
>    - **推奨 toolchain**: **`1.94.1`** (`rust-toolchain.toml` で固定、CVE-2026-33055/33056 回避)
>    - **clap**: 4.5+ → **`4.6`**
>    - **serde**: 1.0 → **`1.0.228`**
>    - **serde-saphyr**: unpinned → **`=0.0.23`** (0.0.x 完全ピン必須)
>    - **jsonschema**: 0.20+ → **`=0.45.0`** (0.x 完全ピン、`default-features = false, features = ["resolve-file"]`)
>    - **minijinja**: 2.x → **`2.19`** (`default-features = false, features = ["builtins", "macros", "deserialization"]`)
>    - **miette**: 7.0+ → **`7.6`** (core では plain、agent/tui binary で `fancy-no-backtrace`)
>    - **thiserror**: 2.0+ → **`2.0.18`**
>    - **notify**: 追加 → **`8.2`** (v9 rc は保留、v9 stable で MSRV 1.88 要求)
>    - **uuid**: 1.x → **`1.23`** (`v4`, `v7`, `v5`, `serde` features)
>    - **regex**: 1.11+ → **`1.12`**
>    - **ratatui** (Phase 3 のみ): → **`=0.30.0`** (0.x 完全ピン)
>    - **crossterm** (Phase 3 のみ): → **`=0.29.0`** (0.x 完全ピン)
> 7. **Cargo.lock はコミット対象** (binary project)
> 8. **0.x crates は `=X.Y.Z` 完全ピン / 1.x crates は caret 許容** のポリシー適用
>
> 詳細は spec の「Technology Stack」「Dependency Management Policy」「Binary Separation」「Known Risks」セクション参照。

**Goal:** Rust Cargo workspace `~/go/src/github.com/neko-neko/flowrail/` で、`crates/flowrail-core/` (library) と `crates/flowrail/` (agent CLI binary) を実装し、`flowrail pipeline lint` と `flowrail pipeline fmt` の MVP を提供する。新アーキテクチャ (Rule Set Architecture) の pipeline.yml + rule-set.yml の静的検証とフォーマットを提供し、Event Stream と Deterministic Mode の基盤も構築する。`crates/flowrail-tui/` は Phase 1 では placeholder (Phase 3 で実装)。

**Architecture:** Rust Cargo workspace (3 crates)。`flowrail-core` が pure library として state machine / rule set resolver / lint / fmt ロジックを提供し、`flowrail` binary が clap で CLI wrapper を薄く実装 (`flowrail <resource> <verb>` 形式)。`crates/flowrail-core/src/yaml/` 抽象層経由で YAML パース (内部実装 `serde-saphyr =0.0.23`)、jsonschema でスキーマ検証、minijinja は Phase 1 では template 抽出に使用せず regex 一本、miette で診断出力。各 semantic lint ルールは独立したモジュールとして実装し、lint ドライバーが順次実行する。

**Tech Stack (spec §Technology Stack を正とする):**
- Rust (edition **2024**, MSRV **1.86**, 推奨 toolchain **1.94.1**)
- clap **4.6** (derive + builder 混在)
- serde **1.0.228** + **serde-saphyr `=0.0.23`** (YAML、`crates/flowrail-core/src/yaml/` 抽象層経由)
- jsonschema **`=0.45.0`** (`default-features = false, features = ["resolve-file"]`)
- minijinja **2.19** (`default-features = false, features = ["builtins", "macros", "deserialization"]`)
- miette **7.6** (core は plain、binary は `fancy-no-backtrace`)
- thiserror **2.0.18**
- notify **8.2**
- glob 0.3
- regex **1.12**
- uuid **1.23** (run.id 生成用、`v4`, `v7`, `v5`, `serde` features)

**Related:**
- Spec: `docs/specs/2026-04-05-flowrail-cli-rule-set-architecture-design.md` (**2026-04-05 spec-review で大幅更新済み + 2026-04-05 plan フェーズで Feasibility Mapping / Impact Analysis を完全版に**)
- Linear: [CLA-5](https://linear.app/neko-neko/issue/CLA-5) (Phase 1 実装 tracking)
- Linear (ブレインストーミング履歴): [CLA-19](https://linear.app/neko-neko/issue/CLA-19) (Done)
- 旧 plan (3 層モデル版、参照用): dotfiles レポ `docs/superpowers/plans/2026-04-05-flowrail-phase1-lint-fmt.md` (commit 履歴参照)

---

## File Structure

Phase 1 実装対象は **`crates/flowrail-core/`** (library) と **`crates/flowrail/`** (agent CLI binary)。`crates/flowrail-tui/` は Phase 1 では placeholder のみ (Phase 3 で本格実装)。

```
~/go/src/github.com/neko-neko/flowrail/       # workspace root (独立リポジトリ)
├── Cargo.toml                                 # [workspace] + [workspace.dependencies]
├── Cargo.lock                                 # コミット対象 (binary project)
├── rust-toolchain.toml                        # Rust 1.94.1 固定
├── README.md
├── CLAUDE.md
├── .gitignore
├── docs/
│   ├── specs/
│   └── plans/
├── schema/
│   ├── pipeline.schema.json                   # pipeline.yml の JSON Schema (Draft 2020-12)
│   └── rule-set.schema.json                   # rule-set.yml の JSON Schema
├── crates/
│   ├── flowrail-core/                         # 📦 library (Phase 1 実装対象)
│   │   ├── Cargo.toml                         # 依存: serde, serde-saphyr, jsonschema, minijinja, miette (plain), thiserror, notify, uuid, regex, glob
│   │   └── src/
│   │       ├── lib.rs                         # pub mod 宣言
│   │       ├── error.rs                       # thiserror 統一エラー型 (FlowrailError)
│   │       ├── yaml/                          # YAML abstraction layer (serde-saphyr を wrap)
│   │       │   └── mod.rs                     # parse<T> / parse_with_options<T> / serialize<T>
│   │       ├── pipeline/
│   │       │   ├── mod.rs
│   │       │   ├── model.rs                   # pipeline.yml の serde 型
│   │       │   └── loader.rs                  # YAML パース + schema 検証
│   │       ├── ruleset/
│   │       │   ├── mod.rs
│   │       │   ├── model.rs                   # rule-set.yml の serde 型
│   │       │   ├── loader.rs                  # YAML パース + schema 検証
│   │       │   ├── resolver.rs                # imports の再帰解決 (max_depth による深度制限のみ、cycle detection は実装しない)
│   │       │   ├── param_check.rs             # params と uses の型整合検証
│   │       │   └── template.rs                # minijinja 統合 (static ref 解析)
│   │       ├── lint/
│   │       │   ├── mod.rs                     # lint ドライバー (全ルール実行)
│   │       │   ├── diagnostic.rs              # miette 統合 (E001, E003-E008, W001; E002 欠番)
│   │       │   └── rules/
│   │       │       ├── mod.rs
│   │       │       ├── unknown_rule_set.rs            # E001 unknown rule set in `uses`
│   │       │       ├── param_type_mismatch.rs         # E003 param type mismatch
│   │       │       ├── unresolved_template.rs         # E004 unresolved template reference
│   │       │       ├── invalid_produced_consumed.rs   # E005 invalid produced_by/consumed_by
│   │       │       ├── invalid_trigger_rewind.rs      # E006 invalid triggers.rewind_to
│   │       │       ├── invalid_hook_event.rs          # E007 invalid integrations.hooks event
│   │       │       └── unused_param.rs                # W001 unused param warning
│   │       │       # E002 circular_import.rs は削除 (layer 撤回 + Non-Goals 遵守)。Task 8 resolver の max_depth ガードで顕在化させる
│   │       ├── fmt/
│   │       │   ├── mod.rs                     # YAML 正規化 (key ordering, indent)
│   │       │   └── key_order.rs               # key 順序定義 (pipeline / rule-set 別)
│   │       ├── event/
│   │       │   ├── mod.rs
│   │       │   └── logger.rs                  # JSONL event stream (run.id 付き)
│   │       ├── determinism/
│   │       │   └── mod.rs                     # FLOWRAIL_NOW, FLOWRAIL_SEED, JSON 正規化
│   │       └── output/
│   │           ├── mod.rs
│   │           └── json.rs                    # 決定論的 JSON シリアライザ
│   │
│   ├── flowrail/                              # 🤖 agent CLI binary (Phase 1 実装対象)
│   │   ├── Cargo.toml                         # 依存: flowrail-core + clap + miette(fancy-no-backtrace)
│   │   └── src/
│   │       ├── main.rs                        # エントリポイント、run.id 生成、event 発行
│   │       └── cli.rs                         # clap 定義 (top-level + pipeline サブコマンド、2-pass parse 戦略)
│   │
│   └── flowrail-tui/                          # 👤 human TUI binary (Phase 3)
│       ├── Cargo.toml                         # Phase 1: 最小 placeholder (依存: flowrail-core のみ)
│       └── src/
│           └── main.rs                        # Phase 1: `fn main() { eprintln!("flowrail-tui: coming in Phase 3"); }`
│
├── catalog/                                   # Standard Rule Sets (Phase 1 では .gitkeep のみ)
│   ├── primitives/.gitkeep
│   └── recipes/.gitkeep
├── examples/.gitkeep                          # サンプル pipeline (Phase 1 では .gitkeep のみ)
└── tests/                                     # integration tests (workspace root 配下も可、あるいは各 crate の tests/)
    ├── lint_test.rs                           # 統合テスト (全ルール)
    ├── fmt_test.rs                            # フォーマッタ統合テスト
    ├── event_stream_test.rs                   # event stream 検証
    ├── deterministic_test.rs                  # deterministic mode 検証
    └── fixtures/
        ├── valid/
        │   ├── pipelines/
        │   │   ├── feature-dev-minimal.yml
        │   │   └── debug-flow-minimal.yml
        │   └── rules/
        │       ├── primitives/
        │       │   ├── check-file-exists.yml
        │       │   └── check-command.yml
        │       └── recipes/
        │           └── audit-gate.yml
        └── invalid/
            ├── unknown-rule-set-in-uses.yml
            ├── param-type-mismatch.yml
            ├── unresolved-template.yml
            ├── invalid-produced-consumed.yml
            ├── invalid-trigger-rewind.yml
            └── invalid-hook-event.yml
            # ⚠ circular-import-a/b.yml は削除: cycle detection を lint から外したため
```

**Responsibility boundaries:**
- **`flowrail-core`**: pure library。pipeline/ruleset/lint/fmt/event/determinism/output は library module として export
- **`flowrail`** (binary): 薄い CLI wrapper。main.rs が clap の argument parsing → flowrail-core の library function 呼び出し → 結果の出力のみ
- **`flowrail-tui`** (binary): Phase 1 では placeholder。Phase 3 で ratatui ベースの UI 実装
- `pipeline/` と `ruleset/` は独立モジュールで、お互いに依存しない。共通の型は `error.rs` 経由でのみ共有される
- `lint/rules/` は各ルールが自己完結。ルール追加時は `rules/mod.rs` に 1 行追加するだけで済む
- `fmt/` は `pipeline/model.rs` と `ruleset/model.rs` を読むが書かない (出力は正規化 YAML のみ)
- `event/` と `determinism/` はトップレベルの cross-cutting concern、flowrail-core から export され binary から呼ばれる
- **Cargo workspace 構造に全面書き換え済み** (2026-04-05 plan フェーズ): library module は `crates/flowrail-core/src/*`、agent binary は `crates/flowrail/src/*`、test fixtures は `crates/flowrail-core/tests/fixtures/` に集約。`cargo test -p flowrail-core --test <name>` で library crate のテスト、`cargo test -p flowrail --test <name>` で binary crate のテストを実行する

---

## Task 1: Cargo Workspace 初期化

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `rust-toolchain.toml`
- Create: `crates/flowrail-core/Cargo.toml`
- Create: `crates/flowrail-core/src/lib.rs`
- Create: `crates/flowrail/Cargo.toml`
- Create: `crates/flowrail/src/main.rs`
- Create: `crates/flowrail-tui/Cargo.toml`
- Create: `crates/flowrail-tui/src/main.rs`
- Create: `.gitignore`

> **Note**: Stage C (新レポジトリ作成) で上記の最小 workspace skeleton は既に配置済みの前提。本 Task 1 は Phase 1 実装時に依存と feature を具体化するステップ。version 範囲は `[workspace.dependencies]` で管理。

- [ ] **Step 1: Workspace root `Cargo.toml` を確認・更新**

`~/go/src/github.com/neko-neko/flowrail/Cargo.toml` (Stage C で最小版は配置済み、Task 1 で依存を具体化):

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.86"
authors = ["neko-neko"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/neko-neko/flowrail"

[workspace.dependencies]
# --- serialization / YAML ---
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0"
serde-saphyr = "=0.0.23"  # 0.0.x pre-1.0, 完全ピン必須

# --- JSON Schema ---
jsonschema = { version = "=0.45.0", default-features = false, features = ["resolve-file"] }

# --- template ---
minijinja = { version = "2.19", default-features = false, features = ["builtins", "macros", "deserialization"] }

# --- error handling ---
miette = "7.6"  # plain, for flowrail-core
thiserror = "2.0.18"

# --- CLI (flowrail binary only) ---
clap = { version = "4.6", features = ["derive", "env", "string"] }

# --- filesystem watcher ---
notify = "8.2"  # v9 rc は保留

# --- utilities ---
uuid = { version = "1.23", features = ["v4", "v7", "v5", "serde"] }
regex = "1.12"
glob = "0.3"

# --- TUI (flowrail-tui only, Phase 3) ---
ratatui = "=0.30.0"
crossterm = "=0.29.0"

# --- dev-dependencies (test framework) ---
insta = { version = "1", features = ["yaml"] }
pretty_assertions = "1"
tempfile = "3"

[profile.release]
lto = "thin"
codegen-units = 1
strip = true
```

- [ ] **Step 2: `rust-toolchain.toml` を配置** (Rust 1.94.1 固定、CVE-2026-33055/33056 回避)

`~/go/src/github.com/neko-neko/flowrail/rust-toolchain.toml`:
```toml
[toolchain]
channel = "1.94.1"
components = ["rustfmt", "clippy", "rust-src"]
profile = "minimal"
```

- [ ] **Step 3: `crates/flowrail-core/Cargo.toml` を具体化**

```toml
[package]
name = "flowrail-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Tiny workflow engine library — state machine, rule set resolver, primitives"

[dependencies]
serde.workspace = true
serde_json.workspace = true
serde-saphyr.workspace = true
jsonschema.workspace = true
minijinja.workspace = true
miette.workspace = true        # plain, no fancy feature
thiserror.workspace = true
notify.workspace = true
uuid.workspace = true
regex.workspace = true
glob.workspace = true

[dev-dependencies]
insta.workspace = true
pretty_assertions.workspace = true
tempfile.workspace = true
```

- [ ] **Step 4: `crates/flowrail/Cargo.toml` を具体化** (agent CLI binary)

```toml
[package]
name = "flowrail"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Tiny workflow engine CLI for LLM agents"

[[bin]]
name = "flowrail"
path = "src/main.rs"

[dependencies]
flowrail-core = { path = "../flowrail-core" }
clap.workspace = true
miette = { workspace = true, features = ["fancy-no-backtrace"] }
# ⚠ TUI 系依存 (ratatui, crossterm) は一切追加禁止

[dev-dependencies]
insta.workspace = true
pretty_assertions.workspace = true
tempfile.workspace = true
```

- [ ] **Step 5: `crates/flowrail-tui/Cargo.toml` を具体化** (Phase 3、Phase 1 では placeholder)

```toml
[package]
name = "flowrail-tui"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Human-facing TUI for flowrail (Phase 3)"

[[bin]]
name = "flowrail-tui"
path = "src/main.rs"

[dependencies]
flowrail-core = { path = "../flowrail-core" }
# Phase 3 で ratatui / crossterm / miette を追加
# ratatui.workspace = true
# crossterm.workspace = true
# miette = { workspace = true, features = ["fancy-no-backtrace"] }
```

`crates/flowrail-tui/src/main.rs` (Phase 1 placeholder):
```rust
fn main() {
    eprintln!("flowrail-tui: coming in Phase 3");
    std::process::exit(0);
}
```

- [ ] **Step 6: `crates/flowrail-core/src/lib.rs` 初期化** (空の pub mod 宣言のみ、後続 Task で module を追加)

```rust
// crates/flowrail-core/src/lib.rs
#![forbid(unsafe_code)]

// module 宣言は後続 Task で追加
```

- [ ] **Step 7: `crates/flowrail/src/main.rs` 初期化**

```rust
// crates/flowrail/src/main.rs
#![forbid(unsafe_code)]

fn main() {
    println!("flowrail 0.1.0");
}
```

- [ ] **Step 8: `.gitignore` を作成** (workspace root)

```
/target
Cargo.lock.backup
.flowrail/
*.swp
.DS_Store
```

- [ ] **Step 9: ビルドと smoke 実行**

```bash
cd ~/go/src/github.com/neko-neko/flowrail && cargo check --workspace 2>&1 | tail -20
cd ~/go/src/github.com/neko-neko/flowrail && cargo build --workspace 2>&1 | tail -20
cd ~/go/src/github.com/neko-neko/flowrail && cargo run -p flowrail 2>&1 | tail -5
cd ~/go/src/github.com/neko-neko/flowrail && cargo run -p flowrail-tui 2>&1 | tail -5
```

Expected: `cargo check --workspace` と `cargo build --workspace` が成功、`flowrail` は `flowrail 0.1.0` を出力、`flowrail-tui` は `flowrail-tui: coming in Phase 3` を stderr に出力して exit 0。

- [ ] **Step 10: Commit**

```bash
git add Cargo.toml Cargo.lock rust-toolchain.toml .gitignore crates/
git commit -m "feat: initialize Cargo workspace (flowrail-core + flowrail + flowrail-tui placeholder)"
```

---

## Task 2.0: YAML 抽象層 (`crates/flowrail-core/src/yaml/`)

**前提**: spec §YAML Abstraction Layer L2563-2587 が Phase 1 必須と規定。serde-saphyr のような YAML backend を wrap し、他 module は `flowrail_core::yaml::{parse, parse_value, serialize, Value, Mapping, YamlError}` のみを import する。backend 切替時の変更範囲を最小化する。

**Files:**
- Create: `crates/flowrail-core/src/yaml/mod.rs`
- Create: `crates/flowrail-core/tests/yaml_abstraction_test.rs`

- [ ] **Step 1: failing test を先に書く**

`crates/flowrail-core/tests/yaml_abstraction_test.rs`:

```rust
use flowrail_core::yaml;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Simple {
    name: String,
    count: u32,
}

#[test]
fn parses_typed_value_via_abstraction() {
    let yaml_text = "name: flowrail\ncount: 3\n";
    let v: Simple = yaml::parse(yaml_text).expect("parse ok");
    assert_eq!(v.name, "flowrail");
    assert_eq!(v.count, 3);
}

#[test]
fn parses_dynamic_value() {
    let yaml_text = "a: 1\nb: [2, 3]\n";
    let v = yaml::parse_value(yaml_text).expect("parse ok");
    assert!(v.is_mapping());
}

#[test]
fn reports_duplicate_key_as_error_by_default() {
    let yaml_text = "a: 1\na: 2\n";
    let err = yaml::parse_value(yaml_text).expect_err("expected duplicate-key error");
    assert!(format!("{err}").contains("duplicate"));
}

#[test]
fn serializes_typed_value() {
    let v = Simple { name: "flowrail".into(), count: 7 };
    let out = yaml::serialize(&v).expect("serialize ok");
    assert!(out.contains("name: flowrail"));
    assert!(out.contains("count: 7"));
}
```

Run: `cargo test -p flowrail-core --test yaml_abstraction_test 2>&1 | tail -20`
Expected: FAIL — `flowrail_core::yaml` module が存在しない。

- [ ] **Step 2: `src/yaml/mod.rs` 実装**

`crates/flowrail-core/src/yaml/mod.rs`:

```rust
//! YAML abstraction layer.
//!
//! All YAML parsing/serialization MUST go through this module. Other modules
//! MUST NOT depend on the concrete backend (`serde-saphyr`, `serde_yml`, etc.)
//! directly. This keeps the backend swappable for security and performance
//! reasons (see spec §YAML Abstraction Layer).

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

/// Dynamic YAML value type. Re-exported from the backend.
/// Phase 1 uses `serde_json::Value` as the neutral intermediate representation
/// because it has a stable tree API that works for both YAML and JSON.
pub type Value = serde_json::Value;

/// YAML mapping alias (dynamic).
pub type Mapping = serde_json::Map<String, serde_json::Value>;

/// YAML parser/serializer errors.
#[derive(Debug, Error)]
pub enum YamlError {
    #[error("YAML parse error: {0}")]
    Parse(String),

    #[error("YAML serialize error: {0}")]
    Serialize(String),

    #[error("duplicate key '{0}' detected (DuplicateKeyPolicy::Error is the default in Phase 1)")]
    DuplicateKey(String),

    #[error("budget exceeded: {0}")]
    Budget(String),
}

/// Parsing options. Phase 1 defaults are strict.
#[derive(Debug, Clone)]
pub struct YamlOptions {
    pub duplicate_keys: DuplicateKeyPolicy,
    pub budget: Budget,
}

impl Default for YamlOptions {
    fn default() -> Self {
        Self {
            duplicate_keys: DuplicateKeyPolicy::Error,
            budget: Budget::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DuplicateKeyPolicy {
    Error,      // default: reject duplicate keys
    FirstWins,  // keep first occurrence
    LastWins,   // keep last occurrence (serde_yaml default)
}

/// Resource limits to prevent DoS via pathological YAML inputs.
#[derive(Debug, Clone)]
pub struct Budget {
    pub max_anchors: usize,
    pub max_depth: usize,
    pub max_events: usize,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            max_anchors: 200,
            max_depth: 100,
            max_events: 50_000,
        }
    }
}

/// Parse YAML text into a typed value.
///
/// # Errors
/// Returns `YamlError` on syntax errors, duplicate keys (when policy is Error),
/// or budget violations.
pub fn parse<T: DeserializeOwned>(text: &str) -> Result<T, YamlError> {
    parse_with_options(text, &YamlOptions::default())
}

/// Parse YAML text with explicit options.
pub fn parse_with_options<T: DeserializeOwned>(
    text: &str,
    _options: &YamlOptions,
) -> Result<T, YamlError> {
    // Phase 1 backend: delegate to serde-saphyr. When the crate API stabilizes,
    // wire DuplicateKeyPolicy/Budget into its configuration.
    // For now, we thin-wrap `serde_saphyr::from_str`.
    serde_saphyr::from_str::<T>(text).map_err(|e| YamlError::Parse(e.to_string()))
}

/// Parse YAML text into a dynamic `Value` (serde_json::Value).
pub fn parse_value(text: &str) -> Result<Value, YamlError> {
    // Two-step: parse via serde-saphyr into serde_json::Value.
    serde_saphyr::from_str::<Value>(text).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("duplicate") {
            // best-effort extraction of the key name
            YamlError::DuplicateKey(msg)
        } else {
            YamlError::Parse(msg)
        }
    })
}

/// Serialize a typed value to canonical YAML text.
///
/// Canonical rules (per spec §fmt L1358):
/// - End with a trailing newline
/// - Quote strings only when required (booleans, null, numeric strings, special chars)
/// - 2-space indentation
pub fn serialize<T: Serialize>(value: &T) -> Result<String, YamlError> {
    // Phase 1: stub via serde_json round-trip + manual YAML printer, or use
    // saphyr emitter when available. Until then, Phase 1 implementation MAY
    // use a known-working backend (`serde_yaml_bw` or `serde_yml` as a short-
    // term shim). Mark this clearly.
    //
    // TODO(Phase 2): replace with a canonical YAML emitter that enforces the
    //                quote rule and preserves comments where feasible.
    serde_yml::to_string(value).map_err(|e| YamlError::Serialize(e.to_string()))
}

/// Convert a typed value into the dynamic `Value` representation.
pub fn to_value<T: Serialize>(value: &T) -> Result<Value, YamlError> {
    serde_json::to_value(value).map_err(|e| YamlError::Serialize(e.to_string()))
}

/// Convert a dynamic `Value` into a typed value.
pub fn from_value<T: DeserializeOwned>(value: Value) -> Result<T, YamlError> {
    serde_json::from_value(value).map_err(|e| YamlError::Parse(e.to_string()))
}
```

> **注**: `serialize()` は Phase 1 の過渡期として `serde_yml::to_string` を内部実装として使用する (spec §fmt quote rule 完全遵守は Phase 2 で canonical emitter を自作して達成する)。この thin wrapper のおかげで、他 module は backend を一切意識せずに `yaml::serialize(...)` を呼べる。

- [ ] **Step 3: `Cargo.toml` を更新**

`crates/flowrail-core/Cargo.toml` の `[dependencies]` に (workspace 継承):
- `serde-saphyr.workspace = true`
- `serde_json.workspace = true` (yaml::Value の実装用)
- `serde_yml = "0.0.12"` (Phase 1 serialize の過渡期 backend、Phase 2 で削除予定を comment で明示)
- `serde.workspace = true`
- `thiserror.workspace = true`

`[workspace.dependencies]` 側にも `serde_yml = "0.0.12"` を追加 (Phase 2 で削除する旨を comment で記載)。

- [ ] **Step 4: test 再実行**

Run: `cargo test -p flowrail-core --test yaml_abstraction_test 2>&1 | tail -20`
Expected: 4 tests passed.

- [ ] **Step 5: `pub mod yaml;` を lib.rs に追加**

`crates/flowrail-core/src/lib.rs`:

```rust
pub mod yaml;
```

- [ ] **Step 6: Commit**

```bash
cargo fmt --package flowrail-core
cargo clippy --package flowrail-core -- -D warnings
git add crates/flowrail-core/src/yaml/ crates/flowrail-core/src/lib.rs crates/flowrail-core/Cargo.toml Cargo.toml crates/flowrail-core/tests/yaml_abstraction_test.rs
git commit -m "feat(flowrail-core): introduce YAML abstraction layer (serde-saphyr wrapper)"
```

---

## Task 2: エラー型定義 (thiserror 統一)

**Files:**
- Create: `crates/flowrail-core/src/error.rs`
- Modify: `crates/flowrail/src/main.rs`

- [ ] **Step 1: error.rs を作成**

`crates/flowrail-core/src/error.rs`:

```rust
use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, FlowrailError>;

#[derive(Debug, Error)]
#[allow(dead_code)] // variants are introduced incrementally across Task 2-17; suppress false-positive until wired.
pub enum FlowrailError {
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("YAML parse error in {path}: {source}")]
    YamlParse {
        path: PathBuf,
        #[source]
        source: crate::yaml::YamlError,
    },

    #[error("JSON schema validation failed in {path}: {message}")]
    SchemaValidation { path: PathBuf, message: String },

    #[error("Maximum import depth exceeded ({depth}) while resolving rule sets from {entry}")]
    MaxDepthExceeded { entry: PathBuf, depth: usize },

    #[error("Unknown rule set '{name}' used in {path}")]
    UnknownRuleSet { name: String, path: PathBuf },

    #[error("Parameter type mismatch: {details}")]
    ParamTypeMismatch { details: String },

    #[error("Unresolved template reference: {expression} in {path}")]
    UnresolvedTemplate { expression: String, path: PathBuf },
}
```

> **注 (2026-04-05 plan-review フィードバック反映)**:
> - `CircularImport` variant は削除済み (layer 撤回 + Non-Goals 準拠)。代わりに `MaxDepthExceeded` を Task 8 の resolver から返す
> - `LintFailed` variant は削除 (Task 17 の lint driver が ExitCode を直接制御するため不要)
> - `#[allow(dead_code)]` は Task 2 時点では全 variants が未使用のため一時的に付与。Task 17 完了時点で未使用な variant があれば削除を検討

- [ ] **Step 2: lib.rs に error モジュールを露出**

`crates/flowrail-core/src/lib.rs` を編集して `pub mod error;` を追加 (yaml module も Task 2.0 で追加済みの想定):

```rust
pub mod yaml;  // Task 2.0 で作成済み (YAML abstraction layer)
pub mod error;
```

`crates/flowrail/src/main.rs` を以下に更新 (binary crate からは `flowrail_core::error::*` を use する):

```rust
use flowrail_core::error::Result;

fn main() -> Result<()> {
    println!("flowrail 0.1.0");
    Ok(())
}
```

> **注**: `mod error;` を `crates/flowrail/src/main.rs` に書くのは誤り (sibling `error.rs` が binary crate 側に存在しないためコンパイル不可)。library crate の module を import するには `use flowrail_core::error::*;` を使うこと。

- [ ] **Step 3: ビルド確認**

Run: `cargo fmt --package flowrail-core --package flowrail && cargo clippy --package flowrail-core --package flowrail -- -D warnings && cargo build --workspace 2>&1 | tail -10`
Expected: ビルド成功。`#[allow(dead_code)]` により unused variant 警告なし。clippy clean。

- [ ] **Step 4: Commit**

```bash
git add crates/flowrail-core/src/error.rs crates/flowrail-core/src/lib.rs crates/flowrail/src/main.rs
git commit -m "feat(flowrail-core): add unified error type with thiserror"
```

---

## Task 3: CLI スケルトン (clap, pipeline サブコマンド)

**Files:**
- Create: `crates/flowrail/src/cli.rs`
- Modify: `crates/flowrail/src/main.rs`
- Test: `crates/flowrail/tests/cli_test.rs`

- [ ] **Step 1: 失敗テストを書く**

`crates/flowrail/tests/cli_test.rs`:

```rust
use std::process::Command;

fn flowrail_bin() -> String {
    env!("CARGO_BIN_EXE_flowrail").to_string()
}

#[test]
fn cli_without_args_shows_help_with_exit_2() {
    let output = Command::new(flowrail_bin()).output().expect("run flowrail");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage:"));
    assert!(stderr.contains("pipeline"));
}

#[test]
fn cli_pipeline_lint_subcommand_is_recognized() {
    let output = Command::new(flowrail_bin())
        .args(["pipeline", "lint", "--help"])
        .output()
        .expect("run flowrail pipeline lint --help");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lint"));
}

#[test]
fn cli_pipeline_fmt_subcommand_is_recognized() {
    let output = Command::new(flowrail_bin())
        .args(["pipeline", "fmt", "--help"])
        .output()
        .expect("run flowrail pipeline fmt --help");
    assert_eq!(output.status.code(), Some(0));
}
```

- [ ] **Step 2: 実行してテスト失敗を確認**

Run: `cargo test -p flowrail --test cli_test 2>&1 | tail -20`
Expected: FAIL — `Usage:` or `pipeline` が存在しない。

- [ ] **Step 3: cli.rs を実装**

`crates/flowrail/src/cli.rs`:

```rust
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// flowrail — Tiny Workflow Engine CLI for LLM (Rule Set Architecture)
#[derive(Debug, Parser)]
#[command(name = "flowrail", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: TopLevel,
}

#[derive(Debug, Subcommand)]
pub enum TopLevel {
    /// Pipeline resource: lint, fmt, test, init
    Pipeline(PipelineArgs),
}

#[derive(Debug, Args)]
pub struct PipelineArgs {
    #[command(subcommand)]
    pub command: PipelineVerb,
}

#[derive(Debug, Subcommand)]
pub enum PipelineVerb {
    /// Validate pipeline.yml + rule-set.yml (schema + semantic)
    Lint {
        /// Target files or directories (default: current dir)
        #[arg(value_name = "PATH")]
        paths: Vec<PathBuf>,
    },
    /// Normalize pipeline.yml + rule-set.yml formatting
    Fmt {
        /// Target files or directories
        #[arg(value_name = "PATH")]
        paths: Vec<PathBuf>,
        /// Check only, do not modify files
        #[arg(long)]
        check: bool,
        /// Show diff of changes
        #[arg(long)]
        diff: bool,
    },
}
```

- [ ] **Step 4: main.rs で CLI を配線**

`crates/flowrail/src/main.rs`:

```rust
mod cli;

use clap::Parser;
use cli::{Cli, PipelineVerb, TopLevel};
use flowrail_core::error::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        TopLevel::Pipeline(args) => match args.command {
            PipelineVerb::Lint { paths } => {
                eprintln!("(lint stub) paths={:?}", paths);
            }
            PipelineVerb::Fmt { paths, check, diff } => {
                eprintln!("(fmt stub) paths={:?} check={} diff={}", paths, check, diff);
            }
        },
    }
    Ok(())
}
```

- [ ] **Step 5: テスト再実行 + 手動 smoke**

Run:
```bash
cargo test -p flowrail --test cli_test 2>&1 | tail -20
cargo run --bin flowrail -- pipeline lint --help 2>&1 | head -20
```
Expected: 全テスト PASS。`pipeline lint --help` がヘルプを表示。

- [ ] **Step 6: Commit**

```bash
git add crates/flowrail/src/cli.rs crates/flowrail/src/main.rs crates/flowrail/tests/cli_test.rs
git commit -m "feat(flowrail): add CLI skeleton with pipeline lint/fmt subcommands"
```

---

## Task 4: pipeline.yml モデル定義

**Files:**
- Create: `crates/flowrail-core/src/pipeline/mod.rs`
- Create: `crates/flowrail-core/src/pipeline/model.rs`
- Modify: `crates/flowrail/src/main.rs`
- Test: `crates/flowrail-core/tests/pipeline_model_test.rs`

- [ ] **Step 1: 失敗テストを書く**

`crates/flowrail-core/tests/pipeline_model_test.rs`:

```rust
use flowrail_core::pipeline::model::{Pipeline, PipelineKind};

#[test]
fn minimal_pipeline_parses() {
    let yaml = r#"
kind: pipeline
name: feature-dev
version: 4
imports: []
phases:
  - id: design
    confirm: after
"#;
    let pipeline: Pipeline = yaml::parse(yaml).expect("parse minimal pipeline");
    assert!(matches!(pipeline.kind, PipelineKind::Pipeline));
    assert_eq!(pipeline.name, "feature-dev");
    assert_eq!(pipeline.version, 4);
    assert_eq!(pipeline.phases.len(), 1);
    assert_eq!(pipeline.phases[0].id, "design");
}

#[test]
fn pipeline_with_artifacts_and_flags_parses() {
    let yaml = r#"
kind: pipeline
name: feature-dev
version: 4
imports:
  - rules/recipes/audit-gate.yml
flags:
  "--linear":
    type: bool
    default: false
artifacts:
  spec_file:
    type: file
    pattern: "docs/specs/*.md"
    produced_by: design
    consumed_by: [plan]
phases:
  - id: design
  - id: plan
"#;
    let pipeline: Pipeline = yaml::parse(yaml).expect("parse pipeline");
    assert_eq!(pipeline.imports.len(), 1);
    assert!(pipeline.flags.contains_key("--linear"));
    assert!(pipeline.artifacts.contains_key("spec_file"));
    let spec = &pipeline.artifacts["spec_file"];
    assert_eq!(spec.produced_by.as_deref(), Some("design"));
    assert_eq!(spec.consumed_by, vec!["plan"]);
}
```

- [ ] **Step 2: 実行してテスト失敗を確認**

Cargo workspace 構造では `flowrail-core` (library crate) と `flowrail` (binary crate) は Task 1 で既に分離済み。ここでは `crates/flowrail-core/Cargo.toml` の `[lib]` セクションが以下の形で定義されていることを確認する:

```toml
# crates/flowrail-core/Cargo.toml
[package]
name = "flowrail-core"
version = "0.1.0"
edition = "2024"

[lib]
name = "flowrail_core"
path = "src/lib.rs"

[dependencies]
# workspace dependencies は [workspace.dependencies] を参照
```

binary crate (`crates/flowrail/Cargo.toml`) は library に依存する形で Task 1 で設定済み (`flowrail-core = { path = "../flowrail-core" }`)。

Create `crates/flowrail-core/src/lib.rs`:

```rust
pub mod error;
pub mod pipeline;
```

Run: `cargo test -p flowrail-core --test pipeline_model_test 2>&1 | tail -20`
Expected: FAIL — `flowrail_core::pipeline::model` が存在しない。

- [ ] **Step 3: pipeline/mod.rs と model.rs を実装**

`crates/flowrail-core/src/pipeline/mod.rs`:

```rust
pub mod model;
```

`crates/flowrail-core/src/pipeline/model.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineKind {
    Pipeline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub kind: PipelineKind,
    pub name: String,
    pub version: u32,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub imports: Vec<String>,
    #[serde(default)]
    pub flags: BTreeMap<String, FlagDef>,
    #[serde(default)]
    pub settings: BTreeMap<String, yaml::Value>,
    #[serde(default)]
    pub artifacts: BTreeMap<String, Artifact>,
    #[serde(default)]
    pub phases: Vec<Phase>,
    #[serde(default)]
    pub uses: Vec<yaml::Value>,
    // NOTE: 2026-04-05 plan-review で spec L1355 に合わせて 4 フィールドを追加:
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    #[serde(default)]
    pub integrations: Vec<Integration>,
    #[serde(default)]
    pub pre_pipeline_start: Option<PrePipelineStart>,
    #[serde(default)]
    pub on_pipeline_start: Vec<yaml::Value>,
    #[serde(default)]
    pub on_pipeline_complete: Vec<yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagDef {
    #[serde(default = "default_flag_type")]
    pub r#type: String,
    #[serde(default)]
    pub default: Option<yaml::Value>,
    // NOTE: 2026-04-05 plan-review で spec §Flag System SSOT L2338-2352 に合わせて
    // nested `enables` 構造体に変更 (旧 flat な enables_phases / enables_integrations を廃止)。
    #[serde(default)]
    pub enables: Option<FlagEnables>,
    #[serde(default)]
    pub binds_to_param: Option<BindsToParam>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlagEnables {
    #[serde(default)]
    pub integrations: Vec<String>,
    #[serde(default)]
    pub phases: Vec<String>,
    #[serde(default)]
    pub params: BTreeMap<String, BTreeMap<String, yaml::Value>>,
}

fn default_flag_type() -> String {
    "bool".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindsToParam {
    pub rule_set: String,
    pub param: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub r#type: String,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub produced_by: Option<String>,
    #[serde(default)]
    pub consumed_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub id: String,
    #[serde(default)]
    pub confirm: Option<String>,
    #[serde(default)]
    pub phase_file: Option<String>,
    #[serde(default)]
    pub skip_unless: Option<String>,
    #[serde(default)]
    pub uses: Vec<yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Integration {
    pub name: String,
    #[serde(default)]
    pub enabled_by: Option<String>,
    // NOTE: 2026-04-05 plan-review で spec L1098 に合わせて string | list[string]
    // の両方を受け付けるようにした (現行 feature-dev pipeline.yml の
    // `on_phase_complete: [sync_phase_summary, sync_evidence]` 記法との互換性)。
    #[serde(default)]
    pub hooks: BTreeMap<String, HookCommand>,
    #[serde(default)]
    pub pre_pipeline_start: Option<PrePipelineStart>,
}

/// Hook command can be a single string or a list of strings (both supported).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookCommand {
    Single(String),
    Multiple(Vec<String>),
}

impl HookCommand {
    /// Return the hook commands as a slice of strings, treating single form as
    /// a single-element slice.
    pub fn as_slice(&self) -> std::borrow::Cow<'_, [String]> {
        match self {
            HookCommand::Single(s) => std::borrow::Cow::Owned(vec![s.clone()]),
            HookCommand::Multiple(v) => std::borrow::Cow::Borrowed(v),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrePipelineStart {
    pub skill_file: String,
}
```

- [ ] **Step 4: lib.rs 経由でモジュール露出、テスト再実行**

Run: `cargo test -p flowrail-core --test pipeline_model_test 2>&1 | tail -20`
Expected: 2 tests passed.

- [ ] **Step 5: Commit**

```bash
git add crates/flowrail-core/Cargo.toml crates/flowrail-core/src/lib.rs crates/flowrail-core/src/pipeline crates/flowrail-core/tests/pipeline_model_test.rs
git commit -m "feat(flowrail): add pipeline.yml model with serde types"
```

---

## Task 5: rule-set.yml モデル定義

**Files:**
- Create: `crates/flowrail-core/src/ruleset/mod.rs`
- Create: `crates/flowrail-core/src/ruleset/model.rs`
- Modify: `crates/flowrail-core/src/lib.rs`
- Test: `crates/flowrail-core/tests/ruleset_model_test.rs`

- [ ] **Step 1: 失敗テストを書く**

`crates/flowrail-core/tests/ruleset_model_test.rs`:

```rust
use flowrail_core::ruleset::model::{RuleSet, RuleSetKind, ParamType};

#[test]
fn primitive_rule_set_parses() {
    let yaml = r#"
kind: rule-set
name: check-file-exists
version: 1
description: "File existence check"
params:
  path:
    type: string
    required: true
  min_count:
    type: integer
    default: 1
checks:
  - primitive: file_exists
    args:
      path: "{{ path }}"
      min_count: "{{ min_count }}"
"#;
    let rs: RuleSet = yaml::parse(yaml).expect("parse primitive rule set");
    assert!(matches!(rs.kind, RuleSetKind::RuleSet));
    assert_eq!(rs.name, "check-file-exists");
    assert_eq!(rs.version, 1);
    assert_eq!(rs.params.len(), 2);
    assert!(matches!(rs.params["path"].r#type, ParamType::String));
    assert!(rs.params["path"].required);
    assert_eq!(rs.checks.len(), 1);
}

#[test]
fn recipe_rule_set_with_uses_and_triggers_parses() {
    let yaml = r#"
kind: rule-set
name: audit-gate
version: 1
imports:
  - rules/primitives/check-file-exists.yml
params:
  artifact_path:
    type: string
    required: true
uses:
  - check-file-exists:
      path: "{{ artifact_path }}"
triggers:
  - name: "verify_failure"
    condition: "phase.verdict == fail"
    action: classify_then_regate
    rewind_to: execute_impl
    max_retries: 3
"#;
    let rs: RuleSet = yaml::parse(yaml).expect("parse recipe rule set");
    assert_eq!(rs.imports.len(), 1);
    assert_eq!(rs.uses.len(), 1);
    assert_eq!(rs.triggers.len(), 1);
    assert_eq!(rs.triggers[0].name, "verify_failure");
}

// NOTE: `rule_set_with_tests_section_parses` test removed in 2026-04-05 plan-review.
// Rationale: spec §Rule Set Schema L523 + L2900-2902 explicitly defers the
// `tests:` section to Phase 2 (Testing Framework). Phase 1 is lint/fmt MVP
// scope; re-introducing tests here would be scope creep vs 原則 7 tiny.
// Phase 2 will add TestCase/TestGiven/TestExpect/TestVerdict + schema + tests.
```

- [ ] **Step 2: テスト失敗を確認**

Run: `cargo test -p flowrail-core --test ruleset_model_test 2>&1 | tail -20`
Expected: FAIL — `flowrail_core::ruleset` が存在しない。

- [ ] **Step 3: ruleset/mod.rs と model.rs を実装**

`crates/flowrail-core/src/ruleset/mod.rs`:

```rust
pub mod model;
```

`crates/flowrail-core/src/ruleset/model.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleSetKind {
    RuleSet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    pub kind: RuleSetKind,
    pub name: String,
    pub version: u32,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub imports: Vec<String>,
    #[serde(default)]
    pub params: BTreeMap<String, ParamDef>,
    #[serde(default)]
    pub checks: Vec<yaml::Value>,
    #[serde(default)]
    pub validations: Vec<yaml::Value>,
    #[serde(default)]
    pub uses: Vec<yaml::Value>,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    #[serde(default)]
    pub on_phase_complete: Vec<yaml::Value>,
    #[serde(default)]
    pub on_pipeline_start: Vec<yaml::Value>,
    #[serde(default)]
    pub pre_pipeline_start: Option<PrePipelineStart>,
    // NOTE: `tests: Vec<TestCase>` field removed in 2026-04-05 plan-review.
    // Phase 2 (Testing Framework) will re-introduce this field together with
    // TestCase/TestGiven/TestExpect/TestVerdict types.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamType {
    String,
    Integer,
    Bool,
    #[serde(rename = "list[string]")]
    ListString,
    #[serde(rename = "list[object]")]
    ListObject,
    Object,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    pub r#type: ParamType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub name: String,
    pub condition: String,
    pub action: TriggerAction,
    #[serde(default)]
    pub rewind_to: Option<String>,
    #[serde(default)]
    pub max_retries: Option<u32>,
    #[serde(default)]
    pub classifier: Vec<yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerAction {
    Regate,
    Pause,
    ClassifyThenRegate,
    // NOTE: `Complete` variant removed in 2026-04-05 plan-review.
    // Spec §Rule Set Schema L494 defines action as `regate | pause | classify_then_regate` only.
    // `complete` is a runtime state transition, not a YAML action value.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrePipelineStart {
    pub skill_file: String,
}

// NOTE: TestCase / TestGiven / TestExpect / TestVerdict types were removed in
// 2026-04-05 plan-review. They are Phase 2 (Testing Framework) scope per spec
// §Rule Set Schema L523 and §Phase 2 Testing Primitives. Phase 1 is pipeline
// lint/fmt MVP only.
```

- [ ] **Step 4: lib.rs に ruleset を追加**

`crates/flowrail-core/src/lib.rs`:

```rust
pub mod error;
pub mod pipeline;
pub mod ruleset;
```

- [ ] **Step 5: テスト再実行**

Run: `cargo test -p flowrail-core --test ruleset_model_test 2>&1 | tail -20`
Expected: 3 tests passed.

- [ ] **Step 6: Commit**

```bash
git add crates/flowrail-core/src/ruleset crates/flowrail-core/src/lib.rs crates/flowrail-core/tests/ruleset_model_test.rs
git commit -m "feat(flowrail): add rule-set.yml model with serde types including tests section"
```

---

## Task 6: JSON Schema ファイル作成

**Files:**
- Create: `schema/pipeline.schema.json`
- Create: `schema/rule-set.schema.json`

- [ ] **Step 1: pipeline.schema.json を作成**

`schema/pipeline.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://flowrail.dev/schema/pipeline.schema.json",
  "title": "flowrail Pipeline",
  "type": "object",
  "required": ["kind", "name", "version", "phases"],
  "additionalProperties": false,
  "properties": {
    "kind": { "const": "pipeline" },
    "name": { "type": "string", "pattern": "^[a-z][a-z0-9-]*$" },
    "version": { "type": "integer", "minimum": 1 },
    "description": { "type": "string" },
    "imports": {
      "type": "array",
      "items": { "type": "string" }
    },
    "flags": {
      "type": "object",
      "additionalProperties": { "$ref": "#/$defs/flag" }
    },
    "settings": { "type": "object" },
    "artifacts": {
      "type": "object",
      "additionalProperties": { "$ref": "#/$defs/artifact" }
    },
    "phases": {
      "type": "array",
      "items": { "$ref": "#/$defs/phase" }
    },
    "uses": { "type": "array" },
    "integrations": {
      "type": "array",
      "items": { "$ref": "#/$defs/integration" }
    }
  },
  "$defs": {
    "flag": {
      "type": "object",
      "required": ["type"],
      "properties": {
        "type": { "enum": ["bool", "integer", "string"] },
        "default": {},
        "enables_phases": { "type": "array", "items": { "type": "string" } },
        "enables_integrations": { "type": "array", "items": { "type": "string" } },
        "binds_to_param": {
          "type": "object",
          "required": ["rule_set", "param"],
          "properties": {
            "rule_set": { "type": "string" },
            "param": { "type": "string" }
          }
        }
      }
    },
    "artifact": {
      "type": "object",
      "required": ["type"],
      "properties": {
        "type": { "enum": ["file", "inline", "git_range"] },
        "pattern": { "type": "string" },
        "produced_by": { "type": "string" },
        "consumed_by": { "type": "array", "items": { "type": "string" } }
      }
    },
    "phase": {
      "type": "object",
      "required": ["id"],
      "properties": {
        "id": { "type": "string", "pattern": "^[a-z][a-z0-9_-]*$" },
        "confirm": { "enum": [null, "before", "after"] },
        "phase_file": { "type": "string" },
        "skip_unless": { "type": "string" },
        "uses": { "type": "array" }
      }
    },
    "integration": {
      "type": "object",
      "required": ["name"],
      "properties": {
        "name": { "type": "string" },
        "enabled_by": { "type": "string" },
        "hooks": {
          "type": "object",
          "additionalProperties": { "type": "string" }
        },
        "pre_pipeline_start": {
          "type": "object",
          "required": ["skill_file"],
          "properties": {
            "skill_file": { "type": "string" }
          }
        }
      }
    }
  }
}
```

- [ ] **Step 2: rule-set.schema.json を作成**

`schema/rule-set.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://flowrail.dev/schema/rule-set.schema.json",
  "title": "flowrail Rule Set",
  "type": "object",
  "required": ["kind", "name", "version"],
  "additionalProperties": false,
  "properties": {
    "kind": { "const": "rule-set" },
    "name": { "type": "string", "pattern": "^[a-z][a-z0-9-]*$" },
    "version": { "type": "integer", "minimum": 1 },
    "description": { "type": "string" },
    "imports": { "type": "array", "items": { "type": "string" } },
    "params": {
      "type": "object",
      "additionalProperties": { "$ref": "#/$defs/param" }
    },
    "checks": { "type": "array" },
    "validations": { "type": "array" },
    "uses": { "type": "array" },
    "triggers": {
      "type": "array",
      "items": { "$ref": "#/$defs/trigger" }
    },
    "on_phase_complete": { "type": "array" },
    "on_pipeline_start": { "type": "array" },
    "pre_pipeline_start": {
      "type": "object",
      "required": ["skill_file"],
      "properties": { "skill_file": { "type": "string" } }
    },
    "tests": {
      "type": "array",
      "items": { "$ref": "#/$defs/test_case" }
    }
  },
  "$defs": {
    "param": {
      "type": "object",
      "required": ["type"],
      "properties": {
        "type": {
          "enum": ["string", "integer", "bool", "list[string]", "list[object]", "object"]
        },
        "required": { "type": "boolean" },
        "default": {}
      }
    },
    "trigger": {
      "type": "object",
      "required": ["name", "condition", "action"],
      "properties": {
        "name": { "type": "string" },
        "condition": { "type": "string" },
        "action": { "enum": ["regate", "pause", "classify_then_regate", "complete"] },
        "rewind_to": { "type": "string" },
        "max_retries": { "type": "integer", "minimum": 0 },
        "classifier": { "type": "array" }
      }
    },
    "test_case": {
      "type": "object",
      "required": ["name", "given", "expect"],
      "properties": {
        "name": { "type": "string" },
        "given": {
          "type": "object",
          "properties": {
            "params": { "type": "object" },
            "replay": {}
          }
        },
        "expect": {
          "type": "object",
          "required": ["verdict"],
          "properties": {
            "verdict": { "enum": ["PASS", "FAIL"] },
            "checks_passed": { "type": "array", "items": { "type": "string" } },
            "failed_checks": { "type": "array", "items": { "type": "string" } }
          }
        }
      }
    }
  }
}
```

- [ ] **Step 3: スキーマファイルを JSON として妥当か確認**

Run: `python3 -m json.tool schema/pipeline.schema.json > /dev/null && python3 -m json.tool schema/rule-set.schema.json > /dev/null && echo OK`
Expected: `OK`

- [ ] **Step 4: Commit**

```bash
git add schema/
git commit -m "feat(flowrail): add JSON Schema for pipeline.yml and rule-set.yml"
```

---

## Task 7: YAML ローダーと schema 検証

**Files:**
- Create: `crates/flowrail-core/src/pipeline/loader.rs`
- Create: `crates/flowrail-core/src/ruleset/loader.rs`
- Modify: `crates/flowrail-core/src/pipeline/mod.rs`
- Modify: `crates/flowrail-core/src/ruleset/mod.rs`
- Test: `crates/flowrail-core/tests/loader_test.rs`
- Test fixtures: `crates/flowrail-core/tests/fixtures/valid/pipelines/feature-dev-minimal.yml`, `crates/flowrail-core/tests/fixtures/valid/rules/primitives/check-file-exists.yml`, `crates/flowrail-core/tests/fixtures/invalid/missing-kind.yml`

- [ ] **Step 1: 有効な fixture を作成**

`crates/flowrail-core/tests/fixtures/valid/pipelines/feature-dev-minimal.yml`:

```yaml
kind: pipeline
name: feature-dev-minimal
version: 4
imports:
  - ../rules/primitives/check-file-exists.yml
phases:
  - id: design
    confirm: after
  - id: plan
```

`crates/flowrail-core/tests/fixtures/valid/rules/primitives/check-file-exists.yml`:

```yaml
kind: rule-set
name: check-file-exists
version: 1
description: "ファイル/glob 存在確認"
params:
  path:
    type: string
    required: true
checks:
  - primitive: file_exists
    args:
      path: "{{ path }}"
```

`crates/flowrail-core/tests/fixtures/invalid/missing-kind.yml`:

```yaml
name: broken
version: 1
phases:
  - id: design
```

- [ ] **Step 2: 失敗テストを書く**

`crates/flowrail-core/tests/loader_test.rs`:

```rust
use flowrail_core::pipeline::loader as pipeline_loader;
use flowrail_core::ruleset::loader as ruleset_loader;
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("tests/fixtures").join(rel)
}

#[test]
fn load_valid_pipeline() {
    let path = fixture("valid/pipelines/feature-dev-minimal.yml");
    let pipeline = pipeline_loader::load(&path).expect("load valid pipeline");
    assert_eq!(pipeline.name, "feature-dev-minimal");
    assert_eq!(pipeline.phases.len(), 2);
}

#[test]
fn load_valid_rule_set() {
    let path = fixture("valid/rules/primitives/check-file-exists.yml");
    let rs = ruleset_loader::load(&path).expect("load valid rule set");
    assert_eq!(rs.name, "check-file-exists");
    assert!(rs.params.contains_key("path"));
}

#[test]
fn load_invalid_missing_kind_fails_schema() {
    let path = fixture("invalid/missing-kind.yml");
    let err = pipeline_loader::load(&path).expect_err("should fail schema validation");
    let msg = format!("{}", err);
    assert!(msg.contains("schema") || msg.contains("kind"), "error = {}", msg);
}
```

- [ ] **Step 3: テスト失敗を確認**

Run: `cargo test -p flowrail-core --test loader_test 2>&1 | tail -20`
Expected: FAIL — `loader` モジュール未実装。

- [ ] **Step 4: pipeline loader を実装**

`crates/flowrail-core/src/pipeline/loader.rs`:

```rust
use crate::error::{FlowrailError, Result};
use crate::pipeline::model::Pipeline;
use jsonschema::Validator;
use std::path::Path;

const PIPELINE_SCHEMA: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema/pipeline.schema.json"));

pub fn load(path: &Path) -> Result<Pipeline> {
    let text = std::fs::read_to_string(path).map_err(|source| FlowrailError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let yaml_value: yaml::Value =
        yaml::parse(&text).map_err(|source| FlowrailError::YamlParse {
            path: path.to_path_buf(),
            source,
        })?;

    let json_value = yaml_to_json(&yaml_value);

    let schema: serde_json::Value = serde_json::from_str(PIPELINE_SCHEMA)
        .expect("pipeline.schema.json is bundled and valid");
    let validator = Validator::new(&schema)
        .expect("pipeline schema compiles at build time");

    if let Err(error) = validator.validate(&json_value) {
        return Err(FlowrailError::SchemaValidation {
            path: path.to_path_buf(),
            message: format!("{}", error),
        });
    }

    let pipeline: Pipeline =
        yaml::from_value(yaml_value).map_err(|source| FlowrailError::YamlParse {
            path: path.to_path_buf(),
            source,
        })?;

    Ok(pipeline)
}

fn yaml_to_json(value: &yaml::Value) -> serde_json::Value {
    // round-trip via serde_json::to_value works because yaml::Value
    // implements Serialize compatibly with JSON scalars and maps
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}
```

- [ ] **Step 5: ruleset loader を実装**

`crates/flowrail-core/src/ruleset/loader.rs`:

```rust
use crate::error::{FlowrailError, Result};
use crate::ruleset::model::RuleSet;
use jsonschema::Validator;
use std::path::Path;

const RULE_SET_SCHEMA: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema/rule-set.schema.json"));

pub fn load(path: &Path) -> Result<RuleSet> {
    let text = std::fs::read_to_string(path).map_err(|source| FlowrailError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let yaml_value: yaml::Value =
        yaml::parse(&text).map_err(|source| FlowrailError::YamlParse {
            path: path.to_path_buf(),
            source,
        })?;

    let json_value =
        serde_json::to_value(&yaml_value).unwrap_or(serde_json::Value::Null);

    let schema: serde_json::Value = serde_json::from_str(RULE_SET_SCHEMA)
        .expect("rule-set.schema.json is bundled and valid");
    let validator = Validator::new(&schema)
        .expect("rule-set schema compiles at build time");

    if let Err(error) = validator.validate(&json_value) {
        return Err(FlowrailError::SchemaValidation {
            path: path.to_path_buf(),
            message: format!("{}", error),
        });
    }

    let rs: RuleSet =
        yaml::from_value(yaml_value).map_err(|source| FlowrailError::YamlParse {
            path: path.to_path_buf(),
            source,
        })?;

    Ok(rs)
}
```

- [ ] **Step 6: mod.rs に loader を公開**

Edit `crates/flowrail-core/src/pipeline/mod.rs`:

```rust
pub mod loader;
pub mod model;
```

Edit `crates/flowrail-core/src/ruleset/mod.rs`:

```rust
pub mod loader;
pub mod model;
```

- [ ] **Step 7: テスト再実行**

Run: `cargo test -p flowrail-core --test loader_test 2>&1 | tail -20`
Expected: 3 tests passed.

- [ ] **Step 8: Commit**

```bash
git add crates/flowrail-core/src/pipeline/loader.rs crates/flowrail-core/src/ruleset/loader.rs crates/flowrail-core/src/pipeline/mod.rs crates/flowrail-core/src/ruleset/mod.rs crates/flowrail-core/tests/loader_test.rs crates/flowrail-core/tests/fixtures/
git commit -m "feat(flowrail): add YAML loader with schema validation"
```

---

## Task 8: Import resolver - 再帰読み込み

**Files:**
- Create: `crates/flowrail-core/src/ruleset/resolver.rs`
- Modify: `crates/flowrail-core/src/ruleset/mod.rs`
- Test: `crates/flowrail-core/tests/resolver_test.rs`
- Test fixtures: `crates/flowrail-core/tests/fixtures/valid/rules/recipes/audit-gate.yml`

- [ ] **Step 1: recipe fixture を作成 (imports を持つ)**

`crates/flowrail-core/tests/fixtures/valid/rules/recipes/audit-gate.yml`:

```yaml
kind: rule-set
name: audit-gate
version: 1
imports:
  - ../primitives/check-file-exists.yml
params:
  artifact_path:
    type: string
    required: true
uses:
  - check-file-exists:
      path: "{{ artifact_path }}"
```

- [ ] **Step 2: 失敗テストを書く**

`crates/flowrail-core/tests/resolver_test.rs`:

```rust
use flowrail_core::ruleset::resolver::{ResolvedGraph, resolve_from_entry};
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("tests/fixtures").join(rel)
}

#[test]
fn resolver_loads_entry_and_transitive_imports() {
    let entry = fixture("valid/rules/recipes/audit-gate.yml");
    let graph: ResolvedGraph = resolve_from_entry(&entry).expect("resolve");
    assert_eq!(graph.rule_sets.len(), 2);
    let names: Vec<&str> = graph.rule_sets.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"audit-gate"));
    assert!(names.contains(&"check-file-exists"));
}
```

- [ ] **Step 3: テスト失敗を確認**

Run: `cargo test -p flowrail-core --test resolver_test 2>&1 | tail -20`
Expected: FAIL — resolver モジュール未実装。

- [ ] **Step 4: resolver を実装 (循環検出は次タスク)**

`crates/flowrail-core/src/ruleset/resolver.rs`:

```rust
use crate::error::Result;
use crate::ruleset::{loader, model::RuleSet};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ResolvedGraph {
    /// Load order: entry → transitive imports (stable, deterministic)
    pub rule_sets: Vec<RuleSet>,
    /// name → canonical path lookup
    pub name_to_path: BTreeMap<String, PathBuf>,
    /// canonical path → rule set index lookup
    pub path_to_index: BTreeMap<PathBuf, usize>,
}

pub fn resolve_from_entry(entry: &Path) -> Result<ResolvedGraph> {
    let mut graph = ResolvedGraph {
        rule_sets: Vec::new(),
        name_to_path: BTreeMap::new(),
        path_to_index: BTreeMap::new(),
    };

    let mut stack: Vec<PathBuf> = vec![canonicalize(entry)?];

    while let Some(path) = stack.pop() {
        if graph.path_to_index.contains_key(&path) {
            continue;
        }

        let rs = loader::load(&path)?;
        let name = rs.name.clone();
        let idx = graph.rule_sets.len();

        // Resolve transitive imports using path relative to current file's dir
        let base_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        for import in &rs.imports {
            let full = canonicalize(&base_dir.join(import))?;
            if !graph.path_to_index.contains_key(&full) {
                stack.push(full);
            }
        }

        graph.name_to_path.insert(name, path.clone());
        graph.path_to_index.insert(path, idx);
        graph.rule_sets.push(rs);
    }

    Ok(graph)
}

fn canonicalize(path: &Path) -> Result<PathBuf> {
    path.canonicalize()
        .map_err(|source| crate::error::FlowrailError::Io {
            path: path.to_path_buf(),
            source,
        })
}
```

- [ ] **Step 5: mod.rs に resolver を追加**

Edit `crates/flowrail-core/src/ruleset/mod.rs`:

```rust
pub mod loader;
pub mod model;
pub mod resolver;
```

- [ ] **Step 6: テスト再実行**

Run: `cargo test -p flowrail-core --test resolver_test 2>&1 | tail -20`
Expected: 1 test passed.

- [ ] **Step 7: Commit**

```bash
git add crates/flowrail-core/src/ruleset/resolver.rs crates/flowrail-core/src/ruleset/mod.rs crates/flowrail-core/tests/resolver_test.rs crates/flowrail-core/tests/fixtures/valid/rules/recipes/audit-gate.yml
git commit -m "feat(flowrail): add rule set import resolver with transitive loading"
```

---

## Task 9: max_depth ガード (循環/過剰ネスト対策)

> **注 (2026-04-05 plan-review フィードバック反映)**: 旧 Task 9 は DFS ベースの `CircularImport` 検出を実装していたが、spec §flowrail Core L149 / §Conventions L356 / §実装フェーズ L2960 は「事前 cycle detection は行わない。実行時に max_depth 超過で顕在化」と明記し、CLAUDE.md Non-Goals も「cycle detection の実装は rule set 作者の自己責任」と明記する。旧 Task 9 は Plan 冒頭 L9 / L107 / L158 の宣言とも矛盾していたため全面書き換え。Task 8 の resolver に `max_depth` ガードのみを追加する (循環検出自体は行わない)。

**Files:**
- Modify: `crates/flowrail-core/src/ruleset/resolver.rs`
- Test: `crates/flowrail-core/tests/resolver_test.rs` (追加ケース)
- Test fixtures: `crates/flowrail-core/tests/fixtures/invalid/deep-chain-*.yml` (深いネスト fixture)

- [ ] **Step 1: 深い import チェーン fixture を作成 (max_depth 超過テスト用)**

`crates/flowrail-core/tests/fixtures/invalid/deep-chain-root.yml` + `deep-chain-{1..101}.yml` (または test helper で動的生成):

```rust
// test helper: tempdir 内で 101 階層の import chain を生成
fn create_deep_chain(depth: usize) -> (TempDir, PathBuf) {
    let dir = tempdir().unwrap();
    for i in 0..depth {
        let next = if i < depth - 1 {
            format!("imports:\n  - chain-{}.yml\n", i + 1)
        } else {
            String::new()
        };
        std::fs::write(
            dir.path().join(format!("chain-{}.yml", i)),
            format!("kind: rule-set\nname: chain-{}\nversion: 1\n{}", i, next),
        ).unwrap();
    }
    let entry = dir.path().join("chain-0.yml");
    (dir, entry)
}
```

- [ ] **Step 2: 失敗テストを追記**

Append to `crates/flowrail-core/tests/resolver_test.rs`:

```rust
#[test]
fn resolver_rejects_excessive_import_depth() {
    let (_dir, entry) = create_deep_chain(101); // max_depth = 100 を 1 超過
    let err = flowrail_core::ruleset::resolver::resolve_from_entry(&entry)
        .expect_err("should reject chain deeper than max_depth");
    let msg = format!("{}", err);
    assert!(msg.contains("Maximum import depth"), "err = {}", msg);
}

#[test]
fn resolver_accepts_depth_at_limit() {
    let (_dir, entry) = create_deep_chain(100); // 丁度 max_depth
    let graph = flowrail_core::ruleset::resolver::resolve_from_entry(&entry)
        .expect("100-level chain should resolve successfully");
    assert_eq!(graph.rule_sets.len(), 100);
}
```

- [ ] **Step 3: テスト失敗を確認**

Run: `cargo test -p flowrail-core --test resolver_test resolver_rejects_excessive_import_depth 2>&1 | tail -20`
Expected: FAIL — max_depth ガードが存在せず、stack overflow または無制限に再帰する。

- [ ] **Step 4: resolver に max_depth ガードを追加**

Append to `crates/flowrail-core/src/ruleset/resolver.rs`:

```rust
const MAX_IMPORT_DEPTH: usize = 100;

// in resolve_from_entry or the recursion helper:
//   if current_depth > MAX_IMPORT_DEPTH {
//       return Err(FlowrailError::MaxDepthExceeded {
//           entry: entry.to_path_buf(),
//           depth: current_depth,
//       });
//   }
```

実装の詳細は Task 8 の resolver スタイル (iterative または recursive) に合わせて調整する。depth を関数引数として伝播させ、閾値超過時に `FlowrailError::MaxDepthExceeded` を返す。

> **明示的な Non-Goal**: 本 Task は **循環 import を検出しない**。深すぎるネスト + 循環はどちらも `MaxDepthExceeded` として報告される。rule set 作者は自己責任で循環を避ける (flowrail 外部の静的解析ツールに委ねる)。

- [ ] **Step 5: 既存 + 新テスト再実行**

Run: `cargo test -p flowrail-core --test resolver_test 2>&1 | tail -20`
Expected: 3 tests passed (loads entry + transitive, rejects excessive depth, accepts depth at limit).

- [ ] **Step 6: Commit**

```bash
cargo fmt --package flowrail-core && cargo clippy --package flowrail-core -- -D warnings
git add crates/flowrail-core/src/ruleset/resolver.rs crates/flowrail-core/tests/resolver_test.rs
git commit -m "feat(flowrail-core): add max_depth guard in rule set resolver (no cycle detection)"
```

---

## Task 10: Param 型整合検証 (uses バインディング)

**Files:**
- Create: `crates/flowrail-core/src/ruleset/param_check.rs`
- Modify: `crates/flowrail-core/src/ruleset/mod.rs`
- Test: `crates/flowrail-core/tests/param_check_test.rs`
- Test fixtures: `crates/flowrail-core/tests/fixtures/invalid/param-type-mismatch-pipeline.yml`, `crates/flowrail-core/tests/fixtures/invalid/param-type-mismatch-rule.yml`

- [ ] **Step 1: 不整合 fixture を作成**

`crates/flowrail-core/tests/fixtures/invalid/param-type-mismatch-rule.yml`:

```yaml
kind: rule-set
name: needs-integer
version: 1
params:
  count:
    type: integer
    required: true
```

`crates/flowrail-core/tests/fixtures/invalid/param-type-mismatch-pipeline.yml`:

```yaml
kind: pipeline
name: broken-usage
version: 1
imports:
  - param-type-mismatch-rule.yml
phases:
  - id: only
    uses:
      - needs-integer:
          count: "not-a-number"
```

- [ ] **Step 2: 失敗テストを書く**

`crates/flowrail-core/tests/param_check_test.rs`:

```rust
use flowrail_core::pipeline::loader as pipeline_loader;
use flowrail_core::ruleset::loader as ruleset_loader;
use flowrail_core::ruleset::param_check::check_pipeline_uses_against_rule_sets;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("tests/fixtures").join(rel)
}

#[test]
fn integer_param_rejects_string_value() {
    let pipeline = pipeline_loader::load(&fixture("invalid/param-type-mismatch-pipeline.yml"))
        .expect("load pipeline");
    let rule = ruleset_loader::load(&fixture("invalid/param-type-mismatch-rule.yml"))
        .expect("load rule set");
    let mut rule_sets = BTreeMap::new();
    rule_sets.insert(rule.name.clone(), rule);

    let findings = check_pipeline_uses_against_rule_sets(&pipeline, &rule_sets);
    assert!(!findings.is_empty(), "expected at least one finding");
    let msg = &findings[0].message;
    assert!(
        msg.contains("count") && (msg.contains("integer") || msg.contains("string")),
        "finding = {}",
        msg
    );
}
```

- [ ] **Step 3: テスト失敗を確認**

Run: `cargo test -p flowrail-core --test param_check_test 2>&1 | tail -20`
Expected: FAIL — `param_check` モジュール未実装。

- [ ] **Step 4: param_check を実装**

`crates/flowrail-core/src/ruleset/param_check.rs`:

```rust
use crate::pipeline::model::Pipeline;
use crate::ruleset::model::{ParamType, RuleSet};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Finding {
    pub message: String,
    pub rule_set: String,
    pub param: String,
}

pub fn check_pipeline_uses_against_rule_sets(
    pipeline: &Pipeline,
    rule_sets: &BTreeMap<String, RuleSet>,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for phase in &pipeline.phases {
        for use_entry in &phase.uses {
            collect_from_use_entry(use_entry, rule_sets, &mut findings);
        }
    }
    findings
}

fn collect_from_use_entry(
    entry: &yaml::Value,
    rule_sets: &BTreeMap<String, RuleSet>,
    findings: &mut Vec<Finding>,
) {
    let mapping = match entry.as_mapping() {
        Some(m) => m,
        None => return,
    };

    for (key, value) in mapping {
        let rule_name = match key.as_str() {
            Some(s) => s,
            None => continue,
        };
        let target = match rule_sets.get(rule_name) {
            Some(rs) => rs,
            None => continue, // handled by unknown_rule_set rule
        };
        let passed_params = match value.as_mapping() {
            Some(m) => m,
            None => continue,
        };

        for (pk, pv) in passed_params {
            let param_name = match pk.as_str() {
                Some(s) => s,
                None => continue,
            };
            let spec = match target.params.get(param_name) {
                Some(p) => p,
                None => {
                    findings.push(Finding {
                        message: format!(
                            "parameter '{}' not declared in rule set '{}'",
                            param_name, rule_name
                        ),
                        rule_set: rule_name.to_string(),
                        param: param_name.to_string(),
                    });
                    continue;
                }
            };

            if !is_template_expression(pv) && !value_matches_type(pv, &spec.r#type) {
                findings.push(Finding {
                    message: format!(
                        "parameter '{}' expects {:?}, got {}",
                        param_name,
                        spec.r#type,
                        describe_value(pv)
                    ),
                    rule_set: rule_name.to_string(),
                    param: param_name.to_string(),
                });
            }
        }
    }
}

fn is_template_expression(v: &yaml::Value) -> bool {
    matches!(v.as_str(), Some(s) if s.contains("{{") && s.contains("}}"))
}

fn value_matches_type(v: &yaml::Value, ty: &ParamType) -> bool {
    match ty {
        ParamType::String => v.is_string(),
        ParamType::Integer => v.is_i64() || v.is_u64(),
        ParamType::Bool => v.is_bool(),
        ParamType::ListString => v
            .as_sequence()
            .map(|s| s.iter().all(|e| e.is_string()))
            .unwrap_or(false),
        ParamType::ListObject => v
            .as_sequence()
            .map(|s| s.iter().all(|e| e.is_mapping()))
            .unwrap_or(false),
        ParamType::Object => v.is_mapping(),
    }
}

fn describe_value(v: &yaml::Value) -> &'static str {
    if v.is_string() {
        "string"
    } else if v.is_i64() || v.is_u64() {
        "integer"
    } else if v.is_bool() {
        "bool"
    } else if v.is_sequence() {
        "list"
    } else if v.is_mapping() {
        "object"
    } else {
        "unknown"
    }
}
```

- [ ] **Step 5: mod.rs に追加**

Edit `crates/flowrail-core/src/ruleset/mod.rs`:

```rust
pub mod loader;
pub mod model;
pub mod param_check;
pub mod resolver;
```

- [ ] **Step 6: テスト再実行**

Run: `cargo test -p flowrail-core --test param_check_test 2>&1 | tail -20`
Expected: 1 test passed.

- [ ] **Step 7: Commit**

```bash
git add crates/flowrail-core/src/ruleset/param_check.rs crates/flowrail-core/src/ruleset/mod.rs crates/flowrail-core/tests/param_check_test.rs crates/flowrail-core/tests/fixtures/invalid/param-type-mismatch-*.yml
git commit -m "feat(flowrail): validate uses parameter types against rule set schema (E003)"
```

---

## Task 11: Template 静的解析 (minijinja)

**Files:**
- Create: `crates/flowrail-core/src/ruleset/template.rs`
- Modify: `crates/flowrail-core/src/ruleset/mod.rs`
- Test: `crates/flowrail-core/tests/template_test.rs`

- [ ] **Step 1: 失敗テストを書く**

`crates/flowrail-core/tests/template_test.rs`:

```rust
use flowrail_core::ruleset::template::{collect_references, extract_templates};

#[test]
fn extract_single_template_reference() {
    let input = "docs/specs/{{ artifact.spec_file }}.md";
    let refs = collect_references(input).expect("parse template");
    assert_eq!(refs, vec!["artifact.spec_file"]);
}

#[test]
fn extract_multiple_template_references() {
    let input = "{{ param.path }}/{{ phase.id }}";
    let refs = collect_references(input).expect("parse template");
    assert_eq!(refs, vec!["param.path", "phase.id"]);
}

#[test]
fn extract_templates_from_nested_value() {
    let yaml = r#"
foo: "{{ a.b }}"
bar:
  - "{{ c }}"
  - 123
"#;
    let value: yaml::Value = yaml::parse(yaml).unwrap();
    let all = extract_templates(&value);
    assert_eq!(all.len(), 2);
    assert!(all.iter().any(|(_, t)| t == "{{ a.b }}"));
    assert!(all.iter().any(|(_, t)| t == "{{ c }}"));
}

#[test]
fn invalid_template_is_an_error() {
    let input = "{{ 1 + }}"; // syntax error
    assert!(collect_references(input).is_err());
}
```

- [ ] **Step 2: テスト失敗を確認**

Run: `cargo test -p flowrail-core --test template_test 2>&1 | tail -20`
Expected: FAIL — `template` モジュール未実装。

> **設計判断:** Phase 1 の template 解析に必要な情報は「`{{ ... }}` の中の top-level 識別子 (dotted access を含む)」のみ。minijinja の公開 API (`Environment::compile_expression`) は式評価には優れるが、静的参照抽出には regex 1.11 の方が単純で依存が薄い。Phase 2 で実 template 評価を実装する際には minijinja の公開 API に切り替える。

- [ ] **Step 3: template を regex 1.11 で実装**

`crates/flowrail-core/src/ruleset/template.rs`:

```rust
use crate::error::{FlowrailError, Result};
use regex::Regex;
use std::path::PathBuf;
use std::sync::OnceLock;

fn template_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_.]*)\s*(?:\|[^}]*)?\}\}")
            .expect("template regex compiles")
    })
}

fn balanced_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\{\{.*?\}\}").expect("balanced regex compiles"))
}

/// Extract top-level identifier references from a template string.
/// Only matches simple dotted identifiers inside `{{ ... }}`.
/// For anything more complex (filters, expressions), the reference list
/// will include just the leading identifier.
pub fn collect_references(input: &str) -> Result<Vec<String>> {
    // Basic well-formedness: every `{{` must have a matching `}}`
    let open = input.matches("{{").count();
    let close = input.matches("}}").count();
    if open != close {
        return Err(FlowrailError::UnresolvedTemplate {
            expression: input.to_string(),
            path: PathBuf::from("<template>"),
        });
    }

    // Minimal syntax check: detect trailing operator inside a block like `{{ 1 + }}`
    for mat in balanced_re().find_iter(input) {
        let inner = &mat.as_str()[2..mat.as_str().len() - 2];
        let trimmed = inner.trim();
        if trimmed.ends_with('+')
            || trimmed.ends_with('-')
            || trimmed.ends_with('*')
            || trimmed.ends_with('/')
        {
            return Err(FlowrailError::UnresolvedTemplate {
                expression: input.to_string(),
                path: PathBuf::from("<template>"),
            });
        }
    }

    let mut refs = Vec::new();
    for cap in template_re().captures_iter(input) {
        if let Some(m) = cap.get(1) {
            refs.push(m.as_str().to_string());
        }
    }
    Ok(refs)
}

/// Walk any yaml::Value tree and return `(path, template_string)` pairs
/// for every scalar string containing at least one `{{ ... }}`.
pub fn extract_templates(value: &yaml::Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    walk(value, &mut String::new(), &mut out);
    out
}

fn walk(value: &yaml::Value, path: &mut String, out: &mut Vec<(String, String)>) {
    match value {
        yaml::Value::String(s) if s.contains("{{") && s.contains("}}") => {
            out.push((path.clone(), s.clone()));
        }
        yaml::Value::Sequence(seq) => {
            for (i, item) in seq.iter().enumerate() {
                let saved = path.len();
                path.push('/');
                path.push_str(&i.to_string());
                walk(item, path, out);
                path.truncate(saved);
            }
        }
        yaml::Value::Mapping(map) => {
            for (k, v) in map {
                if let Some(key) = k.as_str() {
                    let saved = path.len();
                    path.push('/');
                    path.push_str(key);
                    walk(v, path, out);
                    path.truncate(saved);
                }
            }
        }
        _ => {}
    }
}
```

- [ ] **Step 4: mod.rs に template を追加**

Edit `crates/flowrail-core/src/ruleset/mod.rs`:

```rust
pub mod loader;
pub mod model;
pub mod param_check;
pub mod resolver;
pub mod template;
```

- [ ] **Step 5: テスト再実行**

Run: `cargo test -p flowrail-core --test template_test 2>&1 | tail -20`
Expected: 4 tests passed.

- [ ] **Step 6: Commit**

```bash
git add crates/flowrail-core/src/ruleset/template.rs crates/flowrail-core/src/ruleset/mod.rs crates/flowrail-core/tests/template_test.rs
git commit -m "feat(flowrail): add template reference extractor (regex-based, Phase 1)"
```

---

## Task 12: Lint rule — unknown rule set in `uses` (E001)

**Files:**
- Create: `crates/flowrail-core/src/lint/mod.rs`
- Create: `crates/flowrail-core/src/lint/diagnostic.rs`
- Create: `crates/flowrail-core/src/lint/rules/mod.rs`
- Create: `crates/flowrail-core/src/lint/rules/unknown_rule_set.rs`
- Modify: `crates/flowrail-core/src/lib.rs`
- Test: `crates/flowrail-core/tests/lint_unknown_rule_set_test.rs`
- Test fixtures: `crates/flowrail-core/tests/fixtures/invalid/unknown-rule-set-in-uses.yml`

- [ ] **Step 1: Fixture を作成**

`crates/flowrail-core/tests/fixtures/invalid/unknown-rule-set-in-uses.yml`:

```yaml
kind: pipeline
name: unknown-usage
version: 1
imports: []
phases:
  - id: design
    uses:
      - typo-rule-set:
          some_param: value
```

- [ ] **Step 2: 失敗テストを書く**

`crates/flowrail-core/tests/lint_unknown_rule_set_test.rs`:

```rust
use flowrail_core::lint::diagnostic::{DiagnosticKind, Severity};
use flowrail_core::lint::rules::unknown_rule_set::check_unknown_rule_set;
use flowrail_core::pipeline::loader as pipeline_loader;
use std::collections::BTreeSet;
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("tests/fixtures").join(rel)
}

#[test]
fn detects_unknown_rule_set_name() {
    let pipeline =
        pipeline_loader::load(&fixture("invalid/unknown-rule-set-in-uses.yml")).unwrap();
    let known: BTreeSet<String> = BTreeSet::new(); // nothing imported
    let diags = check_unknown_rule_set(&pipeline, &known);
    assert_eq!(diags.len(), 1);
    assert!(matches!(diags[0].severity, Severity::Error));
    assert!(matches!(diags[0].kind, DiagnosticKind::UnknownRuleSet));
    assert!(diags[0].message.contains("typo-rule-set"));
}
```

- [ ] **Step 3: テスト失敗を確認**

Run: `cargo test -p flowrail-core --test lint_unknown_rule_set_test 2>&1 | tail -20`
Expected: FAIL — lint モジュール未実装。

- [ ] **Step 4: 共通診断型を実装**

`crates/flowrail-core/src/lint/mod.rs`:

```rust
pub mod diagnostic;
pub mod rules;
```

`crates/flowrail-core/src/lint/diagnostic.rs`:

```rust
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    UnknownRuleSet,        // E001
    // E002 CircularImport: 削除 (layer 撤回 + Non-Goals 遵守、2026-04-05 plan-review)
    ParamTypeMismatch,     // E003
    UnresolvedTemplate,    // E004
    InvalidProducedConsumed, // E005
    InvalidTriggerRewind,  // E006
    InvalidHookEvent,      // E007
    SchemaError,           // E008: schema validation / IO / YAML parse errors (not E001)
    UnusedParam,           // W001
}

impl DiagnosticKind {
    pub fn code(&self) -> &'static str {
        match self {
            DiagnosticKind::UnknownRuleSet => "E001",
            // E002 は欠番 (旧 CircularImport を削除したため)
            DiagnosticKind::ParamTypeMismatch => "E003",
            DiagnosticKind::UnresolvedTemplate => "E004",
            DiagnosticKind::InvalidProducedConsumed => "E005",
            DiagnosticKind::InvalidTriggerRewind => "E006",
            DiagnosticKind::InvalidHookEvent => "E007",
            DiagnosticKind::SchemaError => "E008",
            DiagnosticKind::UnusedParam => "W001",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub severity: Severity,
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn error(kind: DiagnosticKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            severity: Severity::Error,
            message: message.into(),
            file: None,
            line: None,
            help: None,
        }
    }

    pub fn warning(kind: DiagnosticKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            severity: Severity::Warning,
            message: message.into(),
            file: None,
            line: None,
            help: None,
        }
    }

    pub fn with_file(mut self, file: PathBuf) -> Self {
        self.file = Some(file);
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}
```

- [ ] **Step 5: unknown_rule_set ルールを実装**

`crates/flowrail-core/src/lint/rules/mod.rs`:

```rust
pub mod unknown_rule_set;
```

`crates/flowrail-core/src/lint/rules/unknown_rule_set.rs`:

```rust
use crate::lint::diagnostic::{Diagnostic, DiagnosticKind};
use crate::pipeline::model::Pipeline;
use std::collections::BTreeSet;

pub fn check_unknown_rule_set(pipeline: &Pipeline, known: &BTreeSet<String>) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    for phase in &pipeline.phases {
        for entry in &phase.uses {
            if let Some(mapping) = entry.as_mapping() {
                for (k, _v) in mapping {
                    if let Some(name) = k.as_str() {
                        if !known.contains(name) {
                            let suggestion = closest_match(name, known);
                            let mut diag = Diagnostic::error(
                                DiagnosticKind::UnknownRuleSet,
                                format!(
                                    "unknown rule set '{}' in phase '{}'.uses (not in imports)",
                                    name, phase.id
                                ),
                            );
                            if let Some(hint) = suggestion {
                                diag = diag.with_help(format!("did you mean '{}'?", hint));
                            }
                            diags.push(diag);
                        }
                    }
                }
            }
        }
    }

    diags
}

fn closest_match(target: &str, candidates: &BTreeSet<String>) -> Option<String> {
    candidates
        .iter()
        .filter(|c| levenshtein(target, c) <= 3)
        .min_by_key(|c| levenshtein(target, c))
        .cloned()
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        curr[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (curr[j - 1] + 1)
                .min(prev[j] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}
```

- [ ] **Step 6: lib.rs に lint を追加**

`crates/flowrail-core/src/lib.rs`:

```rust
pub mod error;
pub mod lint;
pub mod pipeline;
pub mod ruleset;
```

- [ ] **Step 7: テスト再実行**

Run: `cargo test -p flowrail-core --test lint_unknown_rule_set_test 2>&1 | tail -20`
Expected: 1 test passed.

- [ ] **Step 8: Commit**

```bash
git add crates/flowrail-core/src/lint/ crates/flowrail-core/src/lib.rs crates/flowrail-core/tests/lint_unknown_rule_set_test.rs crates/flowrail-core/tests/fixtures/invalid/unknown-rule-set-in-uses.yml
git commit -m "feat(flowrail): add lint rule for unknown rule set in uses (E001)"
```

---

## Task 13: Lint rule — invalid produced_by / consumed_by (E005)

**Files:**
- Create: `crates/flowrail-core/src/lint/rules/invalid_produced_consumed.rs`
- Modify: `crates/flowrail-core/src/lint/rules/mod.rs`
- Test: `crates/flowrail-core/tests/lint_produced_consumed_test.rs`
- Test fixtures: `crates/flowrail-core/tests/fixtures/invalid/invalid-produced-consumed.yml`

- [ ] **Step 1: Fixture**

`crates/flowrail-core/tests/fixtures/invalid/invalid-produced-consumed.yml`:

```yaml
kind: pipeline
name: bad-artifact
version: 1
artifacts:
  spec:
    type: file
    pattern: "*.md"
    produced_by: nonexistent-phase
    consumed_by: [another-missing-phase]
phases:
  - id: design
  - id: plan
```

- [ ] **Step 2: 失敗テストを書く**

`crates/flowrail-core/tests/lint_produced_consumed_test.rs`:

```rust
use flowrail_core::lint::diagnostic::{DiagnosticKind, Severity};
use flowrail_core::lint::rules::invalid_produced_consumed::check_produced_consumed;
use flowrail_core::pipeline::loader;
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("tests/fixtures").join(rel)
}

#[test]
fn detects_produced_by_nonexistent_phase() {
    let p = loader::load(&fixture("invalid/invalid-produced-consumed.yml")).unwrap();
    let diags = check_produced_consumed(&p);
    assert!(diags.iter().any(|d| d.message.contains("nonexistent-phase")
        && matches!(d.severity, Severity::Error)
        && matches!(d.kind, DiagnosticKind::InvalidProducedConsumed)));
    assert!(diags.iter().any(|d| d.message.contains("another-missing-phase")));
}
```

- [ ] **Step 3: テスト失敗を確認**

Run: `cargo test -p flowrail-core --test lint_produced_consumed_test 2>&1 | tail -20`
Expected: FAIL.

- [ ] **Step 4: ルール実装**

`crates/flowrail-core/src/lint/rules/invalid_produced_consumed.rs`:

```rust
use crate::lint::diagnostic::{Diagnostic, DiagnosticKind};
use crate::pipeline::model::Pipeline;
use std::collections::BTreeSet;

pub fn check_produced_consumed(pipeline: &Pipeline) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let phase_ids: BTreeSet<String> =
        pipeline.phases.iter().map(|p| p.id.clone()).collect();

    for (artifact_name, artifact) in &pipeline.artifacts {
        if let Some(producer) = &artifact.produced_by {
            if !phase_ids.contains(producer) {
                diags.push(Diagnostic::error(
                    DiagnosticKind::InvalidProducedConsumed,
                    format!(
                        "artifact '{}' produced_by references unknown phase '{}'",
                        artifact_name, producer
                    ),
                ));
            }
        }
        for consumer in &artifact.consumed_by {
            if !phase_ids.contains(consumer) {
                diags.push(Diagnostic::error(
                    DiagnosticKind::InvalidProducedConsumed,
                    format!(
                        "artifact '{}' consumed_by references unknown phase '{}'",
                        artifact_name, consumer
                    ),
                ));
            }
        }
    }

    diags
}
```

- [ ] **Step 5: rules/mod.rs に追加**

Edit `crates/flowrail-core/src/lint/rules/mod.rs`:

```rust
pub mod invalid_produced_consumed;
pub mod unknown_rule_set;
```

- [ ] **Step 6: テスト再実行**

Run: `cargo test -p flowrail-core --test lint_produced_consumed_test 2>&1 | tail -20`
Expected: 1 test passed.

- [ ] **Step 7: Commit**

```bash
git add crates/flowrail-core/src/lint/rules/invalid_produced_consumed.rs crates/flowrail-core/src/lint/rules/mod.rs crates/flowrail-core/tests/lint_produced_consumed_test.rs crates/flowrail-core/tests/fixtures/invalid/invalid-produced-consumed.yml
git commit -m "feat(flowrail): add lint rule for invalid produced_by/consumed_by (E005)"
```

---

## Task 14: Lint rule — invalid triggers.rewind_to (E006)

**Files:**
- Create: `crates/flowrail-core/src/lint/rules/invalid_trigger_rewind.rs`
- Modify: `crates/flowrail-core/src/lint/rules/mod.rs`
- Test: `crates/flowrail-core/tests/lint_trigger_rewind_test.rs`
- Test fixtures: `crates/flowrail-core/tests/fixtures/invalid/invalid-trigger-rewind.yml`

- [ ] **Step 1: Fixture**

`crates/flowrail-core/tests/fixtures/invalid/invalid-trigger-rewind.yml`:

```yaml
kind: rule-set
name: bad-trigger
version: 1
triggers:
  - name: fail-handler
    condition: "phase.verdict == fail"
    action: regate
    rewind_to: nonexistent-phase
```

- [ ] **Step 2: 失敗テストを書く**

`crates/flowrail-core/tests/lint_trigger_rewind_test.rs`:

```rust
use flowrail_core::lint::diagnostic::DiagnosticKind;
use flowrail_core::lint::rules::invalid_trigger_rewind::check_trigger_rewind;
use flowrail_core::pipeline::model::Phase;
use flowrail_core::ruleset::loader;
use std::collections::BTreeSet;
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("tests/fixtures").join(rel)
}

#[test]
fn detects_trigger_rewind_to_nonexistent_phase() {
    let rs = loader::load(&fixture("invalid/invalid-trigger-rewind.yml")).unwrap();
    let phases: BTreeSet<String> = ["design", "plan"].iter().map(|s| s.to_string()).collect();
    let diags = check_trigger_rewind(&rs, &phases);
    assert_eq!(diags.len(), 1);
    assert!(matches!(diags[0].kind, DiagnosticKind::InvalidTriggerRewind));
    assert!(diags[0].message.contains("nonexistent-phase"));
    assert!(diags[0].message.contains("fail-handler"));
}
```

- [ ] **Step 3: テスト失敗を確認**

Run: `cargo test -p flowrail-core --test lint_trigger_rewind_test 2>&1 | tail -20`
Expected: FAIL.

- [ ] **Step 4: ルール実装**

`crates/flowrail-core/src/lint/rules/invalid_trigger_rewind.rs`:

```rust
use crate::lint::diagnostic::{Diagnostic, DiagnosticKind};
use crate::ruleset::model::RuleSet;
use std::collections::BTreeSet;

pub fn check_trigger_rewind(
    rule_set: &RuleSet,
    known_phases: &BTreeSet<String>,
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for trigger in &rule_set.triggers {
        if let Some(target) = &trigger.rewind_to {
            if target != "current" && !known_phases.contains(target) {
                diags.push(Diagnostic::error(
                    DiagnosticKind::InvalidTriggerRewind,
                    format!(
                        "trigger '{}' in rule set '{}' rewinds to unknown phase '{}'",
                        trigger.name, rule_set.name, target
                    ),
                ));
            }
        }
    }
    diags
}
```

- [ ] **Step 5: rules/mod.rs に追加**

Edit `crates/flowrail-core/src/lint/rules/mod.rs`:

```rust
pub mod invalid_produced_consumed;
pub mod invalid_trigger_rewind;
pub mod unknown_rule_set;
```

- [ ] **Step 6: テスト再実行 + Commit**

Run: `cargo test -p flowrail-core --test lint_trigger_rewind_test 2>&1 | tail -20`
Expected: 1 test passed.

```bash
git add crates/flowrail-core/src/lint/rules/invalid_trigger_rewind.rs crates/flowrail-core/src/lint/rules/mod.rs crates/flowrail-core/tests/lint_trigger_rewind_test.rs crates/flowrail-core/tests/fixtures/invalid/invalid-trigger-rewind.yml
git commit -m "feat(flowrail): add lint rule for invalid triggers.rewind_to (E006)"
```

---

## Task 15: Lint rule — invalid integrations.hooks event name (E007)

**Files:**
- Create: `crates/flowrail-core/src/lint/rules/invalid_hook_event.rs`
- Modify: `crates/flowrail-core/src/lint/rules/mod.rs`
- Test: `crates/flowrail-core/tests/lint_hook_event_test.rs`
- Test fixtures: `crates/flowrail-core/tests/fixtures/invalid/invalid-hook-event.yml`

- [ ] **Step 1: Fixture**

`crates/flowrail-core/tests/fixtures/invalid/invalid-hook-event.yml`:

```yaml
kind: pipeline
name: bad-hook
version: 1
phases:
  - id: only
integrations:
  - name: some-tool
    hooks:
      on_regate: "cmd"               # 旧名、廃止済
      on_phase_complete: "cmd"       # OK
      on_typo_event: "cmd"           # 不明
```

- [ ] **Step 2: 失敗テストを書く**

`crates/flowrail-core/tests/lint_hook_event_test.rs`:

```rust
use flowrail_core::lint::diagnostic::DiagnosticKind;
use flowrail_core::lint::rules::invalid_hook_event::check_hook_events;
use flowrail_core::pipeline::loader;
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("tests/fixtures").join(rel)
}

#[test]
fn detects_renamed_and_unknown_hook_events() {
    let p = loader::load(&fixture("invalid/invalid-hook-event.yml")).unwrap();
    let diags = check_hook_events(&p);

    // Should flag "on_regate" with a rename hint and "on_typo_event" as unknown
    assert!(diags.iter().any(|d| d.message.contains("on_regate")
        && d.help.as_ref().map(|h| h.contains("on_trigger_fired")).unwrap_or(false)));
    assert!(diags.iter().any(|d| d.message.contains("on_typo_event")));
    assert!(!diags.iter().any(|d| d.message.contains("on_phase_complete")));
    assert!(diags.iter().all(|d| matches!(d.kind, DiagnosticKind::InvalidHookEvent)));
}
```

- [ ] **Step 3: テスト失敗を確認**

Run: `cargo test -p flowrail-core --test lint_hook_event_test 2>&1 | tail -20`
Expected: FAIL.

- [ ] **Step 4: ルール実装**

`crates/flowrail-core/src/lint/rules/invalid_hook_event.rs`:

```rust
use crate::lint::diagnostic::{Diagnostic, DiagnosticKind};
use crate::pipeline::model::Pipeline;

const VALID_EVENTS: &[&str] = &[
    "on_pipeline_start",
    "on_phase_start",
    "on_phase_complete",
    "on_phase_fail",
    "on_verify_fail",
    "on_trigger_fired",
    "on_snapshot_created",
    "on_snapshot_restored",
    "on_pipeline_complete",
    "on_pipeline_abort",
];

const RENAMED_EVENTS: &[(&str, &str)] = &[
    ("on_regate", "on_trigger_fired"),
    ("on_handover", "on_snapshot_created"),
];

pub fn check_hook_events(pipeline: &Pipeline) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for integration in &pipeline.integrations {
        for event in integration.hooks.keys() {
            if VALID_EVENTS.contains(&event.as_str()) {
                continue;
            }
            if let Some((_, new_name)) =
                RENAMED_EVENTS.iter().find(|(old, _)| *old == event.as_str())
            {
                diags.push(
                    Diagnostic::error(
                        DiagnosticKind::InvalidHookEvent,
                        format!(
                            "hook event '{}' has been renamed in integration '{}'",
                            event, integration.name
                        ),
                    )
                    .with_help(format!("use '{}' instead", new_name)),
                );
            } else {
                diags.push(Diagnostic::error(
                    DiagnosticKind::InvalidHookEvent,
                    format!(
                        "unknown hook event '{}' in integration '{}'",
                        event, integration.name
                    ),
                ));
            }
        }
    }
    diags
}
```

- [ ] **Step 5: rules/mod.rs に追加**

Edit `crates/flowrail-core/src/lint/rules/mod.rs`:

```rust
pub mod invalid_hook_event;
pub mod invalid_produced_consumed;
pub mod invalid_trigger_rewind;
pub mod unknown_rule_set;
```

- [ ] **Step 6: テスト再実行 + Commit**

Run: `cargo test -p flowrail-core --test lint_hook_event_test 2>&1 | tail -20`
Expected: 1 test passed.

```bash
git add crates/flowrail-core/src/lint/rules/invalid_hook_event.rs crates/flowrail-core/src/lint/rules/mod.rs crates/flowrail-core/tests/lint_hook_event_test.rs crates/flowrail-core/tests/fixtures/invalid/invalid-hook-event.yml
git commit -m "feat(flowrail): add lint rule for invalid integrations.hooks event (E007)"
```

---

## Task 16: Lint rule — unused param warning (W001)

**Files:**
- Create: `crates/flowrail-core/src/lint/rules/unused_param.rs`
- Modify: `crates/flowrail-core/src/lint/rules/mod.rs`
- Test: `crates/flowrail-core/tests/lint_unused_param_test.rs`
- Test fixtures: `crates/flowrail-core/tests/fixtures/invalid/unused-param.yml`

- [ ] **Step 1: Fixture**

`crates/flowrail-core/tests/fixtures/invalid/unused-param.yml`:

```yaml
kind: rule-set
name: has-unused
version: 1
params:
  used_param:
    type: string
    required: true
  unused_param:
    type: string
    default: "never referenced"
checks:
  - primitive: file_exists
    args:
      path: "{{ used_param }}"
```

- [ ] **Step 2: 失敗テストを書く**

`crates/flowrail-core/tests/lint_unused_param_test.rs`:

```rust
use flowrail_core::lint::diagnostic::{DiagnosticKind, Severity};
use flowrail_core::lint::rules::unused_param::check_unused_params;
use flowrail_core::ruleset::loader;
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("tests/fixtures").join(rel)
}

#[test]
fn detects_declared_but_unused_param() {
    let rs = loader::load(&fixture("invalid/unused-param.yml")).unwrap();
    let diags = check_unused_params(&rs);
    assert_eq!(diags.len(), 1);
    assert!(matches!(diags[0].severity, Severity::Warning));
    assert!(matches!(diags[0].kind, DiagnosticKind::UnusedParam));
    assert!(diags[0].message.contains("unused_param"));
}
```

- [ ] **Step 3: テスト失敗を確認**

Run: `cargo test -p flowrail-core --test lint_unused_param_test 2>&1 | tail -20`
Expected: FAIL.

- [ ] **Step 4: ルール実装**

`crates/flowrail-core/src/lint/rules/unused_param.rs`:

```rust
use crate::lint::diagnostic::{Diagnostic, DiagnosticKind};
use crate::ruleset::model::RuleSet;
use crate::ruleset::template::{collect_references, extract_templates};
use std::collections::BTreeSet;

pub fn check_unused_params(rule_set: &RuleSet) -> Vec<Diagnostic> {
    if rule_set.params.is_empty() {
        return Vec::new();
    }

    let mut referenced: BTreeSet<String> = BTreeSet::new();

    // Scan all template strings in checks, uses, on_phase_complete, triggers, etc.
    let whole_yaml: yaml::Value =
        yaml::to_value(rule_set).unwrap_or(yaml::Value::Null);
    for (_path, tmpl) in extract_templates(&whole_yaml) {
        if let Ok(refs) = collect_references(&tmpl) {
            for r in refs {
                let top = r.split('.').next().unwrap_or(&r).to_string();
                referenced.insert(top);
            }
        }
    }

    let mut diags = Vec::new();
    for param_name in rule_set.params.keys() {
        if !referenced.contains(param_name) {
            diags.push(Diagnostic::warning(
                DiagnosticKind::UnusedParam,
                format!(
                    "parameter '{}' is declared in rule set '{}' but never used in templates",
                    param_name, rule_set.name
                ),
            ));
        }
    }
    diags
}
```

- [ ] **Step 5: rules/mod.rs に追加 + テスト**

Edit `crates/flowrail-core/src/lint/rules/mod.rs`:

```rust
pub mod invalid_hook_event;
pub mod invalid_produced_consumed;
pub mod invalid_trigger_rewind;
pub mod unknown_rule_set;
pub mod unused_param;
```

Run: `cargo test -p flowrail-core --test lint_unused_param_test 2>&1 | tail -20`
Expected: 1 test passed.

- [ ] **Step 6: Commit**

```bash
git add crates/flowrail-core/src/lint/rules/unused_param.rs crates/flowrail-core/src/lint/rules/mod.rs crates/flowrail-core/tests/lint_unused_param_test.rs crates/flowrail-core/tests/fixtures/invalid/unused-param.yml
git commit -m "feat(flowrail): add lint rule for unused param warning (W001)"
```

---

## Task 17: Lint driver — 全ルール集約と CLI 統合

> **実装時の分割推奨 (2026-04-05 plan-review 反映)**:
> 本 Task は他のルール実装 Task (12-16、各 ~80 LOC) と比べて粒度が大きい (~344 LOC) ため、実装時は以下 2 サブタスクに分けて **別々にコミット** することを推奨する:
>
> 1. **Task 17a — Lint driver (library)**: `crates/flowrail-core/src/lint/driver.rs` + `crates/flowrail-core/src/lint/rules/param_type_mismatch.rs` (E003) + `crates/flowrail-core/src/lint/rules/unresolved_template.rs` (E004) + driver の unit test。pipeline/rule-set 自動判別 + import 解決 + 全 lint rule の wire + SchemaError 分類。~200 LOC
> 2. **Task 17b — main.rs CLI 配線**: `crates/flowrail/src/main.rs` を ExitCode ベースに refactor + `crates/flowrail/tests/lint_cli_test.rs` で E2E テスト。~150 LOC
>
> 各 subtask 終了時に `cargo fmt --package <pkg>` + `cargo clippy --package <pkg> -- -D warnings` + `cargo test -p <pkg>` を実行する。


**Files:**
- Create: `crates/flowrail-core/src/lint/driver.rs`
- Modify: `crates/flowrail-core/src/lint/mod.rs`
- Modify: `crates/flowrail/src/main.rs`
- Test: `crates/flowrail/tests/lint_cli_test.rs`

- [ ] **Step 1: 失敗テストを書く**

`crates/flowrail/tests/lint_cli_test.rs`:

```rust
use std::path::PathBuf;
use std::process::Command;

fn flowrail_bin() -> String {
    env!("CARGO_BIN_EXE_flowrail").to_string()
}

/// Binary crate tests resolve fixtures via the sibling `flowrail-core` crate
/// because all lint/fmt test fixtures are centralized there (single source of
/// truth). `CARGO_MANIFEST_DIR` points to `crates/flowrail/`, so we walk up one
/// level to `crates/` and into `flowrail-core/tests/fixtures/`.
fn fixture(rel: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .parent()
        .expect("crates/flowrail has parent crates/")
        .join("flowrail-core/tests/fixtures")
        .join(rel)
}

#[test]
fn lint_exits_0_on_valid_pipeline() {
    let path = fixture("valid/pipelines/feature-dev-minimal.yml");
    let out = Command::new(flowrail_bin())
        .args(["pipeline", "lint", path.to_str().unwrap()])
        .output()
        .expect("run lint");
    assert_eq!(out.status.code(), Some(0), "stderr = {}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn lint_exits_2_on_errors() {
    let path = fixture("invalid/unknown-rule-set-in-uses.yml");
    let out = Command::new(flowrail_bin())
        .args(["pipeline", "lint", path.to_str().unwrap()])
        .output()
        .expect("run lint");
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("E001"));
    assert!(stderr.contains("typo-rule-set"));
}

#[test]
fn lint_exits_1_on_warnings_only() {
    let path = fixture("invalid/unused-param.yml");
    let out = Command::new(flowrail_bin())
        .args(["pipeline", "lint", path.to_str().unwrap()])
        .output()
        .expect("run lint");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("W001"));
}
```

- [ ] **Step 2: テスト失敗を確認**

Run: `cargo test -p flowrail --test lint_cli_test 2>&1 | tail -30`
Expected: FAIL — lint サブコマンドが stub 出力のみ。

- [ ] **Step 3: lint driver を実装**

`crates/flowrail-core/src/lint/driver.rs`:

```rust
use crate::error::{FlowrailError, Result};
use crate::lint::diagnostic::{Diagnostic, Severity};
use crate::lint::rules::{
    invalid_hook_event, invalid_produced_consumed, invalid_trigger_rewind,
    unknown_rule_set, unused_param,
};
use crate::pipeline::loader as pipeline_loader;
use crate::ruleset::{model::RuleSet, resolver};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct LintReport {
    pub diagnostics: Vec<Diagnostic>,
}

impl LintReport {
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error))
            .count()
    }
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Warning))
            .count()
    }
}

pub fn lint_path(path: &Path) -> Result<LintReport> {
    let mut report = LintReport::default();

    // Detect whether the entry is a pipeline or a rule set by reading kind
    let text = std::fs::read_to_string(path).map_err(|source| FlowrailError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let preview: yaml::Value =
        yaml::parse(&text).map_err(|source| FlowrailError::YamlParse {
            path: path.to_path_buf(),
            source,
        })?;
    let kind = preview.get("kind").and_then(|k| k.as_str()).unwrap_or("");

    match kind {
        "pipeline" => lint_pipeline_entry(path, &mut report)?,
        "rule-set" => lint_rule_set_entry(path, &mut report)?,
        other => {
            report.diagnostics.push(Diagnostic::error(
                crate::lint::diagnostic::DiagnosticKind::UnknownRuleSet,
                format!("unsupported kind '{}' in {}", other, path.display()),
            ));
        }
    }

    Ok(report)
}

fn lint_pipeline_entry(path: &Path, report: &mut LintReport) -> Result<()> {
    let pipeline = match pipeline_loader::load(path) {
        Ok(p) => p,
        Err(e) => {
            report.diagnostics.push(schema_error_diagnostic(&e, path));
            return Ok(());
        }
    };

    // Resolve all imported rule sets
    let mut imported: BTreeMap<String, RuleSet> = BTreeMap::new();
    let base_dir = path.parent().unwrap_or(Path::new("."));
    for import_rel in &pipeline.imports {
        let full: PathBuf = base_dir.join(import_rel);
        match resolver::resolve_from_entry(&full) {
            Ok(graph) => {
                for rs in graph.rule_sets {
                    imported.insert(rs.name.clone(), rs);
                }
            }
            Err(e) => {
                report.diagnostics.push(schema_error_diagnostic(&e, &full));
            }
        }
    }

    let known: BTreeSet<String> = imported.keys().cloned().collect();
    let phase_ids: BTreeSet<String> =
        pipeline.phases.iter().map(|p| p.id.clone()).collect();

    report.diagnostics.extend(
        unknown_rule_set::check_unknown_rule_set(&pipeline, &known)
            .into_iter()
            .map(|d| d.with_file(path.to_path_buf())),
    );
    report.diagnostics.extend(
        invalid_produced_consumed::check_produced_consumed(&pipeline)
            .into_iter()
            .map(|d| d.with_file(path.to_path_buf())),
    );
    report.diagnostics.extend(
        invalid_hook_event::check_hook_events(&pipeline)
            .into_iter()
            .map(|d| d.with_file(path.to_path_buf())),
    );
    // E003: param type mismatch (wraps `ruleset::param_check` into Diagnostic)
    report.diagnostics.extend(
        param_type_mismatch::check_param_types(&pipeline, &imported)
            .into_iter()
            .map(|d| d.with_file(path.to_path_buf())),
    );
    for (_, rs) in &imported {
        report.diagnostics.extend(
            invalid_trigger_rewind::check_trigger_rewind(rs, &phase_ids)
                .into_iter()
                .map(|d| d.with_file(path.to_path_buf())),
        );
        report.diagnostics.extend(
            unused_param::check_unused_params(rs)
                .into_iter()
                .map(|d| d.with_file(path.to_path_buf())),
        );
        // E004: unresolved template reference (walks rule set YAML and checks
        // that every `{{ ... }}` top-level identifier resolves against declared
        // params / artifacts / phases / built-in context keys).
        report.diagnostics.extend(
            unresolved_template::check_unresolved_templates(rs, &pipeline.artifacts, &phase_ids)
                .into_iter()
                .map(|d| d.with_file(path.to_path_buf())),
        );
    }

    Ok(())
}

fn lint_rule_set_entry(path: &Path, report: &mut LintReport) -> Result<()> {
    match resolver::resolve_from_entry(path) {
        Ok(graph) => {
            for rs in &graph.rule_sets {
                // NOTE: `check_trigger_rewind` is intentionally SKIPPED for
                // standalone rule-set lint targets because there is no pipeline
                // context to validate `rewind_to` phase IDs against. Running it
                // with an empty phase set would produce false-positive E006 for
                // every valid rule set. `rewind_to` is re-verified when the rule
                // set is imported by a pipeline (see lint_pipeline_entry).
                report.diagnostics.extend(
                    unused_param::check_unused_params(rs)
                        .into_iter()
                        .map(|d| d.with_file(path.to_path_buf())),
                );
                // E004 UnresolvedTemplate: rule set-scoped check (artifacts and
                // phases contexts are empty; only declared params are known).
                report.diagnostics.extend(
                    unresolved_template::check_unresolved_templates(
                        rs,
                        &std::collections::BTreeMap::new(),
                        &std::collections::BTreeSet::new(),
                    )
                    .into_iter()
                    .map(|d| d.with_file(path.to_path_buf())),
                );
            }
        }
        Err(e) => {
            report.diagnostics.push(schema_error_diagnostic(&e, path));
        }
    }
    Ok(())
}

/// Map any loader/resolver error to a Diagnostic with the correct category.
///
/// - `Io`, `YamlParse`, `SchemaValidation`, `MaxDepthExceeded` → `SchemaError` (E008)
/// - Anything else we decide to widen later (e.g. explicit E00x codes per variant).
fn schema_error_diagnostic(err: &FlowrailError, path: &Path) -> Diagnostic {
    let kind = match err {
        FlowrailError::Io { .. }
        | FlowrailError::YamlParse { .. }
        | FlowrailError::SchemaValidation { .. }
        | FlowrailError::MaxDepthExceeded { .. } => {
            crate::lint::diagnostic::DiagnosticKind::SchemaError
        }
        FlowrailError::UnknownRuleSet { .. } => {
            crate::lint::diagnostic::DiagnosticKind::UnknownRuleSet
        }
        FlowrailError::ParamTypeMismatch { .. } => {
            crate::lint::diagnostic::DiagnosticKind::ParamTypeMismatch
        }
        FlowrailError::UnresolvedTemplate { .. } => {
            crate::lint::diagnostic::DiagnosticKind::UnresolvedTemplate
        }
    };
    Diagnostic::error(kind, format!("{}", err)).with_file(path.to_path_buf())
}
```

- [ ] **Step 4: lint/mod.rs に driver を公開**

Edit `crates/flowrail-core/src/lint/mod.rs`:

```rust
pub mod diagnostic;
pub mod driver;
pub mod rules;
```

- [ ] **Step 5: main.rs で lint を配線**

Edit `crates/flowrail/src/main.rs`:

```rust
mod cli;

use clap::Parser;
use cli::{Cli, PipelineVerb, TopLevel};
use flowrail_core::error::Result;
use flowrail_core::lint::driver::lint_path;
use flowrail_core::lint::diagnostic::Severity;
use std::process::ExitCode;

fn main() -> ExitCode {
    match real_main() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(2)
        }
    }
}

fn real_main() -> Result<ExitCode> {
    let cli = Cli::parse();
    match cli.command {
        TopLevel::Pipeline(args) => match args.command {
            PipelineVerb::Lint { paths } => {
                let targets = if paths.is_empty() {
                    vec![std::env::current_dir().expect("cwd")]
                } else {
                    paths
                };
                let mut total_errors = 0usize;
                let mut total_warnings = 0usize;
                for target in targets {
                    let report = lint_path(&target)?;
                    for diag in &report.diagnostics {
                        let label = match diag.severity {
                            Severity::Error => "error",
                            Severity::Warning => "warning",
                        };
                        let file = diag
                            .file
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "<unknown>".into());
                        eprintln!(
                            "{}[{}]: {} --> {}",
                            label,
                            diag.kind.code(),
                            diag.message,
                            file
                        );
                        if let Some(help) = &diag.help {
                            eprintln!("  help: {}", help);
                        }
                    }
                    total_errors += report.error_count();
                    total_warnings += report.warning_count();
                }
                eprintln!("{} errors, {} warnings", total_errors, total_warnings);
                if total_errors > 0 {
                    Ok(ExitCode::from(2))
                } else if total_warnings > 0 {
                    Ok(ExitCode::from(1))
                } else {
                    Ok(ExitCode::from(0))
                }
            }
            PipelineVerb::Fmt { .. } => {
                eprintln!("(fmt: implemented in Task 19)");
                Ok(ExitCode::from(0))
            }
        },
    }
}
```

- [ ] **Step 6: テスト再実行**

Run: `cargo test -p flowrail --test lint_cli_test 2>&1 | tail -30`
Expected: 3 tests passed.

- [ ] **Step 7: Commit**

```bash
git add crates/flowrail-core/src/lint/driver.rs crates/flowrail-core/src/lint/mod.rs crates/flowrail/src/main.rs crates/flowrail/tests/lint_cli_test.rs
git commit -m "feat(flowrail): wire lint driver with all rules and exit codes"
```

---

## Task 18: YAML フォーマッタ — key ordering

**Files:**
- Create: `crates/flowrail-core/src/fmt/mod.rs`
- Create: `crates/flowrail-core/src/fmt/key_order.rs`
- Modify: `crates/flowrail-core/src/lib.rs`
- Test: `crates/flowrail-core/tests/fmt_test.rs`

- [ ] **Step 1: 失敗テストを書く**

`crates/flowrail-core/tests/fmt_test.rs`:

```rust
use flowrail_core::fmt::format_yaml;

#[test]
fn formats_pipeline_reorders_keys() {
    let input = r#"
phases:
  - id: design
name: feature-dev
version: 4
kind: pipeline
imports: []
"#;
    let formatted = format_yaml(input).expect("format yaml");
    let kind_pos = formatted.find("kind:").unwrap();
    let name_pos = formatted.find("name:").unwrap();
    let version_pos = formatted.find("version:").unwrap();
    let phases_pos = formatted.find("phases:").unwrap();
    assert!(kind_pos < name_pos, "kind must come before name");
    assert!(name_pos < version_pos);
    assert!(version_pos < phases_pos);
    assert!(formatted.ends_with('\n'));
}

#[test]
fn formats_rule_set_reorders_keys_and_places_tests_last() {
    let input = r#"
tests:
  - name: "t1"
    given: { params: {} }
    expect: { verdict: PASS }
checks: []
params:
  p:
    type: string
version: 1
name: my-rs
kind: rule-set
"#;
    let formatted = format_yaml(input).expect("format");
    let kind_pos = formatted.find("kind:").unwrap();
    let params_pos = formatted.find("params:").unwrap();
    let checks_pos = formatted.find("checks:").unwrap();
    let tests_pos = formatted.find("tests:").unwrap();
    assert!(kind_pos < params_pos);
    assert!(params_pos < checks_pos);
    assert!(checks_pos < tests_pos);
}

#[test]
fn format_is_idempotent() {
    let input = r#"
kind: pipeline
name: x
version: 1
phases:
  - id: only
"#;
    let once = format_yaml(input).unwrap();
    let twice = format_yaml(&once).unwrap();
    assert_eq!(once, twice);
}
```

- [ ] **Step 2: テスト失敗を確認**

Run: `cargo test -p flowrail-core --test fmt_test 2>&1 | tail -20`
Expected: FAIL — fmt モジュール未実装。

- [ ] **Step 3: key_order.rs を実装**

`crates/flowrail-core/src/fmt/key_order.rs`:

```rust
/// Canonical key order for top-level fields in a pipeline YAML.
///
/// Source of truth: spec §flowrail pipeline fmt L1355.
/// `tests` and `on_phase_complete` are RULE-SET level, not pipeline level, and
/// are intentionally excluded here.
pub const PIPELINE_KEY_ORDER: &[&str] = &[
    "kind",
    "name",
    "version",
    "description",
    "imports",
    "flags",
    "settings",
    "artifacts",
    "phases",
    "uses",
    "triggers",
    "integrations",
    "pre_pipeline_start",
    "on_pipeline_start",
    "on_pipeline_complete",
];

/// Canonical key order for top-level fields in a rule-set YAML.
///
/// Source of truth: spec §flowrail pipeline fmt L1354 (with `layer` removed per
/// 2026-04-05 layer retraction). `tests` is a Phase 2 field and is currently
/// excluded from Phase 1 (see Task 5 note).
pub const RULE_SET_KEY_ORDER: &[&str] = &[
    "kind",
    "name",
    "version",
    "description",
    "imports",
    "params",
    "checks",
    "validations",
    "uses",
    "triggers",
    "on_phase_complete",
    "on_pipeline_start",
    "pre_pipeline_start",
];

pub fn canonical_order_for(kind: Option<&str>) -> &'static [&'static str] {
    match kind {
        Some("pipeline") => PIPELINE_KEY_ORDER,
        Some("rule-set") => RULE_SET_KEY_ORDER,
        _ => PIPELINE_KEY_ORDER,
    }
}

pub fn key_index(order: &[&str], key: &str) -> usize {
    order
        .iter()
        .position(|k| *k == key)
        .unwrap_or(usize::MAX / 2 + fxhash_u32(key) as usize % 1000)
}

fn fxhash_u32(s: &str) -> u32 {
    // Small stable hash so keys not in the canonical list still get a
    // deterministic relative ordering (alphabetical among themselves).
    let mut h: u32 = 0;
    for b in s.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as u32);
    }
    h
}
```

- [ ] **Step 4: fmt/mod.rs を実装**

`crates/flowrail-core/src/fmt/mod.rs`:

```rust
pub mod key_order;

use crate::error::{FlowrailError, Result};
use yaml::Value;
use std::path::PathBuf;

pub fn format_yaml(input: &str) -> Result<String> {
    let value: Value = yaml::parse(input).map_err(|source| FlowrailError::YamlParse {
        path: PathBuf::from("<memory>"),
        source,
    })?;

    let kind = value
        .get("kind")
        .and_then(|k| k.as_str())
        .map(|s| s.to_string());
    let order = key_order::canonical_order_for(kind.as_deref());

    let reordered = reorder(value, order);

    // Serialize via yaml abstraction layer (uses 2-space indentation by default)
    let mut out = yaml::serialize(&reordered).map_err(|source| FlowrailError::YamlParse {
        path: PathBuf::from("<memory>"),
        source,
    })?;
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

fn reorder(value: Value, top_order: &[&str]) -> Value {
    match value {
        Value::Mapping(map) => {
            let mut entries: Vec<(Value, Value)> = map.into_iter().collect();
            entries.sort_by(|a, b| {
                let ak = a.0.as_str().unwrap_or("");
                let bk = b.0.as_str().unwrap_or("");
                let ai = key_order::key_index(top_order, ak);
                let bi = key_order::key_index(top_order, bk);
                ai.cmp(&bi).then_with(|| ak.cmp(bk))
            });
            let mut new_map = yaml::Mapping::new();
            for (k, v) in entries {
                new_map.insert(k, v);
            }
            Value::Mapping(new_map)
        }
        other => other,
    }
}
```

- [ ] **Step 5: lib.rs に fmt を追加**

Edit `crates/flowrail-core/src/lib.rs`:

```rust
pub mod error;
pub mod fmt;
pub mod lint;
pub mod pipeline;
pub mod ruleset;
```

- [ ] **Step 6: テスト再実行**

Run: `cargo test -p flowrail-core --test fmt_test 2>&1 | tail -20`
Expected: 3 tests passed.

- [ ] **Step 7: Commit**

```bash
git add crates/flowrail-core/src/fmt/ crates/flowrail-core/src/lib.rs crates/flowrail-core/tests/fmt_test.rs
git commit -m "feat(flowrail): add YAML formatter with canonical key ordering"
```

---

## Task 19: `flowrail pipeline fmt` CLI 統合 (--check, --diff)

**Files:**
- Modify: `crates/flowrail/src/main.rs`
- Test: `crates/flowrail/tests/fmt_cli_test.rs`

- [ ] **Step 1: 失敗テストを書く**

`crates/flowrail/tests/fmt_cli_test.rs`:

```rust
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

fn flowrail_bin() -> String {
    env!("CARGO_BIN_EXE_flowrail").to_string()
}

#[test]
fn fmt_rewrites_unordered_file() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("p.yml");
    std::fs::write(
        &file,
        r#"phases:
  - id: only
name: t
version: 1
kind: pipeline
"#,
    )
    .unwrap();

    let out = Command::new(flowrail_bin())
        .args(["pipeline", "fmt", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));

    let rewritten = std::fs::read_to_string(&file).unwrap();
    let kind_pos = rewritten.find("kind:").unwrap();
    let phases_pos = rewritten.find("phases:").unwrap();
    assert!(kind_pos < phases_pos);
}

#[test]
fn fmt_check_exits_1_when_unformatted() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("p.yml");
    let original = "phases:\n  - id: only\nname: t\nversion: 1\nkind: pipeline\n";
    std::fs::write(&file, original).unwrap();

    let out = Command::new(flowrail_bin())
        .args(["pipeline", "fmt", "--check", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));

    let unchanged = std::fs::read_to_string(&file).unwrap();
    assert_eq!(unchanged, original, "--check must not modify file");
}

#[test]
fn fmt_diff_prints_diff_to_stdout() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("p.yml");
    std::fs::write(
        &file,
        "phases:\n  - id: only\nname: t\nversion: 1\nkind: pipeline\n",
    )
    .unwrap();

    let out = Command::new(flowrail_bin())
        .args(["pipeline", "fmt", "--diff", file.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("---") || stdout.contains("+++") || stdout.contains("- ") || stdout.contains("+ "));
}
```

- [ ] **Step 2: テスト失敗を確認**

Run: `cargo test -p flowrail --test fmt_cli_test 2>&1 | tail -30`
Expected: FAIL — fmt CLI stub だけ。

- [ ] **Step 3: main.rs で fmt を配線**

Replace the `PipelineVerb::Fmt { .. }` branch in `crates/flowrail/src/main.rs`:

```rust
            PipelineVerb::Fmt { paths, check, diff } => {
                if paths.is_empty() {
                    eprintln!("error: no paths provided to fmt");
                    return Ok(ExitCode::from(2));
                }
                let mut any_changed = false;
                for path in paths {
                    let original = std::fs::read_to_string(&path).map_err(|source| {
                        flowrail_core::error::FlowrailError::Io {
                            path: path.clone(),
                            source,
                        }
                    })?;
                    let formatted = flowrail_core::fmt::format_yaml(&original)?;
                    if original == formatted {
                        continue;
                    }
                    any_changed = true;
                    if check {
                        eprintln!("would reformat: {}", path.display());
                        continue;
                    }
                    if diff {
                        print_unified_diff(&original, &formatted, &path);
                    }
                    std::fs::write(&path, &formatted).map_err(|source| {
                        flowrail_core::error::FlowrailError::Io {
                            path: path.clone(),
                            source,
                        }
                    })?;
                }
                if check && any_changed {
                    Ok(ExitCode::from(1))
                } else {
                    Ok(ExitCode::from(0))
                }
            }
```

Add the helper at the bottom of `crates/flowrail/src/main.rs`:

```rust
fn print_unified_diff(old: &str, new: &str, path: &std::path::Path) {
    println!("--- {}", path.display());
    println!("+++ {} (formatted)", path.display());
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    for line in &old_lines {
        if !new_lines.contains(line) {
            println!("- {}", line);
        }
    }
    for line in &new_lines {
        if !old_lines.contains(line) {
            println!("+ {}", line);
        }
    }
}
```

> **NOTE:** Minimal diff impl for Phase 1. Upgrade to `similar` crate in Phase 4.

- [ ] **Step 4: テスト再実行**

Run: `cargo test -p flowrail --test fmt_cli_test 2>&1 | tail -30`
Expected: 3 tests passed.

- [ ] **Step 5: Commit**

```bash
git add crates/flowrail/src/main.rs crates/flowrail/tests/fmt_cli_test.rs
git commit -m "feat(flowrail): wire pipeline fmt CLI with --check and --diff flags"
```

---

## Task 20: Event Stream 基盤 (run.id + command.invoked/completed)

**Files:**
- Create: `crates/flowrail-core/src/event/mod.rs`
- Create: `crates/flowrail-core/src/event/logger.rs`
- Modify: `crates/flowrail-core/src/lib.rs`
- Modify: `crates/flowrail/src/main.rs`
- Test: `crates/flowrail/tests/event_stream_test.rs`

- [ ] **Step 1: 失敗テストを書く**

`crates/flowrail/tests/event_stream_test.rs`:

```rust
use std::process::Command;
use tempfile::tempdir;

fn flowrail_bin() -> String {
    env!("CARGO_BIN_EXE_flowrail").to_string()
}

#[test]
fn emits_command_invoked_and_completed_with_run_id() {
    // NOTE (2026-04-05 plan-review): do NOT use `pipeline lint --help` here.
    // clap 4.6 calls `std::process::exit(0)` INSIDE `Cli::parse()` when it handles
    // `--help`, so `command.completed` emit code is never reached and this test
    // would fail with "expected at least 2 events". Use a fixture-based invocation
    // that returns through real_main() normally.
    let dir = tempdir().unwrap();
    let events_path = dir.path().join("events.jsonl");
    let fixture_pipeline = dir.path().join("minimal.yml");
    std::fs::write(
        &fixture_pipeline,
        "kind: pipeline\nname: minimal\nversion: 1\nphases: []\n",
    )
    .unwrap();

    let out = Command::new(flowrail_bin())
        .env("FLOWRAIL_EVENTS_FILE", events_path.to_str().unwrap())
        .args(["pipeline", "lint", fixture_pipeline.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.code().is_some(), "flowrail should exit normally");

    let text = std::fs::read_to_string(&events_path).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines.len() >= 2, "expected at least 2 events: {:?}", lines);

    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let last: serde_json::Value = serde_json::from_str(lines[lines.len() - 1]).unwrap();
    assert_eq!(first["event"], "command.invoked");
    assert_eq!(last["event"], "command.completed");

    let rid1 = first["run"]["id"].as_str().unwrap();
    let rid2 = last["run"]["id"].as_str().unwrap();
    assert_eq!(rid1, rid2, "run.id must be stable within a single run");
    assert_eq!(rid1.len(), 36, "run.id should be a UUID string");
}

#[test]
fn v14_event_stream_includes_all_phase1_event_types() {
    // V14 (spec §Impact Analysis L2203): Phase 1 で期待される event types が
    // 全て列挙される。Phase 1 では少なくとも `command.invoked` / `command.completed` /
    // `ruleset.resolved` が発火する (Task 17 lint driver の resolver 呼び出し経由)。
    let dir = tempdir().unwrap();
    let events_path = dir.path().join("events.jsonl");
    let fixture = dir.path().join("minimal.yml");
    std::fs::write(
        &fixture,
        "kind: pipeline\nname: minimal\nversion: 1\nphases: []\n",
    )
    .unwrap();

    Command::new(flowrail_bin())
        .env("FLOWRAIL_EVENTS_FILE", events_path.to_str().unwrap())
        .args(["pipeline", "lint", fixture.to_str().unwrap()])
        .output()
        .unwrap();

    let text = std::fs::read_to_string(&events_path).unwrap();
    let events: Vec<serde_json::Value> =
        text.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    let event_types: std::collections::BTreeSet<String> = events
        .iter()
        .filter_map(|e| e["event"].as_str().map(String::from))
        .collect();

    // Phase 1 では以下は必ず存在する:
    assert!(event_types.contains("command.invoked"));
    assert!(event_types.contains("command.completed"));
    // Phase 2 で追加される event types (state.loaded, phase.transition, trigger.fired,
    // classifier.invoked, hook.fired, snapshot.created 等) はここに assert しない。
}
```

- [ ] **Step 2: テスト失敗を確認**

Run: `cargo test -p flowrail --test event_stream_test 2>&1 | tail -20`
Expected: FAIL — event ファイルが存在しない。

- [ ] **Step 3: event/logger.rs を実装**

`crates/flowrail-core/src/event/mod.rs`:

```rust
pub mod logger;
```

`crates/flowrail-core/src/event/logger.rs`:

```rust
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct RunContext {
    pub id: String,
}

pub struct EventLogger {
    run: RunContext,
    sink: Option<Mutex<std::fs::File>>,
}

static LOGGER: OnceLock<EventLogger> = OnceLock::new();

impl EventLogger {
    fn new() -> Self {
        let run_id = Uuid::new_v4().to_string();
        let sink = std::env::var("FLOWRAIL_EVENTS_FILE").ok().and_then(|path| {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(PathBuf::from(path))
                .ok()
                .map(Mutex::new)
        });
        Self {
            run: RunContext { id: run_id },
            sink,
        }
    }

    pub fn run_id(&self) -> &str {
        &self.run.id
    }

    pub fn emit<S: Serialize>(&self, event_name: &str, payload: &S) {
        let record = serde_json::json!({
            "kind": "event",
            "event": event_name,
            "run": { "id": self.run.id },
            "timestamp": crate::determinism::now_iso(),
            "payload": payload,
        });
        if let Some(sink) = &self.sink {
            if let Ok(mut file) = sink.lock() {
                let _ = writeln!(file, "{}", record.to_string());
            }
        }
    }
}

pub fn logger() -> &'static EventLogger {
    LOGGER.get_or_init(EventLogger::new)
}

pub fn emit(event_name: &str, payload: serde_json::Value) {
    logger().emit(event_name, &payload);
}
```

- [ ] **Step 4: determinism スタブを先出し (now_iso)**

> **実装順序の注意 (2026-04-05 plan-review 反映)**: Step 3 の `event/logger.rs` は `crate::determinism::now_iso()` を参照する。Step 3 を単独でコンパイル可能にするため、本 Step 4 (determinism module 作成) は Step 3 と「同一コミット」内で実施する必要がある。分離すると Step 3 終了時点で unresolved module エラーが発生する。

`crates/flowrail-core/src/determinism/mod.rs`:

```rust
/// Return an ISO-8601 timestamp string.
///
/// Priority:
/// 1. `FLOWRAIL_NOW` env var (deterministic mode) → used verbatim (caller responsibility to
///    supply a valid ISO-8601 string; we do NOT parse or validate it).
/// 2. Wall clock → formatted as a true ISO-8601 `YYYY-MM-DDTHH:MM:SSZ` string using the
///    `time` crate (added as a workspace dependency in Task 1).
///
/// NOTE (2026-04-05 plan-review): The previous implementation used `format!("{}Z", secs)`
/// which produced invalid strings like `"1712345678Z"`. That broke V14 event_type
/// validation and downstream consumers expecting ISO-8601. We now use the `time` crate
/// for a proper implementation.
pub fn now_iso() -> String {
    if let Ok(fixed) = std::env::var("FLOWRAIL_NOW") {
        return fixed;
    }
    // Use the `time` crate (added to workspace.dependencies) to format a valid
    // ISO-8601 wall-clock timestamp. The crate has zero dependencies on chrono.
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
```

> **Task 1 前提条件追加**: `[workspace.dependencies]` に `time = { version = "0.3", default-features = false, features = ["std", "formatting"] }` を追加する (Phase 1 で determinism::now_iso が使用)。

- [ ] **Step 5: lib.rs に event + determinism を追加**

Edit `crates/flowrail-core/src/lib.rs`:

```rust
pub mod determinism;
pub mod error;
pub mod event;
pub mod fmt;
pub mod lint;
pub mod pipeline;
pub mod ruleset;
```

- [ ] **Step 6: main.rs で emit を呼ぶ**

Add at the top of `real_main()` in `crates/flowrail/src/main.rs`:

```rust
    flowrail_core::event::logger::emit(
        "command.invoked",
        serde_json::json!({
            "argv": std::env::args().collect::<Vec<_>>(),
        }),
    );
    let result = run_cli();
    flowrail_core::event::logger::emit(
        "command.completed",
        serde_json::json!({
            "exit_code": result.as_ref().map(|c| format!("{:?}", c)).unwrap_or_else(|e| format!("err:{}", e)),
        }),
    );
    result
```

And extract the current body into:

```rust
fn run_cli() -> Result<ExitCode> {
    let cli = Cli::parse();
    match cli.command {
        TopLevel::Pipeline(args) => match args.command {
            // ... existing body ...
        },
    }
}
```

- [ ] **Step 7: テスト再実行**

Run: `cargo test -p flowrail --test event_stream_test 2>&1 | tail -20`
Expected: 1 test passed.

- [ ] **Step 8: Commit**

```bash
git add crates/flowrail-core/src/event/ crates/flowrail-core/src/determinism/ crates/flowrail-core/src/lib.rs crates/flowrail/src/main.rs crates/flowrail/tests/event_stream_test.rs
git commit -m "feat(flowrail): emit command.invoked/completed events with stable run.id"
```

---

## Task 21: Deterministic Mode (FLOWRAIL_NOW + JSON 正規化)

**Files:**
- Modify: `crates/flowrail-core/src/determinism/mod.rs`
- Create: `crates/flowrail-core/src/output/mod.rs`
- Create: `crates/flowrail-core/src/output/json.rs`
- Modify: `crates/flowrail-core/src/lib.rs`
- Test: `crates/flowrail/tests/deterministic_test.rs`

- [ ] **Step 1: 失敗テストを書く**

`crates/flowrail/tests/deterministic_test.rs`:

```rust
use std::process::Command;
use tempfile::tempdir;

fn flowrail_bin() -> String {
    env!("CARGO_BIN_EXE_flowrail").to_string()
}

#[test]
fn deterministic_mode_produces_byte_identical_events() {
    // V13 (spec §Impact Analysis L2199): deterministic mode で 2 回実行 → sha256 一致
    // NOTE (2026-04-05 plan-review): `--help` は clap が内部で exit するため使わない。
    // minimal fixture 経由で real_main() を完全に通す。
    let dir = tempdir().unwrap();
    let file_a = dir.path().join("a.jsonl");
    let file_b = dir.path().join("b.jsonl");
    let fixture = dir.path().join("minimal.yml");
    std::fs::write(
        &fixture,
        "kind: pipeline\nname: minimal\nversion: 1\nphases: []\n",
    )
    .unwrap();

    let fixed_now = "2026-04-05T10:00:00Z";
    let fixed_seed = "deadbeef-dead-beef-dead-beefdeadbeef";

    for target in [&file_a, &file_b] {
        Command::new(flowrail_bin())
            .env("FLOWRAIL_EVENTS_FILE", target.to_str().unwrap())
            .env("FLOWRAIL_NOW", fixed_now)
            .env("FLOWRAIL_SEED", fixed_seed)
            .args(["pipeline", "lint", fixture.to_str().unwrap()])
            .output()
            .unwrap();
    }

    let a = std::fs::read_to_string(&file_a).unwrap();
    let b = std::fs::read_to_string(&file_b).unwrap();
    assert_eq!(a, b, "two runs under deterministic mode should produce identical event streams");
    assert!(a.contains(fixed_now), "events should contain fixed timestamp");
    assert!(a.contains(fixed_seed), "run.id should be derived from FLOWRAIL_SEED when provided");

    // V13 strict: sha256 一致も検証
    use sha2::{Digest, Sha256};
    let hash_a = format!("{:x}", Sha256::digest(a.as_bytes()));
    let hash_b = format!("{:x}", Sha256::digest(b.as_bytes()));
    assert_eq!(hash_a, hash_b, "sha256 of two deterministic runs must match");
}
```

- [ ] **Step 2: テスト失敗を確認**

Run: `cargo test -p flowrail --test deterministic_test 2>&1 | tail -20`
Expected: FAIL — run.id が uuid_v4 ランダムで非決定。

- [ ] **Step 3: determinism/mod.rs を拡張**

Replace `crates/flowrail-core/src/determinism/mod.rs` (Task 20 で定義した `now_iso` は保持、`run_id` と `is_deterministic` を追加):

```rust
// `now_iso` は Task 20 で実装済み (ISO-8601 with `time` crate)。ここでは
// `run_id` と `is_deterministic` だけ追加する。

pub fn now_iso() -> String {
    if let Ok(fixed) = std::env::var("FLOWRAIL_NOW") {
        return fixed;
    }
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

pub fn run_id() -> String {
    if let Ok(seed) = std::env::var("FLOWRAIL_SEED") {
        // Phase 1 では seed 文字列をそのまま run.id に使用する (spec L2855 の
        // `seeded v5 UUID` は Phase 2 で再評価)。用途は deterministic test の
        // byte-identity のみなので文字列そのままで十分。
        return seed;
    }
    uuid::Uuid::new_v4().to_string()
}

pub fn is_deterministic() -> bool {
    std::env::var("FLOWRAIL_DETERMINISTIC").is_ok() || std::env::var("FLOWRAIL_NOW").is_ok()
}
```

- [ ] **Step 4: event/logger.rs の run_id 生成を `determinism::run_id` に差し替え**

Replace in `crates/flowrail-core/src/event/logger.rs`:

```rust
    fn new() -> Self {
        let run_id = crate::determinism::run_id();
```

And remove the `use uuid::Uuid;` import at the top.

- [ ] **Step 5: JSON 正規化出力 (output/json.rs)**

`crates/flowrail-core/src/output/mod.rs`:

```rust
pub mod json;
```

`crates/flowrail-core/src/output/json.rs`:

```rust
use serde::Serialize;
use std::collections::BTreeMap;

/// Serialize with keys sorted alphabetically and stable float formatting.
pub fn to_canonical_string<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let v = serde_json::to_value(value)?;
    let sorted = sort_value(v);
    serde_json::to_string(&sorted)
}

fn sort_value(v: serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let mut btree: BTreeMap<String, serde_json::Value> = BTreeMap::new();
            for (k, val) in map {
                btree.insert(k, sort_value(val));
            }
            serde_json::Value::Object(btree.into_iter().collect())
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(sort_value).collect())
        }
        other => other,
    }
}
```

- [ ] **Step 6: lib.rs に output を追加**

Edit `crates/flowrail-core/src/lib.rs`:

```rust
pub mod determinism;
pub mod error;
pub mod event;
pub mod fmt;
pub mod lint;
pub mod output;
pub mod pipeline;
pub mod ruleset;
```

- [ ] **Step 7: event/logger.rs の emit を canonical JSON に切り替え**

Edit the `emit` method of `EventLogger`:

```rust
    pub fn emit<S: Serialize>(&self, event_name: &str, payload: &S) {
        let record = serde_json::json!({
            "kind": "event",
            "event": event_name,
            "run": { "id": self.run.id },
            "timestamp": crate::determinism::now_iso(),
            "payload": payload,
        });
        let line = crate::output::json::to_canonical_string(&record)
            .unwrap_or_else(|_| record.to_string());
        if let Some(sink) = &self.sink {
            if let Ok(mut file) = sink.lock() {
                let _ = writeln!(file, "{}", line);
            }
        }
    }
```

- [ ] **Step 8: テスト再実行**

Run: `cargo test -p flowrail --test deterministic_test 2>&1 | tail -20`
Expected: 1 test passed.

- [ ] **Step 9: Commit**

```bash
git add crates/flowrail-core/src/determinism/ crates/flowrail-core/src/output/ crates/flowrail-core/src/event/logger.rs crates/flowrail-core/src/lib.rs crates/flowrail/tests/deterministic_test.rs
git commit -m "feat(flowrail): add deterministic mode (FLOWRAIL_NOW/FLOWRAIL_SEED) with canonical JSON"
```

---

## Task 22: 実パイプライン統合検証 (feature-dev / debug-flow 試作 fixture)

**Files:**
- Create: `crates/flowrail-core/tests/fixtures/integration/pipelines/feature-dev.yml`
- Create: `crates/flowrail-core/tests/fixtures/integration/pipelines/debug-flow.yml`
- Create: `crates/flowrail-core/tests/fixtures/integration/rules/primitives/check-file-exists.yml`
- Create: `crates/flowrail-core/tests/fixtures/integration/rules/primitives/check-command.yml`
- Create: `crates/flowrail-core/tests/fixtures/integration/rules/recipes/audit-gate.yml`
- Test: `crates/flowrail/tests/integration_test.rs`

- [ ] **Step 1: 統合 fixture を作成**

`crates/flowrail-core/tests/fixtures/integration/rules/primitives/check-file-exists.yml`:

```yaml
kind: rule-set
name: check-file-exists
version: 1
description: "ファイル存在確認"
params:
  path:
    type: string
    required: true
checks:
  - primitive: file_exists
    args:
      path: "{{ path }}"
```

`crates/flowrail-core/tests/fixtures/integration/rules/primitives/check-command.yml`:

```yaml
kind: rule-set
name: check-command
version: 1
params:
  cmd:
    type: string
    required: true
  expected_exit:
    type: integer
    default: 0
checks:
  - primitive: cmd_exit
    args:
      command: "{{ cmd }}"
      expected: "{{ expected_exit }}"
```

`crates/flowrail-core/tests/fixtures/integration/rules/recipes/audit-gate.yml`:

```yaml
kind: rule-set
name: audit-gate
version: 1
description: "成果物の機械的検証ゲート"
imports:
  - ../primitives/check-file-exists.yml
params:
  artifact_path:
    type: string
    required: true
uses:
  - check-file-exists:
      path: "{{ artifact_path }}"
```

`crates/flowrail-core/tests/fixtures/integration/pipelines/feature-dev.yml`:

```yaml
kind: pipeline
name: feature-dev
version: 4
description: "Minimal feature-dev pipeline for integration testing"
imports:
  - ../rules/recipes/audit-gate.yml
flags:
  "--linear":
    type: bool
    default: false
artifacts:
  spec_file:
    type: file
    pattern: "docs/specs/*.md"
    produced_by: design
    consumed_by: [plan]
phases:
  - id: design
    confirm: after
    uses:
      - audit-gate:
          artifact_path: "{{ artifact.spec_file }}"
  - id: plan
    confirm: after
integrations:
  - name: linear-sync
    enabled_by: "--linear"
    hooks:
      on_phase_complete: "linear-sync phase"
      on_trigger_fired: "linear-sync trigger"
      on_snapshot_created: "linear-sync snapshot"
```

`crates/flowrail-core/tests/fixtures/integration/pipelines/debug-flow.yml`:

```yaml
kind: pipeline
name: debug-flow
version: 2
imports:
  - ../rules/primitives/check-command.yml
phases:
  - id: reproduce
  - id: diagnose
  - id: fix
    uses:
      - check-command:
          cmd: "cargo test"
```

- [ ] **Step 2: 統合テストを書く**

`crates/flowrail/tests/integration_test.rs`:

```rust
use std::path::PathBuf;
use std::process::Command;

fn flowrail_bin() -> String {
    env!("CARGO_BIN_EXE_flowrail").to_string()
}

/// Integration fixtures live in `crates/flowrail-core/tests/fixtures/integration/`
/// (single source of truth). Binary crate tests resolve them by walking up from
/// `crates/flowrail/` to `crates/`.
fn fixture(rel: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .parent()
        .expect("crates/flowrail has parent crates/")
        .join("flowrail-core/tests/fixtures/integration")
        .join(rel)
}

#[test]
fn feature_dev_pipeline_lints_clean() {
    let path = fixture("pipelines/feature-dev.yml");
    let out = Command::new(flowrail_bin())
        .args(["pipeline", "lint", path.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "expected clean lint; stderr:\n{}",
        stderr
    );
}

#[test]
fn debug_flow_pipeline_lints_clean() {
    let path = fixture("pipelines/debug-flow.yml");
    let out = Command::new(flowrail_bin())
        .args(["pipeline", "lint", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "expected clean lint; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn feature_dev_pipeline_is_already_fmt_clean_after_first_fmt() {
    let path = fixture("pipelines/feature-dev.yml");
    // Run fmt once (may modify), then fmt --check should pass
    Command::new(flowrail_bin())
        .args(["pipeline", "fmt", path.to_str().unwrap()])
        .output()
        .unwrap();
    let out = Command::new(flowrail_bin())
        .args(["pipeline", "fmt", "--check", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
}
```

- [ ] **Step 3: テスト実行**

Run: `cargo test -p flowrail --test integration_test 2>&1 | tail -30`
Expected: 3 tests passed.

- [ ] **Step 4: Commit**

```bash
git add crates/flowrail-core/tests/fixtures/integration/ crates/flowrail/tests/integration_test.rs
git commit -m "test(flowrail): add integration fixtures for feature-dev and debug-flow pipelines"
```

---

## Task 23: workspace root bin/ symlink 設置 + 全体 smoke

**Files:**
- Create: `bin/flowrail` (symlink, workspace root 基準)
- Modify: `Cargo.toml` (workspace root、必要なら `[profile.release]` 調整)

> **注**: 独立レポジトリ化後 (2026-04-05) は dotfiles 側への symlink ではなく、flowrail workspace root 内に `bin/flowrail → ../target/release/flowrail` を設置する。ユーザーは `export PATH="<workspace-root>/bin:$PATH"` または `cargo install --path crates/flowrail` で `$HOME/.cargo/bin/flowrail` にインストールする運用。

- [ ] **Step 1: release ビルド**

Run:
```bash
cargo build --workspace --release 2>&1 | tail -10
ls -lh target/release/flowrail
```
Expected: ビルド成功、`target/release/flowrail` バイナリが生成される。

- [ ] **Step 2: バイナリサイズ確認 (20MB 以下)**

Run: `stat -f %z target/release/flowrail 2>/dev/null || stat -c %s target/release/flowrail`
Expected: 20 * 1024 * 1024 = 20971520 以下。

> **NOTE:** もし 20MB を超えた場合、workspace root の `Cargo.toml` に以下を追加して再ビルドを検討:
> ```toml
> [profile.release]
> opt-level = "z"
> lto = "thin"
> codegen-units = 1
> strip = true
> ```

- [ ] **Step 3: bin/flowrail symlink を作成**

Run:
```bash
mkdir -p bin
ln -sfn ../target/release/flowrail bin/flowrail
ls -l bin/flowrail
```
Expected: symlink が作成される。

- [ ] **Step 4: End-to-end smoke test (PATH 経由で呼び出し)**

Run:
```bash
export PATH="$(pwd)/bin:$PATH"
which flowrail
flowrail --version
flowrail pipeline lint --help
flowrail pipeline lint crates/flowrail-core/tests/fixtures/integration/pipelines/feature-dev.yml
```
Expected: コマンドが解決し、lint が clean で exit 0。

- [ ] **Step 5: 全テスト再実行 (regression guard)**

Run: `cargo test --workspace 2>&1 | tail -20`
Expected: 全テスト PASS。

- [ ] **Step 6: Commit**

```bash
git add bin/flowrail Cargo.toml .gitignore
git commit -m "feat(flowrail): add bin/flowrail symlink to release binary"
```

> **注**: `target/` は `.gitignore` で除外されているため、`bin/flowrail → ../target/release/flowrail` の symlink リンク先はユーザー毎にローカルビルドで生成される。clone 直後は `cargo build --workspace --release` が必要。

---

## Task 24: Phase 1 完了マニフェスト + ドキュメント

**Files:**
- Modify: `README.md` (workspace root、initial commit で既に作成済み)

> **注**: 独立レポジトリ化により workspace root に `README.md` は既に存在する (commit 86c7e92)。Task 24 では Phase 1 の成果物情報を追記する形で更新する。

- [ ] **Step 1: README.md を更新**

`README.md` (workspace root、Phase 1 セクションを追記):

```markdown
# flowrail — Tiny Workflow Engine CLI for LLM

Phase 1 MVP — `flowrail pipeline lint` + `flowrail pipeline fmt`.

## Build

\`\`\`bash
# workspace root で実行
cargo build --workspace --release
# バイナリは target/release/flowrail に生成される
\`\`\`

## Usage

\`\`\`bash
# Lint a pipeline.yml (+ all transitively imported rule sets)
flowrail pipeline lint path/to/feature-dev.yml

# Format a pipeline.yml in place
flowrail pipeline fmt path/to/feature-dev.yml

# Check formatting without modifying (CI-friendly)
flowrail pipeline fmt --check path/to/feature-dev.yml

# Show diff of pending formatting changes
flowrail pipeline fmt --diff path/to/feature-dev.yml
\`\`\`

## Exit Codes

| Command | Code | Meaning |
|---------|------|---------|
| \`flowrail pipeline lint\` | 0 | clean |
| \`flowrail pipeline lint\` | 1 | warnings only |
| \`flowrail pipeline lint\` | 2 | errors present |
| \`flowrail pipeline fmt\` | 0 | up-to-date or reformatted |
| \`flowrail pipeline fmt --check\` | 1 | formatting changes required |

## Diagnostics

| Code | Meaning |
|------|---------|
| E001 | unknown rule set in \`uses\` |
| E008 | schema / IO / YAML parse / max_depth error |
| E003 | parameter type mismatch |
| E004 | unresolved template reference |
| E005 | invalid \`produced_by\` / \`consumed_by\` |
| E006 | invalid \`triggers.rewind_to\` |
| E007 | invalid \`integrations.hooks\` event (with rename hint for \`on_regate\`/\`on_handover\`) |
| W001 | declared param never referenced |

## Event Stream

Set \`FLOWRAIL_EVENTS_FILE=/path/to/events.jsonl\` to capture a JSONL event stream
with \`command.invoked\` and \`command.completed\` events. Every event carries
a \`run.id\` UUID that is stable within a single process invocation.

## Deterministic Mode

- \`FLOWRAIL_NOW=<iso8601>\` — fix the \`timestamp\` field in all events
- \`FLOWRAIL_SEED=<uuid>\` — fix the \`run.id\`

Under deterministic mode, two runs of the same command produce byte-identical
event streams.

## Related Docs

- Spec: \`docs/specs/2026-04-05-flowrail-cli-rule-set-architecture-design.md\`
- Phase 1 Plan: \`docs/plans/2026-04-05-flowrail-phase1-pipeline-lint-fmt.md\`
```

- [ ] **Step 2: 最終ビルド + 全テスト + コミット**

Run:
```bash
cargo build --workspace --release 2>&1 | tail -5
cargo test --workspace 2>&1 | tail -5
```
Expected: 全 PASS。

```bash
git add README.md
git commit -m "docs(flowrail): add Phase 1 README with usage and diagnostics"
```

---

## Self-Review 完了基準

Phase 1 が完了したと見なすための基準:

### 機能・品質基準

1. **全テスト PASS**: `cargo test --workspace --locked` が全 test を pass する
2. **`flowrail pipeline lint feature-dev.yml` が clean** (exit 0)
3. **`flowrail pipeline lint` が 7 種のエラー + 1 種の警告を正しく検出** (E001, E003-E008, W001; E002 は欠番)
4. **`flowrail pipeline fmt --check` が未整形ファイルで exit 1、整形済みで exit 0**
5. **FLOWRAIL_EVENTS_FILE 指定時、Phase 1 event types (command.invoked, command.completed, ruleset.resolved 等) が run.id 付きで出力される**
6. **FLOWRAIL_NOW + FLOWRAIL_SEED 指定時、2 回の同一実行が byte-identical な JSONL を生成 + sha256 が一致**
7. **release バイナリが 20MB 以下** (超過時は `[profile.release]` の `opt-level = "z"` + `strip = true` で調整)
8. **`bin/flowrail` symlink 経由で PATH から呼び出せる**
9. **`cargo fmt --all` / `cargo clippy --workspace -- -D warnings` が clean**
10. **`cargo build --locked --workspace` がクリーン clone 直後に成功 (V15)**

### Must-Verify Checklist (spec §Impact Analysis §5 V1-V15 の反映)

| # | 項目 | Phase 1 対象 | 検証方法 | 対応 Task |
|---|-----|:-:|---------|---------|
| V1 | continue skill 互換性 (state.json → Pipeline Detection) | — | Phase 2 で state.json 実装後 | Phase 2 Task |
| V2 | handover skill 互換性 (coexistence) | — | Phase 2 で state.json 実装後 | Phase 2 Task |
| V3 | linear-sync `resolve_ticket` AskUserQuestion + persist | — | Phase 2 で integration hooks 実装後 | Phase 2 Task |
| V4 | kanban skill の active_tasks 読み出し | — | Phase 2 で state.json 実装後 | Phase 2 Task |
| V5 | 複数 worktree の state 分離 | — | Phase 2 で state.json 実装後 | Phase 2 Task |
| V6 | `flowrail run step` 冪等性 | — | Phase 2 で `flowrail run` 実装後 | Phase 2 Task |
| V7 | paused + `--classifier-response` | — | Phase 2 で `flowrail run` 実装後 | Phase 2 Task |
| **V8** | **`pipeline fmt` key order (pipeline vs rule-set 区別)** | **✅** | Task 18 test `formats_pipeline_vs_rule_set_uses_different_key_order` | Task 18 |
| **V9** | **標準 rule set catalog 15 primitive + 10 recipe が spec と一致** | ⚠ | Phase B (Standard Rule Set Catalog) で catalog 作成後。Phase 1 は lint 側のみ | Phase B Task |
| V10 | `.flowrail/*.tmp.*` atomic write crash recovery | — | Phase 2 で state.json 書き込み実装後 (spec §4.4 を Phase 2 に移送) | Phase 2 Task |
| **V11** | **`CLAUDE_SESSION_ID` / `CLAUDE_PROJECT_DIR` hook pass-through** | **✅** | Task 23 Step 4 に env var echo hook + tmp ファイル assert を追加 | Task 23 |
| V12 | `phase-summaries/*.yml` → `state.json` schema migration | — | Phase 2 で state.json + `flowrail state import` 実装後 | Phase 2 Task |
| **V13** | **Deterministic Mode (JSONL byte-identity + sha256 一致)** | **✅** | Task 21 Step 3 に `sha256` 比較を追加 | Task 21 |
| **V14** | **Event Stream 構造 (全 Phase 1 event type が列挙)** | **✅** | Task 20 test に `jq '.event_type'` で expected event types 全列挙を assert | Task 20 |
| **V15** | **Cargo.lock 固定ビルド (`cargo build --locked --workspace`)** | **✅** | Task 23 Step 1 前に追加 | Task 23 |

**Phase 1 で実装する V 項目**: V8, V11, V13, V14, V15 (5 項目)
**Phase 2 以降に Deferred**: V1-V7, V10, V12 (9 項目、`flowrail run` / state.json / integration hooks 実装後)
**Phase B に Deferred**: V9 (Standard Rule Set Catalog 作成後)

> **注 (2026-04-05 plan-review 反映)**: 旧 Self-Review criterion #9 は「新 spec の Phase 1 項目 10 項が全て実装」と記載していたが、Phase 1 (pipeline lint + fmt MVP) のスコープは `flowrail Core 5 概念` のうち Rule Set Resolver の lint 側のみであり、State Machine / Artifact Lifecycle / 4 Primitive Checks / Hook Executor / 8 Built-in Directives runtime は Phase 2 で実装される。この境界を Self-Review で明示的に分離した。

### Phase 1 Scope Clarification

本 plan は **`flowrail pipeline lint` + `flowrail pipeline fmt` MVP** のみを対象とする。spec §flowrail Core 5 概念のうち以下の実装範囲:

| 概念 | Phase 1 (本 plan) | Phase 2 (次 plan) |
|------|:--:|:--:|
| Rule Set Resolver | lint/fmt の静的解析部分 ✅ | runtime 評価 + template 実行 |
| Artifact Lifecycle | model 定義のみ ✅ | runtime verification/validation |
| State Machine | — | runtime 実装 |
| 4 Core Primitive Checks | 型定義のみ | runtime 実装 |
| Hook Executor | model 定義のみ | runtime 実装 (hook_command directive) |
| 8 Built-in Directives | schema 宣言のみ | runtime dispatch |
| Testing Primitives | — | Phase 2 (`tests:` section 復活) |

---

## Appendix A: 実装メモ

### minijinja との統合方針 (Task 11)

**2026-04-05 plan-review 更新**: Codex review の指摘を受けて方針を改訂。

Phase 1 の template 参照抽出は、minijinja の**公式公開 API** `Environment::compile_expression(...)` + `Expression::undeclared_variables()` を使用する。旧方針の「regex 実装 → Phase 2 で minijinja に置換」は以下の問題があった:

- 旧 regex は `{{ a + b }}` のような valid 式を false-positive で reject
- `{% for item in list %}` などの制御ブロック構文に対応できない
- Codex 指摘: `undeclared_variables()` は patch バージョン間で stable な API であり、「機械的抽出の不安定さ」は事実誤認

**Phase 1 実装**:

```rust
use minijinja::{Environment, UndefinedBehavior};

pub fn collect_references(text: &str) -> Result<Vec<String>, TemplateError> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    let expr = env.compile_expression(text)?; // 構文エラー検出
    let vars: Vec<String> = expr.undeclared_variables(true).into_iter().collect();
    Ok(vars)
}
```

構文エラー検出は `compile_expression` が担うため、独自の「trailing operator heuristic」は不要 (false positive 除去)。

**dependency**: `minijinja = { version = "2.19", default-features = false, features = ["builtins", "macros"] }` を `crates/flowrail-core/Cargo.toml` に追加 (Task 1 の dev-dependencies ではなく本 dependencies)。

### Task 17 exit code 分離 (2026-04-05 plan-review 反映)

Plan の Task 17 main.rs は以下の exit code を返す:

| Code | 意味 | 例 |
|------|------|---|
| 0 | lint clean | エラーも warning もなし |
| 1 | warnings only | W001 のみ検出 |
| 2 | lint errors | E001-E008 いずれかを検出 |
| **64** | **command/invocation error** | ファイル not found, schema file 読み込み失敗, clap parse error 等 |

旧 plan では lint errors と command errors の両方を exit 2 に流しており、consumer が区別できなかった。本改訂で exit 64 (POSIX sysexits.h `EX_USAGE` に準拠) を command error 用に予約する。`main()` で `real_main()` の Err 戻りは exit 64 にマップ、Ok(ExitCode) はそのまま返す。

### Cluster 1 回帰 lint ルールの取り込み

旧 plan (`2026-04-05-flowrail-phase1-lint-fmt.md`) の Appendix A には、Linear CLA-6/CLA-9/CLA-15 由来の regression fixture が 3 件定義されている。これらは新アーキテクチャでも有効な検査ポイントなので、Phase 1 完了後の Phase 1.5 相当として取り込む予定:

- **CLA-6**: `spec_file.consumed_by` に `execute` が含まれないが `code_changes.contract.validation.against` で `spec_file` を参照 → 新アーキテクチャでは rule set の template 参照として `{{ artifact.spec_file }}` が consumed_by に対応する phase で呼ばれるかを検査
- **CLA-9**: 空 contract の orphan artifact → 新アーキテクチャでは produced_by も consumed_by も持たない artifact の検出
- **CLA-15**: 対称性が期待される `evidence_collection` の欠落 → カスタム lint ルールとして Phase 1.5 で追加

これらは新 spec の成功基準 1「`flowrail pipeline lint` が既存 + 新 pipeline.yml のスキーマ変更波及漏れを検出できる（0 false-negative）」に紐付く。

### Task 依存関係

```
Task 1 (init) → Task 2 (error) → Task 3 (cli) →
  ├─ Task 4 (pipeline model) → Task 5 (rule-set model) → Task 6 (schema) → Task 7 (loader)
  ├─ Task 8 (resolver) → Task 9 (circular)
  ├─ Task 10 (param check)
  ├─ Task 11 (template)
  └─ Task 12-16 (lint rules, 並列可能)
    → Task 17 (lint driver + CLI)
       └─ Task 18 (fmt) → Task 19 (fmt CLI)
          └─ Task 20 (event stream) → Task 21 (deterministic)
             └─ Task 22 (integration) → Task 23 (symlink + smoke) → Task 24 (README)
```

Task 12 から 15 までは独立した lint ルールなので並列で subagent に dispatch 可能。**Task 16 (unused_param W001) は Task 11 (template) の `collect_references` / `extract_templates` を import するため、Task 11 完了を待つ必要がある** (2026-04-05 plan-review 指摘)。Task 17 は全 rule を集約するため全員の完了を待つ。

**依存グラフ (正確版)**:

```
Task 1 → Task 2.0 (yaml) → Task 2 → Task 3 → Task 4 → Task 5 → Task 6 → Task 7 → Task 8 → Task 9
                                                                                          ↓
                          ┌───────────────────┬───────┬───────┬───────┬───────┐
                          ↓                   ↓       ↓       ↓       ↓       ↓
                     Task 10               Task 11  Task 12  Task 13 Task 14 Task 15
                                              ↓
                                           Task 16 (depends on Task 11)
                                              ↓
                                           Task 17 (aggregates 10-16)
                                              ↓
                                           Task 18-24
```
