# Context-Neutral Narrative Artifact

**Linear**: BELT-TBD (to be created)
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)
**Status**: Draft
**Date**: 2026-04-14

## Summary

belt-core に **narrative を Artifact として first-class 化** し、phase 完了時の note 提出を既存 `has_output` gate で決定論的に強制する。前 run の narrative を次 run が `consumes` で参照する際は、新設の `belt://` URI scheme で解決する。これにより `/feature-dev` → `/debug-flow` のような pipeline 間の narrative 引き継ぎを手動負担ゼロで実現する。

`/clear` 自体は Claude Code runtime 制約により人間操作だが、その前後（narrative 保存・復元）を完全自動化することが目的。

## Background

### Problem

現行 belt は phase ごとの gate / validate / regate / artifact を決定論的に管理するが、**narrative（decisions, concerns, directives, observations）の引き継ぎは `/handover` + `/continue` skill の手動フローに完全依存** している。具体的には:

1. **(B) LLM の良心起動**: `/handover` は `"responses slower"` や `"50 tool calls"` 等の heuristic で LLM 自身が判断して起動する。決定論的保証がない
2. **(A) 2 store 断絶**: `.belt/runs/{run_id}/state.json`（belt owned, deterministic）と `.agents/handover/{branch}/{fingerprint}/project-state.json`（skill owned, narrative）が独立しており、両者を繋ぐ識別子がない
3. **(C) Pipeline 間の引き継ぎが手作業**: `/feature-dev` 完了後に `/debug-flow` を起動する際、前 pipeline の concerns / directives を手動で伝える必要がある。skill 側の `phase_summaries.{concerns, directives}` keyed by `target_phase` は存在するが、belt がその構造を認識しない
4. **(D) Session 分割復元**: context 枯渇で同一 pipeline が複数 session に分かれる際、新 session が手動で `/continue` を叩き、LLM が handover.md を読み直す

**本 spec の直接目的**: 上記のうち **B (LLM 良心起動排除) と C (pipeline 間引き継ぎ)** を解決する。A と D は副産物として部分解消（A: `project-state.json.belt_runs[]` の interface 定義、D: `belt-agent status` + per-phase artifact scan で既存機構のまま自動カバー）。

### Design Constraints（確定済み憲法）

- **BELT-24: Context Neutrality** — belt の CLI 設計はコンテキスト戦略（single/multi）に対して中立でなければならない。別 context から呼んでも動作する形が条件
- **BELT-21: CLI is deterministic, skill is protocol** — 決定論的に防げる違反は CLI が拒否すべき。semantic judgment は skill 責務
- **Tiny by Constraint**: belt-core は pure library、HTTP / async runtime / TUI 依存を避ける
- **File-based data flow (BELT-30)**: phase 間のデータ受け渡しは file 経由で統一

### Claude Code Runtime Limits

事前調査で判明した不可避の制約:
- **programmatic `/clear` は存在しない** — hook / tool では conversation reset を発火不能
- **programmatic session respawn は存在しない** — `claude --resume` は directory-bound、起動プロンプト注入不可
- **Session-scoped permissions は resume で失われる** — 再承認必要

したがって **`/clear` の自動化は本 spec の対象外**。ただし前後（narrative snapshot / 自動復元）は完全自動化できる。

## Goals

1. LLM の良心起動を排除し、**phase 完了時の narrative 提出を belt が決定論的に強制**する
2. `/feature-dev` → `/debug-flow` のような **pipeline 間 narrative 引き継ぎを YAML 宣言のみで実現**する
3. 既存 `/handover` `/continue` skill との **役割分離**を明文化し、両者を補完関係で共存させる
4. belt-core の pure library 性 / Linux philosophy / tiny by constraint 原則を維持する

## Non-Goals

- `/clear` / session respawn の自動化（Claude Code runtime 制約、対応不能）
- Cross-repo narrative sync（将来検討）
- Narrative content の semantic validation（belt は中立、parse しない）
- Session-level narrative（active_tasks, recent_decisions 等）の belt 吸収（skill 責務のまま）
- Narrative の TUI 可視化（将来の `belt-tui` スコープ）
- `PreCompact` hook による自動 snapshot 起動（follow-up）

## Design

### Architecture

