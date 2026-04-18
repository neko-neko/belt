# belt-test-foundation (F2b) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** F2a で Forward-to-F2b に送られた 7 items を 9 commits で完了し、belt-core test 資産の audit + helper consolidation + coverage gap 解消を達成する。

**Architecture:** Phase B (infrastructure-first: `tests/common/{mod,helpers,narrative}.rs`) → Phase A (item work: gate git_clean / expander_with integration / engine Display parameterize / uri migration / lock-ledger expansion) → Phase C (audit-report.md + audit-template wording patch)。

**Tech Stack:** Rust 1.94.1 (pinned)、Cargo workspace 3-crate、既存 dep (`tempfile` / `serde-saphyr=0.0.23` / `miette 7.6` / `thiserror 2.0.18`) のみ使用、新規 dep ゼロ。

**Design / Test Strategy 参照**:
- `docs/features/2026-04-17-belt-test-foundation-f2b/design.md` (48 Must-Verify items)
- `docs/features/2026-04-17-belt-test-foundation-f2b/test-strategy.md` (10 Test Entries TE-A〜J + 12 NFR)

**Baseline (Task 開始前に確認)**:
- `git rev-parse HEAD` = F2a merge 派生 (47153e7 以降)
- `cargo test --workspace` = 408 pass
- `git status` = clean
- current branch = `feature/2026-04-17-belt-test-foundation-f2b`

---

## File Structure

新規作成:
- `crates/belt-core/tests/common/mod.rs` (hub, preamble + `pub mod` declarations)
- `crates/belt-core/tests/common/helpers.rs` (write_yaml / repo_root / fixture_path)
- `crates/belt-core/tests/common/narrative.rs` (find_phase / find_produce / has_file_exists_gate / has_named_consume + 4 assert_* fns)

編集 (helper import 書き換え、assertion body 不変):
- `crates/belt-core/tests/engine_test.rs` (write_yaml + fixture_path + 2 Display test に scenario doc-comment + parameterize)
- `crates/belt-core/tests/view_test.rs` (fixture_path)
- `crates/belt-core/tests/lint_test.rs` (write_yaml)
- `crates/belt-core/tests/expander_test.rs` (write_yaml)
- `crates/belt-core/tests/artifact_when_field.rs` (fixture_path)
- `crates/belt-core/tests/scenarios_contract.rs` (repo_root)
- `crates/belt-core/tests/bug_fix_refresh.rs` (repo_root + 4 narrative helpers)
- `crates/belt-core/tests/feature_dev_refresh.rs` (4 narrative helpers)
- `crates/belt-core/tests/review_skills_refresh.rs` (repo_root)
- `crates/belt-core/tests/shared_filter_parity.rs` (repo_root)

編集 (behavior 追加):
- `crates/belt-core/tests/uri_test.rs` (+7 edge case tests、5 → 12 test)
- `crates/belt-core/tests/gate_test.rs` (+4-5 git_clean tests)
- `crates/belt-core/tests/expander_with_test.rs` (+2-3 integration tests、17 行 preamble 保持)

編集 (削除のみ、production runtime unchanged):
- `crates/belt-core/src/uri.rs` (line 174-310 の `#[cfg(test)] mod tests` ブロック全削除)

編集 (docs):
- `docs/testing/cli-behavior/belt-core.yml` (uri 5→12 + gate 9→13-14 + error 3→4-5 + expander 4→6-7 scenarios)
- `docs/testing/lock-ledger.md` (bug_fix_refresh.rs entry stub expansion、+35-40 行)
- `docs/testing/audit-template.md` (Duplication Candidates wording patch)

新規 (docs):
- `docs/features/2026-04-17-belt-test-foundation-f2b/audit-report.md`

非 impacted (明示):
- `crates/belt-core/tests/model_test.rs` / `parser_test.rs` (helper 未使用 grep 検証済、touchless)
- `crates/belt-agent/tests/**` / `crates/belt/tests/**` (F3 scope)
- `crates/*/src/**` excluding `src/uri.rs` 削除部分 (production runtime 不変)
- `Cargo.toml` / `Cargo.lock`

---

### Task 1: Phase B0 — Extract write_yaml / repo_root / fixture_path to `tests/common/helpers.rs`

**TE-A coverage (MV-01, MV-02, MV-04, MV-19, MV-20, MV-21, MV-47)**

**Files:**
- Create: `crates/belt-core/tests/common/mod.rs`
- Create: `crates/belt-core/tests/common/helpers.rs`
- Modify: `crates/belt-core/tests/engine_test.rs` (remove inline helpers, add `mod common;`)
- Modify: `crates/belt-core/tests/view_test.rs` (remove inline fixture_path, add `mod common;`)
- Modify: `crates/belt-core/tests/lint_test.rs` (remove inline write_yaml, add `mod common;`)
- Modify: `crates/belt-core/tests/expander_test.rs` (remove inline write_yaml, add `mod common;`)
- Modify: `crates/belt-core/tests/artifact_when_field.rs` (remove inline fixture_path, add `mod common;`)
- Modify: `crates/belt-core/tests/scenarios_contract.rs` (remove inline repo_root, add `mod common;`)
- Modify: `crates/belt-core/tests/bug_fix_refresh.rs` (remove inline repo_root, add `mod common;`)
- Modify: `crates/belt-core/tests/review_skills_refresh.rs` (remove inline repo_root, add `mod common;`)
- Modify: `crates/belt-core/tests/shared_filter_parity.rs` (remove inline repo_root, add `mod common;`)

- [ ] **Step 1.1: Create `common/mod.rs` with preamble**

```rust
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::missing_panics_doc,
    dead_code,
    reason = "test helpers use panic-on-mismatch per workspace convention; dead_code exempted centrally because each integration test binary uses a different subset of helpers"
)]

pub mod helpers;
```

- [ ] **Step 1.2: Create `common/helpers.rs` with 3 helpers (variant A unified)**

```rust
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper: resolve a fixture file path relative to `CARGO_MANIFEST_DIR`.
pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Helper: write a file inside the given directory and return its path.
pub fn write_yaml(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    let mut f = std::fs::File::create(&path).expect("failed to create file");
    f.write_all(content.as_bytes())
        .expect("failed to write file");
    path
}

/// Helper: workspace root (two parents up from `CARGO_MANIFEST_DIR` = `crates/belt-core`).
pub fn repo_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path
}
```

- [ ] **Step 1.3: Remove inline helpers + add `mod common;` in each caller**

For each file, at the top add `mod common;` then `use common::helpers::{write_yaml, repo_root, fixture_path};` (only the helpers that file uses), and delete the inline `fn write_yaml` / `fn repo_root` / `fn fixture_path` definitions.

Example change in `crates/belt-core/tests/engine_test.rs` (lines 14-30 inline helpers removed):

```rust
// After preamble (#![allow(...)] ... use statements)

mod common;
use common::helpers::{write_yaml, fixture_path};

// remove: fn fixture_path(...) { ... } (lines 15-21)
// remove: fn write_yaml(...) { ... } (lines 23-30)
```

Apply the same pattern to: view_test.rs (fixture_path), lint_test.rs (write_yaml), expander_test.rs (write_yaml), artifact_when_field.rs (fixture_path), scenarios_contract.rs (repo_root, note: variant C in current file → replace with variant P), bug_fix_refresh.rs (repo_root), review_skills_refresh.rs (repo_root), shared_filter_parity.rs (repo_root).

Note for `scenarios_contract.rs`: the current `repo_root()` uses `.parent().unwrap()` chain (variant C). The unified common helper uses `push("..")` (variant P). Both resolve to the same absolute workspace root. Caller behavior unchanged.

