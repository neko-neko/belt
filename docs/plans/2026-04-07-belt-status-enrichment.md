# BELT-29: Status Command Enrichment — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enrich `belt-agent status` to return a structured view model with per-phase status, output paths, and progress summary — assembled at query time without extending RunState.

**Architecture:** New `view` module in belt-core defines view types (`StatusView`, `PhaseView`) and a pure `build_status_view()` function. Engine gets `enriched_status()` that orchestrates: load state → expand pipeline → scan output dirs → build view. belt-agent's `cmd_status` switches to this enriched output.

**Tech Stack:** Rust, serde (Serialize only — view types are never deserialized), std::fs for output dir scanning.

---

## File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Create | `crates/belt-core/src/view.rs` | View types + `build_status_view()` + `scan_phase_outputs()` |
| Modify | `crates/belt-core/src/lib.rs:1-8` | Add `pub mod view` |
| Modify | `crates/belt-core/src/engine.rs:255-258` | Add `enriched_status()` method |
| Modify | `crates/belt-agent/src/main.rs:468-478` | Switch `cmd_status` to enriched view |
| Create | `crates/belt-core/tests/view_test.rs` | Unit tests for view building logic |
| Modify | `crates/belt-agent/tests/cli_test.rs` | Update existing status tests + add enriched status tests |
| Modify | `skills/belt-agent/SKILL.md` | Document enriched status output |
| Modify | `README.md` | Add status command section with output sample |

---

### Task 1: Create view.rs with type definitions

**Files:**
- Create: `crates/belt-core/src/view.rs`
- Modify: `crates/belt-core/src/lib.rs`

- [ ] **Step 1: Create view.rs with all view types**

```rust
// crates/belt-core/src/view.rs
use serde::Serialize;
use std::collections::HashMap;

/// Enriched status view assembled at query time.
///
/// Built from RunState + expanded pipeline + filesystem scan.
/// Never persisted — this is a read-only projection.
#[derive(Debug, Serialize)]
pub struct StatusView {
    pub run_id: String,
    pub pipeline: String,
    pub pipeline_file: String,
    pub version: u32,
    pub args: HashMap<String, serde_json::Value>,
    pub status: PipelineStatus,
    pub current_phase: Option<String>,
    pub progress: Progress,
    pub phases: Vec<PhaseView>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    InProgress,
    Completed,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub completed: usize,
    pub skipped: usize,
    pub remaining: usize,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct PhaseView {
    pub id: String,
    pub status: PhaseState,
    pub verify_passed: Option<bool>,
    pub regate_passed: Option<bool>,
    pub attempt: u32,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseState {
    Completed,
    Current,
    Pending,
    Skipped,
}
```

- [ ] **Step 2: Add pub mod view to lib.rs**

Add `pub mod view;` to `crates/belt-core/src/lib.rs` (after the `parser` line, to maintain alphabetical order):

```rust
pub mod config;
pub mod engine;
pub mod error;
pub mod expander;
pub mod gate;
pub mod lint;
pub mod model;
pub mod parser;
pub mod view;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p belt-core`
Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add crates/belt-core/src/view.rs crates/belt-core/src/lib.rs
git commit -m "feat(belt-core): add view module with StatusView types (BELT-29)"
```

---

### Task 2: TDD — PhaseState determination and progress calculation

**Files:**
- Create: `crates/belt-core/tests/view_test.rs`
- Modify: `crates/belt-core/src/view.rs`

- [ ] **Step 1: Write failing tests for phase state determination**

```rust
// crates/belt-core/tests/view_test.rs
use belt_core::model::RunState;
use belt_core::view::{build_status_view, PhaseState, PipelineStatus};
use std::collections::HashMap;

fn make_state(
    current: &str,
    completed: &[&str],
    skipped: &[&str],
) -> RunState {
    RunState {
        run_id: "test-run".to_string(),
        pipeline: "test".to_string(),
        pipeline_file: "/tmp/pipeline.yml".to_string(),
        version: 1,
        args: HashMap::new(),
        current_phase: current.to_string(),
        completed_phases: completed.iter().map(|s| (*s).to_string()).collect(),
        skipped_phases: skipped.iter().map(|s| (*s).to_string()).collect(),
        phase_attempts: HashMap::new(),
        phase_verify_passed: HashMap::new(),
        regate_passed: HashMap::new(),
        created_at: "2026-04-07T00:00:00Z".to_string(),
        updated_at: "2026-04-07T00:00:00Z".to_string(),
    }
}

