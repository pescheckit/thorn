use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Discover all `.py` files under a directory, respecting exclude patterns.
///
/// Automatically skips `.venv`, `node_modules`, `__pycache__`, and `.git` directories.
pub fn python_files(dir: &Path, excludes: &[String]) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "py"))
        .filter(|e| {
            let path = e.path().to_string_lossy();
            !path.contains("/.venv/")
                && !path.contains("/node_modules/")
                && !path.contains("/__pycache__/")
                && !path.contains("/.git/")
        })
        .filter(|e| {
            if excludes.is_empty() {
                return true;
            }
            let path = e.path().to_string_lossy();
            !excludes.iter().any(|pattern| match_glob(pattern, &path))
        })
        .map(|e| e.path().to_owned())
        .collect()
}

/// Simple glob matching — supports `*` (any chars) and `**` (any path segments).
fn match_glob(pattern: &str, path: &str) -> bool {
    let trimmed = pattern.trim_matches('*');
    if pattern.starts_with('*') && pattern.ends_with('*') {
        path.contains(trimmed)
    } else if pattern.starts_with('*') {
        path.ends_with(trimmed)
    } else if pattern.ends_with('*') {
        path.contains(trimmed)
    } else {
        path.contains(pattern)
    }
}
