# Narrative Follow-Up Bundle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the 3 narrative-artifact follow-up tickets (BELT-33 + BELT-34 + BELT-35) from `docs/specs/2026-04-15-narrative-followup-design.md` as 3 isolated commits on branch `2026-04-15-narrative-followup`.

**Architecture:** Restructure `belt-agent::cmd_init` so External URI resolution happens before `engine.init_with_branch` (atomicity, BELT-33). Refresh `CLAUDE.md` to reflect all 10 belt-core modules (BELT-34). Add documented-as-tests adversarial probes for corrupt / schema-missing / directory state.json (BELT-35).

**Tech Stack:** Rust 1.94.1, Cargo workspace (`belt-core` + `belt-agent`), `assert_cmd`, `tempfile`, `serde_json`, `miette`. No new dependencies.

**Spec:** `docs/specs/2026-04-15-narrative-followup-design.md` (commits `21d0a68` + `5acd2ff`).

---

## Task ordering and rationale

- **Task 1 → BELT-34 commit** first (docs-only, lowest risk)
- **Task 2 → Task 3 → Task 4 → BELT-33 commit** (TDD: failing regression tests first, then the cmd_init restructure)
- **Task 5 → Task 6 → Task 7 → Task 8 → BELT-35 commit** (BELT-35 tests go on top of the post-BELT-33 cmd_init; unit tests in Tasks 5-7 are documented-as-tests that pass immediately against current resolver behaviour)

Each commit must leave `cargo test -p belt-agent` green.

---

### Task 1: Update `CLAUDE.md` belt-core module list (BELT-34)

**Files:**
- Modify: `CLAUDE.md:39` (architecture overview comment)
- Modify: `CLAUDE.md:61` (section heading "belt-core の 7 モジュール")
- Modify: `CLAUDE.md:63-71` (table, insert 3 rows before `error`)

- [ ] **Step 1: Update L39 architecture overview comment**

Edit `CLAUDE.md` L39.

Old:
```
│   ├── belt-core/    # 📦 library: model, parser, expander, engine, gate, lint
```

New:
```
│   ├── belt-core/    # 📦 library: 10 modules (model / parser / expander / engine / gate / lint / config / view / uri / error)
```

- [ ] **Step 2: Update L61 section heading**

Edit `CLAUDE.md` L61.

Old:
```
## belt-core の 7 モジュール
```

New:
```
## belt-core の 10 モジュール
```

- [ ] **Step 3: Insert 3 rows into the module table before `error`**

The table currently ends with:

```
| `lint` | `lint_pipeline()` — 7 静的検証 (duplicate IDs, regate, args, description, uses references) + expansion 試行 |
| `error` | `BeltError` (thiserror + miette) — YamlParse, FileNotFound, InvalidPipeline, GateFailed, State, Io |
```

Insert 3 rows between `lint` and `error` so the table reads:

