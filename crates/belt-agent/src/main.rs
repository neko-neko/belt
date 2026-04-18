use belt_core::config::{parse_config, resolve_pipeline_path};
use belt_core::engine::Engine;
use belt_core::error::BeltError;
use belt_core::expander::expand_pipeline;
use belt_core::gate::{all_passed, execute_gates};
use clap::{Parser, Subcommand};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod git;
mod resolver;

#[derive(Parser)]
#[command(name = "belt-agent", about = "belt-agent — workflow runtime for LLM")]
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
        /// Optional `run_id` to inherit narrative from (context-neutral
        /// narrative artifact). Equivalent to adding a hidden
        /// `belt://run/<run_id>/...` reference for lookup.
        #[arg(long = "inherits-from")]
        inherits_from: Option<String>,
    },
    /// Get current phase info
    Next {
        /// Run ID (default: latest)
        #[arg(long)]
        run: Option<String>,
    },
    /// Run gate checks for current phase
    Verify {
        #[arg(long)]
        run: Option<String>,
    },
    /// Run regate checks for current phase targets
    Regate {
        #[arg(long)]
        run: Option<String>,
    },
    /// Advance to next phase
    Step {
        #[arg(long)]
        run: Option<String>,
        /// Acknowledge confirm/validate requirements
        #[arg(long)]
        confirm: bool,
    },
    /// Show current run state
    Status {
        #[arg(long)]
        run: Option<String>,
    },
    /// Resolve a `belt://` URI to its filesystem path
    Locate {
        /// belt:// URI to resolve
        uri: String,
        /// Run ID (default: latest)
        #[arg(long)]
        run: Option<String>,
    },
}

#[allow(clippy::unnecessary_wraps)] // clap value_parser requires Result
fn parse_arg(s: &str) -> Result<(String, serde_json::Value), String> {
    if let Some((key, val)) = s.split_once('=') {
        let value = match val {
            "true" => serde_json::Value::Bool(true),
            "false" => serde_json::Value::Bool(false),
            _ => {
                if let Ok(n) = val.parse::<f64>() {
                    json!(n)
                } else {
                    serde_json::Value::String(val.to_string())
                }
            }
        };
        Ok((key.to_string(), value))
    } else {
        // bare flag treated as bool true
        Ok((s.to_string(), serde_json::Value::Bool(true)))
    }
}

fn belt_dir() -> PathBuf {
    PathBuf::from(".belt")
}

fn sanitize_phase_id(id: &str) -> String {
    id.replace('/', "_")
}

fn write_result_file(
    belt: &Path,
    run_id: &str,
    subdir: &str,
    phase_id: &str,
    json: &serde_json::Value,
) {
    let dir = belt.join("runs").join(run_id).join(subdir);
    if std::fs::create_dir_all(&dir).is_err() {
        eprintln!("warning: failed to create {}", dir.display());
        return;
    }
    let file = dir.join(format!("{}.json", sanitize_phase_id(phase_id)));
    if let Err(e) = std::fs::write(
        &file,
        serde_json::to_string_pretty(json).unwrap_or_default(),
    ) {
        eprintln!("warning: failed to write {}: {e}", file.display());
    }
}

fn resolve_run(engine: &Engine, run: Option<&String>) -> miette::Result<String> {
    match run {
        Some(id) => Ok(id.clone()),
        None => engine.latest_run_id().map_err(|e| miette::miette!("{e}")),
    }
}

fn resolve_pipeline(config: Option<&String>, file: Option<&String>) -> miette::Result<PathBuf> {
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

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    let engine = Engine::new(&belt_dir());

    match cli.command {
        Command::Init {
            file,
            args,
            inherits_from,
        } => {
            let pipeline_path = resolve_pipeline(cli.config.as_ref(), file.as_ref())?;
            cmd_init(&engine, &pipeline_path, args, inherits_from.as_deref())?;
        }
        Command::Next { run } => cmd_next(&engine, run.as_ref())?,
        Command::Verify { run } => cmd_verify(&engine, run.as_ref())?,
        Command::Regate { run } => cmd_regate(&engine, run.as_ref())?,
        Command::Step { run, confirm } => cmd_step(&engine, run.as_ref(), confirm)?,
        Command::Status { run } => cmd_status(&engine, run.as_ref())?,
        Command::Locate { uri, run } => cmd_locate(&engine, &uri, run.as_ref())?,
    }
    Ok(())
}

