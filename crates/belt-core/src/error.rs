//! Unified error type for belt-core.
//!
//! `BeltError` は belt-core のすべての fallible API から返る共通エラー型。
//! Phase 1 時点では variant は Task 2-17 を通じて段階的に配線されるため、
//! 現時点では未使用 variant を `#[allow(dead_code)]` で許容している。
//!
//! 含めない variant (意図的な設計):
//! - `CircularImport`: cycle detection は Non-Goal (`max_depth` ガードのみ)
//! - `LintFailed`: lint の `ExitCode` 制御は Task 17 driver 側で行う

use std::path::PathBuf;

use thiserror::Error;

/// belt-core 標準の `Result` alias。
pub type Result<T> = std::result::Result<T, BeltError>;

/// belt-core 全体で共有する統一エラー型。
///
/// variants は Task 2-17 を通じて段階的に配線される。現時点 (Task 2) では
/// すべての variant が未使用のため `#[allow(dead_code)]` で false-positive を抑制する。
#[derive(Debug, Error)]
#[allow(dead_code)] // variants are introduced incrementally across Task 2-17; suppress false-positive until wired.
pub enum BeltError {
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("YAML parse error in {path}: {source}")]
    YamlParse {
        path: PathBuf,
        #[source]
        source: crate::yaml::YamlError,
    },

    #[error("JSON schema validation failed in {path}: {message}")]
    SchemaValidation { path: PathBuf, message: String },

    #[error("Maximum import depth exceeded ({depth}) while resolving rule sets from {entry}")]
    MaxDepthExceeded { entry: PathBuf, depth: usize },

    #[error("Unknown rule set '{name}' used in {path}")]
    UnknownRuleSet { name: String, path: PathBuf },

    #[error("Parameter type mismatch: {details}")]
    ParamTypeMismatch { details: String },

    #[error("Unresolved template reference: {expression} in {path}")]
    UnresolvedTemplate { expression: String, path: PathBuf },
}
