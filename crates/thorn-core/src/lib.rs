use rayon::prelude::*;
use ruff_python_parser::parse;
use std::path::{Path, PathBuf};
use thorn_api::{CheckContext, Diagnostic, ModelGraph, Plugin};
use walkdir::WalkDir;

/// The main linter engine. Holds registered plugins and runs checks.
pub struct Linter {
    plugins: Vec<Box<dyn Plugin>>,
    graph: ModelGraph,
    exclude_patterns: Vec<String>,
}

impl Linter {
    pub fn new(graph: ModelGraph) -> Self {
        Self {
            plugins: vec![],
            graph,
            exclude_patterns: vec![],
        }
    }

    /// Set glob patterns to exclude (e.g. "*/migrations/*").
    pub fn set_excludes(&mut self, patterns: Vec<String>) {
        self.exclude_patterns = patterns;
    }

    /// Register a plugin. Call this before linting.
    pub fn register(&mut self, mut plugin: Box<dyn Plugin>) {
        plugin.on_graph_ready(&self.graph);
        self.plugins.push(plugin);
    }

    /// Lint a single source string.
    pub fn lint_source(&self, source: &str, filename: &str) -> Vec<Diagnostic> {
        let parsed = match parse(source, ruff_python_parser::Mode::Module.into()) {
            Ok(p) => p,
            Err(_) => return vec![],
        };

        let module = parsed
            .into_syntax()
            .module()
            .expect("parsed as module")
            .clone();

        let ctx = CheckContext {
            module: &module,
            source,
            filename,
            graph: &self.graph,
        };

        let mut diagnostics = Vec::new();
        for plugin in &self.plugins {
            for check in plugin.ast_checks() {
                let level = check.level();
                let mut check_diags = check.check(&ctx);
                for d in &mut check_diags {
                    d.level = level;
                }
                diagnostics.extend(check_diags);
            }
        }
        // Resolve byte offsets to line:col
        for d in &mut diagnostics {
            d.resolve_location(source);
        }

        // Filter out diagnostics suppressed by inline comments:
        //   # noqa: DJ001
        //   # noqa: DJ001,DJ002
        //   # thorn: ignore[DJ001]
        //   # thorn: ignore
        let source_lines: Vec<&str> = source.lines().collect();
        diagnostics.retain(|d| {
            let Some(line_num) = d.line else { return true };
            let Some(line) = source_lines.get((line_num - 1) as usize) else {
                return true;
            };

            // Find comment
            let Some(comment_start) = line.find('#') else {
                return true;
            };
            let comment = &line[comment_start..];

            // # noqa: DJ001 or # noqa: DJ001,DJ002
            if let Some(noqa_pos) = comment.find("noqa:") {
                let codes = &comment[noqa_pos + 5..].trim();
                let codes: Vec<&str> = codes.split(',').map(|s| s.trim()).collect();
                if codes.iter().any(|c| *c == d.code) {
                    return false;
                }
            }
            // # noqa (suppress all on this line)
            if comment.contains("noqa") && !comment.contains("noqa:") {
                return false;
            }

            // # thorn: ignore[DJ001] or # thorn: ignore
            if let Some(thorn_pos) = comment.find("thorn: ignore") {
                let rest = &comment[thorn_pos + 13..];
                if rest.starts_with('[') {
                    if let Some(end) = rest.find(']') {
                        let codes = &rest[1..end];
                        let codes: Vec<&str> = codes.split(',').map(|s| s.trim()).collect();
                        if codes.iter().any(|c| *c == d.code) {
                            return false;
                        }
                    }
                } else {
                    // # thorn: ignore (suppress all)
                    return false;
                }
            }

            true
        });

        diagnostics
    }

    /// Lint a single file.
    pub fn lint_file(&self, path: &Path) -> Vec<Diagnostic> {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let filename = path.to_string_lossy();
        self.lint_source(&source, &filename)
    }

    /// Lint an entire directory tree in parallel using rayon.
    /// `toml_content` is the pyproject.toml content for project-level checks.
    pub fn lint_dir_with_config(&self, dir: &Path, toml_content: &str) -> Vec<Diagnostic> {
        let mut diagnostics = self.lint_dir(dir);

        // Run project-level checks (settings analysis, etc.)
        for plugin in &self.plugins {
            diagnostics.extend(plugin.project_checks(dir, toml_content));
        }

        diagnostics
    }

    /// Lint an entire directory tree in parallel using rayon.
    pub fn lint_dir(&self, dir: &Path) -> Vec<Diagnostic> {
        let files = discover_python_files(dir, &self.exclude_patterns);

        // Run AST checks in parallel across files
        let mut diagnostics: Vec<Diagnostic> = files
            .par_iter()
            .flat_map(|path| self.lint_file(path))
            .collect();

        // Run graph checks (once, not per-file)
        for plugin in &self.plugins {
            for check in plugin.graph_checks() {
                let level = check.level();
                let mut check_diags = check.check(&self.graph);
                for d in &mut check_diags {
                    d.level = level;
                }
                diagnostics.extend(check_diags);
            }
        }

        // Sort by filename then offset
        diagnostics.sort_by(|a, b| {
            a.filename.cmp(&b.filename).then_with(|| {
                let a_start = a.range.map(|r| r.start());
                let b_start = b.range.map(|r| r.start());
                a_start.cmp(&b_start)
            })
        });

        diagnostics
    }

    /// Get a summary of registered plugins.
    pub fn plugin_summary(&self) -> Vec<(&str, &str)> {
        self.plugins
            .iter()
            .map(|p| (p.name(), p.prefix()))
            .collect()
    }
}

/// Discover all .py files under a directory, respecting exclude patterns.
fn discover_python_files(dir: &Path, excludes: &[String]) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "py"))
        .filter(|e| {
            let path = e.path().to_string_lossy();
            // Skip common non-source directories
            !path.contains("/.venv/")
                && !path.contains("/node_modules/")
                && !path.contains("/__pycache__/")
                && !path.contains("/.git/")
        })
        .filter(|e| {
            // Apply user exclude patterns
            if excludes.is_empty() {
                return true;
            }
            let path = e.path().to_string_lossy();
            !excludes.iter().any(|pattern| match_glob(pattern, &path))
        })
        .map(|e| e.path().to_owned())
        .collect()
}

/// Simple glob matching — supports * (any chars) and ** (any path segments).
fn match_glob(pattern: &str, path: &str) -> bool {
    // Convert glob to a simple contains check for common patterns
    // "*/migrations/*" → "/migrations/"
    // "*.pyc" → ".pyc" at end
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
