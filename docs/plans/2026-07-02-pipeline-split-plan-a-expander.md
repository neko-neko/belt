# Pipeline Split Plan A: Recursive Expander Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** belt-core の expander を再帰展開に拡張し (循環検出 + 深さ上限 + sub 内 regate の namespace リネーム)、Pipeline 形式 YAML の sub 参照 (dual-format) を検証・固定する。

**Architecture:** `expand_pipeline` の 1 段展開 (`expand_sub_pipeline`) を、全レベルを統一処理する `expand_phase_list` 再帰に置き換える。namespace は `{parent}/{sub}/{subsub}` 連結、visited スタックで循環検出、深さ上限 4。親継承規則 (gate/regate/validate append to last / config merge / when propagate) は再帰の各レベルで現行規則を維持する。

**Tech Stack:** Rust (belt-core のみ)。serde-saphyr =0.0.23 / tempfile (既存 dev-dep)。

**Spec:** `docs/specs/2026-07-02-pipeline-split-design.md` (approved)。本 plan は spec の「belt-core 変更: expander 再帰展開」「dual-format」「Open Questions 1, 3」を実装する。plugin 層の分割 (design/diagnose/build/verify YAML、criteria 再配置、lock test 改訂) は **Plan B** (本 plan 完了後に執筆 — spike の実測結果を反映するため) が扱う。

## Spec Gap Notes (Plan B へ引き継ぐ決定事項)

writing-plans 段階で surface し、ユーザーが決定済み:

1. **bug-fix の domain artifact path は `docs/features/<topic>/` に統一** (旧 `docs/plans/*-xxx.md` 規約を廃止、path-convention.md を唯一の SSOT にする)
2. **criteria は structured 記法 (ID/severity/verification/pass_condition) に統一、audit 厳格度は現行 feature-dev の割当を維持** (execute/code-review = required、その他 = lite)

## Global Constraints

- MSRV 1.86.0 / Edition 2024 / toolchain 1.94.1 (rust-toolchain.toml)
- `unsafe_code = forbid` (workspace lints)。`clippy::all` + `pedantic` + `unwrap_used`/`expect_used`/`panic` は `-D warnings` で error 扱い (integration test は既存の `#![allow(...)]` ヘッダを踏襲)
- コミット前: `cargo fmt --package belt-core` / `cargo clippy --package belt-core -- -D warnings` / `cargo test -p belt-core`
- リポジトリコンテンツは英語 (docs/plans, docs/specs のみ日本語可)
- `docs/testing/cli-behavior/belt-core.yml` の scenario id と `crates/*/tests/` の `/// scenario:` doc-comment は集合一致が必須 (`scenarios_contract.rs` が機械照合)。**scenario 付きテストは必ず `crates/belt-core/tests/` 配下に置く** (src 内 `#[cfg(test)]` は walk 対象外で orphan-yml になる)
- 後方互換: 既存の 1 段展開 (`pre-execute-handover` → `../handover/checkpoint.yml`) の挙動・展開 id (`pre-execute-handover/checkpoint`) を変えない (feature_dev_refresh.rs / bug_fix_refresh.rs が lock)

## File Structure

| File | 責務 |
|---|---|
| `crates/belt-core/src/expander.rs` | 再帰展開の実装 (書き換え)。`expand_pipeline` の公開シグネチャは不変 |
| `crates/belt-core/src/lint.rs` | `check_invoke_pipeline_exists` の再帰化 |
| `crates/belt-core/tests/expander_test.rs` | 再帰系 scenario テスト追加 (既存 5 fn に追加) |
| `crates/belt-core/tests/lint_test.rs` | nested 欠損参照の lint テスト追加 |
| `docs/testing/cli-behavior/belt-core.yml` | 新 scenario 6 件追加 |

---

### Task 1: Spike — dual-format (Pipeline 形式 YAML の sub 参照) — **spike 結果: PASS (fallback 不要、model.rs 無変更)**

**Files:**
- Test: `crates/belt-core/tests/expander_test.rs` (末尾に追加)
- Modify (fallback 時のみ): `crates/belt-core/src/model.rs:276-284` (SubPipeline)

**Interfaces:**
- Produces: 「`args:` を持つ Pipeline 形式 YAML を `invoke.pipeline` で参照できる」保証 (Plan B の design/build/verify/diagnose YAML の前提)

**背景 (実装者向け):** `parse_sub_pipeline` は `SubPipeline` 型 (name/description/version/inputs/phases、`deny_unknown_fields` なし) で読む。Plan B では単体実行可能な Pipeline 形式 (args あり) のファイルを sub 参照するため、serde-saphyr が unknown field `args` を無視することを確認・固定する。

- [ ] **Step 1: テストを書く**

`crates/belt-core/tests/expander_test.rs` の末尾に追加:

