//! Config loading from pyproject.toml — [tool.thorn] section only.
//!
//! Framework-specific config (like [tool.pylint]) is handled by each
//! plugin, not here.

use std::path::Path;

/// Thorn configuration loaded from [tool.thorn] in pyproject.toml.
#[derive(Debug, Default)]
pub struct ThornConfig {
    /// Paths/patterns to exclude from linting.
    pub exclude: Vec<String>,
    /// Paths/patterns to include (overrides exclude).
    pub include: Vec<String>,
    /// Rule codes to ignore globally (e.g. ["DJ001", "DJ026"]).
    pub ignore: Vec<String>,
}

impl ThornConfig {
    /// Load config from pyproject.toml in the given directory.
    pub fn from_project_dir(dir: &Path) -> Self {
        match find_pyproject(dir) {
            Some(path) => Self::from_pyproject(&path).unwrap_or_default(),
            None => Self::default(),
        }
    }

    fn from_pyproject(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let doc: toml::Value = content.parse()?;

        let mut config = Self::default();

        if let Some(thorn) = doc.get("tool").and_then(|t| t.get("thorn")) {
            if let Some(exclude) = thorn.get("exclude").and_then(|v| v.as_array()) {
                config.exclude = exclude
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
            if let Some(include) = thorn.get("include").and_then(|v| v.as_array()) {
                config.include = include
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
            if let Some(ignore) = thorn.get("ignore").and_then(|v| v.as_array()) {
                config.ignore = ignore
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
        }

        Ok(config)
    }
}

/// Walk up the directory tree to find pyproject.toml.
pub fn find_pyproject(dir: &Path) -> Option<std::path::PathBuf> {
    let mut current = if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(dir)
    };
    loop {
        let candidate = current.join("pyproject.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }
    None
}