/// Phase IDs to use as the "expanded pipeline" phase list.
/// build_status_view takes phase IDs as &[String] (extracted from ExpandedPhase).
fn phase_ids(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn initial_state_first_current_rest_pending() {
    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build", "test", "deploy"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.status, PipelineStatus::InProgress);
    assert_eq!(view.current_phase.as_deref(), Some("build"));
    assert_eq!(view.phases.len(), 3);
    assert_eq!(view.phases[0].status, PhaseState::Current);
    assert_eq!(view.phases[1].status, PhaseState::Pending);
    assert_eq!(view.phases[2].status, PhaseState::Pending);
}

#[test]
fn partially_completed() {
    let state = make_state("deploy", &["build", "test"], &[]);
    let ids = phase_ids(&["build", "test", "deploy"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases[0].status, PhaseState::Completed);
    assert_eq!(view.phases[1].status, PhaseState::Completed);
    assert_eq!(view.phases[2].status, PhaseState::Current);
}

#[test]
fn fully_completed() {
    let state = make_state("COMPLETED", &["build", "test", "deploy"], &[]);
    let ids = phase_ids(&["build", "test", "deploy"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.status, PipelineStatus::Completed);
    assert!(view.current_phase.is_none());
    assert!(view.phases.iter().all(|p| p.status == PhaseState::Completed));
}

#[test]
fn skipped_phases() {
    let state = make_state("deploy", &["build"], &["test"]);
    let ids = phase_ids(&["build", "test", "deploy"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases[0].status, PhaseState::Completed);
    assert_eq!(view.phases[1].status, PhaseState::Skipped);
    assert_eq!(view.phases[2].status, PhaseState::Current);
}

#[test]
fn mixed_skip_and_completed() {
    let state = make_state("COMPLETED", &["build", "deploy"], &["test", "review"]);
    let ids = phase_ids(&["build", "test", "review", "deploy"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases[0].status, PhaseState::Completed);
    assert_eq!(view.phases[1].status, PhaseState::Skipped);
    assert_eq!(view.phases[2].status, PhaseState::Skipped);
    assert_eq!(view.phases[3].status, PhaseState::Completed);
}

#[test]
fn progress_normal() {
    let state = make_state("deploy", &["build", "test"], &["review"]);
    let ids = phase_ids(&["build", "test", "review", "deploy", "done"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.progress.completed, 2);
    assert_eq!(view.progress.skipped, 1);
    assert_eq!(view.progress.remaining, 2); // current + pending
    assert_eq!(view.progress.total, 5);
}

#[test]
fn progress_all_complete() {
    let state = make_state("COMPLETED", &["a", "b", "c"], &[]);
    let ids = phase_ids(&["a", "b", "c"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.progress.completed, 3);
    assert_eq!(view.progress.skipped, 0);
    assert_eq!(view.progress.remaining, 0);
    assert_eq!(view.progress.total, 3);
}

#[test]
fn progress_all_pending() {
    let state = make_state("a", &[], &[]);
    let ids = phase_ids(&["a", "b", "c"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.progress.completed, 0);
    assert_eq!(view.progress.skipped, 0);
    assert_eq!(view.progress.remaining, 3); // current counts as remaining
    assert_eq!(view.progress.total, 3);
}

#[test]
fn single_phase_pipeline() {
    let state = make_state("only", &[], &[]);
    let ids = phase_ids(&["only"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases.len(), 1);
    assert_eq!(view.phases[0].status, PhaseState::Current);
    assert_eq!(view.progress.total, 1);
    assert_eq!(view.progress.remaining, 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p belt-core --test view_test`
Expected: compilation error — `build_status_view` not found

- [ ] **Step 3: Implement build_status_view**

Add to `crates/belt-core/src/view.rs`:

```rust
use crate::model::RunState;
use std::path::Path;

const COMPLETED_SENTINEL: &str = "COMPLETED";

/// Scan a phase output directory for top-level file names.
///
/// Returns sorted file names. Gracefully returns empty vec on any error
/// (missing dir, permission denied, etc.) — status must not break.
fn scan_phase_outputs(run_dir: &Path, phase_id: &str) -> Vec<String> {
    let dir = run_dir.join(phase_id.replace('/', "_"));
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut files: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|ft| ft.is_file()))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    files.sort();
    files
}

fn determine_phase_state(
    phase_id: &str,
    state: &RunState,
) -> PhaseState {
    if state.completed_phases.contains(&phase_id.to_string()) {
        PhaseState::Completed
    } else if state.skipped_phases.contains(&phase_id.to_string()) {
        PhaseState::Skipped
    } else if state.current_phase == phase_id {
        PhaseState::Current
    } else {
        PhaseState::Pending
    }
}

/// Build an enriched status view from RunState + phase ID list + run directory.
///
/// `phase_ids` is the ordered list of expanded phase IDs from the pipeline YAML.
/// `run_dir` is `.belt/runs/{run_id}/` for output directory scanning.
pub fn build_status_view(
    state: &RunState,
    phase_ids: &[String],
    run_dir: &Path,
) -> StatusView {
    let is_completed = state.current_phase == COMPLETED_SENTINEL;

    // Build per-phase views from YAML-defined phases
    let mut phases: Vec<PhaseView> = phase_ids
        .iter()
        .map(|id| {
            let status = if is_completed && state.completed_phases.contains(id) {
                PhaseState::Completed
            } else {
                determine_phase_state(id, state)
            };
            PhaseView {
                id: id.clone(),
                status,
                verify_passed: state.phase_verify_passed.get(id).copied(),
                regate_passed: state.regate_passed.get(id).copied(),
                attempt: state.phase_attempts.get(id).copied().unwrap_or(0),
                outputs: scan_phase_outputs(run_dir, id),
            }
        })
        .collect();

    // Append orphan phases (in state but removed from YAML)
    let yaml_ids: std::collections::HashSet<&String> = phase_ids.iter().collect();
    for id in &state.completed_phases {
        if !yaml_ids.contains(id) {
            phases.push(PhaseView {
                id: id.clone(),
                status: PhaseState::Completed,
                verify_passed: state.phase_verify_passed.get(id).copied(),
                regate_passed: state.regate_passed.get(id).copied(),
                attempt: state.phase_attempts.get(id).copied().unwrap_or(0),
                outputs: scan_phase_outputs(run_dir, id),
            });
        }
    }
    for id in &state.skipped_phases {
        if !yaml_ids.contains(id) && !state.completed_phases.contains(id) {
            phases.push(PhaseView {
                id: id.clone(),
                status: PhaseState::Skipped,
                verify_passed: None,
                regate_passed: None,
                attempt: 0,
                outputs: Vec::new(),
            });
        }
    }

    // Compute progress
    let completed = phases.iter().filter(|p| p.status == PhaseState::Completed).count();
    let skipped = phases.iter().filter(|p| p.status == PhaseState::Skipped).count();
    let total = phases.len();
    let remaining = total - completed - skipped;

    StatusView {
        run_id: state.run_id.clone(),
        pipeline: state.pipeline.clone(),
        pipeline_file: state.pipeline_file.clone(),
        version: state.version,
        args: state.args.clone(),
        status: if is_completed {
            PipelineStatus::Completed
        } else {
            PipelineStatus::InProgress
        },
        current_phase: if is_completed {
            None
        } else {
            Some(state.current_phase.clone())
        },
        progress: Progress {
            completed,
            skipped,
            remaining,
            total,
        },
        phases,
        created_at: state.created_at.clone(),
        updated_at: state.updated_at.clone(),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p belt-core --test view_test`
Expected: all 10 tests pass

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p belt-core -- -D warnings`
Expected: no warnings

- [ ] **Step 6: Commit**

```bash
git add crates/belt-core/src/view.rs crates/belt-core/tests/view_test.rs
git commit -m "feat(belt-core): implement build_status_view with phase state and progress (BELT-29)"
```

---

### Task 3: TDD — Verify, regate, and attempt state in view

**Files:**
- Modify: `crates/belt-core/tests/view_test.rs`

- [ ] **Step 1: Write tests for verify/regate/attempt fields**

Append to `crates/belt-core/tests/view_test.rs`:

```rust
#[test]
fn verify_not_run_is_none() {
    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert!(view.phases[0].verify_passed.is_none());
}

#[test]
fn verify_pass_reflected() {
    let mut state = make_state("build", &[], &[]);
    state.phase_verify_passed.insert("build".to_string(), true);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases[0].verify_passed, Some(true));
}

#[test]
fn verify_fail_reflected() {
    let mut state = make_state("build", &[], &[]);
    state.phase_verify_passed.insert("build".to_string(), false);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases[0].verify_passed, Some(false));
}

#[test]
fn regate_pass_reflected() {
    let mut state = make_state("build", &[], &[]);
    state.regate_passed.insert("build".to_string(), true);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases[0].regate_passed, Some(true));
}

#[test]
fn regate_not_run_is_none() {
    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert!(view.phases[0].regate_passed.is_none());
}

#[test]
fn attempt_zero_when_not_run() {
    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases[0].attempt, 0);
}

#[test]
fn attempt_count_reflected() {
    let mut state = make_state("build", &[], &[]);
    state.phase_attempts.insert("build".to_string(), 3);
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases[0].attempt, 3);
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p belt-core --test view_test`
Expected: all 17 tests pass (these test already-implemented fields)

- [ ] **Step 3: Commit**

```bash
git add crates/belt-core/tests/view_test.rs
git commit -m "test(belt-core): add verify/regate/attempt view tests (BELT-29)"
```

---

### Task 4: TDD — Output directory scanning

**Files:**
- Modify: `crates/belt-core/tests/view_test.rs`

- [ ] **Step 1: Write tests for output scanning**

Append to `crates/belt-core/tests/view_test.rs`:

```rust
#[test]
fn outputs_lists_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let phase_dir = dir.path().join("build");
    std::fs::create_dir_all(&phase_dir).expect("mkdir");
    std::fs::write(phase_dir.join("report.json"), "{}").expect("write");
    std::fs::write(phase_dir.join("a_summary.md"), "# ok").expect("write");

    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);

    let view = build_status_view(&state, &ids, dir.path());

    // Sorted alphabetically
    assert_eq!(view.phases[0].outputs, vec!["a_summary.md", "report.json"]);
}

#[test]
fn outputs_empty_when_no_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Don't create any phase directory

    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);

    let view = build_status_view(&state, &ids, dir.path());

    assert!(view.phases[0].outputs.is_empty());
}

#[test]
fn outputs_empty_when_dir_exists_but_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let phase_dir = dir.path().join("build");
    std::fs::create_dir_all(&phase_dir).expect("mkdir");

    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);

    let view = build_status_view(&state, &ids, dir.path());

    assert!(view.phases[0].outputs.is_empty());
}

#[test]
fn outputs_excludes_subdirectories() {
    let dir = tempfile::tempdir().expect("tempdir");
    let phase_dir = dir.path().join("build");
    std::fs::create_dir_all(phase_dir.join("subdir")).expect("mkdir");
    std::fs::write(phase_dir.join("file.txt"), "data").expect("write");

    let state = make_state("build", &[], &[]);
    let ids = phase_ids(&["build"]);

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases[0].outputs, vec!["file.txt"]);
}

#[test]
fn outputs_sub_pipeline_phase_uses_underscore_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Phase ID "review/triage" maps to directory "review_triage"
    let phase_dir = dir.path().join("review_triage");
    std::fs::create_dir_all(&phase_dir).expect("mkdir");
    std::fs::write(phase_dir.join("findings.md"), "# findings").expect("write");

    let state = make_state("review/triage", &[], &[]);
    let ids = phase_ids(&["review/triage"]);

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases[0].outputs, vec!["findings.md"]);
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p belt-core --test view_test`
Expected: all 22 tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/belt-core/tests/view_test.rs
git commit -m "test(belt-core): add output directory scanning tests (BELT-29)"
```

---

### Task 5: TDD — YAML drift (phase added/removed)

**Files:**
- Modify: `crates/belt-core/tests/view_test.rs`

- [ ] **Step 1: Write tests for drift handling**

Append to `crates/belt-core/tests/view_test.rs`:

```rust
#[test]
fn yaml_drift_phase_added() {
    // State knows about build and test, but YAML now has a new "lint" phase
    let state = make_state("test", &["build"], &[]);
    let ids = phase_ids(&["build", "lint", "test"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases[0].status, PhaseState::Completed); // build
    assert_eq!(view.phases[1].status, PhaseState::Pending);   // lint (new)
    assert_eq!(view.phases[2].status, PhaseState::Current);    // test
}

#[test]
fn yaml_drift_phase_removed_completed() {
    // "old-phase" was completed but no longer in YAML
    let state = make_state("test", &["old-phase", "build"], &[]);
    let ids = phase_ids(&["build", "test"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    // YAML phases first
    assert_eq!(view.phases[0].status, PhaseState::Completed); // build
    assert_eq!(view.phases[1].status, PhaseState::Current);    // test
    // Orphan appended
    assert_eq!(view.phases.len(), 3);
    assert_eq!(view.phases[2].id, "old-phase");
    assert_eq!(view.phases[2].status, PhaseState::Completed);
}

#[test]
fn yaml_drift_phase_removed_skipped() {
    // "old-phase" was skipped but no longer in YAML
    let state = make_state("test", &["build"], &["old-phase"]);
    let ids = phase_ids(&["build", "test"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.phases.len(), 3);
    assert_eq!(view.phases[2].id, "old-phase");
    assert_eq!(view.phases[2].status, PhaseState::Skipped);
}

#[test]
fn metadata_fields_propagated() {
    let mut state = make_state("build", &[], &[]);
    state.args.insert("smoke".to_string(), serde_json::Value::Bool(true));
    let ids = phase_ids(&["build"]);
    let dir = tempfile::tempdir().expect("tempdir");

    let view = build_status_view(&state, &ids, dir.path());

    assert_eq!(view.run_id, "test-run");
    assert_eq!(view.pipeline, "test");
    assert_eq!(view.pipeline_file, "/tmp/pipeline.yml");
    assert_eq!(view.version, 1);
    assert_eq!(view.args.get("smoke"), Some(&serde_json::Value::Bool(true)));
    assert_eq!(view.created_at, "2026-04-07T00:00:00Z");
    assert_eq!(view.updated_at, "2026-04-07T00:00:00Z");
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p belt-core --test view_test`
Expected: all 26 tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/belt-core/tests/view_test.rs
git commit -m "test(belt-core): add YAML drift and metadata propagation tests (BELT-29)"
```

---

### Task 6: Engine enriched_status method

**Files:**
- Modify: `crates/belt-core/src/engine.rs`

- [ ] **Step 1: Add enriched_status method to Engine**

Add after the existing `status()` method at line 258 in `crates/belt-core/src/engine.rs`:

```rust
    /// Return an enriched status view for a given run.
    ///
    /// Assembles RunState + expanded pipeline YAML + filesystem scan
    /// into a [`StatusView`] projection. Does not modify any state.
    pub fn enriched_status(&self, run_id: &str) -> BeltResult<crate::view::StatusView> {
        let state = self.load_state(run_id)?;
        let pipeline_path = std::path::Path::new(&state.pipeline_file);
        if !pipeline_path.exists() {
            return Err(BeltError::FileNotFound {
                path: state.pipeline_file.clone(),
            });
        }
        let phases = expand_pipeline(pipeline_path)?;
        let phase_ids: Vec<String> = phases.iter().map(|p| p.id.clone()).collect();
        let run_dir = self.belt_dir.join("runs").join(run_id);
        Ok(crate::view::build_status_view(&state, &phase_ids, &run_dir))
    }
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p belt-core`
Expected: compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add crates/belt-core/src/engine.rs
git commit -m "feat(belt-core): add Engine::enriched_status() method (BELT-29)"
```

---

### Task 7: Engine integration tests

**Files:**
- Create: `crates/belt-core/tests/fixtures/status_pipeline.yml`
- Modify: `crates/belt-core/tests/view_test.rs`

- [ ] **Step 1: Create test fixture**

```yaml
# crates/belt-core/tests/fixtures/status_pipeline.yml
name: status-test
version: 1
args:
  skip_review:
    type: bool
    default: false
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
  - id: review
    description: "Review"
    when: "!args.skip_review"
  - id: deploy
    description: "Deploy"
```

- [ ] **Step 2: Write integration tests using Engine**

Append to `crates/belt-core/tests/view_test.rs`:

```rust
use belt_core::engine::Engine;
use belt_core::view::PipelineStatus;

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn engine_enriched_status_after_init() {
    let dir = tempfile::tempdir().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline = fixture_path("status_pipeline.yml");

    let args = HashMap::new();
    let state = engine.init(&pipeline, &args).expect("init");

    let view = engine.enriched_status(&state.run_id).expect("enriched_status");

    assert_eq!(view.status, PipelineStatus::InProgress);
    assert_eq!(view.current_phase.as_deref(), Some("build"));
    assert_eq!(view.phases.len(), 3);
    assert_eq!(view.phases[0].id, "build");
    assert_eq!(view.phases[0].status, PhaseState::Current);
    assert_eq!(view.phases[1].id, "review");
    assert_eq!(view.phases[1].status, PhaseState::Pending);
    assert_eq!(view.phases[2].id, "deploy");
    assert_eq!(view.phases[2].status, PhaseState::Pending);
    assert_eq!(view.progress.total, 3);
    assert_eq!(view.progress.remaining, 3);
}

#[test]
fn engine_enriched_status_with_skipped_phase() {
    let dir = tempfile::tempdir().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline = fixture_path("status_pipeline.yml");

    let mut args = HashMap::new();
    args.insert("skip_review".to_string(), serde_json::Value::Bool(true));
    let mut state = engine.init(&pipeline, &args).expect("init");

    // verify (auto-passed since gate is `cmd: true`) and step
    engine.verify_verdict(&mut state, true).expect("verify");
    engine.step(&mut state, &pipeline).expect("step");

    let view = engine.enriched_status(&state.run_id).expect("enriched_status");

    // build completed, review skipped, deploy current
    assert_eq!(view.phases[0].status, PhaseState::Completed);
    assert_eq!(view.phases[1].status, PhaseState::Skipped);
    assert_eq!(view.phases[2].status, PhaseState::Current);
    assert_eq!(view.progress.completed, 1);
    assert_eq!(view.progress.skipped, 1);
    assert_eq!(view.progress.remaining, 1);
}

#[test]
fn engine_enriched_status_pipeline_not_found() {
    let dir = tempfile::tempdir().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline = fixture_path("status_pipeline.yml");

    let state = engine.init(&pipeline, &HashMap::new()).expect("init");

    // Tamper with state to point to a non-existent pipeline
    let mut loaded = engine.load_state(&state.run_id).expect("load");
    loaded.pipeline_file = "/nonexistent/pipeline.yml".to_string();
    engine.save_state(&loaded).expect("save");

    let result = engine.enriched_status(&state.run_id);
    assert!(result.is_err());
}

#[test]
fn engine_enriched_status_output_files_visible() {
    let dir = tempfile::tempdir().expect("tempdir");
    let belt_dir = dir.path().join(".belt");
    let engine = Engine::new(&belt_dir);
    let pipeline = fixture_path("status_pipeline.yml");

    let state = engine.init(&pipeline, &HashMap::new()).expect("init");

    // Write a file into the build phase output directory
    let output_dir = belt_dir.join("runs").join(&state.run_id).join("build");
    std::fs::create_dir_all(&output_dir).expect("mkdir");
    std::fs::write(output_dir.join("artifact.tar.gz"), b"data").expect("write");

    let view = engine.enriched_status(&state.run_id).expect("enriched_status");

    assert_eq!(view.phases[0].outputs, vec!["artifact.tar.gz"]);
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p belt-core --test view_test`
Expected: all 30 tests pass

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p belt-core -- -D warnings`
Expected: no warnings

- [ ] **Step 5: Commit**

```bash
git add crates/belt-core/tests/fixtures/status_pipeline.yml crates/belt-core/tests/view_test.rs crates/belt-core/src/engine.rs
git commit -m "test(belt-core): add engine enriched_status integration tests (BELT-29)"
```

---

### Task 8: Update belt-agent cmd_status

**Files:**
- Modify: `crates/belt-agent/src/main.rs:468-478`

- [ ] **Step 1: Update cmd_status to use enriched_status**

Replace `cmd_status` function in `crates/belt-agent/src/main.rs` (lines 468-478):

```rust
fn cmd_status(engine: &Engine, run: Option<&String>) -> miette::Result<()> {
    let run_id = resolve_run(engine, run)?;
    let view = engine
        .enriched_status(&run_id)
        .map_err(|e| miette::miette!("{e}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&view).map_err(|e| miette::miette!("{e}"))?
    );
    Ok(())
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p belt-agent`
Expected: compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add crates/belt-agent/src/main.rs
git commit -m "feat(belt-agent): switch status to enriched view (BELT-29)"
```

---

### Task 9: belt-agent CLI integration tests

**Files:**
- Modify: `crates/belt-agent/tests/cli_test.rs`

Note: existing status tests in cli_test.rs need updating since the output format changed.

- [ ] **Step 1: Find and update existing status tests**

Search for existing status-related tests in `crates/belt-agent/tests/cli_test.rs`. Update assertions to match the new enriched output format. If no existing status tests exist, skip this step.

- [ ] **Step 2: Add new enriched status integration tests**

Append to `crates/belt-agent/tests/cli_test.rs` (use the existing `run_belt_agent` helper and `write_yaml` patterns):

```rust
#[test]
fn status_after_init_returns_enriched_view() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: status-e2e
version: 1
phases:
  - id: build
    description: "Build"
  - id: test
    description: "Test"
  - id: deploy
    description: "Deploy"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().expect("run_id");

    let status = run_belt_agent(&dir, &["status", "--run", run_id]);

    assert_eq!(status["status"], "in_progress");
    assert_eq!(status["current_phase"], "build");
    assert_eq!(status["pipeline"], "status-e2e");
    assert_eq!(status["progress"]["completed"], 0);
    assert_eq!(status["progress"]["total"], 3);
    assert_eq!(status["progress"]["remaining"], 3);

    let phases = status["phases"].as_array().expect("phases array");
    assert_eq!(phases.len(), 3);
    assert_eq!(phases[0]["id"], "build");
    assert_eq!(phases[0]["status"], "current");
    assert_eq!(phases[1]["id"], "test");
    assert_eq!(phases[1]["status"], "pending");
    assert_eq!(phases[2]["id"], "deploy");
    assert_eq!(phases[2]["status"], "pending");
}

#[test]
fn status_after_verify_reflects_result() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: verify-status
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
  - id: done
    description: "Done"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().expect("run_id");

    run_belt_agent(&dir, &["verify", "--run", run_id]);
    let status = run_belt_agent(&dir, &["status", "--run", run_id]);

    let phases = status["phases"].as_array().expect("phases");
    assert_eq!(phases[0]["verify_passed"], true);
    assert_eq!(phases[0]["attempt"], 1);
}

#[test]
fn status_after_step_shows_progression() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: step-status
version: 1
phases:
  - id: build
    description: "Build"
  - id: test
    description: "Test"
  - id: deploy
    description: "Deploy"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().expect("run_id");

    // build is gate-less → auto verify_passed
    run_belt_agent(&dir, &["step", "--run", run_id]);
    let status = run_belt_agent(&dir, &["status", "--run", run_id]);

    assert_eq!(status["current_phase"], "test");
    assert_eq!(status["progress"]["completed"], 1);

    let phases = status["phases"].as_array().expect("phases");
    assert_eq!(phases[0]["status"], "completed");
    assert_eq!(phases[1]["status"], "current");
    assert_eq!(phases[2]["status"], "pending");
}

#[test]
fn status_after_completion_shows_null_current() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: complete-status
version: 1
phases:
  - id: only
    description: "Only phase"
"#,
    );

    let init = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run_id = init["run_id"].as_str().expect("run_id");

    // gate-less → auto verify → step completes pipeline
    run_belt_agent(&dir, &["step", "--run", run_id]);
    let status = run_belt_agent(&dir, &["status", "--run", run_id]);

    assert_eq!(status["status"], "completed");
    assert!(status["current_phase"].is_null());
    assert_eq!(status["progress"]["completed"], 1);
    assert_eq!(status["progress"]["remaining"], 0);
}

#[test]
fn status_run_flag_selects_correct_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_yaml(
        &dir,
        "pipeline.yml",
        r#"
name: multi-run
version: 1
phases:
  - id: build
    description: "Build"
  - id: test
    description: "Test"
"#,
    );

    let init1 = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run1 = init1["run_id"].as_str().expect("run_id").to_string();

    // Step first run forward
    run_belt_agent(&dir, &["step", "--run", &run1]);

    // Create second run (latest)
    let init2 = run_belt_agent(&dir, &["init", "pipeline.yml"]);
    let run2 = init2["run_id"].as_str().expect("run_id").to_string();

    // Status of first run (not latest)
    let status1 = run_belt_agent(&dir, &["status", "--run", &run1]);
    assert_eq!(status1["run_id"], run1);
    assert_eq!(status1["current_phase"], "test");

    // Status of second run (latest)
    let status2 = run_belt_agent(&dir, &["status", "--run", &run2]);
    assert_eq!(status2["run_id"], run2);
    assert_eq!(status2["current_phase"], "build");
}
```

- [ ] **Step 3: Run all belt-agent tests**

Run: `cargo test -p belt-agent`
Expected: all tests pass (existing + new)

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p belt-agent -- -D warnings`
Expected: no warnings

- [ ] **Step 5: Commit**

```bash
git add crates/belt-agent/tests/cli_test.rs
git commit -m "test(belt-agent): add enriched status CLI integration tests (BELT-29)"
```

---

### Task 10: Update SKILL.md

**Files:**
- Modify: `skills/belt-agent/SKILL.md`

- [ ] **Step 1: Update SKILL.md**

In `skills/belt-agent/SKILL.md`, update the Commands section and add status output documentation.

Replace the status command line in the Commands section:

```bash
# Inspect full run state (enriched view)
belt-agent status
belt-agent status --run <run_id>
```

Add a new section after the "Decision Rules" table:

```markdown
## Status Output

`status` returns an enriched view assembled from run state, pipeline YAML, and output directories:

```json
{
  "status": "in_progress",
  "current_phase": "review",
  "progress": { "completed": 2, "skipped": 0, "remaining": 2, "total": 4 },
  "phases": [
    { "id": "build", "status": "completed", "verify_passed": true, "attempt": 1, "outputs": ["report.json"] },
    { "id": "review", "status": "current", "verify_passed": false, "attempt": 2, "outputs": [] },
    { "id": "test", "status": "pending", "verify_passed": null, "attempt": 0, "outputs": [] },
    { "id": "deploy", "status": "pending", "verify_passed": null, "attempt": 0, "outputs": [] }
  ]
}
```

Use `status` to understand pipeline state after context recovery or before resuming work.
When pipeline completes, `status` is `"completed"` and `current_phase` is `null`.
```

Add a decision rule row:

```markdown
| Need pipeline state overview | Run `status`. Use for context recovery, progress checks. |
```

- [ ] **Step 2: Commit**

```bash
git add skills/belt-agent/SKILL.md
git commit -m "docs: update belt-agent skill with enriched status output (BELT-29)"
```

---

### Task 11: Update README.md

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add status section to README.md**

After the "Agent loop" section (after line 69), add a new section:

```markdown
### Status

Check pipeline state at any time:

```bash
belt-agent status                  # latest run
belt-agent status --run <run_id>   # specific run
```

Returns an enriched view assembled from run state, pipeline YAML, and output
directories — enough for a new LLM session to resume work without prior context:

```json
{
  "status": "in_progress",
  "current_phase": "review",
  "progress": { "completed": 2, "skipped": 0, "remaining": 2, "total": 4 },
  "phases": [
    { "id": "build", "status": "completed", "verify_passed": true, "attempt": 1, "outputs": ["report.json"] },
    { "id": "review", "status": "current", "verify_passed": false, "attempt": 2, "outputs": [] },
    { "id": "test", "status": "pending", "verify_passed": null, "attempt": 0, "outputs": [] },
    { "id": "deploy", "status": "pending", "verify_passed": null, "attempt": 0, "outputs": [] }
  ]
}
```
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add status command section to README (BELT-29)"
```

---

### Task 12: Final verification

- [ ] **Step 1: Run full workspace tests**

Run: `cargo test --workspace`
Expected: all tests pass

- [ ] **Step 2: Run full workspace clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Run format check**

Run: `cargo fmt --check --package belt-core --package belt-agent`
Expected: no formatting issues

- [ ] **Step 4: Manual smoke test**

```bash
# Create a test pipeline
cat > /tmp/test-pipeline.yml <<'EOF'
name: smoke
version: 1
phases:
  - id: build
    description: "Build"
    gate:
      - cmd: "true"
  - id: test
    description: "Test"
  - id: deploy
    description: "Deploy"
EOF

# Full lifecycle
cargo run -p belt-agent -- init /tmp/test-pipeline.yml
cargo run -p belt-agent -- status
cargo run -p belt-agent -- verify
cargo run -p belt-agent -- status
cargo run -p belt-agent -- step
cargo run -p belt-agent -- status
```

Verify:
- `status` after init: `current_phase` = "build", all 3 phases visible, progress 0/3
- `status` after verify: `verify_passed` = true on build
- `status` after step: `current_phase` = "test", build = completed

- [ ] **Step 5: Clean up temp files**

```bash
rm /tmp/test-pipeline.yml
```