```rust
/// A standalone Pipeline-format YAML (with a top-level `args:` map) can be
/// referenced via `invoke: { pipeline: ... }`: parse_sub_pipeline ignores
/// the unknown `args` field. This is the dual-format guarantee that lets
/// one file serve both `belt-agent init` and sub-pipeline composition.
///
/// scenario: belt-core-expander-pipeline-format-accepted-as-sub-pipeline
#[test]
fn pipeline_format_yaml_accepted_as_sub_pipeline() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Pipeline format: has `args:` (unknown to SubPipeline) and description.
    write_yaml(
        &dir,
        "standalone.yml",
        r#"
name: standalone
version: 1
description: "Standalone pipeline also usable as a sub-pipeline"
args:
  e2e:
    type: bool
    default: false
    description: "flag"
phases:
  - id: work
    description: "Do the work"
    gate:
      - cmd: "true"
"#,
    );

    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: main
version: 1
phases:
  - id: stage
    invoke:
      pipeline: standalone.yml
"#,
    );

    let expanded = expand_pipeline(&pipeline_path).expect("dual-format expand should succeed");
    assert_eq!(expanded.len(), 1);
    assert_eq!(expanded[0].id, "stage/work");
}
```

- [ ] **Step 2: 実行して結果を観測する**

Run: `cargo test -p belt-core --test expander_test pipeline_format_yaml_accepted`
- **PASS の場合**: serde-saphyr は unknown field を無視する。fallback 不要。Step 3 をスキップし Step 4 へ。plan のこのタスク見出し脇に「spike 結果: PASS (fallback 不要)」とメモを残す (Plan B が参照する)
- **FAIL (YamlParse エラー) の場合**: Step 3 の fallback を実装

- [ ] **Step 3 (FAIL 時のみ): fallback — SubPipeline に無視フィールドを追加**

`crates/belt-core/src/model.rs` の `SubPipeline` に追加:

```rust
    /// Ignored. Present so a standalone Pipeline-format YAML (which declares
    /// `args:`) can also be parsed as a sub-pipeline (dual-format).
    #[serde(default)]
    pub args: HashMap<String, ArgDef>,
```

Run: `cargo test -p belt-core --test expander_test pipeline_format_yaml_accepted` → PASS

- [ ] **Step 4: scenario を belt-core.yml に登録**

`docs/testing/cli-behavior/belt-core.yml` の expander module 節 (`# --- expander module ---`、行702 付近) の末尾に追加:

```yaml
  - id: belt-core-expander-pipeline-format-accepted-as-sub-pipeline
    category: expander
    severity: high
    technique: equivalence-partition
    given: "a standalone Pipeline-format YAML with a top-level `args:` map, referenced from a main pipeline via `invoke: { pipeline: ... }`"
    when: "expand_pipeline is called"
    then: "the unknown `args` field is ignored and the file expands as a sub-pipeline with namespaced phase IDs"
```

- [ ] **Step 5: 契約テストと belt-core 全テストを回す**