```
| `lint` | `lint_pipeline()` — 7 静的検証 (duplicate IDs, regate, args, description, uses references) + expansion 試行 |
| `config` | `belt.toml` config 読み込み (相対 path 解決含む)。BELT-22 で追加 |
| `view` | `status` / `next` JSON の query-time assembly (YAML drift 反映)。BELT-29 で追加 |
| `uri` | `belt://` URI parser (Run / Latest / WorkspaceLatest)。pure library、I/O ゼロ。2026-04-15 narrative artifact で追加 |
| `error` | `BeltError` (thiserror + miette) — YamlParse, FileNotFound, InvalidPipeline, GateFailed, State, Io |
```

- [ ] **Step 4: Verify the module list in CLAUDE.md matches `crates/belt-core/src/`**

Run: `ls crates/belt-core/src/*.rs | grep -v lib.rs | sort`

Expected (10 files, alphabetical):
```
crates/belt-core/src/config.rs
crates/belt-core/src/engine.rs
crates/belt-core/src/error.rs
crates/belt-core/src/expander.rs
crates/belt-core/src/gate.rs
crates/belt-core/src/lint.rs
crates/belt-core/src/model.rs
crates/belt-core/src/parser.rs
crates/belt-core/src/uri.rs
crates/belt-core/src/view.rs
```

Then: `grep -c '^| \`' CLAUDE.md | head -1` — confirm the belt-core table has 10 module rows.

- [ ] **Step 5: Commit BELT-34**

```bash
git add CLAUDE.md
git -c commit.gpgsign=false commit -m "docs(claude-md): update belt-core module list to 10 (BELT-34)

Reflect the three modules added after the original 7-module
scaffolding:

- config (BELT-22) — belt.toml config loader with path resolution
- view (BELT-29) — status/next JSON query-time assembly
- uri (2026-04-15 narrative artifact) — belt:// URI parser

The stale count ('7 modules') was 2 revisions behind actual code
and would confuse agents using CLAUDE.md as their primary source
of project structure."
```

---

### Task 2: Add orphan-absence assertion to existing "no completed producer" E2E (BELT-33, failing test)

**Files:**
- Modify: `crates/belt-agent/tests/e2e_test.rs:584-634` (extend `e2e_consumer_init_fails_when_no_completed_producer`)

- [ ] **Step 1: Insert orphan-absence assertion before the closing brace of the existing test**

Find this block at the end of `e2e_consumer_init_fails_when_no_completed_producer`:

```rust
    assert!(!out.status.success(), "init should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no COMPLETED run"), "stderr: {stderr}");
    // Adversarial probe: also assert the pipeline name appears so we know the
    // failure came from the resolver (not a YAML parse error etc).
    assert!(
        stderr.contains("chain-producer"),
        "expected resolver error about chain-producer; stderr: {stderr}"
    );
}
```

Insert a new assertion block immediately before the closing brace:

```rust
    assert!(!out.status.success(), "init should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no COMPLETED run"), "stderr: {stderr}");
    // Adversarial probe: also assert the pipeline name appears so we know the
    // failure came from the resolver (not a YAML parse error etc).
    assert!(
        stderr.contains("chain-producer"),
        "expected resolver error about chain-producer; stderr: {stderr}"
    );

    // Atomicity probe (BELT-33): a failed init must NOT leave an orphan
    // consumer run directory behind. The only run that may exist is the
    // pre-seeded in_progress producer; any other directory indicates
    // init materialised state before resolver validation succeeded.
    let stray: Vec<_> = std::fs::read_dir(tmp.path().join(".belt/runs"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().into_string().unwrap()))
        .filter(|n| n != run)
        .collect();
    assert!(
        stray.is_empty(),
        "orphan consumer run left behind after failed init: {stray:?}"
    );
}
```

- [ ] **Step 2: Run the test to confirm it fails (documents the bug before the fix)**

Run: `cargo test -p belt-agent --test e2e_test e2e_consumer_init_fails_when_no_completed_producer -- --exact`

Expected: FAIL with panic message matching `orphan consumer run left behind after failed init: ["<some-uuid>"]`.

This confirms the pre-fix behaviour: `cmd_init` calls `init_with_branch` before the resolver runs, so a resolver failure leaves an orphan run on disk.

- [ ] **Step 3: Do NOT commit yet — the failing test is baseline for the fix in Task 4**

---

### Task 3: Add new regression E2E `e2e_init_succeeds_after_resolver_failure` (BELT-33, failing test)

**Files:**
- Modify: `crates/belt-agent/tests/e2e_test.rs` (append a new `#[test]` function at end-of-file)

- [ ] **Step 1: Append the new test at end-of-file**

Add this complete test function after the closing brace of `e2e_consumer_init_fails_when_no_completed_producer`:

```rust
/// Regression for BELT-33: a resolver failure during `init` must not
/// leave an orphan `.belt/runs/<id>/` behind. Running `init` twice —
/// once while the producer is missing (fail), once after the producer
/// completes (succeed) — must end up with exactly two run directories:
/// the completed producer plus the successful consumer. A pre-fix
/// cmd_init accumulates a half-initialised consumer run from the first
/// failed call, breaking this invariant.
#[test]
fn e2e_init_succeeds_after_resolver_failure() {
    let tmp = tempfile::tempdir().unwrap();

    // Copy the producer / consumer fixtures into the tempdir so pipeline
    // paths resolve relative to cwd.
    let producer_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("belt-core/tests/fixtures/chain-producer.yml");
    let consumer_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("belt-core/tests/fixtures/chain-consumer.yml");
    std::fs::copy(&producer_src, tmp.path().join("chain-producer.yml")).unwrap();
    std::fs::copy(&consumer_src, tmp.path().join("chain-consumer.yml")).unwrap();

    // 1. Consumer init with no producer — must fail.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["init", "chain-consumer.yml"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "first consumer init should fail (no producer)"
    );

    // 2. `.belt/runs/` must be empty — BELT-33 atomicity.
    let runs_dir = tmp.path().join(".belt/runs");
    let after_fail: Vec<_> = if runs_dir.is_dir() {
        std::fs::read_dir(&runs_dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.file_name().into_string().unwrap()))
            .collect()
    } else {
        Vec::new()
    };
    assert!(
        after_fail.is_empty(),
        "no orphan run should remain after failed init: {after_fail:?}"
    );

    // 3. Run the producer pipeline to completion.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["init", "chain-producer.yml"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "producer init should succeed, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let run_dirs: Vec<_> = std::fs::read_dir(tmp.path().join(".belt/runs"))
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(run_dirs.len(), 1, "exactly one producer run expected");
    let producer_run = run_dirs[0].file_name().into_string().unwrap();
    std::fs::write(
        tmp.path()
            .join(format!(".belt/runs/{producer_run}/notes/phase-review.md")),
        "body",
    )
    .unwrap();
    for _ in 0..2 {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
            .args(["verify"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
            .args(["step", "--confirm"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(out.status.success());
    }

    // 4. Consumer init — must now succeed because the producer is COMPLETED.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["init", "chain-consumer.yml"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "second consumer init should succeed, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 5. Exactly two runs — producer + consumer. No orphan from step 1.
    let final_runs: Vec<_> = std::fs::read_dir(tmp.path().join(".belt/runs"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().into_string().unwrap()))
        .collect();
    assert_eq!(
        final_runs.len(),
        2,
        "expected exactly producer + consumer, got: {final_runs:?}"
    );
}
```

- [ ] **Step 2: Run the test to confirm it fails (documents the bug)**

Run: `cargo test -p belt-agent --test e2e_test e2e_init_succeeds_after_resolver_failure -- --exact`

Expected: FAIL at step 2 assertion — `no orphan run should remain after failed init: [<uuid>]`.

- [ ] **Step 3: Do NOT commit yet — these failing tests are the TDD baseline for Task 4**

---

### Task 4: Restructure `cmd_init` — resolve External URIs before `init_with_branch` (BELT-33, implementation + commit)

**Files:**
- Modify: `crates/belt-agent/src/main.rs:185-230` (reorder the cmd_init body)

- [ ] **Step 1: Replace the current cmd_init body (L185-230) with the resolve-before-init structure**

Old (current L185-230):
```rust
    let args_map: HashMap<String, serde_json::Value> = args.into_iter().collect();
    // Detect the current git branch from the user's shell CWD so
    // workspace-scoped URI resolvers (Task 15) can filter runs by branch.
    // `current_branch` returns `None` outside a git repo, in detached HEAD,
    // or before the first commit — belt-core trusts the value verbatim.
    let branch = crate::git::current_branch(std::path::Path::new("."));
    let mut state = engine
        .init_with_branch(pipeline_path, &args_map, branch)
        .map_err(|e| miette::miette!("{e}"))?;

    // Resolve every External `belt://` URI in each phase's `consumes:` list
    // at init time so downstream steps read pinned filesystem paths straight
    // from RunState (no repeated resolver work, no late binding). Resolved
    // paths are stored absolute when the resolver can produce them; relative
    // fallbacks are preserved verbatim.
    let belt = belt_dir();
    let phases = expand_pipeline(pipeline_path).map_err(|e| miette::miette!("{e}"))?;
    let resolver = crate::resolver::Resolver {
        belt_dir: &belt,
        current_branch: state.branch.clone(),
    };
    let mut resolved_map: HashMap<String, String> = HashMap::new();
    for phase in &phases {
        for aref in &phase.consumes {
            if let belt_core::model::ArtifactRef::External { uri, .. } = aref {
                let path = resolver.resolve(uri).map_err(|e| miette::miette!("{e}"))?;
                resolved_map.insert(uri.to_string(), path.display().to_string());
            }
        }
    }

    // --inherits-from registers the inherited run under a synthetic
    // `belt://run/<id>/` key so skills can locate the parent run directory
    // without requiring an explicit External reference in the pipeline YAML.
    // The existence check above guarantees run_dir is a directory here.
    if let Some(run_id) = inherits_from {
        let run_dir = belt.join("runs").join(run_id);
        resolved_map.insert(
            format!("belt://run/{run_id}/"),
            run_dir.display().to_string(),
        );
    }

    engine
        .set_resolved_consumes(&mut state, resolved_map)
        .map_err(|e| miette::miette!("{e}"))?;
```

New:
```rust
    let args_map: HashMap<String, serde_json::Value> = args.into_iter().collect();
    // Detect the current git branch from the user's shell CWD so
    // workspace-scoped URI resolvers (Task 15) can filter runs by branch.
    // `current_branch` returns `None` outside a git repo, in detached HEAD,
    // or before the first commit — belt-core trusts the value verbatim.
    let branch = crate::git::current_branch(std::path::Path::new("."));

    // Resolve every External `belt://` URI in each phase's `consumes:` list
    // *before* creating the run directory. If any resolver call fails we
    // return early without having written anything to `.belt/runs/`, so a
    // failed init cannot leave an orphan half-initialised run behind
    // (BELT-33). The resolver reads `.belt/runs/` to find completed
    // producer runs, which works fine before the current run is
    // materialised.
    let belt = belt_dir();
    let phases = expand_pipeline(pipeline_path).map_err(|e| miette::miette!("{e}"))?;
    let resolver = crate::resolver::Resolver {
        belt_dir: &belt,
        current_branch: branch.clone(),
    };
    let mut resolved_map: HashMap<String, String> = HashMap::new();
    for phase in &phases {
        for aref in &phase.consumes {
            if let belt_core::model::ArtifactRef::External { uri, .. } = aref {
                let path = resolver.resolve(uri).map_err(|e| miette::miette!("{e}"))?;
                resolved_map.insert(uri.to_string(), path.display().to_string());
            }
        }
    }

    // --inherits-from registers the inherited run under a synthetic
    // `belt://run/<id>/` key so skills can locate the parent run directory
    // without requiring an explicit External reference in the pipeline YAML.
    // The existence check above guarantees run_dir is a directory here.
    if let Some(run_id) = inherits_from {
        let run_dir = belt.join("runs").join(run_id);
        resolved_map.insert(
            format!("belt://run/{run_id}/"),
            run_dir.display().to_string(),
        );
    }

    // Every URI has resolved successfully — now materialise the run
    // directory and persist the resolved mapping. This is the earliest
    // point at which `cmd_init` writes anything to `.belt/runs/`.
    let mut state = engine
        .init_with_branch(pipeline_path, &args_map, branch)
        .map_err(|e| miette::miette!("{e}"))?;
    engine
        .set_resolved_consumes(&mut state, resolved_map)
        .map_err(|e| miette::miette!("{e}"))?;
```

Key changes:
- `init_with_branch` moved from line ~191 to AFTER the resolver loop
- `Resolver::current_branch` now uses `branch.clone()` instead of `state.branch.clone()` (state does not yet exist at resolver time)
- Added a comment block above the resolver loop explaining the atomicity guarantee (BELT-33)

- [ ] **Step 2: Run both BELT-33 regression tests to confirm they now PASS**

Run: `cargo test -p belt-agent --test e2e_test e2e_consumer_init_fails_when_no_completed_producer e2e_init_succeeds_after_resolver_failure`

Expected: both tests PASS (2 passed).

- [ ] **Step 3: Run the full belt-agent test suite to confirm no regression**

Run: `cargo test -p belt-agent`

Expected: all tests PASS. Particularly verify:
- `e2e_chain_producer_to_consumer_happy_path` — producer → consumer happy path still works (the cmd_init reorder must not break the success path)
- `e2e_branch_isolation_for_latest_uri` — branch-scoped resolution still works (resolver now uses `branch.clone()` instead of `state.branch.clone()`; both refer to the same `Option<String>`)
- `init_resolves_external_uris_and_writes_resolved_consumes` — resolved_map still persists to state.json

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p belt-agent -- -D warnings`

Expected: no warnings.

- [ ] **Step 5: Run rustfmt**

Run: `cargo fmt --package belt-agent`

Expected: no output (already formatted) or minor reflow around the edited block.

- [ ] **Step 6: Commit BELT-33**

```bash
git add crates/belt-agent/src/main.rs crates/belt-agent/tests/e2e_test.rs
git -c commit.gpgsign=false commit -m "fix(belt-agent): resolve External URIs before init (BELT-33)

cmd_init previously called engine.init_with_branch first and only
then walked phase.consumes calling resolver.resolve on each
External URI. A resolver failure left an orphan .belt/runs/<id>/
on disk, breaking the invariant that failed inits are idempotent.

Reorder cmd_init so:
1. --inherits-from existence check (unchanged)
2. expand_pipeline + resolver loop
3. --inherits-from synthetic key insertion (unchanged)
4. init_with_branch
5. set_resolved_consumes

Any resolver error now returns before anything is written to
.belt/runs/, so retrying init after a failed run leaves no stray
run directories.

Add two regression tests:
- Extend e2e_consumer_init_fails_when_no_completed_producer with
  an orphan-absence assertion.
- New e2e_init_succeeds_after_resolver_failure: fail → seed
  producer → succeed, with .belt/runs/ checked at each step."
```

---

### Task 5: Add unit test `resolve_latest_errors_on_corrupt_state_json` (BELT-35)

**Files:**
- Modify: `crates/belt-agent/src/resolver.rs` (append a new `#[test]` inside the existing `#[cfg(test)] mod tests { ... }` block)

- [ ] **Step 1: Append the test at the end of `mod tests`, before the final `}`**

Insert immediately after the last existing test (`resolve_workspace_latest_errors_on_non_git`) and before the closing `}` of `mod tests`:

```rust
    /// BELT-35 adversarial probe: a truncated state.json surfaces a loud
    /// `StateParse` error rather than silently selecting a different
    /// candidate or coercing to empty. Documents current fail-loud
    /// behaviour for corrupt JSON.
    #[test]
    fn resolve_latest_errors_on_corrupt_state_json() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let run_dir = belt_dir.join("runs").join("01947cor");
        fs::create_dir_all(run_dir.join("notes")).unwrap();
        fs::write(run_dir.join("notes").join("phase-review.md"), "x").unwrap();
        // Truncated mid-field — not valid JSON.
        fs::write(run_dir.join("state.json"), r#"{"run_id": "trun"#).unwrap();

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
        };
        let uri = BeltUri::Latest {
            pipeline: "feature-dev".into(),
            path: "notes/phase-review.md".into(),
        };
        assert!(matches!(
            r.resolve(&uri),
            Err(ResolveError::StateParse(_))
        ));
    }
```

- [ ] **Step 2: Run the test to confirm it passes**

Run: `cargo test -p belt-agent resolver::tests::resolve_latest_errors_on_corrupt_state_json`

Expected: PASS. (The current resolver implementation is already fail-loud on corrupt JSON — this test documents that behaviour.)

- [ ] **Step 3: Do NOT commit yet — bundled with Tasks 6-8 into a single BELT-35 commit**

---

### Task 6: Add unit test `resolve_latest_skips_state_json_without_pipeline_field` (BELT-35)

**Files:**
- Modify: `crates/belt-agent/src/resolver.rs` (append inside `mod tests`, after Task 5's test)

- [ ] **Step 1: Append the test immediately after `resolve_latest_errors_on_corrupt_state_json`**

```rust
    /// BELT-35 adversarial probe: state.json with a missing `pipeline`
    /// field is *silently skipped* by the candidate loop — it falls back
    /// to an empty pipeline name via `unwrap_or("")`, mismatches the
    /// requested pipeline, and is dropped. With no other candidate, the
    /// resolver returns `NoCompletedRun`. Documents the current
    /// silent-skip behaviour for schema-missing state.json. A loud
    /// variant is explicitly out of scope (see Non-Goals in the spec).
    #[test]
    fn resolve_latest_skips_state_json_without_pipeline_field() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let run_dir = belt_dir.join("runs").join("01947mis");
        fs::create_dir_all(run_dir.join("notes")).unwrap();
        fs::write(run_dir.join("notes").join("phase-review.md"), "x").unwrap();
        // Valid JSON, but `pipeline` field is missing.
        fs::write(
            run_dir.join("state.json"),
            r#"{"run_id": "01947mis", "status": "completed", "branch": null}"#,
        )
        .unwrap();

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
        };
        let uri = BeltUri::Latest {
            pipeline: "feature-dev".into(),
            path: "notes/phase-review.md".into(),
        };
        assert!(matches!(
            r.resolve(&uri),
            Err(ResolveError::NoCompletedRun { .. })
        ));
    }
```

- [ ] **Step 2: Run the test to confirm it passes**

Run: `cargo test -p belt-agent resolver::tests::resolve_latest_skips_state_json_without_pipeline_field`

Expected: PASS.

- [ ] **Step 3: Do NOT commit yet**

---

### Task 7: Add unit test `resolve_latest_skips_state_json_that_is_a_directory` (BELT-35)

**Files:**
- Modify: `crates/belt-agent/src/resolver.rs` (append inside `mod tests`, after Task 6's test)

- [ ] **Step 1: Append the test immediately after `resolve_latest_skips_state_json_without_pipeline_field`**

```rust
    /// BELT-35 adversarial probe: when the `state.json` path is a
    /// directory (not a file), `resolver.rs:80`'s
    /// `if !state_path.is_file() { continue; }` silently skips the run.
    /// With no other candidate, the resolver returns `NoCompletedRun`.
    /// Documents that we do NOT surface an `Io` error in this case —
    /// the `read_to_string` call is never reached.
    #[test]
    fn resolve_latest_skips_state_json_that_is_a_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let run_dir = belt_dir.join("runs").join("01947dir");
        // state.json is a directory, not a file.
        fs::create_dir_all(run_dir.join("state.json")).unwrap();
        fs::create_dir_all(run_dir.join("notes")).unwrap();
        fs::write(run_dir.join("notes").join("phase-review.md"), "x").unwrap();

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
        };
        let uri = BeltUri::Latest {
            pipeline: "feature-dev".into(),
            path: "notes/phase-review.md".into(),
        };
        assert!(matches!(
            r.resolve(&uri),
            Err(ResolveError::NoCompletedRun { .. })
        ));
    }
```

- [ ] **Step 2: Run the test to confirm it passes**

Run: `cargo test -p belt-agent resolver::tests::resolve_latest_skips_state_json_that_is_a_directory`

Expected: PASS.

- [ ] **Step 3: Do NOT commit yet**

---

### Task 8: Add E2E test `e2e_init_fails_when_producer_state_json_is_corrupt` + commit (BELT-35)

**Files:**
- Modify: `crates/belt-agent/tests/e2e_test.rs` (append a new `#[test]` function at end-of-file)

- [ ] **Step 1: Append the test at the end of `tests/e2e_test.rs`**

```rust
/// BELT-35 E2E probe: when the only producer has a corrupt state.json,
/// a consumer init fails loudly (non-zero exit, stderr mentions parse
/// failure) AND leaves no orphan consumer run behind. The orphan
/// assertion cross-verifies BELT-33 atomicity under a different
/// resolver-error path (StateParse, not NoCompletedRun).
#[test]
fn e2e_init_fails_when_producer_state_json_is_corrupt() {
    let tmp = tempfile::tempdir().unwrap();

    // Producer run directory with a truncated state.json.
    let run = "01947cor-0000-7000-8000-000000000000";
    let dir = tmp.path().join(".belt/runs").join(run);
    std::fs::create_dir_all(dir.join("notes")).unwrap();
    std::fs::write(dir.join("notes/phase-review.md"), "x").unwrap();
    std::fs::write(dir.join("state.json"), r#"{"run_id": "trun"#).unwrap();

    // Consumer fixture references chain-producer via belt://latest/...
    let consumer_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("belt-core/tests/fixtures/chain-consumer.yml");
    std::fs::copy(&consumer_src, tmp.path().join("chain-consumer.yml")).unwrap();

    let out = std::process::Command::new(env!("CARGO_BIN_EXE_belt-agent"))
        .args(["init", "chain-consumer.yml"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(!out.status.success(), "init should fail on corrupt state.json");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("state.json parse error")
            || stderr.contains("json")
            || stderr.contains("parse"),
        "stderr should mention state.json parse failure: {stderr}"
    );

    // Atomicity cross-check (BELT-33): the failed init must not leave an
    // orphan consumer run. Only the pre-seeded corrupt producer should
    // remain.
    let survivors: Vec<_> = std::fs::read_dir(tmp.path().join(".belt/runs"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().into_string().unwrap()))
        .filter(|n| n != run)
        .collect();
    assert!(
        survivors.is_empty(),
        "orphan consumer run left behind: {survivors:?}"
    );
}
```

- [ ] **Step 2: Run the new E2E test**

Run: `cargo test -p belt-agent --test e2e_test e2e_init_fails_when_producer_state_json_is_corrupt -- --exact`

Expected: PASS. (Task 4's cmd_init reorder guarantees atomicity; the resolver's fail-loud on parse errors guarantees the stderr contains `parse` / `json`.)

- [ ] **Step 3: Run the full belt-agent test suite**

Run: `cargo test -p belt-agent`

Expected: all tests PASS. Sanity-check count: Task 2 extended an existing test, Task 3/8 added 1 E2E each (+2), Tasks 5/6/7 added 1 unit test each (+3). Total new or extended: 6.

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p belt-agent -- -D warnings`

Expected: no warnings. (The `resolver::tests` module has `#[allow(clippy::unwrap_used, clippy::panic)]` on the mod, so the new tests inherit the relaxation.)

- [ ] **Step 5: Run rustfmt**

Run: `cargo fmt --package belt-agent`

Expected: no output (already formatted) or minor reflow around the new tests.

- [ ] **Step 6: Commit BELT-35**

```bash
git add crates/belt-agent/src/resolver.rs crates/belt-agent/tests/e2e_test.rs
git -c commit.gpgsign=false commit -m "test(belt-agent): adversarial probes for corrupt state.json (BELT-35)

Document the resolver's handling of three adversarial state.json
conditions that the narrative artifact spec's Testing Strategy
listed but left untested:

Unit (crates/belt-agent/src/resolver.rs):
- resolve_latest_errors_on_corrupt_state_json: truncated JSON
  surfaces ResolveError::StateParse (fail-loud).
- resolve_latest_skips_state_json_without_pipeline_field: missing
  'pipeline' field falls back to empty string, candidate is
  filtered, resolver returns NoCompletedRun (silent skip).
- resolve_latest_skips_state_json_that_is_a_directory: non-file
  path is short-circuited by is_file() check, candidate is
  skipped, resolver returns NoCompletedRun.

E2E (crates/belt-agent/tests/e2e_test.rs):
- e2e_init_fails_when_producer_state_json_is_corrupt: cmd_init
  on a corrupt producer fails non-zero with a parse-related
  stderr, and the atomicity probe from BELT-33 still holds
  (no orphan consumer run remains).

Silent-skip behaviour on schema-missing fields is intentionally
preserved (pain-driven-first-class); turning those into loud
errors would be a SchemaVersion / public API change and is a
Non-Goal of this spec."
```

---

## Verification Summary

After all 8 tasks:

- 3 commits on branch `2026-04-15-narrative-followup`:
  1. `docs(claude-md): update belt-core module list to 10 (BELT-34)`
  2. `fix(belt-agent): resolve External URIs before init (BELT-33)`
  3. `test(belt-agent): adversarial probes for corrupt state.json (BELT-35)`
- New tests: 3 unit (`resolver.rs`) + 2 E2E (`e2e_test.rs`)
- Extended tests: 1 (`e2e_consumer_init_fails_when_no_completed_producer`)
- Total test delta: **6** (3 unit + 3 E2E)

Final verification commands:

```bash
cargo test -p belt-agent                                  # all green
cargo test -p belt-core                                   # still green (no belt-core changes)
cargo clippy -p belt-agent -- -D warnings                 # no warnings
cargo clippy -p belt-core -- -D warnings                  # no warnings (docs-only change doesn't touch it)
cargo fmt --package belt-agent --check                    # no diff
```

## Self-Review Notes

Reviewed plan against `docs/specs/2026-04-15-narrative-followup-design.md`:

- **Spec coverage**: Every Proposal (1-3) maps to at least one task. Testing Strategy row counts (3 unit + 2 e2e + 1 extended = 6) match Task 5-8 + Task 2/3 totals
- **Placeholder scan**: No TBD / TODO / "similar to Task N" patterns. Every code block is concrete
- **Type consistency**: `Resolver { belt_dir, current_branch }` shape is identical in Task 4, 5, 6, 7; `ResolveError` variant names (`StateParse`, `NoCompletedRun`) match `resolver.rs:7-26` exactly; `BeltUri::Latest { pipeline, path }` matches `belt_core::uri` current definition
- **Execution order**: BELT-34 (docs) → BELT-33 (behavior + failing tests first) → BELT-35 (tests stacked on post-fix cmd_init) — risk-ascending
