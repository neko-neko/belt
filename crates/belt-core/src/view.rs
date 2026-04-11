use crate::gate::GateResult;
use crate::model::{Artifact, ArtifactRef, Invoker, RunState};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Subset of `ExpandedPhase` fields used by [`build_status_view`] to enrich
/// the per-phase view. Callers (engine) pass this alongside `RunState` to
/// produce a [`StatusView`] with typed invoke / produces / consumes data,
/// decoupling view construction from the full `ExpandedPhase` type.
#[derive(Debug, Clone)]
pub struct PhaseMetadata {
    pub id: String,
    pub invoke: Option<Invoker>,
    pub produces: Vec<Artifact>,
    pub consumes: Vec<ArtifactRef>,
}

/// Enriched status view of a pipeline run, combining `RunState` data
/// with phase output scanning and computed progress.
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

/// Overall pipeline execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    InProgress,
    Completed,
}

/// Numeric progress summary across all phases.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub completed: usize,
    pub skipped: usize,
    pub remaining: usize,
    pub total: usize,
}

/// Per-phase enriched view.
#[derive(Debug, Serialize)]
pub struct PhaseView {
    pub id: String,
    pub status: PhaseState,
    pub verify_passed: Option<bool>,
    pub regate_passed: Option<bool>,
    pub attempt: u32,
    pub outputs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify_checks: Option<Vec<GateResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regate_checks: Option<serde_json::Value>,
    /// Typed invocation target declared by the phase (BELT-32).
    /// Omitted from JSON output when `None` to preserve backwards
    /// compatibility with legacy pipelines that do not declare `invoke:`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoke: Option<Invoker>,
    /// Artifacts the phase is expected to produce (BELT-32).
    /// Omitted from JSON output when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub produces: Vec<Artifact>,
    /// References to artifacts produced by earlier phases (BELT-32).
    /// Omitted from JSON output when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consumes: Vec<ArtifactRef>,
}

/// State of an individual phase within the pipeline run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseState {
    Completed,
    Current,
    Pending,
    Skipped,
}

const COMPLETED_SENTINEL: &str = "COMPLETED";

/// Scan a phase output directory for top-level file names.
/// Returns sorted file names. Gracefully returns empty vec on any error.
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

fn determine_phase_state(phase_id: &str, state: &RunState) -> PhaseState {
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

/// Read verify per-check results from file. Returns `None` on any error.
fn read_verify_checks(run_dir: &Path, phase_id: &str) -> Option<Vec<GateResult>> {
    let file = run_dir
        .join("verify")
        .join(format!("{}.json", phase_id.replace('/', "_")));
    let content = std::fs::read_to_string(&file).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    let checks = parsed.get("checks")?;
    serde_json::from_value(checks.clone()).ok()
}

/// Read regate targets from file. Returns `None` on any error.
fn read_regate_checks(run_dir: &Path, phase_id: &str) -> Option<serde_json::Value> {
    let file = run_dir
        .join("regate")
        .join(format!("{}.json", phase_id.replace('/', "_")));
    let content = std::fs::read_to_string(&file).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    parsed.get("targets").cloned()
}

/// Build an enriched status view from `RunState` + phase metadata + run directory.
#[must_use]
pub fn build_status_view(
    state: &RunState,
    phases_meta: &[PhaseMetadata],
    run_dir: &Path,
) -> StatusView {
    let is_completed = state.current_phase == COMPLETED_SENTINEL;

    let mut phases: Vec<PhaseView> = phases_meta
        .iter()
        .map(|meta| {
            let status = if is_completed && state.completed_phases.contains(&meta.id) {
                PhaseState::Completed
            } else {
                determine_phase_state(&meta.id, state)
            };
            PhaseView {
                id: meta.id.clone(),
                status,
                verify_passed: state.phase_verify_passed.get(&meta.id).copied(),
                regate_passed: state.regate_passed.get(&meta.id).copied(),
                attempt: state.phase_attempts.get(&meta.id).copied().unwrap_or(0),
                outputs: scan_phase_outputs(run_dir, &meta.id),
                verify_checks: read_verify_checks(run_dir, &meta.id),
                regate_checks: read_regate_checks(run_dir, &meta.id),
                invoke: meta.invoke.clone(),
                produces: meta.produces.clone(),
                consumes: meta.consumes.clone(),
            }
        })
        .collect();

    // Append orphan phases (in state but removed from YAML).
    let yaml_ids: HashSet<&String> = phases_meta.iter().map(|m| &m.id).collect();
    for id in &state.completed_phases {
        if !yaml_ids.contains(id) {
            phases.push(PhaseView {
                id: id.clone(),
                status: PhaseState::Completed,
                verify_passed: state.phase_verify_passed.get(id).copied(),
                regate_passed: state.regate_passed.get(id).copied(),
                attempt: state.phase_attempts.get(id).copied().unwrap_or(0),
                outputs: scan_phase_outputs(run_dir, id),
                verify_checks: read_verify_checks(run_dir, id),
                regate_checks: read_regate_checks(run_dir, id),
                invoke: None,
                produces: Vec::new(),
                consumes: Vec::new(),
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
                verify_checks: None,
                regate_checks: None,
                invoke: None,
                produces: Vec::new(),
                consumes: Vec::new(),
            });
        }
    }

    let completed = phases
        .iter()
        .filter(|p| p.status == PhaseState::Completed)
        .count();
    let skipped = phases
        .iter()
        .filter(|p| p.status == PhaseState::Skipped)
        .count();
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