責務レイヤーは以下の 3 層。

```
┌─────────────────────────────────────────────────────────────┐
│ Skill 層 (`/handover`, `/continue`, `/feature-dev` 等)       │
│ - session-level narrative (active_tasks, recent_decisions)  │
│ - user approval gate                                         │
│ - workspace / worktree scan                                  │
│ - frontmatter parse (必要時)                                 │
│ - project-state.json に `belt_runs: [<run_id>]` で link      │
└────────────────────────┬────────────────────────────────────┘
                         │ invokes
┌────────────────────────▼────────────────────────────────────┐
│ belt-agent (binary)                                          │
│ - git rev-parse --abbrev-ref HEAD → RunState.branch         │
│ - URI resolver (branch 認識、COMPLETED filter)               │
│ - init --inherits-from flag 処理                             │
│ - state.json 永続化（resolved_consumes 記録）                │
└────────────────────────┬────────────────────────────────────┘
                         │ calls
┌────────────────────────▼────────────────────────────────────┐
│ belt-core (pure library)                                     │
│ - URI parser (pure、環境触らない)                            │
│ - RunState 型拡張 (branch, resolved_consumes, status)        │
│ - Engine (既存 has_output gate で note 強制)                 │
│ - ArtifactRef 型 (FsPath | Uri)                              │
└─────────────────────────────────────────────────────────────┘
```

**責務分離原則**:
- URI **parse** は belt-core（pure、決定論的、無依存）
- URI **resolve** は belt-agent（環境依存、git CLI 呼び出し）
- URI **consume** は LLM / skill（content 中立）

### Data Model 拡張

#### `RunState` (`crates/belt-core/src/model.rs`)

```rust
pub struct RunState {
    pub run_id: String,
    pub pipeline_file: PathBuf,
    pub pipeline_name: String,                         // 追加: URI latest/{pipeline} 解決の比較対象
    pub branch: Option<String>,                        // 追加
    pub resolved_consumes: HashMap<String, PathBuf>,   // 追加: URI string → 解決 path
    pub args: HashMap<String, serde_json::Value>,
    pub current_phase: Option<String>,
    pub completed_phases: Vec<String>,
    pub skipped_phases: Vec<String>,
    pub phase_attempts: HashMap<String, u32>,
    pub phase_verify_passed: HashMap<String, bool>,
    pub regate_passed: HashMap<String, bool>,
    pub phase_start_times: HashMap<String, DateTime<Utc>>,
    pub status: RunStatus,                             // 追加: InProgress | Completed | Failed
}
```

`pipeline_name` は init 時に `pipeline_file` を open して `Pipeline::name` を読み取り記録する。URI resolution 時は state.json 各ファイルから直接 `pipeline_name` を取れるため、resolution loop で pipeline YAML を再 open する必要なし（性能最適化）。

`RunStatus` variant は MVP では `InProgress | Completed | Failed` の 3 種。BELT-28 の `paused: true` は boolean として別途追加される提案であり、本 spec では scope 外（衝突回避のため `Paused` variant は導入しない）。

**Backward compatibility**: serde default で `branch: None`, `resolved_consumes: HashMap::new()`, `status: RunStatus::InProgress` を既定化。旧 state.json も deserialize 可能。

#### `ArtifactRef` 拡張 (`crates/belt-core/src/model.rs`)

既存 `ArtifactRef` enum に新 variant `External` を追加する。`produces` 側の `Artifact { name, path, ... }` 型は **変更なし**（自分が生成する path は URI 不要）。URI は `consumes` 側でのみ使用する。

**既存**:
```rust
pub enum ArtifactRef {
    Named(String),                                // 同 phase の produces 参照
    Qualified { name: String, from: String },     // 別 phase の produces 参照 (from = phase_id)
}
```

**拡張後**:
```rust
pub enum ArtifactRef {
    Named(String),
    Qualified { name: String, from: String },
    External { name: String, uri: BeltUri },      // 追加: cross-run 参照
}
```

**YAML 表現**:
- Named (既存): `consumes: ["notes"]`
- Qualified (既存): `consumes: [{ name: "notes", from: "review" }]`
- External (新): `consumes: [{ name: "prior_review", uri: "belt://latest/feature-dev/notes/phase-review.md" }]`

