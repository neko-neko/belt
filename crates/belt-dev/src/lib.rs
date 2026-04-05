//! belt-dev の内部ライブラリ。
//!
//! belt-dev は binary crate (`src/main.rs`) だが、integration tests (`tests/*.rs`) から
//! lint / fmt module にアクセスする必要があるため、同時に lib crate (`src/lib.rs`) も
//! 提供する。これは Rust の一般的な lib+bin パターン。
//!
//! - lib crate 名: `belt_dev` (`snake_case`、パッケージ名から自動派生)
//! - bin crate 名: `belt-dev` (`Cargo.toml` の `[[bin]] name = "belt-dev"`)
//!
//! 後続 Task で以下のモジュールを追加する:
//!   - Task 12 以降: `pub mod lint;`
//!   - Task 19 以降: `pub mod fmt;`

// Test/bench code は assertions で `.unwrap()` / `.expect()` / `panic!()` を多用するため、
// workspace.lints.clippy の unwrap_used / expect_used / panic warn を test context では
// 許可する (2026-04-05 plan-review Finding 5 反映)。production code では依然として warn。
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

// module 宣言は Task 12 以降で追加
