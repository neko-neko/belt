# belt://current URI Variant + SKILL Path Knowledge Elimination — Design

**Status**: Draft
**Date**: 2026-04-18
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)

## Summary

`plugins/` 配下の SKILL.md / criteria/*.md / agents/*.md / pipeline.yml に散在する `.belt/runs/{run_id}/...` 形式の path リテラル (87 ファイル) を、新規 URI variant `belt://current/<path>` を中心とした抽象化で **belt 一時ディレクトリの物理 layout 知識を SKILL 層から完全に排除する**。

belt-core は path semantics (notes/ / review/ / handover.md 等) を一切知らず、URI scheme 解釈と `<run_dir>/<path>` への純粋な resolve のみを担う。SKILL は artifact name と `belt://current/...` URI で path を表現し、agents/sub-skill は path 知識ゼロで動作する (orchestrator skill が runtime arg で物理 path を渡す)。

後方互換性は意図的に放棄し、`{run_id}` template と raw `.belt/runs/...` リテラルは engine + lint から完全削除する。既存 plugins pipeline (feature-dev / bug-fix / handover/checkpoint / code-review / spec-review) は同 PR / 同 merge 内で一括 migrate する。

## Goals

- SKILL.md / criteria/*.md / agents/*.md から `.belt/runs/...` path リテラル を **完全削除** (ハードコードゼロ)
- pipeline.yml の `{run_id}` template と `.belt/runs/...` リテラルを **完全削除**、`belt://current/...` URI に置換
- belt-core が **path semantics を知らない** (kind / notes / review 等の概念を持たない)
- agents/*.md は **path 知識ゼロ** で動作 (`output_path` runtime arg で受け取る)
- `belt://current/<path>` URI を新規 variant として `BeltUri` に追加し、既存の Latest / WorkspaceLatest / Run と同列に扱う
- `belt-agent locate <uri>` コマンドで URI → 物理 path 解決を一手に提供
- `belt-agent status` の `phases[].produces[]` shape を `{ name, uri, resolved_path, exists }` に変更 (`path` field 廃止)
- lint で `.belt/runs/` リテラル + `{run_id}` template の混入を **error 化**
- plugins 既存 pipeline (feature-dev / bug-fix / handover/checkpoint) を同 PR 内で URI 化
- code-review / spec-review pipeline phase の produces に **全 observation findings (findings-security / findings-test / 等) を明示宣言**

## Non-Goals

- 後方互換性の維持 (raw path / `{run_id}` template の dual-support 期間を設けない — 一括置換)
- belt-core が artifact `kind` (narrative / review / handover 等) を解釈する責務を持つこと (Tiny by Constraint 違反、ユーザー判断で却下)
- `belt://current/` 以外の新規 URI scheme variant 追加 (Latest / WorkspaceLatest / Run の現行 3 種は不変)
- `belt-agent write <uri> < stdin` のような I/O 仲介コマンド (Tiny by Constraint 違反、agents は通常の Write tool で書く)
- code-review / spec-review pipeline の sub-pipeline 化 (現行 skill 内 dispatch を保持、案 1 採用根拠は本 design Section 6 参照)
- pipeline.yml の `produces` field schema の再設計 (`name` / `path` / `description` / `when` の現行 4 field を維持、`kind` field は追加しない)
- belt-core への並列実行機構 (sub-pipeline は引き続き sequential のみ)
- `Cargo.toml` workspace dependencies の追加 (`belt-core::uri` は std + serde + thiserror のみで実装)
- README / CHANGELOG の本 spec 個別記載 (release note では `BELT-20` リファクタリングの一部として包括記述)

## Background

### 現状の重複構造

`plugins/` 配下を `Grep "\.belt/runs"` で検索すると **87 ファイル** にヒット。重複の系統を分類すると:

| 出現箇所 | 例 | 性質 |
|---|---|---|
| `pipeline.yml` の `produces[].path` | `path: ".belt/runs/{run_id}/notes/phase-design.md"` | SSOT 候補 (`{run_id}` は engine が runtime substitute) |
| `pipeline.yml` の `gate.file_exists` | `file_exists: ".belt/runs/{run_id}/notes/phase-design.md"` | SSOT (同 phase 内で produces と重複) |
| `SKILL.md` の Narrative Notes 節 | "narrative note は `.belt/runs/{run_id}/notes/phase-<id>.md`" | 説明文 (pipeline.yml の再記述) |
| `criteria/*.md` の verify ステップ | "Verify file exists at `.belt/runs/<run_id>/notes/phase-X.md`" | 読み取り検証 (gate.file_exists と重複) |
| `agents/*.md` の output 指示 | "Write findings to `.belt/runs/{run_id}/review/findings-security.json`" | 書き込み先 (pipeline.yml に宣言なし、隠れた middle layer) |
| `references/*-convention.md` | "`.belt/runs/{run_id}/notes/phase-{phase_id}.md`" | convention 文書 |

### 重複の本質的な問題

1. **多重 SSOT**: pipeline.yml と SKILL.md / criteria / agents が同じ path を重複宣言。pipeline.yml の path を変更すると 5〜10 箇所の同期修正が必要
2. **隠れた書き込み先**: agents が書く findings-X.json は pipeline.yml の `produces` に宣言されていない。`belt-agent status` で path を引けない
3. **`{run_id}` template の漏れ**: SKILL.md / agents は LLM が手で `{run_id}` を substitute する前提。drift が頻発 (memory `feedback_plan_test_code_grep_validation.md` が示す類似事例多数)
4. **layout knowledge の散在**: `notes/` / `review/` / `handover.md` といった ディレクトリ規約が plugins / docs / belt-core に分散

### 現状の belt-agent CLI 出力能力

- `belt-agent status` の `StatusView`: `run_id, pipeline, pipeline_file, version, args, status, current_phase, progress, phases[]`
  - **`run_dir` 相当の field なし**
  - `phases[].produces[].path` は **declared raw** (`{run_id}` template substitute は view.rs では行われない、要 verify) 
  - `phases[].produces[].resolved_path` は **既存 fs ファイルのみ** 解決 (書き込み先 path には使えない)
- `belt-agent next` / `init` の `phase.produces[].path`: engine の `expand_run_id` (engine.rs:170-172) で `{run_id}` を substitute 済み
- view.rs vs engine.rs の **shape asymmetry**: 同じ produces が status (raw) と next (substituted) で異なる shape を返す

### 既存の `belt://` URI scheme

`crates/belt-core/src/uri.rs` に既に 3 variant 実装済み:
- `belt://latest/{pipeline}/<path>` — current branch の latest COMPLETED run
- `belt://workspace/{branch}/latest/{pipeline}/<path>` — explicit branch の latest COMPLETED run
- `belt://run/{run_id}/<path>` — explicit run_id

これら 3 variant は **read 側 (External consumes)** にのみ使われており、write 側には使われていない。本 design は `belt://current/<path>` variant を新設し、URI scheme を write 側にも拡張する。

### 既存の `expand_run_id` 機構

`crates/belt-core/src/engine.rs:440-468` に:
- `expand_run_id(s: &str, run_id: &str) -> String` — string 内の `{run_id}` を substitute
- `expand_gate_run_id(gate: &mut [GateCheck], run_id: &str)` — `FileExists` / `Cmd` の `{run_id}` を substitute
- `next_phase_info` で `phase.produces[].path` と `phase.gate` の両方に substitute を適用
- regate 実行時 (belt-agent main.rs:571) も同様に substitute

これらは本 design で全面削除し、URI 解決に統合する。

### 既存の Resolver 実装

`crates/belt-agent/src/resolver.rs` の `Resolver` struct が既に 3 variant を解決:
- `resolve_run` — `belt://run/<id>/<path>`
- `resolve_latest` — `belt://latest/...` (current branch filter)
- `resolve_latest` (explicit_branch) — `belt://workspace/...`

本 design は `Current` variant を resolver に追加するのみ。新 helper `resolve_current` は belt-agent invocation context から run_id (`--run` arg または latest in_progress) を取得し、`<run_dir>/<path>` に解決する。

## Design

### 1. URI scheme: `belt://current/<path>`

#### Variant 定義 (`crates/belt-core/src/uri.rs`)

```rust
pub enum BeltUri {
    Latest { pipeline: String, path: String },
    WorkspaceLatest { branch: String, pipeline: String, path: String },
    Run { run_id: String, path: String },
    /// `belt://current/<path>` — runtime invocation context の current run
    /// (`--run` 指定、未指定なら latest run) の `<run_dir>/<path>` に解決される。
    /// pipeline.yml の `produces[].path` / `gate.file_exists` で書き込み先 +
    /// 読み取り先の宣言に使用。
    Current { path: String },
}
```

#### Parser 追加 (`BeltUri::parse`)

```rust
if let Some(r) = rest.strip_prefix("current/") {
    let path = r;
    if path.is_empty() {
        return Err(UriParseError::EmptyPath { uri: s.to_string() });
    }
    validate_path(path, s)?;  // 既存の `..` / leading `/` reject ロジック流用
    return Ok(BeltUri::Current { path: path.to_string() });
}
```

#### Display / Serialize / Deserialize

既存 3 variant と同形式:
```rust
BeltUri::Current { path } => write!(f, "belt://current/{path}"),
```

`Serialize` / `Deserialize` は `BeltUri::parse` / `Display` 経由で自動。

#### `Current` の path validation

- `..` segment 禁止 (path traversal)
- leading `/` 禁止 (absolute path)
- 空 path 禁止 (`belt://current/` のみは error)
- glob (`*`, `?`, `[`) は許容 (resolver で展開)

### 2. Resolver: `Current` 解決 (`crates/belt-agent/src/resolver.rs`)

```rust
impl Resolver<'_> {
    pub(crate) fn resolve(&self, uri: &BeltUri) -> Result<PathBuf, ResolveError> {
        match uri {
            BeltUri::Run { run_id, path } => self.resolve_run(run_id, path),
            BeltUri::Latest { pipeline, path } => self.resolve_latest(pipeline, path, None),
            BeltUri::WorkspaceLatest { branch, pipeline, path } => { ... }
            BeltUri::Current { path } => self.resolve_current(path),
        }
    }

    fn resolve_current(&self, path: &str) -> Result<PathBuf, ResolveError> {
        let run_id = self.current_run_id.as_ref()
            .ok_or(ResolveError::NoCurrentRun)?;
        let run_dir = self.belt_dir.join("runs").join(run_id);
        if !run_dir.is_dir() {
            return Err(ResolveError::RunNotFound { run_id: run_id.clone() });
        }
        // Note: existence check は呼び出し元 (locate コマンド) で `exists` field
        // として返す。resolver 自体は path を返すだけで existence は assert しない
        // (write target の場合まだファイルが無いケースに対応するため)
        Ok(run_dir.join(path))
    }
}
```

#### `Resolver` struct への field 追加

```rust
pub(crate) struct Resolver<'a> {
    pub belt_dir: &'a Path,
    pub current_branch: Option<String>,
    pub current_run_id: Option<String>,  // 新規: --run / latest 解決済み
}
```

`current_run_id` は CLI invocation 時点で `Engine::latest_run_id()` または `--run` arg から決定し、Resolver 構築時に渡す。

#### `ResolveError` 拡張

```rust
pub(crate) enum ResolveError {
    // 既存 variants ...
    /// `belt://current/...` URI を解決しようとしたが invocation context に
    /// run_id が無い (`--run` 未指定 + latest run 不在)。
    #[error("belt://current/ requires a current run (none found, pass --run <id>)")]
    NoCurrentRun,
}
```

#### resolver の existence check policy

既存 `resolve_run` / `resolve_latest` は `if !abs.exists() { return ArtifactMissing }` で existence を assert する。これは **read 側用 (External consumes)** の動作。

`belt://current/...` は **write 側にも使う** ため、resolver 自体は existence を assert せず、呼び出し元 (locate / status) が `exists: bool` field として返す。既存 `resolve_run` / `resolve_latest` は read 側用途のままで existence assert を継続。

### 3. CLI: `belt-agent locate <uri>`

#### コマンド定義

```rust
#[derive(Subcommand)]
enum Command {
    // 既存: Init, Next, Verify, Regate, Step, Status
    /// Resolve a belt:// URI to its filesystem path
    Locate {
        /// belt:// URI to resolve
        uri: String,
        /// Run ID (default: latest)
        #[arg(long)]
        run: Option<String>,
    },
}
```

#### 出力 shape (JSON pretty)

```json
{
  "uri": "belt://current/notes/phase-design.md",
  "path": "/Users/x/proj/.belt/runs/01J.../notes/phase-design.md",
  "exists": true
}
```

- `uri`: 入力 URI そのまま (echo)
- `path`: 絶対 path (resolver 解決結果)
- `exists`: `std::fs::metadata` 判定。glob URI の場合は match 数 >= 1
- 解決失敗時: stderr に miette diagnostic、exit code 非 0

#### 実装ポイント

- `Current` variant 解決時に `--run` を `Resolver::current_run_id` に渡す
- glob URI (`belt://current/notes/phase-*.md`) は `Resolver::resolve_current` 内で `glob::glob` 展開 (既存 view.rs ロジック流用)
- glob match ゼロは `exists: false`、`path` は declared base (`<run_dir>/<glob pattern>`)

### 4. Engine: `{run_id}` template の削除

#### 削除対象

| 場所 | 内容 |
|---|---|
| `crates/belt-core/src/engine.rs:440-442` | `fn expand_run_id` 削除 |
| `crates/belt-core/src/engine.rs:444-468` | `pub fn expand_gate_run_id` 削除 |
| `crates/belt-core/src/engine.rs:170-173` | `next_phase_info` 内の `for artifact in &mut phase.produces { artifact.path = expand_run_id(...) }` と `expand_gate_run_id(&mut phase.gate, ...)` 削除 |
| `crates/belt-agent/src/main.rs:571` | regate 内の `expand_gate_run_id` 呼び出し削除 |
| `crates/belt-core/tests/engine_test.rs:2213` | `{run_id} must be substituted into file_exists gate path` test 削除 + URI 解決 test 追加 |
| `crates/belt-core/tests/engine_test.rs:2267` | `{run_id} must be substituted into produces[*].path` test 削除 + URI test に置換 |
| `crates/belt-agent/tests/cli_test.rs:558` | `regate_substitutes_run_id_in_target_gate` 削除 + URI 解決 test 追加 |

#### 削除後の動作

- pipeline.yml の `produces[].path` は **declared そのまま** (`belt://current/notes/phase-design.md`) を保持
- `belt-agent status` / `init` / `next` の出力で URI を `resolved_path` field に解決した値を併記
- gate executor (本 design Section 5) が URI を resolver で解決して file_exists 判定

### 5. Gate executor: URI 解決の組み込み (`crates/belt-core/src/gate.rs`)

#### 現状

`execute_gate` の `FileExists` 分岐は `glob::glob(file_exists_pattern)` をそのまま実行。`{run_id}` は engine 側で事前 substitute されている前提。

#### 変更後

`FileExists` の string が `belt://` で始まる場合、URI として parse → resolver で解決した結果の path を `glob::glob` に渡す:

```rust
GateCheck::FileExists { file_exists } => {
    let resolved = if file_exists.starts_with("belt://") {
        let uri = BeltUri::parse(file_exists)?;
        resolver.resolve(&uri)?.to_string_lossy().to_string()
    } else {
        file_exists.clone()  // raw path (domain artifact 等) はそのまま
    };
    glob::glob(&resolved)?...
}
```

#### Resolver の渡し方

`gate::execute_gates` のシグネチャ拡張:
```rust
pub fn execute_gates(
    gates: &[GateCheck],
    work_dir: &Path,
    output_dir: &Path,
    resolver: &Resolver,  // 新規
) -> Vec<GateResult>
```

`belt-agent` 側で `Resolver` を構築済みなので、cmd_verify / cmd_regate で渡す。`belt-core` の Resolver 依存は new circular issue を生むため、`Resolver` trait を `belt-core::gate` に定義し、`belt-agent::resolver::Resolver` が実装する形にする (DI pattern):

```rust
// belt-core/src/gate.rs
pub trait UriResolver {
    fn resolve(&self, uri: &str) -> Result<PathBuf, GateError>;
}

// belt-agent/src/resolver.rs
impl belt_core::gate::UriResolver for Resolver<'_> {
    fn resolve(&self, uri: &str) -> Result<PathBuf, GateError> {
        let parsed = BeltUri::parse(uri).map_err(|e| GateError::UriParse(e.to_string()))?;
        self.resolve(&parsed).map_err(|e| GateError::UriResolve(e.to_string()))
    }
}
```

これにより belt-core は belt-agent に依存しない (循環回避)。

### 6. status JSON shape 変更 (`crates/belt-core/src/view.rs` + `crates/belt-agent/src/main.rs`)

#### 変更前

```json
"phases": [{
  "produces": [{
    "name": "design_notes",
    "path": ".belt/runs/{run_id}/notes/phase-design.md",
    "exists": true,
    "resolved_path": "/abs/.belt/runs/01J.../notes/phase-design.md"
  }]
}]
```

#### 変更後

```json
"phases": [{
  "produces": [{
    "name": "design_notes",
    "uri": "belt://current/notes/phase-design.md",
    "resolved_path": "/abs/.belt/runs/01J.../notes/phase-design.md",
    "exists": true,
    "description": "..."
  }]
}]
```

- `path` field を **削除**、`uri` field に置換
- `uri` は pipeline.yml の declared 値そのまま (URI 形式必須、raw path は domain artifact のみ別 field で扱う検討要)
- `resolved_path` は resolver で URI を解決した絶対 path
- `exists` は `resolved_path` の fs 判定

#### domain artifact の扱い

`docs/features/*/design.md` のような **belt 外 path** は URI ではなく raw path。これは pipeline.yml で:

```yaml
produces:
  - name: design_doc
    path: "docs/features/*/design.md"  # raw path 継続サポート
