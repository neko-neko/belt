use belt_core::uri::BeltUri;
use std::path::{Path, PathBuf};

/// Resolution errors encountered by belt-agent when mapping a `BeltUri`
/// to an absolute filesystem path.
#[derive(Debug, thiserror::Error)]
#[allow(
    dead_code,
    reason = "NoCompletedRun/BranchAwareRequiresGit/Io/StateParse are emitted by Tasks 14-15 (Latest/WorkspaceLatest resolver paths)"
)]
pub(crate) enum ResolveError {
    #[error("run not found: {run_id}")]
    RunNotFound { run_id: String },
    #[error(
        "no COMPLETED run of pipeline '{pipeline}' on branch '{}'",
        branch.as_deref().unwrap_or("(none)")
    )]
    NoCompletedRun {
        pipeline: String,
        branch: Option<String>,
    },
    #[error("branch-aware URI requires git directory")]
    BranchAwareRequiresGit,
    #[error("resolved artifact missing: {path}")]
    ArtifactMissing { path: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("state.json parse error: {0}")]
    StateParse(#[from] serde_json::Error),
}

#[derive(Debug)]
pub(crate) struct Resolver<'a> {
    pub belt_dir: &'a Path,
    #[allow(
        dead_code,
        reason = "consumed by Task 14 (resolve_latest filters runs by current_branch)"
    )]
    pub current_branch: Option<String>,
}

impl Resolver<'_> {
    #[allow(
        dead_code,
        reason = "Tasks 14-15 add Latest/WorkspaceLatest callers; Task 17 wires this into cmd_init"
    )]
    pub(crate) fn resolve(&self, uri: &BeltUri) -> Result<PathBuf, ResolveError> {
        match uri {
            BeltUri::Run { run_id, path } => self.resolve_run(run_id, path),
            BeltUri::Latest { .. } => todo!("Task 14"),
            BeltUri::WorkspaceLatest { .. } => todo!("Task 15"),
        }
    }

    fn resolve_run(&self, run_id: &str, path: &str) -> Result<PathBuf, ResolveError> {
        let run_dir = self.belt_dir.join("runs").join(run_id);
        if !run_dir.is_dir() {
            return Err(ResolveError::RunNotFound {
                run_id: run_id.to_string(),
            });
        }
        let abs = run_dir.join(path);
        if !abs.exists() {
            return Err(ResolveError::ArtifactMissing {
                path: abs.display().to_string(),
            });
        }
        Ok(abs)
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
    use std::fs;

    #[test]
    fn resolve_run_happy_path() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let run_dir = belt_dir.join("runs").join("01947abc");
        fs::create_dir_all(run_dir.join("notes")).unwrap();
        fs::write(run_dir.join("notes").join("phase-review.md"), "x").unwrap();

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
        };
        let uri = BeltUri::Run {
            run_id: "01947abc".into(),
            path: "notes/phase-review.md".into(),
        };
        let resolved = r.resolve(&uri).unwrap();
        assert_eq!(resolved, run_dir.join("notes").join("phase-review.md"));
    }

    #[test]
    fn resolve_run_missing_run_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
        };
        let uri = BeltUri::Run {
            run_id: "nope".into(),
            path: "notes/x.md".into(),
        };
        assert!(matches!(
            r.resolve(&uri),
            Err(ResolveError::RunNotFound { .. })
        ));
    }

    #[test]
    fn resolve_run_missing_artifact() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let run_dir = belt_dir.join("runs").join("01947abc");
        fs::create_dir_all(&run_dir).unwrap();
        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
        };
        let uri = BeltUri::Run {
            run_id: "01947abc".into(),
            path: "notes/missing.md".into(),
        };
        assert!(matches!(
            r.resolve(&uri),
            Err(ResolveError::ArtifactMissing { .. })
        ));
    }
}
