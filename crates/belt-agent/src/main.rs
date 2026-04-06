use belt_core::engine::Engine;
use belt_core::error::BeltError;
use belt_core::gate::{all_passed, execute_gates};
use clap::{Parser, Subcommand};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "belt-agent",
    about = "belt-agent — workflow runtime for LLM/CI"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new run from a pipeline YAML
    Init {
        /// Path to pipeline YAML file
        file: String,
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

fn resolve_run(engine: &Engine, run: Option<&String>) -> miette::Result<String> {
    match run {
        Some(id) => Ok(id.clone()),
        None => engine.latest_run_id().map_err(|e| miette::miette!("{e}")),
    }
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    let engine = Engine::new(&belt_dir());

    match cli.command {
        Command::Init { file, args } => cmd_init(&engine, &file, args)?,
        Command::Next { run } => cmd_next(&engine, run.as_ref())?,
        Command::Verify { run } => cmd_verify(&engine, run.as_ref())?,
        Command::Step { run, confirm } => cmd_step(&engine, run.as_ref(), confirm)?,
        Command::Status { run } => cmd_status(&engine, run.as_ref())?,
    }
    Ok(())
}

fn cmd_init(
    engine: &Engine,
    file: &str,
    args: Vec<(String, serde_json::Value)>,
) -> miette::Result<()> {
    let args_map: HashMap<String, serde_json::Value> = args.into_iter().collect();
    let path = Path::new(file);
    let state = engine
        .init(path, &args_map)
        .map_err(|e| miette::miette!("{e}"))?;
    let pipeline_path = Path::new(&state.pipeline_file);
    let phase = engine
        .next_phase_info(&state, pipeline_path)
        .map_err(|e| miette::miette!("{e}"))?;

    let out = json!({
        "run_id": state.run_id,
        "pipeline": state.pipeline,
        "phase": {
            "id": phase.id,
            "description": phase.description,
            "config": phase.config,
            "artifacts": phase.artifacts,
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
            "artifacts": phase.artifacts,
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
        Err(e) => return Err(miette::miette!("{e}")),
    }
    Ok(())
}

fn cmd_status(engine: &Engine, run: Option<&String>) -> miette::Result<()> {
    let run_id = resolve_run(engine, run)?;
    let state = engine
        .load_state(&run_id)
        .map_err(|e| miette::miette!("{e}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&state).map_err(|e| miette::miette!("{e}"))?
    );
    Ok(())
}
