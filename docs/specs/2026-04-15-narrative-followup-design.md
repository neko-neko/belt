# Narrative Artifact Follow-Up Bundle (BELT-33 + BELT-34 + BELT-35)

**Linear**:
- [BELT-33](https://linear.app/neko-neko/issue/BELT-33) — belt-agent cmd_init: resolver 失敗時の orphan run directory cleanup (resolve-before-init)
- [BELT-34](https://linear.app/neko-neko/issue/BELT-34) — CLAUDE.md: belt-core モジュール一覧を 7 → 10 に更新
- [BELT-35](https://linear.app/neko-neko/issue/BELT-35) — resolver: corrupt state.json adversarial probe + schema 欠落 test

**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)
**Status**: Draft
**Date**: 2026-04-15

## Summary

2026-04-15 に merge した context-neutral narrative artifact 実装 (merge `b4d1195`) の final code review で抽出された 5 件の follow-up のうち、以下 3 件を 1 ブランチ・1 spec・1 plan でまとめて対応する:

- **BELT-33 (Medium)**: `belt-agent init` が resolver 失敗時に orphan run directory を残す問題 → resolve-before-init 構造へ再構成
- **BELT-34 (Low)**: `CLAUDE.md` の belt-core モジュール記述 (7 モジュール) が実態 (10 モジュール: config / view / uri 追加) とずれている → docs 更新
- **BELT-35 (Low-Medium)**: narrative spec の `Testing Strategy > Adversarial Probes` で明示された "stale state.json" 挙動が unit/E2E test 未実装 → documented-as-tests で現挙動を固定

## Background

### 問題 1: BELT-33 — orphan run directory

`belt-agent::cmd_init` (`crates/belt-agent/src/main.rs:185-253`) は現状、

1. `engine.init_with_branch(...)` で `.belt/runs/<id>/state.json` をディスクに書く
2. `phase.consumes` の各 `ArtifactRef::External` に対して `resolver.resolve(uri)` を呼ぶ

という順序で動作する。ステップ 2 のいずれかで `ResolveError` が起きると `?` 経由で miette Report が propagate されて cmd_init は非 0 exit するが、ステップ 1 で作成された run directory はディスクに残る。

#### 観察される影響

- `.belt/runs/` に半初期化 run が累積する
- `belt-agent status` の候補 enumeration が膨れる
- `latest_run_id()` iteration が orphan を含む
- `belt://latest/<pipeline>/...` は `status == completed` フィルタで orphan を除外するが、これは偶然であり invariant が壊れている事実は残る

#### 再現

```bash
belt-agent init chain-consumer.yml  # producer 不在で resolver NoCompletedRun
ls .belt/runs/                       # orphan run が残留
```

既存 E2E `e2e_consumer_init_fails_when_no_completed_producer` (`tests/e2e_test.rs:584-634`) は exit code 非 0 と stderr の `"no COMPLETED run"` メッセージは assert しているが、`.belt/runs/` の orphan 不在は assert していない。

### 問題 2: BELT-34 — CLAUDE.md モジュール列挙の陳腐化

`CLAUDE.md` 内の belt-core モジュール記述が実態とずれている:

| 箇所 | 現記述 | 実態 |
|------|--------|------|
| L39 (architecture overview) | `library: model, parser, expander, engine, gate, lint` (6 モジュール列挙) | 10 モジュール (+ config / view / uri / error) |
| L61 (section heading) | `belt-core の 7 モジュール` | `belt-core の 10 モジュール` |
| Table L63-71 | 7 行 (model / parser / expander / engine / gate / lint / error) | 10 行が必要 (+ config / view / uri) |

反映漏れの由来:

- `config` — BELT-22 で追加、`belt.toml` config 読み込み (相対 path 解決含む)
- `view` — BELT-29 で追加、status/next JSON の query-time assembly (YAML drift 反映)
- `uri` — 2026-04-15 narrative artifact で追加、`belt://` URI parser (Run / Latest / WorkspaceLatest)。pure library、I/O ゼロ

`CLAUDE.md` は `depends-on` semantics を持つプロジェクト primary source であり、新規 session の `/doc-check` が実態との乖離を warning に変換する。陳腐化が 2 バージョン分（BELT-22 以降）蓄積しており、再整合が必要。

### 問題 3: BELT-35 — corrupt state.json の挙動 test 未定義

`resolver::resolve_latest` (`crates/belt-agent/src/resolver.rs:68-140`) は各 `.belt/runs/<id>/state.json` を `fs::read_to_string` + `serde_json::from_str` で読む。

```rust
let content = std::fs::read_to_string(&state_path)?;
let v: serde_json::Value = serde_json::from_str(&content)?;
```

これらは `?` で fail-fast に構築されており、corrupt JSON (truncated / invalid syntax) は `ResolveError::StateParse` / `Io` として surface される。一方、必須フィールド欠落 (例: `pipeline`) は

```rust
let p_name = v.get("pipeline").and_then(|x| x.as_str()).unwrap_or("");
```

で空文字列フォールバックされ、pipeline 名不一致として silent skip される。

#### 現状の test 網羅

| テスト | 対象 | 状態 |
|-------|------|------|
| `resolve_run_missing_run_dir` | run directory 不在 → `RunNotFound` | ✓ |
| `resolve_run_missing_artifact` | artifact ファイル不在 → `ArtifactMissing` | ✓ |
| `resolve_latest_errors_when_no_completed` | COMPLETED run 不在 → `NoCompletedRun` | ✓ |
| `e2e_consumer_init_fails_when_no_completed_producer` | cmd_init 経由 NoCompletedRun | ✓ |
| **corrupt state.json (truncated JSON)** | — | **未** |
| **state.json に `pipeline` field 欠落** | — | **未** |
| **state.json がディレクトリ (ファイルでない)** | — | **未** |

narrative artifact spec (`docs/specs/2026-04-14-belt-context-neutral-narrative-artifact.md`) の Testing Strategy に "stale state.json" 項目があるが test 未実装。現挙動を固定する documented-as-tests が欠落している。

#### 発生シナリオ

1. belt-agent プロセスが state.json 書き込み中に kill → truncated JSON
2. ファイルシステム異常 (SIGBUS / disk full) → 部分書き込み
3. 外部ツール (エディタ、スクリプト) が state.json を手動編集して破損
4. `git checkout` で state.json がコンフリクトマーカー付きで残る

### Design Constraints (確定済み)

- **Tiny by Constraint**: `ResolveError` variant の追加は pain 未顕在の範囲で行わない (feedback memory: `feedback_pain_driven_first_class_principle.md`)
- **Fail-loud for corrupt**: JSON parse 失敗は loud error で surface (narrative spec の `tiny + fail-loud` と整合)
- **Silent skip for missing field**: 現挙動を維持し、documented-as-tests で固定する (設計変更は別 issue)
- **Additive**: belt-core の API shape は変えない (後述 Option B 相当の lift を行わない)
- **File-based data flow (BELT-30)**: state.json は `.belt/runs/<id>/` の single source of truth、これを読み取る resolver の読み込み契約を壊さない

## Goals

1. `cmd_init` の atomicity を保証: resolver 失敗時に何も永続化されず、再実行が冪等
2. `CLAUDE.md` の belt-core モジュール記述を 10 モジュール実態に一致させる
3. resolver の corrupt state.json / schema 欠落挙動を unit/E2E tests で documented-as-tests 化

## Non-Goals

- `ResolveError` variant の追加 (例: `StateSchema { field }`)
- `resolver::resolve_latest` の silent skip → loud error 化 (BELT-35 Option B 相当、別 issue)
- `.belt/runs/` の SchemaVersion 管理、state.json への追加フィールド
- `.belt/runs/` 内の stale run GC / cleanup コマンド
- belt-core 側での `ResolveError` の lift (belt-core public API に影響させない)
- `pipeline` / `status` 以外の任意フィールド (`branch`, `run_id` 等) の schema loud 化
- `state.json` がディレクトリの場合の fail 戦略変更 (現 `Io` error の挙動を test で固定するのみ)

## Proposals

### Proposal 1: BELT-33 — resolve-before-init

**Approach**: `cmd_init` の処理順序を再構成し、state を永続化する前に全検証を済ませる。

#### 新しい実行順序

```
1. --inherits-from の run_dir 存在チェック           [既存 L177-183、変更なし]
2. current_branch 取得                                [既存 L190、変更なし]
3. expand_pipeline(pipeline_path)                     [既存 L201 から前倒し]
4. Resolver 構築 → 全 External URI 解決 → resolved_map 構築  [既存 L202-214 から前倒し]
5. --inherits-from の synthetic key 挿入 (belt://run/<id>/)  [既存 L220-226、順序維持]
6. engine.init_with_branch(...)                      [ステップ 3-5 成功時のみ実行]
7. engine.set_resolved_consumes(state, resolved_map) [既存]
8. next_phase_info → JSON 出力                        [既存]
```

**Invariant**: ステップ 3-5 のいずれかが失敗すると `.belt/runs/` には何も書かれない (atomic)。

#### Resolver 構築の変更点

現行では `state.branch.clone()` を渡している (`main.rs:204`) が、ステップ 2 で取得済みの `branch: Option<String>` を直接渡す。state への依存が消え、resolver 構築が init に先立って実行可能になる。

#### なぜ Option A (resolve-before-init) か

- **Option B (error path cleanup)**: resolver 呼び出しごとに `?` を展開して `remove_dir_all(run_dir)` を挟む案。I/O が増え、cleanup 自体の失敗ハンドリングが必要になり、partial failure 状態が複雑化する
- **Option A**: cleanup が不要。"state を永続化する前に全検証を済ませる" という invariant を `main.rs` 全体に適用でき、`--inherits-from` の existence check (既に init 前) と同じパターンで揃う

採用は **Option A**。

#### 回帰テスト強化 (BELT-33 commit 内に含める)

**既存 E2E 拡張**: `e2e_consumer_init_fails_when_no_completed_producer` に orphan 不在 assertion を追加:

```rust
// 既存 producer (in_progress) 以外の run directory が残っていないことを assert
let stray: Vec<_> = std::fs::read_dir(tmp.path().join(".belt/runs"))
    .unwrap()
    .filter_map(|e| e.ok().map(|e| e.file_name().into_string().unwrap()))
    .filter(|n| n != run)
    .collect();
assert!(stray.is_empty(), "orphan consumer run left: {stray:?}");
```

**新規 regression E2E**: `e2e_init_succeeds_after_resolver_failure`

1. `belt-agent init chain-consumer.yml` 実行 → producer 不在で resolver 失敗
2. `.belt/runs/` が空であることを assert (orphan 不在)
3. producer run を seed (COMPLETED 状態)
4. `belt-agent init chain-consumer.yml` 再実行 → 成功
5. `.belt/runs/` には producer + consumer の 2 件のみ (half-initialised run の累積なし)

### Proposal 2: BELT-34 — CLAUDE.md 更新

docs only の差分 3 箇所。

#### 差分 1: L39 architecture overview

現状:
```
│   ├── belt-core/    # 📦 library: model, parser, expander, engine, gate, lint
```

更新:
```
│   ├── belt-core/    # 📦 library: 10 modules (model / parser / expander / engine / gate / lint / config / view / uri / error)
```

#### 差分 2: L61 section heading

現状: `## belt-core の 7 モジュール`
更新: `## belt-core の 10 モジュール`

#### 差分 3: table 3 行追加

既存 table の `error` 行の**直前**に以下 3 行を挿入:

| モジュール | 責務 |
|-----------|------|
| `config` | `belt.toml` config 読み込み (相対 path 解決含む)。BELT-22 で追加 |
| `view` | `status` / `next` JSON の query-time assembly (YAML drift 反映)。BELT-29 で追加 |
| `uri` | `belt://` URI parser (Run / Latest / WorkspaceLatest)。pure library、I/O ゼロ。2026-04-15 narrative artifact で追加 |

挿入位置の根拠: 追加順序 (BELT-22 → BELT-29 → 2026-04-15) に従い error の前に挿入し、`error` をユーティリティ的役割として末尾に残す。結果の順序: `model / parser / expander / engine / gate / lint / config / view / uri / error`。

### Proposal 3: BELT-35 — adversarial probes

#### Unit tests 3 本追加 (`crates/belt-agent/src/resolver.rs::tests`)

**(1) `resolve_latest_errors_on_corrupt_state_json`**

- 準備: run directory を作り、state.json に truncated JSON (例: `{"run_id": "trun`) を書く
- 実行: `Resolver::resolve(BeltUri::Latest { pipeline: "feature-dev", path: "notes/phase-review.md" })`
- 検証: `matches!(result, Err(ResolveError::StateParse(_)))`

**(2) `resolve_latest_skips_state_json_without_pipeline_field`**

- 準備: state.json に valid JSON を書くが `pipeline` field を欠落させる。他 candidate は作らない
- 実行: `Resolver::resolve(BeltUri::Latest { pipeline: "feature-dev", path: "notes/phase-review.md" })`
- 検証: `matches!(result, Err(ResolveError::NoCompletedRun { .. }))`
- 備考: 観測可能な silent skip の副作用は「この run が candidate に残らず、他候補なしで NoCompletedRun になる」。現挙動を固定する意図。loud 化は Non-Goal

**(3) `resolve_latest_errors_when_state_json_is_directory`**

- 準備: state.json の位置に file ではなくディレクトリを作る
- 実行: `Resolver::resolve(BeltUri::Latest { .. })`
- 検証: `matches!(result, Err(ResolveError::Io(_)))`

#### E2E test 1 本追加 (`crates/belt-agent/tests/e2e_test.rs`)

**`e2e_init_fails_when_producer_state_json_is_corrupt`**

- 準備: producer run directory を作り、`notes/phase-review.md` と truncated state.json を書く
- 実行: `belt-agent init chain-consumer.yml`
- 検証:
  - exit code 非 0
  - stderr に `"state.json parse error"` または `"json"` / `"parse"` (literal の変動を吸収するため `contains` の OR 条件) が含まれる
  - **加えて** `.belt/runs/` に consumer の orphan run が残らない (BELT-33 の atomicity と合わせて cross-verify)

E2E は corrupt のみ 1 本。missing field は silent skip で `NoCompletedRun` と外形区別がつかないため、cmd_init 経由の E2E では別テストを作る価値が薄い (unit で充足)。

## Data Flow

### cmd_init の新 error path

```
args + pipeline_path
    │
    ├─ inherits_from.is_some() かつ run_dir missing → exit 非 0 ── .belt に書き込みなし ──┐
    │                                                                                      │
    ├─ expand_pipeline() → YAML parse failure → exit 非 0 ── .belt に書き込みなし ─────────┤
    │                                                                                      │
    ├─ resolver.resolve(uri) for each External → ResolveError → exit 非 0 ─────────────────┤
    │                                                                                      │
    │                        (全て成功)                                                     │
    │                              ↓                                                       │
    ├─ init_with_branch() → state.json 書き込み                                             │
    ├─ set_resolved_consumes() → state.json 更新                                            │
    └─ next_phase_info() → stdout JSON                                                     │
                                                                                          │
    orphan 不在 ←────────────────────────────────────────────────────────────────────────┘
```

## Error Handling

- `ResolveError` は `?` 経由で `miette::Report` に lift (既存 behavior 維持)
- 新規 E2E は stderr 文字列の `contains` 検査 (現行 E2E test と同パターン)
- `ResolveError` variant は追加しない (pain-driven 原則)
- 新規 unit test は `matches!` マクロで variant を精密 assert

## Testing Strategy

| 種別 | 追加 | 拡張 | 合計 |
|------|------|------|------|
| Unit (`resolver.rs::tests`) | 3 | 0 | 3 |
| E2E (`tests/e2e_test.rs`) | 2 | 1 | 3 |
| 計 | 5 | 1 | **6** |

### Adversarial probes

- **Atomicity**: resolver 失敗後に run_dir が残らない (BELT-33 regression、既存拡張 + 新規)
- **Idempotency**: resolver 失敗 → producer seed → init 再実行が成功 (BELT-33 regression 新規)
- **Corrupt JSON**: loud `StateParse` error (BELT-35 unit + E2E)
- **Schema 欠落**: silent skip 挙動を固定 (BELT-35 unit)
- **Non-file state.json**: directory の場合の `Io` error (BELT-35 unit)

### Verification commands (変更 crate スコープ)

```bash
cargo test -p belt-agent
cargo clippy -p belt-agent -- -D warnings
cargo fmt --package belt-agent
```

`CLAUDE.md` のみの変更では fmt/clippy/test は不要。

## Commit / Branch Structure

- **ブランチ**: `2026-04-15-narrative-followup`
- **Commit (3 本)**:
  1. `docs(claude-md): update belt-core module list to 10 (BELT-34)` — docs only、先行で小さくスタート
  2. `fix(belt-agent): resolve External URIs before init (BELT-33)` — cmd_init 構造変更 + orphan regression E2E (既存拡張 1 + 新規 1)
  3. `test(belt-agent): adversarial probes for corrupt state.json (BELT-35)` — unit 3 本 + E2E 1 本

各 commit 独立に `cargo test -p belt-agent` 全緑を要件とする。順序は任意だが上記を推奨 (docs → behavior change → test addition で risk 低い順)。

## Open Questions

なし (設計判断は brainstorming で確定済み)。

## References

- 親 merge: `b4d1195` (2026-04-15 narrative artifact implementation)
- Review memory: `memory/project_belt_narrative_impl_2026_04_15.md` (5 follow-up の記録)
- 関連 spec: `docs/specs/2026-04-14-belt-context-neutral-narrative-artifact.md` (Testing Strategy > Adversarial Probes で stale state.json 項目が明示)
- 実装ファイル:
  - `crates/belt-agent/src/main.rs` (cmd_init)
  - `crates/belt-agent/src/resolver.rs` (resolve_latest)
  - `crates/belt-agent/tests/e2e_test.rs` (E2E tests)
  - `CLAUDE.md` (L39, L61-71)