```

raw path の判別: `belt://` prefix 不在で URI parse 失敗。view 側は:
- `belt://...` → URI 解決して `uri` + `resolved_path` 両方 emit
- raw path → `uri` field omit、`path` field を残す形 (混合 shape)

または **シンプル化**: 全 produces を URI 必須 (`docs/features/*/design.md` は belt-core が「raw path URI variant」として扱うか、`belt://workspace-relative/<path>` 等の domain URI variant を新設)。

→ **採用案**: raw path 継続サポート。判別ルール: declared 文字列が `belt://` prefix を持つ場合 → `uri` field に格納、URI parse + resolver で `resolved_path` を埋める。`belt://` prefix を持たない場合 → `path` field に格納、glob 解決した値を `resolved_path` に埋める。

```json
{ "name": "design_doc", "path": "docs/features/*/design.md", "exists": true, "resolved_path": "docs/features/2026-04-18-x/design.md" }
{ "name": "design_notes", "uri": "belt://current/notes/phase-design.md", "exists": true, "resolved_path": "/abs/.belt/.../phase-design.md" }
```

`uri` と `path` は mutually exclusive (両方同時には emit しない)。`resolved_path` / `exists` / `description` は両 shape 共通。

### 7. lint: path リテラル + `{run_id}` template 禁止 (`crates/belt-core/src/lint.rs`)

