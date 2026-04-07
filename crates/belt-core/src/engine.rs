use crate::error::{BeltError, BeltResult};
use crate::expander::expand_pipeline;
use crate::model::{ExpandedPhase, RunState};
use std::cmp::Reverse;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// State machine engine that manages pipeline execution runs.
///
/// Each run is persisted under `{belt_dir}/runs/{run_id}/state.json`.
/// Phase output directories are created under the run directory.
#[derive(Debug)]
pub struct Engine {
    belt_dir: PathBuf,
}

impl Engine {
    #[must_use]
    pub fn new(belt_dir: &Path) -> Self {
        Self {
            belt_dir: belt_dir.to_path_buf(),
        }
    }

    /// Initialize a new pipeline run.
    ///
    /// Parses and expands the pipeline, finds the first active phase
    /// (evaluating `when:` conditions against `args`), creates a `RunState`,
    /// persists it, and creates the output directory for the first phase.
    pub fn init(
        &self,
        pipeline_path: &Path,
        args: &HashMap<String, serde_json::Value>,
    ) -> BeltResult<RunState> {
        let pipeline = crate::parser::parse_pipeline(pipeline_path)?;
        let phases = expand_pipeline(pipeline_path)?;

        let active =
            first_active_phase(&phases, args).ok_or_else(|| BeltError::InvalidPipeline {
                message: "no active phases in pipeline".to_string(),
            })?;

        let run_id = Uuid::now_v7().to_string();
        let run_dir = self.belt_dir.join("runs").join(&run_id);
        std::fs::create_dir_all(&run_dir)?;

        let now = now_iso8601();
        let state = RunState {
            run_id,
            pipeline: pipeline.name,
            pipeline_file: std::fs::canonicalize(pipeline_path)?.display().to_string(),
            version: pipeline.version,
            args: args.clone(),
            current_phase: active.id.clone(),
            completed_phases: Vec::new(),
            skipped_phases: Vec::new(),
            phase_attempts: HashMap::new(),
            phase_verify_passed: if active.gate.is_empty() {
                HashMap::from([(active.id.clone(), true)])
            } else {
                HashMap::new()
            },
            regate_passed: HashMap::new(),
            created_at: now.clone(),
            updated_at: now,
        };

        self.save_state(&state)?;

        // Create output_dir for first phase
        let output_dir = run_dir.join(active.id.replace('/', "_"));
        std::fs::create_dir_all(&output_dir)?;

        Ok(state)
    }

    /// Load persisted `RunState` for a given run ID.
    pub fn load_state(&self, run_id: &str) -> BeltResult<RunState> {
        let state_path = self.belt_dir.join("runs").join(run_id).join("state.json");
        let content = std::fs::read_to_string(&state_path).map_err(|_| BeltError::State {
            message: format!("run not found: {run_id}"),
        })?;
        serde_json::from_str(&content).map_err(|e| BeltError::State {
            message: format!("invalid state: {e}"),
        })
    }

    /// Persist the given `RunState` to disk.
    pub fn save_state(&self, state: &RunState) -> BeltResult<()> {
        let run_dir = self.belt_dir.join("runs").join(&state.run_id);
        std::fs::create_dir_all(&run_dir)?;
        let state_path = run_dir.join("state.json");
        let json = serde_json::to_string_pretty(state).map_err(|e| BeltError::State {
            message: e.to_string(),
        })?;
        std::fs::write(&state_path, json)?;
        Ok(())
    }

    /// Return the current `ExpandedPhase` with `output_dir` set.
    pub fn next_phase_info(
        &self,
        state: &RunState,
        pipeline_path: &Path,
    ) -> BeltResult<ExpandedPhase> {
        let phases = expand_pipeline(pipeline_path)?;
        let mut phase = phases
            .into_iter()
            .find(|p| p.id == state.current_phase)
            .ok_or_else(|| BeltError::State {
                message: format!(
                    "current phase '{}' not found in pipeline",
                    state.current_phase
                ),
            })?;

        // Set output_dir
        let run_dir = self.belt_dir.join("runs").join(&state.run_id);
        let output_dir = run_dir.join(phase.id.replace('/', "_"));
        std::fs::create_dir_all(&output_dir)?;
        phase.output_dir = Some(output_dir.display().to_string());

        Ok(phase)
    }