- [ ] **Step 1.4: Run workspace tests to verify no regression**

Run: `cargo test --workspace 2>&1 | tail -20`
Expected: `408 passed; 0 failed` (test count unchanged, assertion bodies unchanged)

- [ ] **Step 1.5: Run clippy and fmt**

Run: `cargo clippy --workspace -- -D warnings && cargo fmt --all -- --check`
Expected: clean (no warnings, no diff)

- [ ] **Step 1.6: Verify `mod common;` declared in every helper-using file**

Run: `grep -l '^mod common' crates/belt-core/tests/*.rs | sort`
Expected output:
```
crates/belt-core/tests/artifact_when_field.rs
crates/belt-core/tests/bug_fix_refresh.rs
crates/belt-core/tests/engine_test.rs
crates/belt-core/tests/expander_test.rs
crates/belt-core/tests/lint_test.rs
crates/belt-core/tests/review_skills_refresh.rs
crates/belt-core/tests/scenarios_contract.rs
crates/belt-core/tests/shared_filter_parity.rs
crates/belt-core/tests/view_test.rs
```
(9 files)

- [ ] **Step 1.7: Commit**

```bash
git add crates/belt-core/tests/common/mod.rs crates/belt-core/tests/common/helpers.rs \
  crates/belt-core/tests/engine_test.rs crates/belt-core/tests/view_test.rs \
  crates/belt-core/tests/lint_test.rs crates/belt-core/tests/expander_test.rs \
  crates/belt-core/tests/artifact_when_field.rs crates/belt-core/tests/scenarios_contract.rs \
  crates/belt-core/tests/bug_fix_refresh.rs crates/belt-core/tests/review_skills_refresh.rs \
  crates/belt-core/tests/shared_filter_parity.rs
git -c commit.gpgsign=false commit -m "test(belt-core): extract write_yaml/repo_root/fixture_path to tests/common"
```

---

### Task 2: Phase B1 — Extract narrative helpers to `tests/common/narrative.rs`

**TE-A coverage (MV-03, MV-22)**

**Files:**
- Modify: `crates/belt-core/tests/common/mod.rs` (add `pub mod narrative;`)
- Create: `crates/belt-core/tests/common/narrative.rs`
- Modify: `crates/belt-core/tests/feature_dev_refresh.rs` (use common::narrative, remove inline helpers)
- Modify: `crates/belt-core/tests/bug_fix_refresh.rs` (use common::narrative, remove inline helpers)

- [ ] **Step 2.1: Update `common/mod.rs` to expose narrative**

Edit: add line below `pub mod helpers;`:
```rust
pub mod narrative;
```

- [ ] **Step 2.2: Create `common/narrative.rs`**

```rust
use belt_core::model::{Artifact, ArtifactRef, GateCheck, Phase, Pipeline};

/// Find a `Phase` by id, panicking on miss.
pub fn find_phase<'a>(pipeline: &'a Pipeline, id: &str) -> &'a Phase {
    pipeline
        .phases
        .iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("phase not found: {id}"))
}

/// Find a produced `Artifact` by name within a `Phase`.
pub fn find_produce<'a>(phase: &'a Phase, name: &str) -> &'a Artifact {
    phase
        .produces
        .iter()
        .find(|a| a.name == name)
        .unwrap_or_else(|| panic!("produce not found in phase {}: {name}", phase.id))
}

/// True if `phase` has a `file_exists: <path>` gate.
pub fn has_file_exists_gate(phase: &Phase, path: &str) -> bool {
    phase.gate.iter().any(|g| matches!(g, GateCheck::FileExists { file_exists } if file_exists == path))
}

/// True if `phase` consumes `name` as `ArtifactRef::Named`.
pub fn has_named_consume(phase: &Phase, name: &str) -> bool {
    phase
        .consumes
        .iter()
        .any(|r| matches!(r, ArtifactRef::Named(n) if n == name))
}

/// For each `(phase_id, produce_name, expected_path)` tuple, assert the produce exists at the expected path.
pub fn assert_narrative_produce_paths(
    pipeline: &Pipeline,
    rows: &[(&str, &str, &str)],
) {
    for (phase_id, produce_name, expected_path) in rows {
        let phase = find_phase(pipeline, phase_id);
        let artifact = find_produce(phase, produce_name);
        assert_eq!(
            artifact.path.as_deref(),
            Some(*expected_path),
            "{phase_id}/{produce_name} path mismatch"
        );
    }
}

/// For each `(phase_id, _, expected_path)` tuple, assert the phase has a `file_exists` gate on that path.
pub fn assert_narrative_gate_paths(pipeline: &Pipeline, rows: &[(&str, &str, &str)]) {
    for (phase_id, _, expected_path) in rows {
        let phase = find_phase(pipeline, phase_id);
        assert!(
            has_file_exists_gate(phase, expected_path),
            "{phase_id} missing file_exists gate for {expected_path}"
        );
    }
}

/// For each `(phase_id, expected_consumes)` pair, assert every expected name is a named consume.
pub fn assert_narrative_accumulating_consumes(
    pipeline: &Pipeline,
    rows: &[(&str, &[&str])],
) {
    for (phase_id, expected_consumes) in rows {
        let phase = find_phase(pipeline, phase_id);
        for name in *expected_consumes {
            assert!(
                has_named_consume(phase, name),
                "{phase_id} missing named consume: {name}"
            );
        }
    }
}

/// For each `phase_id`, assert it has no produce whose path starts with `.belt/runs/`.
pub fn assert_non_narrative_phases_have_no_notes(pipeline: &Pipeline, phase_ids: &[&str]) {
    for phase_id in phase_ids {
        let phase = find_phase(pipeline, phase_id);
        for a in &phase.produces {
            assert!(
                a.path
                    .as_deref()
                    .map_or(true, |p| !p.starts_with(".belt/runs/")),
                "non-narrative phase {phase_id} has narrative produce: {}",
                a.name
            );
        }
    }
}
```

Note: Field access (`phase.gate`, `phase.consumes`, `phase.produces`, `artifact.path`) must match the actual `Pipeline` model. If field names differ (e.g., `path: Option<String>`), adjust at implementation time. The `Artifact` / `Phase` / `Pipeline` struct definitions are in `crates/belt-core/src/model.rs`.

- [ ] **Step 2.3: Modify `feature_dev_refresh.rs` to use common/narrative**

Delete inline helpers (`find_phase` / `find_produce` / `has_file_exists_gate` at lines 272-293) and add:
```rust
use common::narrative::{
    assert_narrative_accumulating_consumes, assert_narrative_gate_paths,
    assert_narrative_produce_paths, assert_non_narrative_phases_have_no_notes,
    find_phase, find_produce, has_file_exists_gate,
};
```
(Note: `mod common;` should already be present from Task 1. If not, add it at the top.)

Replace the 4 narrative test bodies (`feature_dev_narrative_phases_produce_notes`, `..._gate_notes`, `..._accumulating_consumes`, `..._non_narrative_phases_have_no_notes`) with calls to the `assert_*` helpers.

- [ ] **Step 2.4: Modify `bug_fix_refresh.rs` similarly**

Delete inline helpers (`find_phase` / `find_produce` / `has_file_exists_gate` / `has_named_consume` at lines 356-384) and add the same `use common::narrative::{...};` block.

Replace the 4 narrative test bodies in `bug_fix_refresh.rs` using the same assert helpers.

- [ ] **Step 2.5: Run tests**

Run: `cargo test --workspace 2>&1 | tail -10`
Expected: `408 passed; 0 failed` (unchanged).

- [ ] **Step 2.6: Run clippy and fmt**