serde-saphyr の untagged enum disambiguation は既存 `Qualified` が field 判別に依存しているため、`External` も `uri` field の存在で判別可能。既存 pattern を踏襲。

#### `BeltUri` (`crates/belt-core/src/uri.rs` 新設)

```rust
pub enum BeltUri {
    Latest { pipeline: String, path: String },
    WorkspaceLatest { branch: String, pipeline: String, path: String },
    Run { run_id: String, path: String },
}

impl BeltUri {
    pub fn parse(s: &str) -> Result<Self, UriParseError>;
    pub fn to_string(&self) -> String;
}
```

pure 実装。ファイルシステムや git に触れない。

### URI Scheme

#### Grammar

```
URI       := "belt://" Selector "/" Path
Selector  := Latest | WorkspaceLatest | Run
Latest           := "latest/" Pipeline
WorkspaceLatest  := "workspace/" Branch "/latest/" Pipeline
Run              := "run/" RunId
Pipeline  := [A-Za-z0-9_-]+
Branch    := [^/]+                 (RFC 3986 percent-encoded; "/" は %2F)
RunId     := UUIDv7 canonical string
Path      := RelativePath          (解決後 run directory root `<.belt/runs/{run_id}>/` からの相対。`..` / absolute prefix 禁止)
```

#### 具体例

| URI | 意味 |
|---|---|
| `belt://latest/feature-dev/notes/phase-review.md` | 現在 branch の COMPLETED latest feature-dev run の該当 file |
| `belt://workspace/develop/latest/feature-dev/notes/phase-review.md` | `develop` branch の COMPLETED latest feature-dev run の該当 file |
| `belt://run/01947abc-.../notes/phase-review.md` | 指定 run_id の該当 file（branch 無関係） |

#### Selector の責務分離

- **`latest/{pipeline}`**: デフォルトの isolation。現在 branch 内で完結。cross-branch 流入を防ぐ安全側デフォルト
- **`workspace/{branch}/latest/{pipeline}`**: explicit cross-branch（例: `main` の run から `develop` の前 run を参照）
- **`run/{id}`**: 完全 explicit。`--inherits-from` flag 使用時の内部表現

### Resolution ロジック

#### Init 時 resolve（唯一の resolve point）

```
belt-agent init <pipeline.yml> [--inherits-from <run_id>]
```

**手順**:

1. `git rev-parse --abbrev-ref HEAD` を実行して `RunState.branch` を設定
   - 失敗（非 git directory）: `branch = None`
   - detached HEAD（output == `"HEAD"`）: `branch = None`
   - worktree 内: 通常通り取得
2. `--inherits-from <run_id>` 指定時: そのまま escape hatch として記録。`belt://run/<run_id>/...` と同等扱い
3. Pipeline YAML を expand して `consumes` / `produces` 内の全 URI 値を収集
4. 各 URI を resolve:
   - `belt://latest/{pipeline}/...`:
     - `.belt/runs/*/state.json` を scan
     - `current_branch == Some(b)` なら `state.branch == Some(b) && pipeline_name == specified && status == Completed` で filter
     - `current_branch == None`（非 git / detached HEAD）なら **branch filter を無効化**し `pipeline_name == specified && status == Completed` のみで filter
     - UUIDv7 の lexicographic max で latest 決定 → `<.belt/runs/{run_id}>/<path>` を構築
   - `belt://workspace/{branch}/latest/{pipeline}/...`: `state.branch == Some(specified_branch)` で filter。`current_branch == None` でも指定 branch との match は成立可能。ただし git directory でない場合は init 時エラー（branch-aware URI は git 必須）
   - `belt://run/{id}/...`: `.belt/runs/{id}/` 直接参照、`state.branch` は無視
5. 各 URI の resolved absolute path を `RunState.resolved_consumes` に記録（key = URI string）
6. Resolve 失敗は init を失敗させる（consume できない pipeline は実行不可）

**Init 時 resolve の意図**:
- Snapshot 性を確保。実行中に前 run が新規 note を追加しても影響しない
- Resolved path が state.json に記録されるため、後続 step / status / verify で再 resolve 不要
- Debuggability: state.json を見れば何が consume されたか追跡可能

#### "latest" 定義

