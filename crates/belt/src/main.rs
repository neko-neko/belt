use belt_core::lint::{Severity, lint_pipeline};
use clap::{Parser, Subcommand};
use std::path::Path;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "belt", about = "belt — pipeline authoring tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a pipeline YAML file
    Lint {
        /// Path to pipeline YAML file
        file: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Lint { file } => {
            let path = Path::new(&file);
            match lint_pipeline(path) {
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
                        eprintln!("ok: {file}");
                        ExitCode::SUCCESS
                    } else {
                        eprintln!("ok (with warnings): {file}");
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