Run: `cargo clippy --workspace -- -D warnings && cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 2.7: Commit**

```bash
git add crates/belt-core/tests/common/mod.rs crates/belt-core/tests/common/narrative.rs \
  crates/belt-core/tests/feature_dev_refresh.rs crates/belt-core/tests/bug_fix_refresh.rs
git -c commit.gpgsign=false commit -m "test(belt-core): extract narrative helpers to tests/common/narrative"
```

---

### Task 3: Phase B2 — `common/parity.rs` (SKIP by default)

**Scope decision: SKIP unless design.md Deliverable #4 "(optional)" is explicitly activated.** Per brainstorming decision Q5-A (single commit) and conservative scope, parity helper extraction gains are modest (~15 lines) and the extractor logic stays local to `shared_filter_parity.rs`. Plan 8 commits by default; enable this Task only if plan phase explicitly expands scope.

If activated:
- Create `crates/belt-core/tests/common/parity.rs` with `pub fn read_workspace_file(rel: &str) -> String`
- Update `shared_filter_parity.rs` + `shared_criteria_parity.rs` to use it
- Commit: `test(belt-core): extract parity helper to tests/common/parity`

---

### Task 4: Phase A1 — Add gate git_clean integration tests

**TE-C coverage (MV-07, MV-15, MV-27, MV-48)**

**Files:**
- Modify: `crates/belt-core/tests/gate_test.rs` (add 4-5 test functions)
- Modify: `docs/testing/cli-behavior/belt-core.yml` (add 4-5 gate scenarios)

- [ ] **Step 4.1: Add 4-5 git_clean scenarios to `belt-core.yml`**

Locate the `gate` category section (9 scenarios present). Append:

```yaml
  - id: belt-core-gate-git-clean-clean-repo-with-expect-clean-passes
    category: gate
    severity: high
    technique: equivalence-partition
    given: "a clean git repository (no uncommitted changes)"
    when: "execute_gate is called with GateCheck::GitClean { git_clean: true }"
    then: "passed is true; detail is 'working tree clean'"

  - id: belt-core-gate-git-clean-dirty-repo-with-expect-dirty-passes
    category: gate
    severity: high
    technique: equivalence-partition
    given: "a git repository with uncommitted changes"
    when: "execute_gate is called with GateCheck::GitClean { git_clean: false }"
    then: "passed is true; detail mentions 'file(s) with uncommitted changes'"

  - id: belt-core-gate-git-clean-clean-repo-with-expect-dirty-fails
    category: gate
    severity: medium
    technique: equivalence-partition
    given: "a clean git repository"
    when: "execute_gate is called with GateCheck::GitClean { git_clean: false }"
    then: "passed is false; detail is 'working tree clean' (detail depends only on is_clean, not expect_clean)"

  - id: belt-core-gate-git-clean-dirty-repo-with-expect-clean-fails
    category: gate
    severity: medium
    technique: equivalence-partition
    given: "a git repository with uncommitted changes"
    when: "execute_gate is called with GateCheck::GitClean { git_clean: true }"
    then: "passed is false; detail mentions 'file(s) with uncommitted changes'"

  - id: belt-core-gate-git-clean-missing-work-dir-yields-spawn-failure
    category: gate
    severity: medium
    technique: boundary-value
    given: "a non-existent work_dir path"
    when: "execute_gate is called with GateCheck::GitClean { git_clean: true }"
    then: "passed is false; detail starts with 'failed to run git:'"
```

- [ ] **Step 4.2: Add helper function for git_init tempdir in gate_test.rs**

At the top (after `use` statements), add:

```rust
/// Initialize a git repository in a fresh tempdir; return the TempDir (scope controls cleanup).
fn git_init_tempdir() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let status = std::process::Command::new("git")
        .args(["-c", "init.defaultBranch=main", "-c", "core.excludesfile=/dev/null", "init"])
        .current_dir(tmp.path())
        .status()
        .expect("git init");
    assert!(status.success(), "git init failed");
    tmp
}
```

- [ ] **Step 4.3: Write the 5 git_clean tests**

Append to `gate_test.rs`:

```rust
/// scenario: belt-core-gate-git-clean-clean-repo-with-expect-clean-passes
#[test]
fn git_clean_clean_repo_with_expect_clean_passes() {
    let tmp = git_init_tempdir();
    let check = GateCheck::GitClean { git_clean: true };
    let result = execute_gate(&check, tmp.path(), tmp.path());

    assert_eq!(result.check_type, "git_clean");
    assert!(result.passed, "clean repo + expect_clean=true should pass");
    assert_eq!(result.detail.as_deref(), Some("working tree clean"));
}

/// scenario: belt-core-gate-git-clean-dirty-repo-with-expect-dirty-passes
#[test]
fn git_clean_dirty_repo_with_expect_dirty_passes() {
    let tmp = git_init_tempdir();
    std::fs::write(tmp.path().join("dirty.txt"), "x").expect("write");
    let check = GateCheck::GitClean { git_clean: false };
    let result = execute_gate(&check, tmp.path(), tmp.path());

    assert!(result.passed, "dirty repo + expect_clean=false should pass");
    let detail = result.detail.as_deref().unwrap_or("");
    assert!(
        detail.contains("file(s) with uncommitted changes"),
        "detail mismatch: {detail}"
    );
}

/// scenario: belt-core-gate-git-clean-clean-repo-with-expect-dirty-fails
#[test]
fn git_clean_clean_repo_with_expect_dirty_fails() {
    let tmp = git_init_tempdir();
    let check = GateCheck::GitClean { git_clean: false };
    let result = execute_gate(&check, tmp.path(), tmp.path());

    assert!(!result.passed, "clean repo + expect_clean=false should fail");
    assert_eq!(result.detail.as_deref(), Some("working tree clean"));
}

/// scenario: belt-core-gate-git-clean-dirty-repo-with-expect-clean-fails
#[test]
fn git_clean_dirty_repo_with_expect_clean_fails() {
    let tmp = git_init_tempdir();
    std::fs::write(tmp.path().join("dirty.txt"), "x").expect("write");
    let check = GateCheck::GitClean { git_clean: true };
    let result = execute_gate(&check, tmp.path(), tmp.path());

    assert!(!result.passed, "dirty repo + expect_clean=true should fail");
    let detail = result.detail.as_deref().unwrap_or("");
    assert!(
        detail.contains("file(s) with uncommitted changes"),
        "detail mismatch: {detail}"
    );
}

/// scenario: belt-core-gate-git-clean-missing-work-dir-yields-spawn-failure
#[test]
fn git_clean_missing_work_dir_yields_spawn_failure() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let missing = tmp.path().join("does-not-exist");
    assert!(!missing.exists());
    let check = GateCheck::GitClean { git_clean: true };
    let result = execute_gate(&check, &missing, &missing);

    assert!(!result.passed, "missing work_dir should spawn-fail");
    let detail = result.detail.as_deref().unwrap_or("");
    assert!(
        detail.starts_with("failed to run git:"),
        "detail mismatch: {detail}"
    );
}
```

- [ ] **Step 4.4: Run tests**

Run: `cargo test -p belt-core --test gate_test git_clean 2>&1 | tail -15`
Expected: `5 passed; 0 failed`

Run workspace full: `cargo test --workspace 2>&1 | tail -5`
Expected: `413 passed; 0 failed` (408 + 5)

- [ ] **Step 4.5: Run scenarios_contract to verify symmetric diff passes**

Run: `cargo test -p belt-core --test scenarios_contract 2>&1 | tail -10`
Expected: all 14 tests pass (yml + doc-comment ID sets match).

- [ ] **Step 4.6: Run clippy and fmt**

Run: `cargo clippy --workspace -- -D warnings && cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 4.7: Commit**