Run: `cargo test -p belt-core --test scenarios_contract && cargo test -p belt-core --test expander_test`
Expected: PASS (orphan なし)

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/tests/expander_test.rs docs/testing/cli-behavior/belt-core.yml
# fallback を実装した場合は crates/belt-core/src/model.rs も add
git commit -m "test(belt-core): lock dual-format sub-pipeline parse (Pipeline YAML with args)"
```

---

### Task 2: 再帰展開 + sub 内 regate の namespace リネーム (TDD)

**Files:**
- Test: `crates/belt-core/tests/expander_test.rs` (末尾に追加)
- Modify: `crates/belt-core/src/expander.rs` (`expand_pipeline` / `expand_sub_pipeline` / `leaf_phase` を書き換え)
- Modify: `docs/testing/cli-behavior/belt-core.yml` (scenario 5 件)

**Interfaces:**
- Consumes: Task 1 の dual-format 保証
- Produces: `expand_pipeline(pipeline_path: &Path) -> BeltResult<Vec<ExpandedPhase>>` (シグネチャ不変)。新挙動: (a) sub-pipeline 内の `Invoker::Pipeline` を再帰展開、id は `{parent}/{sub}/{subsub}` 連結。(b) 循環参照は `BeltError::InvalidPipeline`。(c) 参照深さ > 4 は `InvalidPipeline`。(d) sub 内 leaf の `regate` ターゲットは同レベル namespace を prefix (`execute` → `stage/execute`)。(e) 親 `when` は再帰を貫通して伝播。(f) **挙動変更**: sub 内 leaf phase も `description` 必須 (現行は `unwrap_or_default()` で空文字許容だった — DD-8 の精神に統一)

- [ ] **Step 1: failing tests を書く (5 本)**

`crates/belt-core/tests/expander_test.rs` の末尾に追加:

```rust
/// Nested `invoke: { pipeline: ... }` references expand recursively with
/// `{parent}/{sub}/{subsub}` namespaced IDs, and the outermost parent's
/// gate is appended to the LAST innermost leaf.
///
/// scenario: belt-core-expander-nested-sub-pipeline-expands-recursively
#[test]
fn nested_sub_pipeline_expands_recursively() {
    let dir = TempDir::new().expect("failed to create tempdir");

    write_yaml(
        &dir,
        "inner.yml",
        r#"
name: inner
version: 1
phases:
  - id: monkey-test
    description: "Replay scenarios"
    gate:
      - cmd: "true"
  - id: dogfood
    description: "Exploratory testing"
    gate:
      - cmd: "true"
"#,
    );

    write_yaml(
        &dir,
        "middle.yml",
        r#"
name: middle
version: 1
phases:
  - id: execute
    description: "Implement"
    gate:
      - cmd: "true"
  - id: verify
    invoke:
      pipeline: inner.yml
"#,
    );

    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: main
version: 1
phases:
  - id: build
    invoke:
      pipeline: middle.yml
    gate:
      - git_clean: true
"#,
    );

    let expanded = expand_pipeline(&pipeline_path).expect("nested expand should succeed");
    let ids: Vec<&str> = expanded.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["build/execute", "build/verify/monkey-test", "build/verify/dogfood"]
    );
    // Outer parent gate lands on the LAST innermost leaf only.
    assert_eq!(expanded[0].gate.len(), 1, "inner leaves keep own gates");
    assert_eq!(expanded[1].gate.len(), 1);
    assert_eq!(
        expanded[2].gate.len(),
        2,
        "last leaf inherits the outer parent gate appended"
    );
}

/// A cyclic sub-pipeline reference (a.yml -> b.yml -> a.yml) is rejected
/// with InvalidPipeline instead of infinite recursion.
///
/// scenario: belt-core-expander-cyclic-reference-yields-invalid-pipeline
#[test]
fn cyclic_sub_pipeline_reference_yields_invalid_pipeline() {
    let dir = TempDir::new().expect("failed to create tempdir");

    write_yaml(
        &dir,
        "a.yml",
        r#"
name: a
version: 1
phases:
  - id: to-b
    invoke:
      pipeline: b.yml
"#,
    );
    write_yaml(
        &dir,
        "b.yml",
        r#"
name: b
version: 1
phases:
  - id: back-to-a
    invoke:
      pipeline: a.yml
"#,
    );
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: main
version: 1
phases:
  - id: entry
    invoke:
      pipeline: a.yml
"#,
    );

    let err = expand_pipeline(&pipeline_path).expect_err("cycle must be rejected");
    assert!(
        matches!(err, BeltError::InvalidPipeline { ref message } if message.contains("cyclic")),
        "unexpected error: {err:?}"
    );
}

/// Nesting deeper than 4 sub-pipeline levels is rejected with
/// InvalidPipeline naming the depth limit.
///
/// scenario: belt-core-expander-depth-limit-exceeded-yields-invalid-pipeline
#[test]
fn nesting_beyond_depth_limit_yields_invalid_pipeline() {
    let dir = TempDir::new().expect("failed to create tempdir");

    // Chain: pipeline.yml -> d1 -> d2 -> d3 -> d4 -> d5 (leaf). Depth 5 > 4.
    for i in 1..=4 {
        write_yaml(
            &dir,
            &format!("d{i}.yml"),
            &format!(
                r#"
name: d{i}
version: 1
phases:
  - id: next
    invoke:
      pipeline: d{}.yml
"#,
                i + 1
            ),
        );
    }
    write_yaml(
        &dir,
        "d5.yml",
        r#"
name: d5
version: 1
phases:
  - id: leaf
    description: "Bottom"
    gate:
      - cmd: "true"
"#,
    );
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: main
version: 1
phases:
  - id: entry
    invoke:
      pipeline: d1.yml
"#,
    );

    let err = expand_pipeline(&pipeline_path).expect_err("depth must be limited");
    assert!(
        matches!(err, BeltError::InvalidPipeline { ref message } if message.contains("depth")),
        "unexpected error: {err:?}"
    );
}

