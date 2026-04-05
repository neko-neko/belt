//! belt-core — Tiny workflow engine runtime library
//!
//! Pure library providing the agent runtime primitives shared across all belt
//! binaries (`belt`, `belt-dev`, `belt-tui`). Consistent with 原則 8
//! (Separation by Audience, 3 audience: Developer / Agent / Human) and
//! 原則 7 (Tiny by Constraint).
//!
//! belt-core が知る 5 概念:
//! - State machine (phase transitions)
//! - Artifact lifecycle
//! - Rule set resolver (YAML declarations, uses resolution, max-depth guard)
//! - 4 core primitive checks (`file_exists`, `cmd_exit`, `regex_match`, `git_status`)
//! - Hook executor (lifecycle events, stdin JSON contract)
//!
//! belt-core が持たない概念 (原則 8 + 原則 2):
//! - Lint / Fmt (developer-only、`belt-dev` binary crate に private モジュールとして配置)
//! - TUI rendering (`belt-tui` binary crate 専用)
//! - CLI argument parsing (各 binary crate で `clap` 経由)
//!
//! 8 Built-in Directives (低次語彙) と 4 LLM Response Types も本 crate が提供する。
//!
//! See `docs/specs/` and `docs/plans/` for design and implementation details.

// Test/bench code は assertions で `.unwrap()` / `.expect()` / `panic!()` を多用するため、
// workspace.lints.clippy の unwrap_used / expect_used / panic warn を test context では
// 許可する (2026-04-05 plan-review Finding 5 反映)。production code では依然として warn。
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

// Module declarations to be added during Phase 1 implementation (Task 2+)