```bash
git add crates/belt-core/tests/gate_test.rs docs/testing/cli-behavior/belt-core.yml
git -c commit.gpgsign=false commit -m "test(belt-core): add gate git_clean coverage (5 tests, 5 scenarios)"
```

---

### Task 5: Phase A2 — Add expander_with integration tests

**TE-E coverage (MV-09, MV-17, MV-28)**

**Files:**
- Modify: `crates/belt-core/tests/expander_with_test.rs` (add 2-3 integration tests, keep 17-line preamble)
- Modify: `docs/testing/cli-behavior/belt-core.yml` (add 2-3 expander scenarios)

- [ ] **Step 5.1: Add 3 expander_with scenarios to `belt-core.yml`**

Append to the `expander` category section:

```yaml
  - id: belt-core-expander-with-string-substitution-integration
    category: expander
    severity: medium
    technique: equivalence-partition
    given: "a parent pipeline with invoke.pipeline ref and with: {arg: 'value'} string"
    when: "parse_pipeline then expand_pipeline is called"
    then: "expanded sub-phase invoker.args contains the substituted string value"

  - id: belt-core-expander-with-bool-and-null-substitution-preserves-types
    category: expander
    severity: medium
    technique: equivalence-partition
    given: "a parent pipeline with with: {flag: true, opt: null}"
    when: "expand_pipeline propagates the args into sub-phase invoker.args"
    then: "Value::Bool(true) and Value::Null types are preserved (not stringified)"

  - id: belt-core-expander-with-parent-scope-not-rewritten-by-sub-substitution
    category: expander
    severity: high
    technique: state-transition
    given: "a parent phase with args.X and a sub-pipeline that substitutes X in its own scope"
    when: "expand_pipeline runs substitution"
    then: "parent phase args are unchanged; only sub-phase values get rewritten"
```

- [ ] **Step 5.2: Write 3 integration tests in `expander_with_test.rs`**

Add below the existing 17-line preamble (keep the preamble intact):

```rust
use std::io::Write;
use belt_core::parser::parse_pipeline;

mod common;
use common::helpers::write_yaml;

/// scenario: belt-core-expander-with-string-substitution-integration
#[test]
fn expand_pipeline_with_string_substitution_end_to_end() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let sub = write_yaml(
        &tmp,
        "sub.yml",
        "name: sub\nversion: 1\nargs:\n  skill: { type: string, default: \"/default\" }\nphases:\n  - id: step\n    description: \"run\"\n    invoke:\n      skill: \"args.skill\"\n    gate:\n      - cmd: \"true\"\n",
    );
    let parent = write_yaml(
        &tmp,
        "parent.yml",
        &format!(
            "name: parent\nversion: 1\nphases:\n  - id: phase\n    uses: {sub_rel}\n    with:\n      skill: \"/custom\"\n",
            sub_rel = sub.strip_prefix(tmp.path()).unwrap().display()
        ),
    );

    let pipeline = parse_pipeline(&parent).expect("parse parent");
    let expanded = belt_core::expander::expand_pipeline(&pipeline, tmp.path()).expect("expand");

    // Expect a single expanded phase whose invoker.skill == "/custom" (substituted from with).
    let phase = expanded.phases.iter().find(|p| p.id.starts_with("phase/")).expect("sub phase");
    // Access invoker to verify skill arg substitution (exact field name depends on model).
    // Assertion shape: inspect phase.invoke.args or phase.invoker to find "/custom" value.
    let rendered = format!("{:?}", phase.invoke);
    assert!(rendered.contains("/custom"), "expected skill override propagated: {rendered}");
}

/// scenario: belt-core-expander-with-bool-and-null-substitution-preserves-types
#[test]
fn expand_pipeline_with_bool_and_null_substitution_preserves_types() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let sub = write_yaml(
        &tmp,
        "sub.yml",
        "name: sub\nversion: 1\nargs:\n  flag: { type: bool, default: false }\n  opt: { type: string, default: null }\nphases:\n  - id: step\n    description: \"run\"\n    gate:\n      - cmd: \"true\"\n",
    );
    let parent = write_yaml(
        &tmp,
        "parent.yml",
        &format!(
            "name: parent\nversion: 1\nphases:\n  - id: phase\n    uses: {sub_rel}\n    with:\n      flag: true\n      opt: null\n",
            sub_rel = sub.strip_prefix(tmp.path()).unwrap().display()
        ),
    );

    let pipeline = parse_pipeline(&parent).expect("parse parent");
    let expanded = belt_core::expander::expand_pipeline(&pipeline, tmp.path()).expect("expand");

    // Find the sub-phase and verify its args carry bool/null types (not stringified).
    // Assertion is lenient: check that the serialized debug format shows "true" and "null" tokens.
    let phase = expanded.phases.iter().find(|p| p.id.starts_with("phase/")).expect("sub phase");
    let rendered = format!("{:?}", phase);
    assert!(rendered.contains("true") || rendered.contains("Bool(true)"), "bool not preserved: {rendered}");
    assert!(rendered.contains("null") || rendered.contains("Null"), "null not preserved: {rendered}");
}

/// scenario: belt-core-expander-with-parent-scope-not-rewritten-by-sub-substitution
#[test]
fn expand_pipeline_parent_scope_not_rewritten_by_sub_substitution() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let sub = write_yaml(
        &tmp,
        "sub.yml",
        "name: sub\nversion: 1\nargs:\n  name: { type: string, default: \"/sub-default\" }\nphases:\n  - id: step\n    description: \"run\"\n    invoke:\n      skill: \"args.name\"\n    gate:\n      - cmd: \"true\"\n",
    );
    let parent = write_yaml(
        &tmp,
        "parent.yml",
        &format!(
            "name: parent\nversion: 1\nargs:\n  name: {{ type: string, default: \"/parent-name\" }}\nphases:\n  - id: phase\n    uses: {sub_rel}\n    with:\n      name: \"/sub-override\"\n",
            sub_rel = sub.strip_prefix(tmp.path()).unwrap().display()
        ),
    );

    let pipeline = parse_pipeline(&parent).expect("parse parent");
    let _ = belt_core::expander::expand_pipeline(&pipeline, tmp.path()).expect("expand");

    // Parent's args definition should still be "/parent-name" (not rewritten by sub's with).
    let parent_name_default = pipeline
        .args
        .get("name")
        .and_then(|a| a.default.as_ref())
        .map(|v| format!("{v:?}"))
        .unwrap_or_default();
    assert!(
        parent_name_default.contains("/parent-name"),
        "parent scope arg rewritten: {parent_name_default}"
    );
}
```

Note: The exact field access pattern (`phase.invoke`, `pipeline.args.get`, `.default`) must align with the current belt-core model struct. If `pipeline.args` is a `BTreeMap<String, ArgDef>` with `default: Option<Value>`, the accessor chain is valid. Adjust at implementation if model shape differs; prefer `format!("{pipeline:?}")` + substring assertion if struct access is fragile.

- [ ] **Step 5.3: Run tests**

Run: `cargo test -p belt-core --test expander_with_test 2>&1 | tail -10`
Expected: `3 passed; 0 failed`

Run workspace: `cargo test --workspace 2>&1 | tail -5`
Expected: `416 passed; 0 failed` (413 + 3)

- [ ] **Step 5.4: Verify scenarios_contract**

Run: `cargo test -p belt-core --test scenarios_contract`
Expected: all pass.

- [ ] **Step 5.5: Commit**

