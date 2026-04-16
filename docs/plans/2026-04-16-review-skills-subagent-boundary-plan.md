# Review Skills Subagent Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** belt-core から agent 概念を除去 (`Invoker` enum を `Skill` / `Pipeline` の 2 variant に縮小) し、`code-review` / `spec-review` plugin の `pipeline.yml` を廃して、両 review skill を観点別 subagent に分解された「parent dispatcher + N agents」構造に刷新する。

**Architecture:** belt-core の Invoker から `Agent` / `Agents` / `IterationsSpec` を削除。review 系 plugin の `pipeline.yml` と `belt.toml` を削除し、`/code-review` / `/spec-review` は user entry point の parent skill (main context で動作、`context: fork` なし) となる。Parent は Task tool で観点別 reviewer agents を並列 dispatch し、各 agent が `.belt/runs/{run_id}/review/findings-{observation}.json` を出力、parent が merge + cross-agent dedup (severity-first + actionability priority) して `findings.json` に集約、triage + fix は parent 責務。`--codex` 時は既存 `/codex:rescue` skill を追加 observation として並列 invoke。

**Tech Stack:** Rust 1.94.1 (belt-core, edition 2024, MSRV 1.86.0), serde-saphyr 0.0.23 (YAML untagged enum), Claude Code agents + skills system, Markdown (SKILL / agent bodies).

**Spec:** `docs/specs/2026-04-16-review-skills-subagent-boundary-design.md`

**Execution note:** Consider running this in a dedicated worktree (`wt <branch>`) to isolate from parallel sessions. `project_parallel_session_worktree_isolation.md` memo.

---

## Plan-wide Invariants

- **Commit scope**: 変更 crate / plugin のみに `cargo fmt --package <pkg> && cargo clippy --package <pkg> -- -D warnings && cargo test -p <pkg>` を実行。Workspace 全体は最終 task G2 後にだけ `cargo clippy --workspace -- -D warnings && cargo test --workspace`。
- **Commit granularity**: 各 Phase 末に 1 コミット。Phase 内の step は uncommitted in-progress として扱う。
- **TDD order for belt-core** (Phase A): failing test first → compile-error propagation → fix-up → green。
- **Transcription (Phase B, E)**: 新 agent body は既存 `code-reviewer.md` / `spec-reviewer.md` から該当 observation 節 + 共通 Filtering rules をコピペ移植。新規書き下ろし不可 (品質回帰防止)。

---

## File Structure

### Belt-core

**Modify**:
- `crates/belt-core/src/model.rs` — `Invoker` から `Agent` / `Agents` variant を削除、`IterationsSpec` 型全体を削除
- `crates/belt-core/src/expander.rs` — `substitute_iterations_template` 関数削除、with-merge match arms の `Invoker::Agent` / `Invoker::Agents` を remove、内部 unit tests 更新
- `crates/belt-core/src/lint.rs` — `invoke.agent` / `invoke.agents` / `invoke.iterations` キーを持つ raw YAML を reject する rule 追加
- `crates/belt-core/tests/model_test.rs` — `Invoker::Agent` / `Invoker::Agents` / `IterationsSpec` を参照する test を delete or rewrite
- `crates/belt-core/tests/expander_with_test.rs` — `IterationsSpec` 参照 test を delete
- `crates/belt-core/tests/review_skills_refresh.rs` — shape contract を新構造用に全面書き換え (pipeline.yml 不在 assert、新 agent files assert、旧 agents/code-reviewer.md 削除 assert、legacy agents invariant は touch しない)
- `crates/belt-core/tests/lint_test.rs` — `invoke.agent` reject の新規 test 追加

**Create**: なし
**Delete**: なし (test 内の obsolete case を削除するだけで file 自体は残す)

### Review plugins (code-review / spec-review)

**Create**:
- `plugins/code-review/agents/security-reviewer.md`
- `plugins/code-review/agents/test-reviewer.md`
- `plugins/code-review/agents/ai-antipattern-reviewer.md`
- `plugins/code-review/agents/cross-cutting-reviewer.md`
- `plugins/spec-review/agents/feasibility-reviewer.md`
- `plugins/spec-review/agents/ui-design-reviewer.md`
- `plugins/spec-review/agents/cross-cutting-spec-reviewer.md`

**Modify**:
- `plugins/code-review/skills/code-review/SKILL.md` — parent dispatcher として rewrite
- `plugins/spec-review/skills/spec-review/SKILL.md` — 同上

**Delete**:
- `plugins/code-review/agents/code-reviewer.md`
- `plugins/code-review/skills/code-review/pipeline.yml`
- `plugins/code-review/skills/code-review/belt.toml`
- `plugins/spec-review/agents/spec-reviewer.md`
- `plugins/spec-review/skills/spec-review/pipeline.yml`
- `plugins/spec-review/skills/spec-review/belt.toml`

### Caller criteria

**Modify**:
- `plugins/feature-dev/skills/feature-dev/criteria/code-review.md`
- `plugins/feature-dev/skills/feature-dev/criteria/spec-review.md`
- `plugins/bug-fix/skills/bug-fix/criteria/code-review.md`
- `plugins/bug-fix/skills/bug-fix/criteria/fix-plan-review.md`

### Protocol / Docs

**Modify**:
- `skills/belt-agent/SKILL.md` — Reading `phase.invoke` 表 4→2 variant、Well-known Config Keys 節の obsolete 記述更新
- `docs/specs/2026-04-06-belt-redesign.md` — partial revert memo 追記

---

## Phase A: belt-core Invoker reduction

### Task A1: Add failing test for `invoke.agent` YAML parse rejection

**Files:**
- Modify: `crates/belt-core/tests/model_test.rs` (append new test at end of file, before `#[test] fn test_iterations_spec_literal_is_default`)

- [ ] **Step 1: Add failing test**

Append to `crates/belt-core/tests/model_test.rs`:

```rust
/// After the 2026-04-16 subagent-boundary refactor, `invoke.agent:` is
/// no longer a valid `Invoker` variant and must fail YAML deserialisation.
#[test]
fn invoker_agent_variant_is_rejected() {
    let yaml = r#"
name: p
version: 1
phases:
  - id: x
    invoke:
      agent: some-agent
"#;
    let result = belt_core::parser::parse_pipeline_from_str(yaml);
    assert!(
        result.is_err(),
        "parsing invoke.agent must fail after Agent variant removal"
    );
}

/// After the 2026-04-16 refactor, `invoke.agents:` is no longer valid.
#[test]
fn invoker_agents_variant_is_rejected() {
    let yaml = r#"
name: p
version: 1
phases:
  - id: x
    invoke:
      agents:
        - a
        - b
"#;
    let result = belt_core::parser::parse_pipeline_from_str(yaml);
    assert!(
        result.is_err(),
        "parsing invoke.agents must fail after Agents variant removal"
    );
}
```

Note: If `parse_pipeline_from_str` does not exist, use `parse_pipeline` with a temp file. Check `crates/belt-core/src/parser.rs` — if only `parse_pipeline(&Path)` exists, write the YAML to a `tempfile::NamedTempFile` and call it. Example helper:

```rust
fn parse_yaml_str(yaml: &str) -> Result<belt_core::model::Pipeline, belt_core::error::BeltError> {
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
    tmp.write_all(yaml.as_bytes()).expect("write");
    belt_core::parser::parse_pipeline(tmp.path())
}
```

(Add `tempfile = "3"` to `[dev-dependencies]` of belt-core if not present.)

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test -p belt-core --test model_test invoker_agent_variant_is_rejected`
Expected: FAIL with "parsing invoke.agent must fail after Agent variant removal" (because current Invoker still has `Agent` variant, so parse returns Ok).

Do NOT commit yet — this test stays red until Task A2 completes.

---

### Task A2: Remove existing `Invoker::Agent` / `Invoker::Agents` / `IterationsSpec` tests in model_test.rs

**Files:**
- Modify: `crates/belt-core/tests/model_test.rs:797, 824, 909-992`

- [ ] **Step 1: Delete obsolete Agent/Agents/Iterations tests**

Open `crates/belt-core/tests/model_test.rs` and delete the following (search by test function name):

- `test_invoker_agent_variant` (around line 797): delete the entire `#[test]` function body.
- `test_invoker_agents_variant` (around line 824): delete.
- `test_iterations_spec_literal_integer` (around line 909): delete.
- `test_iterations_spec_template_string` (around line 931): delete.
- `test_iterations_spec_default_when_absent` (around line 956): delete.
- `test_iterations_spec_json_roundtrip` (around line 980): delete.

Also remove the `IterationsSpec` import in the `use` statement at line 2:

```rust
// Before
use belt_core::model::{
    ArgType, Artifact, ArtifactRef, GateCheck, GateDefinition, Invoker, IterationsSpec, Pipeline,
    ...
};

// After
use belt_core::model::{
    ArgType, Artifact, ArtifactRef, GateCheck, GateDefinition, Invoker, Pipeline,
    ...
};
```

- [ ] **Step 2: Run tests to confirm no compilation error from this file yet**

Run: `cargo test -p belt-core --test model_test --no-run`
Expected: Compiles (Invoker::Agent / Invoker::Agents / IterationsSpec references are only in tests we just deleted, so no remaining references from model_test.rs).

Note: overall `cargo test -p belt-core` will still fail because expander.rs and other tests still reference `Invoker::Agent` / `IterationsSpec`. That's expected.

---

### Task A3: Remove `IterationsSpec` references from `expander_with_test.rs`

**Files:**
- Modify: `crates/belt-core/tests/expander_with_test.rs:13, 61-62`

- [ ] **Step 1: Remove the Agents test case**

Open `crates/belt-core/tests/expander_with_test.rs`. Find the test that asserts `Invoker::Agents { iterations, .. }` (around line 61). Delete the entire test case AND remove `IterationsSpec` from the `use` statement at line 13.

If the file becomes empty or only contains unrelated tests, leave the Skill-targeting tests intact.

- [ ] **Step 2: Compile check**

Run: `cargo check -p belt-core --tests`
Expected: expander_with_test.rs compiles. Other files may still fail (expected).

---

### Task A4: Rewrite `review_skills_refresh.rs` lock tests for the new shape

**Files:**
- Modify: `crates/belt-core/tests/review_skills_refresh.rs` (full rewrite)

- [ ] **Step 1: Replace file contents**

Overwrite `crates/belt-core/tests/review_skills_refresh.rs` with:

