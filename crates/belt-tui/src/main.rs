//! belt-tui — human TUI binary (Phase 3)
//!
//! 開発者・運用者が手動で workflow を監視・デバッグするための rich TUI。
//! 原則 8 (Separation by Audience) により、agent CLI (`belt`) と developer CLI
//! (`belt-dev`) とは独立した binary として提供される。
//!
//! Phase 1 では placeholder。Phase 3 で ratatui-based の real-time state
//! visualization を実装する。依存 (ratatui, crossterm 等) は Phase 3 時に
//! `crates/belt-tui/Cargo.toml` に追加する。

fn main() {
    eprintln!("belt-tui: coming in Phase 3");
    std::process::exit(0);
}
