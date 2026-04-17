#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test: panic-on-mismatch is the intended assertion style"
)]

use belt_core::config::{parse_config, resolve_pipeline_path};
use belt_core::error::BeltError;
use std::io::Write;
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};

/// scenario: belt-core-config-valid-toml-parses
#[test]
fn parse_valid_config() {
    let mut f = NamedTempFile::new().expect("failed to create temp file");
    write!(f, r#"pipeline = "pipeline.yml""#).expect("failed to write");

    let config = parse_config(f.path()).expect("parse_config should succeed");
    assert_eq!(config.pipeline, "pipeline.yml");
}

/// scenario: belt-core-config-missing-file-yields-file-not-found
#[test]
fn parse_config_missing_file() {
    let result = parse_config(Path::new("/nonexistent/belt.toml"));
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        BeltError::FileNotFound { .. }
    ));
}

/// scenario: belt-core-config-invalid-toml-yields-config-parse
#[test]
fn parse_config_invalid_toml() {
    let mut f = NamedTempFile::new().expect("failed to create temp file");
    write!(f, "not valid toml [[[").expect("failed to write");

    let result = parse_config(f.path());
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), BeltError::ConfigParse { .. }));
}

/// scenario: belt-core-config-missing-pipeline-file-field-yields-config-parse
#[test]
fn parse_config_missing_pipeline_field() {
    let mut f = NamedTempFile::new().expect("failed to create temp file");
    write!(f, r#"something_else = "value""#).expect("failed to write");

    let result = parse_config(f.path());
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), BeltError::ConfigParse { .. }));
}

/// scenario: belt-core-config-resolves-relative-pipeline-path
#[test]
fn resolve_pipeline_path_relative_to_config() {
    let dir = TempDir::new().expect("failed to create temp dir");
    let config_path = dir.path().join("belt.toml");
    std::fs::write(&config_path, r#"pipeline = "pipeline.yml""#).expect("write");

    let config = parse_config(&config_path).expect("parse");
    let resolved = resolve_pipeline_path(&config_path, &config);
    assert_eq!(resolved, dir.path().join("pipeline.yml"));
}

/// scenario: belt-core-config-resolves-subdirectory-pipeline-path
#[test]
fn resolve_pipeline_path_with_subdirectory() {
    let dir = TempDir::new().expect("failed to create temp dir");
    let config_path = dir.path().join("belt.toml");
    std::fs::write(&config_path, r#"pipeline = "pipelines/main.yml""#).expect("write");

    let config = parse_config(&config_path).expect("parse");
    let resolved = resolve_pipeline_path(&config_path, &config);
    assert_eq!(resolved, dir.path().join("pipelines/main.yml"));
}

/// scenario: belt-core-config-preserves-absolute-pipeline-path
#[test]
fn preserves_absolute_pipeline_path() {
    use belt_core::config::BeltConfig;
    use std::path::PathBuf;

    let config = BeltConfig {
        pipeline: "/tmp/absolute/pipeline.yml".to_string(),
    };
    let config_path = Path::new("/some/config/dir/belt.toml");

    let resolved = resolve_pipeline_path(config_path, &config);

    assert_eq!(
        resolved,
        PathBuf::from("/tmp/absolute/pipeline.yml"),
        "absolute pipeline path in belt.toml must be returned unchanged (not joined with config_dir)"
    );
}
