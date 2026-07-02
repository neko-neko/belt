use crate::error::{BeltError, BeltResult};
use crate::expander::expand_pipeline;
use crate::model::{ArtifactRef, GateCheck, Invoker, Pipeline, ValidationSource};
use crate::parser::parse_pipeline_from_str;
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

    // Phase 0: Read the YAML source once, then run the raw-YAML lint for
    // obsolete `invoke.agent` / `invoke.agents` / `invoke.iterations` keys
    // BEFORE feeding the typed parser. Without this ordering the user sees
    // the cryptic serde untagged-enum error (e.g. "data did not match any
    // variant of untagged enum Invoker") instead of the migration hint.
    // The string is reused below for typed deserialisation so this is a
    // single read.
    let Ok(content) = std::fs::read_to_string(path) else {
        return Err(BeltError::FileNotFound {
            path: path.display().to_string(),
        });
    };

    // Raw-YAML obsolete-key lint: short-circuits with an Error diagnostic
    // so the migration hint is the first thing the user sees. Further
    // structural lints do not run because the typed parser would fail
    // anyway, and re-reporting the serde error on top of the hint would
    // only add noise.
    if let Err(e) = lint_raw_pipeline_yaml(&content) {
        diagnostics.push(LintDiagnostic {
            severity: Severity::Error,
            message: e.to_string(),
        });
        return Ok(diagnostics);
    }

    // Phase 1: Parse (shared deserialization helper — keeps `BeltError::
    // YamlParse` wrapping and any future parser preflight logic unified
    // between file-path and in-memory entry points).
    let pipeline = match parse_pipeline_from_str(&content) {
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
    // `invoke: { pipeline: ... }`. The `Skill` variant executes at the current
    // phase and still needs a human-readable description for `belt-agent
    // status` / `belt-agent next` output.
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
    check_invoke_pipeline_exists(&pipeline, base_dir, path, &mut diagnostics);

    // Check: invoke.skill has leading slash
    check_invoke_skill_format(&pipeline, &mut diagnostics);

    // Check: produces uniqueness per phase + consumes resolves to earlier phase
    check_artifact_flow(&pipeline, &mut diagnostics);

    // Check: External `consumes` URIs parse as valid belt:// grammar
    check_external_uri_grammar(&pipeline, &mut diagnostics);

    // Check: External `consumes` URIs with Latest/WorkspaceLatest selectors
    // reference a pipeline with a sibling YAML in the current directory.
    // Warning-only: producers can legitimately live outside the repo.
    check_external_uri_sibling_producer(&pipeline, base_dir, &mut diagnostics);

    // Check: each `produces` entry is protected by a gate (file_exists
    // matching the path, or has_output: true). Warning-only: the phase may
    // legitimately write elsewhere, but without gate protection downstream
    // consumers may see missing files at runtime.
    check_produces_protected_by_gate(&pipeline, &mut diagnostics);

    // Check: validate file references exist on disk
    check_validate_file_refs(&pipeline, base_dir, &mut diagnostics);

    // Check: Artifact.when expressions must reference defined args.
    check_artifact_when_references(&pipeline, &mut diagnostics);

    // Check: no `.belt/runs/` literal or `{run_id}` template in pipeline strings.
    check_belt_runs_literal(&pipeline, &mut diagnostics);

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

/// Raw YAML lint — detects obsolete `invoke.agent` / `invoke.agents` /
/// `invoke.iterations` keys that were removed on 2026-04-16.
///
/// This supplements the `Invoker` enum's parse-time rejection by producing
/// a targeted, human-readable diagnostic with a migration hint before the
/// generic "variant did not match any" serde error surfaces.
///
/// Invariants:
/// - If the YAML is malformed, returns `Ok(())` so that the typed parser
///   (or `lint_pipeline`'s phase 1) can produce its own diagnostic rather
///   than emitting a misleading "agent key found" message when the real
///   problem is unrelated YAML syntax.
/// - Only phases whose `invoke:` is a mapping are inspected; scalar or
///   sequence forms are deliberately ignored because they cannot carry the
///   offending keys.
///
/// # Errors
///
/// Returns `BeltError::InvalidPipeline` with a migration hint when any
/// phase's `invoke:` contains `agent`, `agents`, or `iterations`.
pub fn lint_raw_pipeline_yaml(yaml: &str) -> Result<(), BeltError> {
    /// Obsolete `invoke.*` keys removed on 2026-04-16, paired with the
    /// migration hint surfaced after the common "no longer supported" prefix.
    /// Add future retirements here rather than duplicating the match arm.
    const OBSOLETE_INVOKE_KEYS: &[(&str, &str)] = &[
        (
            "agent",
            "Use `invoke.skill: /<plugin>:<skill-name>` where the skill forks a subagent \
             via `context: fork` + `agent:`.",
        ),
        (
            "agents",
            "Dispatch subagents from inside a parent skill via Task tool, and reference \
             the skill from `invoke.skill: /<plugin>:<skill-name>`.",
        ),
        (
            "iterations",
            "N-way voting is not part of the belt pipeline surface.",
        ),
    ];

    // Parse to serde_json::Value first; if the YAML is malformed, fall
    // through and let the typed parser handle it.
    let doc: serde_json::Value = match serde_saphyr::from_str(yaml) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };

    let Some(phases) = doc.get("phases").and_then(|p| p.as_array()) else {
        return Ok(());
    };

    for (idx, phase) in phases.iter().enumerate() {
        let Some(invoke) = phase.get("invoke") else {
            continue;
        };
        let phase_id = phase
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("<unnamed>");
        for (key, hint) in OBSOLETE_INVOKE_KEYS {
            if invoke.get(*key).is_some() {
                return Err(BeltError::InvalidPipeline {
                    message: format!(
                        "phase[{idx}] '{phase_id}': `invoke.{key}` is no longer supported \
                         (removed 2026-04-16). {hint}"
                    ),
                });
            }
        }
    }
    Ok(())
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
/// file on disk — recursively through sub-pipelines. The `Skill` variant is
/// not path-like and is checked elsewhere. Cycles terminate via the visited
/// set (the cycle itself is reported by expansion, not here).
///
/// `visited` is seeded with the root pipeline's own canonical path, mirroring
/// `expand_pipeline`'s `canonical_or_self` seeding. Without this, a
/// self-referencing phase (`invoke: { pipeline: <own file> }`) would cause
/// the root pipeline to be re-traversed once as its own "sub-pipeline",
/// duplicating every other phase's diagnostics under a spurious namespace.
fn check_invoke_pipeline_exists(
    pipeline: &Pipeline,
    base_dir: &Path,
    pipeline_path: &Path,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let mut visited: Vec<std::path::PathBuf> = vec![
        pipeline_path
            .canonicalize()
            .unwrap_or_else(|_| pipeline_path.to_path_buf()),
    ];
    check_phases_invoke_pipeline(&pipeline.phases, base_dir, "", &mut visited, diagnostics);
}

fn check_phases_invoke_pipeline(
    phases: &[crate::model::Phase],
    base_dir: &Path,
    ns: &str,
    visited: &mut Vec<std::path::PathBuf>,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for phase in phases {
        if let Some(Invoker::Pipeline {
            pipeline: sub_path, ..
        }) = &phase.invoke
        {
            let resolved = base_dir.join(sub_path);
            if !resolved.exists() {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Error,
                    message: format!(
                        "phase '{ns}{}': invoke pipeline '{}' not found",
                        phase.id, sub_path
                    ),
                });
                continue;
            }
            let canonical = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
            if visited.contains(&canonical) {
                continue; // cycle: expansion reports it
            }
            if let Ok(sub) = crate::parser::parse_sub_pipeline(&resolved) {
                let sub_base = resolved.parent().map_or_else(
                    || std::path::PathBuf::from("."),
                    std::path::Path::to_path_buf,
                );
                let child_ns = format!("{ns}{}/", phase.id);
                visited.push(canonical);
                check_phases_invoke_pipeline(
                    &sub.phases,
                    &sub_base,
                    &child_ns,
                    visited,
                    diagnostics,
                );
                visited.pop();
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
                    // validation is owned by `check_external_uri_grammar`.
                }
            }
        }
    }
}

