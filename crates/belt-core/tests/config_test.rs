use belt_core::config::{parse_config, resolve_pipeline_path};
use belt_core::error::BeltError;
use std::io::Write;
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn parse_valid_config() {
    let mut f = NamedTempFile::new().expect("failed to create temp file");
    write!(f, r#"pipeline = "pipeline.yml""#).expect("failed to write");

    let config = parse_config(f.path()).expect("parse_config should succeed");
    assert_eq!(config.pipeline, "pipeline.yml");
}

#[test]
fn parse_config_missing_file() {
    let result = parse_config(Path::new("/nonexistent/belt.toml"));
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        BeltError::FileNotFound { .. }
    ));
}

#[test]
fn parse_config_invalid_toml() {
    let mut f = NamedTempFile::new().expect("failed to create temp file");
    write!(f, "not valid toml [[[").expect("failed to write");

    let result = parse_config(f.path());
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), BeltError::ConfigParse { .. }));
}

#[test]
fn parse_config_missing_pipeline_field() {
    let mut f = NamedTempFile::new().expect("failed to create temp file");
    write!(f, r#"something_else = "value""#).expect("failed to write");

    let result = parse_config(f.path());
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), BeltError::ConfigParse { .. }));
}

#[test]
fn resolve_pipeline_path_relative_to_config() {
    let dir = TempDir::new().expect("failed to create temp dir");
    let config_path = dir.path().join("belt.toml");
    std::fs::write(&config_path, r#"pipeline = "pipeline.yml""#).expect("write");

    let config = parse_config(&config_path).expect("parse");
    let resolved = resolve_pipeline_path(&config_path, &config);
    assert_eq!(resolved, dir.path().join("pipeline.yml"));
}

#[test]
fn resolve_pipeline_path_with_subdirectory() {
    let dir = TempDir::new().expect("failed to create temp dir");
    let config_path = dir.path().join("belt.toml");
    std::fs::write(&config_path, r#"pipeline = "pipelines/main.yml""#).expect("write");

    let config = parse_config(&config_path).expect("parse");
    let resolved = resolve_pipeline_path(&config_path, &config);
    assert_eq!(resolved, dir.path().join("pipelines/main.yml"));
}