    /// Advance to the next active phase.
    ///
    /// Marks the current phase as completed, skips phases whose `when:`
    /// evaluates to false, and returns the ID of the next active phase
    /// (or `None` if the pipeline is complete).
    pub fn step(&self, state: &mut RunState, pipeline_path: &Path) -> BeltResult<Option<String>> {
        let phases = expand_pipeline(pipeline_path)?;

        // Find current phase index
        let current_idx = phases
            .iter()
            .position(|p| p.id == state.current_phase)
            .ok_or_else(|| BeltError::State {
                message: format!("current phase '{}' not found", state.current_phase),
            })?;

        // Guard: verify-before-step
        if state.phase_verify_passed.get(&state.current_phase) != Some(&true) {
            return Err(BeltError::VerifyRequired {
                phase_id: state.current_phase.clone(),
            });
        }

        // Guard: regate-before-step
        let current_phase_def = &phases[current_idx];
        if !current_phase_def.regate.is_empty() {
            match state.regate_passed.get(&state.current_phase) {
                Some(true) => {}
                Some(false) => {
                    return Err(BeltError::RegateFailed {
                        phase_id: state.current_phase.clone(),
                        targets: current_phase_def.regate.clone(),
                    });
                }
                None => {
                    return Err(BeltError::RegateRequired {
                        phase_id: state.current_phase.clone(),
                        targets: current_phase_def.regate.clone(),
                    });
                }
            }
        }

        // Guard: max_retries
        if current_phase_def.max_retries > 0 {
            let attempts = state
                .phase_attempts
                .get(&state.current_phase)
                .copied()
                .unwrap_or(0);
            if attempts > current_phase_def.max_retries {
                return Err(BeltError::MaxRetriesExceeded {
                    phase_id: state.current_phase.clone(),
                    attempts,
                    max_retries: current_phase_def.max_retries,
                });
            }
        }

        // Mark current as completed
        state.completed_phases.push(state.current_phase.clone());

        // Find next active phase, skipping those whose `when:` is false
        let mut next_phase = None;
        let mut next_gate_empty = false;
        for phase in &phases[current_idx + 1..] {
            if eval_when(phase.when.as_ref(), &state.args) {
                next_phase = Some(phase.id.clone());
                next_gate_empty = phase.gate.is_empty();
                break;
            }
            state.skipped_phases.push(phase.id.clone());
        }

        match &next_phase {
            Some(next_id) => {
                state.current_phase.clone_from(next_id);
                // Create output_dir for next phase
                let run_dir = self.belt_dir.join("runs").join(&state.run_id);
                let output_dir = run_dir.join(next_id.replace('/', "_"));
                std::fs::create_dir_all(&output_dir)?;
                // Auto-set verify for gate-less next phase
                if next_gate_empty {
                    state.phase_verify_passed.insert(next_id.clone(), true);
                }
            }
            None => {
                // Pipeline complete
                state.current_phase = "COMPLETED".to_string();
            }
        }

        state.updated_at = now_iso8601();
        self.save_state(state)?;

        Ok(next_phase)
    }

    /// Record a gate verification result.
    ///
    /// Increments the attempt counter for the current phase and returns
    /// the `passed` value unchanged. Clears any existing `regate_passed`
    /// entry for the current phase so that re-verification forces re-regate.
    pub fn verify_verdict(&self, state: &mut RunState, passed: bool) -> BeltResult<bool> {
        let count = state
            .phase_attempts
            .entry(state.current_phase.clone())
            .or_insert(0);
        *count += 1;
        state
            .phase_verify_passed
            .insert(state.current_phase.clone(), passed);
        state.regate_passed.remove(&state.current_phase);
        state.updated_at = now_iso8601();
        self.save_state(state)?;
        Ok(passed)
    }

    /// Record the result of regate gate checks for the current phase.
    pub fn record_regate(&self, state: &mut RunState, all_passed: bool) -> BeltResult<()> {
        state
            .regate_passed
            .insert(state.current_phase.clone(), all_passed);
        state.updated_at = now_iso8601();
        self.save_state(state)?;
        Ok(())
    }

    /// Return the current `RunState` (alias for `load_state`).
    pub fn status(&self, run_id: &str) -> BeltResult<RunState> {
        self.load_state(run_id)
    }

    /// Find the most recent run ID by lexicographic ordering.
    ///
    /// Works because `UUIDv7` embeds a monotonic timestamp prefix,
    /// so lexicographic order matches chronological order.
    pub fn latest_run_id(&self) -> BeltResult<String> {
        let runs_dir = self.belt_dir.join("runs");
        if !runs_dir.exists() {
            return Err(BeltError::State {
                message: "no runs found".to_string(),
            });
        }

        let mut entries: Vec<_> = std::fs::read_dir(&runs_dir)?
            .filter_map(std::result::Result::ok)
            .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            .collect();

        // Sort descending by name (UUIDv7 is time-ordered)
        entries.sort_by_key(|e| Reverse(e.file_name()));

        entries
            .first()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .ok_or_else(|| BeltError::State {
                message: "no runs found".to_string(),
            })
    }
}