- **COMPLETED-only**: `state.json.status == Completed` のみ candidate
- **Ordering**: UUIDv7 の lexicographic order（time-ordered）。既存 `latest_run_id` と同じ logic を流用
- **同一 millisecond tie-break**: lexicographic。実用上衝突しない

#### Step 時 consume

`belt-agent step` の JSON output には既に BELT-32 で `consumes` field がある（`Named` / `Qualified` 向け、`name: <name> + resolved path` 形式）。本 spec の拡張は:

- `External { name, uri }` variant を resolve 結果として含める
- 各 consumes entry に `uri` field（存在時のみ）と、`resolved_path` field（既存通り absolute path）を付加
- 既存 `Named` / `Qualified` の JSON 形状は変更しない（追加のみ）

LLM は `resolved_path` を read するのみ。belt は mount も symlink も作らない。

### Phase 完了強制（既存 gate 再利用）

新規 gate kind / phase field を **追加しない**。既存の `has_output` gate で宣言する。

```yaml
phases:
  - id: review
    description: "Code review"
    produces:
      - name: notes
        path: "notes/phase-review.md"
        description: "Phase review narrative: decisions, concerns, directives"
    gate:
      - has_output: "notes/phase-review.md"
      - cmd: "cargo test"
```

LLM が `notes/phase-review.md` を書かない限り `has_output` gate が fail し、step が通らない。これが **決定論的な note 提出強制** の実装。

新規コードゼロ（既存 `has_output` gate を活用）。

### Note Body 規約（belt は parse しない）

本 spec に **規約として文書化** する。belt-core は content を検証しない。frontmatter parse は skill 層（`/handover` / `/feature-dev` 等）の責務。

```markdown
---
target_phase: <pipeline_id>/<phase_id>    # optional: 次 pipeline/phase への directive
kind: directive | concern | observation | decision
severity: info | warn | critical           # optional (kind=concern/observation 時)
run_id: <source run_id>                    # 参照元 run_id（MVP では LLM が step output の run_id から書き写す）
created_at: <ISO 8601>
---

# Title

Body...
```

**belt 側の扱い**:
- belt-core / belt-agent は frontmatter を **parse しない / inject しない**（質問 6 で確定した content 中立原則）
- `run_id` / `created_at` を含む全 field は LLM 責任で記述。`belt-agent step` JSON output に既に含まれる `run_id` を LLM が参照して書き写す前提
- 本 spec の範囲では belt が frontmatter に関与するのは「`belt-agent step` JSON に必要 data を渡す」までであり、生成後の検証や書き換えは行わない
- follow-up: `run_id` auto-inject mechanism は別 spec で検討（LLM の書き漏れが頻発する場合）

### Pipeline Chain（End-to-End 例）

**feature-dev.yml** (producer):
```yaml
name: feature-dev
phases:
  - id: review
    produces:
      - name: notes
        path: "notes/phase-review.md"
        description: "Review phase narrative"
    gate:
      - has_output: "notes/phase-review.md"
      - cmd: "cargo test"
  - id: done
    confirm: true
```

**debug-flow.yml** (consumer):
```yaml
name: debug-flow
phases:
  - id: rca
    consumes:
      - name: prior_review
        uri: "belt://latest/feature-dev/notes/phase-review.md"
    produces:
      - name: notes
        path: "notes/phase-rca.md"
        description: "RCA phase narrative"
    gate:
      - has_output: "notes/phase-rca.md"
```

**実行フロー**:

1. User: `belt-agent init feature-dev.yml` → branch が記録される
2. feature-dev 各 phase を進行、review phase で `notes/phase-review.md` を LLM が生成（has_output gate 通過）
3. done phase 完了で `RunState.status = Completed`
4. User: `belt-agent init debug-flow.yml`
   - URI `belt://latest/feature-dev/notes/phase-review.md` を resolve
   - 現在 branch の feature-dev COMPLETED runs を scan、UUIDv7 max を latest と判定
   - `.belt/runs/<feature_dev_run_id>/notes/phase-review.md` の absolute path を `resolved_consumes["belt://latest/feature-dev/notes/phase-review.md"]` に記録
5. rca phase で belt-agent step → consumes JSON field に resolved path が出現 → LLM がその path を read → narrative 引き継ぎ完了

