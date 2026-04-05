//! belt-dev — developer CLI binary
//!
//! Rule set 作者 (developer) 向けの authoring-time ツール。pipeline.yml と
//! rule set YAML に対する lint / fmt を提供する。Phase 1 MVP のメインターゲット。
//!
//! 原則 8 (Separation by Audience, 3 audience: Developer / Agent / Human):
//! - belt-dev は **Developer 専用**。agent runtime (`belt` binary) とは独立した binary
//! - lint/fmt ロジックは本 crate 内の private module (`lint/`, `fmt/`) として配置し、
//!   belt-core (runtime library) には含めない
//! - これにより `belt` (agent CLI) の binary には lint/fmt コードが一切入らず、
//!   supply chain 面・binary size 面で isolation を達成する
//!
//! CLI 体系: `belt-dev <resource> <verb>` (Phase 1 リソース: pipeline/help)
//!   - `belt-dev pipeline lint [path...]`
//!   - `belt-dev pipeline fmt  [path...] [--check|--diff]`
//!   - `belt-dev help`

mod cli;

use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, PipelineVerb, TopLevel};

// Exit codes (POSIX sysexits.h):
//   0  success
//   1  warnings only (Task 17 以降で使用)
//   2  lint errors (Task 17 以降で使用)
//   64 command/invocation error (EX_USAGE: clap parse error, missing file, schema load fail)
fn main() -> ExitCode {
    // `try_parse` を使うことで、clap parse error を自前の exit code 64 にマップする。
    // `Cli::parse()` は内部で `std::process::exit(2)` を呼ぶため使わない。
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            // clap error には help/version request も含まれる:
            //   - help/version は exit 0 (informational output → stdout)
            //   - それ以外の parse error は exit 64 (EX_USAGE → stderr)
            let exit_code = if err.use_stderr() { 64 } else { 0 };
            let _ = err.print();
            return ExitCode::from(exit_code);
        }
    };

    match cli.command {
        TopLevel::Pipeline(args) => match args.command {
            PipelineVerb::Lint { paths } => {
                eprintln!("(lint stub) paths={paths:?}");
            }
            PipelineVerb::Fmt { paths, check, diff } => {
                eprintln!("(fmt stub) paths={paths:?} check={check} diff={diff}");
            }
        },
    }

    ExitCode::SUCCESS
}