#### 新 lint rule: `check_belt_runs_literal`

pipeline.yml 全 phase の以下 field を walk:
- `produces[].path`
- `gate.file_exists`
- `gate.cmd`
- `output_dir`

各 string 値に対して:
- `.belt/runs/` を含む → error: `"raw '.belt/runs/' literal forbidden, use belt://current/<path> URI"`
- `{run_id}` を含む → error: `"{run_id} template forbidden, use belt://current/<path> URI"`

#### 例外

- `docs/testing/lock-ledger.md` 等の文書ファイル中の例示は対象外 (lint scope は pipeline.yml のみ)
- `crates/belt-core/tests/fixtures/*.yml` の test fixture は除外 (test 自身が古い shape を扱う)

### 8. Plugin pipeline.yml 一括 URI 化

#### `plugins/belt/skills/feature-dev/pipeline.yml`

変更例 (design phase):

```yaml
- id: design
  description: "Generate design document via interactive brainstorming"
  invoke:
    skill: /brainstorming
  produces:
    - name: design_doc
      path: "docs/features/*/design.md"  # raw 継続 (domain artifact)
      description: "Design document"
    - name: design_notes
      path: "belt://current/notes/phase-design.md"  # 新 URI
      description: "Phase narrative"
  gate:
    - file_exists: "docs/features/*/design.md"
    - file_exists: "belt://current/notes/phase-design.md"
  validate: ./criteria/design.md
  confirm: true
  max_retries: 3
```

