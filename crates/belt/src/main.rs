//! belt — agent runtime CLI binary (Phase 2 target)
//!
//! LLM エージェント向けの軽量ワークフローエンジン CLI runtime。
//! stdin/stdout JSON 通信が主。security-first、minimal dependencies。
//!
//! 原則 8 (Separation by Audience, 3 audience: Developer / Agent / Human):
//! - belt は **Agent 専用**。lint/fmt 等の authoring-time ツール (`belt-dev`) および
//!   TUI (`belt-tui`) とは独立した binary として提供される
//! - 依存グラフ上 `belt` は TUI deps (ratatui, crossterm) も lint rules code も
//!   一切取り込まない (supply chain isolation)
//!
//! Phase 1: placeholder (Phase 2 で state machine runtime を実装する)。
//! Phase 2: `belt run next / verify / step` 系 subcommand 実装、5 リソース体系
//!   (pipeline / run / state / snapshot / help)。
//!
//! CLI 体系 (Phase 2 以降): `belt <resource> <verb>`
//!   - `belt run init --pipeline <path>`
//!   - `belt run next [--format md|json]`
//!   - `belt run verify [--rule-set <name>]`
//!   - `belt run step [--validation-result ...]`
//!   - `belt state show|list|reset|prune`
//!   - `belt snapshot create|restore|list|prune`
//!   - `belt help`

fn main() {
    eprintln!("belt (agent runtime CLI): coming in Phase 2");
    eprintln!("Phase 1 MVP target is belt-dev (developer CLI). See docs/plans/.");
    std::process::exit(0);
}