/// A regate target declared inside a sub-pipeline is renamed into the
/// sub-pipeline's expansion namespace so it points at the expanded id.
///
/// scenario: belt-core-expander-sub-internal-regate-targets-namespaced
#[test]
fn sub_internal_regate_targets_are_namespaced() {
    let dir = TempDir::new().expect("failed to create tempdir");

    write_yaml(
        &dir,
        "stage.yml",
        r#"
name: stage
version: 1
phases:
  - id: execute
    description: "Implement"
    gate:
      - cmd: "true"
  - id: code-review
    description: "Review"
    regate: [execute]
    gate:
      - cmd: "true"
"#,
    );
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: main
version: 1
phases:
  - id: build
    invoke:
      pipeline: stage.yml
"#,
    );

    let expanded = expand_pipeline(&pipeline_path).expect("expand should succeed");
    assert_eq!(expanded[1].id, "build/code-review");
    assert_eq!(
        expanded[1].regate,
        vec!["build/execute".to_string()],
        "sub-internal regate target must be renamed into the expansion namespace"
    );
}

/// The outermost parent `when:` propagates through nested levels to every
/// expanded leaf that does not declare its own when.
///
/// scenario: belt-core-expander-parent-when-propagates-through-nested-levels
#[test]
fn parent_when_propagates_through_nested_levels() {
    let dir = TempDir::new().expect("failed to create tempdir");

    write_yaml(
        &dir,
        "inner.yml",
        r#"
name: inner
version: 1
phases:
  - id: check
    description: "Inner check"
    gate:
      - cmd: "true"
"#,
    );
    write_yaml(
        &dir,
        "middle.yml",
        r#"
name: middle
version: 1
phases:
  - id: deep
    invoke:
      pipeline: inner.yml
"#,
    );
    let pipeline_path = write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: main
version: 1
phases:
  - id: gated
    when: "args.e2e"
    invoke:
      pipeline: middle.yml
"#,
    );

    let expanded = expand_pipeline(&pipeline_path).expect("expand should succeed");
    assert_eq!(expanded.len(), 1);
    assert_eq!(expanded[0].id, "gated/deep/check");
    assert_eq!(
        expanded[0].when.as_deref(),
        Some("args.e2e"),
        "outer when must reach the innermost leaf"
    );
}
```

- [ ] **Step 2: RED を確認する**

Run: `cargo test -p belt-core --test expander_test`
Expected: 新 5 本が FAIL (現行 expander は nested の `Invoker::Pipeline` を leaf 扱いできず `debug_assert` / id 不一致 / regate 素通し / エラー不発で落ちる)。既存 5 本 + Task 1 の 1 本は PASS のまま。

- [ ] **Step 3: expander.rs を再帰版に書き換える**

`crates/belt-core/src/expander.rs` の `expand_pipeline` / `expand_sub_pipeline` / `leaf_phase` を以下に置き換える (substitute 系ヘルパー `substitute_arg_in_value` / `value_to_when_string` / `substitute_in_value_map` / `substitute_in_invoker` と `#[cfg(test)] mod tests` は無変更):

