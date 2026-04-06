# belt.toml Config File and Pipeline Co-location

**Linear**: [BELT-22](https://linear.app/neko-neko/issue/BELT-22)
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)
**Status**: Draft
**Date**: 2026-04-06

## Summary

Introduce a `belt.toml` configuration file so that pipeline files can live alongside the skills that own them, rather than in a centralized `pipelines/` directory. Both `belt` and `belt-agent` CLIs gain a `--config` global argument to resolve pipeline paths from the config file.

## Background

Pipeline YAML files currently live in a top-level `pipelines/` directory. This is a belt-repository convenience, not a design principle. Pipelines are owned by skills; they should co-locate with the skill that uses them.

The `belt.toml` pattern follows the standard configuration file convention (Cargo.toml, package.json, .claude/settings.json). It provides a stable entry point for belt to discover a skill's pipeline and, in future versions, additional runtime settings.

## Design

### belt.toml Schema

```toml
pipeline = "pipeline.yml"
```

- `pipeline` (string, required): Path to the pipeline YAML file, resolved relative to the `belt.toml` file's parent directory.

The schema is intentionally minimal. Future versions may add fields such as `belt_dir`, `args` defaults, or path customization. The TOML format was chosen for Rust ecosystem alignment.

### belt-core: config Module

A new `config` module in belt-core provides shared parsing and path resolution logic.

```rust
// crates/belt-core/src/config.rs

/// Parsed representation of a belt.toml file.
#[derive(Debug, Deserialize)]
pub struct BeltConfig {
    /// Path to the pipeline YAML file (relative to belt.toml location).
    pub pipeline: String,
}

/// Parse a belt.toml file at the given path.
pub fn parse_config(path: &Path) -> Result<BeltConfig, BeltError>;

/// Resolve the pipeline path from a config file's location.
/// Returns config_path.parent() joined with config.pipeline.
pub fn resolve_pipeline_path(config_path: &Path, config: &BeltConfig) -> PathBuf;
```

- `parse_config()` reads and deserializes the TOML file. Returns `BeltError::FileNotFound` or `BeltError::ConfigParse` on failure.
- `resolve_pipeline_path()` joins the config file's parent directory with the `pipeline` field. It does not verify the resolved path exists; that responsibility stays with `parse_pipeline()`.

### BeltError Extension

```rust
// Added to existing BeltError enum
ConfigParse { path: String, detail: String }
```

`FileNotFound` (existing) is reused for missing config files.

### CLI: --config Global Argument

Both binaries add `--config <path>` as a top-level (global) argument on the `Cli` struct.

```
# belt-agent
belt-agent [--config <path>] <command> [options]

# belt
belt [--config <path>] <command> [options]
```

When `--config` is provided:
1. `parse_config(config_path)` reads the config file
2. `resolve_pipeline_path(config_path, &config)` produces the pipeline path
3. The resolved path is passed to `engine.init()` or `lint_pipeline()`

### Exclusivity with Positional Arguments

`--config` and the positional `<file>` argument on `init` / `lint` are mutually exclusive. Both provided = error.

```
# Config mode
belt-agent --config examples/skills/linear-refresh/belt.toml init --arg force=true

# Direct mode (unchanged)
belt-agent init pipeline.yml --arg force=true

# Error: conflicting
belt-agent --config belt.toml init pipeline.yml
```

This is enforced via clap's `ArgGroup` or conditional validation in the subcommand handler.

### Path Resolution Rules

| Mode | Pipeline path resolved from | `uses:` resolved from |
|------|---------------------------|----------------------|
| `--config belt.toml` | config dir + `pipeline` field | pipeline file's parent dir |
| Direct `<file>` | argument as-is | pipeline file's parent dir |

Both modes converge: once the pipeline file path is determined, all downstream resolution (uses:, sub-pipelines) follows existing rules in `expander.rs`. No changes to belt-core's engine, expander, or parser are required.

### Dependency Addition

```toml
# Cargo.toml [workspace.dependencies]
toml = "1.1"
```

`toml` is 1.x, so caret versioning per the project's dependency policy. Added to belt-core's `Cargo.toml` dependencies.

## File Relocation

### Moves

| From | To |
|------|-----|
| `pipelines/linear-refresh.yml` | `examples/skills/linear-refresh/pipeline.yml` |
| `pipelines/linear-cleanup.yml` | `examples/skills/linear-refresh/linear-cleanup.yml` |
| `pipelines/linear-add.yml` | `examples/skills/linear-refresh/linear-add.yml` |
| `pipelines/smoke-test.yml` | `examples/skills/smoke-test/pipeline.yml` |
| `skills/linear-refresh/SKILL.md` | `examples/skills/linear-refresh/SKILL.md` |
| `skills/linear-refresh/references/` | `examples/skills/linear-refresh/references/` |
| `skills/smoke-test/SKILL.md` | `examples/skills/smoke-test/SKILL.md` |
| `skills/smoke-test/references/` | `examples/skills/smoke-test/references/` |

### New Files

| File | Content |
|------|---------|
| `examples/skills/linear-refresh/belt.toml` | `pipeline = "pipeline.yml"` |
| `examples/skills/smoke-test/belt.toml` | `pipeline = "pipeline.yml"` |

### Deletions

| Path | Reason |
|------|--------|
| `pipelines/` directory | All files moved |
| `skills/linear-refresh/` | Moved to examples/skills/ |
| `skills/smoke-test/` | Moved to examples/skills/ |

### Unchanged

- `skills/belt-agent/` — protocol skill, not an example, no pipeline
- `examples/feature-dev/`, `examples/gates/`, `examples/pipelines/` — out of scope

## Resulting Directory Structure

```
examples/skills/
├── linear-refresh/
│   ├── belt.toml
│   ├── pipeline.yml
│   ├── linear-cleanup.yml
│   ├── linear-add.yml
│   ├── SKILL.md
│   └── references/
└── smoke-test/
    ├── belt.toml
    ├── pipeline.yml
    ├── SKILL.md
    └── references/

skills/
└── belt-agent/
    └── SKILL.md
```

## Testing Strategy

### belt-core config module (unit tests)

- Parse valid belt.toml with `pipeline` field
- Error on missing file (`BeltError::FileNotFound`)
- Error on invalid TOML (`BeltError::ConfigParse`)
- Error on missing `pipeline` field (`BeltError::ConfigParse`)
- Path resolution: config dir + pipeline field produces correct absolute path

### belt-agent CLI (integration tests)

- `--config` resolves pipeline and runs init successfully
- `--config` + positional `<file>` = error
- `--config` with nonexistent file = error
- `--config` with invalid TOML = error

### belt CLI (integration tests)

- `--config` resolves pipeline and runs lint successfully
- `--config` + positional `<file>` = error

### Existing tests

No changes required. belt-core engine, expander, parser, gate, and lint modules are unaffected.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Config format | TOML | Rust ecosystem alignment (Cargo.toml pattern) |
| File name | `belt.toml` | Explicit, not hidden. Matches Cargo.toml convention |
| Initial scope | `pipeline` field only | YAGNI. Extend when needed |
| Config parsing location | belt-core | Avoid duplication across binaries; natural extension of parser module |
| `--config` placement | Global argument | Future config fields may be relevant to all subcommands |
| Positional relationship | Mutually exclusive | Avoids precedence rule complexity |
| Path resolution base | Config file's parent directory | Consistent with direct-path mode (pipeline file's parent dir for uses:) |

## Non-Goals

- Config file auto-discovery (searching parent directories)
- Config file generation command (`belt init-config`)
- Remote pipeline resolution
- CLAUDE.md / README.md updates (handled by doc-check separately)