fn cmd_init(
    engine: &Engine,
    pipeline_path: &Path,
    args: Vec<(String, serde_json::Value)>,
    inherits_from: Option<&str>,
) -> miette::Result<()> {
    // Validate that --inherits-from points to an existing run directory.
    // Fail-fast early (before init creates a run directory) so we never
    // leave an orphaned state.json pointing at a non-existent parent run.
    // The *synthetic* resolved_consumes entry for --inherits-from is added
    // further below, after the run is materialised.
    if let Some(run_id) = inherits_from {
        let run_dir = belt_dir().join("runs").join(run_id);
        if !run_dir.is_dir() {
            return Err(miette::miette!("--inherits-from: run not found: {run_id}"));
        }
    }

    let args_map: HashMap<String, serde_json::Value> = args.into_iter().collect();
    // Detect the current git branch from the user's shell CWD so
    // workspace-scoped URI resolvers (Task 15) can filter runs by branch.
    // `current_branch` returns `None` outside a git repo, in detached HEAD,
    // or before the first commit — belt-core trusts the value verbatim.
    let branch = crate::git::current_branch(std::path::Path::new("."));

    // Resolve every External `belt://` URI in each phase's `consumes:` list
    // *before* creating the run directory. If any resolver call fails we
    // return early without having written anything to `.belt/runs/`, so a
    // failed init cannot leave an orphan half-initialised run behind
    // (BELT-33). The resolver reads `.belt/runs/` to find completed
    // producer runs, which works fine before the current run is
    // materialised.
    let belt = belt_dir();
    let phases = expand_pipeline(pipeline_path).map_err(|e| miette::miette!("{e}"))?;
    let resolver = crate::resolver::Resolver {
        belt_dir: &belt,
        current_branch: branch.clone(),
        current_run_id: None, // init has no current run yet
    };
    let mut resolved_map: HashMap<String, String> = HashMap::new();
    for phase in &phases {
        for aref in &phase.consumes {
            if let belt_core::model::ArtifactRef::External { uri, .. } = aref {
                let path = resolver.resolve(uri).map_err(|e| miette::miette!("{e}"))?;
                resolved_map.insert(uri.to_string(), path.display().to_string());
            }
        }
    }

    // --inherits-from registers the inherited run under a synthetic
    // `belt://run/<id>/` key so skills can locate the parent run directory
    // without requiring an explicit External reference in the pipeline YAML.
    // The existence check above guarantees run_dir is a directory here.
    if let Some(run_id) = inherits_from {
        let run_dir = belt.join("runs").join(run_id);
        resolved_map.insert(
            format!("belt://run/{run_id}/"),
            run_dir.display().to_string(),
        );
    }

    // Every URI has resolved successfully — now materialise the run
    // directory and persist the resolved mapping. This is the earliest
    // point at which `cmd_init` writes anything to `.belt/runs/`.
    let mut state = engine
        .init_with_branch(pipeline_path, &args_map, branch)
        .map_err(|e| miette::miette!("{e}"))?;
    engine
        .set_resolved_consumes(&mut state, resolved_map)
        .map_err(|e| miette::miette!("{e}"))?;

    let pipeline_file = Path::new(&state.pipeline_file);
    let phase = engine
        .next_phase_info(&state, pipeline_file)
        .map_err(|e| miette::miette!("{e}"))?;

    let phase_obj = phase_json(&phase);
    let out = json!({
        "run_id": state.run_id,
        "pipeline": state.pipeline,
        "phase": phase_obj,
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

/// Build the per-phase JSON sub-object used by `init` and `next` responses.
///
/// Mirrors the shape surfaced by `status`: the typed `invoke`, `produces`, and
/// `consumes` fields from BELT-32 are forwarded so orchestrators can read the
/// invocation target directly from the command they already call, without
/// issuing a separate `status` round-trip. `invoke` is omitted when absent
/// (preserving backwards compatibility with legacy pipelines), and
/// `produces` / `consumes` are always included as (possibly empty) arrays for
/// consumer ergonomics.
fn phase_json(phase: &belt_core::model::ExpandedPhase) -> serde_json::Value {
    let mut phase_obj = json!({
        "id": phase.id,
        "description": phase.description,
        "config": phase.config,
        "output_dir": phase.output_dir,
        "produces": phase.produces,
        "consumes": phase.consumes,
    });
    if let Some(invoke) = &phase.invoke {
        if let serde_json::Value::Object(map) = &mut phase_obj {
            map.insert(
                "invoke".to_string(),
                serde_json::to_value(invoke).unwrap_or(serde_json::Value::Null),
            );
        }
    }
    phase_obj
}

fn cmd_next(engine: &Engine, run: Option<&String>) -> miette::Result<()> {
    let run_id = resolve_run(engine, run)?;
    let state = engine
        .load_state(&run_id)
        .map_err(|e| miette::miette!("{e}"))?;

    if state.current_phase == "COMPLETED" {
        let out = json!({
            "run_id": state.run_id,
            "completed": true,
            "completed_phases": state.completed_phases,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
        );
        return Ok(());
    }

    let pipeline_path = Path::new(&state.pipeline_file);
    let phase = engine
        .next_phase_info(&state, pipeline_path)
        .map_err(|e| miette::miette!("{e}"))?;
    let attempt = state.phase_attempts.get(&phase.id).copied().unwrap_or(0);
    let mut phase_obj = phase_json(&phase);
    // Augment External consumes entries with `resolved_path` from
    // `state.resolved_consumes`, computed at `init` time. Named/Qualified
    // entries keep their existing (derived-Serialize) JSON shape so existing
    // consumers are not broken.
    if let serde_json::Value::Object(map) = &mut phase_obj {
        map.insert(
            "consumes".to_string(),
            build_consumes_with_resolved(&phase.consumes, &state.resolved_consumes),
        );
    }

    let out = json!({
        "run_id": state.run_id,
        "phase": phase_obj,
        "gate": phase.gate,
        "validate": phase.validate,
        "confirm": phase.confirm,
        "regate": phase.regate,
        "max_retries": phase.max_retries,
        "attempt": attempt,
        "args": state.args,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
    );
    Ok(())
}

/// Render the `consumes` list for `next` output.
///
/// `ArtifactRef::External` entries are augmented with a `resolved_path` field
/// looked up by URI in `state.resolved_consumes` (populated at `init` time).
/// `resolved_path` is `null` if the lookup misses, which should not happen
/// under normal operation but is preserved as an explicit JSON signal rather
/// than silently omitting the field (consumers can distinguish "unresolvable"
/// from "not External"). `Named` and `Qualified` variants are emitted via
/// their derived `Serialize` shape to preserve backwards compatibility with
/// existing integration tests and skill-side consumers.
fn build_consumes_with_resolved(
    refs: &[belt_core::model::ArtifactRef],
    resolved: &HashMap<String, String>,
) -> serde_json::Value {
    use belt_core::model::ArtifactRef;
    let mut out: Vec<serde_json::Value> = Vec::with_capacity(refs.len());
    for aref in refs {
        match aref {
            ArtifactRef::External { name, uri } => {
                let uri_str = uri.to_string();
                let resolved_path = resolved.get(&uri_str);
                out.push(json!({
                    "name": name,
                    "uri": uri_str,
                    "resolved_path": resolved_path,
                }));
            }
            other => {
                out.push(serde_json::to_value(other).unwrap_or(serde_json::Value::Null));
            }
        }
    }
    serde_json::Value::Array(out)
}

fn cmd_verify(engine: &Engine, run: Option<&String>) -> miette::Result<()> {
    let run_id = resolve_run(engine, run)?;
    let mut state = engine
        .load_state(&run_id)
        .map_err(|e| miette::miette!("{e}"))?;
    let pipeline_path_str = state.pipeline_file.clone();
    let pipeline_path = Path::new(&pipeline_path_str);
    let phase = engine
        .next_phase_info(&state, pipeline_path)
        .map_err(|e| miette::miette!("{e}"))?;

    let output_dir = phase.output_dir.as_deref().unwrap_or(".");
    let work_dir = std::env::current_dir().map_err(|e| miette::miette!("{e}"))?;
    let results = execute_gates(&phase.gate, &work_dir, Path::new(output_dir));
    let verdict = all_passed(&results);

    engine
        .verify_verdict(&mut state, verdict)
        .map_err(|e| miette::miette!("{e}"))?;
    let attempt = state.phase_attempts.get(&phase.id).copied().unwrap_or(0);

    let verify_result = json!({
        "phase": phase.id,
        "verdict": if verdict { "PASS" } else { "FAIL" },
        "checks": results,
        "attempt": attempt,
        "timestamp": belt_core::engine::now_iso8601(),
    });
    write_result_file(
        &belt_dir(),
        &state.run_id,
        "verify",
        &phase.id,
        &verify_result,
    );

    let out = json!({
        "run_id": state.run_id,
        "phase": phase.id,
        "verdict": if verdict { "PASS" } else { "FAIL" },
        "checks": results,
        "attempt": attempt,
        "max_retries": phase.max_retries,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
    );
    Ok(())
}

fn cmd_step(engine: &Engine, run: Option<&String>, confirm: bool) -> miette::Result<()> {
    let run_id = resolve_run(engine, run)?;
    let mut state = engine
        .load_state(&run_id)
        .map_err(|e| miette::miette!("{e}"))?;
    let pipeline_path_str = state.pipeline_file.clone();
    let pipeline_path = Path::new(&pipeline_path_str);

    // Check confirm requirement before engine.step() to avoid the
    // (marginally wasteful) expand_pipeline() call when confirm would
    // reject anyway. Engine-level guards (verify, max_retries) are
    // enforced inside step() regardless of this check's outcome.
    let phase = engine
        .next_phase_info(&state, pipeline_path)
        .map_err(|e| miette::miette!("{e}"))?;
    if (phase.confirm || !phase.validate.is_empty()) && !confirm {
        let out = json!({
            "advanced": false,
            "reason": "confirmation_required",
            "phase": phase.id,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
        );
        return Ok(());
    }

    let from = state.current_phase.clone();
    match engine.step(&mut state, pipeline_path) {
        Ok(next) => {
            let out = match next {
                Some(to) => json!({
                    "advanced": true,
                    "from": from,
                    "to": to,
                }),
                None => json!({
                    "advanced": true,
                    "from": from,
                    "to": null,
                    "completed": true,
                }),
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
            );
        }
        Err(BeltError::VerifyRequired { phase_id }) => {
            let out = json!({
                "advanced": false,
                "reason": "verify_required",
                "phase": phase_id,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
            );
        }
        Err(BeltError::MaxRetriesExceeded {
            phase_id,
            attempts,
            max_retries,
        }) => {
            let out = json!({
                "advanced": false,
                "reason": "max_retries_exceeded",
                "phase": phase_id,
                "attempts": attempts,
                "max_retries": max_retries,
                "escalation": true,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
            );
        }
        Err(BeltError::RegateRequired { phase_id, targets }) => {
            let out = json!({
                "advanced": false,
                "reason": "regate_not_executed",
                "phase": phase_id,
                "regate_targets": targets,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
            );
        }
        Err(BeltError::RegateFailed { phase_id, targets }) => {
            let out = json!({
                "advanced": false,
                "reason": "regate_failed",
                "phase": phase_id,
                "regate_targets": targets,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
            );
        }
        Err(e) => return Err(miette::miette!("{e}")),
    }
    Ok(())
}

/// Execute regate gate checks for each target, returning (`targets_map`, `all_passed`).
fn execute_regate_targets(
    phase: &belt_core::model::ExpandedPhase,
    state: &belt_core::model::RunState,
    all_phases: &[belt_core::model::ExpandedPhase],
    belt: &Path,
) -> miette::Result<(serde_json::Map<String, serde_json::Value>, bool)> {
    let work_dir = std::env::current_dir().map_err(|e| miette::miette!("{e}"))?;
    let mut targets = serde_json::Map::new();
    let mut all_passed_flag = true;

    for target_id in &phase.regate {
        // Skip regate for skipped phases (auto-passed per spec)
        if state.skipped_phases.contains(target_id) {
            targets.insert(
                target_id.clone(),
                json!({ "passed": true, "skipped": true, "checks": [] }),
            );
            continue;
        }

        let target_phase = all_phases
            .iter()
            .find(|p| &p.id == target_id)
            .ok_or_else(|| {
                miette::miette!("regate target '{}' not found in pipeline", target_id)
            })?;

        // The expander does not substitute `{run_id}` (it has no access to
        // runtime state). Apply the same gate-text expansion that
        // `next_phase_info` applies to the current phase, otherwise gates
        // like `file_exists: ".belt/runs/{run_id}/notes/phase-X.md"` would
        // be checked against the literal `{run_id}` directory.
        let mut target_gate = target_phase.gate.clone();
        belt_core::engine::expand_gate_run_id(&mut target_gate, &state.run_id);

        let run_dir = belt.join("runs").join(&state.run_id);
        let output_dir = run_dir.join(target_id.replace('/', "_"));
        let results = execute_gates(&target_gate, &work_dir, &output_dir);
        let passed = all_passed(&results);
        if !passed {
            all_passed_flag = false;
        }
        targets.insert(
            target_id.clone(),
            json!({ "passed": passed, "checks": results }),
        );
    }

    Ok((targets, all_passed_flag))
}

fn cmd_regate(engine: &Engine, run: Option<&String>) -> miette::Result<()> {
    let run_id = resolve_run(engine, run)?;
    let mut state = engine
        .load_state(&run_id)
        .map_err(|e| miette::miette!("{e}"))?;

    if state.current_phase == "COMPLETED" {
        let out = json!({
            "error": "pipeline_completed",
            "message": "pipeline is already completed"
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
        );
        return Ok(());
    }

    // Pre-check: verify must have passed
    if state.phase_verify_passed.get(&state.current_phase) != Some(&true) {
        let out = json!({
            "error": "verify_not_passed",
            "phase": state.current_phase,
            "message": "verify must pass before regate"
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
        );
        return Ok(());
    }

    let pipeline_path_str = state.pipeline_file.clone();
    let pipeline_path = Path::new(&pipeline_path_str);
    let phase = engine
        .next_phase_info(&state, pipeline_path)
        .map_err(|e| miette::miette!("{e}"))?;

    // No regate targets
    if phase.regate.is_empty() {
        let regate_result = json!({
            "phase": phase.id,
            "targets": {},
            "all_passed": true,
            "timestamp": belt_core::engine::now_iso8601(),
        });
        write_result_file(
            &belt_dir(),
            &state.run_id,
            "regate",
            &phase.id,
            &regate_result,
        );

        let out = json!({
            "run_id": state.run_id,
            "phase": phase.id,
            "targets": {},
            "all_passed": true
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
        );
        return Ok(());
    }

    // Get all expanded phases for target lookup
    let all_phases = expand_pipeline(pipeline_path).map_err(|e| miette::miette!("{e}"))?;
    let belt = belt_dir();

    let (targets, all_passed_flag) = execute_regate_targets(&phase, &state, &all_phases, &belt)?;

    engine
        .record_regate(&mut state, all_passed_flag)
        .map_err(|e| miette::miette!("{e}"))?;

    let regate_result = json!({
        "phase": phase.id,
        "targets": targets.clone(),
        "all_passed": all_passed_flag,
        "timestamp": belt_core::engine::now_iso8601(),
    });
    write_result_file(&belt, &state.run_id, "regate", &phase.id, &regate_result);

    let out = json!({
        "run_id": state.run_id,
        "phase": phase.id,
        "targets": targets,
        "all_passed": all_passed_flag
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
    );
    Ok(())
}

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

fn cmd_locate(engine: &Engine, uri_str: &str, run: Option<&String>) -> miette::Result<()> {
    use belt_core::uri::BeltUri;
    let uri = BeltUri::parse(uri_str).map_err(|e| miette::miette!("{e}"))?;

    // Determine the current run id: explicit --run wins, otherwise fall back
    // to engine.latest_run_id(). For non-Current variants the field is ignored
    // by the resolver, so failure to resolve a current run is not fatal here
    // unless the URI is BeltUri::Current.
    let current_run_id = match run {
        Some(id) => Some(id.clone()),
        None => engine.latest_run_id().ok(),
    };

    let branch = crate::git::current_branch(std::path::Path::new("."));
    let belt = belt_dir();
    let resolver = crate::resolver::Resolver {
        belt_dir: &belt,
        current_branch: branch,
        current_run_id,
    };
    let resolved = resolver.resolve(&uri).map_err(|e| miette::miette!("{e}"))?;

    // existence is computed via fs metadata; for glob URIs this is `glob match >= 1`.
    // For simplicity here we only handle direct path existence; glob was
    // expanded by the resolver path semantics.
    let exists = std::fs::metadata(&resolved).is_ok();

    let out = json!({
        "uri": uri.to_string(),
        "path": resolved.display().to_string(),
        "exists": exists,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&out).map_err(|e| miette::miette!("{e}"))?
    );
    Ok(())
}
