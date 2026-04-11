use crate::error::BeltResult;
use crate::expander::expand_pipeline;
use crate::model::{GateCheck, Invoker, Pipeline};
use crate::parser::parse_pipeline;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct LintDiagnostic {
    pub severity: Severity,
    pub message: String,
}

/// Lint a pipeline YAML file and return a list of diagnostics.
///
/// The linter runs in two phases:
///
/// 1. **Static checks** on the parsed `Pipeline` model (duplicate IDs, dangling
///    references, missing descriptions, file existence).
/// 2. **Expansion** via [`expand_pipeline`] — only attempted when phase 1 produced
///    no errors — to catch issues in referenced sub-pipelines.
pub fn lint_pipeline(path: &Path) -> BeltResult<Vec<LintDiagnostic>> {
    let mut diagnostics = Vec::new();

    // Phase 1: Parse
    let pipeline = match parse_pipeline(path) {
        Ok(p) => p,
        Err(e) => {
            diagnostics.push(LintDiagnostic {
                severity: Severity::Error,
                message: format!("parse error: {e}"),
            });
            return Ok(diagnostics);
        }
    };

    // Check: duplicate phase IDs
    let mut seen_ids = HashSet::new();
    for phase in &pipeline.phases {
        if !seen_ids.insert(&phase.id) {
            diagnostics.push(LintDiagnostic {
                severity: Severity::Error,
                message: format!("duplicate phase id: '{}'", phase.id),
            });
        }
    }

    // Check: regate references valid phase IDs
    let all_ids: HashSet<&str> = pipeline.phases.iter().map(|p| p.id.as_str()).collect();
    for phase in &pipeline.phases {
        for regate_target in &phase.regate {
            if !all_ids.contains(regate_target.as_str()) {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Error,
                    message: format!(
                        "phase '{}': regate target '{}' does not exist",
                        phase.id, regate_target
                    ),
                });
            }
        }
    }

    // Check: when references valid args
    for phase in &pipeline.phases {
        if let Some(when) = &phase.when {
            let arg_name = when.trim_start_matches('!').trim_start_matches("args.");
            if !arg_name.is_empty() && !pipeline.args.contains_key(arg_name) {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Error,
                    message: format!(
                        "phase '{}': when references undefined arg '{}'",
                        phase.id, arg_name
                    ),
                });
            }
        }
    }

    // Check: leaf phase must have description.
    // A phase is a leaf unless it delegates to a sub-pipeline via `uses:` or
    // `invoke: { pipeline: ... }`. Other `Invoker` variants (`Skill`, `Agent`,
    // `Agents`) execute at the current phase and still need a human-readable
    // description for `belt-agent status` / `belt-agent next` output.
    for phase in &pipeline.phases {
        let delegates_to_sub_pipeline =
            phase.uses.is_some() || matches!(phase.invoke, Some(Invoker::Pipeline { .. }));
        if !delegates_to_sub_pipeline && phase.description.is_none() {
            diagnostics.push(LintDiagnostic {
                severity: Severity::Error,
                message: format!("phase '{}': leaf phase must have a description", phase.id),
            });
        }
    }

    // Check: uses: references exist
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    for phase in &pipeline.phases {
        if let Some(uses) = &phase.uses {
            let resolved = base_dir.join(uses);
            if !resolved.exists() {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Error,
                    message: format!("phase '{}': uses '{}' not found", phase.id, uses),
                });
            }
        }
    }

    // Check: gate uses: references exist
    check_gate_uses_exist(&pipeline, base_dir, &mut diagnostics);

    // Check: invoke.pipeline references exist
    check_invoke_pipeline_exists(&pipeline, base_dir, &mut diagnostics);

    // Phase 2: Try expansion (catches issues in sub-pipelines)
    if diagnostics.iter().all(|d| d.severity != Severity::Error) {
        if let Err(e) = expand_pipeline(path) {
            diagnostics.push(LintDiagnostic {
                severity: Severity::Error,
                message: format!("expansion error: {e}"),
            });
        }
    }

    Ok(diagnostics)
}

/// Verify that every `gate: uses:` reference points to an existing file on disk.
fn check_gate_uses_exist(
    pipeline: &Pipeline,
    base_dir: &Path,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for phase in &pipeline.phases {
        for check in &phase.gate {
            if let GateCheck::Uses { uses, .. } = check {
                let resolved = base_dir.join(uses);
                if !resolved.exists() {
                    diagnostics.push(LintDiagnostic {
                        severity: Severity::Error,
                        message: format!("phase '{}': gate uses '{}' not found", phase.id, uses),
                    });
                }
            }
        }
    }
}

/// Verify that every `phase.invoke.pipeline` reference points to an existing
/// file on disk. Other `Invoker` variants (`Skill`, `Agent`, `Agents`) are not
/// path-like and are checked elsewhere.
fn check_invoke_pipeline_exists(
    pipeline: &Pipeline,
    base_dir: &Path,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for phase in &pipeline.phases {
        if let Some(Invoker::Pipeline {
            pipeline: sub_path, ..
        }) = &phase.invoke
        {
            let resolved = base_dir.join(sub_path);
            if !resolved.exists() {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Error,
                    message: format!(
                        "phase '{}': invoke pipeline '{}' not found",
                        phase.id, sub_path
                    ),
                });
            }
        }
    }
}
