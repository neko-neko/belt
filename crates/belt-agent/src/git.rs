use std::path::Path;
use std::process::Command;

/// Attempt to resolve the current branch name via `git rev-parse
/// --abbrev-ref HEAD`. Returns `None` when:
/// - git is not available
/// - the directory is not a git repo
/// - HEAD is detached (rev-parse returns literal "HEAD")
/// - the repo has no commits yet (rev-parse fails with "ambiguous argument")
#[must_use]
pub(crate) fn current_branch(work_dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(work_dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8(output.stdout).ok()?;
    let name = name.trim().to_string();
    if name.is_empty() || name == "HEAD" {
        return None;
    }
    Some(name)
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
    fn current_branch_returns_none_for_non_git_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(current_branch(tmp.path()), None);
    }

    #[test]
    fn current_branch_returns_name_for_git_dir() {
        let tmp = tempfile::tempdir().unwrap();
        // init a git repo with a branch
        let status = Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(tmp.path())
            .status()
            .unwrap();
        assert!(status.success());
        // `git rev-parse --abbrev-ref HEAD` requires at least one commit to
        // resolve; a freshly-initialized empty repo emits the literal string
        // "HEAD" and exits 128. Create an empty commit with inline identity
        // so the test is independent of the user's global git config.
        let status = Command::new("git")
            .args([
                "-c",
                "user.name=belt-test",
                "-c",
                "user.email=belt-test@example.invalid",
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .current_dir(tmp.path())
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(current_branch(tmp.path()), Some("main".to_string()));
    }
}
