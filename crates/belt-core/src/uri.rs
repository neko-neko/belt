//! belt:// URI parser for cross-run artifact references.
//!
//! Pure module: no filesystem or git access. All resolution is performed
//! by belt-agent against a persisted `.belt/runs/*/state.json` index.

/// Parsed belt:// URI used in `ArtifactRef::External { uri, ... }`.
#[derive(Debug, Clone, PartialEq, Eq)]
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
            validate_path(path, s)?;
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
            validate_path(path, s)?;
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
            validate_path(path, s)?;
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

/// Reject paths containing `..` segments or leading `/`.
/// Called after the path has been split out of the URI.
fn validate_path(path: &str, original: &str) -> Result<(), UriParseError> {
    if path.starts_with('/') {
        return Err(UriParseError::PathTraversal {
            uri: original.to_string(),
        });
    }
    for segment in path.split('/') {
        if segment == ".." {
            return Err(UriParseError::PathTraversal {
                uri: original.to_string(),
            });
        }
    }
    Ok(())
}

impl std::fmt::Display for BeltUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BeltUri::Latest { pipeline, path } => {
                write!(f, "belt://latest/{pipeline}/{path}")
            }
            BeltUri::WorkspaceLatest {
                branch,
                pipeline,
                path,
            } => {
                write!(f, "belt://workspace/{branch}/latest/{pipeline}/{path}")
            }
            BeltUri::Run { run_id, path } => {
                write!(f, "belt://run/{run_id}/{path}")
            }
        }
    }
}

impl serde::Serialize for BeltUri {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for BeltUri {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        BeltUri::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "unwrap/panic are conventional assertion failure modes in test-only code"
)]
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

    #[test]
    fn parse_missing_scheme() {
        assert!(matches!(
            BeltUri::parse("https://example.com/foo"),
            Err(UriParseError::MissingScheme(_))
        ));
        assert!(matches!(
            BeltUri::parse(""),
            Err(UriParseError::MissingScheme(_))
        ));
    }

    #[test]
    fn parse_unknown_selector() {
        assert!(matches!(
            BeltUri::parse("belt://unknown/x/y.md"),
            Err(UriParseError::UnknownSelector { .. })
        ));
    }

    #[test]
    fn parse_empty_pipeline() {
        assert!(matches!(
            BeltUri::parse("belt://latest//notes/x.md"),
            Err(UriParseError::EmptyPipeline { .. })
        ));
    }

    #[test]
    fn parse_empty_run_id() {
        assert!(matches!(
            BeltUri::parse("belt://run//notes/x.md"),
            Err(UriParseError::EmptyRunId { .. })
        ));
    }

    #[test]
    fn parse_empty_path() {
        // "belt://latest/feature-dev/" — rest = "latest/feature-dev/"
        // strip "latest/" => "feature-dev/"; split_once('/') => ("feature-dev", "")
        assert!(matches!(
            BeltUri::parse("belt://latest/feature-dev/"),
            Err(UriParseError::EmptyPath { .. })
        ));
    }

    #[test]
    fn parse_path_traversal_rejected() {
        assert!(matches!(
            BeltUri::parse("belt://latest/feature-dev/../etc/passwd"),
            Err(UriParseError::PathTraversal { .. })
        ));
        assert!(matches!(
            BeltUri::parse("belt://latest/feature-dev/notes/../secret"),
            Err(UriParseError::PathTraversal { .. })
        ));
    }

    #[test]
    fn parse_absolute_path_rejected() {
        // Path component starts with '/': "belt://latest/feature-dev//notes/x.md"
        // After split_once, path would be "/notes/x.md" which is absolute-like.
        assert!(matches!(
            BeltUri::parse("belt://latest/feature-dev//notes/x.md"),
            Err(UriParseError::PathTraversal { .. })
        ));
    }

    #[test]
    fn parse_workspace_missing_latest() {
        assert!(matches!(
            BeltUri::parse("belt://workspace/develop/foo/feature-dev/x.md"),
            Err(UriParseError::Malformed { .. })
        ));
    }

    #[test]
    fn to_string_roundtrip_all_variants() {
        for s in [
            "belt://latest/feature-dev/notes/phase-review.md",
            "belt://workspace/develop/latest/feature-dev/notes/phase-review.md",
            "belt://run/01947abc-0000-7000-8000-000000000000/notes/phase-review.md",
        ] {
            let u = BeltUri::parse(s).unwrap();
            assert_eq!(u.to_string(), s);
        }
    }
}
