use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper: resolve a fixture file path relative to `CARGO_MANIFEST_DIR`.
pub(crate) fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Helper: write a file inside the given directory and return its path.
pub(crate) fn write_yaml(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    let mut f = std::fs::File::create(&path).expect("failed to create file");
    f.write_all(content.as_bytes())
        .expect("failed to write file");
    path
}

/// Helper: workspace root (two parents up from `CARGO_MANIFEST_DIR` = `crates/belt-core`).
pub(crate) fn repo_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path
}