/// Lint rule: every `consumes: External { uri }` entry must be a syntactically
/// valid `belt://` URI.
///
/// In practice the YAML deserializer already rejects malformed URIs via
/// `BeltUri::parse` (called from `BeltUri::Deserialize`), so by the time a
/// `Pipeline` is materialized all External refs are grammar-valid. This lint
/// performs a `Display → parse` roundtrip as a defensive, belt-and-suspenders
/// check: if Display ever diverges from parse (future refactor), the lint
/// catches the drift. It also encodes the invariant explicitly so that if
/// `BeltUri::parse` is ever relaxed (e.g., to emit warnings instead of hard
/// errors), the lint continues to flag invalid grammar at authoring time.
fn check_external_uri_grammar(pipeline: &Pipeline, diagnostics: &mut Vec<LintDiagnostic>) {
    for phase in &pipeline.phases {
        for aref in &phase.consumes {
            if let ArtifactRef::External { name, uri } = aref {
                let s = uri.to_string();
                if let Err(e) = crate::uri::BeltUri::parse(&s) {
                    diagnostics.push(LintDiagnostic {
                        severity: Severity::Error,
                        message: format!(
                            "phase '{}': consumes '{}' URI invalid: {e}",
                            phase.id, name
                        ),
                    });
                }
            }
        }
    }
}

