//! flowrail — agent CLI binary
//!
//! LLM エージェント向けの軽量ワークフローエンジン CLI。
//! stdin/stdout JSON 通信が主。security-first、minimal dependencies。
//! TUI は別バイナリ `flowrail-tui` として Phase 3 で提供される。
//!
//! CLI 体系: `flowrail <resource> <verb>` (5 resources: pipeline/run/state/snapshot/help)

#![forbid(unsafe_code)]

fn main() {
    println!("flowrail 0.1.0");
}