10 phase 全ての `belt://current/...` 該当箇所 (design / plan / execute / code-review / monkey-test / dogfood の narrative + monkey-test / dogfood の domain output 等) を URI 化。

#### `plugins/belt/skills/bug-fix/pipeline.yml`

同様に 9 phase (rca / fix-plan / fix-plan-review / pre-execute-handover / execute / code-review / monkey-test / dogfood / integrate) を URI 化。

#### `plugins/belt/skills/handover/checkpoint.yml`

```yaml
phases:
  - id: checkpoint
    description: >-
      Context checkpoint before execute. Run `/belt:handover`, then `/clear`,
      then `/belt:resume` in a new session. The gate passes once the handover
      note exists.
    confirm: true
    gate:
      - file_exists: "belt://current/handover.md"
```

### 9. code-review / spec-review pipeline phase の produces 拡張

#### 現状

`/belt:code-review` skill は内部で 4 observation agents を dispatch し、各 agent が独自 path に findings-X.json を書く。pipeline.yml の `produces` には宣言なし。

#### 変更後 (案 1 採用)

`feature-dev/pipeline.yml` の code-review phase:

```yaml
- id: code-review
  description: "Multi-perspective code review"
  invoke:
    skill: /belt:code-review
    args:
      codex: "args.codex"
  consumes:
    - design_doc
    - plan_doc
    - design_notes
    - plan_notes
    - execute_notes
  produces:
    - name: findings-security
      path: "belt://current/review/findings-security.json"
    - name: findings-test
      path: "belt://current/review/findings-test.json"
    - name: findings-ai-antipattern
      path: "belt://current/review/findings-ai-antipattern.json"
    - name: findings-cross-cutting
      path: "belt://current/review/findings-cross-cutting.json"
    - name: findings-codex
      path: "belt://current/review/findings-codex.json"
      when: "args.codex"
    - name: findings
      path: "belt://current/review/findings.json"  # merged
    - name: code_review_notes
      path: "belt://current/notes/phase-code-review.md"
  gate:
    - file_exists: "belt://current/notes/phase-code-review.md"
    - file_exists: "belt://current/review/findings.json"
  validate: ./criteria/code-review.md
  regate: [execute]
  confirm: true
  max_retries: 3
```

