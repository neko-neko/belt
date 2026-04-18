# belt://current URI Variant + SKILL Path Knowledge Elimination — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** SKILL.md / criteria/*.md / agents/*.md / pipeline.yml から `.belt/runs/{run_id}/...` リテラル + `{run_id}` template を完全削除し、`belt://current/<path>` URI を中心とした抽象化に置き換える。

**Architecture:** `BeltUri::Current { path }` variant を belt-core uri.rs に追加 → belt-agent resolver.rs に `resolve_current` を実装 → 新 CLI `belt-agent locate <uri>` で URI → 物理 path 解決を一元化 → engine から `{run_id}` template substitute を削除 → status JSON shape を path → uri + resolved_path に変更 → lint で raw `.belt/runs/` リテラル禁止 → plugins pipeline.yml / SKILL.md / criteria / agents を一括 URI 化 → agents は `output_path` runtime arg で path 受け取り。

**Tech Stack:** Rust 1.94.1, serde-saphyr 0.0.23, miette 7.6, glob 0.3, thiserror 2.0, clap 4.6 (既存 belt-core / belt-agent / belt 3 crate workspace)

**Spec:** `docs/specs/2026-04-18-belt-uri-current-and-skill-path-elimination-design.md`

---

## Implementation Order Rationale

1. **Additive first** (Phase A-B): URI variant / Resolver / Locate command を additive で追加。既存動作維持
2. **Plugin migration** (Phase C): pipeline.yml を URI 化。**この時点で lint rule はまだ無いので raw path も URI も両方 valid**
3. **Removal** (Phase D): engine から `expand_run_id` 削除、status JSON path field 削除。pipeline.yml が URI 化済みなので影響ゼロ
4. **Lint enforcement** (Phase E): `.belt/runs/` リテラル禁止 lint rule 追加。既存 plugin が URI 化済みなので新 lint pass
5. **Test scenarios + new fn** (Phase F): scenarios.yml 更新 + 新 test fn 追加 (TDD per fn)
6. **Plugin SKILL/criteria/agents** (Phase G): orchestrator が status から path 取得するよう書き換え
7. **Shape lock test** (Phase H): plugin 変更を反映、URI dimensions 追加
8. **Documentation** (Phase I): lock-ledger.md 更新
9. **CI verification** (Phase J)

---

## File Structure

### 新規作成
- `crates/belt-core/tests/uri_test.rs` — `BeltUri::Current` の parse / Display / glob / validation 5 fn
- `crates/belt-core/tests/gate_test.rs` — gate::execute_gates の URI 解決 / raw path passthrough 2 fn (or inline 内で増設)

### 主要修正
- `crates/belt-core/src/uri.rs` — `BeltUri::Current` variant + parser + Display
- `crates/belt-core/src/gate.rs` — `UriResolver` trait + `execute_gates` シグネチャ拡張
- `crates/belt-core/src/engine.rs` — `expand_run_id` / `expand_gate_run_id` 削除、関連呼び出し削除
- `crates/belt-core/src/view.rs` — `ResolvedArtifact` shape 変更 (`path` → `uri` + `path` mutually exclusive)
- `crates/belt-core/src/lint.rs` — `check_belt_runs_literal` rule 追加
- `crates/belt-agent/src/resolver.rs` — `resolve_current` + `current_run_id` field + `UriResolver` impl
- `crates/belt-agent/src/main.rs` — `Locate` subcommand + Resolver の cmd_verify/cmd_regate plumbing
- `plugins/belt/skills/{feature-dev,bug-fix,handover,resume,code-review,spec-review}/SKILL.md`
- `plugins/belt/skills/{feature-dev,bug-fix}/pipeline.yml`
- `plugins/belt/skills/handover/checkpoint.yml`
- `plugins/belt/skills/{feature-dev,bug-fix}/criteria/*.md` (6 file each)
- `plugins/belt/agents/*.md` (7 file)
- `plugins/belt-agent/skills/protocol/SKILL.md`
- `plugins/belt-agent/references/narrative-convention.md`
- `plugins/belt/skills/feature-dev/references/path-convention.md`
- `plugins/belt-agent/skills/protocol/references/resume-mode.md`
- `crates/belt-core/tests/{feature_dev_refresh,bug_fix_refresh,review_skills_refresh,lint_test,engine_test}.rs`
- `crates/belt-agent/tests/{cli_test,e2e_test}.rs`
- `docs/testing/cli-behavior/{belt-core,belt-agent}.yml`
- `docs/testing/lock-ledger.md`

---

# Phase A — belt-core URI infrastructure (additive)

## Task 1: Add `BeltUri::Current` variant + parser + Display

**Files:**
- Modify: `crates/belt-core/src/uri.rs:8-21` (enum), `:43-114` (parse), `:141-159` (Display)

- [ ] **Step 1: Add the `Current` variant to `BeltUri` enum**

Locate the `BeltUri` enum definition (currently lines 7-21) and add the `Current` variant after `Run`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeltUri {
    /// `belt://latest/{pipeline}/<path>` — COMPLETED latest run of `pipeline`
    /// on the *current* branch. `path` is relative to the resolved run dir.
    Latest { pipeline: String, path: String },
    /// `belt://workspace/{branch}/latest/{pipeline}/<path>` — COMPLETED latest
    /// of `pipeline` on *explicit* `branch`.
    WorkspaceLatest {
        branch: String,
        pipeline: String,
        path: String,
    },
    /// `belt://run/{run_id}/<path>` — explicit `run_id` (branch-independent).
    Run { run_id: String, path: String },
    /// `belt://current/<path>` — runtime invocation context の current run
    /// (`--run` 指定、未指定なら latest run) の `<run_dir>/<path>` に解決される。
    /// pipeline.yml の `produces[].path` / `gate.file_exists` で書き込み先 +
    /// 読み取り先の宣言に使用。
    Current { path: String },
}
```

- [ ] **Step 2: Add the `Current` branch in `BeltUri::parse`**

Locate `impl BeltUri { pub fn parse(s: &str) ... }` around line 43. Add the `current/` selector branch BEFORE the final `Err(UriParseError::UnknownSelector { ... })` line:

```rust
        if let Some(r) = rest.strip_prefix("current/") {
            // <path...> — no pipeline / branch / run_id segment
            let path = r;
            if path.is_empty() {
                return Err(UriParseError::EmptyPath { uri: s.to_string() });
            }
            validate_path(path, s)?;
            return Ok(BeltUri::Current {
                path: path.to_string(),
            });
        }
```

- [ ] **Step 3: Add the `Current` arm in `Display`**

Locate `impl std::fmt::Display for BeltUri` around line 141. Add the `Current` arm in the `match self`:

```rust
            BeltUri::Current { path } => {
                write!(f, "belt://current/{path}")
            }
```

- [ ] **Step 4: Run existing belt-core tests to confirm no regression**

Run: `cargo test -p belt-core --lib uri::`
Expected: all existing tests PASS, new behavior not yet exercised.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/src/uri.rs
git commit -m "feat(belt-core): add BeltUri::Current variant for runtime-bound URI resolution"
```

---

## Task 2: Add `BeltUri::Current` parse / glob / validation tests

**Files:**
- Create: `crates/belt-core/tests/uri_test.rs`
- Reference: `docs/testing/cli-behavior/belt-core.yml` (5 scenario IDs to bind in Step 4)

- [ ] **Step 1: Create the test file with 5 fn**

Create `crates/belt-core/tests/uri_test.rs`:

```rust
//! Integration tests for `BeltUri::Current` variant.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use belt_core::uri::{BeltUri, UriParseError};

/// scenario: belt-core-uri-parses-current-variant
#[test]
fn parses_current_variant() {
    let uri = BeltUri::parse("belt://current/notes/phase-design.md").unwrap();
    assert_eq!(
        uri,
        BeltUri::Current {
            path: "notes/phase-design.md".to_string(),
        }
    );
    assert_eq!(uri.to_string(), "belt://current/notes/phase-design.md");
}

/// scenario: belt-core-uri-current-rejects-empty-path
#[test]
fn current_rejects_empty_path() {
    // `belt://current/` (no path segment after the slash)
    let result = BeltUri::parse("belt://current/");
    assert!(matches!(result, Err(UriParseError::EmptyPath { .. })));
}

/// scenario: belt-core-uri-current-rejects-traversal
#[test]
fn current_rejects_traversal() {
    let result = BeltUri::parse("belt://current/../foo");
    assert!(matches!(result, Err(UriParseError::PathTraversal { .. })));
}

/// scenario: belt-core-uri-current-rejects-leading-slash
#[test]
fn current_rejects_leading_slash() {
    // After `current/`, an empty first segment indicates an absolute path:
    // `belt://current//foo` -> path starts with `/` after split (path = "/foo")
    let result = BeltUri::parse("belt://current//foo");
    assert!(matches!(result, Err(UriParseError::PathTraversal { .. })));
}

/// scenario: belt-core-uri-current-allows-glob-syntax
#[test]
fn current_allows_glob_syntax() {
    let uri = BeltUri::parse("belt://current/notes/phase-*.md").unwrap();
    assert!(matches!(uri, BeltUri::Current { .. }));
    assert_eq!(uri.to_string(), "belt://current/notes/phase-*.md");
}
```

- [ ] **Step 2: Run the new tests to verify they pass (Task 1 already implemented the variant)**

Run: `cargo test -p belt-core --test uri_test`
Expected: 5 tests PASS

- [ ] **Step 3: Add the 5 scenarios to `docs/testing/cli-behavior/belt-core.yml`**

Locate the file `docs/testing/cli-behavior/belt-core.yml`. Append the following 5 scenarios at the end of the `scenarios:` list (after the last existing entry, preserving 2-space indentation):

```yaml
  # --- uri::Current variant (2026-04-18 spec) ---
  - id: belt-core-uri-parses-current-variant
    category: uri
    severity: high
    technique: equivalence-partition
    given: "a belt://current/<path> URI string"
    when: "BeltUri::parse is called"
    then: "returns BeltUri::Current with the path field populated, Display round-trips"
  - id: belt-core-uri-current-rejects-empty-path
    category: uri
    severity: high
    technique: boundary-value
    given: "the literal 'belt://current/' (no path segment)"
    when: "BeltUri::parse is called"
    then: "returns UriParseError::EmptyPath"
  - id: belt-core-uri-current-rejects-traversal
    category: uri
    severity: high
    technique: boundary-value
    given: "a belt://current/../foo URI containing a parent-dir segment"
    when: "BeltUri::parse is called"
    then: "returns UriParseError::PathTraversal"
  - id: belt-core-uri-current-rejects-leading-slash
    category: uri
    severity: high
    technique: boundary-value
    given: "a belt://current//foo URI with an empty first path segment"
    when: "BeltUri::parse is called"
    then: "returns UriParseError::PathTraversal"
  - id: belt-core-uri-current-allows-glob-syntax
    category: uri
    severity: medium
    technique: equivalence-partition
    given: "a belt://current/notes/phase-*.md URI containing a glob"
    when: "BeltUri::parse is called"
    then: "parse succeeds; resolver-side glob expansion is responsible for matching"
```

Also update the `scope:` line at top to reflect URI Current additions:

Locate the existing scope line (line 1) and append `+ uri::Current 5` at the end of the cumulative count.

- [ ] **Step 4: Run scenarios_contract.rs to verify yml ↔ doc-comment binding**

Run: `cargo test -p belt-core --test scenarios_contract`
Expected: PASS (no symmetric diff)

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/tests/uri_test.rs docs/testing/cli-behavior/belt-core.yml
git commit -m "test(belt-core): add 5 uri::Current scenarios + integration tests"
```

---

## Task 3: Add `belt-core::gate::UriResolver` trait

**Files:**
- Modify: `crates/belt-core/src/gate.rs:1-12` (imports + struct), `:51-56` (execute_gates)

- [ ] **Step 1: Add the `UriResolver` trait at the top of gate.rs**

Locate the imports area at the top of `crates/belt-core/src/gate.rs`. The existing first line is `use std::path::Path;`. Modify to import `PathBuf` as well, and insert the `UriResolver` trait + no-op default impl AFTER `use crate::model::GateCheck;`:

```rust
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::model::GateCheck;

/// Resolves a `belt://` URI string to its absolute filesystem path.
///
/// Implemented by `belt_agent::resolver::Resolver` to enable gate-time URI
/// resolution without introducing a `belt-agent` -> `belt-core` cycle.
/// `belt-core` defines the trait; the binary crate implements it. Callers
/// that have no URI semantics (lint, tests against raw fixtures) can pass
/// `&NoopUriResolver` to keep the gate executor a no-op for URI strings.
pub trait UriResolver {
    /// Parse `uri` (a `belt://` URI string) and return its resolved
    /// filesystem path. Implementations MUST NOT assert file existence —
    /// callers handle existence as a separate concern (write targets vs
    /// read targets).
    ///
    /// # Errors
    ///
    /// Returns a string-form error when the URI is malformed or cannot be
    /// resolved (no current run, no completed run for selector, etc.).
    fn resolve(&self, uri: &str) -> Result<PathBuf, String>;
}

