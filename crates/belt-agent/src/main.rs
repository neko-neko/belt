use clap::{Parser, Subcommand};

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

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init { file: _ } => {
            eprintln!("belt-agent init: not yet implemented");
            std::process::exit(64);
        }
        Command::Next { run: _ } => {
            eprintln!("belt-agent next: not yet implemented");
            std::process::exit(64);
        }
        Command::Verify { run: _ } => {
            eprintln!("belt-agent verify: not yet implemented");
            std::process::exit(64);
        }
        Command::Step { run: _, confirm: _ } => {
            eprintln!("belt-agent step: not yet implemented");
            std::process::exit(64);
        }
        Command::Status { run: _ } => {
            eprintln!("belt-agent status: not yet implemented");
            std::process::exit(64);
        }
    }
}