```rust
use crate::error::{BeltError, BeltResult};
use crate::model::{ExpandedPhase, Invoker, Phase};
use crate::parser::{parse_pipeline, parse_sub_pipeline};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Maximum sub-pipeline reference depth (root pipeline = depth 0).
const MAX_EXPANSION_DEPTH: usize = 4;

/// Parse a pipeline and expand all `invoke: { pipeline: ... }` references
/// into flat, namespaced phases — recursively.
///
/// Each phase whose `invoke` is an [`Invoker::Pipeline`] references a
/// sub-pipeline YAML file (resolved relative to the referencing file's
/// directory). Sub-pipeline phases are flattened with namespaced IDs
/// (`{parent_id}/{sub_phase_id}`, nested levels concatenate:
/// `{a}/{b}/{c}`). A sub-pipeline phase may itself reference another
/// sub-pipeline; cycles are rejected and nesting is capped at
/// [`MAX_EXPANSION_DEPTH`].
///
/// Inheritance rules at every level, applied to the expansion of each
/// referencing phase:
/// - `gate`, `regate`, `validate`: parent entries are **appended** to the
///   LAST expanded (innermost) leaf
/// - `config`: merged into the last leaf with parent keys winning
/// - `when`: propagates to **all** expanded leaves that lack their own
///
/// `regate` targets declared inside a sub-pipeline are renamed into that
/// sub-pipeline's expansion namespace (`execute` → `{parent_id}/execute`).
///
/// Every leaf phase (no `invoke: { pipeline: ... }`) **must** have a
/// `description`; otherwise `BeltError::InvalidPipeline` is returned.
pub fn expand_pipeline(pipeline_path: &Path) -> BeltResult<Vec<ExpandedPhase>> {
    let pipeline = parse_pipeline(pipeline_path)?;
    let base_dir = pipeline_path.parent().unwrap_or_else(|| Path::new("."));
    let mut visited = vec![canonical_or_self(pipeline_path)];
    expand_phase_list(&pipeline.phases, base_dir, "", &HashMap::new(), &mut visited)
}

/// Canonicalize for cycle detection; fall back to the raw path when the
/// file cannot be canonicalized (missing file errors surface in parsing).
fn canonical_or_self(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Expand one level of phases, recursing into `Invoker::Pipeline` refs.
///
/// - `ns`: namespace prefix — `""` at the root, `"{parent}/"` below.
/// - `with`: the substitution scope THIS level's phases were referenced
///   with (empty at the root). Applied to leaf `when`/`config`/`invoke`
///   and folded into child reference `with` maps before descending.
/// - `visited`: reference-path stack for cycle and depth detection.
fn expand_phase_list(
    phases: &[Phase],
    base_dir: &Path,
    ns: &str,
    with: &HashMap<String, serde_json::Value>,
    visited: &mut Vec<PathBuf>,
) -> BeltResult<Vec<ExpandedPhase>> {
    let local_ids: Vec<&str> = phases.iter().map(|p| p.id.as_str()).collect();
    let mut expanded = Vec::new();
    for phase in phases {
        if let Some(Invoker::Pipeline {
            pipeline: sub_ref,
            with: phase_with,
        }) = &phase.invoke
        {
            // Resolve the child's `with` in THIS level's scope first —
            // mirrors substitute_in_invoker (I1 parent-scope rule).
            let mut child_with = phase_with.clone();
            if !with.is_empty() {
                substitute_in_value_map(&mut child_with, with);
            }

            let sub_path = base_dir.join(sub_ref);
            let canonical = canonical_or_self(&sub_path);
            if visited.contains(&canonical) {
                return Err(BeltError::InvalidPipeline {
                    message: format!(
                        "phase '{ns}{}': cyclic sub-pipeline reference '{sub_ref}'",
                        phase.id
                    ),
                });
            }
            if visited.len() > MAX_EXPANSION_DEPTH {
                return Err(BeltError::InvalidPipeline {
                    message: format!(
                        "phase '{ns}{}': sub-pipeline nesting exceeds depth {MAX_EXPANSION_DEPTH}",
                        phase.id
                    ),
                });
            }

            let sub = parse_sub_pipeline(&sub_path)?;
            let sub_base = sub_path
                .parent()
                .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
            let child_ns = format!("{ns}{}/", phase.id);

            visited.push(canonical);
            let mut sub_expanded =
                expand_phase_list(&sub.phases, &sub_base, &child_ns, &child_with, visited)?;
            visited.pop();

            if sub_expanded.is_empty() {
                return Err(BeltError::InvalidPipeline {
                    message: format!(
                        "phase '{ns}{}': sub-pipeline '{sub_ref}' has no phases",
                        phase.id
                    ),
                });
            }

            // Parent `when` propagates to every expanded leaf lacking one.
            // The parent's when is authored in the parent's arg scope, so
            // it is substituted against THIS level's `with`, not the child's.
            let parent_when = substituted_when(phase.when.as_deref(), with);
            for sub_phase in &mut sub_expanded {
                if sub_phase.when.is_none() {
                    sub_phase.when.clone_from(&parent_when);
                }
            }

            // gate/regate/validate append + config merge on the LAST leaf.
            if let Some(last) = sub_expanded.last_mut() {
                last.gate.extend(phase.gate.clone());
                last.regate.extend(phase.regate.clone());
                last.validate.extend(phase.validate.clone());
                let mut parent_config = phase.config.clone();
                if !with.is_empty() {
                    substitute_in_value_map(&mut parent_config, with);
                }
                for (k, v) in parent_config {
                    last.config.insert(k, v);
                }
            }

            expanded.extend(sub_expanded);
        } else {
            expanded.push(leaf_phase(phase, ns, with, &local_ids)?);
        }
    }
    Ok(expanded)
}

/// Substitute a `when` template against a `with` scope; returns the
/// (possibly rewritten) owned when.
fn substituted_when(
    when: Option<&str>,
    with: &HashMap<String, serde_json::Value>,
) -> Option<String> {
    let w = when?;
    if !with.is_empty() {
        if let Some(replacement) = substitute_arg_in_value(w, with) {
            if let Some(rewritten) = value_to_when_string(&replacement) {
                return Some(rewritten);
            }
        }
    }
    Some(w.to_owned())
}

/// Materialize a leaf phase at namespace `ns`, applying this level's
/// `with` substitution to `when`, `config`, and the invoker, and renaming
/// sibling-scoped `regate` targets into the namespace.
fn leaf_phase(
    phase: &Phase,
    ns: &str,
    with: &HashMap<String, serde_json::Value>,
    local_ids: &[&str],
) -> BeltResult<ExpandedPhase> {
    let description = phase
        .description
        .clone()
        .ok_or_else(|| BeltError::InvalidPipeline {
            message: format!("leaf phase '{ns}{}' must have a description", phase.id),
        })?;

    let mut config = phase.config.clone();
    if !with.is_empty() {
        substitute_in_value_map(&mut config, with);
    }

    let invoke = {
        let mut inv = phase.invoke.clone();
        if !with.is_empty() {
            if let Some(v) = inv.as_mut() {
                substitute_in_invoker(v, with);
            }
        }
        inv
    };

    // Regate targets naming a sibling phase in this file are renamed into
    // the expansion namespace; anything else is left verbatim.
    let regate = phase
        .regate
        .iter()
        .map(|t| {
            if local_ids.contains(&t.as_str()) {
                format!("{ns}{t}")
            } else {
                t.clone()
            }
        })
        .collect();

    Ok(ExpandedPhase {
        id: format!("{ns}{}", phase.id),
        description,
        config,
        produces: phase.produces.clone(),
        consumes: phase.consumes.clone(),
        gate: phase.gate.clone(),
        validate: phase.validate.clone(),
        regate,
        confirm: phase.confirm,
        max_retries: phase.max_retries,
        when: substituted_when(phase.when.as_deref(), with),
        invoke,
        output_dir: None,
    })
}
```

