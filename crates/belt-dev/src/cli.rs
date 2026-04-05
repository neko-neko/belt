//! Command-line interface definition for `belt-dev` (developer CLI).
//!
//! Phase 1 CLI surface:
//!   - `belt-dev pipeline lint  [PATH...]`
//!   - `belt-dev pipeline fmt   [PATH...] [--check|--diff]`
//!
//! Future resources (Phase 2+) will be added as additional [`TopLevel`] variants.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// belt-dev — Developer CLI for belt rule set authors (pipeline lint + fmt).
///
/// `cli.rs` is a binary-crate-internal module (`mod cli;` in `main.rs`), so the
/// types here are only consumed by `main.rs`. `pub(crate)` keeps visibility
/// minimal while still allowing `main.rs` to destructure the enum variants.
#[derive(Debug, Parser)]
#[command(name = "belt-dev", version, about, long_about = None)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: TopLevel,
}

/// Top-level resource (kubectl/helm-style: resource + verb).
#[derive(Debug, Subcommand)]
pub(crate) enum TopLevel {
    /// Pipeline resource: lint, fmt (Phase 1); test, init (Phase 2+).
    Pipeline(PipelineArgs),
}

/// `belt-dev pipeline <verb>` argument container.
#[derive(Debug, Args)]
pub(crate) struct PipelineArgs {
    #[command(subcommand)]
    pub(crate) command: PipelineVerb,
}

/// Verbs available under the `pipeline` resource in Phase 1.
#[derive(Debug, Subcommand)]
pub(crate) enum PipelineVerb {
    /// Validate pipeline.yml + rule-set.yml (schema + semantic checks).
    Lint {
        /// Target files or directories (default: current dir).
        #[arg(value_name = "PATH")]
        paths: Vec<PathBuf>,
    },
    /// Normalize pipeline.yml + rule-set.yml formatting.
    Fmt {
        /// Target files or directories.
        #[arg(value_name = "PATH")]
        paths: Vec<PathBuf>,
        /// Check only, do not modify files.
        #[arg(long)]
        check: bool,
        /// Show diff of changes.
        #[arg(long)]
        diff: bool,
    },
}
