# BELT-29: status command enrichment for context-neutral orchestration

## Summary

Enrich `belt-agent status` output with a structured view model so that any consumer (new LLM context, TUI, multi-agent orchestrator) can understand the full pipeline state from a single command.

## Design Philosophy

- **Context-neutral**: status provides enough information for any caller to act without prior context
- **Read-only view computation**: RunState (state.json) is NOT extended. status assembles the view at query time from RunState + pipeline YAML + filesystem
- **Single source of truth view**: All visualization layers (CLI JSON, future TUI, future Web UI) consume the same view model
- **Clear command boundaries**: status = "where are we across all phases", next = "what to do now for current phase", verify = "run gate checks"

## Scope

**In scope:**
- Per-phase ordered list with status labels (completed/current/pending/skipped)
- Per-phase verify_passed / regate_passed (bool only)
- Per-phase attempt count
- Per-phase output file paths (`.belt/runs/{run_id}/{phase_id}/` scan)
- Pipeline-level progress summary
- Pipeline-level status (in_progress/completed)

**Documentation updates:**
- `skills/belt-agent/SKILL.md` — Update status command description with enriched output format
- `README.md` — Update status description to reflect enriched output

**Out of scope:**
- Phase definition info (description, gate definitions, validate, config) — `next` command's responsibility
- Per-check gate results — BELT-30
- `artifacts` field glob resolution — future enhancement
- RunState schema changes — engine untouched

## JSON Output Structure

### In-progress pipeline

```json
{
  "run_id": "019614a8-...",
  "pipeline": "feature-dev",
  "pipeline_file": "/abs/path/pipeline.yml",
  "version": 1,
  "args": { "smoke": true },
  "status": "in_progress",
  "current_phase": "review/triage",
  "progress": {
    "completed": 3,
    "skipped": 1,
    "remaining": 3,
    "total": 7
  },
  "phases": [
    {
      "id": "build",
      "status": "completed",
      "verify_passed": true,
      "regate_passed": null,
      "attempt": 1,
      "outputs": ["report.json", "summary.md"]
    },
    {
      "id": "review/explore",
      "status": "completed",
      "verify_passed": true,
      "regate_passed": null,
      "attempt": 1,
      "outputs": []
    },
    {
      "id": "review/triage",
      "status": "current",
      "verify_passed": false,
      "regate_passed": null,
      "attempt": 2,
      "outputs": []
    },
    {
      "id": "test",
      "status": "skipped",
      "verify_passed": null,
      "regate_passed": null,
      "attempt": 0,
      "outputs": []
    },
    {
      "id": "deploy",
      "status": "pending",
      "verify_passed": null,
      "regate_passed": null,
      "attempt": 0,
      "outputs": []
    }
  ],
  "created_at": "2026-04-07T12:00:00Z",
  "updated_at": "2026-04-07T13:30:00Z"
}
```

### Completed pipeline

```json
{
  "run_id": "019614a8-...",
  "pipeline": "simple-flow",
  "pipeline_file": "/abs/path/pipeline.yml",
  "version": 1,
  "args": {},
  "status": "completed",
  "current_phase": null,
  "progress": {
    "completed": 2,
    "skipped": 1,
    "remaining": 0,
    "total": 3
  },
  "phases": [
    { "id": "build", "status": "completed", "verify_passed": true, "regate_passed": null, "attempt": 1, "outputs": ["artifact.tar.gz"] },
    { "id": "test", "status": "skipped", "verify_passed": null, "regate_passed": null, "attempt": 0, "outputs": [] },
    { "id": "deploy", "status": "completed", "verify_passed": true, "regate_passed": null, "attempt": 1, "outputs": [] }
  ],
  "created_at": "2026-04-07T12:00:00Z",
  "updated_at": "2026-04-07T14:00:00Z"
}
```

## Module Structure

```
belt-core/src/
├── model.rs      # Existing. RunState etc. NO CHANGES
├── engine.rs     # Add enriched_status() method
├── view.rs       # NEW. StatusView types + build logic
└── lib.rs        # Add pub mod view
```

### view.rs — View Types

```rust
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    InProgress,
    Completed,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseState {
    Completed,
    Current,
    Pending,
    Skipped,
}
```

## Logic Flow

`engine.enriched_status(run_id)`:

1. `load_state(run_id)` -> RunState
2. `parse_pipeline(&state.pipeline_file)` -> Pipeline
3. `expand_pipeline(pipeline)` -> `Vec<ExpandedPhase>`
4. For each expanded phase, build `PhaseView`:
   - **status**: `completed_phases` contains id -> Completed; `skipped_phases` contains id -> Skipped; equals `current_phase` -> Current; else -> Pending
   - **verify_passed**: `phase_verify_passed.get(id)` -> `Option<bool>`
   - **regate_passed**: `regate_passed.get(id)` -> `Option<bool>`
   - **attempt**: `phase_attempts.get(id).copied().unwrap_or(0)`
   - **outputs**: scan `.belt/runs/{run_id}/{phase_id}/` for files
5. Compute `Progress` from phase statuses
6. Translate `current_phase == "COMPLETED"` -> `status: Completed`, `current_phase: None`
7. Return `StatusView`

## Output Directory Scanning

- Path: `.belt/runs/{run_id}/{phase_id}/` (sub-pipeline phases nest: `review/triage/`)
- Directory does not exist -> empty array
- Directory exists but empty -> empty array
- List **top-level file names only** (no recursion, no directory entries)
- `read_dir` errors (permission etc.) -> empty array (graceful degradation; status must not break)