/// No-op resolver used by callers without URI semantics (e.g. lint, raw
/// fixture tests). Calling `.resolve()` with a `belt://` URI returns an
/// error; non-URI strings should never reach this resolver in production.
#[derive(Debug, Default)]
pub struct NoopUriResolver;

impl UriResolver for NoopUriResolver {
    fn resolve(&self, uri: &str) -> Result<PathBuf, String> {
        Err(format!(
            "NoopUriResolver cannot resolve '{uri}'; pass a real Resolver"
        ))
    }
}
```

- [ ] **Step 2: Run belt-core tests to confirm no regression**

Run: `cargo test -p belt-core --lib`
Expected: all PASS (additive change)

- [ ] **Step 3: Commit**

```bash
git add crates/belt-core/src/gate.rs
git commit -m "feat(belt-core): add UriResolver trait + NoopUriResolver default"
```

---

## Task 4: Extend `gate::execute_gates` to resolve `belt://` URIs in `file_exists`

**Files:**
- Modify: `crates/belt-core/src/gate.rs:32-56` (execute_gate, execute_gates), `:225-252` (execute_file_exists)
- Test: `crates/belt-core/tests/gate_test.rs` (new)

- [ ] **Step 1: Write failing tests in new `crates/belt-core/tests/gate_test.rs`**

Create `crates/belt-core/tests/gate_test.rs`:

```rust
//! Integration tests for gate::execute_gates with UriResolver.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use belt_core::gate::{execute_gates, NoopUriResolver, UriResolver};
use belt_core::model::GateCheck;
use std::path::{Path, PathBuf};

/// Test resolver that maps any `belt://current/<path>` to `<base>/<path>`.
struct TestCurrentResolver {
    base: PathBuf,
}

impl UriResolver for TestCurrentResolver {
    fn resolve(&self, uri: &str) -> Result<PathBuf, String> {
        let path = uri
            .strip_prefix("belt://current/")
            .ok_or_else(|| format!("not a belt://current/ uri: {uri}"))?;
        Ok(self.base.join(path))
    }
}

/// scenario: belt-core-gate-resolves-belt-current-via-uri-resolver
#[test]
fn gate_resolves_belt_current_via_uri_resolver() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("notes").join("phase-design.md");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, "x").unwrap();

    let resolver = TestCurrentResolver {
        base: tmp.path().to_path_buf(),
    };
    let gates = vec![GateCheck::FileExists {
        file_exists: "belt://current/notes/phase-design.md".to_string(),
    }];
    let results = execute_gates(&gates, Path::new("/"), Path::new("/"), &resolver);
    assert_eq!(results.len(), 1);
    assert!(results[0].passed, "URI-resolved file must exist");
}

/// scenario: belt-core-gate-passes-raw-domain-path-untouched
#[test]
fn gate_passes_raw_domain_path_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("docs").join("design.md");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, "x").unwrap();

    let resolver = NoopUriResolver;
    let gates = vec![GateCheck::FileExists {
        file_exists: "docs/design.md".to_string(),
    }];
    // Raw path: no belt:// prefix, resolver MUST NOT be invoked.
    let results = execute_gates(&gates, tmp.path(), Path::new("/"), &resolver);
    assert_eq!(results.len(), 1);
    assert!(results[0].passed, "raw glob must match against work_dir");
}
```

Add `tempfile` to dev-dependencies if not already there. Run `cargo tree -p belt-core --depth 1` to check; if absent, add to `crates/belt-core/Cargo.toml` `[dev-dependencies]`:

```toml
tempfile = "3"
```

- [ ] **Step 2: Run tests to verify they fail (signature mismatch)**

Run: `cargo test -p belt-core --test gate_test`
Expected: FAIL with E0061 (function takes 3 arguments but 4 were supplied) or similar.

- [ ] **Step 3: Update `execute_gate` and `execute_gates` signatures**

Locate `crates/belt-core/src/gate.rs:32-56`. Replace the existing `execute_gate` and `execute_gates` functions with:

```rust
/// Execute a single gate check.
///
/// # Arguments
/// * `check`      - The gate check variant to evaluate.
/// * `work_dir`   - Working directory for command execution and file lookups.
/// * `output_dir` - Directory where phase outputs are written (used by `has_output`).
/// * `resolver`   - URI resolver used to translate `belt://` URIs in
///                  `file_exists` patterns to filesystem paths. Pass
///                  `&NoopUriResolver` when no URI semantics apply.
#[must_use]
pub fn execute_gate(
    check: &GateCheck,
    work_dir: &Path,
    output_dir: &Path,
    resolver: &dyn UriResolver,
) -> GateResult {
    match check {
        GateCheck::Cmd { cmd, timeout } => execute_cmd(cmd, work_dir, *timeout),
        GateCheck::FileExists { file_exists } => {
            execute_file_exists(file_exists, work_dir, resolver)
        }
        GateCheck::GitClean { git_clean } => execute_git_clean(*git_clean, work_dir),
        GateCheck::HasOutput { has_output } => execute_has_output(*has_output, output_dir),
        GateCheck::Uses { uses, .. } => GateResult {
            check_type: "uses".to_owned(),
            passed: true,
            detail: Some(format!("uses: {uses} not yet resolved")),
            duration_ms: None,
            timed_out: false,
        },
    }
}

/// Execute all gate checks sequentially and return results.
#[must_use]
pub fn execute_gates(
    checks: &[GateCheck],
    work_dir: &Path,
    output_dir: &Path,
    resolver: &dyn UriResolver,
) -> Vec<GateResult> {
    checks
        .iter()
        .map(|c| execute_gate(c, work_dir, output_dir, resolver))
        .collect()
}
```

- [ ] **Step 4: Update `execute_file_exists` to honor URI prefix**

Locate `execute_file_exists` (currently around line 225). Replace its body with:

```rust
/// Match `pattern` (glob) relative to `work_dir`. Passes if at least one
/// file matches. When `pattern` starts with `belt://`, the resolver is used
/// to translate it to an absolute filesystem path BEFORE glob expansion;
/// otherwise the pattern is joined with `work_dir` (raw-path behavior).
fn execute_file_exists(
    pattern: &str,
    work_dir: &Path,
    resolver: &dyn UriResolver,
) -> GateResult {
    let resolved_pattern = if pattern.starts_with("belt://") {
        match resolver.resolve(pattern) {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(e) => {
                return GateResult {
                    check_type: "file_exists".to_owned(),
                    passed: false,
                    detail: Some(format!("URI resolution failed: {e}")),
                    duration_ms: None,
                    timed_out: false,
                };
            }
        }
    } else {
        work_dir.join(pattern).to_string_lossy().to_string()
    };

    match glob::glob(&resolved_pattern) {
        Ok(paths) => {
            let matches: Vec<_> = paths.filter_map(Result::ok).collect();
            let passed = !matches.is_empty();
            let detail = if passed {
                Some(format!("matched {} file(s)", matches.len()))
            } else {
                Some(format!("no files matched pattern: {pattern}"))
            };
            GateResult {
                check_type: "file_exists".to_owned(),
                passed,
                detail,
                duration_ms: None,
                timed_out: false,
            }
        }
        Err(e) => GateResult {
            check_type: "file_exists".to_owned(),
            passed: false,
            detail: Some(format!("invalid glob pattern: {e}")),
            duration_ms: None,
            timed_out: false,
        },
    }
}
```

- [ ] **Step 5: Update existing callers of `execute_gates` / `execute_gate`**

Locate all call sites. Run: `cargo build -p belt-core 2>&1 | grep -E "execute_gates?\b" | head`

Update each caller (likely in `engine.rs` and inline tests inside `gate.rs`) to pass `&NoopUriResolver` when there is no URI semantics:

In `crates/belt-core/src/gate.rs:323-348` (`#[cfg(test)] mod tests`), there are no `execute_gates` calls — they only call `all_passed`. No change needed there.

Search for `execute_gates(` and `execute_gate(` in `crates/belt-core/src/`:

Run: `rg "execute_gate" crates/belt-core/src/` to find sites. The expected sites are inside `gate.rs` itself; engine does not call these directly. If `engine.rs` does invoke gate code, update accordingly.

- [ ] **Step 6: Run tests**

Run: `cargo test -p belt-core --test gate_test`
Expected: 2 tests PASS

Run: `cargo test -p belt-core`
Expected: all PASS

- [ ] **Step 7: Add the 2 scenarios to `docs/testing/cli-behavior/belt-core.yml`**

Append after the URI Current scenarios:

```yaml
  # --- gate::execute_file_exists URI resolution (2026-04-18 spec) ---
  - id: belt-core-gate-resolves-belt-current-via-uri-resolver
    category: gate
    severity: high
    technique: equivalence-partition
    given: "a gate file_exists pattern starting with belt://current/ and a UriResolver impl that maps it to an existing file"
    when: "execute_gates is called with the resolver"
    then: "the gate passes (resolver-translated path exists)"
  - id: belt-core-gate-passes-raw-domain-path-untouched
    category: gate
    severity: high
    technique: equivalence-partition
    given: "a gate file_exists pattern that does NOT start with belt:// (raw path / glob)"
    when: "execute_gates is called with NoopUriResolver"
    then: "the resolver is bypassed and the pattern is glob-matched against work_dir"
```

- [ ] **Step 8: Commit**

```bash
git add crates/belt-core/src/gate.rs crates/belt-core/tests/gate_test.rs crates/belt-core/Cargo.toml docs/testing/cli-behavior/belt-core.yml
git commit -m "feat(belt-core): gate executor resolves belt:// URIs via UriResolver trait"
```

---

# Phase B — belt-agent extension (additive)

## Task 5: Add `ResolveError::NoCurrentRun` + `Resolver::current_run_id` field

**Files:**
- Modify: `crates/belt-agent/src/resolver.rs:7-32` (struct + error)

- [ ] **Step 1: Add the new error variant**

Locate `crates/belt-agent/src/resolver.rs:7-26`. Replace the `ResolveError` enum with:

```rust
/// Resolution errors encountered by belt-agent when mapping a `BeltUri`
/// to an absolute filesystem path.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ResolveError {
    #[error("run not found: {run_id}")]
    RunNotFound { run_id: String },
    #[error(
        "no COMPLETED run of pipeline '{pipeline}' on branch '{}'",
        branch.as_deref().unwrap_or("(none)")
    )]
    NoCompletedRun {
        pipeline: String,
        branch: Option<String>,
    },
    #[error("branch-aware URI requires git directory")]
    BranchAwareRequiresGit,
    #[error("resolved artifact missing: {path}")]
    ArtifactMissing { path: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("state.json parse error: {0}")]
    StateParse(#[from] serde_json::Error),
    /// `belt://current/...` URI を解決しようとしたが invocation context に
    /// run_id が無い (`--run` 未指定 + latest run 不在)。
    #[error("belt://current/ requires a current run (none found, pass --run <id>)")]
    NoCurrentRun,
}
```

- [ ] **Step 2: Add `current_run_id` field to `Resolver` struct**

Locate the `Resolver` struct (lines 28-32). Replace with:

```rust
#[derive(Debug)]
pub(crate) struct Resolver<'a> {
    pub belt_dir: &'a Path,
    pub current_branch: Option<String>,
    /// Resolved `--run` arg (or latest run id) used to bind `belt://current/`
    /// URIs to a concrete run directory. `None` when no run is in scope
    /// (e.g. before any `init`).
    pub current_run_id: Option<String>,
}
```

- [ ] **Step 3: Run tests to confirm no regression (additive change)**

Run: `cargo test -p belt-agent --test cli_test`
Expected: PASS (existing tests construct Resolver via `..Default` patterns or struct literals; if there are construction sites in `main.rs` they will fail to compile — proceed to fix in Task 6).

If compile fails in main.rs, defer the fix to Task 6 (Step 5) where we add construction at all callsites.

- [ ] **Step 4: Commit**

```bash
git add crates/belt-agent/src/resolver.rs
git commit -m "feat(belt-agent): add NoCurrentRun error + current_run_id field on Resolver"
```

---

## Task 6: Implement `Resolver::resolve_current` + dispatch in `Resolver::resolve`

**Files:**
- Modify: `crates/belt-agent/src/resolver.rs:34-50` (resolve dispatch), append `resolve_current` method

- [ ] **Step 1: Update `Resolver::resolve` to dispatch `Current` variant**

Locate the `impl Resolver<'_>` block at line 34. Replace the `resolve` method with:

```rust
impl Resolver<'_> {
    pub(crate) fn resolve(&self, uri: &BeltUri) -> Result<PathBuf, ResolveError> {
        match uri {
            BeltUri::Run { run_id, path } => self.resolve_run(run_id, path),
            BeltUri::Latest { pipeline, path } => self.resolve_latest(pipeline, path, None),
            BeltUri::WorkspaceLatest {
                branch,
                pipeline,
                path,
            } => {
                if self.current_branch.is_none() {
                    return Err(ResolveError::BranchAwareRequiresGit);
                }
                self.resolve_latest(pipeline, path, Some(branch))
            }
            BeltUri::Current { path } => self.resolve_current(path),
        }
    }
```