`bug-fix/pipeline.yml` の code-review / fix-plan-review phase / spec-review phase も同様に observation findings を全列挙。

`when: "args.codex"` で findings-codex を conditional artifact 化 (既存 `evaluate_when` 機構流用)。

### 10. SKILL.md / criteria/*.md / agents/*.md からの path 削除

#### SKILL.md (feature-dev / bug-fix / handover / resume)

- `feature-dev/SKILL.md` の "Narrative Notes" 節 (line 46-62) から `.belt/runs/{run_id}/notes/phase-<id>.md` 言及を削除。代わりに "narrative note paths are declared in pipeline.yml as `belt://current/...` URIs and resolved via `belt-agent status`" のような記述に
- `bug-fix/SKILL.md` 同様
- `handover/SKILL.md` の workflow / Schema / Step detail から `.belt/runs/<run_id>/handover.md` 削除。"Write the file at the path resolved from `belt://current/handover.md`" 等
- `resume/SKILL.md` の Preconditions table の `.belt/runs/<run_id>/handover.md` を `belt://current/handover.md` に置換 + recovery 説明簡素化

#### criteria/*.md (feature-dev / bug-fix の各 phase)

`code-review.md` の例:
```markdown
### CODE-REVIEW-05: Narrative note captures review findings and directives
- **verification**:
  1. Read `belt-agent status` output, locate the `code_review_notes` artifact's resolved_path
  2. Read the file at resolved_path
  3. Verify frontmatter contains `phase: code-review` and `run_id: <run_id>`
  4. Verify 4 required sections exist: ## Decisions, ## Concerns, ## Directives, ## Observations
  ...
- **depends_on_artifacts**: [code_review_notes]  # artifact name 参照
```

`depends_on_artifacts` の値は path から **artifact name** に変更。