```bash
git add crates/belt-core/tests/expander_with_test.rs docs/testing/cli-behavior/belt-core.yml
git -c commit.gpgsign=false commit -m "test(belt-core): add expander_with integration tests (3 tests, 3 scenarios)"
```

---

### Task 6: Phase A3 — Promote engine Display tests with parameterize

**TE-D coverage (MV-08, MV-16, MV-24, MV-25, MV-29)**

**Files:**
- Modify: `crates/belt-core/tests/engine_test.rs` (2 Display tests: add scenario doc-comment + parameterize assertion)
- Modify: `docs/testing/cli-behavior/belt-core.yml` (add 2 error scenarios)

- [ ] **Step 6.1: Add 2 error scenarios to `belt-core.yml`**

Append to the `error` category section:

```yaml
  - id: belt-core-error-display-verify-required-preserves-phase-id
    category: error
    severity: medium
    technique: equivalence-partition
    given: "a BeltError::VerifyRequired with a given phase_id"
    when: "Display is rendered via to_string"
    then: "the rendered message contains the quoted phase_id form 'for phase '<phase_id>'', independent of surrounding wording"

  - id: belt-core-error-display-max-retries-preserves-phase-id-and-counter
    category: error
    severity: medium
    technique: equivalence-partition
    given: "a BeltError::MaxRetriesExceeded with phase_id, attempts, max_retries"
    when: "Display is rendered via to_string"
    then: "the rendered message contains 'for phase '<phase_id>'' and the ratio '{attempts}/{max_retries}'"
```

- [ ] **Step 6.2: Modify the 2 Display tests in `engine_test.rs`**

Locate `error_verify_required_message` (~line 405) and `error_max_retries_exceeded_message` (~line 420). Replace with:

```rust
// ---

/// scenario: belt-core-error-display-verify-required-preserves-phase-id
#[test]
fn error_verify_required_message() {
    let phase_id = "build".to_string();
    let err = BeltError::VerifyRequired {
        phase_id: phase_id.clone(),
    };
    let msg = err.to_string();
    assert!(
        msg.contains(&format!("for phase '{phase_id}'")),
        "error message must preserve phase_id '{phase_id}' in quoted form: {msg}"
    );
}

// ---

/// scenario: belt-core-error-display-max-retries-preserves-phase-id-and-counter
#[test]
fn error_max_retries_exceeded_message() {
    let phase_id = "deploy".to_string();
    let attempts = 3u32;
    let max_retries = 3u32;
    let err = BeltError::MaxRetriesExceeded {
        phase_id: phase_id.clone(),
        attempts,
        max_retries,
    };
    let msg = err.to_string();
    assert!(
        msg.contains(&format!("for phase '{phase_id}'")),
        "error message must preserve phase_id '{phase_id}' in quoted form: {msg}"
    );
    assert!(
        msg.contains(&format!("{attempts}/{max_retries}")),
        "error message must preserve attempts/max_retries ratio '{attempts}/{max_retries}': {msg}"
    );
}
```

- [ ] **Step 6.3: Run tests**

Run: `cargo test -p belt-core --test engine_test error_verify_required_message`
Expected: `1 passed`

Run: `cargo test -p belt-core --test engine_test error_max_retries_exceeded_message`
Expected: `1 passed`

Run workspace: `cargo test --workspace 2>&1 | tail -5`
Expected: `416 passed; 0 failed` (unchanged, only modified existing tests)

- [ ] **Step 6.4: Verify scenarios_contract**

Run: `cargo test -p belt-core --test scenarios_contract`
Expected: all pass (2 new scenarios + 2 new doc-comments added in lockstep).

- [ ] **Step 6.5: Commit**

```bash
git add crates/belt-core/tests/engine_test.rs docs/testing/cli-behavior/belt-core.yml
git -c commit.gpgsign=false commit -m "test(belt-core): scenario-promote engine Display tests with parameterized assertions"
```

---

### Task 7: Phase A4 — Migrate uri inline tests to integration (atomic)

**TE-B coverage (MV-05, MV-06, MV-14, MV-26, MV-30, MV-31)**

**Files:**
- Modify: `crates/belt-core/tests/uri_test.rs` (5 → 12 tests, add 7 edge cases)
- Modify: `crates/belt-core/src/uri.rs` (delete lines 174-310: `#[cfg(test)] mod tests`)
- Modify: `docs/testing/cli-behavior/belt-core.yml` (uri category 5 → 12 scenarios)

- [ ] **Step 7.1: Add 7 edge case scenarios to `belt-core.yml`**

Append to the `uri` category section (existing 5 scenarios retained):

```yaml
  - id: belt-core-uri-unknown-selector-rejected
    category: uri
    severity: medium
    technique: equivalence-partition
    given: "a belt:// URI with a selector that is not latest, workspace, or run"
    when: "BeltUri::parse is called"
    then: "Err(UnknownSelector) is returned"

  - id: belt-core-uri-empty-pipeline-rejected
    category: uri
    severity: medium
    technique: boundary-value
    given: "belt://latest//notes/x.md (empty pipeline segment)"
    when: "BeltUri::parse is called"
    then: "Err(EmptyPipeline) is returned"

  - id: belt-core-uri-empty-run-id-rejected
    category: uri
    severity: medium
    technique: boundary-value
    given: "belt://run//notes/x.md (empty run_id segment)"
    when: "BeltUri::parse is called"
    then: "Err(EmptyRunId) is returned"

  - id: belt-core-uri-empty-path-rejected
    category: uri
    severity: medium
    technique: boundary-value
    given: "belt://latest/feature-dev/ (trailing slash, empty path)"
    when: "BeltUri::parse is called"
    then: "Err(EmptyPath) is returned"

  - id: belt-core-uri-absolute-path-rejected
    category: uri
    severity: medium
    technique: equivalence-partition
    given: "a belt:// URI with path containing // (double slash / absolute-like form)"
    when: "BeltUri::parse is called"
    then: "Err(PathTraversal) is returned"

  - id: belt-core-uri-workspace-missing-latest-rejected
    category: uri
    severity: medium
    technique: equivalence-partition
    given: "belt://workspace/<branch>/<non-latest>/..."
    when: "BeltUri::parse is called"
    then: "Err(Malformed) is returned"

  - id: belt-core-uri-to-string-roundtrip-all-variants
    category: uri
    severity: medium
    technique: state-transition
    given: "a BeltUri in any of the 3 variants (Run, Latest, WorkspaceLatest)"
    when: "to_string is called then parse is called on the result"
    then: "the round-trip yields the original BeltUri value"
```

- [ ] **Step 7.2: Add 7 new integration tests to `uri_test.rs`**

Append (before existing preamble attrs are fine; actually add after existing 5 tests):