- [ ] **Step 2: Add `resolve_current` method**

Inside the same `impl Resolver<'_>` block, AFTER `resolve_run` and BEFORE `resolve_latest`, add:

```rust
    fn resolve_current(&self, path: &str) -> Result<PathBuf, ResolveError> {
        let run_id = self
            .current_run_id
            .as_ref()
            .ok_or(ResolveError::NoCurrentRun)?;
        let run_dir = self.belt_dir.join("runs").join(run_id);
        if !run_dir.is_dir() {
            return Err(ResolveError::RunNotFound {
                run_id: run_id.clone(),
            });
        }
        // existence assertion is the caller's concern (write targets vs
        // read targets); resolver only computes the path.
        Ok(run_dir.join(path))
    }
```

- [ ] **Step 3: Add unit tests for `resolve_current`**

Locate the existing `#[cfg(test)] mod tests` block in `resolver.rs` (line 143). Append the following tests AFTER `resolve_workspace_latest_errors_on_non_git`:

```rust
    #[test]
    fn resolve_current_returns_run_dir_path() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let run_dir = belt_dir.join("runs").join("01947abc");
        fs::create_dir_all(run_dir.join("notes")).unwrap();

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
            current_run_id: Some("01947abc".to_string()),
        };
        let uri = BeltUri::Current {
            path: "notes/phase-design.md".into(),
        };
        let resolved = r.resolve(&uri).unwrap();
        // existence is NOT asserted by resolve_current (write target case)
        assert_eq!(resolved, run_dir.join("notes").join("phase-design.md"));
    }

    #[test]
    fn resolve_current_errors_when_no_current_run_id() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
            current_run_id: None,
        };
        let uri = BeltUri::Current {
            path: "notes/x.md".into(),
        };
        assert!(matches!(r.resolve(&uri), Err(ResolveError::NoCurrentRun)));
    }

    #[test]
    fn resolve_current_errors_when_run_dir_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
            current_run_id: Some("missing".to_string()),
        };
        let uri = BeltUri::Current {
            path: "notes/x.md".into(),
        };
        assert!(matches!(
            r.resolve(&uri),
            Err(ResolveError::RunNotFound { .. })
        ));
    }
```

- [ ] **Step 4: Update existing test struct literals to include `current_run_id: None`**

Run: `rg "Resolver \{" crates/belt-agent/src/resolver.rs` to find all struct literals.

Add `current_run_id: None,` to each existing `Resolver { ... }` literal in the test block (currently `resolve_run_happy_path`, `resolve_run_missing_run_dir`, `resolve_run_missing_artifact`, `resolve_latest_picks_completed_on_current_branch`, `resolve_latest_prefers_newer_uuidv7`, `resolve_latest_errors_when_no_completed`, `resolve_latest_falls_back_when_branch_none`, `resolve_workspace_latest_uses_explicit_branch`, `resolve_workspace_latest_errors_on_non_git`, `resolve_latest_errors_on_corrupt_state_json`, `resolve_latest_skips_state_json_without_pipeline_field`, `resolve_latest_skips_state_json_that_is_a_directory`).

- [ ] **Step 5: Update `Resolver` construction sites in `crates/belt-agent/src/main.rs`**

Run: `rg "Resolver \{" crates/belt-agent/src/main.rs`

Locate the existing construction (currently line 201-204 in `cmd_init`):

```rust
    let resolver = crate::resolver::Resolver {
        belt_dir: &belt,
        current_branch: branch.clone(),
    };
```

Replace with:

```rust
    let resolver = crate::resolver::Resolver {
        belt_dir: &belt,
        current_branch: branch.clone(),
        current_run_id: None, // init has no current run yet
    };
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p belt-agent --lib`
Expected: 3 new resolver tests PASS, no regression.

Run: `cargo test -p belt-agent`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add crates/belt-agent/src/resolver.rs crates/belt-agent/src/main.rs
git commit -m "feat(belt-agent): implement Resolver::resolve_current with NoCurrentRun guard"
```

---

## Task 7: Implement `belt_core::gate::UriResolver` for `Resolver`

**Files:**
- Modify: `crates/belt-agent/src/resolver.rs` (append impl)

- [ ] **Step 1: Add the trait impl after the `impl Resolver<'_>` block**

Locate the end of `impl Resolver<'_> { ... }` block. After it (and BEFORE `#[cfg(test)] mod tests`), add:

```rust
impl belt_core::gate::UriResolver for Resolver<'_> {
    fn resolve(&self, uri: &str) -> Result<std::path::PathBuf, String> {
        let parsed = BeltUri::parse(uri).map_err(|e| e.to_string())?;
        Resolver::resolve(self, &parsed).map_err(|e| e.to_string())
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p belt-agent`
Expected: success.

- [ ] **Step 3: Add an integration test**

Locate `crates/belt-agent/src/resolver.rs` test block. Append:

```rust
    #[test]
    fn impl_uri_resolver_trait_for_current_uri() {
        use belt_core::gate::UriResolver as _;
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let run_dir = belt_dir.join("runs").join("01947zzz");
        fs::create_dir_all(run_dir.join("notes")).unwrap();

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
            current_run_id: Some("01947zzz".to_string()),
        };
        let resolved = <Resolver<'_> as belt_core::gate::UriResolver>::resolve(
            &r,
            "belt://current/notes/phase-design.md",
        )
        .unwrap();
        assert_eq!(resolved, run_dir.join("notes").join("phase-design.md"));
    }
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p belt-agent --lib`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/belt-agent/src/resolver.rs
git commit -m "feat(belt-agent): impl belt_core::gate::UriResolver for Resolver"
```

---

## Task 8: Add `belt-agent locate <uri>` subcommand

**Files:**
- Modify: `crates/belt-agent/src/main.rs:25-69` (Command enum), append `cmd_locate`

- [ ] **Step 1: Add `Locate` variant to the `Command` enum**

Locate `crates/belt-agent/src/main.rs:25-69` (`enum Command`). Insert AFTER the `Status` variant:

```rust
    /// Resolve a `belt://` URI to its filesystem path
    Locate {
        /// belt:// URI to resolve
        uri: String,
        /// Run ID (default: latest)
        #[arg(long)]
        run: Option<String>,
    },
```

- [ ] **Step 2: Add the dispatch arm in `main()`**

Locate the `match cli.command` block (around line 149-163). Add the arm after `Status`:

```rust
        Command::Locate { uri, run } => cmd_locate(&engine, &uri, run.as_ref())?,
```

- [ ] **Step 3: Implement `cmd_locate`**

Append the following function AFTER the existing `cmd_status` (around line 697 end):

```rust
fn cmd_locate(engine: &Engine, uri_str: &str, run: Option<&String>) -> miette::Result<()> {
    use belt_core::uri::BeltUri;
    let uri = BeltUri::parse(uri_str).map_err(|e| miette::miette!("{e}"))?;

    // Determine the current run id: explicit --run wins, otherwise fall back
    // to engine.latest_run_id(). For non-Current variants the field is ignored
    // by the resolver, so failure to resolve a current run is not fatal here
    // unless the URI is BeltUri::Current.
    let current_run_id = match run {
        Some(id) => Some(id.clone()),
        None => engine.latest_run_id().ok(),
    };

    let branch = crate::git::current_branch(std::path::Path::new("."));
    let belt = belt_dir();
    let resolver = crate::resolver::Resolver {
        belt_dir: &belt,
        current_branch: branch,
        current_run_id,
    };
    let resolved = resolver
        .resolve(&uri)
        .map_err(|e| miette::miette!("{e}"))?;

    // existence is computed via fs metadata; for glob URIs this is `glob match >= 1`.
    // For simplicity here we only handle direct path existence; glob was
    // expanded by the resolver path semantics.
    let exists = std::fs::metadata(&resolved).is_ok();

    let out = json!({
        "uri": uri.to_string(),
        "path": resolved.display().to_string(),
        "exists": exists,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
    );
    Ok(())
}
```

- [ ] **Step 4: Build and run a smoke test**

Run: `cargo build -p belt-agent`
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-agent/src/main.rs
git commit -m "feat(belt-agent): add `belt-agent locate <uri>` subcommand for URI resolution"
```

---

## Task 9: Pass `Resolver` to `cmd_verify` / `cmd_regate` for gate URI resolution

**Files:**
- Modify: `crates/belt-agent/src/main.rs:379-428` (cmd_verify), `:589-685` (cmd_regate, execute_regate_targets)

- [ ] **Step 1: Update `cmd_verify` to construct + pass Resolver**

Locate `cmd_verify` around line 379. After the existing `let phase = engine.next_phase_info(...)` line, but BEFORE `let results = execute_gates(...)`, insert:

```rust
    let branch = crate::git::current_branch(std::path::Path::new("."));
    let resolver = crate::resolver::Resolver {
        belt_dir: &belt_dir(),
        current_branch: branch,
        current_run_id: Some(state.run_id.clone()),
    };
```

Then update the `execute_gates` call to pass the resolver:

```rust
    let results = execute_gates(&phase.gate, &work_dir, Path::new(output_dir), &resolver);
```

- [ ] **Step 2: Update `execute_regate_targets` signature + call site**

Locate `execute_regate_targets` around line 538. Add a `resolver` parameter:

```rust
fn execute_regate_targets(
    phase: &belt_core::model::ExpandedPhase,
    state: &belt_core::model::RunState,
    all_phases: &[belt_core::model::ExpandedPhase],
    belt: &Path,
    resolver: &dyn belt_core::gate::UriResolver,
) -> miette::Result<(serde_json::Map<String, serde_json::Value>, bool)> {
```

Inside the function, replace the existing `execute_gates(&target_gate, &work_dir, &output_dir)` call with:

```rust
        let results = execute_gates(&target_gate, &work_dir, &output_dir, resolver);
```

- [ ] **Step 3: Update `cmd_regate` to construct Resolver and pass to `execute_regate_targets`**

Locate `cmd_regate` around line 589. Before the `execute_regate_targets` call (around line 660), insert:

```rust
    let branch = crate::git::current_branch(std::path::Path::new("."));
    let resolver = crate::resolver::Resolver {
        belt_dir: &belt,
        current_branch: branch,
        current_run_id: Some(state.run_id.clone()),
    };
```

Update the call:

```rust
    let (targets, all_passed_flag) =
        execute_regate_targets(&phase, &state, &all_phases, &belt, &resolver)?;
```

- [ ] **Step 4: Build and run all tests**

Run: `cargo build -p belt-agent && cargo test -p belt-agent`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/belt-agent/src/main.rs
git commit -m "feat(belt-agent): plumb Resolver into verify + regate gate execution"
```

---

## Task 10: Add `uri` field to status / init / next produces JSON output (additive, path field retained)

**Files:**
- Modify: `crates/belt-core/src/view.rs:84-97` (ResolvedArtifact), `:160-232` (resolve_artifact)
- Modify: `crates/belt-agent/src/main.rs:270-288` (phase_json)

- [ ] **Step 1: Add `uri` field to `ResolvedArtifact`**

Locate `crates/belt-core/src/view.rs:88-97`. Replace `ResolvedArtifact` struct with:

```rust
/// An artifact produced by a phase, enriched with runtime-resolved
/// filesystem state. When the declared path is a `belt://` URI, `uri`
/// holds the URI and `path` is omitted; otherwise `path` holds the raw
/// declared path. `resolved_path` is the concrete filesystem path.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedArtifact {
    pub name: String,
    /// `belt://...` URI (when declared as URI). Mutually exclusive with `path`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// Raw declared path (when declared as raw path, e.g. domain artifacts
    /// like `docs/features/*/design.md`). Mutually exclusive with `uri`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_path: Option<String>,
}
```

- [ ] **Step 2: Update `resolve_artifact` to populate uri/path mutually exclusively**

Locate `resolve_artifact` (line 160-232). Replace its return ResolvedArtifact construction with:

```rust
    let is_uri = artifact.path.starts_with("belt://");
    ResolvedArtifact {
        name: artifact.name.clone(),
        uri: if is_uri {
            Some(artifact.path.clone())
        } else {
            None
        },
        path: if is_uri {
            None
        } else {
            Some(artifact.path.clone())
        },
        description: artifact.description.clone(),
        exists,
        resolved_path,
    }