#### agents/*.md (security-reviewer / test-reviewer / ai-antipattern-reviewer / cross-cutting-reviewer / cross-cutting-spec-reviewer / feasibility-reviewer / ui-design-reviewer)

`security-reviewer.md` の "Output Format" 節:

```markdown
## Output Format

Write findings to the path provided in your prompt's `output_path` field:

\`\`\`json
{
  "observation": "security",
  "findings": [...]
}
\`\`\`

The orchestrator skill resolves the artifact path from `belt-agent status` and
passes it to you as `output_path`. Do not construct the path yourself.
```

7 agent 全てを同形式に書き換え。

#### `references/narrative-convention.md`

"Path" 節 (line 9-17) を URI 表現に書き換え:

```markdown
## Path

Each phase's narrative note is declared in pipeline.yml as a `produces` artifact:

\`\`\`yaml
produces:
  - name: design_notes
    path: "belt://current/notes/phase-design.md"
\`\`\`

Resolve the physical path via `belt-agent status` (read `phases[].produces[].resolved_path`)
or `belt-agent locate belt://current/notes/phase-design.md`.

Convention: artifacts named `<id>_notes` use the `belt://current/notes/phase-<id>.md`
URI by convention. belt-core does not enforce this — it is owned by the SKILL layer.
```

### 11. lock test 全更新

#### `crates/belt-core/tests/feature_dev_refresh.rs`

- 既存 phase 順序 / phase count assertion は不変 (URI 化は pipeline 構造を変えない)
- `produces[].path` の値を URI 形式で assert する test を追加
- `gate.file_exists` の URI prefix を assert
- code-review phase の produces count を 4 (現行) → 7 (findings-X 5 + merged + notes) に更新

#### `crates/belt-core/tests/bug_fix_refresh.rs`

同様に produces 拡張に追従。

#### `crates/belt-core/tests/review_skills_refresh.rs`

- `REVIEW_SKILLS` constant の各 entry の `produces` 列挙を更新
- `belt:code-review` / `belt:spec-review` skill 配下の dispatch 確認を維持

#### `crates/belt-core/tests/lint_test.rs`

- 新 lint rule `check_belt_runs_literal` の test を追加
  - PASS case: `belt://current/notes/phase-design.md` を含む pipeline
  - FAIL case 1: `.belt/runs/{run_id}/notes/phase-design.md` を含む pipeline → error
  - FAIL case 2: `{run_id}` template を含む URI → error

#### `crates/belt-core/tests/engine_test.rs`

- `{run_id}` substitute 関連 test 削除
- URI 解決 test 追加 (`belt://current/...` を含む pipeline で `next_phase_info` が URI を保持して返す確認、resolved_path は belt-agent 側で行う)

#### `crates/belt-agent/tests/cli_test.rs` / `e2e_test.rs`

- `regate_substitutes_run_id_in_target_gate` 削除
- 新 `locate` コマンドの integration test 追加
  - `belt://current/notes/phase-design.md` の解決
  - `--run` 指定なしで latest 解決
  - 不在 run_dir で error
  - glob URI の解決
- status JSON shape 変更の verification (`uri` field 存在、`resolved_path` 存在)

### 12. protocol/SKILL.md に "Path Resolution" 節追加

`plugins/belt-agent/skills/protocol/SKILL.md` に:

```markdown
## Path Resolution

belt does not expose physical paths to skill authors. All artifact paths are
declared in pipeline.yml as `belt://current/<path>` URIs (or raw paths for
domain artifacts under `docs/`, `src/`, etc.).

### Reading paths

To get the physical path of an artifact:

1. Call `belt-agent status` and read `phases[].produces[].resolved_path`
2. Or call `belt-agent locate <uri>` for direct URI resolution

### Passing paths to subagents

When dispatching a subagent (Task tool), the orchestrator skill MUST resolve
the URI to a physical path and pass it as `output_path` in the subagent's
prompt. Subagents do not see URIs and do not call `belt-agent locate`
themselves — they receive a concrete path.

### Forbidden patterns

