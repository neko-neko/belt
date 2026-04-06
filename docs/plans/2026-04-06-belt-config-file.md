# BELT-22: belt.toml Config File Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce `belt.toml` config file support so pipelines co-locate with skills, and both CLIs resolve pipelines via `--config`.

**Architecture:** New `config` module in belt-core handles TOML parsing and path resolution. Both binaries add `--config` as a global argument, mutually exclusive with positional file arguments. Example pipelines and skills move from `pipelines/` and `skills/` to `examples/skills/`.

**Tech Stack:** Rust, `toml` 1.1 crate, clap derive API, existing belt-core error infrastructure.

---

### Task 1: Add `toml` workspace dependency

**Files:**
- Modify: `Cargo.toml:17-28` (workspace dependencies section)
- Modify: `crates/belt-core/Cargo.toml:10-18` (dependencies section)

- [ ] **Step 1: Add `toml` to workspace dependencies**

In `Cargo.toml`, add to `[workspace.dependencies]` after the serialization section:

```toml
toml = "1.1"
```

- [ ] **Step 2: Add `toml` to belt-core dependencies**

In `crates/belt-core/Cargo.toml`, add to `[dependencies]`:

```toml
toml = { workspace = true }
```

- [ ] **Step 3: Verify the dependency resolves**

Run: `cargo check -p belt-core`
Expected: compiles without error

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock crates/belt-core/Cargo.toml
git commit -m "chore(belt-core): add toml workspace dependency for config parsing (BELT-22)"
```

---

### Task 2: Add `BeltError::ConfigParse` variant

**Files:**
- Modify: `crates/belt-core/src/error.rs:4-47`

- [ ] **Step 1: Add the ConfigParse variant to BeltError**

In `crates/belt-core/src/error.rs`, add after the `FileNotFound` variant:

```rust
#[error("config parse error in {path}: {detail}")]
#[diagnostic(code(belt::config_parse))]
ConfigParse { path: String, detail: String },
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p belt-core`
Expected: compiles without error

- [ ] **Step 3: Commit**

```bash
git add crates/belt-core/src/error.rs
git commit -m "feat(belt-core): add BeltError::ConfigParse variant (BELT-22)"
```

---

### Task 3: Implement belt-core config module (TDD)

**Files:**
- Create: `crates/belt-core/src/config.rs`
- Modify: `crates/belt-core/src/lib.rs:1-7`
- Create: `crates/belt-core/tests/config_test.rs`

- [ ] **Step 1: Write failing tests for config module**

Create `crates/belt-core/tests/config_test.rs`:

```rust
use belt_core::config::{parse_config, resolve_pipeline_path};
use belt_core::error::BeltError;
use std::io::Write;
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn parse_valid_config() {
    let mut f = NamedTempFile::new().expect("failed to create temp file");
    write!(f, r#"pipeline = "pipeline.yml""#).expect("failed to write");

    let config = parse_config(f.path()).expect("parse_config should succeed");
    assert_eq!(config.pipeline, "pipeline.yml");
}

#[test]
fn parse_config_missing_file() {
    let result = parse_config(Path::new("/nonexistent/belt.toml"));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), BeltError::FileNotFound { .. }));
}

#[test]
fn parse_config_invalid_toml() {
    let mut f = NamedTempFile::new().expect("failed to create temp file");
    write!(f, "not valid toml [[[").expect("failed to write");

    let result = parse_config(f.path());
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), BeltError::ConfigParse { .. }));
}

#[test]
fn parse_config_missing_pipeline_field() {
    let mut f = NamedTempFile::new().expect("failed to create temp file");
    write!(f, r#"something_else = "value""#).expect("failed to write");

    let result = parse_config(f.path());
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), BeltError::ConfigParse { .. }));
}

