use belt_core::config::{parse_config, resolve_pipeline_path};
use belt_core::engine::Engine;
use belt_core::error::BeltError;
use belt_core::expander::expand_pipeline;
use belt_core::gate::{all_passed, execute_gates};
use clap::{Parser, Subcommand};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
        Command::Init { file, args } => {
            let pipeline_path = resolve_pipeline(cli.config.as_ref(), file.as_ref())?;
            cmd_init(&engine, &pipeline_path, args)?;
        }
        Command::Next { run } => cmd_next(&engine, run.as_ref())?,
        Command::Verify { run } => cmd_verify(&engine, run.as_ref())?,
        Command::Regate { run } => cmd_regate(&engine, run.as_ref())?,
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

    let out = json!({
        "run_id": state.run_id,
        "phase": {
            "id": phase.id,
            "description": phase.description,
            "config": phase.config,
            "output_dir": phase.output_dir,
        },
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

        let run_dir = belt.join("runs").join(&state.run_id);
        let output_dir = run_dir.join(target_id.replace('/', "_"));
        let results = execute_gates(&target_phase.gate, &work_dir, &output_dir);
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
