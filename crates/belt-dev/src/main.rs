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

use belt_core::error::Result;

// Task 2 時点では `?` 演算子を使う fallible 呼び出しがないため、clippy::unnecessary_wraps が
// 発火する。`Result<()>` を返す signature は Task 3 以降で ruleset loader / jsonschema validator
// 等の Result を `?` で伝播させるための forward-compatible な placeholder なので、ここでは
// function-level の allow で抑制する。Task 3 以降で `?` 使用箇所が増えた時点で外せる。
#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<()> {
    println!("belt-dev 0.1.0");
    Ok(())
}
