use crate::error::{BeltError, BeltResult};
use crate::model::{GateDefinition, Pipeline, SubPipeline};
use std::path::Path;

/// Parse a top-level pipeline definition from a YAML file.
///
/// # Errors
///
/// Returns `BeltError::FileNotFound` if `path` does not exist or is unreadable,
/// or `BeltError::YamlParse` if the YAML content cannot be deserialized into a
/// [`Pipeline`].
pub fn parse_pipeline(path: &Path) -> BeltResult<Pipeline> {
    let content = std::fs::read_to_string(path).map_err(|_| BeltError::FileNotFound {
        path: path.display().to_string(),
    })?;
    let pipeline: Pipeline =
        serde_saphyr::from_str(&content).map_err(|e| BeltError::YamlParse {
            message: e.to_string(),
            src: Some(content.clone()),
        })?;
    Ok(pipeline)
}

/// Parse a reusable gate definition from a YAML file.
///
/// # Errors
///
/// Returns `BeltError::FileNotFound` if `path` does not exist or is unreadable,
/// or `BeltError::YamlParse` if the YAML content cannot be deserialized into a
/// [`GateDefinition`].
pub fn parse_gate_definition(path: &Path) -> BeltResult<GateDefinition> {
    let content = std::fs::read_to_string(path).map_err(|_| BeltError::FileNotFound {
        path: path.display().to_string(),
    })?;
    let gate_def: GateDefinition =
        serde_saphyr::from_str(&content).map_err(|e| BeltError::YamlParse {
            message: e.to_string(),
            src: Some(content.clone()),
        })?;
    Ok(gate_def)
}

/// Parse a sub-pipeline definition from a YAML file.
///
/// # Errors
///
/// Returns `BeltError::FileNotFound` if `path` does not exist or is unreadable,
/// or `BeltError::YamlParse` if the YAML content cannot be deserialized into a
/// [`SubPipeline`].
pub fn parse_sub_pipeline(path: &Path) -> BeltResult<SubPipeline> {
    let content = std::fs::read_to_string(path).map_err(|_| BeltError::FileNotFound {
        path: path.display().to_string(),
    })?;
    let sub: SubPipeline = serde_saphyr::from_str(&content).map_err(|e| BeltError::YamlParse {
        message: e.to_string(),
        src: Some(content.clone()),
    })?;
    Ok(sub)
}