```

(Replace BOTH the early-return constructions inside the function — the success branch and the early-error branch in the glob path — using the same pattern.)

- [ ] **Step 3: Update `phase_json` in `crates/belt-agent/src/main.rs:270-288`**

Locate `phase_json`. Replace with:

```rust
fn phase_json(phase: &belt_core::model::ExpandedPhase) -> serde_json::Value {
    let produces_json: Vec<serde_json::Value> = phase
        .produces
        .iter()
        .map(|a| {
            let is_uri = a.path.starts_with("belt://");
            if is_uri {
                json!({
                    "name": a.name,
                    "uri": a.path,
                    "description": a.description,
                })
            } else {
                json!({
                    "name": a.name,
                    "path": a.path,
                    "description": a.description,
                })
            }
        })
        .collect();
    let mut phase_obj = json!({
        "id": phase.id,
        "description": phase.description,
        "config": phase.config,
        "output_dir": phase.output_dir,
        "produces": produces_json,
        "consumes": phase.consumes,
    });
    if let Some(invoke) = &phase.invoke {
        if let serde_json::Value::Object(map) = &mut phase_obj {
            map.insert(
                "invoke".to_string(),
                serde_json::to_value(invoke).unwrap_or(serde_json::Value::Null),
            );
        }
    }
    phase_obj
}
```

- [ ] **Step 4: Build and test**

Run: `cargo build --workspace && cargo test --workspace`
Expected: PASS (some existing tests may need adjustments — the JSON shape change `path → uri | path` is backward-compatible-ish since `path` is now optional, but old tests asserting `produces[].path == "..."` for URI-declared artifacts will need updating. Update them in this commit.)

For each failing test that asserts `phases[].produces[N].path == "belt://..."`, change it to assert `phases[].produces[N].uri`.

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/src/view.rs crates/belt-agent/src/main.rs crates/belt-core/tests/ crates/belt-agent/tests/
git commit -m "feat(belt-agent): add uri field to status/init/next produces JSON (path remains for raw)"
```

---

# Phase C — Plugin pipeline.yml 一括 URI 化

## Task 11: Migrate `plugins/belt/skills/handover/checkpoint.yml`

**Files:**
- Modify: `plugins/belt/skills/handover/checkpoint.yml`

- [ ] **Step 1: Replace the gate path**

Locate the file. Replace the entire content with:

```yaml
name: pre-execute-handover-checkpoint
version: 1
description: "Context reset checkpoint shared by pipelines that require /belt:handover + /clear + /belt:resume before entering the execute phase"

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

- [ ] **Step 2: Lint the parent pipelines to confirm sub-pipeline still resolves**

Run: `cargo run -p belt -- lint plugins/belt/skills/feature-dev/pipeline.yml 2>&1 | head -20`
Run: `cargo run -p belt -- lint plugins/belt/skills/bug-fix/pipeline.yml 2>&1 | head -20`
Expected: lint passes (sub-pipeline existence + URI grammar OK).

- [ ] **Step 3: Commit**

```bash
git add plugins/belt/skills/handover/checkpoint.yml
git commit -m "refactor(plugins/handover): migrate checkpoint.yml to belt://current/ URI"
```

---

## Task 12: Migrate `plugins/belt/skills/feature-dev/pipeline.yml`

**Files:**
- Modify: `plugins/belt/skills/feature-dev/pipeline.yml`

- [ ] **Step 1: Update the design phase produces + gate**

Locate the design phase (lines 16-32). Replace the `produces:` list and `gate:` list with:

```yaml
    produces:
      - name: design_doc
        path: "docs/features/*/design.md"
        description: "Design document with explored context and test perspectives"
      - name: design_notes
        path: "belt://current/notes/phase-design.md"
        description: "Phase narrative: decisions, concerns, directives, observations"
    gate:
      - file_exists: "docs/features/*/design.md"
      - file_exists: "belt://current/notes/phase-design.md"
```

- [ ] **Step 2: Update the plan phase produces + gate**

Locate the plan phase (lines 69-89). Replace produces + gate with:

```yaml
    produces:
      - name: plan_doc
        path: "docs/features/*/plan.md"
        description: "Task-level implementation plan (TDD)"
      - name: plan_notes
        path: "belt://current/notes/phase-plan.md"
        description: "Phase narrative"
    gate:
      - file_exists: "docs/features/*/plan.md"
      - file_exists: "belt://current/notes/phase-plan.md"
```

- [ ] **Step 3: Update the execute phase produces + gate**

Locate the execute phase (lines 95-112). Replace produces + gate with:

```yaml
    produces:
      - name: execute_notes
        path: "belt://current/notes/phase-execute.md"
        description: "Phase narrative"
    gate:
      - file_exists: "belt://current/notes/phase-execute.md"
```

- [ ] **Step 4: Update the code-review phase — expand produces to 7 entries + URI gate**

Locate the code-review phase (lines 114-135). Replace the entire phase with:

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
        description: "Security observation findings"
      - name: findings-test
        path: "belt://current/review/findings-test.json"
        description: "Test observation findings"
      - name: findings-ai-antipattern
        path: "belt://current/review/findings-ai-antipattern.json"
        description: "AI antipattern observation findings"
      - name: findings-cross-cutting
        path: "belt://current/review/findings-cross-cutting.json"
        description: "Cross-cutting observation findings"
      - name: findings-codex
        path: "belt://current/review/findings-codex.json"
        description: "Codex adversarial review findings"
        when: "args.codex"
      - name: findings
        path: "belt://current/review/findings.json"
        description: "Merged findings (post-dedup)"
      - name: code_review_notes
        path: "belt://current/notes/phase-code-review.md"
        description: "Phase narrative"
    gate:
      - file_exists: "belt://current/notes/phase-code-review.md"
      - file_exists: "belt://current/review/findings.json"
    validate: ./criteria/code-review.md
    regate: [execute]
    confirm: true
    max_retries: 3
```

- [ ] **Step 5: Update the monkey-test phase produces + gate**

Locate the monkey-test phase (lines 137-164). Replace produces + gate with:

```yaml
    produces:
      - name: monkey_test_report
        path: "docs/features/*/monkey-test-report.md"
      - name: monkey_test_results
        path: "docs/features/*/monkey-test-results.json"
      - name: monkey_test_notes
        path: "belt://current/notes/phase-monkey-test.md"
        description: "Phase narrative"
    gate:
      - file_exists: "docs/features/*/monkey-test-report.md"
      - file_exists: "belt://current/notes/phase-monkey-test.md"
```

- [ ] **Step 6: Update the dogfood phase produces + gate**

Locate the dogfood phase (lines 166-194). Replace produces + gate with:

```yaml
    produces:
      - name: dogfood_report
        path: "docs/features/*/dogfood-report/report.md"
      - name: dogfood_notes
        path: "belt://current/notes/phase-dogfood.md"
        description: "Phase narrative"
    gate:
      - file_exists: "docs/features/*/dogfood-report/report.md"
      - file_exists: "belt://current/notes/phase-dogfood.md"
```

- [ ] **Step 7: Lint and grep verify**

Run: `cargo run -p belt -- lint plugins/belt/skills/feature-dev/pipeline.yml`
Expected: lint passes (warnings about produces protection are OK; no errors).

Run: `grep -n "\.belt/runs\|{run_id}" plugins/belt/skills/feature-dev/pipeline.yml`
Expected: no matches.

- [ ] **Step 8: Commit**

```bash
git add plugins/belt/skills/feature-dev/pipeline.yml
git commit -m "refactor(plugins/feature-dev): migrate pipeline.yml to belt://current/ URIs + expand code-review produces"
```

---

## Task 13: Migrate `plugins/belt/skills/bug-fix/pipeline.yml`

**Files:**
- Modify: `plugins/belt/skills/bug-fix/pipeline.yml`

- [ ] **Step 1: Read the file to map current phase ids**

Run: `cat plugins/belt/skills/bug-fix/pipeline.yml`

The pipeline has 9 phases per lock-ledger (`rca → fix-plan → fix-plan-review → pre-execute-handover → execute → code-review → monkey-test → dogfood → integrate`).

- [ ] **Step 2: Apply the same URI migration pattern as Task 12 to each narrative-producing phase**

For each of `rca`, `fix-plan`, `execute`, `code-review`, `monkey-test`, `dogfood`:

Replace produces narrative entries from `path: ".belt/runs/{run_id}/notes/phase-<id>.md"` to `path: "belt://current/notes/phase-<id>.md"`.

Replace gate `file_exists: ".belt/runs/{run_id}/notes/phase-<id>.md"` to `file_exists: "belt://current/notes/phase-<id>.md"`.

- [ ] **Step 3: For `code-review` and `fix-plan-review` phases, expand produces to include observation findings**

For `code-review`, mirror Task 12 Step 4 — 7 produces entries (findings-security, findings-test, findings-ai-antipattern, findings-cross-cutting, findings-codex with `when: args.codex`, findings (merged), code_review_notes), gate of 2 entries.

For `fix-plan-review`, expand to include spec-review observation findings (3 observation agents per `plugins/belt/agents/`):

```yaml
    produces:
      - name: findings-feasibility
        path: "belt://current/review/findings-feasibility.json"
        description: "Feasibility observation findings"
      - name: findings-cross-cutting-spec
        path: "belt://current/review/findings-cross-cutting-spec.json"
        description: "Cross-cutting spec observation findings"
      - name: findings-ui-design
        path: "belt://current/review/findings-ui-design.json"
        description: "UI design observation findings"
      - name: findings-codex
        path: "belt://current/review/findings-codex.json"
        description: "Codex adversarial spec review findings"
        when: "args.codex"
      - name: findings
        path: "belt://current/review/findings.json"
        description: "Merged spec-review findings"
```

(If fix-plan-review currently has narrative produces, also keep `fix_plan_review_notes` -> `belt://current/notes/phase-fix-plan-review.md`, with gate.)

- [ ] **Step 4: Lint + grep**

Run: `cargo run -p belt -- lint plugins/belt/skills/bug-fix/pipeline.yml`
Run: `grep -n "\.belt/runs\|{run_id}" plugins/belt/skills/bug-fix/pipeline.yml`
Expected: lint passes; grep returns nothing.

- [ ] **Step 5: Commit**

```bash
git add plugins/belt/skills/bug-fix/pipeline.yml
git commit -m "refactor(plugins/bug-fix): migrate pipeline.yml to belt://current/ URIs + expand review produces"
```

---

# Phase D — Removal (engine cleanup)

## Task 14: Remove `expand_run_id` / `expand_gate_run_id` + obsolete tests

**Files:**
- Modify: `crates/belt-core/src/engine.rs:167-173, 440-468`
- Modify: `crates/belt-core/tests/engine_test.rs` (delete 2 fn around lines 2213, 2267)

- [ ] **Step 1: Identify obsolete fn names**

Run: `grep -n "fn .*run_id" crates/belt-core/tests/engine_test.rs`
Note the 2 fn names that test `{run_id}` substitution. Likely candidates: `next_phase_info_substitutes_run_id_in_produces_path` and `next_phase_info_substitutes_run_id_in_file_exists_gate_path` (per memory; confirm by reading).

- [ ] **Step 2: Delete the obsolete tests**

Open `crates/belt-core/tests/engine_test.rs`. Delete the 2 test functions (entire `#[test] fn ... { ... }` blocks around lines 2213 and 2267).

- [ ] **Step 3: Remove substitute logic from `next_phase_info`**

Locate `crates/belt-core/src/engine.rs:167-174`. Delete the loop that mutates `phase.produces[].path` and the `expand_gate_run_id` call:

DELETE:
```rust
        // Expand `{run_id}` template in fields the LLM sees at step time.
        // Idempotent: a second call on already-expanded text is a no-op
        // because the token is gone after the first substitution.
        for artifact in &mut phase.produces {
            artifact.path = expand_run_id(&artifact.path, &state.run_id);
        }
        expand_gate_run_id(&mut phase.gate, &state.run_id);
```

- [ ] **Step 4: Delete the helper functions**

Locate `crates/belt-core/src/engine.rs:440-468`. Delete `fn expand_run_id` and `pub fn expand_gate_run_id` entirely.

- [ ] **Step 5: Build to verify no remaining callers**

Run: `cargo build --workspace 2>&1 | head -30`
If build fails on `expand_gate_run_id` callers (likely `crates/belt-agent/src/main.rs` cmd_regate), proceed to Task 15. Otherwise:

Run: `cargo test -p belt-core`
Expected: PASS

- [ ] **Step 6: Add a replacement scenario in belt-core.yml**

Append to `docs/testing/cli-behavior/belt-core.yml`:

```yaml
  - id: belt-core-engine-emits-declared-uri-in-next-phase-info
    category: engine
    severity: high
    technique: equivalence-partition
    given: "a pipeline phase whose produces entry declares a belt://current/ URI"
    when: "Engine::next_phase_info is called"
    then: "the returned ExpandedPhase.produces[].path is the declared URI string verbatim (no substitution)"
```

- [ ] **Step 7: Add a replacement test in `crates/belt-core/tests/engine_test.rs`**

Append:

```rust
/// scenario: belt-core-engine-emits-declared-uri-in-next-phase-info
#[test]
fn next_phase_info_emits_declared_uri_verbatim() {
    let tmp = tempfile::tempdir().unwrap();
    let pipeline_path = tmp.path().join("pipeline.yml");
    std::fs::write(
        &pipeline_path,
        r#"name: t
version: 1
phases:
  - id: p
    description: x
    produces:
      - name: notes
        path: "belt://current/notes/phase-p.md"
    gate:
      - file_exists: "belt://current/notes/phase-p.md"
"#,
    )
    .unwrap();
    let belt_dir = tmp.path().join(".belt");
    let engine = belt_core::engine::Engine::new(&belt_dir);
    let state = engine
        .init(&pipeline_path, &std::collections::HashMap::new())
        .unwrap();
    let phase = engine.next_phase_info(&state, &pipeline_path).unwrap();
    assert_eq!(phase.produces[0].path, "belt://current/notes/phase-p.md");
}
```

