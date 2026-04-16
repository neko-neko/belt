use crate::error::{BeltError, BeltResult};
use crate::model::{GateDefinition, Pipeline, SubPipeline};
use std::path::Path;

/// Parse a top-level pipeline definition from an in-memory YAML string.
///
/// Shared deserialization helper so that both `parse_pipeline` (file path
/// entry point) and `lint::lint_pipeline` (which reads the file itself to
/// thread the source through the raw-YAML obsolete-key lint before the
/// typed parse) go through a single `serde_saphyr` call site. This keeps
/// `BeltError::YamlParse` wrapping and any future preflight logic (BOM
/// strip, schema-version gates, etc.) in one place.
///
/// Visibility is `pub(crate)` so the helper is reachable from `lint` but
/// not part of the public API.
///
/// # Errors
///
/// Returns `BeltError::YamlParse` if `content` cannot be deserialized into
/// a [`Pipeline`]. The full source string is attached as `src` so miette's
/// fancy diagnostic renderer can point at the offending span.
pub(crate) fn parse_pipeline_from_str(content: &str) -> BeltResult<Pipeline> {
    serde_saphyr::from_str(content).map_err(|e| BeltError::YamlParse {
        message: e.to_string(),
        src: Some(content.to_string()),
    })
}

/// Parse a top-level pipeline definition from a YAML file.
///
/// # Errors
///
/// Returns `BeltError::FileNotFound` if `path` does not exist or is unreadable,
/// or `BeltError::YamlParse` if the YAML content cannot be deserialized into a
/// [`Pipeline`].
pub fn parse_pipeline(path: &Path) -> BeltResult<Pipeline> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Err(BeltError::FileNotFound {
            path: path.display().to_string(),
        });
    };
    parse_pipeline_from_str(&content)
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
