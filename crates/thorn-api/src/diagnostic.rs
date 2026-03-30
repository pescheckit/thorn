use serde::{Deserialize, Serialize};

/// A byte offset range in source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteRange {
    pub start: u32,
    pub end: u32,
}

impl ByteRange {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
}

impl From<(u32, u32)> for ByteRange {
    fn from((start, end): (u32, u32)) -> Self {
        Self { start, end }
    }
}

/// What level of checking should surface this diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Level {
    /// Bugs, security, data loss — shown in `--check fix`
    Fix,
    /// Performance, deprecation — shown in `--check improve` (default)
    Improve,
    /// Code style, complexity — shown in `--check all`
    All,
}

impl Level {
    pub fn label(&self) -> &'static str {
        match self {
            Level::Fix => "fix",
            Level::Improve => "improve",
            Level::All => "all",
        }
    }
}

fn default_level() -> Level {
    Level::Improve
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub range: Option<ByteRange>,
    pub filename: String,
    #[serde(default)]
    pub line: Option<u32>,
    #[serde(default)]
    pub col: Option<u32>,
    #[serde(default = "default_level")]
    pub level: Level,
}

impl Diagnostic {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        filename: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            range: None,
            filename: filename.into(),
            line: None,
            col: None,
            level: Level::Improve,
        }
    }

    pub fn with_range(mut self, range: ByteRange) -> Self {
        self.range = Some(range);
        self
    }

    pub fn with_level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    pub fn resolve_location(&mut self, source: &str) {
        if let Some(range) = self.range {
            let offset = range.start as usize;
            if offset <= source.len() {
                let before = &source[..offset];
                let line = before.chars().filter(|c| *c == '\n').count() + 1;
                let col = before.rfind('\n').map_or(offset + 1, |nl| offset - nl);
                self.line = Some(line as u32);
                self.col = Some(col as u32);
            }
        }
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.line, self.col) {
            (Some(line), Some(col)) => write!(
                f,
                "{}:{}:{}: {} {}",
                self.filename, line, col, self.code, self.message
            ),
            (Some(line), None) => write!(
                f,
                "{}:{}: {} {}",
                self.filename, line, self.code, self.message
            ),
            _ => write!(f, "{}: {} {}", self.filename, self.code, self.message),
        }
    }
}