/// Lint rule: warn (not error) when a `consumes: External` URI with a
/// `Latest` or `WorkspaceLatest` selector references a pipeline with no
/// sibling YAML in the same directory as the current pipeline file.
///
/// Resolution strategy: look for `<base_dir>/<pipeline>.yml` *or*
/// `<base_dir>/<pipeline>/pipeline.yml`. If neither exists, emit a warning.
///
/// The `Run` variant is intentionally skipped — it targets a specific run
/// UUID and is not expected to have a sibling pipeline definition.
///
/// False positives are acceptable by design (spec intent): producers can
/// legitimately live in a separate repository or be defined elsewhere on
/// disk. Severity is Warning so authors can dismiss per case.
fn check_external_uri_sibling_producer(
    pipeline: &Pipeline,
    base_dir: &Path,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for phase in &pipeline.phases {
        for aref in &phase.consumes {
            if let ArtifactRef::External { name, uri } = aref {
                let producer = match uri {
                    crate::uri::BeltUri::Latest { pipeline: p, .. }
                    | crate::uri::BeltUri::WorkspaceLatest { pipeline: p, .. } => p,
                    // `Run` targets a specific run UUID; `Current` targets the
                    // invoking run's own dir — neither has a sibling pipeline
                    // YAML to look up.
                    crate::uri::BeltUri::Run { .. } | crate::uri::BeltUri::Current { .. } => {
                        continue;
                    }
                };
                let sibling_file = base_dir.join(format!("{producer}.yml"));
                let sibling_dir = base_dir.join(producer).join("pipeline.yml");
                if !sibling_file.is_file() && !sibling_dir.is_file() {
                    diagnostics.push(LintDiagnostic {
                        severity: Severity::Warning,
                        message: format!(
                            "phase '{}': consumes '{}' references pipeline '{producer}' but no sibling YAML found (looked for '{producer}.yml' and '{producer}/pipeline.yml')",
                            phase.id, name
                        ),
                    });
                }
            }
        }
    }
}

