//! flowrail-core — Tiny workflow engine library
//!
//! Pure library providing:
//! - State machine (phase transitions)
//! - Artifact lifecycle
//! - Rule set resolver (YAML declarations, uses resolution)
//! - 4 core primitive checks (file_exists, cmd_exit, regex_match, git_status)
//! - Hook executor (lifecycle events, stdin JSON contract)
//! - 8 built-in directives
//! - YAML abstraction layer (`yaml` module) — direct serde-saphyr use is forbidden
//!
//! See `docs/specs/` and `docs/plans/` for design and implementation details.

#![forbid(unsafe_code)]

// Module declarations to be added during Phase 1 implementation (Task 2+)