- [ ] **Step 8: Run all tests**

Run: `cargo test -p belt-core`
Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add crates/belt-core/src/engine.rs crates/belt-core/tests/engine_test.rs docs/testing/cli-behavior/belt-core.yml
git commit -m "refactor(belt-core): drop expand_run_id template substitution; URIs are declared verbatim"
```

---

## Task 15: Drop `expand_gate_run_id` callsite in `belt-agent` cmd_regate + replace with URI scenario

**Files:**
- Modify: `crates/belt-agent/src/main.rs:565-575` (cmd_regate)
- Modify: `crates/belt-agent/tests/cli_test.rs` (delete `regate_substitutes_run_id_in_target_gate`, add URI version)

- [ ] **Step 1: Remove the `expand_gate_run_id` call**

Locate `crates/belt-agent/src/main.rs:565-575` (inside `execute_regate_targets`). DELETE these lines:

```rust
        // The expander does not substitute `{run_id}` (it has no access to
        // runtime state). Apply the same gate-text expansion that
        // `next_phase_info` applies to the current phase, otherwise gates
        // like `file_exists: ".belt/runs/{run_id}/notes/phase-X.md"` would
        // be checked against the literal `{run_id}` directory.
        let mut target_gate = target_phase.gate.clone();
        belt_core::engine::expand_gate_run_id(&mut target_gate, &state.run_id);
```

Replace with:

```rust
        let target_gate = target_phase.gate.clone();
