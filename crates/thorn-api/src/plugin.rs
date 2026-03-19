use ruff_python_ast::ModModule;

use crate::diagnostic::{Diagnostic, Level};
use crate::graph::AppGraph;

pub struct CheckContext<'a> {
    pub module: &'a ModModule,
    pub source: &'a str,
    pub filename: &'a str,
    pub graph: &'a AppGraph,
}

impl<'a> CheckContext<'a> {
    pub fn diag(&self, code: &str, msg: impl Into<String>, range: ruff_text_size::TextRange) -> Diagnostic {
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

pub trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn prefix(&self) -> &'static str;
    fn on_graph_ready(&mut self, _graph: &AppGraph) {}
    fn ast_checks(&self) -> Vec<Box<dyn AstCheck>> { vec![] }
    fn graph_checks(&self) -> Vec<Box<dyn GraphCheck>> { vec![] }
    fn project_checks(&self, _project_dir: &std::path::Path, _toml: &str) -> Vec<Diagnostic> { vec![] }
    fn read_config_excludes(&self, _toml: &str) -> Vec<String> { vec![] }
}

pub trait AstCheck: Send + Sync {
    fn code(&self) -> &'static str;
    fn level(&self) -> Level { Level::Improve }
    fn check(&self, ctx: &CheckContext) -> Vec<Diagnostic>;
}

pub trait GraphCheck: Send + Sync {
    fn code(&self) -> &'static str;
    fn level(&self) -> Level { Level::Improve }
    fn check(&self, graph: &AppGraph) -> Vec<Diagnostic>;
}
