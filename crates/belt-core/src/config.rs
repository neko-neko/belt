use crate::error::{BeltError, BeltResult};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Parsed representation of a `belt.toml` configuration file.
#[derive(Debug, Deserialize)]
pub struct BeltConfig {
    /// Path to the pipeline YAML file, relative to the config file's directory.
    pub pipeline: String,
}

/// Parse a `belt.toml` file at the given path.
///
/// # Errors
///
/// Returns `BeltError::FileNotFound` if `path` does not exist or is unreadable,
/// or `BeltError::ConfigParse` if the TOML content cannot be deserialized into
/// a [`BeltConfig`].
pub fn parse_config(path: &Path) -> BeltResult<BeltConfig> {
    let content = std::fs::read_to_string(path).map_err(|_| BeltError::FileNotFound {
        path: path.display().to_string(),
    })?;
    let config: BeltConfig = toml::from_str(&content).map_err(|e| BeltError::ConfigParse {
        path: path.display().to_string(),
        detail: e.message().to_string(),
    })?;
    Ok(config)
}

/// Resolve the pipeline file path from a config file's location.
///
/// Joins the config file's parent directory with the `pipeline` field.
/// Does not verify the resolved path exists.
#[must_use]
pub fn resolve_pipeline_path(config_path: &Path, config: &BeltConfig) -> PathBuf {
    let base_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    base_dir.join(&config.pipeline)
}
