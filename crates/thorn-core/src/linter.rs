use rayon::prelude::*;
use ruff_python_parser::parse;
use std::path::Path;
use thorn_api::{AppGraph, CheckContext, Diagnostic, Plugin};

use crate::discover;
use crate::suppress;

/// The main linter engine. Holds registered plugins and runs checks.
pub struct Linter {
    plugins: Vec<Box<dyn Plugin>>,
    graph: AppGraph,
    exclude_patterns: Vec<String>,
}

impl Linter {
    pub fn new(graph: AppGraph) -> Self {
        Self {
            plugins: vec![],
            graph,
            exclude_patterns: vec![],
        }
    }

    /// Set glob patterns to exclude (e.g. `"*/migrations/*"`).
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

        for d in &mut diagnostics {
            d.resolve_location(source);
        }

        suppress::filter_suppressed(&mut diagnostics, source);
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

    /// Lint an entire directory tree in parallel, including project-level checks.
    pub fn lint_dir_with_config(&self, dir: &Path, toml_content: &str) -> Vec<Diagnostic> {
        let mut diagnostics = self.lint_dir(dir);

        for plugin in &self.plugins {
            diagnostics.extend(plugin.project_checks(dir, toml_content));
        }

        diagnostics
    }

    /// Lint an entire directory tree in parallel using rayon.
    pub fn lint_dir(&self, dir: &Path) -> Vec<Diagnostic> {
        let files = discover::python_files(dir, &self.exclude_patterns);

        let mut diagnostics: Vec<Diagnostic> = files
            .par_iter()
            .flat_map(|path| self.lint_file(path))
            .collect();

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

        diagnostics.sort_by(|a, b| {
            a.filename.cmp(&b.filename).then_with(|| {
                let a_start = a.range.map(|r| r.start());
                let b_start = b.range.map(|r| r.start());
                a_start.cmp(&b_start)
            })
        });

        diagnostics
    }

    /// Get a summary of registered plugins: `(name, prefix)`.
    pub fn plugin_summary(&self) -> Vec<(&str, &str)> {
        self.plugins
            .iter()
            .map(|p| (p.name(), p.prefix()))
            .collect()
    }

    /// Ask all registered plugins for their framework-specific excludes.
    pub fn plugin_config_excludes(&self, toml_content: &str) -> Vec<Vec<String>> {
        self.plugins
            .iter()
            .map(|p| p.read_config_excludes(toml_content))
            .collect()
    }
}