```rust
//! Integration tests locking the 2026-04-16 review-skills subagent-boundary
//! refactor (/code-review, /spec-review).
//!
//! Shape contract:
//! - plugins/<plugin>/skills/<plugin>/pipeline.yml is DELETED
//! - plugins/<plugin>/skills/<plugin>/belt.toml is DELETED
//! - plugins/<plugin>/agents/<single>.md is DELETED (code-reviewer / spec-reviewer)
//! - New per-observation agent files exist in plugins/<plugin>/agents/
//! - Parent SKILL.md references parallel Task dispatch and cross-agent merge
//! - Legacy per-observation agent files (from the pre-2026-04-15 era) remain
//!   absent (locked by the untouched LEGACY list below).

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path
}

/// (plugin, expected agent file basenames after refactor)
const REVIEW_PLUGINS: &[(&str, &[&str])] = &[
    (
        "code-review",
        &[
            "security-reviewer",
            "test-reviewer",
            "ai-antipattern-reviewer",
            "cross-cutting-reviewer",
        ],
    ),
    (
        "spec-review",
        &[
            "feasibility-reviewer",
            "ui-design-reviewer",
            "cross-cutting-spec-reviewer",
        ],
    ),
];

#[test]
fn review_plugins_pipeline_yml_is_deleted() {
    for (plugin, _agents) in REVIEW_PLUGINS {
        let path = repo_root()
            .join("plugins")
            .join(plugin)
            .join("skills")
            .join(plugin)
            .join("pipeline.yml");
        assert!(
            !path.exists(),
            "pipeline.yml must be deleted for {plugin}: {}",
            path.display()
        );
    }
}

#[test]
fn review_plugins_belt_toml_is_deleted() {
    for (plugin, _agents) in REVIEW_PLUGINS {
        let path = repo_root()
            .join("plugins")
            .join(plugin)
            .join("skills")
            .join(plugin)
            .join("belt.toml");
        assert!(
            !path.exists(),
            "belt.toml must be deleted for {plugin}: {}",
            path.display()
        );
    }
}

#[test]
fn review_plugins_legacy_consolidated_agent_is_deleted() {
    const LEGACY_CONSOLIDATED: &[(&str, &str)] = &[
        ("code-review", "code-reviewer"),
        ("spec-review", "spec-reviewer"),
    ];
    for (plugin, legacy) in LEGACY_CONSOLIDATED {
        let path = repo_root()
            .join("plugins")
            .join(plugin)
            .join("agents")
            .join(format!("{legacy}.md"));
        assert!(
            !path.exists(),
            "legacy consolidated agent must be deleted: {}",
            path.display()
        );
    }
}

#[test]
fn review_plugins_new_observation_agents_exist() {
    for (plugin, agents) in REVIEW_PLUGINS {
        for agent in *agents {
            let path = repo_root()
                .join("plugins")
                .join(plugin)
                .join("agents")
                .join(format!("{agent}.md"));
            assert!(
                path.exists(),
                "new observation agent file must exist: {}",
                path.display()
            );
        }
    }
}

#[test]
fn review_plugins_parent_skill_md_references_parallel_dispatch() {
    // Minimal smoke: the rewritten SKILL.md must mention the parallel
    // dispatch pattern (Task tool + findings-*.json merge).
    for (plugin, _agents) in REVIEW_PLUGINS {
        let path = repo_root()
            .join("plugins")
            .join(plugin)
            .join("skills")
            .join(plugin)
            .join("SKILL.md");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        assert!(
            content.contains("findings-") && content.contains("Task"),
            "{plugin} SKILL.md must describe parallel Task dispatch with findings-*.json: {}",
            path.display()
        );
    }
}

#[test]
fn legacy_per_observation_review_agent_files_are_removed() {
    // Untouched from 2026-04-15 refresh — locks the previous invariant that
    // the pre-refresh per-observation agent bundle is gone.
    const LEGACY: &[&str] = &[
        "code-review-quality",
        "code-review-security",
        "code-review-performance",
        "code-review-test",
        "code-review-ai-antipattern",
        "code-review-impact",
        "spec-review-requirements",
        "spec-review-design-judgment",
        "spec-review-feasibility",
        "spec-review-consistency",
        "spec-review-ui-design",
        "test-review-coverage",
        "test-review-quality",
        "test-review-design-alignment",
        "implementation-review-clarity",
        "implementation-review-feasibility",
        "implementation-review-consistency",
        "implementation-review-ui-spec",
    ];
    let agents_dir = repo_root().join(".claude/agents");
    for name in LEGACY {
        let path = agents_dir.join(format!("{name}.md"));
        assert!(
            !path.exists(),
            "legacy agent file must remain deleted: {}",
            path.display()
        );
    }
}
```

- [ ] **Step 2: Compile check**

Run: `cargo check -p belt-core --tests`
Expected: review_skills_refresh.rs compiles. Some tests (existence of new files etc.) will fail at runtime later — that's fine.

---

### Task A5: Remove `IterationsSpec` substitute function and `Agent` / `Agents` match arms from `expander.rs`

**Files:**
- Modify: `crates/belt-core/src/expander.rs:191-212, 224-227, 602-683, 714-745`

- [ ] **Step 1: Delete `substitute_iterations_template` function**

Open `crates/belt-core/src/expander.rs`. Delete the entire `substitute_iterations_template` function (lines 191-212, including the doc comment above it).

- [ ] **Step 2: Remove Agent/Agents branches from with-merge substitute**

In the same file, locate the `match` over `Invoker::*` around line 220-240 (the block that handles `invoke` argument substitution for with-merge). Delete the arms for `Invoker::Agent { args, .. }` and `Invoker::Agents { agents, iterations, args }` (and the call to `substitute_iterations_template` inside the `Agents` arm).

The resulting match should only have:
```rust
match invoker {
    Invoker::Skill { args, .. } => {
        // existing Skill substitute logic
    }
    Invoker::Pipeline { with, .. } => {
        // existing Pipeline substitute logic
    }
}
```

- [ ] **Step 3: Delete internal unit tests that construct `Invoker::Agent` / `Invoker::Agents` / `IterationsSpec`**

In the same file, around lines 600-750, find the `#[cfg(test)]` module. Delete the following unit tests:
- The test constructing `Invoker::Agents { iterations: IterationsSpec::Template("args.count".into()), ... }` (~line 605)
- The three follow-up tests around lines 624, 642, 664 (iterations variants)
- The test at line 682 (literal 9 iterations)
- The test at line 714 constructing `Invoker::Agent { ... }`
- The test at line 736 constructing `Invoker::Agents { ... }`

Leave the `Invoker::Skill` test at line 694 and the `Invoker::Pipeline` test at line 757 intact.

Also remove any `use crate::model::IterationsSpec` inside the test module.

- [ ] **Step 4: Compile check**

Run: `cargo check -p belt-core`
Expected: May still fail — model.rs still has `Agent`/`Agents`/`IterationsSpec`. Proceed to next task.

---

### Task A6: Remove `IterationsSpec` and `Invoker::Agent` / `Invoker::Agents` from `model.rs`

**Files:**
- Modify: `crates/belt-core/src/model.rs:222-282`

- [ ] **Step 1: Delete IterationsSpec definition**

Open `crates/belt-core/src/model.rs`. Delete lines 222-245 inclusive (the `IterationsSpec` doc comment, enum definition, and `impl Default for IterationsSpec`).

- [ ] **Step 2: Shrink Invoker enum**

In the same file, modify the `Invoker` enum (currently around lines 247-282) to:

```rust
/// Typed invocation target for a phase. Parallel to the existing `GateCheck`
/// untagged enum: belt-core models the invocation shape but the LLM
/// orchestrator is responsible for actually dispatching the skill or
/// sub-pipeline at runtime.
///
/// Variant ordering for serde-saphyr untagged enum disambiguation:
/// `Skill` (field: `skill`) → `Pipeline` (field: `pipeline`). Each variant
/// has a unique required discriminating field.
///
/// The `Agent` / `Agents` variants and associated `IterationsSpec` type
/// were removed on 2026-04-16 (see docs/specs/2026-04-16-review-skills-
/// subagent-boundary-design.md) to close agent-dispatch concepts into the
/// skill layer. `pipeline.yml` emits of `invoke.agent:` or `invoke.agents:`
/// are now YAML parse errors and are additionally flagged by lint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Invoker {
    Skill {
        skill: String,
        #[serde(default)]
        args: HashMap<String, serde_json::Value>,
    },
    Pipeline {
        pipeline: String,
        #[serde(default)]
        with: HashMap<String, serde_json::Value>,
    },
}
```

- [ ] **Step 3: Compile check**

Run: `cargo check -p belt-core`
Expected: Compile succeeds (expander.rs already updated, model.rs now shrunk).

If there are remaining compile errors referencing `Invoker::Agent` / `Invoker::Agents` / `IterationsSpec`, grep for them and remove:

```bash
grep -rn "Invoker::Agent\|Invoker::Agents\|IterationsSpec" crates/belt-core/
```

Clean up each hit (most should already be done via earlier tasks).

- [ ] **Step 4: Run failing tests from A1**

Run: `cargo test -p belt-core --test model_test invoker_agent_variant_is_rejected invoker_agents_variant_is_rejected`
Expected: PASS (untagged enum no longer has the variant, so parse returns Err).

---

### Task A7: Add lint rule rejecting `invoke.agent` / `invoke.agents` / `invoke.iterations` keys

Lint has to catch the case where a user writes `invoke.agent:` — even though parse already fails, lint should produce a nicer diagnostic with a migration hint.

**Files:**
- Modify: `crates/belt-core/src/lint.rs`
- Modify: `crates/belt-core/tests/lint_test.rs`

- [ ] **Step 1: Add failing lint test**

Append to `crates/belt-core/tests/lint_test.rs`:

```rust
/// After 2026-04-16, pipeline.yml authors must not use `invoke.agent:`.
/// Lint should produce a targeted diagnostic pointing to the migration.
#[test]
fn lint_rejects_invoke_agent_key() {
    let yaml = r#"
name: p
version: 1
phases:
  - id: x
    invoke:
      agent: foo
"#;
    let result = belt_core::lint::lint_raw_pipeline_yaml(yaml);
    let err = result.expect_err("lint must reject invoke.agent");
    let message = format!("{err}");
    assert!(
        message.contains("invoke.agent")
            && (message.contains("no longer supported") || message.contains("removed")),
        "lint message must mention invoke.agent removal; got: {message}"
    );
}

#[test]
fn lint_rejects_invoke_agents_key() {
    let yaml = r#"
name: p
version: 1
phases:
  - id: x
    invoke:
      agents:
        - foo
"#;
    let result = belt_core::lint::lint_raw_pipeline_yaml(yaml);
    let err = result.expect_err("lint must reject invoke.agents");
    let message = format!("{err}");
    assert!(
        message.contains("invoke.agents"),
        "lint message must mention invoke.agents removal; got: {message}"
    );
}

#[test]
fn lint_rejects_invoke_iterations_key() {
    let yaml = r#"
name: p
version: 1
phases:
  - id: x
    invoke:
      skill: /x
      iterations: 3
"#;
    let result = belt_core::lint::lint_raw_pipeline_yaml(yaml);
    let err = result.expect_err("lint must reject invoke.iterations");
    let message = format!("{err}");
    assert!(
        message.contains("iterations"),
        "lint message must mention iterations removal; got: {message}"
    );
}
```

