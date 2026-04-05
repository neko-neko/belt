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

fn main() {
    println!("belt-dev 0.1.0 — Phase 1 implementation pending (see docs/plans/)");
}