### Session 分割復元（Case D の既存機構による吸収）

同一 pipeline が context 枯渇で session A → session B に分割される場合、新規機構は不要。以下の既存資産で自動カバーされる:

- **BELT-23**: `pipeline_file` の canonical absolute path 化により、別 cwd からの resume でも pipeline YAML が一意に解決
- **BELT-29**: `belt-agent status` が RunState + pipeline YAML + filesystem scan から query-time で view を合成。新 session が `.belt/runs/` を scan して `latest_run_id` を取得すれば phase 復元可能
- **BELT-30**: per-phase verify/regate JSON が保持されるため、前 session の gate 失敗理由が新 session で読み取り可能
- **本 spec**: 各 phase の `produces` で narrative note が file として残るため、新 session は `belt-agent status` + note file read で narrative context を再構築可能

したがって Case D は本 spec の新機構（URI scheme、resolved_consumes）を使わず、既存機構のみで自動的にカバーされる。skill 層での `/continue` の承認ゲートは維持される（人間の意図確認として）。

### Skill 層との link（今回の spec では仕様のみ、実装は別 ticket）

`/handover` skill が扱う `project-state.json` に以下 field を追加:

```json
{
  "version": 6,
  "belt_runs": [
    {"run_id": "01947abc-...", "pipeline": "feature-dev", "status": "Completed"}
  ],
  "phase_summaries": { ... }
}
```

- `/handover` が session 内で開始された belt runs を記録
- `/continue` が resume 時に `belt_runs` から最新 run_id を取得し、`belt-agent status --run <run_id>` で phase 復元

**本 spec のスコープ外**: 実装は別 spec / ticket で定義する。本 spec では interface の存在を言及するのみ。

### Lint Rules（belt lint 拡張）

1. **URI grammar**: `belt://` で始まる value が legal grammar に合致するか
2. **Unknown selector**: `belt://unknown/...` 等の未定義 selector を reject
3. **Path traversal**: resolved relative path に `..` や absolute prefix があれば reject
4. **Dangling reference warn**: `consumes: belt://latest/<pipeline>/<path>` で参照する path が、対応 pipeline の `produces` 宣言に存在するか（cross-pipeline dependency lint。runtime には影響しない warn）
5. **Orphan produces warn**: `produces` で宣言された path が phase gate の `has_output` で保護されていない場合 warn

既存の lint 7 項目に 5 項目追加。

## Error Handling

| Scenario | Behavior |
|---|---|
| 非 git directory で `belt://latest/...` 使用 | `branch = None`、branch filter 無効化で全 runs から latest |
| 非 git directory で `belt://workspace/{branch}/...` 使用 | Init 時エラー "branch-aware URI requires git directory" |
| Detached HEAD で `belt://latest/...` | `branch = None`、上と同じ |
| Resolve 対象 run が存在しない | Init 時エラー "no COMPLETED run of pipeline X on branch Y" |
| Resolved path にファイルが存在しない | Init 時エラー "resolved artifact missing: {abs_path}" |
| `--inherits-from <id>` の id が存在しない | Init 時エラー |
| URI parse 失敗 | Lint で検出、init でも検出 |
| Path traversal (`..` / absolute) | Parse 時エラー |
| 循環 chain (将来): A が B に depend、B が A に depend | Future: lint で cycle detection |

全エラーは `miette` diagnostic でユーザーに提示。

## Migration / Backward Compatibility

### 既存 pipeline YAML

変更不要。`consumes` / `produces` は既に Optional、URI 値は opt-in。既存の `has_output` gate / `file_exists` gate も影響なし。

### 既存 state.json

serde default で新 field を既定化:
- `pipeline_name: ""` (empty string)
- `branch: None`
- `resolved_consumes: HashMap::new()`
- `status: RunStatus::InProgress`

既存 run の扱い:
- `pipeline_name` が空文字列の state.json を resolution 時に検出した場合: `pipeline_file` を open して name を読み取り state.json を rewrite（lazy migration）
- status は completed_phases / current_phase の状態から init 時に導出:
  - 全 phases が completed_phases にあり `current_phase == None` → `Completed`
  - それ以外 → `InProgress`

init 時に migrate logic を実行。