注意点 (実装者向け):
- `use crate::model::{..., SubPipeline}` の import から `SubPipeline` が不要になったら外す (clippy unused)
- 旧 `expand_sub_pipeline` / 旧 `leaf_phase` は削除する
- `#[cfg(test)] mod tests` 内の unit test のうち旧 `expand_sub_pipeline` を直接呼ぶものの扱い:
  - `expand_sub_pipeline_with_empty_with_is_byte_identical_to_legacy` は「旧関数が legacy と一致する」という趣旨自体が旧関数前提なので**削除**する (等価カバレッジは統合テスト `expand_invoke_pipeline_phase_to_namespaced_ids` と Task 2 の新テストが持つ)
  - `when_*` / `config_*` / `invoker_*` 系の substitute スコープ規則テストは、旧 `expand_sub_pipeline(parent_id, parent, sub, with)` 呼び出しを `expand_phase_list` 経由に書き換えるのが素直でない場合、tempdir + YAML を使う統合テスト形式に移すのではなく、`leaf_phase(phase, ns, with, local_ids)` と `substituted_when` を直接呼ぶ形へ最小書き換えする。**各 test の意図 (substitute スコープ規則の lock: sub 先 rewrite → parent merge の順、parent-inherit 値は rewrite しない) は必ず維持**。`mk_*` ヘルパーのシグネチャは触らない
- 旧実装で `description` が `unwrap_or_default()` だった sub-phase は新実装で必須化される (意図的な挙動変更)。既存テストの sub-pipeline fixture は全て description を持つため影響なし

- [ ] **Step 4: GREEN を確認する**

Run: `cargo test -p belt-core`
Expected: 全 PASS (新 5 本 + 既存全部 + src 内 unit tests)。特に `feature_dev_refresh` / `bug_fix_refresh` の `pre_execute_handover_expands_to_namespaced_checkpoint` が引き続き PASS すること (1 段展開の後方互換)。

- [ ] **Step 5: scenario 5 件を belt-core.yml に登録**

`docs/testing/cli-behavior/belt-core.yml` の expander module 節末尾 (Task 1 で追加した scenario の後) に追加:

```yaml
  - id: belt-core-expander-nested-sub-pipeline-expands-recursively
    category: expander
    severity: high
    technique: equivalence-partition
    given: "a pipeline referencing a sub-pipeline whose phase itself references another sub-pipeline"
    when: "expand_pipeline is called"
    then: "phases expand recursively with `{parent}/{sub}/{subsub}` namespaced IDs and the outer parent gate is appended to the last innermost leaf only"
  - id: belt-core-expander-cyclic-reference-yields-invalid-pipeline
    category: expander
    severity: high
    technique: error-guessing
    given: "sub-pipelines that reference each other in a cycle (a.yml -> b.yml -> a.yml)"
    when: "expand_pipeline is called"
    then: "returns BeltError::InvalidPipeline naming the cyclic reference instead of recursing infinitely"
  - id: belt-core-expander-depth-limit-exceeded-yields-invalid-pipeline
    category: expander
    severity: medium
    technique: boundary-value
    given: "a reference chain nested deeper than 4 sub-pipeline levels"
    when: "expand_pipeline is called"
    then: "returns BeltError::InvalidPipeline naming the depth limit"
  - id: belt-core-expander-sub-internal-regate-targets-namespaced
    category: expander
    severity: high
    technique: equivalence-partition
    given: "a sub-pipeline whose phase declares `regate:` targeting a sibling phase in the same file"
    when: "expand_pipeline is called"
    then: "the regate target is renamed into the sub-pipeline's expansion namespace so it matches the expanded phase id"
  - id: belt-core-expander-parent-when-propagates-through-nested-levels
    category: expander
    severity: medium
    technique: state-transition
    given: "an outermost phase with `when:` referencing a sub-pipeline that itself references another sub-pipeline whose leaf has no when"
    when: "expand_pipeline is called"
    then: "the outermost when reaches the innermost expanded leaf verbatim"
```

