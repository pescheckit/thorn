use ruff_python_ast::ModModule;

use crate::diagnostic::{Diagnostic, Level};
use crate::graph::AppGraph;

/// Result of plugin initialization — plugins return their graph and
/// any diagnostics from dynamic validation.
#[derive(Default)]
pub struct InitResult {
    /// The model/entity graph extracted by this plugin.
    pub graph: AppGraph,
    /// Diagnostics from dynamic validation (e.g., DV001, DV202).
    pub diagnostics: Vec<Diagnostic>,
}

pub struct CheckContext<'a> {
    pub module: &'a ModModule,
    pub source: &'a str,
    pub filename: &'a str,
    pub graph: &'a AppGraph,
}

impl<'a> CheckContext<'a> {
    pub fn diag(
        &self,
        code: &str,
        msg: impl Into<String>,
        range: ruff_text_size::TextRange,
    ) -> Diagnostic {
        Diagnostic {
            code: code.into(),
            message: msg.into(),
            range: Some(range),
            filename: self.filename.into(),
            line: None,
            col: None,
            level: Level::Improve,
        }
    }
}

/// A CLI parameter declared by a plugin.
pub struct PluginParam {
    /// Parameter name (e.g. "settings"). Becomes `--{plugin-name}-{name}` on CLI.
    pub name: &'static str,
    /// Help text shown in --help.
    pub help: &'static str,
    /// Whether this param takes a value (true) or is a flag (false).
    pub takes_value: bool,
}

pub trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn prefix(&self) -> &'static str;
    fn on_graph_ready(&mut self, _graph: &AppGraph) {}

    /// Declare CLI parameters this plugin needs.
    /// Each param becomes `--{name()}-{param.name}` on the CLI.
    /// E.g. plugin "django" with param "settings" → `--django-settings`.
    fn cli_params(&self) -> Vec<PluginParam> {
        vec![]
    }

    /// Called once before linting. Plugins discover and load their own
    /// framework graph, settings, and dynamic diagnostics here.
    /// `project_dir` is the resolved absolute path being linted.
    /// `toml_content` is the pyproject.toml content (may be empty).
    /// `cli_args` contains the plugin's CLI param values (key = param name, value = user input).
    fn initialize(
        &mut self,
        _project_dir: &std::path::Path,
        _toml_content: &str,
        _cli_args: &std::collections::HashMap<String, String>,
    ) -> InitResult {
        InitResult::default()
    }
    fn ast_checks(&self) -> Vec<Box<dyn AstCheck>> {
        vec![]
    }
    fn graph_checks(&self) -> Vec<Box<dyn GraphCheck>> {
        vec![]
    }
    fn project_checks(&self, _project_dir: &std::path::Path, _toml: &str) -> Vec<Diagnostic> {
        vec![]
    }
    fn read_config_excludes(&self, _toml: &str) -> Vec<String> {
        vec![]
    }
}

pub trait AstCheck: Send + Sync {
    fn code(&self) -> &'static str;
    fn level(&self) -> Level {
        Level::Improve
    }
    fn check(&self, ctx: &CheckContext) -> Vec<Diagnostic>;
}

pub trait GraphCheck: Send + Sync {
    fn code(&self) -> &'static str;
    fn level(&self) -> Level {
        Level::Improve
    }
    fn check(&self, graph: &AppGraph) -> Vec<Diagnostic>;
}