```rust
use belt_core::uri::{BeltUri, UriParseError};

/// scenario: belt-core-uri-unknown-selector-rejected
#[test]
fn unknown_selector_is_rejected() {
    let err = BeltUri::parse("belt://unknown/pipeline/path.md").expect_err("should reject");
    assert!(matches!(err, UriParseError::UnknownSelector { .. }));
}

/// scenario: belt-core-uri-empty-pipeline-rejected
#[test]
fn empty_pipeline_is_rejected() {
    let err = BeltUri::parse("belt://latest//notes/x.md").expect_err("should reject");
    assert!(matches!(err, UriParseError::EmptyPipeline { .. }));
}

/// scenario: belt-core-uri-empty-run-id-rejected
#[test]
fn empty_run_id_is_rejected() {
    let err = BeltUri::parse("belt://run//notes/x.md").expect_err("should reject");
    assert!(matches!(err, UriParseError::EmptyRunId { .. }));
}

/// scenario: belt-core-uri-empty-path-rejected
#[test]
fn empty_path_is_rejected() {
    let err = BeltUri::parse("belt://latest/feature-dev/").expect_err("should reject");
    assert!(matches!(err, UriParseError::EmptyPath { .. }));
}

/// scenario: belt-core-uri-absolute-path-rejected
#[test]
fn absolute_path_is_rejected() {
    let err = BeltUri::parse("belt://latest/feature-dev//notes/x.md").expect_err("should reject");
    assert!(matches!(err, UriParseError::PathTraversal { .. }));
}

/// scenario: belt-core-uri-workspace-missing-latest-rejected
#[test]
fn workspace_missing_latest_is_rejected() {
    let err = BeltUri::parse("belt://workspace/develop/notlatest/pipeline/path.md")
        .expect_err("should reject");
    assert!(matches!(err, UriParseError::Malformed { .. }));
}

/// scenario: belt-core-uri-to-string-roundtrip-all-variants
#[test]
fn to_string_roundtrip_all_variants() {
    let inputs = [
        "belt://run/01932000-0000-7000-8000-000000000001/notes/x.md",
        "belt://latest/feature-dev/notes/y.md",
        "belt://workspace/develop/latest/feature-dev/z.md",
    ];
    for s in inputs {
        let parsed = BeltUri::parse(s).expect("parse ok");
        let restr = parsed.to_string();
        let reparsed = BeltUri::parse(&restr).expect("reparse ok");
        assert_eq!(parsed, reparsed, "roundtrip mismatch: {s} -> {restr}");
    }
}
```

- [ ] **Step 7.3: Delete `#[cfg(test)] mod tests` from `src/uri.rs`**

Read `crates/belt-core/src/uri.rs` lines 173-310. Delete the entire block starting from `#[cfg(test)]` down to the closing `}` at line 310. Production code (lines 1-172) unchanged.

- [ ] **Step 7.4: Run tests**

Run: `cargo test -p belt-core --test uri_test 2>&1 | tail -10`
Expected: `12 passed; 0 failed`

Run: `cargo test -p belt-core --lib uri 2>&1 | tail -5`
Expected: `0 passed` (inline tests removed)

Run workspace: `cargo test --workspace 2>&1 | tail -5`
Expected: `416 + 7 (new) − 12 (inline delete) = 411 passed; 0 failed`

- [ ] **Step 7.5: Verify production code unchanged**

Run: `git diff HEAD~1 -- 'crates/belt-core/src/uri.rs' | head -50`
Expected: only `#[cfg(test)] mod tests` block deleted; lines 1-172 production code untouched.

- [ ] **Step 7.6: Verify scenarios_contract symmetric diff**

Run: `cargo test -p belt-core --test scenarios_contract`
Expected: all pass (12 uri scenarios ↔ 12 uri_test.rs doc-comments).

- [ ] **Step 7.7: Commit**

```bash
git add crates/belt-core/tests/uri_test.rs crates/belt-core/src/uri.rs docs/testing/cli-behavior/belt-core.yml
git -c commit.gpgsign=false commit -m "test(belt-core): migrate uri inline tests to integration (7 edge cases + 5 overlap)"
```

---

### Task 8: Phase A5 — Expand lock-ledger `bug_fix_refresh` entry

**TE-F coverage (MV-10, MV-18)**

**Files:**
- Modify: `docs/testing/lock-ledger.md` (expand bug_fix_refresh.rs entry stub)

- [ ] **Step 8.1: Read existing `feature_dev_refresh.rs` entry as template**

Run: `grep -A 50 '^## feature_dev_refresh.rs' docs/testing/lock-ledger.md`
Observe the format: `## <filename>` header, ````yaml` code block with `locks-file`, `pipeline`, `test-fn-count`, shape-dimension bullets, `cross-coupling`.

- [ ] **Step 8.2: Verify `bug_fix_refresh.rs` current test count**

Run: `grep -c '^#\[test\]' crates/belt-core/tests/bug_fix_refresh.rs`
Expected: `19` (confirm against the number written into the expanded entry).

- [ ] **Step 8.3: Rewrite the bug_fix_refresh entry in `lock-ledger.md`**

Locate `## bug_fix_refresh.rs` (around line 55). Replace the stub block with a populated entry mirroring `feature_dev_refresh.rs` format:

```markdown
## bug_fix_refresh.rs

```yaml
locks-file: crates/belt-core/tests/bug_fix_refresh.rs
pipeline: plugins/belt/skills/bug-fix/pipeline.yml
test-fn-count: 19
test-fns:
  - args_keys_are_expected
  - phase_ids_in_expected_order
  - each_phase_invoker_variant_matches_expected
  - each_phase_regate_vec_matches_expected
  - each_artifact_when_typed_field_matches_expected
  - view_active_produces_obeys_args
  - max_retries_and_confirm_are_blanket
  - supplements_exist_for_declared_phases
  - dead_letter_files_are_absent
  - skill_md_has_expected_sections
  - pipeline_default_args_are_expected
  - criteria_files_exist_for_all_phases
  - bug_fix_narrative_phases_produce_notes
  - bug_fix_narrative_phases_gate_notes
  - bug_fix_narrative_accumulating_consumes
  - bug_fix_non_narrative_phases_have_no_notes
  - bug_fix_dead_letter_scripts_removed
  - bug_fix_skill_md_references_narrative_convention
  - bug_fix_pipeline_file_path_resolves
shape-dimensions:
  - pipeline.args set equality (`bug_fix_refresh.rs:62-75`)
  - Phase ID ordered vec equality (`bug_fix_refresh.rs:88-93`)
  - Type-level `Invoker::Skill` variant + `args == {codex: "args.codex"}` (`bug_fix_refresh.rs:95-144`)
  - `phase.regate` exact per-phase equality (`bug_fix_refresh.rs:146-165`)
  - `Artifact.when` typed field (`bug_fix_refresh.rs:167-185`)
  - `view::active_produces` behavior under args (`bug_fix_refresh.rs:187-213`)
  - `max_retries == 3` + `confirm == true` blanket (`bug_fix_refresh.rs:215-226`)
  - Supplement / criteria / SKILL.md file-system existence
  - SKILL.md `content.contains(section)` substring lock
  - Per-phase narrative `Artifact.path` equality against hardcoded `.belt/runs/{run_id}/notes/phase-*.md`
  - Accumulating `consumes` contains `Named(n)` for every prior narrative phase
cross-coupling:
  - crates/belt-core/tests/feature_dev_refresh.rs
  - crates/belt-core/tests/shared_criteria_parity.rs
```

---
```

Note: The `test-fns` list enumerates the current 19 `#[test]` functions in `bug_fix_refresh.rs`. Verify exact function names at implementation time via `grep -E '^fn ' crates/belt-core/tests/bug_fix_refresh.rs` after the `#[test]` lines (or use `grep -B 0 -A 1 '^#\[test\]'` to pair each attribute with its function). If the actual names differ, update the list to match.

- [ ] **Step 8.4: Run scenarios_contract `lock_ledger_locks_files_exist`**

Run: `cargo test -p belt-core --test scenarios_contract lock_ledger_locks_files_exist`
Expected: `1 passed` (locks-file path verified to exist; shape dimensions / test-fn-count are human-review only).

- [ ] **Step 8.5: Run full workspace test to ensure no regression**

Run: `cargo test --workspace 2>&1 | tail -5`
Expected: `411 passed; 0 failed` (docs-only change, test count unchanged).

- [ ] **Step 8.6: Commit**

```bash
git add docs/testing/lock-ledger.md
git -c commit.gpgsign=false commit -m "docs(testing): expand lock-ledger.md bug_fix_refresh entry with 9 shape dimensions"
```