### 既存 `/handover` `/continue` skill

本 spec では変更不要。次期 skill 改修 spec で `belt_runs: []` field を追加する。

## Testing Strategy

### Unit (belt-core)

- `BeltUri::parse` grammar matrix:
  - 正常系: 3 selector × 複数 path pattern
  - 異常系: unknown selector, empty pipeline, malformed run_id, path traversal, percent-encoding edge
- `RunState` serde backward compat:
  - 旧 state.json (branch / resolved_consumes / status なし) が deserialize 可能
  - default 値が想定通り
- `ArtifactRef` serde:
  - plain string → `FsPath`
  - `"belt://..."` string → `Uri`
  - invalid URI は parse error

### Integration (belt-agent)

- **3 selector × (成功 / 失敗)** の init 時 resolve 網羅
- **Chain end-to-end**: pipeline A run → pipeline B init で consume → step JSON に正しい path が出現 → LLM side で read 可能
- **Branch isolation**: branch X の run が branch Y の `belt://latest/...` に出現しないこと
- **`--inherits-from`** flag の escape hatch 動作
- **非 git / detached HEAD / worktree** での branch 取得挙動

### Adversarial Probes（verification contract 準拠）

- 同一 branch で複数 pipeline が並行進行中 → COMPLETED-only filter で正しく latest 決定
- 前 run が未完了状態で新 run が `belt://latest/...` consume → init エラー（COMPLETED が無い）
- UUIDv7 の秒以下衝突時の tie-break 確認
- Worktree を跨いで同じ repo の別 branch に switch した直後の init
- `.belt/runs/` に手動で削除された stale state.json がある場合の robustness
- `--inherits-from` に存在しない run_id → エラーメッセージが明確か

### Lint テスト

- 5 新規 lint rule × (pass / warn / fail) fixture

## Impact on Redesign Spec

`docs/specs/2026-04-06-belt-redesign.md` に以下の追記 / 修正が必要:

1. **責務境界の表 (L79-101, L590)**: "handover / session notes" 行を分割
   - **phase-scoped narrative (as Artifact)** → belt 側
   - **session-level narrative** → SKILL.md protocol 側
2. **RunState schema (L444-482)**: `branch`, `resolved_consumes`, `status` field を追加
3. **Non-goals**: narrative 関連の記述を更新
4. **YAML Universe (Future) (L553 付近)**: 新設 `belt://` URI scheme と将来の cross-repo 構想との関係を注記

## Future Work（Follow-up）

以下は本 spec の範囲外。別 ticket として追跡する。

- `/handover` skill 改修: `belt_runs: []` field 追加と belt-agent status 連携
- `/continue` skill 改修: `belt_runs` を使った自動 phase 復元
- `PreCompact` hook 連携: 自動 snapshot trigger（Claude Code hook 設定側の変更）
- `belt://repo/<url>/...` cross-repo URI scheme（YAML Universe 関連）
- `belt-agent status` の note structured view（frontmatter parse を含む場合は belt-core 責務境界の再検討要）
- Circular chain detection lint rule
- `run_id` frontmatter の belt-agent による auto-inject mechanism
- TUI 可視化（`belt-tui` 独立 crate の責務）

## References

- [BELT-20](https://linear.app/neko-neko/issue/BELT-20): master tracking
- [BELT-24](https://linear.app/neko-neko/issue/BELT-24): Context neutrality 原則
- [BELT-29](https://linear.app/neko-neko/issue/BELT-29): Status enrichment（context recovery foundation）
- [BELT-30](https://linear.app/neko-neko/issue/BELT-30): Per-phase verify/regate JSON（file-based data flow）
- [BELT-32](https://linear.app/neko-neko/issue/BELT-32): Invoker + Artifact first-class
- `docs/specs/2026-04-06-belt-redesign.md`: 責務境界 / RunState schema
- `docs/specs/2026-04-07-belt-regate-auto-execution.md`: context neutrality 実例
- `docs/specs/2026-04-07-belt-status-enrichment.md`: context-neutral view computation
- `~/.dotfiles/claude/skills/handover/SKILL.md`: 現行 session narrative schema v5
- `~/.dotfiles/claude/skills/continue/SKILL.md`: 現行 resume protocol
