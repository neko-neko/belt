use belt_core::uri::BeltUri;
use std::path::{Path, PathBuf};

/// Resolution errors encountered by belt-agent when mapping a `BeltUri`
/// to an absolute filesystem path.
#[derive(Debug, thiserror::Error)]
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
    /// `belt://current/...` URI を解決しようとしたが invocation context に
    /// run_id が無い (`--run` 未指定 + latest run 不在)。
    #[error("belt://current/ requires a current run (none found, pass --run <id>)")]
    NoCurrentRun,
}

#[derive(Debug)]
pub(crate) struct Resolver<'a> {
    pub belt_dir: &'a Path,
    pub current_branch: Option<String>,
    /// Resolved `--run` arg (or latest run id) used to bind `belt://current/`
    /// URIs to a concrete run directory. `None` when no run is in scope
    /// (e.g. before any `init`).
    pub current_run_id: Option<String>,
}

impl Resolver<'_> {
    pub(crate) fn resolve(&self, uri: &BeltUri) -> Result<PathBuf, ResolveError> {
        match uri {
            BeltUri::Run { run_id, path } => self.resolve_run(run_id, path),
            BeltUri::Latest { pipeline, path } => self.resolve_latest(pipeline, path, None),
            BeltUri::WorkspaceLatest {
                branch,
                pipeline,
                path,
            } => {
                if self.current_branch.is_none() {
                    return Err(ResolveError::BranchAwareRequiresGit);
                }
                self.resolve_latest(pipeline, path, Some(branch))
            }
            BeltUri::Current { path } => self.resolve_current(path),
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

    fn resolve_current(&self, path: &str) -> Result<PathBuf, ResolveError> {
        let run_id = self
            .current_run_id
            .as_ref()
            .ok_or(ResolveError::NoCurrentRun)?;
        let run_dir = self.belt_dir.join("runs").join(run_id);
        if !run_dir.is_dir() {
            return Err(ResolveError::RunNotFound {
                run_id: run_id.clone(),
            });
        }
        // existence assertion is the caller's concern (write targets vs
        // read targets); resolver only computes the path.
        Ok(run_dir.join(path))
    }

    fn resolve_latest(
        &self,
        pipeline: &str,
        path: &str,
        explicit_branch: Option<&str>,
    ) -> Result<PathBuf, ResolveError> {
        let runs_dir = self.belt_dir.join("runs");
        let mut candidates: Vec<(String, PathBuf)> = Vec::new();
        if runs_dir.is_dir() {
            for entry in std::fs::read_dir(&runs_dir)? {
                let entry = entry?;
                let state_path = entry.path().join("state.json");
                if !state_path.is_file() {
                    continue;
                }
                let content = std::fs::read_to_string(&state_path)?;
                let v: serde_json::Value = serde_json::from_str(&content)?;
                let p_name = v.get("pipeline").and_then(|x| x.as_str()).unwrap_or("");
                let p_status = v
                    .get("status")
                    .and_then(|x| x.as_str())
                    .unwrap_or("in_progress");
                let p_branch = v.get("branch").and_then(|x| x.as_str());

                if p_status != "completed" {
                    continue;
                }
                if p_name != pipeline {
                    continue;
                }
                // Branch filter:
                match (explicit_branch, &self.current_branch) {
                    (Some(target), _) => {
                        if p_branch != Some(target) {
                            continue;
                        }
                    }
                    (None, Some(current)) => {
                        if p_branch != Some(current.as_str()) {
                            continue;
                        }
                    }
                    (None, None) => {
                        // current_branch == None: no branch filter.
                    }
                }

                let run_id = v
                    .get("run_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string();
                candidates.push((run_id, entry.path()));
            }
        }

        // Pick max run_id lexicographically (UUIDv7 = time-ordered).
        candidates.sort_by(|a, b| a.0.cmp(&b.0));
        let (_run_id, chosen_run_dir) = candidates.pop().ok_or(ResolveError::NoCompletedRun {
            pipeline: pipeline.to_string(),
            branch: explicit_branch
                .map(String::from)
                .or_else(|| self.current_branch.clone()),
        })?;

        let abs = chosen_run_dir.join(path);
        if !abs.exists() {
            return Err(ResolveError::ArtifactMissing {
                path: abs.display().to_string(),
            });
        }
        Ok(abs)
    }
}

impl belt_core::gate::UriResolver for Resolver<'_> {
    fn resolve(&self, uri: &str) -> Result<std::path::PathBuf, String> {
        let parsed = BeltUri::parse(uri).map_err(|e| e.to_string())?;
        Resolver::resolve(self, &parsed).map_err(|e| e.to_string())
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
            current_run_id: None,
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
            current_run_id: None,
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
            current_run_id: None,
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

    fn write_state(
        belt_dir: &Path,
        run_id: &str,
        pipeline: &str,
        branch: Option<&str>,
        status: &str,
    ) {
        let dir = belt_dir.join("runs").join(run_id);
        fs::create_dir_all(dir.join("notes")).unwrap();
        fs::write(dir.join("notes").join("phase-review.md"), "x").unwrap();
        let branch_json = match branch {
            Some(b) => format!("\"{b}\""),
            None => "null".to_string(),
        };
        let state = format!(
            r#"{{
  "run_id": "{run_id}",
  "pipeline": "{pipeline}",
  "pipeline_file": "/tmp/x.yml",
  "version": 1,
  "branch": {branch_json},
  "args": {{}},
  "current_phase": "review",
  "completed_phases": [],
  "skipped_phases": [],
  "status": "{status}",
  "created_at": "2026-04-14T00:00:00Z",
  "updated_at": "2026-04-14T00:00:00Z"
}}"#
        );
        fs::write(dir.join("state.json"), state).unwrap();
    }

    #[test]
    fn resolve_latest_picks_completed_on_current_branch() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        // two runs on main, one in_progress and one completed. Also a
        // completed run on a different branch.
        write_state(
            &belt_dir,
            "01947a00",
            "feature-dev",
            Some("main"),
            "in_progress",
        );
        write_state(
            &belt_dir,
            "01947a01",
            "feature-dev",
            Some("main"),
            "completed",
        );
        write_state(
            &belt_dir,
            "01947a02",
            "feature-dev",
            Some("develop"),
            "completed",
        );

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: Some("main".to_string()),
            current_run_id: None,
        };
        let uri = BeltUri::Latest {
            pipeline: "feature-dev".into(),
            path: "notes/phase-review.md".into(),
        };
        let resolved = r.resolve(&uri).unwrap();
        assert!(
            resolved.ends_with("01947a01/notes/phase-review.md"),
            "got {}",
            resolved.display()
        );
    }

    #[test]
    fn resolve_latest_prefers_newer_uuidv7() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        write_state(
            &belt_dir,
            "01947aaa",
            "feature-dev",
            Some("main"),
            "completed",
        );
        write_state(
            &belt_dir,
            "01947bbb",
            "feature-dev",
            Some("main"),
            "completed",
        );

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: Some("main".to_string()),
            current_run_id: None,
        };
        let uri = BeltUri::Latest {
            pipeline: "feature-dev".into(),
            path: "notes/phase-review.md".into(),
        };
        let resolved = r.resolve(&uri).unwrap();
        assert!(resolved.ends_with("01947bbb/notes/phase-review.md"));
    }

    #[test]
    fn resolve_latest_errors_when_no_completed() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        write_state(
            &belt_dir,
            "01947a00",
            "feature-dev",
            Some("main"),
            "in_progress",
        );

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: Some("main".to_string()),
            current_run_id: None,
        };
        let uri = BeltUri::Latest {
            pipeline: "feature-dev".into(),
            path: "notes/phase-review.md".into(),
        };
        assert!(matches!(
            r.resolve(&uri),
            Err(ResolveError::NoCompletedRun { .. })
        ));
    }

    #[test]
    fn resolve_latest_falls_back_when_branch_none() {
        // non-git or detached HEAD: branch filter is disabled, all branches candidate.
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        write_state(&belt_dir, "01947a00", "feature-dev", None, "completed");
        write_state(
            &belt_dir,
            "01947a01",
            "feature-dev",
            Some("develop"),
            "completed",
        );

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
            current_run_id: None,
        };
        let uri = BeltUri::Latest {
            pipeline: "feature-dev".into(),
            path: "notes/phase-review.md".into(),
        };
        let resolved = r.resolve(&uri).unwrap();
        assert!(resolved.ends_with("01947a01/notes/phase-review.md"));
    }

    #[test]
    fn resolve_workspace_latest_uses_explicit_branch() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        write_state(
            &belt_dir,
            "01947a00",
            "feature-dev",
            Some("main"),
            "completed",
        );
        write_state(
            &belt_dir,
            "01947a01",
            "feature-dev",
            Some("develop"),
            "completed",
        );

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: Some("main".to_string()),
            current_run_id: None,
        };
        let uri = BeltUri::WorkspaceLatest {
            branch: "develop".into(),
            pipeline: "feature-dev".into(),
            path: "notes/phase-review.md".into(),
        };
        let resolved = r.resolve(&uri).unwrap();
        assert!(resolved.ends_with("01947a01/notes/phase-review.md"));
    }

    #[test]
    fn resolve_workspace_latest_errors_on_non_git() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        write_state(
            &belt_dir,
            "01947a00",
            "feature-dev",
            Some("develop"),
            "completed",
        );

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None, // non-git
            current_run_id: None,
        };
        let uri = BeltUri::WorkspaceLatest {
            branch: "develop".into(),
            pipeline: "feature-dev".into(),
            path: "notes/phase-review.md".into(),
        };
        assert!(matches!(
            r.resolve(&uri),
            Err(ResolveError::BranchAwareRequiresGit)
        ));
    }

    /// BELT-35 adversarial probe: a truncated state.json surfaces a loud
    /// `StateParse` error rather than silently selecting a different
    /// candidate or coercing to empty. Documents current fail-loud
    /// behaviour for corrupt JSON.
    #[test]
    fn resolve_latest_errors_on_corrupt_state_json() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let run_dir = belt_dir.join("runs").join("01947cor");
        fs::create_dir_all(run_dir.join("notes")).unwrap();
        fs::write(run_dir.join("notes").join("phase-review.md"), "x").unwrap();
        // Truncated mid-field — not valid JSON.
        fs::write(run_dir.join("state.json"), r#"{"run_id": "trun"#).unwrap();

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
            current_run_id: None,
        };
        let uri = BeltUri::Latest {
            pipeline: "feature-dev".into(),
            path: "notes/phase-review.md".into(),
        };
        assert!(matches!(r.resolve(&uri), Err(ResolveError::StateParse(_))));
    }

    /// BELT-35 adversarial probe: state.json with a missing `pipeline`
    /// field is *silently skipped* by the candidate loop — it falls back
    /// to an empty pipeline name via `unwrap_or("")`, mismatches the
    /// requested pipeline, and is dropped. With no other candidate, the
    /// resolver returns `NoCompletedRun`. Documents the current
    /// silent-skip behaviour for schema-missing state.json. A loud
    /// variant is explicitly out of scope (see Non-Goals in the spec).
    #[test]
    fn resolve_latest_skips_state_json_without_pipeline_field() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let run_dir = belt_dir.join("runs").join("01947mis");
        fs::create_dir_all(run_dir.join("notes")).unwrap();
        fs::write(run_dir.join("notes").join("phase-review.md"), "x").unwrap();
        // Valid JSON, but `pipeline` field is missing.
        fs::write(
            run_dir.join("state.json"),
            r#"{"run_id": "01947mis", "status": "completed", "branch": null}"#,
        )
        .unwrap();

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
            current_run_id: None,
        };
        let uri = BeltUri::Latest {
            pipeline: "feature-dev".into(),
            path: "notes/phase-review.md".into(),
        };
        assert!(matches!(
            r.resolve(&uri),
            Err(ResolveError::NoCompletedRun { .. })
        ));
    }

    /// BELT-35 adversarial probe: when the `state.json` path is a
    /// directory (not a file), `resolver.rs:80`'s
    /// `if !state_path.is_file() { continue; }` silently skips the run.
    /// With no other candidate, the resolver returns `NoCompletedRun`.
    /// Documents that we do NOT surface an `Io` error in this case —
    /// the `read_to_string` call is never reached.
    #[test]
    fn resolve_latest_skips_state_json_that_is_a_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let run_dir = belt_dir.join("runs").join("01947dir");
        // state.json is a directory, not a file.
        fs::create_dir_all(run_dir.join("state.json")).unwrap();
        fs::create_dir_all(run_dir.join("notes")).unwrap();
        fs::write(run_dir.join("notes").join("phase-review.md"), "x").unwrap();

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
            current_run_id: None,
        };
        let uri = BeltUri::Latest {
            pipeline: "feature-dev".into(),
            path: "notes/phase-review.md".into(),
        };
        assert!(matches!(
            r.resolve(&uri),
            Err(ResolveError::NoCompletedRun { .. })
        ));
    }

    #[test]
    fn resolve_current_returns_run_dir_path() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let run_dir = belt_dir.join("runs").join("01947abc");
        fs::create_dir_all(run_dir.join("notes")).unwrap();

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
            current_run_id: Some("01947abc".to_string()),
        };
        let uri = BeltUri::Current {
            path: "notes/phase-design.md".into(),
        };
        let resolved = r.resolve(&uri).unwrap();
        // existence is NOT asserted by resolve_current (write target case)
        assert_eq!(resolved, run_dir.join("notes").join("phase-design.md"));
    }

    #[test]
    fn resolve_current_errors_when_no_current_run_id() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
            current_run_id: None,
        };
        let uri = BeltUri::Current {
            path: "notes/x.md".into(),
        };
        assert!(matches!(r.resolve(&uri), Err(ResolveError::NoCurrentRun)));
    }

    #[test]
    fn resolve_current_errors_when_run_dir_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
            current_run_id: Some("missing".to_string()),
        };
        let uri = BeltUri::Current {
            path: "notes/x.md".into(),
        };
        assert!(matches!(
            r.resolve(&uri),
            Err(ResolveError::RunNotFound { .. })
        ));
    }

    #[test]
    fn impl_uri_resolver_trait_for_current_uri() {
        use belt_core::gate::UriResolver as _;
        let tmp = tempfile::tempdir().unwrap();
        let belt_dir = tmp.path().join(".belt");
        let run_dir = belt_dir.join("runs").join("01947zzz");
        fs::create_dir_all(run_dir.join("notes")).unwrap();

        let r = Resolver {
            belt_dir: &belt_dir,
            current_branch: None,
            current_run_id: Some("01947zzz".to_string()),
        };
        let resolved = <Resolver<'_> as belt_core::gate::UriResolver>::resolve(
            &r,
            "belt://current/notes/phase-design.md",
        )
        .unwrap();
        assert_eq!(resolved, run_dir.join("notes").join("phase-design.md"));
    }
}