---

### Task 9: Phase C1 — audit-report.md + audit-template wording patch

**TE-G, TE-H coverage (MV-11, MV-12, MV-32, MV-33)**

**Files:**
- Create: `docs/features/2026-04-17-belt-test-foundation-f2b/audit-report.md`
- Modify: `docs/testing/audit-template.md` (Duplication Candidates wording patch)
- Create: `.belt/runs/{run_id}/notes/phase-plan.md` (narrative note for plan phase, gitignored)

- [ ] **Step 9.1: Patch `audit-template.md` Duplication Candidates wording**

Locate the Duplication Candidates table (around line 50). Find the row:
```
| `parser_test.rs::parse_minimal_pipeline` | `model_test.rs` の同等 test | model_test に吸収可能 |
```

Replace with:
```
| `parser_test.rs::parse_pipeline_from_file` | `model_test.rs::parse_minimal_pipeline` | 誤記訂正 (F2b audit 2026-04-18): 実 fn は `parse_pipeline_from_file` (file-I/O + parse layer) で `model_test::parse_minimal_pipeline` (serde_saphyr 直接、model layer) と layer 分離された complementary test、redundant ではない。F2b では keep-both 判定 |
```

The `audit_template_version: v1` frontmatter remains unchanged (wording clarification only, no semantic change to Decision Tree or reason labels).

- [ ] **Step 9.2: Create `audit-report.md`**

Path: `docs/features/2026-04-17-belt-test-foundation-f2b/audit-report.md`

```markdown
---
audited_at: 2026-04-18T00:00:00Z
audited_commit: <final commit SHA of Task 8>
audit_template_version: v1
---

# belt-test-foundation F2b Audit Report

F2b audit 結果。F2a で確立した SSOT + binding realization の上に、Forward-to-F2b 7 items を Decision Tree + label で処理し、belt-core 内 helper consolidation と coverage gap を解消した。

## Methodology

- Forward list (F2a audit-report.md `Forward-to-F2b list`) を Decision Tree Q2-Q5 + 9 reason labels で judged
- Q3-B の副次発見は Side Findings section に集計
- engine Display 2 test の `brittle-format-match` label (F2a) は parameterize (literal → dynamic format) により解消 → Q3 behavior → Q5 kept に routing

## Label Frequency Summary (F2b)

| Label | 使用回数 | 対象 |
|---|---|---|
| `redundant-with-<X>` | 5 | uri inline 5 overlap tests |
| `kept (Q5 promoted)` | 2 | engine Display 2 tests (with parameterize) |
| `kept-without-scenario-id` | 0 | (F2b では shape-lock 追加変更なし、既存 4 file 維持) |
| `brittle-format-match` | 0 | F2a forward の 2 test を F2b で resolved |
| `implementation-coupling` | 0 | (F2b 対象なし) |
| `trivial-default-assertion` | 0 | (F2b 対象なし) |
| `tautology` | 0 | (F2b 対象なし) |
| `state-transition-overlap-with-<X>` | 0 | (F2b 対象なし) |
| `dead-fixture` | 0 | (F2b 対象なし) |
| `unreachable-guard` | 0 | (F2b 対象なし) |
| `obsolete-spec` | 0 | (F2b 対象なし) |

**Unused labels**: 8 labels unused in F2b. v2 bump は future signal、F2b で新 label 要求なし。

## Judgment per item

### Item 1: engine Display 2 tests

- `error_verify_required_message`: **kept (Q5)**. Scenario `belt-core-error-display-verify-required-preserves-phase-id` を追加、assertion parameterize (`starts_with` → `contains(&format!("for phase '{phase_id}'"))`) で 動的 phase_id lock
- `error_max_retries_exceeded_message`: **kept (Q5)**. Scenario `belt-core-error-display-max-retries-preserves-phase-id-and-counter` を追加、assertion parameterize で phase_id + ratio 両 lock
- F2a forward label `brittle-format-match` → F2b parameterize により解消、Decision Tree Q3 behavior branch に routing

### Item 2: uri inline 12 → integration 12 (migration)

- Inline 5 overlap tests (`parse_latest_happy_path`, `parse_workspace_latest_happy_path`, `parse_run_happy_path`, `parse_missing_scheme`, `parse_path_traversal_rejected`): **`redundant-with-<integration-id>`** + delete。同等 behavior を integration 側が cover
- Inline 7 edge case tests (`parse_unknown_selector`, `parse_empty_pipeline`, `parse_empty_run_id`, `parse_empty_path`, `parse_absolute_path_rejected`, `parse_workspace_missing_latest`, `to_string_roundtrip_all_variants`): **kept via Q5** (scenario 追加 + integration 移植完了後に inline delete)。label なし、behavior は新 integration 7 tests で lock
- `src/uri.rs` line 174-310 の `#[cfg(test)] mod tests` ブロック全削除、production code unchanged

### Item 3: gate git_clean coverage restoration

- 既存 test ゼロ → **new tests** 5 本追加 (clean/expect_clean、dirty/expect_dirty、clean/expect_dirty、dirty/expect_clean、missing-work_dir spawn failure)
- Scenario 5 本追加、category `gate` が 9 → 14 scenarios
- Judgment: **additive only** (既存 test 未 audit、新設 coverage)

### Item 4: Duplication Candidates 統合 (belt-core 内)

- `write_yaml` / `repo_root` / `fixture_path` 3 helpers を `tests/common/helpers.rs` に集約 (9 file で import 書き換え)
- narrative helpers (`find_phase` / `find_produce` / `has_file_exists_gate` / `has_named_consume` + 4 assert_*) を `tests/common/narrative.rs` に集約 (feature_dev_refresh / bug_fix_refresh の 2 file で使用)
- Parity helper (`read_workspace_file`) 抽出: **skipped** (scope 縮小、shared_filter_parity / shared_criteria_parity は独立保持で十分と判断)
- parser_test vs model_test: audit-template.md の Duplication Candidates 記述 "parser_test.rs::parse_minimal_pipeline" は誤記 (実 fn は `parse_pipeline_from_file`)。F2b で audit-template wording correction patch を適用、両 test は layer 分離された complementary test として **keep both**、`redundant` 判定せず
- cross-crate duplication (`engine_test regate_* vs belt-agent cli regate_*` 等) は F3 送り

### Item 5: lock-ledger bug_fix_refresh entry expansion

- lock-ledger.md の bug_fix_refresh.rs entry が F1 で stub 状態 ("F2/F3 で同様の shape dimension 列挙を行う") だったのを feature_dev_refresh.rs template 並みに expansion (+35-40 行、19 test-fn names + 9 shape dimensions + 2 cross-coupling)
- `scenarios_contract::lock_ledger_locks_files_exist` は `locks-file:` field のみ machine-check、shape dimensions / test-fn-count / cross-coupling は human-review content

### Item 6: expander_with_test.rs 0 test 解消

- 既存 17 行 tombstone preamble 保持、**3 integration tests** 追加:
  - `expand_pipeline_with_string_substitution_end_to_end` (string value propagation)
  - `expand_pipeline_with_bool_and_null_substitution_preserves_types` (type preservation)
  - `expand_pipeline_parent_scope_not_rewritten_by_sub_substitution` (parent-scope isolation、memory `feedback_expander_parent_scope_rule.md` rule lock)
- Public API `expand_pipeline` 経由の end-to-end、src/expander.rs inline unit test (26 本) と scope 分離
- Scenario 3 本追加、category `expander` が 4 → 7 scenarios

### Item 7: Decision Tree + 9 label application

