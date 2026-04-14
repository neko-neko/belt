//! belt:// URI parser for cross-run artifact references.
//!
//! Pure module: no filesystem or git access. All resolution is performed
//! by belt-agent against a persisted `.belt/runs/*/state.json` index.

use serde::{Deserialize, Serialize};

/// Parsed belt:// URI used in `ArtifactRef::External { uri, ... }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeltUri {
    /// `belt://latest/{pipeline}/<path>` — COMPLETED latest run of `pipeline`
    /// on the *current* branch. `path` is relative to the resolved run dir.
    Latest { pipeline: String, path: String },
    /// `belt://workspace/{branch}/latest/{pipeline}/<path>` — COMPLETED latest
    /// of `pipeline` on *explicit* `branch`.
    WorkspaceLatest {
        branch: String,
        pipeline: String,
        path: String,
    },
    /// `belt://run/{run_id}/<path>` — explicit `run_id` (branch-independent).
    Run { run_id: String, path: String },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UriParseError {
    #[error("URI must start with 'belt://': got '{0}'")]
    MissingScheme(String),
    #[error("unknown selector in URI '{uri}' (expected latest/, workspace/, or run/)")]
    UnknownSelector { uri: String },
    #[error("empty pipeline name in URI '{uri}'")]
    EmptyPipeline { uri: String },
    #[error("empty run_id in URI '{uri}'")]
    EmptyRunId { uri: String },
    #[error("empty path in URI '{uri}'")]
    EmptyPath { uri: String },
    #[error("path traversal not allowed in URI '{uri}'")]
    PathTraversal { uri: String },
    #[error("malformed URI '{uri}': {detail}")]
    Malformed { uri: String, detail: String },
}

impl BeltUri {
    /// Parse a string into a `BeltUri`. Pure, deterministic, no I/O.
    pub fn parse(s: &str) -> Result<Self, UriParseError> {
        let rest = s
            .strip_prefix("belt://")
            .ok_or_else(|| UriParseError::MissingScheme(s.to_string()))?;

        // Selector prefix match.
        if let Some(r) = rest.strip_prefix("latest/") {
            // <pipeline>/<path...>
            let (pipeline, path) = split_once_or_err(r, s)?;
            if pipeline.is_empty() {
                return Err(UriParseError::EmptyPipeline { uri: s.to_string() });
            }
            if path.is_empty() {
                return Err(UriParseError::EmptyPath { uri: s.to_string() });
            }
            return Ok(BeltUri::Latest {
                pipeline: pipeline.to_string(),
                path: path.to_string(),
            });
        }

        if let Some(r) = rest.strip_prefix("workspace/") {
            // <branch>/latest/<pipeline>/<path...>
            let (branch, after_branch) = split_once_or_err(r, s)?;
            let after_branch =
                after_branch
                    .strip_prefix("latest/")
                    .ok_or_else(|| UriParseError::Malformed {
                        uri: s.to_string(),
                        detail: "expected 'latest/' after branch".to_string(),
                    })?;
            let (pipeline, path) = split_once_or_err(after_branch, s)?;
            if branch.is_empty() {
                return Err(UriParseError::Malformed {
                    uri: s.to_string(),
                    detail: "empty branch".to_string(),
                });
            }
            if pipeline.is_empty() {
                return Err(UriParseError::EmptyPipeline { uri: s.to_string() });
            }
            if path.is_empty() {
                return Err(UriParseError::EmptyPath { uri: s.to_string() });
            }
            return Ok(BeltUri::WorkspaceLatest {
                branch: branch.to_string(),
                pipeline: pipeline.to_string(),
                path: path.to_string(),
            });
        }

        if let Some(r) = rest.strip_prefix("run/") {
            // <run_id>/<path...>
            let (run_id, path) = split_once_or_err(r, s)?;
            if run_id.is_empty() {
                return Err(UriParseError::EmptyRunId { uri: s.to_string() });
            }
            if path.is_empty() {
                return Err(UriParseError::EmptyPath { uri: s.to_string() });
            }
            return Ok(BeltUri::Run {
                run_id: run_id.to_string(),
                path: path.to_string(),
            });
        }

        Err(UriParseError::UnknownSelector { uri: s.to_string() })
    }
}

fn split_once_or_err<'a>(s: &'a str, original: &str) -> Result<(&'a str, &'a str), UriParseError> {
    s.split_once('/').ok_or_else(|| UriParseError::Malformed {
        uri: original.to_string(),
        detail: "missing path segment separator '/'".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_latest_happy_path() {
        let u = BeltUri::parse("belt://latest/feature-dev/notes/phase-review.md").unwrap();
        assert_eq!(
            u,
            BeltUri::Latest {
                pipeline: "feature-dev".into(),
                path: "notes/phase-review.md".into(),
            }
        );
    }

    #[test]
    fn parse_workspace_latest_happy_path() {
        let u = BeltUri::parse("belt://workspace/develop/latest/feature-dev/notes/phase-review.md")
            .unwrap();
        assert_eq!(
            u,
            BeltUri::WorkspaceLatest {
                branch: "develop".into(),
                pipeline: "feature-dev".into(),
                path: "notes/phase-review.md".into(),
            }
        );
    }

    #[test]
    fn parse_run_happy_path() {
        let u =
            BeltUri::parse("belt://run/01947abc-0000-7000-8000-000000000000/notes/phase-review.md")
                .unwrap();
        assert_eq!(
            u,
            BeltUri::Run {
                run_id: "01947abc-0000-7000-8000-000000000000".into(),
                path: "notes/phase-review.md".into(),
            }
        );
    }
}