- Hardcoding `.belt/runs/<run_id>/...` literals in SKILL.md, criteria/*.md,
  or agents/*.md (lint enforces this in pipeline.yml)
- `{run_id}` template strings (removed from belt-core; lint rejects them)
- Constructing paths inside agent prompts using string interpolation
```

## Contract changes

### Removed: `{run_id}` template support

- `engine::expand_run_id` / `engine::expand_gate_run_id` 関数削除
- `next_phase_info` の substitute logic 削除
- `belt-agent regate` の substitute logic 削除

### Removed: status JSON `phases[].produces[].path` field (URI 形式の場合)

- URI 形式の produces は `uri` field で出力
- raw path (domain) のみ `path` field を保持
- 既存 consumer (skill / orchestrator) は両 field 対応に更新

### Added: `BeltUri::Current` variant

- 新 URI scheme `belt://current/<path>`
- parser / Display / Serialize / Deserialize 対応

### Added: `belt-agent locate <uri>` command

- read/write 両用
- `--run` で run 指定可
- glob URI 対応

### Added: lint rule `check_belt_runs_literal`

- `.belt/runs/` リテラル禁止
- `{run_id}` template 禁止
- pipeline.yml scope のみ (test fixture / docs は除外)

### Added: `gate::UriResolver` trait

- belt-core ↔ belt-agent の循環を回避する DI

### Changed: agents/*.md prompt convention

- "Write findings to `<hardcoded path>`" → "Write findings to the path in `output_path`"
- 7 agent 全て一括書き換え

## Impact analysis

| File | 変更内容 |
|---|---|
| `crates/belt-core/src/uri.rs` | `BeltUri::Current` variant 追加 + parser + Display |
| `crates/belt-core/src/engine.rs` | `expand_run_id` / `expand_gate_run_id` 削除、関連呼び出し削除 |
| `crates/belt-core/src/gate.rs` | `UriResolver` trait 追加、`execute_gates` シグネチャ拡張 |
| `crates/belt-core/src/view.rs` | `ResolvedArtifact` の field を `path` → `uri` + `resolved_path` に変更 (raw path 継続 fallback) |
| `crates/belt-core/src/lint.rs` | `check_belt_runs_literal` rule 追加 |
| `crates/belt-agent/src/resolver.rs` | `resolve_current` 追加、`Resolver` struct に `current_run_id` 追加 |
| `crates/belt-agent/src/main.rs` | `Locate` subcommand 追加、`Resolver` 構築時に `current_run_id` を渡す、regate の `expand_gate_run_id` 削除、status output で URI emit |
| `crates/belt-core/tests/uri_test.rs` (新規) | `BeltUri::Current` parse / Display / glob / validation test |
| `crates/belt-agent/tests/cli_test.rs` | `locate` test 追加、`regate_substitutes_run_id_in_target_gate` 削除、status URI shape test |
| `crates/belt-agent/tests/e2e_test.rs` | URI resolution e2e test 追加 |
| `crates/belt-core/tests/engine_test.rs` | `{run_id}` substitute test 削除、URI test 追加 |
| `crates/belt-core/tests/lint_test.rs` | `check_belt_runs_literal` test 追加 |
| `crates/belt-core/tests/feature_dev_refresh.rs` | produces URI assertion 追加、code-review produces 7 件 assertion |
| `crates/belt-core/tests/bug_fix_refresh.rs` | 同上 |
| `crates/belt-core/tests/review_skills_refresh.rs` | REVIEW_SKILLS の produces shape 更新 |
| `plugins/belt/skills/feature-dev/pipeline.yml` | 全 phase の path を URI 化、code-review produces 拡張 |
| `plugins/belt/skills/bug-fix/pipeline.yml` | 同上 |
| `plugins/belt/skills/handover/checkpoint.yml` | gate URI 化 |
| `plugins/belt/skills/feature-dev/SKILL.md` | Narrative Notes 節 path 削除、status 経由 resolve に書き換え |
| `plugins/belt/skills/bug-fix/SKILL.md` | 同上 |
| `plugins/belt/skills/handover/SKILL.md` | path 削除、URI 化 |
| `plugins/belt/skills/resume/SKILL.md` | Preconditions table 更新 |
| `plugins/belt/skills/code-review/SKILL.md` | path リテラル削除、orchestrator が `belt-agent status` で path 取得 → agents prompt に `output_path` 埋め込み の手順追加 |
| `plugins/belt/skills/spec-review/SKILL.md` | 同上 |
| `plugins/belt/skills/feature-dev/criteria/*.md` (6 file) | path 言及削除、artifact name 参照に変更 |
| `plugins/belt/skills/bug-fix/criteria/*.md` (6 file) | 同上 |
| `plugins/belt/agents/*.md` (7 file) | "Write to <hardcoded>" → "Write to the path in output_path" |
| `plugins/belt-agent/skills/protocol/SKILL.md` | "Path Resolution" 節追加 |
| `plugins/belt-agent/references/narrative-convention.md` | Path 節を URI 表現に書き換え |
| `plugins/belt/skills/feature-dev/references/path-convention.md` | `.belt/runs/*/review/findings.json` 言及を URI に書き換え |
| `plugins/belt-agent/skills/protocol/references/resume-mode.md` | `.belt/runs/<id>/handover.md` を `belt://run/<id>/handover.md` または status 経由に書き換え |
| `docs/testing/lock-ledger.md` | review_skills_refresh / feature_dev_refresh / bug_fix_refresh entry の produces shape を URI 形式に追記 |
| `docs/testing/cli-behavior/belt-agent.yml` | `locate` scenario 追加、status の URI shape scenario 追加 |
| `docs/testing/cli-behavior/belt-core.yml` | URI parse scenario 追加 |
| `Cargo.lock` | 変更なし (新規依存追加なし) |

## Migration sequence

後方互換性なしの一括置換のため、以下を **同 PR / 同 merge** で実施。

