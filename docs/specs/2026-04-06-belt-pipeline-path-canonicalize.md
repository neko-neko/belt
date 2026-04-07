# pipeline_file Path Canonicalization

**Linear**: BELT-23 (to be created)
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)
**Status**: Draft
**Date**: 2026-04-06

## Summary

Canonicalize the `pipeline_file` path stored in `state.json` so that `next`/`verify`/`step` commands work regardless of the caller's working directory.

## Background

`Engine::init()` stores `pipeline_path.display().to_string()` directly into `RunState.pipeline_file` (engine.rs:52). When a caller passes a relative path (e.g., `./pipeline.yml`), the relative string is persisted in `state.json`.

Subsequent commands (`next`, `verify`, `step`) reconstruct the path via `Path::new(&state.pipeline_file)`. If the working directory differs from the one used during `init`, the relative path resolves to the wrong location or fails entirely.

This was documented as a Known Risk in CLAUDE.md and became more visible after BELT-22 introduced `--config`, which adds an indirect resolution layer (`belt.toml` → relative `pipeline` field → `resolve_pipeline_path()` → potentially relative `PathBuf`).

## Design

### Approach: Canonicalize in Engine::init()

Apply `std::fs::canonicalize()` to `pipeline_path` inside `Engine::init()` before storing it in `RunState.pipeline_file`. This is the single point where `pipeline_file` is written to state.

```rust
// engine.rs init() — line 52
// Before
pipeline_file: pipeline_path.display().to_string(),

// After
pipeline_file: std::fs::canonicalize(pipeline_path)
    .map_err(BeltError::Io)?
    .display()
    .to_string(),
```

### Why canonicalize() over path::absolute()

- `std::fs::canonicalize()`: resolves symlinks, requires file existence, returns true absolute path
- `std::path::absolute()` (Rust 1.79+): pure path manipulation, no symlink resolution, no existence check

`canonicalize()` is the right choice because:
1. `parse_pipeline(pipeline_path)` runs before this line, guaranteeing the file exists
2. Symlink resolution is desirable — the stored path should point to the real file
3. Belt targets Linux; the Windows `\\?\` prefix is not a concern (Non-Goal)

### What Changes

| File | Change |
|------|--------|
| `crates/belt-core/src/engine.rs` | `init()`: canonicalize `pipeline_path` before storing in state |
| `crates/belt-core/tests/engine_test.rs` | Update existing assertions, add 6 new test cases |
| `CLAUDE.md` | Remove `pipeline_file` entry from Known Risks |

### What Does NOT Change

| File | Reason |
|------|--------|
| `crates/belt-core/src/config.rs` | `resolve_pipeline_path()` output feeds into `init()`, which canonicalizes |
| `crates/belt-core/src/model.rs` | `RunState.pipeline_file` remains `String` — no type change needed |
| `crates/belt-agent/src/main.rs` | `next`/`verify`/`step` use `state.pipeline_file` which is now absolute |

### Error Handling

| Case | Behavior |
|------|----------|
| File does not exist | Unreachable — `parse_pipeline()` fails first |
| Permission denied | `BeltError::Io` propagates naturally |
| Symlink target | Resolved to real path (intended) |
| Windows `\\?\` prefix | Out of scope (Non-Goal) |

### Backward Compatibility

None. MVP phase — no production state.json files to migrate. Existing runs can be re-initialized.

## Test Plan

### Existing Test Updates

| Test | Change |
|------|--------|
| `engine_load_state_round_trip` | Assert `pipeline_file` is absolute path |

### New Test Cases

| # | Test Name | Verification |
|---|-----------|-------------|
| 1 | `init_stores_absolute_path` | Relative path init → `state.pipeline_file.starts_with("/")` (is absolute) |
| 2 | `init_with_absolute_path_preserved` | Absolute path init → canonicalized path stored unchanged |
| 3 | `init_canonicalizes_dot_segments` | `./dir/../dir/pipeline.yml` init → normalized path without `.`/`..` segments |
| 4 | `init_resolves_symlink` | Symlink to pipeline → real file's absolute path stored |
| 5 | `state_pipeline_file_usable_after_reload` | init → save → load → `expand_pipeline(Path::new(&state.pipeline_file))` succeeds |
| 6 | `init_canonicalize_error_on_nonexistent` | Non-existent path → `parse_pipeline` error returned (not canonicalize error) |

### Test Implementation Notes

- Tests 1–5 use `TempDir` which provides absolute paths; relative paths are constructed by joining with `./` or `../` segments
- Test 4 creates a symlink via `std::os::unix::fs::symlink` in the temp directory
- Test 6 confirms error ordering: `parse_pipeline` is the first gate, not `canonicalize`