#[test]
fn resolve_pipeline_path_relative_to_config() {
    let dir = TempDir::new().expect("failed to create temp dir");
    let config_path = dir.path().join("belt.toml");
    std::fs::write(&config_path, r#"pipeline = "pipeline.yml""#).expect("write");

    let config = parse_config(&config_path).expect("parse");
    let resolved = resolve_pipeline_path(&config_path, &config);
    assert_eq!(resolved, dir.path().join("pipeline.yml"));
}

#[test]
fn resolve_pipeline_path_with_subdirectory() {
    let dir = TempDir::new().expect("failed to create temp dir");
    let config_path = dir.path().join("belt.toml");
    std::fs::write(&config_path, r#"pipeline = "pipelines/main.yml""#).expect("write");

    let config = parse_config(&config_path).expect("parse");
    let resolved = resolve_pipeline_path(&config_path, &config);
    assert_eq!(resolved, dir.path().join("pipelines/main.yml"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p belt-core --test config_test 2>&1`
Expected: compilation error — `config` module does not exist

- [ ] **Step 3: Add config module declaration**

In `crates/belt-core/src/lib.rs`, add:

```rust
pub mod config;
```

- [ ] **Step 4: Implement config module**

Create `crates/belt-core/src/config.rs`:

```rust
use crate::error::{BeltError, BeltResult};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Parsed representation of a `belt.toml` configuration file.
#[derive(Debug, Deserialize)]
pub struct BeltConfig {
    /// Path to the pipeline YAML file, relative to the config file's directory.
    pub pipeline: String,
}

/// Parse a `belt.toml` file at the given path.
///
/// # Errors
///
/// Returns `BeltError::FileNotFound` if `path` does not exist or is unreadable,
/// or `BeltError::ConfigParse` if the TOML content cannot be deserialized into
/// a [`BeltConfig`].
pub fn parse_config(path: &Path) -> BeltResult<BeltConfig> {
    let content = std::fs::read_to_string(path).map_err(|_| BeltError::FileNotFound {
        path: path.display().to_string(),
    })?;
    let config: BeltConfig = toml::from_str(&content).map_err(|e| BeltError::ConfigParse {
        path: path.display().to_string(),
        detail: e.message().to_string(),
    })?;
    Ok(config)
}

/// Resolve the pipeline file path from a config file's location.
///
/// Joins the config file's parent directory with the `pipeline` field.
/// Does not verify the resolved path exists.
pub fn resolve_pipeline_path(config_path: &Path, config: &BeltConfig) -> PathBuf {
    let base_dir = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    base_dir.join(&config.pipeline)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p belt-core --test config_test`
Expected: all 6 tests PASS

- [ ] **Step 6: Run clippy**

Run: `cargo clippy -p belt-core -- -D warnings`
Expected: no warnings

- [ ] **Step 7: Commit**

```bash
git add crates/belt-core/src/config.rs crates/belt-core/src/lib.rs crates/belt-core/tests/config_test.rs
git commit -m "feat(belt-core): add config module for belt.toml parsing (BELT-22)"
```

---

### Task 4: Add `--config` global argument to belt-agent CLI

**Files:**
- Modify: `crates/belt-agent/src/main.rs:9-53,87-98`
- Modify: `crates/belt-agent/tests/cli_test.rs`

- [ ] **Step 1: Write failing tests for --config on belt-agent**

Append to `crates/belt-agent/tests/cli_test.rs`:

```rust
#[test]
fn init_with_config_resolves_pipeline() {
    let dir = TempDir::new().unwrap();

    // Write pipeline
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: config-test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
"#,
    );

    // Write belt.toml
    std::fs::write(dir.path().join("belt.toml"), r#"pipeline = "pipeline.yml""#).unwrap();

    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .arg("--config")
        .arg(dir.path().join("belt.toml").to_str().unwrap())
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("failed to run belt-agent init with --config");

    assert!(
        output.status.success(),
        "init with --config failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let v: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("invalid JSON output");
    assert_eq!(v["pipeline"], "config-test");
    assert_eq!(v["phase"]["id"], "build");
}

#[test]
fn init_config_and_positional_file_errors() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("belt.toml"), r#"pipeline = "pipeline.yml""#).unwrap();

    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .arg("--config")
        .arg(dir.path().join("belt.toml").to_str().unwrap())
        .arg("init")
        .arg("pipeline.yml")
        .current_dir(dir.path())
        .output()
        .expect("failed to run");

    assert!(
        !output.status.success(),
        "should fail when both --config and positional are provided"
    );
}

#[test]
fn init_config_nonexistent_file_errors() {
    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .arg("--config")
        .arg("/nonexistent/belt.toml")
        .arg("init")
        .output()
        .expect("failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("file not found"),
        "stderr should mention file not found: {stderr}"
    );
}

#[test]
fn init_config_invalid_toml_errors() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("belt.toml"), "not valid [[[").unwrap();

    let output = Command::cargo_bin("belt-agent")
        .unwrap()
        .arg("--config")
        .arg(dir.path().join("belt.toml").to_str().unwrap())
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("config parse error") || stderr.contains("parse"),
        "stderr should mention config parse: {stderr}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p belt-agent --test cli_test init_with_config 2>&1`
Expected: FAIL — `--config` argument not recognized

- [ ] **Step 3: Modify Cli struct to add --config global argument**

In `crates/belt-agent/src/main.rs`, modify the `Cli` struct and `Command::Init`:

```rust
#[derive(Parser)]
#[command(
    name = "belt-agent",
    about = "belt-agent — workflow runtime for LLM/CI"
)]
struct Cli {
    /// Path to belt.toml config file
    #[arg(long, global = true)]
    config: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new run from a pipeline YAML
    Init {
        /// Path to pipeline YAML file (mutually exclusive with --config)
        file: Option<String>,
        /// Pipeline arguments (KEY=VALUE)
        #[arg(long = "arg", value_parser = parse_arg)]
        args: Vec<(String, serde_json::Value)>,
    },
    // ... rest unchanged
}
```

- [ ] **Step 4: Add pipeline resolution helper function**

In `crates/belt-agent/src/main.rs`, add a helper that resolves the pipeline path from either `--config` or positional argument:

```rust
use belt_core::config::{parse_config, resolve_pipeline_path};

fn resolve_pipeline(
    config: Option<&String>,
    file: Option<&String>,
) -> miette::Result<PathBuf> {
    match (config, file) {
        (Some(_), Some(_)) => Err(miette::miette!(
            "conflicting arguments: --config and positional <file> are mutually exclusive"
        )),
        (Some(config_path), None) => {
            let config_path = Path::new(config_path);
            let cfg = parse_config(config_path).map_err(|e| miette::miette!("{e}"))?;
            Ok(resolve_pipeline_path(config_path, &cfg))
        }
        (None, Some(f)) => Ok(PathBuf::from(f)),
        (None, None) => Err(miette::miette!(
            "missing argument: provide either --config <path> or a pipeline file"
        )),
    }
}
```

- [ ] **Step 5: Update cmd_init to use the resolver**

In `crates/belt-agent/src/main.rs`, update the match arm and `cmd_init`:

```rust
fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    let engine = Engine::new(&belt_dir());

    match cli.command {
        Command::Init { file, args } => {
            let pipeline_path = resolve_pipeline(cli.config.as_ref(), file.as_ref())?;
            cmd_init(&engine, &pipeline_path, args)?;
        }
        Command::Next { run } => cmd_next(&engine, run.as_ref())?,
        Command::Verify { run } => cmd_verify(&engine, run.as_ref())?,
        Command::Step { run, confirm } => cmd_step(&engine, run.as_ref(), confirm)?,
        Command::Status { run } => cmd_status(&engine, run.as_ref())?,
    }
    Ok(())
}

fn cmd_init(
    engine: &Engine,
    pipeline_path: &Path,
    args: Vec<(String, serde_json::Value)>,
) -> miette::Result<()> {
    let args_map: HashMap<String, serde_json::Value> = args.into_iter().collect();
    let state = engine
        .init(pipeline_path, &args_map)
        .map_err(|e| miette::miette!("{e}"))?;
    let pipeline_file = Path::new(&state.pipeline_file);
    let phase = engine
        .next_phase_info(&state, pipeline_file)
        .map_err(|e| miette::miette!("{e}"))?;

    let out = json!({
        "run_id": state.run_id,
        "pipeline": state.pipeline,
        "phase": {
            "id": phase.id,
            "description": phase.description,
            "config": phase.config,
            "artifacts": phase.artifacts,
            "output_dir": phase.output_dir,
        },
        "gate": phase.gate,
        "validate": phase.validate,
        "confirm": phase.confirm,
        "max_retries": phase.max_retries,
        "attempt": 0,
        "args": state.args,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
    );
    Ok(())
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p belt-agent --test cli_test`
Expected: all tests PASS (existing + 4 new)

- [ ] **Step 7: Run clippy**

Run: `cargo clippy -p belt-agent -- -D warnings`
Expected: no warnings

- [ ] **Step 8: Commit**

```bash
git add crates/belt-agent/src/main.rs crates/belt-agent/tests/cli_test.rs
git commit -m "feat(belt-agent): add --config global argument for belt.toml resolution (BELT-22)"
```

---

### Task 5: Add `--config` global argument to belt CLI

**Files:**
- Modify: `crates/belt/src/main.rs:1-57`
- Modify: `crates/belt/tests/cli_test.rs`

- [ ] **Step 1: Write failing tests for --config on belt**

Append to `crates/belt/tests/cli_test.rs`:

```rust
#[test]
fn lint_with_config_resolves_pipeline() {
    let dir = TempDir::new().unwrap();

    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: test
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "cargo build"
"#,
    );

    std::fs::write(dir.path().join("belt.toml"), r#"pipeline = "pipeline.yml""#).unwrap();

    Command::cargo_bin("belt")
        .unwrap()
        .arg("--config")
        .arg(dir.path().join("belt.toml").to_str().unwrap())
        .arg("lint")
        .assert()
        .success()
        .stderr(predicates::str::contains("ok"));
}

#[test]
fn lint_config_and_positional_file_errors() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("belt.toml"), r#"pipeline = "pipeline.yml""#).unwrap();

    Command::cargo_bin("belt")
        .unwrap()
        .arg("--config")
        .arg(dir.path().join("belt.toml").to_str().unwrap())
        .arg("lint")
        .arg("pipeline.yml")
        .assert()
        .code(1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p belt --test cli_test lint_with_config 2>&1`
Expected: FAIL — `--config` argument not recognized

- [ ] **Step 3: Modify belt CLI to add --config and pipeline resolution**

Replace `crates/belt/src/main.rs`:

```rust
use belt_core::config::{parse_config, resolve_pipeline_path};
use belt_core::lint::{Severity, lint_pipeline};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "belt", about = "belt — pipeline authoring tool")]
struct Cli {
    /// Path to belt.toml config file
    #[arg(long, global = true)]
    config: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a pipeline YAML file
    Lint {
        /// Path to pipeline YAML file (mutually exclusive with --config)
        file: Option<String>,
    },
}

fn resolve_pipeline(
    config: Option<&String>,
    file: Option<&String>,
) -> Result<PathBuf, String> {
    match (config, file) {
        (Some(_), Some(_)) => Err(
            "conflicting arguments: --config and positional <file> are mutually exclusive"
                .to_string(),
        ),
        (Some(config_path), None) => {
            let config_path = Path::new(config_path);
            let cfg = parse_config(config_path).map_err(|e| e.to_string())?;
            Ok(resolve_pipeline_path(config_path, &cfg))
        }
        (None, Some(f)) => Ok(PathBuf::from(f)),
        (None, None) => {
            Err("missing argument: provide either --config <path> or a pipeline file".to_string())
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Lint { file } => {
            let path = match resolve_pipeline(cli.config.as_ref(), file.as_ref()) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("error: {e}");
                    return ExitCode::from(1);
                }
            };
            match lint_pipeline(&path) {
                Ok(diagnostics) => {
                    let mut has_errors = false;
                    for diag in &diagnostics {
                        let prefix = match diag.severity {
                            Severity::Error => {
                                has_errors = true;
                                "error"
                            }
                            Severity::Warning => "warning",
                        };
                        eprintln!("{prefix}: {}", diag.message);
                    }
                    if has_errors {
                        ExitCode::from(1)
                    } else if diagnostics.is_empty() {
                        let display = file
                            .as_deref()
                            .unwrap_or_else(|| path.to_str().unwrap_or("pipeline"));
                        eprintln!("ok: {display}");
                        ExitCode::SUCCESS
                    } else {
                        let display = file
                            .as_deref()
                            .unwrap_or_else(|| path.to_str().unwrap_or("pipeline"));
                        eprintln!("ok (with warnings): {display}");
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(1)
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p belt --test cli_test`
Expected: all 5 tests PASS (existing 3 + new 2)

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p belt -- -D warnings`
Expected: no warnings

- [ ] **Step 6: Commit**

```bash
git add crates/belt/src/main.rs crates/belt/tests/cli_test.rs
git commit -m "feat(belt): add --config global argument for belt.toml resolution (BELT-22)"
```

---

### Task 6: Relocate pipeline and skill files to examples/skills/

**Files:**
- Move: `pipelines/linear-refresh.yml` → `examples/skills/linear-refresh/pipeline.yml`
- Move: `pipelines/linear-cleanup.yml` → `examples/skills/linear-refresh/linear-cleanup.yml`
- Move: `pipelines/linear-add.yml` → `examples/skills/linear-refresh/linear-add.yml`
- Move: `pipelines/smoke-test.yml` → `examples/skills/smoke-test/pipeline.yml`
- Move: `skills/linear-refresh/SKILL.md` → `examples/skills/linear-refresh/SKILL.md`
- Move: `skills/linear-refresh/references/` → `examples/skills/linear-refresh/references/`
- Move: `skills/smoke-test/SKILL.md` → `examples/skills/smoke-test/SKILL.md`
- Move: `skills/smoke-test/references/` → `examples/skills/smoke-test/references/`
- Create: `examples/skills/linear-refresh/belt.toml`
- Create: `examples/skills/smoke-test/belt.toml`
- Delete: `pipelines/` directory
- Delete: `skills/linear-refresh/` directory
- Delete: `skills/smoke-test/` directory

- [ ] **Step 1: Create target directories**

```bash
mkdir -p examples/skills/linear-refresh
mkdir -p examples/skills/smoke-test
```

- [ ] **Step 2: Move linear-refresh files**

```bash
git mv pipelines/linear-refresh.yml examples/skills/linear-refresh/pipeline.yml
git mv pipelines/linear-cleanup.yml examples/skills/linear-refresh/linear-cleanup.yml
git mv pipelines/linear-add.yml examples/skills/linear-refresh/linear-add.yml
git mv skills/linear-refresh/SKILL.md examples/skills/linear-refresh/SKILL.md
git mv skills/linear-refresh/references examples/skills/linear-refresh/references
```

- [ ] **Step 3: Move smoke-test files**

```bash
git mv pipelines/smoke-test.yml examples/skills/smoke-test/pipeline.yml
git mv skills/smoke-test/SKILL.md examples/skills/smoke-test/SKILL.md
git mv skills/smoke-test/references examples/skills/smoke-test/references
```

- [ ] **Step 4: Create belt.toml files**

Create `examples/skills/linear-refresh/belt.toml`:

```toml
pipeline = "pipeline.yml"
```

Create `examples/skills/smoke-test/belt.toml`:

```toml
pipeline = "pipeline.yml"
```

- [ ] **Step 5: Remove empty directories**

```bash
rmdir pipelines
rmdir skills/linear-refresh
rmdir skills/smoke-test
```

(git tracks files not directories; these should be empty after `git mv`.)

- [ ] **Step 6: Verify directory structure**

```bash
ls -R examples/skills/
ls skills/
```

Expected:
- `examples/skills/linear-refresh/` contains: `belt.toml`, `pipeline.yml`, `linear-cleanup.yml`, `linear-add.yml`, `SKILL.md`, `references/`
- `examples/skills/smoke-test/` contains: `belt.toml`, `pipeline.yml`, `SKILL.md`, `references/`
- `skills/` contains only: `belt-agent/`

- [ ] **Step 7: Verify belt lint via --config**

```bash
belt lint --config examples/skills/linear-refresh/belt.toml
belt lint --config examples/skills/smoke-test/belt.toml
```

Expected: both exit 0 (ok or ok with warnings)

- [ ] **Step 8: Verify belt lint direct path still works**

```bash
belt lint examples/skills/linear-refresh/pipeline.yml
belt lint examples/skills/smoke-test/pipeline.yml
```

Expected: both exit 0

- [ ] **Step 9: Commit**

```bash
git add examples/skills/ skills/
git add -A pipelines/
git commit -m "refactor: relocate pipelines and skills to examples/skills/ with belt.toml (BELT-22)"
```

---

### Task 7: Final verification

**Files:** None (verification only)

- [ ] **Step 1: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: all tests pass (existing + new config/CLI tests)

- [ ] **Step 2: Run workspace clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Run workspace fmt check**

Run: `cargo fmt --check --all`
Expected: no formatting issues

- [ ] **Step 4: Verify no stale references to pipelines/ directory**

Run: `grep -r "pipelines/" --include="*.rs" --include="*.yml" --include="*.md" . | grep -v target/ | grep -v ".git/" | grep -v "docs/specs/" | grep -v "docs/plans/"`
Expected: no references to the old `pipelines/` directory in active code (docs/specs references are fine)

- [ ] **Step 5: Run belt-agent init via --config end-to-end**

```bash
cd /tmp && belt-agent --config /path/to/belt/examples/skills/linear-refresh/belt.toml init --arg force=true
```

Expected: JSON output with `pipeline: "linear-refresh"`, phase info, and run_id. Verify that `uses:` sub-pipelines resolve correctly (phases should include `cleanup-analysis/analyze` and `add-analysis/analyze`).