```

(URI resolution now happens inside `execute_gates` via the `Resolver`.)

- [ ] **Step 2: Delete the obsolete test**

Locate `crates/belt-agent/tests/cli_test.rs:558` `regate_substitutes_run_id_in_target_gate`. Delete the entire test fn.

- [ ] **Step 3: Add the replacement test using the existing regate harness**

Read existing regate tests in `crates/belt-agent/tests/cli_test.rs` first. Run:

```bash
grep -nE "^(fn|#\[test\]|fn .*regate)" crates/belt-agent/tests/cli_test.rs | grep -i regate
```

Identify the harness used by existing `regate_*` tests (typically: tempdir setup, `cargo run -p belt-agent --` invocations via `std::process::Command`, JSON parse of stdout). Append the new test using the SAME harness style — do NOT introduce a new helper. Concrete content:

```rust
/// scenario: belt-agent-regate-resolves-uri-in-target-gate
#[test]
fn regate_resolves_uri_in_target_gate() {
    let tmp = tempfile::tempdir().unwrap();
    let pipeline_path = tmp.path().join("pipeline.yml");
    // Two-phase pipeline: phase A produces a notes file via belt://current/
    // URI; phase B has regate=[A]. After completing A and entering B, run
    // regate and assert verdict PASS (URI resolves to a file that exists).
    std::fs::write(
        &pipeline_path,
        r#"name: t
version: 1
phases:
  - id: a
    description: x
    produces:
      - name: notes
        path: "belt://current/notes/phase-a.md"
    gate:
      - file_exists: "belt://current/notes/phase-a.md"
    confirm: true
  - id: b
    description: y
    regate: [a]
    confirm: true
"#,
    )
    .unwrap();

    // init
    let init = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .current_dir(tmp.path())
        .args(["init", pipeline_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(init.status.success(), "init stderr: {}", String::from_utf8_lossy(&init.stderr));
    let init_json: serde_json::Value = serde_json::from_slice(&init.stdout).unwrap();
    let run_id = init_json["run_id"].as_str().unwrap().to_string();

    // write the produces target
    let belt_dir = tmp.path().join(".belt");
    let run_dir = belt_dir.join("runs").join(&run_id);
    std::fs::create_dir_all(run_dir.join("notes")).unwrap();
    std::fs::write(run_dir.join("notes").join("phase-a.md"), "x").unwrap();

    // verify A
    let verify = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .current_dir(tmp.path())
        .args(["verify"])
        .output()
        .unwrap();
    assert!(verify.status.success());
    let verify_json: serde_json::Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(verify_json["verdict"], "PASS");

    // step to B
    let step = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .current_dir(tmp.path())
        .args(["step", "--confirm"])
        .output()
        .unwrap();
    assert!(step.status.success(), "step stderr: {}", String::from_utf8_lossy(&step.stderr));

    // regate (target = A; gate URI must resolve via current run_id)
    let regate = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .current_dir(tmp.path())
        .args(["regate"])
        .output()
        .unwrap();
    assert!(regate.status.success(), "regate stderr: {}", String::from_utf8_lossy(&regate.stderr));
    let regate_json: serde_json::Value = serde_json::from_slice(&regate.stdout).unwrap();
    assert_eq!(regate_json["all_passed"], true, "regate URI gate must resolve and pass");
    assert_eq!(regate_json["targets"]["a"]["passed"], true);
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p belt-agent --test cli_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/belt-agent/src/main.rs crates/belt-agent/tests/cli_test.rs
git commit -m "refactor(belt-agent): replace regate run_id substitute with URI resolution test"
```

---

# Phase E — Lint rule

## Task 16: Add `lint::check_belt_runs_literal` rule

**Files:**
- Modify: `crates/belt-core/src/lint.rs:130-140` (add new check call), append fn
- Modify: `crates/belt-core/tests/lint_test.rs` (add 4 fn)

- [ ] **Step 1: Write the failing tests in `lint_test.rs`**

Append to `crates/belt-core/tests/lint_test.rs`:

```rust
/// scenario: belt-core-lint-rejects-belt-runs-literal-in-produces-path
#[test]
fn lint_rejects_belt_runs_literal_in_produces_path() {
    let tmp = tempfile::tempdir().unwrap();
    let pipeline_path = tmp.path().join("pipeline.yml");
    std::fs::write(
        &pipeline_path,
        r#"name: t
version: 1
phases:
  - id: p
    description: x
    produces:
      - name: notes
        path: ".belt/runs/{run_id}/notes/phase-p.md"
    gate:
      - file_exists: ".belt/runs/{run_id}/notes/phase-p.md"
"#,
    )
    .unwrap();
    let diags = belt_core::lint::lint_pipeline(&pipeline_path).unwrap();
    assert!(
        diags.iter().any(|d| d.severity == belt_core::lint::Severity::Error
            && d.message.contains(".belt/runs/")),
        "lint must reject .belt/runs/ literal in produces.path"
    );
}

/// scenario: belt-core-lint-rejects-belt-runs-literal-in-gate-file-exists
#[test]
fn lint_rejects_belt_runs_literal_in_gate_file_exists() {
    let tmp = tempfile::tempdir().unwrap();
    let pipeline_path = tmp.path().join("pipeline.yml");
    std::fs::write(
        &pipeline_path,
        r#"name: t
version: 1
phases:
  - id: p
    description: x
    gate:
      - file_exists: ".belt/runs/{run_id}/handover.md"
    confirm: true
"#,
    )
    .unwrap();
    let diags = belt_core::lint::lint_pipeline(&pipeline_path).unwrap();
    assert!(
        diags.iter().any(|d| d.severity == belt_core::lint::Severity::Error
            && d.message.contains(".belt/runs/"))
    );
}

/// scenario: belt-core-lint-rejects-run-id-template-in-produces-path
#[test]
fn lint_rejects_run_id_template_in_produces_path() {
    let tmp = tempfile::tempdir().unwrap();
    let pipeline_path = tmp.path().join("pipeline.yml");
    std::fs::write(
        &pipeline_path,
        r#"name: t
version: 1
phases:
  - id: p
    description: x
    produces:
      - name: notes
        path: "belt://current/notes/{run_id}.md"
    gate:
      - file_exists: "belt://current/notes/{run_id}.md"
"#,
    )
    .unwrap();
    let diags = belt_core::lint::lint_pipeline(&pipeline_path).unwrap();
    assert!(
        diags.iter().any(|d| d.severity == belt_core::lint::Severity::Error
            && d.message.contains("{run_id}"))
    );
}

/// scenario: belt-core-lint-rejects-run-id-template-in-gate-cmd
#[test]
fn lint_rejects_run_id_template_in_gate_cmd() {
    let tmp = tempfile::tempdir().unwrap();
    let pipeline_path = tmp.path().join("pipeline.yml");
    std::fs::write(
        &pipeline_path,
        r#"name: t
version: 1
phases:
  - id: p
    description: x
    gate:
      - cmd: "test -f .belt/runs/{run_id}/x"
"#,
    )
    .unwrap();
    let diags = belt_core::lint::lint_pipeline(&pipeline_path).unwrap();
    assert!(
        diags.iter().any(|d| d.severity == belt_core::lint::Severity::Error
            && (d.message.contains("{run_id}") || d.message.contains(".belt/runs/")))
    );
}
```

- [ ] **Step 2: Run tests to confirm they fail**

Run: `cargo test -p belt-core --test lint_test lint_rejects_`
Expected: 4 FAILS

- [ ] **Step 3: Implement the lint rule**

Append to `crates/belt-core/src/lint.rs`:

```rust
/// Lint rule: reject raw `.belt/runs/` path literals and `{run_id}` template
/// strings in `produces[].path`, `gate.file_exists`, and `gate.cmd`. These
/// constructs were removed by the 2026-04-18 belt://current URI migration;
/// pipeline.yml authors must use `belt://current/<path>` instead.
fn check_belt_runs_literal(pipeline: &Pipeline, diagnostics: &mut Vec<LintDiagnostic>) {
    fn check_string(
        s: &str,
        phase_id: &str,
        field: &str,
        diagnostics: &mut Vec<LintDiagnostic>,
    ) {
        if s.contains(".belt/runs/") {
            diagnostics.push(LintDiagnostic {
                severity: Severity::Error,
                message: format!(
                    "phase '{phase_id}': {field} contains forbidden '.belt/runs/' literal — use belt://current/<path>"
                ),
            });
        }
        if s.contains("{run_id}") {
            diagnostics.push(LintDiagnostic {
                severity: Severity::Error,
                message: format!(
                    "phase '{phase_id}': {field} contains forbidden '{{run_id}}' template — use belt://current/<path>"
                ),
            });
        }
    }

    for phase in &pipeline.phases {
        for art in &phase.produces {
            check_string(&art.path, &phase.id, "produces.path", diagnostics);
        }
        for g in &phase.gate {
            match g {
                GateCheck::FileExists { file_exists } => {
                    check_string(file_exists, &phase.id, "gate.file_exists", diagnostics);
                }
                GateCheck::Cmd { cmd, .. } => {
                    check_string(cmd, &phase.id, "gate.cmd", diagnostics);
                }
                _ => {}
            }
        }
    }
}
```

Then locate `lint_pipeline` (around line 130) and add the call AFTER `check_artifact_when_references`:

```rust
    // Check: no `.belt/runs/` literal or `{run_id}` template in pipeline strings.
    check_belt_runs_literal(&pipeline, &mut diagnostics);
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p belt-core --test lint_test lint_rejects_`
Expected: 4 PASS

Run: `cargo test -p belt-core`
Expected: all PASS

- [ ] **Step 5: Add 4 scenarios to belt-core.yml**

Append to `docs/testing/cli-behavior/belt-core.yml`:

```yaml
  # --- lint::check_belt_runs_literal (2026-04-18 spec) ---
  - id: belt-core-lint-rejects-belt-runs-literal-in-produces-path
    category: lint
    severity: high
    technique: equivalence-partition
    given: "a pipeline.yml with produces.path = '.belt/runs/{run_id}/...'"
    when: "lint_pipeline is called"
    then: "an Error diagnostic mentions '.belt/runs/' is forbidden"
  - id: belt-core-lint-rejects-belt-runs-literal-in-gate-file-exists
    category: lint
    severity: high
    technique: equivalence-partition
    given: "a pipeline.yml with gate.file_exists = '.belt/runs/...'"
    when: "lint_pipeline is called"
    then: "an Error diagnostic mentions '.belt/runs/' is forbidden"
  - id: belt-core-lint-rejects-run-id-template-in-produces-path
    category: lint
    severity: high
    technique: equivalence-partition
    given: "a pipeline.yml whose produces.path contains '{run_id}' template"
    when: "lint_pipeline is called"
    then: "an Error diagnostic mentions '{run_id}' template is forbidden"
  - id: belt-core-lint-rejects-run-id-template-in-gate-cmd
    category: lint
    severity: high
    technique: equivalence-partition
    given: "a pipeline.yml with gate.cmd containing '{run_id}' template"
    when: "lint_pipeline is called"
    then: "an Error diagnostic flags either '{run_id}' or '.belt/runs/' usage"
```

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/lint.rs crates/belt-core/tests/lint_test.rs docs/testing/cli-behavior/belt-core.yml
git commit -m "feat(belt-core): lint rejects .belt/runs/ literals + {run_id} templates"
```

---

# Phase F — scenarios.yml + new test fn binding (belt-agent)

## Task 17: Add 14 belt-agent scenarios + scope update

**Files:**
- Modify: `docs/testing/cli-behavior/belt-agent.yml`

- [ ] **Step 1: Replace the file content**

Currently the file is a stub. Replace its content with:

```yaml
scope: "belt-agent CLI subcommand JSON contract + state.json shape. 2026-04-18: locate command + status URI shape (14 scenarios). F3 で全 subcommand カバレッジ拡充予定"
scenarios:
  - id: belt-agent-locate-resolves-current-uri-happy
    category: locate
    severity: high
    technique: equivalence-partition
    given: "an existing run with a file at <run_dir>/notes/phase-design.md"
    when: "`belt-agent locate belt://current/notes/phase-design.md` is invoked"
    then: "stdout JSON has uri, path (absolute), exists=true; exit 0"
  - id: belt-agent-locate-defaults-to-latest-run
    category: locate
    severity: high
    technique: equivalence-partition
    given: "multiple runs in .belt/runs/ with the latest being run_id X"
    when: "`belt-agent locate belt://current/<path>` is invoked WITHOUT --run"
    then: "the resolved path is rooted at <belt_dir>/runs/X/<path>"
  - id: belt-agent-locate-uses-explicit-run
    category: locate
    severity: high
    technique: equivalence-partition
    given: "multiple runs in .belt/runs/ and --run <id> targeting a non-latest run"
    when: "`belt-agent locate belt://current/<path> --run <id>` is invoked"
    then: "the resolved path is rooted at <belt_dir>/runs/<id>/<path>"
  - id: belt-agent-locate-errors-when-no-current-run
    category: locate
    severity: high
    technique: error-guessing
    given: "no runs exist in .belt/runs/ and --run is not passed"
    when: "`belt-agent locate belt://current/<path>` is invoked"
    then: "exit non-zero with a NoCurrentRun-style miette diagnostic on stderr"
  - id: belt-agent-locate-errors-on-malformed-uri
    category: locate
    severity: medium
    technique: error-guessing
    given: "an invalid URI string (e.g. 'belt://garbage')"
    when: "`belt-agent locate <bad-uri>` is invoked"
    then: "exit non-zero with a parse error diagnostic"
  - id: belt-agent-locate-emits-exists-false-for-missing-write-target
    category: locate
    severity: medium
    technique: boundary-value
    given: "a current run with no file yet at <run_dir>/notes/phase-X.md"
    when: "`belt-agent locate belt://current/notes/phase-X.md` is invoked"
    then: "stdout JSON has exists=false and path equal to the would-be absolute path"
  - id: belt-agent-locate-resolves-glob-uri
    category: locate
    severity: medium
    technique: equivalence-partition
    given: "multiple files at <run_dir>/notes/phase-*.md and a glob URI"
    when: "`belt-agent locate belt://current/notes/phase-*.md` is invoked"
    then: "stdout JSON has exists=true; path matches one of the existing files"
  - id: belt-agent-locate-emits-glob-base-when-zero-match
    category: locate
    severity: medium
    technique: boundary-value
    given: "a glob URI with zero matching files"
    when: "`belt-agent locate <glob-uri>` is invoked"
    then: "stdout JSON has exists=false and path is the declared base (or empty match list)"
  - id: belt-agent-status-emits-uri-field-for-uri-produces
    category: status
    severity: high
    technique: equivalence-partition
    given: "a phase whose produces.path is declared as belt://current/..."
    when: "`belt-agent status` is invoked"
    then: "the produces entry in JSON has a 'uri' field equal to the declared URI"
  - id: belt-agent-status-emits-path-field-for-raw-produces
    category: status
    severity: high
    technique: equivalence-partition
    given: "a phase whose produces.path is a raw domain path (e.g. docs/features/*/design.md)"
    when: "`belt-agent status` is invoked"
    then: "the produces entry in JSON has a 'path' field equal to the declared path"
  - id: belt-agent-status-uri-and-path-are-mutually-exclusive
    category: status
    severity: medium
    technique: boundary-value
    given: "any produces entry in status output"
    when: "`belt-agent status` is invoked"
    then: "the entry has either 'uri' OR 'path' but never both simultaneously"
  - id: belt-agent-init-emits-uri-in-phase-produces
    category: init
    severity: medium
    technique: equivalence-partition
    given: "a pipeline whose first phase declares a belt://current/ produces URI"
    when: "`belt-agent init <pipeline.yml>` is invoked"
    then: "the init JSON's phase.produces[].uri matches the declared URI"
  - id: belt-agent-next-emits-uri-in-phase-produces
    category: next
    severity: medium
    technique: equivalence-partition
    given: "a pipeline phase that declares a belt://current/ produces URI"
    when: "`belt-agent next` is invoked"
    then: "the next JSON's phase.produces[].uri matches the declared URI"
  - id: belt-agent-regate-resolves-uri-in-target-gate
    category: regate
    severity: high
    technique: equivalence-partition
    given: "a regate target phase whose gate.file_exists is a belt://current/ URI"
    when: "`belt-agent regate` runs against the target"
    then: "the URI is resolved against the current run_dir and the gate passes when the file exists"
```

- [ ] **Step 2: Run scenarios_contract.rs**

Run: `cargo test -p belt-core --test scenarios_contract`
Expected: at this point, scenarios are declared but doc-comment binding is not yet present in test fns. The contract may report missing doc-comments; if the test FAILs, it is expected — Tasks 18-22 will add the bindings.

If `scenarios_contract` is strict (FAIL on missing binding), defer this commit until Tasks 18-22 are done. Otherwise commit now and proceed.

- [ ] **Step 3: Commit**

```bash
git add docs/testing/cli-behavior/belt-agent.yml
git commit -m "test(belt-agent): declare 14 locate + status URI scenarios in cli-behavior yml"
```

---

## Task 18: Add belt-agent integration tests for `locate` (8 fn) + status URI shape (5 fn) + regate URI (1 fn)

**Files:**
- Modify: `crates/belt-agent/tests/cli_test.rs` (append 14 fn)

Implementation note: the existing `cli_test.rs` has helpers (look for `fn fixture_*` or similar). The subagent should read the file's existing helpers first and write each fn using the same harness style.

- [ ] **Step 1: Identify the existing fixture helpers**

Run: `grep -nE "^fn |^#\[test\]" crates/belt-agent/tests/cli_test.rs | head -40`

Note the helpers (e.g., `fn write_pipeline`, `fn run_belt_agent`, etc.) and the typical test pattern.

- [ ] **Step 2: Append 14 new tests, each with `/// scenario: <id>` doc-comment**

Each test must use the existing harness. Pattern (subagent: replicate the harness structure):

```rust
/// scenario: belt-agent-locate-resolves-current-uri-happy
#[test]
fn locate_resolves_current_uri_happy() {
    let tmp = tempfile::tempdir().unwrap();
    let belt_dir = tmp.path().join(".belt");
    let run_dir = belt_dir.join("runs").join("01947abc");
    std::fs::create_dir_all(run_dir.join("notes")).unwrap();
    std::fs::write(run_dir.join("notes").join("phase-design.md"), "x").unwrap();
    std::fs::write(
        run_dir.join("state.json"),
        r#"{"run_id":"01947abc","pipeline":"t","pipeline_file":"x","version":1,"args":{},"current_phase":"p","completed_phases":[],"skipped_phases":[],"created_at":"2026-04-18T00:00:00Z","updated_at":"2026-04-18T00:00:00Z","status":"in_progress"}"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .current_dir(tmp.path())
        .args(["locate", "belt://current/notes/phase-design.md"])
        .output()
        .unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["uri"], "belt://current/notes/phase-design.md");
    assert_eq!(json["exists"], true);
    assert!(json["path"].as_str().unwrap().ends_with("notes/phase-design.md"));
}
```

(Pattern repeats for each of the 14 scenario IDs in Task 17. Each test must have the `/// scenario:` doc-comment. The subagent constructs each test's GIVEN/WHEN/THEN per the scenario yml entry. For status/init/next variants, use a real `belt-agent init` first to set up state, then invoke status/init/next and assert the JSON shape.)

- [ ] **Step 3: Run all new tests**

Run: `cargo test -p belt-agent --test cli_test locate_ status_emits_uri status_emits_path init_emits_uri next_emits_uri regate_resolves_uri`
Expected: 14 PASS

Run: `cargo test -p belt-core --test scenarios_contract`
Expected: PASS (yml ↔ doc-comment now consistent for these IDs)

- [ ] **Step 4: Commit**

```bash
git add crates/belt-agent/tests/cli_test.rs
git commit -m "test(belt-agent): bind 14 locate + status URI scenarios via /// scenario: doc-comments"
```

---

## Task 19: Add e2e URI workflow test

**Files:**
- Modify: `crates/belt-agent/tests/e2e_test.rs`

- [ ] **Step 1: Add a single end-to-end test with a real pipeline + URI gate**

Append to `crates/belt-agent/tests/e2e_test.rs`:

```rust
/// scenario: belt-agent-locate-resolves-current-uri-happy
#[test]
fn e2e_init_verify_step_with_belt_current_uri() {
    let tmp = tempfile::tempdir().unwrap();
    let pipeline_path = tmp.path().join("pipeline.yml");
    std::fs::write(
        &pipeline_path,
        r#"name: e2e
version: 1
phases:
  - id: p
    description: x
    produces:
      - name: notes
        path: "belt://current/notes/p.md"
    gate:
      - file_exists: "belt://current/notes/p.md"
    confirm: true
"#,
    )
    .unwrap();

    // init
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .current_dir(tmp.path())
        .args(["init", pipeline_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let init_json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let run_id = init_json["run_id"].as_str().unwrap();

    // write the produces target file
    let belt_dir = tmp.path().join(".belt");
    let run_dir = belt_dir.join("runs").join(run_id);
    std::fs::create_dir_all(run_dir.join("notes")).unwrap();
    std::fs::write(run_dir.join("notes").join("p.md"), "x").unwrap();

    // verify
    let verify = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .current_dir(tmp.path())
        .args(["verify"])
        .output()
        .unwrap();
    let verify_json: serde_json::Value = serde_json::from_slice(&verify.stdout).unwrap();
    assert_eq!(verify_json["verdict"], "PASS");
}
```

(Note: this scenario id is reused; if the contract requires unique IDs per `/// scenario:` doc-comment, drop the doc-comment for this test or add a new scenario. For simplicity, drop the doc-comment — this is an integration test, not a behavior assertion.)

- [ ] **Step 2: Run**

Run: `cargo test -p belt-agent --test e2e_test e2e_init_verify_step_with_belt_current_uri`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/belt-agent/tests/e2e_test.rs
git commit -m "test(belt-agent): add e2e init→verify happy path with belt://current/ URI"
```

---

# Phase G — Plugin SKILL.md / criteria / agents path elimination

(Tasks 20-30 follow the same pattern: read each file, replace `.belt/runs/{run_id}/...` and `.belt/runs/<run_id>/...` and `{run_id}` references with appropriate replacements per file role. Patterns:

- **SKILL.md narrative explanation**: replace path with "see `belt-agent status` produces.resolved_path or `belt-agent locate belt://current/notes/phase-<id>.md`"
- **criteria/*.md verify steps**: replace "Verify file exists at .belt/runs/<run_id>/notes/X.md" with "Read `belt-agent status` and locate the artifact's resolved_path; verify the file exists at that path" + change `depends_on_artifacts` from path glob to artifact name list
- **agents/*.md output sections**: replace "Write findings to .belt/runs/{run_id}/review/findings-X.json" with "Write findings to the path provided in your prompt's `output_path` field"
- **code-review/spec-review SKILL.md**: add an "Output path resolution" step where orchestrator calls `belt-agent status` to get each finding artifact's resolved_path and embeds it in agent prompts

Each task in Phase G follows the same 4 steps: read file → grep `.belt/runs\|{run_id}` → replace per pattern → commit.)

## Task 20: Update `plugins/belt/skills/feature-dev/SKILL.md`

**Files:**
- Modify: `plugins/belt/skills/feature-dev/SKILL.md`

- [ ] **Step 1: Locate path references**

Run: `grep -n "\.belt/runs\|{run_id}" plugins/belt/skills/feature-dev/SKILL.md`
Expected matches around line 48 (`.belt/runs/{run_id}/notes/phase-<id>.md`).

- [ ] **Step 2: Update the Narrative Notes section**

Locate the "## Narrative Notes" section (around line 46-62). Replace:

```markdown
These phases produce a narrative note so context can be restored after `/clear`
(`.belt/runs/{run_id}/notes/phase-<id>.md`):
```

With:

```markdown
These phases produce a narrative note so context can be restored after `/clear`.
The note paths are declared in `pipeline.yml` as `belt://current/notes/phase-<id>.md` URIs.
Resolve the physical path via `belt-agent status` (read `phases[].produces[].resolved_path`)
or `belt-agent locate belt://current/notes/phase-<id>.md`:
```

- [ ] **Step 3: Verify no remaining literals**

Run: `grep -n "\.belt/runs\|{run_id}" plugins/belt/skills/feature-dev/SKILL.md`
Expected: no matches.

- [ ] **Step 4: Commit**

```bash
git add plugins/belt/skills/feature-dev/SKILL.md
git commit -m "refactor(plugins/feature-dev): drop path literals from SKILL.md, defer to status/locate"
```

---

## Task 21: Update `plugins/belt/skills/bug-fix/SKILL.md`

**Files:**
- Modify: `plugins/belt/skills/bug-fix/SKILL.md`

Same pattern as Task 20.

- [ ] **Step 1: Grep**: `grep -n "\.belt/runs\|{run_id}" plugins/belt/skills/bug-fix/SKILL.md`
- [ ] **Step 2: Replace** the path literal in the Narrative Notes section with the same replacement from Task 20 Step 2 (s/feature-dev/bug-fix/ where appropriate)
- [ ] **Step 3: Verify** no remaining literals
- [ ] **Step 4: Commit** `refactor(plugins/bug-fix): drop path literals from SKILL.md`

---

## Task 22: Update `plugins/belt/skills/handover/SKILL.md`

**Files:**
- Modify: `plugins/belt/skills/handover/SKILL.md`

- [ ] **Step 1: Grep**: `grep -n "\.belt/runs\|{run_id}\|<run_id>" plugins/belt/skills/handover/SKILL.md`

Multiple matches expected (line 19, 33, 46).

- [ ] **Step 2: Replace path references**

Replace `.belt/runs/<run_id>/handover.md` with the resolved path obtained via `belt-agent locate belt://current/handover.md`. Update the workflow step 5 to:

```markdown
- [ ] Step 5: Write the handover note to the path returned by `belt-agent locate belt://current/handover.md`
```

Update the Step detail #5 to:

```markdown
5. Resolve the target path via `belt-agent locate belt://current/handover.md`
   (read the `path` field from the JSON output) and write the file there with
   the schema below. Overwrite if it exists.
```

- [ ] **Step 3: Verify**: `grep -n "\.belt/runs" plugins/belt/skills/handover/SKILL.md` returns nothing
- [ ] **Step 4: Commit** `refactor(plugins/handover): use belt-agent locate instead of path literal`

---

## Task 23: Update `plugins/belt/skills/resume/SKILL.md`

**Files:**
- Modify: `plugins/belt/skills/resume/SKILL.md`

- [ ] **Step 1: Grep**: `grep -n "\.belt/runs\|{run_id}\|<run_id>" plugins/belt/skills/resume/SKILL.md`

Matches expected at lines 30, 41.

- [ ] **Step 2: Replace** workflow step 2 and preconditions table:

For Step 2 (line 30):

```markdown
- [ ] Step 2: Resolve handover path via `belt-agent locate belt://current/handover.md`; read the file at the returned path; load Resume hint into context
```

For preconditions row 3 (line 41), replace `.belt/runs/<run_id>/handover.md` with `belt://current/handover.md` (URI form):

```markdown
| 3 | `belt-agent locate belt://current/handover.md` returns exists=true | "No handover note for latest run. Run /belt:handover first." | run `/belt:handover` first / abort |
```

- [ ] **Step 3: Verify**: `grep -n "\.belt/runs" plugins/belt/skills/resume/SKILL.md` returns nothing
- [ ] **Step 4: Commit** `refactor(plugins/resume): use belt-agent locate for handover path`

---

## Task 24: Update `plugins/belt/skills/code-review/SKILL.md`

**Files:**
- Modify: `plugins/belt/skills/code-review/SKILL.md`

- [ ] **Step 1: Grep**: `grep -n "\.belt/runs\|{run_id}\|<run_id>" plugins/belt/skills/code-review/SKILL.md`

Matches at lines 38, 40, 48, 55.

- [ ] **Step 2: Replace orchestrator workflow**

The skill currently embeds physical paths into agent dispatch prompts. Replace those occurrences with a workflow that calls `belt-agent status` first to get each artifact's resolved_path, then embeds the resolved path into the agent prompts.

Replace the "## Parallel Dispatch" section's mention of `.belt/runs/<run_id>/review/findings-<observation>.json` with:

```markdown
Before dispatching agents, call `belt-agent status` and read each finding artifact's
`resolved_path` (artifacts named `findings-security`, `findings-test`,
`findings-ai-antipattern`, `findings-cross-cutting`, optionally `findings-codex`,
and `findings` for the merged output). Pass the resolved physical path to each
agent in its prompt as `output_path`. Agents write to that path without knowing
the underlying URI semantics.

Dispatch observation agents in parallel via the Agent (Task) tool. Send all Task calls in **one single message** with multiple tool-use blocks so they run concurrently:

- `Task(subagent_type: belt:security-reviewer, prompt: <diff + output_path: <resolved-findings-security>>)`
- `Task(subagent_type: belt:test-reviewer, prompt: <diff + output_path: <resolved-findings-test>>)`
- `Task(subagent_type: belt:ai-antipattern-reviewer, prompt: <diff + output_path: <resolved-findings-ai-antipattern>>)`
- `Task(subagent_type: belt:cross-cutting-reviewer, prompt: <diff + optional design-doc Impact Analysis + output_path: <resolved-findings-cross-cutting>>)`

If `--codex` is set, also invoke `/codex:rescue` in the same parallel batch with a review-specific prompt: supply the diff, the expected `findings-codex.json` format (same shape as observation agents, `source: "codex"`), and the resolved `output_path` (from `belt-agent status` `findings-codex` artifact).
```

Replace the "## Merge + Cross-agent Dedup" section's path references with:

```markdown
After all agents complete:

1. For each finding artifact name (`findings-security`, `findings-test`,
   `findings-ai-antipattern`, `findings-cross-cutting`, optionally `findings-codex`),
   call `belt-agent status` to get the resolved_path, then read the JSON file at that path.
2. ... (existing dedup logic) ...
4. Resolve `findings` artifact's path via `belt-agent status` (or
   `belt-agent locate belt://current/review/findings.json`) and write the merged JSON there.
```

- [ ] **Step 3: Verify**: `grep -n "\.belt/runs" plugins/belt/skills/code-review/SKILL.md` returns nothing
- [ ] **Step 4: Commit** `refactor(plugins/code-review): orchestrator resolves paths via belt-agent status, agents receive output_path`

---

## Task 25: Update `plugins/belt/skills/spec-review/SKILL.md`

**Files:**
- Modify: `plugins/belt/skills/spec-review/SKILL.md`

Same pattern as Task 24, applied to spec-review observations (`findings-feasibility`, `findings-cross-cutting-spec`, `findings-ui-design`, optional `findings-codex`, `findings`).

- [ ] **Step 1: Grep**: `grep -n "\.belt/runs\|{run_id}\|<run_id>" plugins/belt/skills/spec-review/SKILL.md`
- [ ] **Step 2: Replace** path literals with `belt-agent status` + `output_path` pattern (mirror Task 24)
- [ ] **Step 3: Verify** no remaining literals
- [ ] **Step 4: Commit** `refactor(plugins/spec-review): orchestrator resolves paths via belt-agent status, agents receive output_path`

---

## Task 26: Update `plugins/belt/skills/feature-dev/criteria/*.md` (6 files)

**Files:**
- Modify: `plugins/belt/skills/feature-dev/criteria/{design,plan,test-scenarios,spec-review,execute,code-review,monkey-test,dogfood,integrate}.md` (subset that has narrative or review verify steps)

- [ ] **Step 1: Identify which criteria files contain path literals**

Run: `grep -ln "\.belt/runs\|{run_id}\|<run_id>" plugins/belt/skills/feature-dev/criteria/*.md`

- [ ] **Step 2: For each affected file, replace verify step paths and `depends_on_artifacts` lists**

Pattern: in each verify step that says "Verify file exists at `.belt/runs/<run_id>/notes/phase-<id>.md`", replace with:

```markdown
  1. Read `belt-agent status` and locate the artifact `<artifact-name>` resolved_path
  2. Verify the file exists at the resolved_path
```

For `depends_on_artifacts: [.belt/runs/*/notes/phase-<id>.md]`, replace with the artifact name:

```markdown
- **depends_on_artifacts**: [<artifact-name>]
```

(e.g., `[design_notes]`, `[plan_notes]`, `[execute_notes]`, `[code_review_notes]`, `[findings]` etc.)

- [ ] **Step 3: Verify**: `grep -n "\.belt/runs" plugins/belt/skills/feature-dev/criteria/*.md` returns nothing
- [ ] **Step 4: Commit** `refactor(plugins/feature-dev/criteria): drop path literals, reference artifact names`

---

## Task 27: Update `plugins/belt/skills/bug-fix/criteria/*.md` (6 files)

Same pattern as Task 26 applied to bug-fix.

- [ ] **Step 1: Grep**: `grep -ln "\.belt/runs\|{run_id}\|<run_id>" plugins/belt/skills/bug-fix/criteria/*.md`
- [ ] **Step 2: Replace** per Task 26 Step 2 pattern
- [ ] **Step 3: Verify** no remaining literals
- [ ] **Step 4: Commit** `refactor(plugins/bug-fix/criteria): drop path literals, reference artifact names`

---

## Task 28: Update `plugins/belt/agents/*.md` (7 files)

**Files:**
- Modify: `plugins/belt/agents/{security-reviewer,test-reviewer,ai-antipattern-reviewer,cross-cutting-reviewer,cross-cutting-spec-reviewer,feasibility-reviewer,ui-design-reviewer}.md`

- [ ] **Step 1: Grep**: `grep -ln "\.belt/runs\|{run_id}" plugins/belt/agents/*.md`

All 7 agents are expected to match.

- [ ] **Step 2: For each agent file, replace the "## Output Format" section**

In each agent file, locate the "## Output Format" section. Replace the line that says "Write findings to `.belt/runs/{run_id}/review/findings-<X>.json`" with:

```markdown
## Output Format

Write findings to the path provided in your prompt's `output_path` field:

\`\`\`json
{
  "observation": "<observation-name>",
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
\`\`\`

The orchestrator skill resolves the artifact path via `belt-agent status`
and passes it to you as `output_path`. Do not construct the path yourself.
```

(Replace `<observation-name>` with the agent's observation: `security`, `test`, `ai-antipattern`, `cross-cutting`, `cross-cutting-spec`, `feasibility`, `ui-design`.)

- [ ] **Step 3: Verify**: `grep -n "\.belt/runs" plugins/belt/agents/*.md` returns nothing
- [ ] **Step 4: Commit** `refactor(plugins/agents): agents receive output_path runtime arg, no path knowledge`

---

## Task 29: Update `plugins/belt-agent/skills/protocol/SKILL.md` — add "Path Resolution" section

**Files:**
- Modify: `plugins/belt-agent/skills/protocol/SKILL.md`

- [ ] **Step 1: Locate the insertion point**

Read the file. Find the "## Well-known Config Keys" section.

- [ ] **Step 2: Insert the new "Path Resolution" section BEFORE "## Well-known Config Keys"**

Insert:

```markdown
## Path Resolution

belt does not expose physical paths to skill authors. All artifact paths are
declared in pipeline.yml as `belt://current/<path>` URIs (or raw paths for
domain artifacts under `docs/`, `src/`, etc.).

### Reading paths

To get the physical path of an artifact:

1. Call `belt-agent status` and read `phases[].produces[].resolved_path`.
2. Or call `belt-agent locate <uri>` for direct URI resolution.

### Passing paths to subagents

When dispatching a subagent (Task tool), the orchestrator skill MUST resolve
the URI to a physical path and pass it as `output_path` in the subagent's
prompt. Subagents do not see URIs and do not call `belt-agent locate`
themselves — they receive a concrete path.

### Forbidden patterns

- Hardcoding `.belt/runs/<run_id>/...` literals in SKILL.md, criteria/*.md,
  or agents/*.md (lint enforces this in pipeline.yml).
- `{run_id}` template strings (removed from belt-core; lint rejects them).
- Constructing paths inside agent prompts using string interpolation.

```

- [ ] **Step 3: Verify** the section is in place
- [ ] **Step 4: Commit** `docs(plugins/protocol): add Path Resolution section to SKILL.md`

---

## Task 30: Update `plugins/belt-agent/references/narrative-convention.md` + 2 other references

**Files:**
- Modify: `plugins/belt-agent/references/narrative-convention.md`
- Modify: `plugins/belt/skills/feature-dev/references/path-convention.md`
- Modify: `plugins/belt-agent/skills/protocol/references/resume-mode.md`

- [ ] **Step 1: Update `narrative-convention.md`**

Locate the "## Path" section (lines 9-17). Replace with:

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

- [ ] **Step 2: Update `path-convention.md`**

Run: `grep -n "\.belt/runs" plugins/belt/skills/feature-dev/references/path-convention.md`

Match expected at line 61 (`.belt/runs/*/review/findings.json`). Replace with:

```markdown
... `belt://current/review/findings.json` (resolved via `belt-agent status` or `belt-agent locate`), not under `docs/features/`.
```

- [ ] **Step 3: Update `resume-mode.md`**

Run: `grep -n "\.belt/runs" plugins/belt-agent/skills/protocol/references/resume-mode.md`

Match expected at line 24. Replace `.belt/runs/<id>/handover.md` with:

```markdown
4. If `belt-agent locate belt://run/<id>/handover.md` returns exists=true, read it and incorporate the
```

(Use `belt://run/<id>/...` here because resume-mode targets a specific run id, not the current invocation.)

- [ ] **Step 4: Verify**: `grep -n "\.belt/runs" plugins/belt-agent/ plugins/belt/skills/*/references/` returns nothing
- [ ] **Step 5: Commit** `docs(plugins/refs): convert path literals to belt:// URI references in 3 reference docs`

---

# Phase H — Shape lock test 更新

## Task 31: Update `feature_dev_refresh.rs` — 2 new fn + URI dimensions

**Files:**
- Modify: `crates/belt-core/tests/feature_dev_refresh.rs`
- Modify: `crates/belt-core/tests/common/narrative.rs` (the `assert_narrative_*` helpers reference path patterns)

- [ ] **Step 1: Update narrative helpers to expect URI form**

Run: `grep -n "\.belt/runs" crates/belt-core/tests/common/narrative.rs`

For each match, replace the expected path pattern from `.belt/runs/{run_id}/notes/phase-<id>.md` to `belt://current/notes/phase-<id>.md`. The narrative helper compares `phase.produces[].path` — adjust the expected string accordingly.

- [ ] **Step 2: Add 2 new shape lock tests**

Append to `crates/belt-core/tests/feature_dev_refresh.rs`:

```rust
#[test]
fn feature_dev_produces_use_belt_current_uri() {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path()).unwrap();
    // For each narrative-producing phase, the corresponding produces entry
    // must use the belt://current/notes/phase-<id>.md URI form (not a raw
    // .belt/runs literal).
    let narrative_phases = ["design", "plan", "execute", "code-review", "monkey-test", "dogfood"];
    for phase in &pipeline.phases {
        if !narrative_phases.contains(&phase.id.as_str()) {
            continue;
        }
        let notes_artifact = phase
            .produces
            .iter()
            .find(|a| a.name.ends_with("_notes"))
            .unwrap_or_else(|| panic!("phase {} missing notes artifact", phase.id));
        let expected = format!("belt://current/notes/phase-{}.md", phase.id);
        assert_eq!(
            notes_artifact.path, expected,
            "phase {} notes path must equal {expected}",
            phase.id
        );
    }
}

#[test]
fn feature_dev_pipeline_has_no_run_id_template() {
    let yaml = std::fs::read_to_string(feature_dev_pipeline_path()).unwrap();
    assert!(
        !yaml.contains("{run_id}"),
        "pipeline must not contain {{run_id}} template anywhere"
    );
    assert!(
        !yaml.contains(".belt/runs/"),
        "pipeline must not contain .belt/runs/ literal anywhere"
    );
}
```

- [ ] **Step 3: Update `code-review` produces count assertion (if any new test needs adding)**

Add an assertion that code-review phase has 7 produces entries:

```rust
#[test]
fn feature_dev_code_review_produces_seven_artifacts() {
    let pipeline = parse_pipeline(&feature_dev_pipeline_path()).unwrap();
    let code_review = pipeline
        .phases
        .iter()
        .find(|p| p.id == "code-review")
        .unwrap();
    let names: Vec<&str> = code_review.produces.iter().map(|a| a.name.as_str()).collect();
    let expected = [
        "findings-security",
        "findings-test",
        "findings-ai-antipattern",
        "findings-cross-cutting",
        "findings-codex",
        "findings",
        "code_review_notes",
    ];
    assert_eq!(names, expected, "code-review produces names + order must match");
}
```

(This is the 3rd new fn, but lock-ledger says +2 fn — either keep both new narrative-related fn and merge code-review assertion into one of them, or accept +3. Use +3 and update lock-ledger accordingly in Task 34.)

- [ ] **Step 4: Run all feature_dev_refresh tests**

Run: `cargo test -p belt-core --test feature_dev_refresh`
Expected: PASS (all existing + 3 new)

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/tests/feature_dev_refresh.rs crates/belt-core/tests/common/narrative.rs
git commit -m "test(belt-core): lock feature-dev URI shape (3 new fn + URI-form narrative helpers)"
```

---

## Task 32: Update `bug_fix_refresh.rs` — 2 new fn + URI dimensions

**Files:**
- Modify: `crates/belt-core/tests/bug_fix_refresh.rs`

Same pattern as Task 31, applied to bug-fix.

- [ ] **Step 1: Add `bug_fix_produces_use_belt_current_uri`** (mirror Task 31 Step 2 first fn, with narrative phases `["rca", "fix-plan", "execute", "code-review", "monkey-test", "dogfood"]`)
- [ ] **Step 2: Add `bug_fix_pipeline_has_no_run_id_template`** (mirror Task 31 Step 2 second fn, with `bug_fix_pipeline_path()` helper)
- [ ] **Step 3: Add `bug_fix_code_review_produces_seven_artifacts`** (mirror Task 31 Step 3)
- [ ] **Step 4: Add `bug_fix_fix_plan_review_produces_spec_findings`** (assert fix-plan-review has the 5 spec-review artifacts: findings-feasibility, findings-cross-cutting-spec, findings-ui-design, findings-codex (when codex), findings)
- [ ] **Step 5: Run** `cargo test -p belt-core --test bug_fix_refresh`
- [ ] **Step 6: Commit** `test(belt-core): lock bug-fix URI shape (4 new fn + URI-form narrative helpers)`

---

## Task 33: Update `review_skills_refresh.rs` — 1 new fn for output_path agent pattern

**Files:**
- Modify: `crates/belt-core/tests/review_skills_refresh.rs`

- [ ] **Step 1: Append the new fn**

Append:

```rust
#[test]
fn per_observation_agents_use_output_path_arg_pattern() {
    use std::fs;
    let agents = [
        "security-reviewer.md",
        "test-reviewer.md",
        "ai-antipattern-reviewer.md",
        "cross-cutting-reviewer.md",
        "feasibility-reviewer.md",
        "cross-cutting-spec-reviewer.md",
        "ui-design-reviewer.md",
    ];
    for name in agents {
        let path = repo_root().join("plugins/belt/agents").join(name);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert!(
            content.contains("output_path"),
            "{name} must reference 'output_path' runtime arg in Output Format section"
        );
        assert!(
            !content.contains(".belt/runs/"),
            "{name} must not hardcode .belt/runs/ literals"
        );
    }
}
```

- [ ] **Step 2: Run** `cargo test -p belt-core --test review_skills_refresh`
- [ ] **Step 3: Commit** `test(belt-core): lock per-observation agents use output_path arg pattern`

---

# Phase I — lock-ledger.md

## Task 34: Update `docs/testing/lock-ledger.md` for the 3 affected entries

**Files:**
- Modify: `docs/testing/lock-ledger.md`

- [ ] **Step 1: Update `feature_dev_refresh.rs` entry**

Locate the entry. Update `test-fn-count: 13` to `test-fn-count: 16` (or matching the actual count after Task 31; use the +3 figure from Task 31 Step 3).

In the **13 test fn 名** list (now 16), append:
- `feature_dev_produces_use_belt_current_uri`
- `feature_dev_pipeline_has_no_run_id_template`
- `feature_dev_code_review_produces_seven_artifacts`

In **pipeline.yml shape dimensions locked** list, append:
- "narrative artifact 6 phase の path が exactly `belt://current/notes/phase-<id>.md` URI 形式"
- "code-review.produces が 7 entries (findings-security / findings-test / findings-ai-antipattern / findings-cross-cutting / findings-codex with `when: args.codex` / findings (merged) / code_review_notes)"
- "全 phase の `produces[].path` および `gate.file_exists` が `belt://...` URI または `docs/`/`src/` raw path のみ (`.belt/runs/` リテラル + `{run_id}` template の non-existence)"

Remove (or rewrite to URI form) the existing dimension that mentions `.belt/runs/{run_id}/notes/phase-*.md`.

- [ ] **Step 2: Update `bug_fix_refresh.rs` entry**

Update `test-fn-count: 21` to `test-fn-count: 25` (per Task 32 added 4 fn).

Append the 4 new fn names. Append the same shape dimensions as Step 1 (s/feature-dev/bug-fix/).

- [ ] **Step 3: Update `review_skills_refresh.rs` entry**

Update `test-fn-count: 6` to `test-fn-count: 7`.

Append `per_observation_agents_use_output_path_arg_pattern` to the test-fn list.

In a new locked-shape dimension, add: "per-observation agents (`security-reviewer.md` etc.) reference `output_path` in their Output Format section and do not hardcode `.belt/runs/` literals".

- [ ] **Step 4: Run scenarios_contract.rs**

Run: `cargo test -p belt-core --test scenarios_contract`
Expected: PASS (lock-ledger entries now reflect actual test counts)

- [ ] **Step 5: Commit**

```bash
git add docs/testing/lock-ledger.md
git commit -m "docs(testing): refresh lock-ledger for belt://current URI migration (3 entries)"
```

---

# Phase J — Final CI verification

## Task 35: Final workspace-wide CI verification

- [ ] **Step 1: Run `cargo fmt`**

Run: `cargo fmt --all -- --check`
If failures, run `cargo fmt --all` and commit.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS (zero warnings).

- [ ] **Step 3: Run all tests**

Run: `cargo test --workspace`
Expected: all PASS.

- [ ] **Step 4: Lint plugins pipelines**

Run:
```bash
cargo run -p belt -- lint plugins/belt/skills/feature-dev/pipeline.yml
cargo run -p belt -- lint plugins/belt/skills/bug-fix/pipeline.yml
cargo run -p belt -- lint plugins/belt/skills/handover/checkpoint.yml
```

Expected: lint passes (warnings about produces protection acceptable; zero errors).

- [ ] **Step 5: Smoke-test the locate command**

Run:
```bash
cd /tmp && mkdir -p belt-smoke && cd belt-smoke
cargo run -p belt-agent -- init /Users/nishikataseiichi/go/src/github.com/neko-neko/belt/plugins/belt/skills/feature-dev/pipeline.yml
cargo run -p belt-agent -- locate belt://current/notes/phase-design.md
```

Expected: locate returns JSON with `uri`, `path` (absolute under `<cwd>/.belt/runs/<run_id>/notes/phase-design.md`), `exists: false` (the file is not yet written).

Cleanup: `rm -rf /tmp/belt-smoke`.

- [ ] **Step 6: Final whole-tree grep**

Run:
```bash
grep -rn "\.belt/runs\|{run_id}" plugins/ docs/specs/2026-04-18-belt-uri-current-and-skill-path-elimination-design.md \
  | grep -v "^docs/specs/.*Background\b" \
  | grep -v "Forbidden patterns" \
  | head
```

Expected: only background/forbidden-patterns references remain (no live path literals in working configs).

- [ ] **Step 7: Final commit (if any cleanup landed)**

```bash
git status
# If clean, no commit. Otherwise:
git commit -m "chore: final cleanup after belt://current URI migration"
```

---

## Self-Review Checklist (executed at plan-write time, not subagent-side)

1. **Spec coverage**: All 12 sections of the spec map to tasks (Phase A-J covers Sections 1-12 of the spec; Section 11 test strategy maps to Phase F + H).
2. **Placeholder scan**: No "TBD" / "TODO" — each task has concrete code.
3. **Type consistency**: `BeltUri::Current { path: String }` used uniformly. `Resolver::current_run_id: Option<String>` matches across resolver/main.rs uses. `UriResolver::resolve(&self, uri: &str) -> Result<PathBuf, String>` matches gate caller signature.
4. **Test count consistency**: Plan tasks add 5+4+2+1+14+1 = 27 fn (scenario-bound) + 3+4+1 = 8 fn (shape lock) = 35 fn net (note: spec said +29 net = +32 added - 3 deleted; the plan adds +35 - 3 = +32 net. The +3 discrepancy comes from Task 31/32 adding `_seven_artifacts` and `_spec_findings` shape lock fn beyond the +2/+2 originally specified. Update Task 34 lock-ledger counts to match actual additions.)

---

## References

- `docs/specs/2026-04-18-belt-uri-current-and-skill-path-elimination-design.md` — design spec (parent)
- `docs/testing/README.md` — test foundation 3-layer structure
- `docs/testing/audit-template.md` — audit v1 reason labels
- `docs/testing/lock-ledger.md` — shape lock ledger
- `crates/belt-core/src/uri.rs` — existing URI scheme
- `crates/belt-core/src/engine.rs` — existing `expand_run_id` (Task 14 deletion target)
- `crates/belt-agent/src/resolver.rs` — existing `Resolver` (Task 5-7 extension target)
- memory `feedback_plan_test_code_grep_validation.md` — Rust test code validation requirement
- memory `feedback_subagent_prompt_verbatim_spec.md` — verbatim spec transcription requirement
- memory `project_belt_test_foundation_f1.md` — F1 test foundation establishment
