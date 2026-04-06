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

    #[error("invalid pipeline: {message}")]
    #[diagnostic(code(belt::invalid_pipeline))]
    InvalidPipeline { message: String },

    #[error("gate failed: {message}")]
    #[diagnostic(code(belt::gate_failed))]
    GateFailed { message: String },

    #[error("state error: {message}")]
    #[diagnostic(code(belt::state))]
    State { message: String },

    #[error(transparent)]
    #[diagnostic(code(belt::io))]
    Io(#[from] std::io::Error),
}

pub type BeltResult<T> = Result<T, BeltError>;