/// Evaluate a `when:` expression against the provided args.
///
/// Supported forms:
/// - `None` -> always true (no condition)
/// - `"args.foo"` -> true if `args["foo"]` is truthy (bool `true`, or present and non-null)
/// - `"!args.foo"` -> negated
fn eval_when(when: Option<&String>, args: &HashMap<String, serde_json::Value>) -> bool {
    let Some(expr) = when else { return true };
    let (negated, arg_name) = if let Some(stripped) = expr.strip_prefix('!') {
        (true, stripped.trim_start_matches("args."))
    } else {
        (false, expr.trim_start_matches("args."))
    };

    let val = args.get(arg_name).is_some_and(|v| match v {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Null => false,
        _ => true,
    });

    if negated { !val } else { val }
}

/// Find the first phase whose `when:` evaluates to true.
fn first_active_phase<'a>(
    phases: &'a [ExpandedPhase],
    args: &HashMap<String, serde_json::Value>,
) -> Option<&'a ExpandedPhase> {
    phases.iter().find(|p| eval_when(p.when.as_ref(), args))
}

/// Produce an ISO 8601 UTC timestamp (`YYYY-MM-DDTHH:MM:SSZ`).
///
/// Uses `SystemTime` + Hinnant's civil-from-days algorithm to avoid
/// pulling in `chrono` or `time` crates.
fn now_iso8601() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    epoch_to_iso8601(dur.as_secs())
}

/// Convert Unix epoch seconds to an ISO 8601 UTC string.
///
/// Algorithm: Howard Hinnant's `civil_from_days` for the date portion,
/// simple modular arithmetic for the time portion.
fn epoch_to_iso8601(secs: u64) -> String {
    // Safe: secs/86400 fits in i64 for any realistic timestamp (up to year ~25 billion)
    #[allow(clippy::cast_possible_wrap)]
    let days = (secs / 86_400) as i64;
    let time_of_day = secs % 86_400;

    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Hinnant's civil_from_days (epoch = 1970-01-01)
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_when_none_is_true() {
        let args = HashMap::new();
        assert!(eval_when(None, &args));
    }

    #[test]
    fn eval_when_truthy_bool() {
        let mut args = HashMap::new();
        args.insert("smoke".to_string(), serde_json::Value::Bool(true));
        let expr = "args.smoke".to_string();
        assert!(eval_when(Some(&expr), &args));
    }

    #[test]
    fn eval_when_falsy_bool() {
        let mut args = HashMap::new();
        args.insert("smoke".to_string(), serde_json::Value::Bool(false));
        let expr = "args.smoke".to_string();
        assert!(!eval_when(Some(&expr), &args));
    }

    #[test]
    fn eval_when_negated() {
        let mut args = HashMap::new();
        args.insert("smoke".to_string(), serde_json::Value::Bool(false));
        let expr = "!args.smoke".to_string();
        assert!(eval_when(Some(&expr), &args));
    }

    #[test]
    fn eval_when_missing_arg_is_false() {
        let args = HashMap::new();
        let expr = "args.missing".to_string();
        assert!(!eval_when(Some(&expr), &args));
    }

    #[test]
    fn eval_when_null_is_false() {
        let mut args = HashMap::new();
        args.insert("x".to_string(), serde_json::Value::Null);
        let expr = "args.x".to_string();
        assert!(!eval_when(Some(&expr), &args));
    }

    #[test]
    fn eval_when_string_value_is_truthy() {
        let mut args = HashMap::new();
        args.insert(
            "name".to_string(),
            serde_json::Value::String("hello".to_string()),
        );
        let expr = "args.name".to_string();
        assert!(eval_when(Some(&expr), &args));
    }

    #[test]
    fn epoch_to_iso8601_unix_epoch() {
        assert_eq!(epoch_to_iso8601(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn epoch_to_iso8601_known_date() {
        // 2024-01-01T00:00:00Z = 1704067200
        assert_eq!(epoch_to_iso8601(1_704_067_200), "2024-01-01T00:00:00Z");
    }

    #[test]
    fn epoch_to_iso8601_with_time() {
        // 2023-06-15T14:30:45Z = 1686836445 (but let me use a verified value)
        // 2000-02-29T23:59:59Z (leap year) = 951868799
        assert_eq!(epoch_to_iso8601(951_868_799), "2000-02-29T23:59:59Z");
    }

    #[test]
    fn epoch_to_iso8601_leap_second_boundary() {
        // 2000-03-01T00:00:00Z = 951868800
        assert_eq!(epoch_to_iso8601(951_868_800), "2000-03-01T00:00:00Z");
    }

    #[test]
    fn now_iso8601_format() {
        let ts = now_iso8601();
        // Must match YYYY-MM-DDTHH:MM:SSZ pattern
        assert_eq!(ts.len(), 20, "timestamp length: {ts}");
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[13..14], ":");
        assert_eq!(&ts[16..17], ":");
        assert_eq!(&ts[19..], "Z");
    }
}
