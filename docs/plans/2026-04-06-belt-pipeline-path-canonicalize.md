# BELT-23: pipeline_file Path Canonicalization — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Canonicalize pipeline_file in Engine::init() so state.json stores an absolute path, making next/verify/step work regardless of caller's working directory.

**Architecture:** Single change in `Engine::init()` — apply `std::fs::canonicalize()` to `pipeline_path` before storing in `RunState.pipeline_file`. No changes to other Engine methods, config.rs, or main.rs. Remove Known Risk from CLAUDE.md.

**Tech Stack:** Rust std::fs::canonicalize, tempfile (tests), std::os::unix::fs::symlink (symlink test)

**Spec:** `docs/specs/2026-04-06-belt-pipeline-path-canonicalize.md`

---

### Task 1: Canonicalize pipeline_path in Engine::init()

**Files:**
- Modify: `crates/belt-core/src/engine.rs:52`
- Test: `crates/belt-core/tests/engine_test.rs`

- [ ] **Step 1: Write the failing test — init stores absolute path**

Add this test at the end of `crates/belt-core/tests/engine_test.rs`:

```rust
// ===========================================================================
// BELT-23: pipeline_file path canonicalization
// ===========================================================================

// ---------------------------------------------------------------------------
// Test: init stores absolute path in state.pipeline_file
// ---------------------------------------------------------------------------
#[test]
fn init_stores_absolute_path() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    let stored = std::path::Path::new(&state.pipeline_file);
    assert!(
        stored.is_absolute(),
        "pipeline_file should be absolute, got: {}",
        state.pipeline_file
    );
}
```

- [ ] **Step 2: Run test to verify it passes (TempDir already provides absolute paths)**

Run: `cargo test -p belt-core --test engine_test init_stores_absolute_path -- --exact`

Expected: PASS (TempDir paths are already absolute, so the current code happens to pass). This test establishes the contract — the next tests will exercise relative path inputs.

- [ ] **Step 3: Write the failing test — init canonicalizes dot segments**

Add this test after the previous one:

```rust
// ---------------------------------------------------------------------------
// Test: init canonicalizes dot segments (../ and ./)
// ---------------------------------------------------------------------------
#[test]
fn init_canonicalizes_dot_segments() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);

    // Construct a path with redundant dot segments:
    // /tmp/xxx/pipeline.yml -> /tmp/xxx/./subdir/../pipeline.yml
    let subdir = dir.path().join("subdir");
    std::fs::create_dir(&subdir).expect("create subdir");
    let dotty_path = subdir.join("..").join("pipeline.yml");

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let state = engine.init(&dotty_path, &HashMap::new()).expect("init");

    assert!(
        !state.pipeline_file.contains(".."),
        "pipeline_file should not contain '..', got: {}",
        state.pipeline_file
    );
    assert!(
        !state.pipeline_file.contains("/./"),
        "pipeline_file should not contain '/./', got: {}",
        state.pipeline_file
    );
    assert!(
        std::path::Path::new(&state.pipeline_file).is_absolute(),
        "pipeline_file should be absolute, got: {}",
        state.pipeline_file
    );
}
```

- [ ] **Step 4: Run test to verify it fails**

Run: `cargo test -p belt-core --test engine_test init_canonicalizes_dot_segments -- --exact`

Expected: FAIL — current code stores the raw `display()` string which contains `..`.

- [ ] **Step 5: Implement canonicalize in Engine::init()**

In `crates/belt-core/src/engine.rs`, replace line 52:

```rust
// Before
pipeline_file: pipeline_path.display().to_string(),

// After
pipeline_file: std::fs::canonicalize(pipeline_path)?.display().to_string(),
```

The `?` operator works because `BeltError` has `#[from] std::io::Error` on the `Io` variant.

- [ ] **Step 6: Run both tests to verify they pass**

Run: `cargo test -p belt-core --test engine_test init_stores_absolute_path init_canonicalizes_dot_segments`

Expected: Both PASS.

- [ ] **Step 7: Run all existing tests to verify no regressions**

Run: `cargo test -p belt-core`

Expected: All tests PASS. Existing tests use `TempDir` absolute paths, so canonicalize is a no-op for them.

- [ ] **Step 8: Commit**

```bash
git add crates/belt-core/src/engine.rs crates/belt-core/tests/engine_test.rs
git commit -m "feat(belt-core): canonicalize pipeline_path in Engine::init() (BELT-23)

state.json now stores an absolute path for pipeline_file,
fixing cross-cwd breakage in next/verify/step commands."
```

---

### Task 2: Additional test cases — symlink, reload, error ordering

**Files:**
- Test: `crates/belt-core/tests/engine_test.rs`

- [ ] **Step 1: Write test — init with absolute path preserved**