## Error Handling

| Condition | Behavior |
|-----------|----------|
| run_id not found | `BeltError::State` (existing) |
| pipeline YAML missing/moved | `BeltError::FileNotFound` with clear message |
| pipeline YAML changed (phase added) | New phase shown as Pending |
| pipeline YAML changed (phase removed) | State-only phase retained as Completed/Skipped |
| output dir read failure | Empty array (graceful) |

### Pipeline YAML Drift

When the pipeline YAML has been modified since the run was created:

- **Phase added in YAML**: appears in `phases` array as `Pending` (not in state, so no history)
- **Phase removed from YAML**: phases recorded in RunState (completed/skipped) are still included in the view, preserving history. They appear after YAML-defined phases, retaining their recorded status.

This ensures status never loses information about work that was done, even if the pipeline definition evolves.

## belt-agent Changes

`cmd_status` in `main.rs` switches from raw RunState dump to enriched view:

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

## Documentation Updates

### skills/belt-agent/SKILL.md

Update the `status` command description. Current description is just "Inspect full run state". Replace with enriched output explanation:

- Document that status returns a structured view with per-phase status, progress, and output paths
- Add a brief output format example showing key fields (status, current_phase, progress, phases)
- Add decision rule: "Use `status` to understand pipeline state after context recovery or before resuming work"

### README.md

Update the Key Concepts or CLI section:

- Mention that `status` returns an enriched view (not raw state)
- Briefly note the per-phase status labels and progress summary

## Test Plan

### A. PhaseState determination (view.rs unit tests)

| # | Case | Input | Expected |
|---|------|-------|----------|
| 1 | Initial state | After init, current = first phase | first = Current, rest = Pending |
| 2 | Partially completed | 3 completed, 1 current | 3 Completed + 1 Current + rest Pending |
| 3 | Fully completed | current = "COMPLETED" | All Completed, pipeline status = completed, current_phase = null |
| 4 | Skipped phases | 2 phases in skipped_phases | Skipped on correct phases |
| 5 | Mixed skip + completed | Some skipped, some completed | Each status accurate |

### B. Verify / regate state (view.rs unit tests)

| # | Case | Input | Expected |
|---|------|-------|----------|
| 6 | verify not run | No key in phase_verify_passed | verify_passed = null |
| 7 | verify PASS | phase_verify_passed[id] = true | verify_passed = true |
| 8 | verify FAIL | phase_verify_passed[id] = false | verify_passed = false |
| 9 | regate PASS | regate_passed[id] = true | regate_passed = true |
| 10 | regate not run | No key in regate_passed | regate_passed = null |

### C. Attempt count (view.rs unit tests)

| # | Case | Input | Expected |
|---|------|-------|----------|
| 11 | First attempt | No key in phase_attempts | attempt = 0 |
| 12 | Retrying | phase_attempts[id] = 3 | attempt = 3 |

### D. Progress calculation (view.rs unit tests)

| # | Case | Input | Expected |
|---|------|-------|----------|
| 13 | Normal progress | 3 completed, 1 skipped, 7 total | {completed: 3, skipped: 1, remaining: 3, total: 7} |
| 14 | All completed | 6 completed, 1 skipped, 7 total | {completed: 6, skipped: 1, remaining: 0, total: 7} |
| 15 | All pending | After init | {completed: 0, skipped: 0, remaining: 7, total: 7} |

### E. Output directory scanning (filesystem tests)

| # | Case | Condition | Expected |
|---|------|-----------|----------|
| 16 | Files present | 2 files in output dir | outputs = ["a.md", "b.json"] |
| 17 | Empty directory | output dir exists, no files | outputs = [] |
| 18 | No directory | output dir not created | outputs = [] |
| 19 | Mixed entries | Files + subdirectories | Files only |
| 20 | Sub-pipeline phase | id = "review/triage" | Scans `.belt/runs/{id}/review/triage/` |

### F. Error / edge cases (engine integration tests)

| # | Case | Condition | Expected |
|---|------|-----------|----------|
| 21 | Pipeline YAML missing | pipeline_file does not exist | FileNotFound error |
| 22 | Pipeline changed (phase added) | New phase in YAML, not in state | New phase = Pending |
| 23 | Pipeline changed (phase removed) | Phase in state, removed from YAML | Phase retained with recorded status |
| 24 | Single phase pipeline | Only 1 phase | phases array length 1, progress total = 1 |
| 25 | Pipeline with args | args stored in state | args field returned correctly |

### G. belt-agent CLI integration tests

| # | Case | Steps | Verify |
|---|------|-------|--------|
| 26 | Status after init | init -> status | Full JSON structure valid, first phase = current |
| 27 | Status after verify | init -> verify -> status | verify_passed reflected |
| 28 | Status after step | init -> verify -> step -> status | completed/current transition correct |
| 29 | Status after completion | Full lifecycle -> status | status = "completed", current_phase = null |
| 30 | --run flag | Multiple runs -> status --run {id} | Correct run returned |

## Future Extensions

- **BELT-30**: verify per-check results persisted to `.belt/runs/{run_id}/verify/{phase_id}.json`, read by status
- **artifacts field**: Resolve `artifacts` globs and include in output
- **TUI consumption**: `belt-tui` renders StatusView directly
- **Filtering**: `--phase <id>` for single-phase detail, `--format compact` for summary only
