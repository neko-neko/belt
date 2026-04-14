use crate::error::BeltResult;
use crate::expander::expand_pipeline;
use crate::model::{ArtifactRef, GateCheck, Invoker, Pipeline, ValidationSource};
use crate::parser::parse_pipeline;
use std::collections::{HashMap, HashSet};
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
    // A phase is a leaf unless it delegates to a sub-pipeline via
    // `invoke: { pipeline: ... }`. Other `Invoker` variants (`Skill`, `Agent`,
    // `Agents`) execute at the current phase and still need a human-readable
    // description for `belt-agent status` / `belt-agent next` output.
    for phase in &pipeline.phases {
        let delegates_to_sub_pipeline = matches!(phase.invoke, Some(Invoker::Pipeline { .. }));
        if !delegates_to_sub_pipeline && phase.description.is_none() {
            diagnostics.push(LintDiagnostic {
                severity: Severity::Error,
                message: format!("phase '{}': leaf phase must have a description", phase.id),
            });
        }
    }

    // Check: a phase must do at least one of: invoke, gate, validate, confirm.
    // Completely empty phases are almost always authoring mistakes (spec DD-8).
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    for phase in &pipeline.phases {
        check_empty_phase(phase, &mut diagnostics);
    }

    // Check: gate uses: references exist
    check_gate_uses_exist(&pipeline, base_dir, &mut diagnostics);

    // Check: invoke.pipeline references exist
    check_invoke_pipeline_exists(&pipeline, base_dir, &mut diagnostics);

    // Check: invoke.skill has leading slash
    check_invoke_skill_format(&pipeline, &mut diagnostics);

    // Check: produces uniqueness per phase + consumes resolves to earlier phase
    check_artifact_flow(&pipeline, &mut diagnostics);

    // Check: validate file references exist on disk
    check_validate_file_refs(&pipeline, base_dir, &mut diagnostics);

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

/// Reject phases that have no action, no verification, and no interaction.
/// A phase must do at least one of: invoke something, run a gate, declare
/// a validate criterion, or require confirmation. Completely empty phases
/// are almost always authoring mistakes (spec DD-8).
fn check_empty_phase(phase: &crate::model::Phase, diagnostics: &mut Vec<LintDiagnostic>) {
    let has_action = phase.invoke.is_some();
    let has_verification = !phase.gate.is_empty() || !phase.validate.is_empty();
    let has_interaction = phase.confirm;
    if !has_action && !has_verification && !has_interaction {
        diagnostics.push(LintDiagnostic {
            severity: Severity::Error,
            message: format!(
                "phase '{}' has neither invoke, gate, validate, nor confirm — add at least one",
                phase.id
            ),
        });
    }
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

/// Lint rule: `invoke: { skill: ... }` must be non-empty and start with a
/// leading slash (skill invocation convention).
fn check_invoke_skill_format(pipeline: &Pipeline, diagnostics: &mut Vec<LintDiagnostic>) {
    for phase in &pipeline.phases {
        if let Some(Invoker::Skill { skill, .. }) = &phase.invoke {
            if skill.is_empty() {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Error,
                    message: format!("phase '{}': invoke skill is empty", phase.id),
                });
            } else if !skill.starts_with('/') {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Error,
                    message: format!(
                        "phase '{}': invoke skill '{}' must start with a leading slash (e.g. '/{}')",
                        phase.id, skill, skill
                    ),
                });
            }
        }
    }
}

/// Lint rule: `validate: { file: ... }` references must resolve to existing
/// files (resolved relative to the pipeline YAML's directory). Inline
/// variants are ignored — they are orchestrator-evaluated strings, not
/// filesystem references.
fn check_validate_file_refs(
    pipeline: &Pipeline,
    base_dir: &Path,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for phase in &pipeline.phases {
        for v in &phase.validate {
            if let ValidationSource::File { file } = v {
                let resolved = base_dir.join(file);
                if !resolved.exists() {
                    diagnostics.push(LintDiagnostic {
                        severity: Severity::Error,
                        message: format!(
                            "phase '{}': validate file '{}' not found",
                            phase.id, file
                        ),
                    });
                }
            }
        }
    }
}

/// Lint rule: `produces:` names must be unique within a phase, and every
/// `consumes:` reference must resolve to an artifact produced by an earlier
/// phase (Named → any earlier phase; Qualified → specific earlier `phase_id`).
fn check_artifact_flow(pipeline: &Pipeline, diagnostics: &mut Vec<LintDiagnostic>) {
    // Check: produces names are unique within a phase
    for phase in &pipeline.phases {
        let mut seen_names: HashSet<&str> = HashSet::new();
        for artifact in &phase.produces {
            if !seen_names.insert(artifact.name.as_str()) {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Error,
                    message: format!(
                        "phase '{}': duplicate produces name '{}'",
                        phase.id, artifact.name
                    ),
                });
            }
        }
    }

    // Build index of (name → Vec<(phase_index, phase_id)>) for all produces.
    let mut produces_index: HashMap<String, Vec<(usize, String)>> = HashMap::new();
    for (i, phase) in pipeline.phases.iter().enumerate() {
        for artifact in &phase.produces {
            produces_index
                .entry(artifact.name.clone())
                .or_default()
                .push((i, phase.id.clone()));
        }
    }

    // Check: consumes references resolve to an earlier phase's produces
    for (i, phase) in pipeline.phases.iter().enumerate() {
        for consumed in &phase.consumes {
            match consumed {
                ArtifactRef::Named(name) => {
                    let found = produces_index
                        .get(name)
                        .is_some_and(|locs| locs.iter().any(|(j, _)| *j < i));
                    if !found {
                        diagnostics.push(LintDiagnostic {
                            severity: Severity::Error,
                            message: format!(
                                "phase '{}': consumes '{}' not produced by any earlier phase",
                                phase.id, name
                            ),
                        });
                    }
                }
                ArtifactRef::Qualified { name, from } => {
                    let found = produces_index.get(name).is_some_and(|locs| {
                        locs.iter().any(|(j, phase_id)| phase_id == from && *j < i)
                    });
                    if !found {
                        diagnostics.push(LintDiagnostic {
                            severity: Severity::Error,
                            message: format!(
                                "phase '{}': consumes {{name: '{}', from: '{}'}} not found in earlier phases",
                                phase.id, name, from
                            ),
                        });
                    }
                }
                ArtifactRef::External { .. } => {
                    // External refs are resolved at init time by belt-agent
                    // against a different run's state.json, so there is no
                    // earlier-phase produces to check here. URI grammar
                    // validation is owned by a dedicated lint rule.
                }
            }
        }
    }
}
