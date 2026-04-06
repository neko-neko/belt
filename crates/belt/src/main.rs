use belt_core::config::{parse_config, resolve_pipeline_path};
use belt_core::lint::{Severity, lint_pipeline};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "belt", about = "belt — pipeline authoring tool")]
struct Cli {
    /// Path to belt.toml config file
    #[arg(long, global = true)]
    config: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a pipeline YAML file
    Lint {
        /// Path to pipeline YAML file (mutually exclusive with --config)
        file: Option<String>,
    },
}

fn resolve_pipeline(config: Option<&String>, file: Option<&String>) -> Result<PathBuf, String> {
    match (config, file) {
        (Some(_), Some(_)) => Err(
            "conflicting arguments: --config and positional <file> are mutually exclusive"
                .to_string(),
        ),
        (Some(config_path), None) => {
            let config_path = Path::new(config_path);
            let cfg = parse_config(config_path).map_err(|e| e.to_string())?;
            Ok(resolve_pipeline_path(config_path, &cfg))
        }
        (None, Some(f)) => Ok(PathBuf::from(f)),
        (None, None) => {
            Err("missing argument: provide either --config <path> or a pipeline file".to_string())
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Lint { file } => {
            let path = match resolve_pipeline(cli.config.as_ref(), file.as_ref()) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("error: {e}");
                    return ExitCode::from(1);
                }
            };
            match lint_pipeline(&path) {
                Ok(diagnostics) => {
                    let mut has_errors = false;
                    for diag in &diagnostics {
                        let prefix = match diag.severity {
                            Severity::Error => {
                                has_errors = true;
                                "error"
                            }
                            Severity::Warning => "warning",
                        };
                        eprintln!("{prefix}: {}", diag.message);
                    }
                    if has_errors {
                        ExitCode::from(1)
                    } else if diagnostics.is_empty() {
                        let display = file
                            .as_deref()
                            .unwrap_or_else(|| path.to_str().unwrap_or("pipeline"));
                        eprintln!("ok: {display}");
                        ExitCode::SUCCESS
                    } else {
                        let display = file
                            .as_deref()
                            .unwrap_or_else(|| path.to_str().unwrap_or("pipeline"));
                        eprintln!("ok (with warnings): {display}");
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(1)
                }
            }
        }
    }
}