1. **belt-core uri.rs**: `BeltUri::Current` variant 追加 (TDD: 失敗 test → impl → green)
2. **belt-core gate.rs**: `UriResolver` trait 追加、`execute_gates` シグネチャ拡張 (test fixture 含む既存 caller 全更新)
3. **belt-agent resolver.rs**: `resolve_current` 実装、`Resolver` struct に `current_run_id` 追加 (impl belt_core::gate::UriResolver も併設)
4. **belt-agent main.rs**: `Locate` subcommand 追加、cmd_verify / cmd_regate に Resolver 渡し
5. **belt-core engine.rs**: `expand_run_id` / `expand_gate_run_id` 削除、`next_phase_info` の関連 logic 削除
6. **belt-core view.rs + belt-agent main.rs**: status / next / init JSON の `path` → `uri` + `resolved_path` shape 変更
7. **belt-core lint.rs**: `check_belt_runs_literal` rule 追加
8. **plugins/belt/skills/handover/checkpoint.yml**: URI 化
9. **plugins/belt/skills/feature-dev/pipeline.yml**: 全 path URI 化、code-review produces 拡張
10. **plugins/belt/skills/bug-fix/pipeline.yml**: 同上
11. **plugins/belt/skills/feature-dev/SKILL.md, bug-fix/SKILL.md, handover/SKILL.md, resume/SKILL.md**: path リテラル削除
12. **plugins/belt/skills/code-review/SKILL.md, spec-review/SKILL.md**: orchestrator workflow 更新 (agents への output_path 渡し手順)
13. **plugins/belt/skills/feature-dev/criteria/*.md, bug-fix/criteria/*.md**: artifact name 参照化
14. **plugins/belt/agents/*.md** (7 file): output_path arg pattern に書き換え
15. **plugins/belt-agent/skills/protocol/SKILL.md**: "Path Resolution" 節追加
16. **plugins/belt-agent/references/narrative-convention.md, plugins/belt/skills/feature-dev/references/path-convention.md, plugins/belt-agent/skills/protocol/references/resume-mode.md**: 文書更新
17. **lock test 全更新**: feature_dev_refresh / bug_fix_refresh / review_skills_refresh / lint_test / engine_test / cli_test / e2e_test
18. **docs/testing/lock-ledger.md**: produces shape 追記
19. **docs/testing/cli-behavior/*.yml**: locate / status URI scenario 追加
20. **CI 確認**:
    - `cargo fmt --all`
    - `cargo clippy --workspace -- -D warnings`
    - `cargo test --workspace`
    - `cargo run -p belt -- lint plugins/belt/skills/feature-dev/pipeline.yml` (新 lint rule 通過確認)
    - `cargo run -p belt-agent -- init plugins/belt/skills/feature-dev/pipeline.yml` + `belt-agent locate belt://current/notes/phase-design.md` で URI 解決確認

## Open questions

- **Q1**: domain artifact (raw path) の status JSON 表現
  - 案 A: `path` field を残す (URI 形式は `uri`、raw は `path`)
  - 案 B: 全部 `uri` field、raw path は string そのまま (例: `"uri": "docs/features/*/design.md"`)
  - **採用案**: A (両 field を持つ asymmetric shape)。consumer の判別ロジックがシンプル
- **Q2**: glob URI (`belt://current/notes/phase-*.md`) の resolve でゼロ match 時の `path` field
  - **採用案**: declared base (`<run_dir>/<glob pattern>`) を返し `exists: false`
- **Q3**: `belt://current/` の semantics (init 中 vs status 時)
  - **採用案**: pipeline.yml に書かれた URI は **runtime 解決時の** current run を指す。init 時は newly-created run、status 時は `--run` または latest
- **Q4**: code-review の merged `findings.json` を produces に含めるか
  - **採用案**: 含める (orchestrator が書く merged 結果も SSOT 化)。pipeline.yml の produces count は 7 で許容

## References

- `crates/belt-core/src/uri.rs` — 既存 URI scheme 実装
- `crates/belt-core/src/engine.rs:440-468` — `expand_run_id` / `expand_gate_run_id` (削除対象)
- `crates/belt-core/src/view.rs` — status JSON 構築
- `crates/belt-agent/src/resolver.rs` — URI resolution
- `crates/belt-agent/src/main.rs` — CLI entry
- `crates/belt-core/src/lint.rs` — pipeline 静的検証
- `crates/belt-core/src/gate.rs` — gate executor
- `plugins/belt/skills/feature-dev/pipeline.yml` — 既存 path 形式の代表例
- `plugins/belt-agent/skills/protocol/SKILL.md` — Belt Protocol (Path Resolution 節追加先)
- `plugins/belt-agent/references/narrative-convention.md` — 既存 path convention 文書
- `docs/specs/2026-04-14-belt-context-neutral-narrative-artifact.md` — narrative artifact 設計の前提
- `docs/specs/2026-04-07-belt-status-enrichment.md` — status enrichment (BELT-29) の設計
- `docs/testing/lock-ledger.md` — lock test 台帳
- memory `feedback_belt_cli_vs_skill_responsibility.md` — belt CLI と skill の責務分離原則
- memory `project_belt_agent_json_shape_asymmetry.md` — 既存 init/next vs status の shape asymmetry (本 spec で解消)
