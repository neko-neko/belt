use clap::{Parser, Subcommand};

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

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Lint { file: _ } => {
            eprintln!("belt lint: not yet implemented");
            std::process::exit(64);
        }
    }
}