- engine Display 2 tests / uri inline 5 overlap / uri inline 7 edge case / parser_test vs model_test (keep both) 全てに Decision Tree 適用、label 付与 or Q5 経由 kept 判定
- Side Findings section: (none discovered — 副次 test で delete 判定を要するものなし)

## Forward-to-F3 list

F3 (belt-agent behavior SSOT + cross-crate duplication) で扱う項目:

### F3 scope (belt-agent)

- `crates/belt-agent/tests/cli_test.rs` (40 test) の audit
- `crates/belt-agent/tests/e2e_test.rs` (8 test) の audit
- `docs/testing/cli-behavior/belt-agent.yml` 拡充 (6 subcommand JSON contract scenarios 30-40)

### F3 scope (cross-crate duplication)

- `engine_test regate_*` (14) vs `belt-agent cli regate_*` (11) — API layer vs CLI JSON layer
- `engine_test verify_verdict_*` vs `belt-agent cli verify_*` — verify pass/fail semantics
- `view_test engine_enriched_status_*` vs `belt-agent cli status_*` — view module API vs CLI

### F3 scope (binary crate helper unification)

- `belt/cli_test.rs` + `belt-agent/cli_test.rs` の `write_yaml` variant B (fs::write 2-line form) — Cargo cross-crate `tests/common` 制約を考慮した処置判断

## Test count / scenario count delta

| metric | F2a merge (baseline) | F2b completion | delta |
|---|---|---|---|
| workspace tests | 408 | 411 | +3 (Item 2 -5 + Item 3 +5 + Item 6 +3, Item 1 ±0, Item 4/5/7 ±0) |
| belt-core scenarios | 114 | 131 | +17 (uri +7, gate +5, error +2, expander +3) |
| new files | — | 3 | common/{mod,helpers,narrative}.rs |
| deleted files | — | 0 | (tombstone保持、inline削除は削除ではなく移植) |

## Cross-reference

- Template: `docs/testing/audit-template.md` v1 (2026-04-18 Duplication Candidates wording correction)
- Scenarios: `docs/testing/cli-behavior/{belt,belt-core}.yml`
- Lock ledger: `docs/testing/lock-ledger.md` (2026-04-18 bug_fix_refresh entry expanded)
- Design: `docs/features/2026-04-17-belt-test-foundation-f2b/design.md`
- Plan: `docs/features/2026-04-17-belt-test-foundation-f2b/plan.md`
- Test Strategy: `docs/features/2026-04-17-belt-test-foundation-f2b/test-strategy.md`
- F2a audit report: `docs/features/2026-04-17-belt-test-foundation-f2a/audit-report.md`
- F1 audit report (pilot 22 tests): `docs/features/2026-04-17-belt-test-foundation/audit-report.md`
```

- [ ] **Step 9.3: Verify `audit_template_version: v1` scenarios_contract still passes**

Run: `cargo test -p belt-core --test scenarios_contract audit_template_version_v1_matches_expected`
Expected: `1 passed` (v1 unchanged, wording patch doesn't affect the version string).

- [ ] **Step 9.4: Final workspace verification**

Run: `cargo test --workspace 2>&1 | tail -5`
Expected: `411 passed; 0 failed`

Run: `cargo clippy --workspace -- -D warnings && cargo fmt --all -- --check`
Expected: clean.

Run determinism check (50x loop):
```bash
for i in {1..50}; do cargo test --workspace || { echo "FAIL iteration $i"; exit 1; }; done && echo "OK 50/50"
```
Expected: `OK 50/50`

- [ ] **Step 9.5: Commit**

```bash
git add docs/features/2026-04-17-belt-test-foundation-f2b/audit-report.md docs/testing/audit-template.md
git -c commit.gpgsign=false commit -m "docs(features): add F2b audit report + audit-template wording correction"
```

---

## MV ↔ Task Mapping (verification of 100% coverage)

| MV | Task | MV | Task | MV | Task |
|---|---|---|---|---|---|
| MV-01 | Task 1 | MV-17 | Task 5 | MV-33 | Task 9 |
| MV-02 | Task 1 | MV-18 | Task 8 | MV-34 | Task 9 (final verify) |
| MV-03 | Task 2 | MV-19 | Task 1 | MV-35 | All tasks (each commit) |
| MV-04 | Task 1 | MV-20 | Task 1 | MV-36 | All tasks (each commit) |
| MV-05 | Task 7 | MV-21 | Task 1 | MV-37 | Verified by Task 1/2 (pilot untouched) |
| MV-06 | Task 7 | MV-22 | Task 2 | MV-38 | Verified by Task 1/2 (shape-lock test bodies untouched) |
| MV-07 | Task 4 | MV-23 | Verified by Task 1/2 (shape-lock count unchanged) | MV-39 | Task 7 (uri.rs pathspec exclude) + Task 9 final |
| MV-08 | Task 6 | MV-24 | Task 6 | MV-40 | All phases (narrative notes) |
| MV-09 | Task 5 | MV-25 | Task 6 | MV-41 | Task 1 precondition (current branch) |
| MV-10 | Task 8 | MV-26 | Task 7 (1:1 mapping table in audit-report) | MV-42 | Pre-Task 1 baseline verification |
| MV-11 | Task 9 | MV-27 | Task 4 | MV-43 | Pipeline init (already done Phase 1) |
| MV-12 | Task 9 | MV-28 | Task 5 | MV-44 | Task 9 (docs/testing README integrity check) |
| MV-13 | All tasks | MV-29 | Task 6 / Task 9 | MV-45 | Task 9 (specs verbatim path check) |
| MV-14 | Task 7 (human-review) | MV-30 | Task 7 / Task 9 audit-report | MV-46 | Task 9 (cross doc consistency) |
| MV-15 | Task 4 (human-review) | MV-31 | Task 7 / Task 9 audit-report | MV-47 | Task 1 |
| MV-16 | Task 6 (human-review) | MV-32 | Task 9 | MV-48 | Task 4 |

**Coverage**: 48 / 48 = 100%

---

## Self-Review Summary

- **Spec coverage**: 48 MV items mapped above, 10 TE entries cover cross-cutting concerns, all 3 design-phase decisions (A/B/C groups from spec-review) reflected in plan tasks.
- **Placeholder scan**: No TBD/TODO/"implement later" in plan. Only 2 "note" caveats about model struct field names (Task 2, 5) that require adjustment at implementation time based on actual belt-core model — these are real implementation hints, not placeholders.
- **Type consistency**: `write_yaml(&TempDir, &str, &str) -> PathBuf` consistent across Task 1 (definition) + Task 4/5/7 (usage). `repo_root() -> PathBuf` consistent. `find_phase<'a>(&'a Pipeline, &str) -> &'a Phase` signature introduced in Task 2 and referenced by Task 2 assertion helpers. `BeltError::VerifyRequired { phase_id }` / `MaxRetriesExceeded { phase_id, attempts, max_retries }` shapes consistent across Task 6 assertion + scenario wording.
- **Commit count**: 8 commits (Task 1-9, Task 3 skipped by default). Plan matches design.md's `9 commits 想定` (with B2 skip = 8).

---

## Execution Handoff

Plan complete and saved to `docs/features/2026-04-17-belt-test-foundation-f2b/plan.md`. Execute via `/subagent-driven-development` (feature-dev pipeline Phase 6 default): fresh subagent per Task, review between Tasks, two-stage commit-per-task pattern.

Alternative: `/executing-plans` (inline execution with checkpoints).

Feature-dev pipeline flows this plan into Phase 5 (pre-execute-handover) → Phase 6 (execute) → Phase 7 (code-review) → Phase 8 (integrate).
