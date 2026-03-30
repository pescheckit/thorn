use crate::ast::Module;
use crate::diagnostic::{ByteRange, Diagnostic, Level};
use crate::graph::AppGraph;

/// Result of plugin initialization — plugins return their graph and
/// any diagnostics from dynamic validation.
#[derive(Default)]
pub struct InitResult {
    pub graph: AppGraph,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct CheckContext<'a> {
    pub module: &'a Module,
    pub source: &'a str,
    pub filename: &'a str,
    pub graph: &'a AppGraph,
}

impl<'a> CheckContext<'a> {
    pub fn new(
        module: &'a Module,
        source: &'a str,
        filename: &'a str,
        graph: &'a AppGraph,
    ) -> Self {
        Self {
            module,
            source,
            filename,
            graph,
        }
    }

    pub fn diag(&self, code: &str, msg: impl Into<String>, range: ByteRange) -> Diagnostic {
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
    pub name: &'static str,
    pub help: &'static str,
    pub takes_value: bool,
}

pub trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn prefix(&self) -> &'static str;
    fn on_graph_ready(&mut self, _graph: &AppGraph) {}

    fn cli_params(&self) -> Vec<PluginParam> {
        vec![]
    }

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