/// Lint rule: warn when a phase's `produces` entries are not protected by a
/// matching gate check. A `produces` entry is considered protected when any
/// gate in the same phase is either:
///   - `file_exists: <path>` with a literal string-equal `path` to the
///     produces entry's `path` (raw template string — `{run_id}` is not
///     expanded at lint time), or
///   - `has_output: true` (presence-check on the phase's `output_dir`, weaker
///     but spec-accepted).
///
/// Without gate protection, the phase can complete "successfully" without
/// actually writing the promised file. Downstream consumers then observe
/// missing paths or empty `resolved_consumes` at runtime — a class of bug
/// this lint catches at authoring time.
///
/// Warning-only: the phase may legitimately write the file via an external
/// mechanism (e.g., tracked via another pipeline's gate), so the check is
/// advisory rather than fatal.
fn check_produces_protected_by_gate(pipeline: &Pipeline, diagnostics: &mut Vec<LintDiagnostic>) {
    for phase in &pipeline.phases {
        for art in &phase.produces {
            let protected = phase.gate.iter().any(|g| match g {
                GateCheck::FileExists { file_exists } => file_exists == &art.path,
                GateCheck::HasOutput { has_output: true } => true,
                _ => false,
            });
            if !protected {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Warning,
                    message: format!(
                        "phase '{}': produces '{}' path '{}' is not protected by gate",
                        phase.id, art.name, art.path
                    ),
                });
            }
        }
    }
}

/// Lint rule: `Artifact.when` expressions must reference args that are
/// declared in the pipeline's `args:` map. Only `args.<flag>` form is
/// supported at runtime (view / engine resolution); anything else is
/// authoring-time dead code.
///
/// Warning-only — an undefined arg reference always evaluates to `false`,
/// so the artifact is silently filtered from `status` / `resolved_consumes`.
/// The lint surfaces this drift at authoring time.
fn check_artifact_when_references(pipeline: &Pipeline, diagnostics: &mut Vec<LintDiagnostic>) {
    let defined_args: HashSet<&str> = pipeline.args.keys().map(String::as_str).collect();
    for phase in &pipeline.phases {
        for artifact in &phase.produces {
            let Some(expr) = artifact.when.as_deref() else {
                continue;
            };
            let expr = expr.trim();
            let Some(arg_name) = expr.strip_prefix("args.") else {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Warning,
                    message: format!(
                        "phase '{}' artifact '{}' has unsupported when expression '{}'; only `args.<flag>` is supported",
                        phase.id, artifact.name, expr
                    ),
                });
                continue;
            };
            if !defined_args.contains(arg_name) {
                diagnostics.push(LintDiagnostic {
                    severity: Severity::Warning,
                    message: format!(
                        "phase '{}' artifact '{}' references undefined arg '{}' in when clause",
                        phase.id, artifact.name, arg_name
                    ),
                });
            }
        }
    }
}

/// Lint rule: reject raw `.belt/runs/` path literals and `{run_id}` template
/// strings in `produces[].path`, `gate.file_exists`, and `gate.cmd`. These
/// constructs were removed by the 2026-04-18 `belt://current` URI migration;
/// pipeline.yml authors must use `belt://current/<path>` instead.
fn check_belt_runs_literal(pipeline: &Pipeline, diagnostics: &mut Vec<LintDiagnostic>) {
    fn check_string(s: &str, phase_id: &str, field: &str, diagnostics: &mut Vec<LintDiagnostic>) {
        if s.contains(".belt/runs/") {
            diagnostics.push(LintDiagnostic {
                severity: Severity::Error,
                message: format!(
                    "phase '{phase_id}': {field} contains forbidden '.belt/runs/' literal — use belt://current/<path>"
                ),
            });
        }
        if s.contains("{run_id}") {
            diagnostics.push(LintDiagnostic {
                severity: Severity::Error,
                message: format!(
                    "phase '{phase_id}': {field} contains forbidden '{{run_id}}' template — use belt://current/<path>"
                ),
            });
        }
    }

    for phase in &pipeline.phases {
        for art in &phase.produces {
            check_string(&art.path, &phase.id, "produces.path", diagnostics);
        }
        for g in &phase.gate {
            match g {
                GateCheck::FileExists { file_exists } => {
                    check_string(file_exists, &phase.id, "gate.file_exists", diagnostics);
                }
                GateCheck::Cmd { cmd, .. } => {
                    check_string(cmd, &phase.id, "gate.cmd", diagnostics);
                }
                _ => {}
            }
        }
    }
}
