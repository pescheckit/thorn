use crate::ByteRange;

/// Helper: any struct with a `range: ByteRange` field gets a `.range()` accessor.
macro_rules! impl_range {
    ($($ty:ident),* $(,)?) => {
        $(impl $ty {
            pub fn range(&self) -> ByteRange { self.range }
        })*
    };
}

/// A parsed Python module — the root AST node.
#[derive(Debug, Clone)]
pub struct Module {
    pub body: Vec<Stmt>,
    pub range: ByteRange,
}

// ── Statements ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Stmt {
    FunctionDef(StmtFunctionDef),
    ClassDef(StmtClassDef),
    Return(StmtReturn),
    Assign(StmtAssign),
    AnnAssign(StmtAnnAssign),
    AugAssign(StmtAugAssign),
    For(StmtFor),
    While(StmtWhile),
    If(StmtIf),
    With(StmtWith),
    Raise(StmtRaise),
    Try(StmtTry),
    Assert(StmtAssert),
    Import(StmtImport),
    ImportFrom(StmtImportFrom),
    Global(StmtGlobal),
    Nonlocal(StmtNonlocal),
    Expr(StmtExpr),
    Pass(StmtPass),
    Break(StmtBreak),
    Continue(StmtContinue),
    Delete(StmtDelete),
}