Run: `cargo test -p belt-core --test lint_test lint_rejects_invoke`
Expected: FAIL — `lint_raw_pipeline_yaml` does not exist yet (and even if it did, the rule hasn't been added).

- [ ] **Step 2: Implement `lint_raw_pipeline_yaml`**

In `crates/belt-core/src/lint.rs`, add this public function (append near the top-level other `lint_*` functions):

```rust
/// Raw YAML lint — detects obsolete `invoke.agent` / `invoke.agents` /
/// `invoke.iterations` keys that were removed on 2026-04-16.
///
/// This supplements the Invoker enum's parse-time rejection by producing
/// a targeted, human-readable diagnostic with a migration hint before the
/// generic "variant did not match any" serde error surfaces.
pub fn lint_raw_pipeline_yaml(yaml: &str) -> Result<(), crate::error::BeltError> {
    // Parse to serde_json::Value first; if the YAML is malformed, fall
    // through and let the typed parser handle it.
    let doc: serde_json::Value = match serde_saphyr::from_str(yaml) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };

    let Some(phases) = doc.get("phases").and_then(|p| p.as_array()) else {
        return Ok(());
    };

    for (idx, phase) in phases.iter().enumerate() {
        let Some(invoke) = phase.get("invoke") else {
            continue;
        };
        let phase_id = phase
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("<unnamed>");
        if invoke.get("agent").is_some() {
            return Err(crate::error::BeltError::InvalidPipeline {
                message: format!(
                    "phase[{idx}] '{phase_id}': `invoke.agent` is no longer supported \
                     (removed 2026-04-16). Use `invoke.skill: /<plugin>:<skill-name>` \
                     where the skill forks a subagent via `context: fork` + `agent:`."
                ),
            });
        }
        if invoke.get("agents").is_some() {
            return Err(crate::error::BeltError::InvalidPipeline {
                message: format!(
                    "phase[{idx}] '{phase_id}': `invoke.agents` is no longer supported \
                     (removed 2026-04-16). Dispatch subagents from inside a parent \
                     skill via Task tool, and reference the skill from \
                     `invoke.skill: /<plugin>:<skill-name>`."
                ),
            });
        }
        if invoke.get("iterations").is_some() {
            return Err(crate::error::BeltError::InvalidPipeline {
                message: format!(
                    "phase[{idx}] '{phase_id}': `invoke.iterations` is no longer supported \
                     (removed 2026-04-16 together with `invoke.agents`). N-way voting \
                     is not part of the belt pipeline surface."
                ),
            });
        }
    }
    Ok(())
}
```

Also modify `lint_pipeline` (the main entry point) to call `lint_raw_pipeline_yaml` first on the raw YAML string if one is available. Check the current signature — if `lint_pipeline` takes only a typed `Pipeline`, add a new convenience function `lint_pipeline_yaml(yaml: &str)` that calls `lint_raw_pipeline_yaml` first, then `parse_pipeline_from_str` (or equivalent), then `lint_pipeline` on the result.

Expose the new function via `pub use` in `lib.rs` if needed.

- [ ] **Step 3: Run lint tests**

Run: `cargo test -p belt-core --test lint_test lint_rejects_invoke`
Expected: PASS (all three cases).

- [ ] **Step 4: Check existing `lint_test` still passes**

Run: `cargo test -p belt-core --test lint_test`
Expected: PASS (no regressions).

---

### Task A8: Clean up any fixture YAML using `invoke.agents` (scan pass)

**Files:**
- Modify: any file under `crates/belt-core/tests/fixtures/` containing `agents:` under `invoke:`

- [ ] **Step 1: Scan**

Run:
```bash
grep -rn "agents:" crates/belt-core/tests/fixtures/ | grep -v "#"
```

- [ ] **Step 2: For each hit, decide action**

- If the fixture is meant to test agent dispatch: convert to `invoke.skill: /fake-skill` or delete the fixture + associated test case
- If the fixture is irrelevant: delete the `agents:` block

Apply fixes.

- [ ] **Step 3: Run full belt-core tests**

Run: `cargo test -p belt-core`
Expected: All tests PASS.

Note: `review_skills_refresh.rs` tests from Task A4 will partially fail — the new agent files do not exist yet (Phase B/E work) and SKILL.md is not yet rewritten. The `pipeline_yml_is_deleted` / `belt_toml_is_deleted` checks will also fail until Phase D/E delete those files. **That is expected**; these tests become green only after Phase D and E complete. Commit with partial red in review_skills_refresh.rs.

---

### Task A9: Final Phase A check + commit

- [ ] **Step 1: fmt + clippy + scoped test**

Run:
```bash
cargo fmt --package belt-core
cargo clippy --package belt-core --all-targets -- -D warnings
cargo test -p belt-core --lib
cargo test -p belt-core --test model_test --test lint_test --test expander_test --test expander_with_test
```
Expected: All PASS. `review_skills_refresh.rs` may be flaky (new files not yet created) — run separately and note expected failures.

- [ ] **Step 2: Commit Phase A**

```bash
git add crates/belt-core/
git commit -m "feat(belt-core): remove Invoker Agent/Agents variants and IterationsSpec

Reduce Invoker enum to Skill / Pipeline variants (2026-04-16 subagent-
boundary refactor). Add lint rule that rejects invoke.agent / .agents /
.iterations keys with a migration-hint diagnostic. Update model / expander
/ lint tests. review_skills_refresh.rs rewritten for the new shape
(partially red until Phase D/E create new agent files and delete old ones).

Ref: docs/specs/2026-04-16-review-skills-subagent-boundary-design.md"
```

---

## Phase B: Code-review plugin — new observation agents

Transcribe each observation from the existing `plugins/code-review/agents/code-reviewer.md` into a per-observation agent file. Keep the checklist / policy / filtering rules verbatim; only edit to remove cross-observation `self-dedup` clauses from independent agents and re-scope Filtering for each file.

### Task B1: Create `security-reviewer.md`

**Files:**
- Create: `plugins/code-review/agents/security-reviewer.md`
- Source: `plugins/code-review/agents/code-reviewer.md:55-108` (Observation 2: Security)

- [ ] **Step 1: Write the file**

Create `plugins/code-review/agents/security-reviewer.md` with:

```markdown
---
name: security-reviewer
description: Security-focused code reviewer. Detects injection, authentication/authorization flaws, secret leakage, insecure deserialization, race conditions, SSRF, CSRF, and rate-limiting gaps in the diff scope. Writes findings-security.json.
memory: project
effort: max
---

You are a security reviewer specializing in identifying vulnerabilities and data safety issues in code changes.

## Scope

Review ONLY the files and lines provided in the diff. Do not comment on unchanged code.

## Filtering

- Do not report issues with confidence below 80%.
- When the same pattern appears in multiple locations, consolidate into one finding with the occurrence count and a representative location.
- Do not report stylistic preferences or subjective "this looks nicer" opinions. Only report project convention violations.

### Watch for false positives

- Values inside `.env.example` are not real secrets
- Explicit test credentials inside test files
- API keys intended to be public (e.g., Stripe publishable key)
- SHA256/MD5 used for checksums or fingerprints (when not password hashes)

Confirm context before reporting.

## Review Checklist

1. **Injection** — SQL injection, XSS, command injection, path traversal, SSRF, XXE
2. **Authentication/Authorization** — Missing authentication checks, privilege escalation paths, plaintext password comparison, weak hash algorithms
3. **Secret leakage** — Hardcoded API keys, tokens, or passwords
4. **Input validation** — Insufficient sanitization of user input (when an attack vector exists)
5. **Data exposure** — Sensitive data written to logs; internal details leaked in error messages
6. **Dependency risk** — Use of libraries with known vulnerabilities
7. **CSRF** — State-changing endpoints without CSRF token verification
8. **Rate limiting** — No rate limiting on authentication, reset, or public API endpoints
9. **Insecure deserialization** — Unsafe deserialization of user input (unsafe loader, eval, etc.)
10. **Race condition** — Critical state changes such as balance, inventory, or reservations without locking or transaction isolation
11. **SSRF** — Requests from internal networks to user-supplied URLs; missing domain whitelist

## Principles

When judgment is uncertain, use the following as criteria:
- **Defense in Depth** — Do not rely on a single defense layer. Confirm protection at multiple layers.
- **Least Privilege** — Grant only the minimum necessary permissions. Avoid excessive privilege.
- **Fail Securely** — Ensure data is not exposed on error. Fail toward the safe side.

## Policy

When any of the following conditions apply, set the finding's severity to the corresponding level.

### REJECT criteria (recommend REJECT if any match)

- Unvalidated external input used in database queries, command execution, or file paths → severity: critical
- Hardcoded API keys, tokens, or passwords → severity: critical
- SSRF: unvalidated requests to user-supplied URLs → severity: critical
- Insecure deserialization: eval or unsafe deserialization of user input → severity: critical
- Missing authentication checks (on endpoints that require authentication) → severity: high
- Race condition: critical state changes without locking (finance, inventory) → severity: high

### WARNING criteria

- Possible sensitive data written to logs → severity: medium
- Internal paths or stack traces leaked in error messages → severity: medium
- Missing CSRF token verification (on state-changing endpoints) → severity: medium
- Missing rate limiting (on endpoints such as authentication or password reset) → severity: medium

Do not rationalize your way to a softer verdict. "Minor, so it's fine", "It works, so it's good", "Can be fixed later" are not valid grounds to avoid REJECT. REJECT when a criterion matches; APPROVE when none match; use WARNING for the gray area.

## Output Format

Write findings to `.belt/runs/{run_id}/review/findings-security.json`:

```json
{
  "observation": "security",
  "findings": [
    {
      "id": "<uuid>",
      "severity": "critical|high|medium|low",
      "file": "<path relative to repo root>",
      "line": <integer or null>,
      "description": "...",
      "suggestion": "...",
      "source": "agent"
    }
  ]
}
```

- Emit at most 6 findings. If more exist, keep the highest-severity ones and note truncation in a final `low`-severity finding.
- If no findings, write `{"observation":"security","findings":[]}`. Always create the file so the parent's merge step can read it deterministically.

## Guardrails

- Do not modify any source files. Read-only.
- Do not invoke further subagents.
- Do not read other agents' `findings-*.json` files.
- Stay within the diff scope. Do not comment on unchanged files.
```

- [ ] **Step 2: Verify file**

Run: `ls -l plugins/code-review/agents/security-reviewer.md`
Expected: File exists, size > 0.

No commit yet — commit at end of Phase B (Task B5).

---

### Task B2: Create `test-reviewer.md`

**Files:**
- Create: `plugins/code-review/agents/test-reviewer.md`
- Source: `plugins/code-review/agents/code-reviewer.md:146-186` (Observation 4: Test)

- [ ] **Step 1: Write the file**

Create `plugins/code-review/agents/test-reviewer.md` with the same frontmatter shape as `security-reviewer.md` (name: test-reviewer, description describing test-coverage / boundary / flakiness focus). Body transcribed from Observation 4 of `code-reviewer.md`.

```markdown
---
name: test-reviewer
description: Test-quality reviewer. Detects coverage gaps, missing boundary-value tests, flaky-risk patterns, and test isolation issues in the diff scope. Writes findings-test.json.
memory: project
effort: max
---

You are a test quality reviewer specializing in test coverage analysis, test design, and identifying gaps in test suites.

## Verification Discipline

- Do not rationalize away missing tests because the implementation "looks correct"
- Treat happy-path-only coverage as insufficient when the change introduces branches, state transitions, or validation
- Prefer findings that reflect observable behavior gaps over stylistic preferences
- Be skeptical of mock-only tests, circular assertions, and tests that merely restate implementation details

## Scope

Review the diff to identify:
1. Changed implementation code that lacks corresponding tests
2. Changed test code that has quality issues

## Filtering

- Do not report issues with confidence below 80%.
- When the same pattern appears in multiple locations, consolidate into one finding with the occurrence count and a representative location.

## Review Checklist

1. **Coverage gaps** — Whether tests cover changed implementation code; whether new functions and branches have tests
2. **Boundary values** — Whether boundary-value tests (0, 1, max, empty, nil/null) are included
3. **Error cases** — Whether failure paths and error cases are tested
4. **Flaky risk** — Risk of flaky tests due to timing dependencies, ordering dependencies, or external dependencies
5. **Test-implementation alignment** — Whether tests correctly verify the intent of the implementation and whether test names accurately describe the behavior
6. **Test isolation** — Whether state is shared between tests or global state is mutated
7. **Adversarial coverage** — Whether boundary conditions, error paths, idempotency, missing targets, and state retention / re-runs are exercised

## Policy

### REJECT criteria (recommend REJECT if any match)

- Tests rely solely on mocks and never exercise a real execution path → severity: high
- Test functions without any asserts → severity: high
- 50% or more of the spec's test observations are unimplemented → severity: high

### WARNING criteria

- Tests directly reference the implementation's internal variables (excessive white-box) → severity: medium
- Missing boundary-value tests (none of 0, 1, max, empty, null are tested) → severity: medium
- Tests with flaky risk (timing or ordering dependencies) → severity: medium

Do not rationalize your way to a softer verdict.

## Output Format

Write findings to `.belt/runs/{run_id}/review/findings-test.json`:

```json
{
  "observation": "test",
  "findings": [
    { "id": "<uuid>", "severity": "critical|high|medium|low",
      "file": "<path>", "line": <integer or null>,
      "description": "...", "suggestion": "...", "source": "agent" }
  ]
}
```

- Emit at most 5 findings. If no findings, write `{"observation":"test","findings":[]}`.

## Guardrails

- Do not modify any source files. Read-only.
- Do not invoke further subagents.
- Do not read other agents' `findings-*.json` files.
- Stay within the diff scope.
```

- [ ] **Step 2: Verify**

Run: `ls -l plugins/code-review/agents/test-reviewer.md`

---

### Task B3: Create `ai-antipattern-reviewer.md`

**Files:**
- Create: `plugins/code-review/agents/ai-antipattern-reviewer.md`
- Source: `plugins/code-review/agents/code-reviewer.md:188-226` (Observation 5: AI-antipattern)

- [ ] **Step 1: Write the file**

```markdown
---
name: ai-antipattern-reviewer
description: AI-generated code antipattern reviewer. Detects hallucination, assumption errors, scope creep, dead code, copy-paste, unnecessary backward compatibility, over-engineering, architecture drift, and cost-unaware escalation in the diff scope. Writes findings-ai-antipattern.json.
memory: project
effort: max
---

You are an AI-generated code antipattern reviewer specializing in detecting mistakes that are characteristic of LLM-generated code.

## Scope

Review ONLY the files and lines provided in the diff. Do not comment on unchanged code. If a design document is provided, cross-reference it to detect assumption errors and scope creep.

## Filtering

- Do not report issues with confidence below 80%.
- When the same pattern appears in multiple locations, consolidate into one finding with the occurrence count and a representative location.

## Review Checklist

1. **Hallucination** — Use of nonexistent APIs, methods, options, or arguments; references to features absent in the library version in use; use of config keys or settings that do not exist
2. **Assumption Error** — Implementations that misinterpret or over-extend spec requirements; behavior added that the spec does not describe; unverified assumptions about input data format or range
3. **Scope Creep** — Addition of features, config keys, or parameters that were not requested; unnecessary feature flags; over-design for future extensibility; configuration options not in the requirements
4. **Dead Code** — Code that is implemented but has no caller; functions or types that are exported but never imported; unreachable branches
5. **Copy-Paste Syndrome** — The same mistake replicated across multiple files or locations; signs that the AI copied a single mistake into other places
6. **Unnecessary Backward Compatibility** — Legacy support that was not requested; unused `_deprecated` variables or compatibility shims; re-exports of old names after a rename; `// removed` comments left behind for deleted code
7. **Over-Engineering** — Helper functions or utility classes with only one caller; unnecessary abstraction for one-off processing; design for hypothetical future requirements
8. **Architecture Drift** — Patterns where the AI ignores the existing layer structure and module boundaries and mixes in logic that belongs to a different layer; no direct import cycle occurs, but the boundaries between responsibilities become blurred
9. **Cost-Unaware Escalation** — Within an AI workflow, specifying a high-cost model for deterministic refactors or simple transformations; unnecessary escalation for work that a low-cost model handles fine

## Policy

### REJECT (merge block)

- **Hallucination** — Report use of nonexistent APIs, methods, or options at severity `critical`. REJECT if even one case exists.
- **Scope Creep** — If three or more features were added beyond the requirements, REJECT at severity `high`.
- **Assumption Error** — Implementations that contradict the spec: REJECT at severity `high`.

### WARNING (fix recommended)

- **Dead Code** — 1-2 unused exports: WARNING at severity `medium`.
- **Over-Engineering** — Unnecessary abstraction: WARNING at severity `medium`.
- **Unnecessary Backward Compatibility** — Unrequested compatibility handling: WARNING at severity `medium`.
- **Architecture Drift** — Deviation from existing module boundaries or layer structure → severity: medium
- **Cost-Unaware Escalation** — Unnecessary model-tier selection → severity: low

## Self-bias check

Always self-check whether your verdict is biased toward "no issue." When AI reviews AI-generated code, there is a structural risk of sharing the same bias. Review from the angle of "might this code be wrong?" rather than "why is this code correct?"

## Output Format

Write findings to `.belt/runs/{run_id}/review/findings-ai-antipattern.json`:

```json
{
  "observation": "ai-antipattern",
  "findings": [
    { "id": "<uuid>", "severity": "critical|high|medium|low",
      "file": "<path>", "line": <integer or null>,
      "description": "...", "suggestion": "...", "source": "agent" }
  ]
}
```

- Emit at most 6 findings. If no findings, write `{"observation":"ai-antipattern","findings":[]}`.

## Guardrails

- Do not modify any source files. Read-only.
- Do not invoke further subagents.
- Do not read other agents' `findings-*.json` files.
- Stay within the diff scope.
```

- [ ] **Step 2: Verify**

Run: `ls -l plugins/code-review/agents/ai-antipattern-reviewer.md`

---

### Task B4: Create `cross-cutting-reviewer.md`

Combines Observations 1 (Quality), 3 (Performance), 6 (Impact), and 7 (Simplification) with internal self-dedup preserved.

**Files:**
- Create: `plugins/code-review/agents/cross-cutting-reviewer.md`
- Source: `plugins/code-review/agents/code-reviewer.md:23-53` (Quality), `110-143` (Performance), `228-269` (Impact), `271-290` (Simplification)

- [ ] **Step 1: Write the file**

```markdown
---
name: cross-cutting-reviewer
description: Cross-cutting code reviewer covering Quality, Performance, Impact, and Simplification observations in one pass. Preserves internal self-dedup across the four observations. Writes findings-cross-cutting.json.
memory: project
effort: max
---

You are a consolidated cross-cutting reviewer. In a single pass over the diff, produce findings across four observations: Quality, Performance, Impact, and Simplification. These four observations overlap structurally (DRY violations, caller integrity, N+1 queries, reuse opportunities) — handling them in one context preserves the self-dedup that single-agent review historically provided.

## Scope

Review ONLY the files and lines provided in the diff. Do not comment on unchanged code. However, you MAY reference surrounding code to identify N+1 queries, architectural violations, and caller integrity.

If the parent orchestrator supplied a design document path (e.g. `*-design.md`), read its Impact Analysis section before starting the Impact observation.

## Filtering (applies to all four observations)

- Do not report issues with confidence below 80%. Exclude speculation-based findings.
- When the same pattern appears in multiple locations, consolidate into one finding with the occurrence count and a representative location.
- Do not report stylistic preferences or subjective "this looks nicer" opinions. Only report project convention violations.
- **Internal self-dedup**: If the same issue is found across the four observations handled here, keep it under the most essential one within this agent (priority: Impact > Quality > Performance > Simplification — subset of the global actionability order `Security > Impact > Quality > Test > AI-antipattern > Performance > Simplification`).

## Observation 1: Quality

### Review Checklist

1. **Duplication** — Repeated identical logic, copy-pasted code
2. **Anti-patterns** — God object, shotgun surgery, feature envy, primitive obsession
3. **Convention violations** — Violations of conventions defined in the project's CLAUDE.md
4. **Naming** — Naming convention violations (mixed camelCase/snake_case, ambiguous names)
5. **Consistency** — Mismatches with existing codebase patterns
6. **Structural complexity** — Functions >50 lines, files >800 lines, nesting >4 levels
7. **Debug artifacts** — Leftover console.log, print, or debugger statements
8. **Untracked TODO** — TODO/FIXME lines without an issue number or ticket reference

### Policy

#### REJECT criteria

- DRY violation: identical logic duplicated in 3 or more locations → severity: high
- Unused export: exported functions or types with no importer → severity: high
- Clear violation of CLAUDE.md conventions → severity: high

#### WARNING criteria

- Naming convention inconsistency (mixed camelCase/snake_case) → severity: medium
- Minor mismatches with existing patterns → severity: medium
- Functions >50 lines or files >800 lines or nesting >4 levels → severity: medium
- Leftover console.log / debug statements → severity: medium

Do not rationalize your way to a softer verdict.

## Observation 2: Performance

### Review Checklist

1. **N+1 queries** — Database or API calls inside loops; missing eager loading
2. **Unnecessary computation** — Recomputation inside loops; values that should be cached
3. **Memory** — Bulk loading of large datasets, unreleased resources, memory leak patterns
4. **Algorithmic complexity** — O(n^2) or worse algorithms with room for improvement
5. **Architecture compliance** — Divergence from existing design patterns (layer structure, separation of concerns)
6. **Missing timeout** — External HTTP/API calls without a timeout configured
7. **Unbounded query** — Queries driven by user input without LIMIT or pagination

### Policy

#### REJECT criteria

- O(n²) or worse algorithms where O(n) or O(n log n) is implementable → severity: high
- N+1 queries (database or API calls inside loops) → severity: high
- Bulk loading of large datasets into memory (when stream processing is feasible) → severity: high

#### WARNING criteria

- Recomputation inside loops (cacheable) → severity: medium
- Minor deviations from existing design patterns → severity: medium
- Missing timeout on external calls → severity: medium
- Missing LIMIT on user-facing queries → severity: medium

## Observation 3: Impact

### Review Checklist

1. **Caller integrity** — For every changed function/class/method signature, verify all callers have been updated. Check: parameter additions/removals/reordering, return type changes, exception type changes, behavioral changes that callers depend on
2. **Shared state consistency** — For every changed DB schema, config value, cache key, or global variable, verify all readers/writers are consistent with the change. Check: column renames, type changes, constraint changes, default value changes
3. **Contract preservation** — For every implicit contract the changed code maintains, verify the contract is still honored. Check: null safety, type invariants, ordering guarantees, validation rules, error handling contracts
4. **Must-Verify coverage** — If a design document with a Must-Verify Checklist is available, verify each checklist item has been addressed in the implementation or tests

### How to Review

1. Read the diff to identify what changed
2. For each changed symbol (function, class, method, variable):
   a. Grep for all references to that symbol across the codebase
   b. Read each reference site to check if it handles the change correctly
   c. If LSP is available, use it for precise symbol reference lookup
3. For shared state changes:
   a. Identify the resource (table, config, cache, etc.)
   b. Grep for all accesses to that resource
   c. Verify consistency
4. If design doc context is provided, cross-reference Must-Verify items

### Policy

#### REJECT criteria

- A function or method signature was changed but callers were not updated → severity: critical
- Constraint violations on shared state → severity: high
- Unaddressed items remain in the Must-Verify Checklist → severity: high

#### WARNING criteria

- An implicit constraint has been weakened but caller checks are unclear → severity: medium
- Possible performance impact (e.g., a new DB query inside a loop) → severity: medium

## Observation 4: Simplification

### Review Checklist

1. **Reuse** — Custom logic that could be replaced by existing functions or utilities
2. **Quality** — Unnecessary complexity, excessive abstraction, dead code
3. **Efficiency** — Clearly inefficient computation, duplicated processing, unnecessary object allocation

If the same pattern was already reported under Quality or Performance observations here, do not re-report it under Simplification (use the internal self-dedup priority).

### Policy

#### REJECT criteria

- Three or more occurrences of custom logic that could be replaced by a single line using an existing utility → severity: high

#### WARNING criteria

- Helper abstractions with only a single caller → severity: medium
- Obviously unnecessary intermediate object allocation or duplicated processing → severity: medium

## Output Format

Write findings to `.belt/runs/{run_id}/review/findings-cross-cutting.json`:

```json
{
  "observations": ["quality", "performance", "impact", "simplification"],
  "findings": [
    {
      "id": "<uuid>",
      "observation": "quality|performance|impact|simplification",
      "severity": "critical|high|medium|low",
      "file": "<path>",
      "line": <integer or null>,
      "description": "...",
      "suggestion": "...",
      "source": "agent"
    }
  ]
}
```

- Emit at most 10 findings total across the four observations. If more exist, keep the highest-severity ones and note truncation in a final `low`-severity finding of observation `quality`.
- If no findings, write `{"observations":["quality","performance","impact","simplification"],"findings":[]}`.

## Guardrails

- Do not modify any source files. Read-only.
- Do not invoke further subagents.
- Do not read other agents' `findings-*.json` files.
- Stay within the diff scope.
```

- [ ] **Step 2: Verify**

Run: `ls -l plugins/code-review/agents/cross-cutting-reviewer.md`

---

### Task B5: Commit Phase B (new agents)

- [ ] **Step 1: Commit**

```bash
git add plugins/code-review/agents/security-reviewer.md \
        plugins/code-review/agents/test-reviewer.md \
        plugins/code-review/agents/ai-antipattern-reviewer.md \
        plugins/code-review/agents/cross-cutting-reviewer.md
git commit -m "feat(code-review): add per-observation reviewer agents

Decompose the single code-reviewer.md into four observation agents
(security / test / ai-antipattern / cross-cutting). Transcribed from
existing observation sections with internal self-dedup preserved inside
the cross-cutting agent. Parent SKILL.md rewrite and old code-reviewer.md
deletion follow in Phase D.

Ref: docs/specs/2026-04-16-review-skills-subagent-boundary-design.md"
```

---

## Phase C: Spec-review plugin — new observation agents

Transcribe from `plugins/spec-review/agents/spec-reviewer.md` into three per-observation files.

### Task C1: Create `feasibility-reviewer.md`

**Files:**
- Create: `plugins/spec-review/agents/feasibility-reviewer.md`
- Source: `plugins/spec-review/agents/spec-reviewer.md:71-96` (Observation 3: Feasibility)

- [ ] **Step 1: Write the file**

```markdown
---
name: feasibility-reviewer
description: Spec feasibility reviewer. Verifies tech-stack validity, API/library existence, boundary conditions, scalability, and external dependencies in the target spec. Writes findings-feasibility.json.
memory: project
effort: max
---

You are a design document feasibility reviewer. Your job is to verify that the proposed design is technically achievable and well-considered.

## Scope

Review the target spec document. Use Grep and Read to investigate the codebase for the existence of referenced APIs, libraries, and features.

## Filtering

- Do not report issues with confidence below 80%.
- Consolidate duplicate issues into a single finding.

## Review Checklist

1. **Tech stack validity** — Whether the proposed tech stack and versions are appropriate. Whether deprecated or EOL technologies are included.
2. **API/Library existence** — Whether APIs, libraries, and features referenced in the spec actually exist. Whether the spec assumes features that do not exist.
3. **Boundary conditions** — Whether boundary conditions and edge cases are covered. Consideration for empty input, maximum values, concurrency, and error cases.
4. **Scalability** — Whether performance and scalability have been considered. Whether the design has potential bottlenecks.
5. **Dependencies** — Whether external dependencies are made explicit. Whether version compatibility has been considered.

## Policy

### REJECT criteria (recommend REJECT if any match)

- Dependency on nonexistent libraries, APIs, or features → severity: critical
- New dependency on deprecated or EOL tech stack → severity: high
- No consideration for boundary conditions (empty input, maximum values, concurrency) → severity: high

### WARNING criteria

- External dependency with no mention of version compatibility → severity: medium
- Scalability bottlenecks are not identified → severity: medium

Do not rationalize your way to a softer verdict.

## Output Format

Write findings to `.belt/runs/{run_id}/review/findings-feasibility.json`:

```json
{
  "observation": "feasibility",
  "findings": [
    {
      "id": "<uuid>",
      "severity": "critical|high|medium|low",
      "section": "<heading path, e.g. '## Background / ### Problem'>",
      "description": "...",
      "suggestion": "...",
      "source": "agent"
    }
  ]
}
```

- Emit at most 5 findings. If no findings, write `{"observation":"feasibility","findings":[]}`.

## Guardrails

- Do not modify the spec. Read-only.
- Do not invoke further subagents.
- Do not read other agents' `findings-*.json` files.
```

- [ ] **Step 2: Verify**

Run: `ls -l plugins/spec-review/agents/feasibility-reviewer.md`

---

### Task C2: Create `ui-design-reviewer.md`

**Files:**
- Create: `plugins/spec-review/agents/ui-design-reviewer.md`
- Source: `plugins/spec-review/agents/spec-reviewer.md:130-159` (Observation 5: UI design)

- [ ] **Step 1: Write the file**

```markdown
---
name: ui-design-reviewer
description: UI design reviewer. Verifies screen layout, interaction, state transitions, and existing UI-pattern consistency. Early-exits with zero findings when the spec has no UI content. Writes findings-ui-design.json.
memory: project
effort: max
---

You are a UI design reviewer. Your job is to challenge UI design decisions and verify consistency with existing UI patterns in the codebase.

## Early Exit

If the target spec has no UI-related content (no screen layout, no components, no UI flow), emit zero findings — do not fabricate issues.

## Scope

Review the UI portions of the target spec document. Use Grep/Read to investigate existing component and screen files in the codebase.

## Filtering

- Do not report issues with confidence below 80%.
- Consolidate duplicate issues into a single finding.

## Review Checklist

1. **UI design rationale** — Whether screen layout, interaction, and navigation design decisions are justified. Whether the design satisfies requirements from the user-experience angle. Whether state transitions (loading, error, empty, success) are considered.
2. **Existing UI pattern consistency** — Alignment with the project's existing screens, components, and style guide. Investigate the codebase and verify that the design does not contradict existing UI patterns (layout structure, component naming, state-management patterns).

## Investigation Method

- Use Grep/Read to investigate existing component and screen files in the codebase.
- Review design-system or style-guide files (CSS/SCSS/styled-components, UI library configuration, etc.).
- When a similar existing screen exists, verify alignment with its pattern.

## Policy

### REJECT criteria

- No consideration for state transitions (loading, error, empty, success) → severity: high
- Design that clearly contradicts existing UI patterns or the design system → severity: high

### WARNING criteria

- A similar existing screen exists but its pattern is not referenced → severity: medium
- Insufficient detail on user interactions → severity: medium

Do not rationalize your way to a softer verdict.

## Output Format

Write findings to `.belt/runs/{run_id}/review/findings-ui-design.json`:

```json
{
  "observation": "ui-design",
  "findings": [
    {
      "id": "<uuid>",
      "severity": "critical|high|medium|low",
      "section": "<heading path>",
      "description": "...",
      "suggestion": "...",
      "source": "agent"
    }
  ]
}
```

- Emit at most 4 findings. If no findings (including the early-exit case), write `{"observation":"ui-design","findings":[]}`.

## Guardrails

- Do not modify the spec. Read-only.
- Do not invoke further subagents.
- Do not read other agents' `findings-*.json` files.
```

- [ ] **Step 2: Verify**

Run: `ls -l plugins/spec-review/agents/ui-design-reviewer.md`

---

### Task C3: Create `cross-cutting-spec-reviewer.md`

Combines Requirements + Design-judgment + Consistency observations.

**Files:**
- Create: `plugins/spec-review/agents/cross-cutting-spec-reviewer.md`
- Source: `plugins/spec-review/agents/spec-reviewer.md:20-46` (Requirements), `48-69` (Design judgment), `98-128` (Consistency)

- [ ] **Step 1: Write the file**

```markdown
---
name: cross-cutting-spec-reviewer
description: Cross-cutting spec reviewer covering Requirements, Design-judgment, and Consistency observations in one pass. Preserves internal self-dedup. Writes findings-cross-cutting-spec.json.
memory: project
effort: max
---

You are a consolidated cross-cutting spec reviewer. In a single pass, produce findings across three overlapping observations: Requirements, Design-judgment, and Consistency. These three observations overlap (implicit assumptions in requirements surface as codebase-alignment gaps in consistency; alternatives-evaluation in design-judgment overlaps with impact-analysis in consistency). Handling them in one context preserves self-dedup.

## Scope

Review the target spec document. Use Grep and Read to investigate the codebase for implicit business rules, existing patterns, and constraints referenced by the spec.

## Filtering

- Do not report issues with confidence below 80%.
- Consolidate duplicate issues into a single finding.
- **Internal self-dedup**: If the same issue is found across the three observations handled here, keep it under the most essential one within this agent (priority: Requirements > Design-judgment > Consistency — subset of the global actionability order `Feasibility > Requirements > Design-judgment > Consistency > UI-design`).

## Observation 1: Requirements

### Review Checklist

1. **Requirements clarity** — Whether requirements and goals are concrete enough to be implementable and verifiable. Watch for vague phrasing like "handle appropriately" or "improve performance." Concrete numbers, conditions, and behaviors must be defined.
2. **Implicit assumptions** — Enumerate the business rules and constraints the spec implicitly assumes. Investigate the codebase and verify whether related existing validations, conditional branches, and business logic are considered in the spec.

### Investigation Method

- Grep the codebase for every model, table, and class name that appears in the spec, and identify the related validations, callbacks, and scopes.
- Verify that the identified existing logic does not contradict the spec's assumptions and that no unaddressed constraints remain.

### Policy

#### REJECT criteria

- Functional requirements lack verifiable completion conditions → severity: high
- Three or more implicit assumptions are not stated in the spec → severity: high

#### WARNING criteria

- Missing concrete numbers or conditions (phrases like "large amounts of data" or "fast") → severity: medium
- Existing validations or conditional branches in the code that the spec does not consider → severity: medium

## Observation 2: Design judgment

### Review Checklist

1. **Design rationale** — Whether the rationale for why the chosen approach is optimal is presented. When the spec includes a comparison with alternatives considered during brainstorming, verify whether the decision rationale is sufficient. Whether trade-offs are made explicit.
2. **Requirements fulfillment** — Whether the design actually solves the problem it is meant to address. Whether the design covers not only the happy path but also edge cases and error paths. Whether success criteria are reflected in the design.

### Policy

#### REJECT criteria

- Technology selection without stated rationale → severity: high
- Only the happy path is considered; edge-case and error-path behavior is undefined → severity: high

#### WARNING criteria

- Shallow alternative evaluation → severity: medium
- Success criteria are not reflected in the design → severity: medium

## Observation 3: Consistency

### Review Checklist

1. **Codebase alignment** — Whether the design contradicts the existing code's structure and patterns. Whether the proposed file placement and module structure align with what exists.
2. **Unresolved markers** — Whether unresolved markers such as TODO, TBD, "needs confirmation", "assumption", or FIXME remain in the spec.
3. **Business logic gaps** — Whether unanswered business-logic questions remain. Whether important decisions are sidestepped with "we assume ..." phrasing.
4. **Naming conventions** — Whether proposed names align with the existing naming convention. Whether camelCase and snake_case are mixed.
5. **Architecture consistency** — Alignment with existing architectural patterns (layer structure, separation of concerns, directory layout).
6. **Impact analysis** — Whether the blast radius of the design change is sufficiently identified. Starting from the models, controllers, jobs, etc. being modified, investigate callers, dependents, and any code that references the same tables, and verify that the spec has not missed any affected sites.
7. **Impact Analysis section completeness** — Whether the spec includes an Impact Analysis section (Reverse Dependencies, Shared State, Implicit Contracts, Side Effect Risks) with each item described concretely. Entries must include specific file:line references, resource names, and scenarios. A Must-Verify Checklist must exist and enumerate items that are verifiable during implementation and testing. Use Grep/Read against the code to confirm each item's accuracy. Verify the Assumptions section does not contradict Implicit Contracts.

### Policy

#### REJECT criteria

- Design that contradicts the existing code's structure and patterns → severity: high
- Unresolved markers (TODO / TBD / needs confirmation) remain → severity: high
- Impact of the design change has gaps (callers or dependents are not identified) → severity: high
- The Impact Analysis section is missing or incomplete → severity: high
- Impact descriptions are abstract (no specific file:line references) → severity: high

#### WARNING criteria

- Naming convention mismatch → severity: medium
- Decisions sidestepped with "we assume ..." where the assumption is actually verifiable → severity: medium
- Must-Verify Checklist is missing → severity: medium

## Output Format

Write findings to `.belt/runs/{run_id}/review/findings-cross-cutting-spec.json`:

```json
{
  "observations": ["requirements", "design-judgment", "consistency"],
  "findings": [
    {
      "id": "<uuid>",
      "observation": "requirements|design-judgment|consistency",
      "severity": "critical|high|medium|low",
      "section": "<heading path>",
      "description": "...",
      "suggestion": "...",
      "source": "agent"
    }
  ]
}
```

- Emit at most 10 findings. If no findings, write the empty `findings` array with all three observations listed.

## Guardrails

- Do not modify the spec. Read-only.
- Do not invoke further subagents.
- Do not read other agents' `findings-*.json` files.
```

- [ ] **Step 2: Verify**

Run: `ls -l plugins/spec-review/agents/cross-cutting-spec-reviewer.md`

---

### Task C4: Commit Phase C (spec-review new agents)

- [ ] **Step 1: Commit**

```bash
git add plugins/spec-review/agents/feasibility-reviewer.md \
        plugins/spec-review/agents/ui-design-reviewer.md \
        plugins/spec-review/agents/cross-cutting-spec-reviewer.md
git commit -m "feat(spec-review): add per-observation reviewer agents

Decompose spec-reviewer.md into three observation agents (feasibility /
ui-design / cross-cutting-spec). ui-design agent has an explicit early-
exit when the spec contains no UI content. Parent SKILL.md rewrite and
old spec-reviewer.md deletion follow in Phase E.

Ref: docs/specs/2026-04-16-review-skills-subagent-boundary-design.md"
```

---

## Phase D: Code-review plugin — rewrite parent SKILL.md, delete old files

### Task D1: Rewrite `plugins/code-review/skills/code-review/SKILL.md`

**Files:**
- Modify: `plugins/code-review/skills/code-review/SKILL.md` (full rewrite)

- [ ] **Step 1: Replace file contents**

Overwrite `plugins/code-review/skills/code-review/SKILL.md` with:

```markdown
---
name: code-review
description: >-
  Multi-perspective code review with parallel observation subagents.
  Dispatches security, test, ai-antipattern, and cross-cutting reviewers
  in parallel; merges findings with severity-first + actionability-priority
  cross-agent dedup. --codex adds an adversarial pass via /codex:rescue.
argument-hint: "[--codex]"
---

# Code Review

Parent dispatcher for parallel multi-observation code review. This skill runs in the main context (no `context: fork`) because triage (user selection) and fix apply require user dialogue and direct Edit tool usage.

## Scope Detection

Determine the diff scope before dispatching observation agents:

1. If the current branch differs from `main` → `git diff main...HEAD`
2. Else if staged changes exist → `git diff --staged`
3. Else → report "no diff detected" and exit without dispatching.

Pass the diff summary (file list + line counts) as context to each observation agent.

## Impact Observation Context

If a design document exists in the current run's output directory (filename matches `*-design.md`), pass the Impact Analysis section content as additional context to the `cross-cutting-reviewer` agent's prompt. The Impact observation inside cross-cutting consumes it.

## Parallel Dispatch

Dispatch observation agents in parallel via the Agent (Task) tool. Send all Task calls in **one single message** with multiple tool-use blocks so they run concurrently:

- `Task(subagent_type: code-review:security-reviewer, prompt: <diff + path to write findings-security.json>)`
- `Task(subagent_type: code-review:test-reviewer, prompt: <diff + path to write findings-test.json>)`
- `Task(subagent_type: code-review:ai-antipattern-reviewer, prompt: <diff + path to write findings-ai-antipattern.json>)`
- `Task(subagent_type: code-review:cross-cutting-reviewer, prompt: <diff + optional design-doc Impact Analysis + path to write findings-cross-cutting.json>)`

If `--codex` is set, also invoke `/codex:rescue` in the same parallel batch with a review-specific prompt: supply the diff, the expected `findings-codex.json` format (same shape as observation agents, `source: "codex"`), and the output path `.belt/runs/<run_id>/review/findings-codex.json`.

All agents write to `.belt/runs/<run_id>/review/findings-<observation>.json`. Each file is independent — no race condition between agents.

Announce each dispatched agent (and Codex, if `--codex`) before sending.

## Merge + Cross-agent Dedup

After all agents complete:

1. Read each `findings-<observation>.json` file under `.belt/runs/<run_id>/review/`.
2. For each finding, determine if it is the same issue as a finding from another agent. Use file + line + description overlap as the primary signal (LLM judgment — when `file` + `line` match and descriptions share core vocabulary, treat as the same issue candidate).
3. For same-issue candidates, apply the dedup rule:
   - **Severity-first**: keep the finding with the highest severity (critical > high > medium > low).
   - **Tie-break on severity equality — observation priority (actionability order)**:
     `Security > Impact > Quality > Test > AI-antipattern > Performance > Simplification`
   - **Codex findings are NOT deduplicated**. If a Codex finding overlaps with another observation, keep both — Codex signal carries separate "external-provider adversarial" value.
4. Write the merged `.belt/runs/<run_id>/review/findings.json`:

```json
{
  "findings": [
    {
      "id": "<uuid>",
      "observation": "security|test|ai-antipattern|quality|performance|impact|simplification|codex",
      "severity": "critical|high|medium|low",
      "file": "<path>",
      "line": <integer or null>,
      "description": "...",
      "suggestion": "...",
      "source": "agent|codex"
    }
  ]
}
```

- Cap at 20 findings total. If more exist after dedup, keep the highest-severity ones and append a final `low`-severity finding of observation `quality` noting the truncation.
- If no findings at all, write `{"findings": []}`.

## Triage

Present all merged findings as a numbered list sorted by severity descending, then by observation priority. User selects which to fix by number. No dialogue phase (dialogue is reserved for spec-review).

## Fix apply

For each user-selected finding:
1. Read the file at the `file` / `line` location.
2. Apply the `suggestion` via the Edit tool in the main context.
3. Note in a scratch area which findings are applied (for verification).

Apply fixes serially to avoid merge conflicts in the same file.

## Verify (after fix)

1. `git diff` — confirm changes match approved findings scope.
2. Auto-detect and run the project linter:

| Indicator file | Linter command |
|---|---|
| `Cargo.toml` | `cargo clippy -- -D warnings` |
| `package.json` (has lint script) | `npm run lint` |
| `pyproject.toml` / `setup.cfg` | `ruff check .` or `flake8` |
| `go.mod` | `go vet ./...` |
| `Makefile` (has lint target) | `make lint` |

3. Auto-detect and run the project test suite:

| Indicator file | Test command |
|---|---|
| `Cargo.toml` | `cargo test` |
| `package.json` (has test script) | `npm test` |
| `pyproject.toml` | `pytest` |
| `go.mod` | `go test ./...` |
| `Makefile` (has test target) | `make test` |

4. If linter or tests fail → report honestly, do not suppress.

## Red Flags

**Never:**
- Modify code without user approval of findings.
- Change files outside the diff scope.
- Omit or filter findings before presenting to user (except via the severity-first + observation-priority dedup rule above).
- Suppress or hide test/linter failures.
- Attempt to read other agents' `findings-*.json` files inside any observation agent's prompt (those agents must stay self-contained).

**Always:**
- Announce each dispatched agent (and Codex, if `--codex`).
- Dispatch all observation agents in a single parallel batch (one message, multiple Task tool uses).
- Apply the dedup rule deterministically — severity first, observation priority only on tie.
- Preserve Codex findings as separate entries (no dedup into other observations).
- Run linter and tests after applying fixes.
- Apply fixes serially to avoid merge conflicts.
```

- [ ] **Step 2: Verify**

Run: `cat plugins/code-review/skills/code-review/SKILL.md | grep -c "findings-"`
Expected: ≥ 3 (references to findings-security / findings-test / findings-ai-antipattern / findings-cross-cutting / findings.json).

Run: `grep -c "Task" plugins/code-review/skills/code-review/SKILL.md`
Expected: ≥ 4 (one mention per dispatched agent).

---

### Task D2: Delete old files (code-reviewer.md, pipeline.yml, belt.toml)

**Files:**
- Delete: `plugins/code-review/agents/code-reviewer.md`
- Delete: `plugins/code-review/skills/code-review/pipeline.yml`
- Delete: `plugins/code-review/skills/code-review/belt.toml`

- [ ] **Step 1: Delete**

```bash
git rm plugins/code-review/agents/code-reviewer.md
git rm plugins/code-review/skills/code-review/pipeline.yml
git rm plugins/code-review/skills/code-review/belt.toml
```

- [ ] **Step 2: Verify**

Run: `ls plugins/code-review/`
Expected: No `agents/code-reviewer.md`, no `skills/code-review/pipeline.yml`, no `skills/code-review/belt.toml`.

Run: `ls plugins/code-review/agents/`
Expected: Only `security-reviewer.md`, `test-reviewer.md`, `ai-antipattern-reviewer.md`, `cross-cutting-reviewer.md`.

---

### Task D3: Run `review_skills_refresh.rs` lock tests

- [ ] **Step 1: Run**

Run: `cargo test -p belt-core --test review_skills_refresh`
Expected: All 6 tests PASS. Specifically:
- `review_plugins_pipeline_yml_is_deleted` — code-review side passes (spec-review side still fails, fixed in Phase E)
- `review_plugins_belt_toml_is_deleted` — same
- `review_plugins_legacy_consolidated_agent_is_deleted` — code-review side passes
- `review_plugins_new_observation_agents_exist` — code-review side passes
- `review_plugins_parent_skill_md_references_parallel_dispatch` — code-review side passes
- `legacy_per_observation_review_agent_files_are_removed` — always passes (untouched invariant)

Half of these will be yellow/red for spec-review until Phase E completes. That is expected.

---

### Task D4: Commit Phase D

- [ ] **Step 1: Commit**

```bash
git add plugins/code-review/skills/code-review/SKILL.md
git commit -m "feat(code-review): rewrite parent SKILL.md and delete pipeline.yml

Convert /code-review into a main-context parent dispatcher. Parallel
dispatch of security / test / ai-antipattern / cross-cutting observation
agents via Task tool; merge findings-*.json with severity-first +
actionability-priority dedup (Codex findings excluded from dedup). Delete
pipeline.yml, belt.toml, and agents/code-reviewer.md.

Ref: docs/specs/2026-04-16-review-skills-subagent-boundary-design.md"
```

---

## Phase E: Spec-review plugin — rewrite parent SKILL.md, delete old files

### Task E1: Rewrite `plugins/spec-review/skills/spec-review/SKILL.md`

**Files:**
- Modify: `plugins/spec-review/skills/spec-review/SKILL.md` (full rewrite)

- [ ] **Step 1: Replace file contents**

Overwrite `plugins/spec-review/skills/spec-review/SKILL.md` with:

```markdown
---
name: spec-review
description: >-
  Multi-perspective spec review with parallel observation subagents.
  Dispatches feasibility, ui-design, and cross-cutting-spec reviewers in
  parallel; merges findings with severity-first + actionability-priority
  dedup. Findings in requirements/design-judgment with high/medium severity
  enter a grill-me dialogue; everything else uses selection triage.
  --codex adds an adversarial pass via /codex:rescue.
argument-hint: "[--codex]"
---

# Spec Review

Parent dispatcher for parallel multi-observation spec review with grill-me dialogue on design-critical findings. This skill runs in the main context (no `context: fork`) because grill-me dialogue requires user interaction.

## Scope

Locate the target spec document (most recent `*-design.md` under `docs/`, or user-supplied path).

## Parallel Dispatch

Dispatch observation agents in parallel via the Agent (Task) tool. Send all Task calls in **one single message**:

- `Task(subagent_type: spec-review:feasibility-reviewer, prompt: <spec path + path to write findings-feasibility.json>)`
- `Task(subagent_type: spec-review:ui-design-reviewer, prompt: <spec path + path to write findings-ui-design.json>)` — agent will early-exit with zero findings if spec has no UI content
- `Task(subagent_type: spec-review:cross-cutting-spec-reviewer, prompt: <spec path + path to write findings-cross-cutting-spec.json>)`

If `--codex` is set, also invoke `/codex:rescue` in the same parallel batch with a review-specific prompt: supply the spec, expected findings format, and output path `.belt/runs/<run_id>/review/findings-codex.json`.

Announce each dispatched agent before sending.

## Merge + Cross-agent Dedup

After all agents complete:

1. Read each `findings-<observation>.json` file under `.belt/runs/<run_id>/review/`.
2. Determine same-issue candidates using `section` overlap + description vocabulary (LLM judgment).
3. Apply dedup rule:
   - **Severity-first**: keep highest severity.
   - **Tie-break — observation priority (actionability order)**:
     `Feasibility > Requirements > Design-judgment > Consistency > UI-design`
   - **Codex findings are NOT deduplicated** (same as code-review).
4. Write `.belt/runs/<run_id>/review/findings.json` (cap 20 findings).

## Triage

After merge, partition findings:

- **Grill-me group**: `observation ∈ {requirements, design-judgment}` AND `severity ∈ {high, medium}`
- **Selection group**: everything else (feasibility, consistency, ui-design, low-severity, codex)

Process grill-me group first, then selection group.

## Grill-me Dialogue (Grill-me group)

Principles (borrowed from `/grill-me`):
- **One question at a time** — present a single finding; do not batch
- **Orchestrator provides a recommended answer** for every question
- **Codebase-answerable questions are not asked** — use Read/Grep to resolve them, update the suggestion, and move on
- **Rounds are unlimited** — iterate until the user explicitly accepts, rejects, or says "enough / move on"
- **Decision-tree order** — if finding A's decision affects finding B's proposal, resolve A first

Loop (pseudo):
```
order = topologically_sort(grill_group, by decision dependency)
for finding in order:
    while not resolved:
        if finding is answerable by codebase inspection:
            explore with Read/Grep
            revise finding.suggestion
            continue
        present finding + recommended_answer to user
        response = await user
        if response in {"accept", "OK", "approved"}:
            finding.resolution = "accept"; break
        if response in {"reject", "skip"}:
            finding.resolution = "reject"; break
        if response in {"enough", "move on"}:
            finding.resolution = "accept_current"; break  # accept revised state
        revise finding.suggestion based on response
```

After the loop, every grill-group finding has `resolution ∈ {accept, reject, accept_current}`.

## Selection Group

Present selection-group findings as a numbered list sorted by severity descending. User picks by number which to fix.

## Fix apply

For each accepted / selected finding, apply the suggestion via Edit tool on the target spec document.

## Verify (after fix)

1. `git diff` — confirm only target spec files changed
2. Markdown syntax check — no broken links, headers, or list formatting
3. Cross-reference consistency — internal document links still resolve

## Red Flags

**Never:**
- Modify spec without user approval of findings
- Change files outside the review target
- Omit or filter findings before presenting to user (except via the dedup rule)
- Ask a user question that could be answered by inspecting the codebase
- Present multiple grill-group findings simultaneously
- Read other agents' `findings-*.json` from inside any observation agent

**Always:**
- Announce each dispatched agent (and Codex, if `--codex`)
- Dispatch observation agents in a single parallel batch
- Apply the dedup rule deterministically
- Preserve Codex findings (no dedup into other observations)
- Provide a recommended answer with every grill-me question
- Explore the codebase before asking user questions
- Honor the user's "enough / move on" signal without pushback
- Get explicit user approval before applying any fixes
- Run verification checks after fixes are applied
```

- [ ] **Step 2: Verify**

Run: `grep -c "Task\|findings-" plugins/spec-review/skills/spec-review/SKILL.md`
Expected: ≥ 6.

---

### Task E2: Delete old files (spec-reviewer.md, pipeline.yml, belt.toml)

- [ ] **Step 1: Delete**

```bash
git rm plugins/spec-review/agents/spec-reviewer.md
git rm plugins/spec-review/skills/spec-review/pipeline.yml
git rm plugins/spec-review/skills/spec-review/belt.toml
```

- [ ] **Step 2: Verify**

Run: `ls plugins/spec-review/agents/`
Expected: Only `feasibility-reviewer.md`, `ui-design-reviewer.md`, `cross-cutting-spec-reviewer.md`.

---

### Task E3: Run full `review_skills_refresh.rs` lock tests

- [ ] **Step 1: Run**

Run: `cargo test -p belt-core --test review_skills_refresh`
Expected: All 6 tests PASS (now covering both code-review and spec-review).

---

### Task E4: Commit Phase E

- [ ] **Step 1: Commit**

```bash
git add plugins/spec-review/skills/spec-review/SKILL.md
git commit -m "feat(spec-review): rewrite parent SKILL.md and delete pipeline.yml

Convert /spec-review into a main-context parent dispatcher. Parallel
dispatch of feasibility / ui-design / cross-cutting-spec observation
agents via Task tool; merge findings with severity-first + actionability-
priority dedup. grill-me dialogue preserved for requirements / design-
judgment high/medium findings; other findings use selection triage.
Delete pipeline.yml, belt.toml, and agents/spec-reviewer.md.

Ref: docs/specs/2026-04-16-review-skills-subagent-boundary-design.md"
```

---

## Phase F: Caller-side criteria updates

Tighten the `validate:` criteria of feature-dev and bug-fix pipelines that call `/code-review:code-review` or `/spec-review:spec-review`, so that triage + fix completion is checked by the caller.

### Task F1: Update `plugins/feature-dev/skills/feature-dev/criteria/code-review.md`

**Files:**
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/code-review.md`

- [ ] **Step 1: Read current file**

Run: `cat plugins/feature-dev/skills/feature-dev/criteria/code-review.md`
Note the existing criteria structure.

- [ ] **Step 2: Add / update criteria**

Append these validation criteria (if not already present) to the file, using the existing file's formatting style:

- The merged `findings.json` exists at `.belt/runs/<run_id>/review/findings.json`.
- User has triaged the findings: each finding in `findings.json` has a user-approved disposition (accepted for fix, or explicitly rejected).
- All findings marked for fix have been applied to the codebase (verifiable via `git diff`).
- Project linter (`cargo clippy --package <pkg> -- -D warnings` for Rust, equivalent per language) passes on the modified files.
- Project tests pass on the modified crates/packages.
- Phase narrative `.belt/runs/<run_id>/notes/phase-code-review.md` is written and documents: which findings were accepted, which were rejected (with reason), and any verification output.

- [ ] **Step 3: Verify**

Run: `grep -c "findings.json\|triage" plugins/feature-dev/skills/feature-dev/criteria/code-review.md`
Expected: ≥ 2.

No commit yet — commit at end of Phase F.

---

### Task F2: Update `plugins/feature-dev/skills/feature-dev/criteria/spec-review.md`

**Files:**
- Modify: `plugins/feature-dev/skills/feature-dev/criteria/spec-review.md`

- [ ] **Step 1: Append criteria**

Append (with existing style):

- The merged `findings.json` exists at `.belt/runs/<run_id>/review/findings.json`.
- Grill-me group findings each have a `resolution` (`accept` / `reject` / `accept_current`).
- Selection-group findings have a user-numbered selection recorded (either applied or explicitly skipped).
- All applied changes are confined to the target spec document(s) (verifiable via `git diff`).
- Internal markdown links in the spec still resolve after the applied fixes.

- [ ] **Step 2: Verify**

Run: `grep -c "grill-me\|findings.json" plugins/feature-dev/skills/feature-dev/criteria/spec-review.md`
Expected: ≥ 2.

---

### Task F3: Update `plugins/bug-fix/skills/bug-fix/criteria/code-review.md`

**Files:**
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/code-review.md`

- [ ] **Step 1: Apply the same additions as Task F1**

Append the criteria set from Task F1 to this file.

- [ ] **Step 2: Verify**

Run: `grep -c "findings.json\|triage" plugins/bug-fix/skills/bug-fix/criteria/code-review.md`
Expected: ≥ 2.

---

### Task F4: Update `plugins/bug-fix/skills/bug-fix/criteria/fix-plan-review.md`

**Files:**
- Modify: `plugins/bug-fix/skills/bug-fix/criteria/fix-plan-review.md`

- [ ] **Step 1: Apply the same additions as Task F2**

Append the criteria set from Task F2 (with `spec` → `fix-plan` substitution where appropriate).

- [ ] **Step 2: Verify**

Run: `grep -c "grill-me\|findings.json" plugins/bug-fix/skills/bug-fix/criteria/fix-plan-review.md`
Expected: ≥ 2.

---

### Task F5: Run caller-side lock tests + commit

- [ ] **Step 1: Run lock tests**

Run: `cargo test -p belt-core --test feature_dev_refresh --test bug_fix_refresh`
Expected: All PASS (criteria text additions do not change pipeline shape, only content).

- [ ] **Step 2: Commit**

```bash
git add plugins/feature-dev/skills/feature-dev/criteria/code-review.md \
        plugins/feature-dev/skills/feature-dev/criteria/spec-review.md \
        plugins/bug-fix/skills/bug-fix/criteria/code-review.md \
        plugins/bug-fix/skills/bug-fix/criteria/fix-plan-review.md
git commit -m "feat(criteria): require triage+fix completion for review phases

feature-dev and bug-fix now explicitly validate that callers of
/code-review and /spec-review have completed findings triage, applied
fixes, and passed linter/tests in the parent context. Moves the fix
responsibility from the review plugin (which no longer has a fix phase)
to the caller's phase criteria.

Ref: docs/specs/2026-04-16-review-skills-subagent-boundary-design.md"
```

---

## Phase G: Protocol doc + redesign memo

### Task G1: Update `skills/belt-agent/SKILL.md` invoke variant table

**Files:**
- Modify: `skills/belt-agent/SKILL.md:49-63` (Reading `phase.invoke` section)

- [ ] **Step 1: Replace the variant table**

Open `skills/belt-agent/SKILL.md`. Find the section "## Reading `phase.invoke`" and replace the 4-row table with this 2-row table:

```markdown
| Variant | Shape | Orchestrator action |
|---------|-------|---------------------|
| `skill` | `{ skill: "/name", args: { ... } }` | Invoke the Claude Code slash-command skill named in `invoke.skill`, passing `invoke.args` as parameters. |
| `pipeline` | `{ pipeline: "./path.yml", with: { ... } }` | Initialise a nested `belt-agent` run on the referenced sub-pipeline. Pass `with` as args. Treat the nested run as a black-box until it reports `completed`. |
```

Delete the `agent` and `agents` rows that were below `skill` in the old table.

Also add a historical note immediately below the table:

```markdown
> **Note (2026-04-16):** The `agent` and `agents` variants were removed. Agent dispatch is now a skill-layer concern: a parent skill uses the Task tool internally (or wraps `context: fork` + `agent:` in a child skill) to launch subagents, and `pipeline.yml` references only `invoke.skill`. See `docs/specs/2026-04-16-review-skills-subagent-boundary-design.md`.
```

- [ ] **Step 2: Update the "Well-known Config Keys" section**

In the same file, find the "## Well-known Config Keys" section. Update the obsolete config key list. Remove any mention of `config.iterations` or `config.agents` if still present. Add a note that `invoke.agents` / `invoke.iterations` are permanently removed.

- [ ] **Step 3: Verify**

Run: `grep -c "invoke.agent\|invoke.agents" skills/belt-agent/SKILL.md`
Expected: Matches only in the removal-note context (≤ 2 mentions, both documenting the removal).

---

### Task G2: Update `docs/specs/2026-04-06-belt-redesign.md` with partial revert memo

**Files:**
- Modify: `docs/specs/2026-04-06-belt-redesign.md`

- [ ] **Step 1: Append a revision note**

Append a section at the end of `docs/specs/2026-04-06-belt-redesign.md`:

```markdown
## Revision: 2026-04-16 — Invoker partial revert

The `Invoker::Agent` and `Invoker::Agents` variants and the `IterationsSpec` type introduced with BELT-32 (first-class Invoker) have been removed. The Invoker enum now contains only `Skill` and `Pipeline` variants.

The rest of BELT-32 — Invoker as a first-class typed field, Artifact types (`produces` / `consumes`), and the produces/consumes resolution machinery — is retained.

Agent dispatch is now exclusively a skill-layer concern: a parent skill (such as `/code-review`) uses the Task tool to launch observation subagents, or wraps `context: fork` + `agent:` in a child skill. `pipeline.yml` files no longer reference agents directly.

See: `docs/specs/2026-04-16-review-skills-subagent-boundary-design.md`
```

- [ ] **Step 2: Verify**

Run: `tail -20 docs/specs/2026-04-06-belt-redesign.md`
Expected: The revision note appears at the end.

---

### Task G3: Final workspace-wide verification + commit Phase G

- [ ] **Step 1: Workspace checks**

Run:
```bash
cargo fmt --package belt-core
cargo clippy --package belt-core --all-targets -- -D warnings
cargo clippy --workspace -- -D warnings
cargo test --workspace
```
Expected: All PASS.

If any clippy warning appears in belt-core, fix inline and re-run.

- [ ] **Step 2: Commit**

```bash
git add skills/belt-agent/SKILL.md docs/specs/2026-04-06-belt-redesign.md
git commit -m "docs(protocol): update belt-agent invoke table and redesign memo

Shrink the phase.invoke variant table to skill / pipeline. Append a
partial-revert note to the 2026-04-06 belt-redesign spec pointing to
the 2026-04-16 subagent-boundary design.

Ref: docs/specs/2026-04-16-review-skills-subagent-boundary-design.md"
```

---

## Phase H: Final verification

### Task H1: Full test matrix

- [ ] **Step 1: Belt-core all targets**

Run: `cargo test -p belt-core`
Expected: All PASS.

- [ ] **Step 2: Workspace test**

Run: `cargo test --workspace`
Expected: All PASS.

- [ ] **Step 3: Workspace clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: No warnings.

- [ ] **Step 4: Manual smoke — invoke `/code-review` dry run**

If possible in the user's local environment, trigger `/code-review` with no diff (or a trivial diff) and verify that:
- All four observation agents are dispatched in parallel.
- `.belt/runs/<run_id>/review/findings-*.json` files are created.
- Merged `findings.json` is produced.
- User is presented with a numbered list (or a "no findings" message).

If dry run is not practical, skip and document the limitation in the PR / commit body.

---

### Task H2: Memory updates (post-implementation)

Update project memory files to reflect the new state (per Knowledge Capture rule in CLAUDE.md).

**Files:**
- Modify: `/Users/nishikataseiichi/.claude/projects/-Users-nishikataseiichi-go-src-github-com-neko-neko-belt/memory/project_belt32_invoker_artifact.md`
- Modify: `/Users/nishikataseiichi/.claude/projects/-Users-nishikataseiichi-go-src-github-com-neko-neko-belt/memory/project_belt_core_model_shapes_2026_04_14.md`
- Modify: `/Users/nishikataseiichi/.claude/projects/-Users-nishikataseiichi-go-src-github-com-neko-neko-belt/memory/project_review_skills_refresh_2026_04_15.md`
- Modify: `/Users/nishikataseiichi/.claude/projects/-Users-nishikataseiichi-go-src-github-com-neko-neko-belt/memory/project_belt_agent_cli_shape.md`

- [ ] **Step 1: `project_belt32_invoker_artifact.md`**

Append:
```markdown
**2026-04-16 partial revert**: `Invoker::Agent` / `Invoker::Agents` / `IterationsSpec` 削除 (plan `docs/plans/2026-04-16-review-skills-subagent-boundary-plan.md`)。Skill / Pipeline variant と Artifact 体系は保持。pipeline.yml からの agent 露出は消えた。
```

- [ ] **Step 2: `project_belt_core_model_shapes_2026_04_14.md`**

Update the Invoker shape entry:
```markdown
Invoker enum (2026-04-16 以降): Skill { skill, args } / Pipeline { pipeline, with } の 2 variant。Agent / Agents / IterationsSpec は削除済み。
```

- [ ] **Step 3: `project_review_skills_refresh_2026_04_15.md`**

Append:
```markdown
**2026-04-16 進化**: 単一 reviewer subagent 構造を observation 分解 (independent parallel + cross-cutting integrated) + belt-core からの agent 概念除去 + pipeline.yml 廃止へ移行。Spec/Plan は `docs/specs/2026-04-16-review-skills-subagent-boundary-design.md` / `docs/plans/2026-04-16-review-skills-subagent-boundary-plan.md`。
```

- [ ] **Step 4: `project_belt_agent_cli_shape.md`**

Update the invoke variant description:
```markdown
**2026-04-16 以降**: invoke は skill / pipeline のみ。agent / agents variants は削除 (parse error + lint reject)。
```

- [ ] **Step 5: No git commit (memory files are outside the repo)**

Memory files live in `~/.claude/projects/.../memory/`, not in the belt repo. These are session-local files.

---

### Task H3: Self-contained commit log summary + optional PR

- [ ] **Step 1: Generate summary**

Run: `git log --oneline main..HEAD`
Expected: 7 feature commits (one per Phase A-G) plus one initial spec commit. Confirm commit messages are clean.

- [ ] **Step 2 (optional): Create PR**

If the user wants a PR:
```bash
gh pr create --title "Remove agent concept from belt-core; decompose review skills into per-observation subagents" --body "$(cat <<'EOF'
## Summary
- Shrink belt-core `Invoker` to `Skill` / `Pipeline` (partial revert of BELT-32 Agent/Agents first-class; Artifact types and Skill/Pipeline variants retained).
- Decompose `/code-review` into 4 observation agents (security / test / ai-antipattern / cross-cutting) dispatched in parallel by the parent skill.
- Decompose `/spec-review` into 3 observation agents (feasibility / ui-design / cross-cutting-spec) with grill-me dialogue for design-critical findings.
- Delete `pipeline.yml` / `belt.toml` from both review plugins. Parent skills run in the main context and dispatch via Task tool.
- `--codex` reuses existing `/codex:rescue` as an additional observation; no new `/codex:review` skill.
- Cross-agent dedup: severity-first + actionability-ordered observation priority. Codex findings excluded from dedup.
- feature-dev / bug-fix criteria now require triage + fix completion (verifiable by git diff + linter/test).

## Test plan
- [ ] cargo test --workspace passes
- [ ] cargo clippy --workspace -- -D warnings is clean
- [ ] review_skills_refresh.rs lock tests pass for both code-review and spec-review
- [ ] invoke.agent / invoke.agents / invoke.iterations keys produce YAML parse error AND lint error
- [ ] Manual smoke: /code-review dispatches 4 agents in parallel and writes findings-*.json
EOF
)"
```

Note: do NOT run `gh pr create` unless the user explicitly requests it.

---

## Self-Review Checklist (for plan author)

### Spec coverage

- [x] Invoker enum shrinking (Task A5, A6) covers spec §belt-core の変更 / §model.rs
- [x] IterationsSpec deletion (Task A6) covers spec §IterationsSpec 削除
- [x] Lint rule (Task A7) covers spec §lint.rs
- [x] Fixtures cleanup (Task A8) covers spec §fixtures 更新
- [x] belt-agent SKILL.md update (Task G1) covers spec §belt-agent/SKILL.md の更新
- [x] code-review agent decomposition (Tasks B1-B5) covers spec §code-review 観点分類
- [x] spec-review agent decomposition (Tasks C1-C4) covers spec §spec-review 観点分類
- [x] parent SKILL.md rewrite for both plugins (Tasks D1, E1) covers spec §Parent SKILL.md の責務
- [x] pipeline.yml / belt.toml deletion (Tasks D2, E2) covers spec §削除対象
- [x] Caller criteria updates (Tasks F1-F5) covers spec §呼び出し側 (feature-dev / bug-fix) の扱い
- [x] Redesign memo (Task G2) covers spec §Migration Order step 8 and Memory Updates
- [x] Memory updates (Task H2) covers spec §Memory Updates

### Placeholder scan

- No "TBD", "TODO", "implement later" in step bodies
- All code blocks are complete (no `...` placeholders in the content to be written)
- Ambiguous cases (e.g., "parse_pipeline_from_str does not exist") are flagged with exact fallback instructions

### Type consistency

- `Invoker::Skill { skill, args }` shape is consistent across all tasks
- `Invoker::Pipeline { pipeline, with }` shape is consistent
- `findings-<observation>.json` shape is consistent across agent definitions
- `observation` label strings match in agent files, SKILL.md, and dedup priority lists

---

## Execution Options

Plan complete. Commit this plan to git and confirm user choice of execution mode:

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per Phase task, review between tasks, fast iteration.
2. **Inline Execution** — execute tasks in this session using `executing-plans` skill, batch execution with checkpoints.

Wait for the user's choice before invoking the corresponding sub-skill.