```rust
// ---------------------------------------------------------------------------
// Test: init with absolute path stores same canonical path
// ---------------------------------------------------------------------------
#[test]
fn init_with_absolute_path_preserved() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);

    // pipeline_path from TempDir is already absolute
    assert!(pipeline_path.is_absolute());

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Canonicalize the input for comparison (resolves macOS /private/var symlinks)
    let expected = std::fs::canonicalize(&pipeline_path)
        .expect("canonicalize")
        .display()
        .to_string();
    assert_eq!(state.pipeline_file, expected);
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p belt-core --test engine_test init_with_absolute_path_preserved -- --exact`

Expected: PASS.

- [ ] **Step 3: Write test — init resolves symlink**

```rust
// ---------------------------------------------------------------------------
// Test: init resolves symlink to real path
// ---------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn init_resolves_symlink() {
    let dir = TempDir::new().expect("tempdir");
    let real_path = two_phase_pipeline(&dir);

    // Create symlink: linked.yml -> pipeline.yml
    let link_path = dir.path().join("linked.yml");
    std::os::unix::fs::symlink(&real_path, &link_path).expect("create symlink");

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let state = engine.init(&link_path, &HashMap::new()).expect("init");

    // Should resolve to the real file, not the symlink
    let canonical_real = std::fs::canonicalize(&real_path)
        .expect("canonicalize")
        .display()
        .to_string();
    assert_eq!(
        state.pipeline_file, canonical_real,
        "symlink should resolve to real path"
    );
    assert!(
        !state.pipeline_file.contains("linked.yml"),
        "should not contain symlink name"
    );
}
```

- [ ] **Step 4: Run test**

Run: `cargo test -p belt-core --test engine_test init_resolves_symlink -- --exact`

Expected: PASS.

- [ ] **Step 5: Write test — state pipeline_file usable after reload**

```rust
// ---------------------------------------------------------------------------
// Test: state.pipeline_file is usable after save/load round-trip
// ---------------------------------------------------------------------------
#[test]
fn state_pipeline_file_usable_after_reload() {
    let dir = TempDir::new().expect("tempdir");
    let pipeline_path = two_phase_pipeline(&dir);

    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let state = engine.init(&pipeline_path, &HashMap::new()).expect("init");

    // Reload state from disk
    let loaded = engine.load_state(&state.run_id).expect("load");

    // Use the stored pipeline_file to call next_phase_info (proves path resolves)
    let restored_path = std::path::Path::new(&loaded.pipeline_file);
    let phase = engine
        .next_phase_info(&loaded, restored_path)
        .expect("next_phase_info should work with stored absolute path");
    assert_eq!(phase.id, "build");
}
```

- [ ] **Step 6: Run test**

Run: `cargo test -p belt-core --test engine_test state_pipeline_file_usable_after_reload -- --exact`

Expected: PASS.

- [ ] **Step 7: Write test — nonexistent path returns parse error, not canonicalize error**

```rust
// ---------------------------------------------------------------------------
// Test: nonexistent path returns parse_pipeline error, not canonicalize error
// ---------------------------------------------------------------------------
#[test]
fn init_nonexistent_path_returns_parse_error() {
    let dir = TempDir::new().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);

    let bogus_path = dir.path().join("does_not_exist.yml");
    let result = engine.init(&bogus_path, &HashMap::new());

    assert!(result.is_err());
    // Should be a FileNotFound or YamlParse from parse_pipeline,
    // NOT an Io error from canonicalize
    let err = result.unwrap_err();
    assert!(
        !matches!(err, BeltError::Io(_)),
        "error should come from parse_pipeline, not canonicalize: {err}"
    );
}
```

- [ ] **Step 8: Run test**

Run: `cargo test -p belt-core --test engine_test init_nonexistent_path_returns_parse_error -- --exact`

Expected: PASS — `parse_pipeline()` on line 36 returns `FileNotFound` before canonicalize on line 52 is reached.

- [ ] **Step 9: Run all belt-core tests**

Run: `cargo test -p belt-core`

Expected: All PASS.

- [ ] **Step 10: Commit**

```bash
git add crates/belt-core/tests/engine_test.rs
git commit -m "test(belt-core): add path canonicalization test cases (BELT-23)

- absolute path preserved
- symlink resolved to real path
- state.pipeline_file usable after reload
- nonexistent path returns parse error (not canonicalize error)"
```

---

### Task 3: Update CLAUDE.md Known Risks

**Files:**
- Modify: `CLAUDE.md:218`

- [ ] **Step 1: Remove the Known Risk entry**

In `CLAUDE.md`, delete line 218:

```
- **`belt-agent` の pipeline_file 相対パス**: init 時の相対パスが state.json に保存される。step/verify 時の cwd が異なると壊れる可能性がある。将来 canonicalize で解決予定
```

- [ ] **Step 2: Run linter and formatter on changed files**

Run: `cargo fmt --package belt-core && cargo clippy --package belt-core -- -D warnings`

Expected: Clean output, no warnings.

- [ ] **Step 3: Run full test suite**

Run: `cargo test -p belt-core`

Expected: All tests PASS.

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: remove pipeline_file Known Risk, resolved by BELT-23"
```