impl Stmt {
    pub fn range(&self) -> ByteRange {
        match self {
            Stmt::FunctionDef(s) => s.range,
            Stmt::ClassDef(s) => s.range,
            Stmt::Return(s) => s.range,
            Stmt::Assign(s) => s.range,
            Stmt::AnnAssign(s) => s.range,
            Stmt::AugAssign(s) => s.range,
            Stmt::For(s) => s.range,
            Stmt::While(s) => s.range,
            Stmt::If(s) => s.range,
            Stmt::With(s) => s.range,
            Stmt::Raise(s) => s.range,
            Stmt::Try(s) => s.range,
            Stmt::Assert(s) => s.range,
            Stmt::Import(s) => s.range,
            Stmt::ImportFrom(s) => s.range,
            Stmt::Global(s) => s.range,
            Stmt::Nonlocal(s) => s.range,
            Stmt::Expr(s) => s.range,
            Stmt::Pass(s) => s.range,
            Stmt::Break(s) => s.range,
            Stmt::Continue(s) => s.range,
            Stmt::Delete(s) => s.range,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StmtFunctionDef {
    pub name: String,
    pub parameters: Parameters,
    pub body: Vec<Stmt>,
    pub decorator_list: Vec<Expr>,
    pub returns: Option<Box<Expr>>,
    pub is_async: bool,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtClassDef {
    pub name: String,
    pub arguments: Option<Box<Arguments>>,
    pub body: Vec<Stmt>,
    pub decorator_list: Vec<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtReturn {
    pub value: Option<Box<Expr>>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtAssign {
    pub targets: Vec<Expr>,
    pub value: Box<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtAnnAssign {
    pub target: Box<Expr>,
    pub annotation: Box<Expr>,
    pub value: Option<Box<Expr>>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtAugAssign {
    pub target: Box<Expr>,
    pub op: Operator,
    pub value: Box<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtFor {
    pub target: Box<Expr>,
    pub iter: Box<Expr>,
    pub body: Vec<Stmt>,
    pub orelse: Vec<Stmt>,
    pub is_async: bool,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtWhile {
    pub test: Box<Expr>,
    pub body: Vec<Stmt>,
    pub orelse: Vec<Stmt>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtIf {
    pub test: Box<Expr>,
    pub body: Vec<Stmt>,
    pub elif_else_clauses: Vec<ElifElseClause>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ElifElseClause {
    pub test: Option<Expr>,
    pub body: Vec<Stmt>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtWith {
    pub items: Vec<WithItem>,
    pub body: Vec<Stmt>,
    pub is_async: bool,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct WithItem {
    pub context_expr: Expr,
    pub optional_vars: Option<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtRaise {
    pub exc: Option<Box<Expr>>,
    pub cause: Option<Box<Expr>>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtTry {
    pub body: Vec<Stmt>,
    pub handlers: Vec<ExceptHandler>,
    pub orelse: Vec<Stmt>,
    pub finalbody: Vec<Stmt>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExceptHandler {
    pub type_: Option<Box<Expr>>,
    pub name: Option<String>,
    pub body: Vec<Stmt>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtAssert {
    pub test: Box<Expr>,
    pub msg: Option<Box<Expr>>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtImport {
    pub names: Vec<Alias>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtImportFrom {
    pub module: Option<String>,
    pub names: Vec<Alias>,
    pub level: u32,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct Alias {
    pub name: String,
    pub asname: Option<String>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtGlobal {
    pub names: Vec<String>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtNonlocal {
    pub names: Vec<String>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtExpr {
    pub value: Box<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtPass {
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtBreak {
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtContinue {
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct StmtDelete {
    pub targets: Vec<Expr>,
    pub range: ByteRange,
}

// ── Expressions ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Expr {
    BoolOp(ExprBoolOp),
    Named(ExprNamed),
    BinOp(ExprBinOp),
    UnaryOp(ExprUnaryOp),
    Lambda(ExprLambda),
    If(ExprIf),
    Dict(ExprDict),
    Set(ExprSet),
    ListComp(ExprListComp),
    SetComp(ExprSetComp),
    DictComp(ExprDictComp),
    Generator(ExprGenerator),
    Await(ExprAwait),
    Yield(ExprYield),
    YieldFrom(ExprYieldFrom),
    Compare(ExprCompare),
    Call(ExprCall),
    FString(ExprFString),
    StringLiteral(ExprStringLiteral),
    BytesLiteral(ExprBytesLiteral),
    NumberLiteral(ExprNumberLiteral),
    BooleanLiteral(ExprBooleanLiteral),
    NoneLiteral(ExprNoneLiteral),
    EllipsisLiteral(ExprEllipsisLiteral),
    Attribute(ExprAttribute),
    Subscript(ExprSubscript),
    Starred(ExprStarred),
    Name(ExprName),
    List(ExprList),
    Tuple(ExprTuple),
}

impl Expr {
    pub fn range(&self) -> ByteRange {
        match self {
            Expr::BoolOp(e) => e.range,
            Expr::Named(e) => e.range,
            Expr::BinOp(e) => e.range,
            Expr::UnaryOp(e) => e.range,
            Expr::Lambda(e) => e.range,
            Expr::If(e) => e.range,
            Expr::Dict(e) => e.range,
            Expr::Set(e) => e.range,
            Expr::ListComp(e) => e.range,
            Expr::SetComp(e) => e.range,
            Expr::DictComp(e) => e.range,
            Expr::Generator(e) => e.range,
            Expr::Await(e) => e.range,
            Expr::Yield(e) => e.range,
            Expr::YieldFrom(e) => e.range,
            Expr::Compare(e) => e.range,
            Expr::Call(e) => e.range,
            Expr::FString(e) => e.range,
            Expr::StringLiteral(e) => e.range,
            Expr::BytesLiteral(e) => e.range,
            Expr::NumberLiteral(e) => e.range,
            Expr::BooleanLiteral(e) => e.range,
            Expr::NoneLiteral(e) => e.range,
            Expr::EllipsisLiteral(e) => e.range,
            Expr::Attribute(e) => e.range,
            Expr::Subscript(e) => e.range,
            Expr::Starred(e) => e.range,
            Expr::Name(e) => e.range,
            Expr::List(e) => e.range,
            Expr::Tuple(e) => e.range,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExprBoolOp {
    pub op: BoolOp,
    pub values: Vec<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprNamed {
    pub target: Box<Expr>,
    pub value: Box<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprBinOp {
    pub left: Box<Expr>,
    pub op: Operator,
    pub right: Box<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprUnaryOp {
    pub op: UnaryOp,
    pub operand: Box<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprLambda {
    pub parameters: Option<Box<Parameters>>,
    pub body: Box<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprIf {
    pub test: Box<Expr>,
    pub body: Box<Expr>,
    pub orelse: Box<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprDict {
    pub items: Vec<DictItem>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct DictItem {
    pub key: Option<Expr>,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct ExprSet {
    pub elts: Vec<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprListComp {
    pub elt: Box<Expr>,
    pub generators: Vec<Comprehension>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprSetComp {
    pub elt: Box<Expr>,
    pub generators: Vec<Comprehension>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprDictComp {
    pub key: Box<Expr>,
    pub value: Box<Expr>,
    pub generators: Vec<Comprehension>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprGenerator {
    pub elt: Box<Expr>,
    pub generators: Vec<Comprehension>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct Comprehension {
    pub target: Expr,
    pub iter: Expr,
    pub ifs: Vec<Expr>,
    pub is_async: bool,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprAwait {
    pub value: Box<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprYield {
    pub value: Option<Box<Expr>>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprYieldFrom {
    pub value: Box<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprCompare {
    pub left: Box<Expr>,
    pub ops: Vec<CmpOp>,
    pub comparators: Vec<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprCall {
    pub func: Box<Expr>,
    pub arguments: Arguments,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprFString {
    pub parts: Vec<FStringPart>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub enum FStringPart {
    Literal(String),
    Expression(FStringExpression),
}

#[derive(Debug, Clone)]
pub struct FStringExpression {
    pub value: Box<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprStringLiteral {
    pub value: String,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprBytesLiteral {
    pub value: Vec<u8>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprNumberLiteral {
    pub value: Number,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub enum Number {
    Int(i64),
    Float(f64),
    Complex { real: f64, imag: f64 },
}

#[derive(Debug, Clone)]
pub struct ExprBooleanLiteral {
    pub value: bool,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprNoneLiteral {
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprEllipsisLiteral {
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprAttribute {
    pub value: Box<Expr>,
    pub attr: String,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprSubscript {
    pub value: Box<Expr>,
    pub slice: Box<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprStarred {
    pub value: Box<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprName {
    pub id: String,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprList {
    pub elts: Vec<Expr>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct ExprTuple {
    pub elts: Vec<Expr>,
    pub range: ByteRange,
}

// ── Common types ───────────────────────────────────────────────────

/// Function/method call arguments.
#[derive(Debug, Clone)]
pub struct Arguments {
    pub args: Vec<Expr>,
    pub keywords: Vec<Keyword>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct Keyword {
    pub arg: Option<String>,
    pub value: Expr,
    pub range: ByteRange,
}

/// Function parameter list.
#[derive(Debug, Clone)]
pub struct Parameters {
    pub args: Vec<Parameter>,
    pub vararg: Option<Box<Parameter>>,
    pub kwonlyargs: Vec<Parameter>,
    pub kwarg: Option<Box<Parameter>>,
    pub posonlyargs: Vec<Parameter>,
    pub range: ByteRange,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub annotation: Option<Box<Expr>>,
    pub default: Option<Box<Expr>>,
    pub range: ByteRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOp {
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Add,
    Sub,
    Mult,
    Div,
    Mod,
    Pow,
    LShift,
    RShift,
    BitOr,
    BitXor,
    BitAnd,
    FloorDiv,
    MatMult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Invert,
    Not,
    UAdd,
    USub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    NotEq,
    Lt,
    LtE,
    Gt,
    GtE,
    Is,
    IsNot,
    In,
    NotIn,
}

// Add .range() methods to all AST node structs
impl_range!(
    Module,
    StmtFunctionDef,
    StmtClassDef,
    StmtReturn,
    StmtAssign,
    StmtAnnAssign,
    StmtAugAssign,
    StmtFor,
    StmtWhile,
    StmtIf,
    StmtWith,
    StmtRaise,
    StmtTry,
    StmtAssert,
    StmtImport,
    StmtImportFrom,
    StmtGlobal,
    StmtNonlocal,
    StmtExpr,
    StmtPass,
    StmtBreak,
    StmtContinue,
    StmtDelete,
    ElifElseClause,
    WithItem,
    ExceptHandler,
    Alias,
    ExprBoolOp,
    ExprNamed,
    ExprBinOp,
    ExprUnaryOp,
    ExprLambda,
    ExprIf,
    ExprDict,
    ExprSet,
    ExprListComp,
    ExprSetComp,
    ExprDictComp,
    ExprGenerator,
    ExprAwait,
    ExprYield,
    ExprYieldFrom,
    ExprCompare,
    ExprCall,
    ExprFString,
    ExprStringLiteral,
    ExprBytesLiteral,
    ExprNumberLiteral,
    ExprBooleanLiteral,
    ExprNoneLiteral,
    ExprEllipsisLiteral,
    ExprAttribute,
    ExprSubscript,
    ExprStarred,
    ExprName,
    ExprList,
    ExprTuple,
    Arguments,
    Keyword,
    Parameters,
    Parameter,
    Comprehension,
    FStringExpression,
);