- [ ] **Step 6: 契約テスト + fmt + clippy**

Run: `cargo test -p belt-core --test scenarios_contract && cargo fmt --package belt-core && cargo clippy --package belt-core -- -D warnings`
Expected: 全 PASS / No issues

- [ ] **Step 7: Commit**

```bash
git add crates/belt-core/src/expander.rs crates/belt-core/tests/expander_test.rs docs/testing/cli-behavior/belt-core.yml
git commit -m "feat(belt-core): expand sub-pipelines recursively with cycle/depth guards

Sub-pipeline phases may now reference further sub-pipelines; IDs
namespace as {a}/{b}/{c}. Cycles and nesting beyond depth 4 return
InvalidPipeline. Sub-internal regate targets are renamed into the
expansion namespace. Sub-pipeline leaf phases now require description
(was silently defaulted to empty)."
```

---

### Task 3: lint の nested 参照検証

**Files:**
- Test: `crates/belt-core/tests/lint_test.rs` (末尾に追加)
- Modify: `crates/belt-core/src/lint.rs` (`check_invoke_pipeline_exists`)

**Interfaces:**
- Consumes: Task 2 の再帰 expander (lint は expansion 試行でも再帰エラーを拾えるようになっている)
- Produces: nested の欠損 `invoke.pipeline` 参照が phase を名指しする lint Error になる

**背景:** 現行 `check_invoke_pipeline_exists` (lint.rs:300-322) は top-level `pipeline.phases` のみ走査する。sub-pipeline 内の欠損参照は expansion 試行の `FileNotFound` でしか出ず、どの phase の参照かが分からない。

- [ ] **Step 1: failing test を書く**

`crates/belt-core/tests/lint_test.rs` の末尾に追加 (このファイルの既存テストのヘルパー慣行 — tempdir + YAML 書き出し + `lint_pipeline` 呼び出し — の same pattern に従う。既存 import をそのまま使い、新規 import は追加しない):

```rust
/// A missing `invoke.pipeline` reference INSIDE a sub-pipeline is reported
/// as a lint error naming the nested phase, not just a parse failure.
///
/// scenario: belt-core-lint-nested-invoke-pipeline-missing-file-reported
#[test]
fn nested_invoke_pipeline_missing_file_is_reported() {
    let dir = tempfile::TempDir::new().expect("failed to create tempdir");

    std::fs::write(
        dir.path().join("stage.yml"),
        r#"
name: stage
version: 1
phases:
  - id: deep
    invoke:
      pipeline: missing.yml
"#,
    )
    .expect("write stage.yml");

    let pipeline_path = dir.path().join("pipeline.yml");
    std::fs::write(
        &pipeline_path,
        r#"
name: main
version: 1
phases:
  - id: build
    invoke:
      pipeline: stage.yml
"#,
    )
    .expect("write pipeline.yml");

    let report = belt_core::lint::lint_pipeline(&pipeline_path);
    let messages: Vec<String> = report
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("missing.yml") && m.contains("deep")),
        "expected a diagnostic naming the nested phase and the missing file, got: {messages:?}"
    );
}
```

注意: `lint_pipeline` の実際のシグネチャ・戻り値型 (`LintReport` のフィールド名) は lint_test.rs の既存テストで使われている形に合わせること。上のコードはその慣行確認後に field 名を合わせて調整してよい (assert の意図は変えない)。

- [ ] **Step 2: RED を確認する**

Run: `cargo test -p belt-core --test lint_test nested_invoke_pipeline_missing`
Expected: FAIL (現行は nested を見ないため phase 名入り diagnostic が出ない)

- [ ] **Step 3: check_invoke_pipeline_exists を再帰化する**

`crates/belt-core/src/lint.rs` の `check_invoke_pipeline_exists` を置き換え:

