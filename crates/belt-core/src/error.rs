use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum BeltError {
    #[error("YAML parse error: {message}")]
    #[diagnostic(code(belt::yaml_parse))]
    YamlParse {
        message: String,
        #[source_code]
        src: Option<String>,
    },

    #[error("file not found: {path}")]
    #[diagnostic(code(belt::file_not_found))]
    FileNotFound { path: String },

    #[error("config parse error in {path}: {detail}")]
    #[diagnostic(code(belt::config_parse))]
    ConfigParse { path: String, detail: String },

    #[error("invalid pipeline: {message}")]
    #[diagnostic(code(belt::invalid_pipeline))]
    InvalidPipeline { message: String },

    #[error("gate failed: {message}")]
    #[diagnostic(code(belt::gate_failed))]
    GateFailed { message: String },

    #[error("verify required for phase '{phase_id}': run verify before step")]
    #[diagnostic(code(belt::verify_required))]
    VerifyRequired { phase_id: String },

    #[error("max retries exceeded for phase '{phase_id}': {attempts}/{max_retries}")]
    #[diagnostic(code(belt::max_retries_exceeded))]
    MaxRetriesExceeded {
        phase_id: String,
        attempts: u32,
        max_retries: u32,
    },

    #[error("regate required for phase '{phase_id}': run regate before step")]
    #[diagnostic(code(belt::regate_required))]
    RegateRequired {
        phase_id: String,
        targets: Vec<String>,
    },

    #[error("regate failed for phase '{phase_id}': targets {targets:?} did not pass")]
    #[diagnostic(code(belt::regate_failed))]
    RegateFailed {
        phase_id: String,
        targets: Vec<String>,
    },

    #[error("state error: {message}")]
    #[diagnostic(code(belt::state))]
    State { message: String },

    #[error(transparent)]
    #[diagnostic(code(belt::io))]
    Io(#[from] std::io::Error),
}

pub type BeltResult<T> = Result<T, BeltError>;
