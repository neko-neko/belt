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
    /// `belt://current/<path>` — runtime invocation context の current run
    /// (`--run` 指定、未指定なら latest run) の `<run_dir>/<path>` に解決される。
    /// pipeline.yml の `produces[].path` / `gate.file_exists` で書き込み先 +
    /// 読み取り先の宣言に使用。
    Current { path: String },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UriParseError {
    #[error("URI must start with 'belt://': got '{0}'")]
    MissingScheme(String),
    #[error("unknown selector in URI '{uri}' (expected latest/, workspace/, current/, or run/)")]
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

        if let Some(r) = rest.strip_prefix("current/") {
            // <path...> — no pipeline / branch / run_id segment
            let path = r;
            if path.is_empty() {
                return Err(UriParseError::EmptyPath { uri: s.to_string() });
            }
            validate_path(path, s)?;
            return Ok(BeltUri::Current {
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
            BeltUri::Current { path } => {
                write!(f, "belt://current/{path}")
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