```rust
/// Verify that every `phase.invoke.pipeline` reference points to an existing
/// file on disk — recursively through sub-pipelines. The `Skill` variant is
/// not path-like and is checked elsewhere. Cycles terminate via the visited
/// set (the cycle itself is reported by expansion, not here).
fn check_invoke_pipeline_exists(
    pipeline: &Pipeline,
    base_dir: &Path,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let mut visited: Vec<std::path::PathBuf> = Vec::new();
    check_phases_invoke_pipeline(&pipeline.phases, base_dir, "", &mut visited, diagnostics);
}

fn check_phases_invoke_pipeline(
    phases: &[Phase],
    base_dir: &Path,
    ns: &str,
    visited: &mut Vec<std::path::PathBuf>,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for phase in phases {
        if let Some(Invoker::Pipeline {
            pipeline: sub_path, ..
        }) = &phase.invoke
        {
            let resolved = base_dir.join(sub_path);
            if !resolved.exists() {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Error,
                    message: format!(
                        "phase '{ns}{}': invoke pipeline '{}' not found",
                        phase.id, sub_path
                    ),
                });
                continue;
            }
            let canonical = resolved
                .canonicalize()
                .unwrap_or_else(|_| resolved.clone());
            if visited.contains(&canonical) {
                continue; // cycle: expansion reports it
            }
            if let Ok(sub) = crate::parser::parse_sub_pipeline(&resolved) {
                let sub_base = resolved
                    .parent()
                    .map_or_else(|| std::path::PathBuf::from("."), std::path::Path::to_path_buf);
                let child_ns = format!("{ns}{}/", phase.id);
                visited.push(canonical);
                check_phases_invoke_pipeline(
                    &sub.phases,
                    &sub_base,
                    &child_ns,
                    visited,
                    diagnostics,
                );
                visited.pop();
            }
        }
    }
}
```

注意: 既存 diagnostic message `"phase '{}': invoke pipeline '{}' not found"` の形式は top-level では `ns = ""` なので**従来と byte 一致**する (既存 lint テストとの互換)。

- [ ] **Step 4: GREEN + scenario 登録**

Run: `cargo test -p belt-core --test lint_test`
Expected: PASS

`docs/testing/cli-behavior/belt-core.yml` の lint 系 scenario 節 (`belt-core-lint-...` が並ぶ箇所、行335 付近の invoke.pipeline nonexistent file 系の隣) に追加:

```yaml
  - id: belt-core-lint-nested-invoke-pipeline-missing-file-reported
    category: lint
    severity: medium
    technique: error-guessing
    given: "a pipeline whose sub-pipeline contains an `invoke: { pipeline: ... }` reference to a nonexistent file"
    when: "lint_pipeline is called"
    then: "a lint Error diagnostic names the nested phase (namespaced) and the missing file"
```

- [ ] **Step 5: 契約テスト + fmt + clippy + Commit**

Run: `cargo test -p belt-core --test scenarios_contract && cargo fmt --package belt-core && cargo clippy --package belt-core -- -D warnings && cargo test -p belt-core`
Expected: 全 PASS

```bash
git add crates/belt-core/src/lint.rs crates/belt-core/tests/lint_test.rs docs/testing/cli-behavior/belt-core.yml
git commit -m "feat(belt-core): lint nested invoke.pipeline references recursively"
```

---

### Task 4: 最終検証 (workspace + 実 pipeline 回帰)

**Files:** なし (検証のみ。失敗時は該当 Task に戻る)

- [ ] **Step 1: CI と同一コマンドをローカル実行**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --locked -- -D warnings && cargo test --workspace --locked`
Expected: fmt OK / No issues / 全テスト PASS (Task 完了時点で 460 前後)

- [ ] **Step 2: 実 pipeline の後方互換を belt lint で確認 (adversarial probe)**

Run: `cargo run -p belt -- lint plugins/belt/skills/feature-dev/pipeline.yml && cargo run -p belt -- lint plugins/belt/skills/bug-fix/pipeline.yml`
Expected: 両方 exit 0 (エラーなし)。1 段参照 (checkpoint.yml) の展開が従来どおり動く。

- [ ] **Step 3: push して CI green を確認**

```bash
git push origin main
gh run list --workflow=ci.yml --limit 1   # conclusion: success を確認
```

注意: push はユーザー承認済みフロー内 (Plan A 全体が承認対象)。CI 結果は `gh run view <id>` の conclusion で必ず直接確認する (watch コマンドの exit code は信用しない)。

---

## 完了条件 (Plan A)

1. `cargo test --workspace --locked` 全 PASS + CI green
2. 新 scenario 7 件 (dual-format 1 + 再帰系 5 + lint 1) が belt-core.yml と doc-comment の双方に存在し scenarios_contract PASS
3. 実 pipeline 2 本の `belt lint` が exit 0
4. **Plan B の前提記録**: Task 1 の spike 結果 (fallback 要否) を本 plan ファイルの Task 1 見出し脇に追記済み

## Plan B への引き継ぎ

Plan B (plugin 層分割: design/diagnose/build/verify YAML 新設、criteria/references 再配置 + structured 統一 + docs/features path 統一、feature-dev/bug-fix 合成化、lock tests 改訂、protocol SKILL.md 修正、codex timeout、AGENTS.md 更新、version 0.3.0) は Plan A 完了後に writing-plans で執筆する。Spec Gap Notes の決定 2 件と spike 結果を反映すること。
