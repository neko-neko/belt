use crate::gate::GateResult;
use crate::model::{Artifact, ArtifactRef, Invoker, RunState};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;
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
    /// Artifacts the phase is expected to produce, with runtime-resolved
    /// filesystem state (BELT-32 Plan B). `None` when the phase declares
    /// no `produces:` entries; omitted from JSON output in that case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub produces: Option<Vec<ResolvedArtifact>>,
    /// References to artifacts produced by earlier phases (BELT-32).
    /// Omitted from JSON output when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consumes: Vec<ArtifactRef>,
}

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

/// Resolve a single [`Artifact`] to its runtime filesystem state.
///
/// - Concrete paths use [`std::fs::metadata`] for existence.
/// - Glob paths (containing `*`, `?`, or `[`) are enumerated via
///   [`glob::glob`], filtered by `mtime >= phase_start` when `phase_start`
///   is provided, and the newest match is picked (alphabetically smallest
///   filename on mtime ties).
/// - Returns `exists: false, resolved_path: None` on zero matches or
///   invalid glob syntax.
fn resolve_artifact(artifact: &Artifact, phase_start: Option<DateTime<Utc>>) -> ResolvedArtifact {
    let is_glob =
        artifact.path.contains('*') || artifact.path.contains('?') || artifact.path.contains('[');

    let is_uri = artifact.path.starts_with("belt://");
    let (exists, resolved_path) = if is_glob {
        let Ok(entries) = glob::glob(&artifact.path) else {
            return ResolvedArtifact {
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
                exists: false,
                resolved_path: None,
            };
        };
        let mut candidates: Vec<(std::time::SystemTime, std::path::PathBuf)> = Vec::new();
        for entry in entries.flatten() {
            let Ok(meta) = std::fs::metadata(&entry) else {
                continue;
            };
            let Ok(mtime) = meta.modified() else {
                continue;
            };
            if let Some(start) = phase_start {
                let mtime_dt = DateTime::<Utc>::from(mtime);
                if mtime_dt < start {
                    continue;
                }
            }
            candidates.push((mtime, entry));
        }
        if candidates.is_empty() {
            (false, None)
        } else {
            // Sort by (newest mtime first, ascending filename on ties).
            candidates.sort_by(|a, b| match b.0.cmp(&a.0) {
                std::cmp::Ordering::Equal => a.1.cmp(&b.1),
                non_equal => non_equal,
            });
            // Safe: candidates is non-empty; sort does not change length.
            let path = candidates
                .into_iter()
                .next()
                .map(|(_, p)| p)
                .unwrap_or_default();
            (true, Some(path.to_string_lossy().to_string()))
        }
    } else {
        let exists = std::fs::metadata(&artifact.path).is_ok();
        let resolved = if exists {
            Some(artifact.path.clone())
        } else {
            None
        };
        (exists, resolved)
    };

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
}

/// Evaluates an [`Artifact::when`] expression.
///
/// Grammar (MVP, debug-flow refresh spec, 2026-04-15):
/// - `None` (no `when:` clause) → `true` (artifact always active)
/// - `"args.<flag>"` → true iff `args[<flag>]` is `Value::Bool(true)`
///
/// Unsupported expressions, undefined arg references, and non-bool
/// arg values all evaluate to `false`. This mirrors the conservative
/// "if unsure, omit" stance chosen for status JSON: only the explicit
/// `args.<flag>` form may activate a conditional artifact, so typos
/// and future grammar extensions fail closed rather than leaking
/// phantom artifacts into the view.
#[must_use]
pub fn evaluate_when<S: BuildHasher>(
    when: Option<&str>,
    args: &HashMap<String, serde_json::Value, S>,
) -> bool {
    let Some(expr) = when else {
        return true;
    };
    let expr = expr.trim();
    let Some(arg_name) = expr.strip_prefix("args.") else {
        return false;
    };
    args.get(arg_name)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

/// Returns the subset of `phase.produces` whose [`Artifact::when`]
/// expression evaluates to true under `args` (or which have no
/// `when:` clause).
///
/// Callers use this to filter conditional artifacts out of the
/// status JSON before runtime resolution, so that flags like
/// `--e2e=false` do not surface artifacts that the run will never
/// produce.
#[must_use]
pub fn active_produces<'a, S: BuildHasher>(
    phase: &'a crate::model::ExpandedPhase,
    args: &HashMap<String, serde_json::Value, S>,
) -> Vec<&'a Artifact> {
    phase
        .produces
        .iter()
        .filter(|artifact| evaluate_when(artifact.when.as_deref(), args))
        .collect()
}

/// Resolve the `produces` list for a phase, or return `None` when the
/// phase declares no artifacts. Keeping this `None`-vs-`Some(empty)`
/// distinction mirrors the Plan A serialization shape (absent vs empty
/// list both collapse to "omitted" via `skip_serializing_if`, but
/// `Option` semantics are clearer for consumers).
///
/// `args` is used to filter conditional artifacts via [`evaluate_when`]
/// before resolving filesystem state. An empty `args` map treats every
/// `when:` clause as false (undefined flag → conservative omission).
fn resolve_produces<S: BuildHasher>(
    artifacts: &[Artifact],
    phase_start: Option<DateTime<Utc>>,
    args: &HashMap<String, serde_json::Value, S>,
) -> Option<Vec<ResolvedArtifact>> {
    let active: Vec<&Artifact> = artifacts
        .iter()
        .filter(|a| evaluate_when(a.when.as_deref(), args))
        .collect();
    if active.is_empty() {
        None
    } else {
        Some(
            active
                .into_iter()
                .map(|a| resolve_artifact(a, phase_start))
                .collect(),
        )
    }
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
            let phase_start = state.phase_start_times.get(&meta.id).copied();
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
                produces: resolve_produces(&meta.produces, phase_start, &state.args),
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
                produces: None,
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
                produces: None,
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
