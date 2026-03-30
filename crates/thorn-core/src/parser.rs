use thorn_api::ast::*;
use thorn_api::ByteRange;
use tree_sitter::{Node, Parser};

fn range_of(node: Node) -> ByteRange {
    ByteRange::new(node.start_byte() as u32, node.end_byte() as u32)
}

fn text_of<'a>(node: Node<'a>, src: &'a str) -> &'a str {
    &src[node.start_byte()..node.end_byte()]
}

pub fn parse_python(source: &str) -> Option<Module> {
    let mut parser = Parser::new();
    let lang = tree_sitter_python::LANGUAGE;
    parser.set_language(&lang.into()).ok()?;
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();
    Some(Module {
        body: parse_body(root, source),
        range: range_of(root),
    })
}

fn parse_body(node: Node, src: &str) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(stmt) = parse_stmt(child, src) {
            stmts.push(stmt);
        }
    }
    stmts
}

fn parse_block(node: Node, src: &str) -> Vec<Stmt> {
    if node.kind() == "block" {
        parse_body(node, src)
    } else if let Some(stmt) = parse_stmt(node, src) {
        vec![stmt]
    } else {
        vec![]
    }
}

fn parse_stmt(node: Node, src: &str) -> Option<Stmt> {
    match node.kind() {
        "function_definition" | "decorated_definition" => parse_function_def(node, src),
        "class_definition" => parse_class_def(node, src, vec![]),
        "return_statement" => parse_return(node, src),
        "expression_statement" => parse_expr_stmt(node, src),
        "assignment" => parse_assignment(node, src),
        "augmented_assignment" => parse_aug_assignment(node, src),
        "type_alias_statement" => None, // skip type aliases for now
        "for_statement" => parse_for(node, src),
        "while_statement" => parse_while(node, src),
        "if_statement" => parse_if(node, src),
        "with_statement" => parse_with(node, src),
        "raise_statement" => parse_raise(node, src),
        "try_statement" => parse_try(node, src),
        "assert_statement" => parse_assert(node, src),
        "import_statement" => parse_import(node, src),
        "import_from_statement" => parse_import_from(node, src),
        "global_statement" => parse_global(node, src),
        "nonlocal_statement" => parse_nonlocal(node, src),
        "pass_statement" => Some(Stmt::Pass(StmtPass {
            range: range_of(node),
        })),
        "break_statement" => Some(Stmt::Break(StmtBreak {
            range: range_of(node),
        })),
        "continue_statement" => Some(Stmt::Continue(StmtContinue {
            range: range_of(node),
        })),
        "delete_statement" => parse_delete(node, src),
        _ => None,
    }
}

fn parse_function_def(node: Node, src: &str) -> Option<Stmt> {
    let mut decorators = Vec::new();

    if node.kind() == "decorated_definition" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "decorator" {
                if let Some(expr_node) = child.child(1) {
                    if let Some(expr) = parse_expr(expr_node, src) {
                        decorators.push(expr);
                    }
                }
            } else if child.kind() == "function_definition" {
                return Some(Stmt::FunctionDef(build_function_def(
                    child, src, decorators, false,
                )));
            } else if child.kind() == "class_definition" {
                return parse_class_def(child, src, decorators);
            }
        }
        return None;
    }

    let is_async = text_of(node, src).starts_with("async");
    Some(Stmt::FunctionDef(build_function_def(
        node, src, decorators, is_async,
    )))
}

fn build_function_def(
    node: Node,
    src: &str,
    decorators: Vec<Expr>,
    is_async: bool,
) -> StmtFunctionDef {
    let name = node
        .child_by_field_name("name")
        .map(|n| text_of(n, src).to_string())
        .unwrap_or_default();

    let parameters = node
        .child_by_field_name("parameters")
        .map(|n| parse_parameters(n, src))
        .unwrap_or(Parameters {
            args: vec![],
            vararg: None,
            kwonlyargs: vec![],
            kwarg: None,
            posonlyargs: vec![],
            range: range_of(node),
        });

    let returns = node
        .child_by_field_name("return_type")
        .and_then(|n| parse_expr(n, src))
        .map(Box::new);

    let body = node
        .child_by_field_name("body")
        .map(|n| parse_block(n, src))
        .unwrap_or_default();

    StmtFunctionDef {
        name,
        parameters,
        body,
        decorator_list: decorators,
        returns,
        is_async,
        range: range_of(node),
    }
}

fn parse_parameters(node: Node, src: &str) -> Parameters {
    let mut args = Vec::new();
    let mut vararg = None;
    let mut kwonlyargs = Vec::new();
    let mut kwarg = None;
    let mut seen_star = false;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "typed_parameter" | "default_parameter" | "typed_default_parameter" => {
                let param = parse_parameter(child, src);
                if seen_star {
                    kwonlyargs.push(param);
                } else {
                    args.push(param);
                }
            }
            "list_splat_pattern" | "dictionary_splat_pattern" => {
                // *args or **kwargs
                let is_kwargs = child.kind() == "dictionary_splat_pattern";
                let inner = child.child(1).or_else(|| child.child(0));
                let name = inner
                    .map(|n| text_of(n, src).to_string())
                    .unwrap_or_default();
                let param = Parameter {
                    name,
                    annotation: None,
                    default: None,
                    range: range_of(child),
                };
                if is_kwargs {
                    kwarg = Some(Box::new(param));
                } else {
                    vararg = Some(Box::new(param));
                    seen_star = true;
                }
            }
            "*" => {
                seen_star = true;
            }
            _ => {}
        }
    }

    Parameters {
        args,
        vararg,
        kwonlyargs,
        kwarg,
        posonlyargs: vec![],
        range: range_of(node),
    }
}

fn parse_parameter(node: Node, src: &str) -> Parameter {
    match node.kind() {
        "identifier" => Parameter {
            name: text_of(node, src).to_string(),
            annotation: None,
            default: None,
            range: range_of(node),
        },
        "typed_parameter" => {
            let name = node
                .child(0)
                .map(|n| text_of(n, src).to_string())
                .unwrap_or_default();
            let annotation = node
                .child_by_field_name("type")
                .and_then(|n| parse_expr(n, src))
                .map(Box::new);
            Parameter {
                name,
                annotation,
                default: None,
                range: range_of(node),
            }
        }
        "default_parameter" => {
            let name_node = node.child_by_field_name("name");
            let name = name_node
                .map(|n| text_of(n, src).to_string())
                .unwrap_or_default();
            let default = node
                .child_by_field_name("value")
                .and_then(|n| parse_expr(n, src))
                .map(Box::new);
            Parameter {
                name,
                annotation: None,
                default,
                range: range_of(node),
            }
        }
        "typed_default_parameter" => {
            let name_node = node.child_by_field_name("name");
            let name = name_node
                .map(|n| text_of(n, src).to_string())
                .unwrap_or_default();
            let annotation = node
                .child_by_field_name("type")
                .and_then(|n| parse_expr(n, src))
                .map(Box::new);
            let default = node
                .child_by_field_name("value")
                .and_then(|n| parse_expr(n, src))
                .map(Box::new);
            Parameter {
                name,
                annotation,
                default,
                range: range_of(node),
            }
        }
        _ => Parameter {
            name: text_of(node, src).to_string(),
            annotation: None,
            default: None,
            range: range_of(node),
        },
    }
}

fn parse_class_def(node: Node, src: &str, decorators: Vec<Expr>) -> Option<Stmt> {
    let name = node
        .child_by_field_name("name")
        .map(|n| text_of(n, src).to_string())
        .unwrap_or_default();

    let arguments = node
        .child_by_field_name("superclasses")
        .map(|n| Box::new(parse_arguments(n, src)));

    let body = node
        .child_by_field_name("body")
        .map(|n| parse_block(n, src))
        .unwrap_or_default();

    Some(Stmt::ClassDef(StmtClassDef {
        name,
        arguments,
        body,
        decorator_list: decorators,
        range: range_of(node),
    }))
}

fn parse_return(node: Node, src: &str) -> Option<Stmt> {
    let value = node.child(1).and_then(|n| parse_expr(n, src)).map(Box::new);
    Some(Stmt::Return(StmtReturn {
        value,
        range: range_of(node),
    }))
}

fn parse_expr_stmt(node: Node, src: &str) -> Option<Stmt> {
    // expression_statement can contain assignment too
    let child = node.child(0)?;
    if child.kind() == "assignment" {
        return parse_assignment(child, src);
    }
    if child.kind() == "augmented_assignment" {
        return parse_aug_assignment(child, src);
    }
    let expr = parse_expr(child, src)?;
    Some(Stmt::Expr(StmtExpr {
        value: Box::new(expr),
        range: range_of(node),
    }))
}

fn parse_assignment(node: Node, src: &str) -> Option<Stmt> {
    let left = node
        .child_by_field_name("left")
        .and_then(|n| parse_expr(n, src));
    let right = node
        .child_by_field_name("right")
        .and_then(|n| parse_expr(n, src));

    // Check for type annotation
    let type_node = node.child_by_field_name("type");
    if let Some(type_n) = type_node {
        let target = left.map(Box::new)?;
        let annotation = parse_expr(type_n, src).map(Box::new)?;
        let value = right.map(Box::new);
        return Some(Stmt::AnnAssign(StmtAnnAssign {
            target,
            annotation,
            value,
            range: range_of(node),
        }));
    }

    let target = left?;
    let value = right.map(Box::new)?;
    Some(Stmt::Assign(StmtAssign {
        targets: vec![target],
        value,
        range: range_of(node),
    }))
}

fn parse_aug_assignment(node: Node, src: &str) -> Option<Stmt> {
    let target = node
        .child_by_field_name("left")
        .and_then(|n| parse_expr(n, src))
        .map(Box::new)?;
    let value = node
        .child_by_field_name("right")
        .and_then(|n| parse_expr(n, src))
        .map(Box::new)?;
    let op_text = node
        .child_by_field_name("operator")
        .map(|n| text_of(n, src))
        .unwrap_or("+=");
    let op = match op_text {
        "+=" => Operator::Add,
        "-=" => Operator::Sub,
        "*=" => Operator::Mult,
        "/=" => Operator::Div,
        "%=" => Operator::Mod,
        "**=" => Operator::Pow,
        "<<=" => Operator::LShift,
        ">>=" => Operator::RShift,
        "|=" => Operator::BitOr,
        "^=" => Operator::BitXor,
        "&=" => Operator::BitAnd,
        "//=" => Operator::FloorDiv,
        "@=" => Operator::MatMult,
        _ => Operator::Add,
    };
    Some(Stmt::AugAssign(StmtAugAssign {
        target,
        op,
        value,
        range: range_of(node),
    }))
}

fn parse_for(node: Node, src: &str) -> Option<Stmt> {
    let target = node
        .child_by_field_name("left")
        .and_then(|n| parse_expr(n, src))
        .map(Box::new)?;
    let iter = node
        .child_by_field_name("right")
        .and_then(|n| parse_expr(n, src))
        .map(Box::new)?;
    let body = node
        .child_by_field_name("body")
        .map(|n| parse_block(n, src))
        .unwrap_or_default();
    let orelse = node
        .child_by_field_name("alternative")
        .map(|n| parse_block(n, src))
        .unwrap_or_default();
    Some(Stmt::For(StmtFor {
        target,
        iter,
        body,
        orelse,
        is_async: text_of(node, src).starts_with("async"),
        range: range_of(node),
    }))
}

fn parse_while(node: Node, src: &str) -> Option<Stmt> {
    let test = node
        .child_by_field_name("condition")
        .and_then(|n| parse_expr(n, src))
        .map(Box::new)?;
    let body = node
        .child_by_field_name("body")
        .map(|n| parse_block(n, src))
        .unwrap_or_default();
    let orelse = node
        .child_by_field_name("alternative")
        .map(|n| parse_block(n, src))
        .unwrap_or_default();
    Some(Stmt::While(StmtWhile {
        test,
        body,
        orelse,
        range: range_of(node),
    }))
}

fn parse_if(node: Node, src: &str) -> Option<Stmt> {
    let test = node
        .child_by_field_name("condition")
        .and_then(|n| parse_expr(n, src))
        .map(Box::new)?;
    let body = node
        .child_by_field_name("consequence")
        .map(|n| parse_block(n, src))
        .unwrap_or_default();

    let mut elif_else_clauses = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "elif_clause" => {
                let elif_test = child
                    .child_by_field_name("condition")
                    .and_then(|n| parse_expr(n, src));
                let elif_body = child
                    .child_by_field_name("consequence")
                    .map(|n| parse_block(n, src))
                    .unwrap_or_default();
                elif_else_clauses.push(ElifElseClause {
                    test: elif_test,
                    body: elif_body,
                    range: range_of(child),
                });
            }
            "else_clause" => {
                let else_body = child
                    .child_by_field_name("body")
                    .map(|n| parse_block(n, src))
                    .unwrap_or_default();
                elif_else_clauses.push(ElifElseClause {
                    test: None,
                    body: else_body,
                    range: range_of(child),
                });
            }
            _ => {}
        }
    }

    Some(Stmt::If(StmtIf {
        test,
        body,
        elif_else_clauses,
        range: range_of(node),
    }))
}

fn parse_with(node: Node, src: &str) -> Option<Stmt> {
    let mut items = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "with_clause" {
            let mut clause_cursor = child.walk();
            for item_node in child.children(&mut clause_cursor) {
                if item_node.kind() == "with_item" {
                    let context_expr = item_node
                        .child_by_field_name("value")
                        .and_then(|n| parse_expr(n, src));
                    let optional_vars = item_node
                        .child_by_field_name("alias")
                        .and_then(|n| parse_expr(n, src));
                    if let Some(ce) = context_expr {
                        items.push(WithItem {
                            context_expr: ce,
                            optional_vars,
                            range: range_of(item_node),
                        });
                    }
                }
            }
        }
    }
    if items.is_empty() {
        // fallback: try direct children
        let mut cursor2 = node.walk();
        for child in node.children(&mut cursor2) {
            if let Some(expr) = parse_expr(child, src) {
                items.push(WithItem {
                    context_expr: expr,
                    optional_vars: None,
                    range: range_of(child),
                });
                break;
            }
        }
    }
    let body = node
        .child_by_field_name("body")
        .map(|n| parse_block(n, src))
        .unwrap_or_default();
    Some(Stmt::With(StmtWith {
        items,
        body,
        is_async: text_of(node, src).starts_with("async"),
        range: range_of(node),
    }))
}

fn parse_raise(node: Node, src: &str) -> Option<Stmt> {
    let mut exc = None;
    let mut cause = None;
    let mut cursor = node.walk();
    let mut seen_from = false;
    for child in node.children(&mut cursor) {
        if child.kind() == "from" {
            seen_from = true;
            continue;
        }
        if child.kind() == "raise" {
            continue;
        }
        if let Some(expr) = parse_expr(child, src) {
            if seen_from {
                cause = Some(Box::new(expr));
            } else {
                exc = Some(Box::new(expr));
            }
        }
    }
    Some(Stmt::Raise(StmtRaise {
        exc,
        cause,
        range: range_of(node),
    }))
}

fn parse_try(node: Node, src: &str) -> Option<Stmt> {
    let mut body = Vec::new();
    let mut handlers = Vec::new();
    let mut orelse = Vec::new();
    let mut finalbody = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "block" => {
                // First block is the try body
                if body.is_empty() && handlers.is_empty() {
                    body = parse_block(child, src);
                }
            }
            "except_clause" | "except_group_clause" => {
                // tree-sitter-python uses field "value" for the exception type.
                // It may be a bare identifier like `ValueError`, or an
                // `as_pattern` like `ValueError as e`.
                let value_node = child.child_by_field_name("value");
                let (type_, name) = match value_node {
                    Some(v) if v.kind() == "as_pattern" => {
                        // `except ValueError as e:` — first named child is the
                        // type, the as_pattern_target holds the binding name.
                        let mut vc = v.walk();
                        let type_expr = v
                            .children(&mut vc)
                            .find(|c| c.is_named() && c.kind() != "as_pattern_target")
                            .and_then(|n| parse_expr(n, src))
                            .map(Box::new);
                        let alias = v
                            .children(&mut v.walk())
                            .find(|c| c.kind() == "as_pattern_target")
                            .and_then(|c| c.child(0))
                            .map(|n| text_of(n, src).to_string());
                        (type_expr, alias)
                    }
                    Some(v) => {
                        // Bare exception type, e.g. `except ValueError:`
                        let type_expr = parse_expr(v, src).map(Box::new);
                        (type_expr, None)
                    }
                    None => (None, None),
                };
                // The except clause body block may not have a field name,
                // so fall back to finding the first "block" child.
                let handler_body = child
                    .child_by_field_name("body")
                    .or_else(|| {
                        let mut ec = child.walk();
                        let block = child.children(&mut ec).find(|c| c.kind() == "block");
                        block
                    })
                    .map(|n| parse_block(n, src))
                    .unwrap_or_default();
                handlers.push(ExceptHandler {
                    type_,
                    name,
                    body: handler_body,
                    range: range_of(child),
                });
            }
            "else_clause" => {
                orelse = child
                    .child_by_field_name("body")
                    .map(|n| parse_block(n, src))
                    .unwrap_or_default();
            }
            "finally_clause" => {
                finalbody = child
                    .child_by_field_name("body")
                    .map(|n| parse_block(n, src))
                    .unwrap_or_default();
            }
            _ => {}
        }
    }

    Some(Stmt::Try(StmtTry {
        body,
        handlers,
        orelse,
        finalbody,
        range: range_of(node),
    }))
}

fn parse_assert(node: Node, src: &str) -> Option<Stmt> {
    let mut exprs = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(expr) = parse_expr(child, src) {
            exprs.push(expr);
        }
    }
    let test = exprs.first().cloned().map(Box::new)?;
    let msg = exprs.get(1).cloned().map(Box::new);
    Some(Stmt::Assert(StmtAssert {
        test,
        msg,
        range: range_of(node),
    }))
}

fn parse_import(node: Node, src: &str) -> Option<Stmt> {
    let mut names = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "dotted_name" || child.kind() == "aliased_import" {
            names.push(parse_alias(child, src));
        }
    }
    Some(Stmt::Import(StmtImport {
        names,
        range: range_of(node),
    }))
}

fn parse_import_from(node: Node, src: &str) -> Option<Stmt> {
    let mut module = None;
    let mut names = Vec::new();
    let mut level = 0u32;

    // Extract module name from the module_name field
    if let Some(mod_node) = node.child_by_field_name("module_name") {
        let t = text_of(mod_node, src);
        // For relative imports like "...foo", count dots and strip them
        level = t.chars().take_while(|c| *c == '.').count() as u32;
        let mod_name = t.trim_start_matches('.');
        if !mod_name.is_empty() {
            module = Some(mod_name.to_string());
        }
    }

    // Check for import_prefix (bare relative import like "from . import x")
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "import_prefix" => {
                level = text_of(child, src).chars().filter(|c| *c == '.').count() as u32;
            }
            "relative_import" => {
                let t = text_of(child, src);
                level = t.chars().take_while(|c| *c == '.').count() as u32;
                let mod_name = t.trim_start_matches('.');
                if !mod_name.is_empty() {
                    module = Some(mod_name.to_string());
                }
            }
            _ => {}
        }
    }

    // Collect imported names using the "name" field
    let mut cursor2 = node.walk();
    for (i, child) in node.children(&mut cursor2).enumerate() {
        let field = node.field_name_for_child(i as u32);
        if field == Some("name") {
            match child.kind() {
                "dotted_name" => {
                    names.push(Alias {
                        name: text_of(child, src).to_string(),
                        asname: None,
                        range: range_of(child),
                    });
                }
                "aliased_import" => {
                    names.push(parse_alias(child, src));
                }
                _ => {}
            }
        }
        if child.kind() == "wildcard_import" {
            names.push(Alias {
                name: "*".to_string(),
                asname: None,
                range: range_of(child),
            });
        }
    }

    Some(Stmt::ImportFrom(StmtImportFrom {
        module,
        names,
        level,
        range: range_of(node),
    }))
}

fn parse_alias(node: Node, src: &str) -> Alias {
    if node.kind() == "aliased_import" {
        let name = node
            .child_by_field_name("name")
            .map(|n| text_of(n, src).to_string())
            .unwrap_or_default();
        let asname = node
            .child_by_field_name("alias")
            .map(|n| text_of(n, src).to_string());
        Alias {
            name,
            asname,
            range: range_of(node),
        }
    } else {
        Alias {
            name: text_of(node, src).to_string(),
            asname: None,
            range: range_of(node),
        }
    }
}

fn parse_global(node: Node, src: &str) -> Option<Stmt> {
    let mut names = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            names.push(text_of(child, src).to_string());
        }
    }
    Some(Stmt::Global(StmtGlobal {
        names,
        range: range_of(node),
    }))
}

fn parse_nonlocal(node: Node, src: &str) -> Option<Stmt> {
    let mut names = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            names.push(text_of(child, src).to_string());
        }
    }
    Some(Stmt::Nonlocal(StmtNonlocal {
        names,
        range: range_of(node),
    }))
}

fn parse_delete(node: Node, src: &str) -> Option<Stmt> {
    let mut targets = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(expr) = parse_expr(child, src) {
            targets.push(expr);
        }
    }
    Some(Stmt::Delete(StmtDelete {
        targets,
        range: range_of(node),
    }))
}

// ── Expression parsing ─────────────────────────────────────────────

/// Extract the content of a string node by looking for the `string_content`
/// child.  Falls back to stripping quotes manually if the child is missing.
fn extract_string_content(node: Node, src: &str) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "string_content" {
            return text_of(child, src).to_string();
        }
    }
    // Fallback: strip quotes manually
    let raw = text_of(node, src);
    strip_string_quotes(raw).to_string()
}

/// Strip surrounding quotes (single, double, triple) and optional prefix
/// (b, r, u, etc.) from a Python string literal.
fn strip_string_quotes(s: &str) -> &str {
    // Skip prefix chars like b, r, u, B, R, U
    let without_prefix = s.trim_start_matches(|c: char| c.is_ascii_alphabetic());
    if let Some(rest) = without_prefix
        .strip_prefix("\"\"\"")
        .and_then(|r| r.strip_suffix("\"\"\""))
    {
        rest
    } else if let Some(rest) = without_prefix
        .strip_prefix("'''")
        .and_then(|r| r.strip_suffix("'''"))
    {
        rest
    } else if let Some(rest) = without_prefix
        .strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
    {
        rest
    } else if let Some(rest) = without_prefix
        .strip_prefix('\'')
        .and_then(|r| r.strip_suffix('\''))
    {
        rest
    } else {
        s
    }
}

fn parse_expr(node: Node, src: &str) -> Option<Expr> {
    match node.kind() {
        "identifier" => Some(Expr::Name(ExprName {
            id: text_of(node, src).to_string(),
            range: range_of(node),
        })),
        "integer" => {
            let val = text_of(node, src)
                .replace('_', "")
                .parse::<i64>()
                .unwrap_or(0);
            Some(Expr::NumberLiteral(ExprNumberLiteral {
                value: Number::Int(val),
                range: range_of(node),
            }))
        }
        "float" => {
            let val = text_of(node, src)
                .replace('_', "")
                .parse::<f64>()
                .unwrap_or(0.0);
            Some(Expr::NumberLiteral(ExprNumberLiteral {
                value: Number::Float(val),
                range: range_of(node),
            }))
        }
        "string" => {
            let raw = text_of(node, src);
            let is_fstring = raw.starts_with("f\"")
                || raw.starts_with("f'")
                || raw.starts_with("F\"")
                || raw.starts_with("F'");
            if is_fstring {
                Some(Expr::FString(ExprFString {
                    parts: vec![FStringPart::Literal(raw.to_string())],
                    range: range_of(node),
                }))
            } else {
                // Extract the string content from the string_content child
                // to avoid including quotes in the value.
                let value = extract_string_content(node, src);
                Some(Expr::StringLiteral(ExprStringLiteral {
                    value,
                    range: range_of(node),
                }))
            }
        }
        "concatenated_string" => {
            // Concatenated strings: "a" "b" -> join contents
            let mut parts = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "string" {
                    let raw = text_of(child, src);
                    let is_fstring = raw.starts_with("f\"")
                        || raw.starts_with("f'")
                        || raw.starts_with("F\"")
                        || raw.starts_with("F'");
                    if is_fstring {
                        // If any part is an f-string, return the whole thing as FString
                        return Some(Expr::FString(ExprFString {
                            parts: vec![FStringPart::Literal(text_of(node, src).to_string())],
                            range: range_of(node),
                        }));
                    }
                    parts.push(extract_string_content(child, src));
                }
            }
            Some(Expr::StringLiteral(ExprStringLiteral {
                value: parts.join(""),
                range: range_of(node),
            }))
        }
        "true" => Some(Expr::BooleanLiteral(ExprBooleanLiteral {
            value: true,
            range: range_of(node),
        })),
        "false" => Some(Expr::BooleanLiteral(ExprBooleanLiteral {
            value: false,
            range: range_of(node),
        })),
        "none" => Some(Expr::NoneLiteral(ExprNoneLiteral {
            range: range_of(node),
        })),
        "ellipsis" => Some(Expr::EllipsisLiteral(ExprEllipsisLiteral {
            range: range_of(node),
        })),
        "attribute" => {
            let obj = node
                .child_by_field_name("object")
                .and_then(|n| parse_expr(n, src))
                .map(Box::new)?;
            let attr = node
                .child_by_field_name("attribute")
                .map(|n| text_of(n, src).to_string())
                .unwrap_or_default();
            Some(Expr::Attribute(ExprAttribute {
                value: obj,
                attr,
                range: range_of(node),
            }))
        }
        "call" => {
            let func = node
                .child_by_field_name("function")
                .and_then(|n| parse_expr(n, src))
                .map(Box::new)?;
            let arguments = node
                .child_by_field_name("arguments")
                .map(|n| parse_arguments(n, src))
                .unwrap_or(Arguments {
                    args: vec![],
                    keywords: vec![],
                    range: range_of(node),
                });
            Some(Expr::Call(ExprCall {
                func,
                arguments,
                range: range_of(node),
            }))
        }
        "subscript" => {
            let value = node
                .child_by_field_name("value")
                .and_then(|n| parse_expr(n, src))
                .map(Box::new)?;
            let slice = node
                .child_by_field_name("subscript")
                .and_then(|n| parse_expr(n, src))
                .map(Box::new)?;
            Some(Expr::Subscript(ExprSubscript {
                value,
                slice,
                range: range_of(node),
            }))
        }
        "binary_operator" => {
            let left = node
                .child_by_field_name("left")
                .and_then(|n| parse_expr(n, src))
                .map(Box::new)?;
            let right = node
                .child_by_field_name("right")
                .and_then(|n| parse_expr(n, src))
                .map(Box::new)?;
            let op_text = node
                .child_by_field_name("operator")
                .map(|n| text_of(n, src))
                .unwrap_or("+");
            let op = match op_text {
                "+" => Operator::Add,
                "-" => Operator::Sub,
                "*" => Operator::Mult,
                "/" => Operator::Div,
                "%" => Operator::Mod,
                "**" => Operator::Pow,
                "<<" => Operator::LShift,
                ">>" => Operator::RShift,
                "|" => Operator::BitOr,
                "^" => Operator::BitXor,
                "&" => Operator::BitAnd,
                "//" => Operator::FloorDiv,
                "@" => Operator::MatMult,
                _ => Operator::Add,
            };
            Some(Expr::BinOp(ExprBinOp {
                left,
                op,
                right,
                range: range_of(node),
            }))
        }
        "unary_operator" => {
            let op_text = node
                .child_by_field_name("operator")
                .map(|n| text_of(n, src))
                .unwrap_or("not");
            let op = match op_text {
                "not" => UnaryOp::Not,
                "-" => UnaryOp::USub,
                "+" => UnaryOp::UAdd,
                "~" => UnaryOp::Invert,
                _ => UnaryOp::Not,
            };
            let operand = node
                .child_by_field_name("argument")
                .and_then(|n| parse_expr(n, src))
                .map(Box::new)?;
            Some(Expr::UnaryOp(ExprUnaryOp {
                op,
                operand,
                range: range_of(node),
            }))
        }
        "boolean_operator" => {
            let left = node
                .child_by_field_name("left")
                .and_then(|n| parse_expr(n, src))?;
            let right = node
                .child_by_field_name("right")
                .and_then(|n| parse_expr(n, src))?;
            let op_text = node
                .child_by_field_name("operator")
                .map(|n| text_of(n, src))
                .unwrap_or("and");
            let op = if op_text == "or" {
                BoolOp::Or
            } else {
                BoolOp::And
            };
            Some(Expr::BoolOp(ExprBoolOp {
                op,
                values: vec![left, right],
                range: range_of(node),
            }))
        }
        "not_operator" => {
            let operand = node
                .child_by_field_name("argument")
                .and_then(|n| parse_expr(n, src))
                .map(Box::new)?;
            Some(Expr::UnaryOp(ExprUnaryOp {
                op: UnaryOp::Not,
                operand,
                range: range_of(node),
            }))
        }
        "comparison_operator" => parse_comparison(node, src),
        "list" => {
            let mut elts = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(expr) = parse_expr(child, src) {
                    elts.push(expr);
                }
            }
            Some(Expr::List(ExprList {
                elts,
                range: range_of(node),
            }))
        }
        "tuple" => {
            let mut elts = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(expr) = parse_expr(child, src) {
                    elts.push(expr);
                }
            }
            Some(Expr::Tuple(ExprTuple {
                elts,
                range: range_of(node),
            }))
        }
        "dictionary" => {
            let mut items = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "pair" {
                    let key = child
                        .child_by_field_name("key")
                        .and_then(|n| parse_expr(n, src));
                    let value = child
                        .child_by_field_name("value")
                        .and_then(|n| parse_expr(n, src));
                    if let Some(v) = value {
                        items.push(DictItem { key, value: v });
                    }
                } else if child.kind() == "dictionary_splat" {
                    if let Some(expr) = child.child(1).and_then(|n| parse_expr(n, src)) {
                        items.push(DictItem {
                            key: None,
                            value: expr,
                        });
                    }
                }
            }
            Some(Expr::Dict(ExprDict {
                items,
                range: range_of(node),
            }))
        }
        "set" => {
            let mut elts = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(expr) = parse_expr(child, src) {
                    elts.push(expr);
                }
            }
            Some(Expr::Set(ExprSet {
                elts,
                range: range_of(node),
            }))
        }
        "list_comprehension" => parse_list_comp(node, src),
        "set_comprehension" => parse_set_comp(node, src),
        "dictionary_comprehension" => parse_dict_comp(node, src),
        "generator_expression" => parse_generator(node, src),
        "lambda" => {
            let parameters = node
                .child_by_field_name("parameters")
                .map(|n| Box::new(parse_parameters(n, src)));
            let body = node
                .child_by_field_name("body")
                .and_then(|n| parse_expr(n, src))
                .map(Box::new)?;
            Some(Expr::Lambda(ExprLambda {
                parameters,
                body,
                range: range_of(node),
            }))
        }
        "conditional_expression" => {
            // body if test else orelse
            let mut children = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "if" && child.kind() != "else" {
                    if let Some(e) = parse_expr(child, src) {
                        children.push(e);
                    }
                }
            }
            if children.len() >= 3 {
                Some(Expr::If(ExprIf {
                    body: Box::new(children.remove(0)),
                    test: Box::new(children.remove(0)),
                    orelse: Box::new(children.remove(0)),
                    range: range_of(node),
                }))
            } else {
                None
            }
        }
        "named_expression" => {
            let target = node
                .child_by_field_name("name")
                .and_then(|n| parse_expr(n, src))
                .map(Box::new)?;
            let value = node
                .child_by_field_name("value")
                .and_then(|n| parse_expr(n, src))
                .map(Box::new)?;
            Some(Expr::Named(ExprNamed {
                target,
                value,
                range: range_of(node),
            }))
        }
        "await" => {
            let value = node
                .child(1)
                .and_then(|n| parse_expr(n, src))
                .map(Box::new)?;
            Some(Expr::Await(ExprAwait {
                value,
                range: range_of(node),
            }))
        }
        "yield" => {
            let value = node.child(1).and_then(|n| parse_expr(n, src)).map(Box::new);
            Some(Expr::Yield(ExprYield {
                value,
                range: range_of(node),
            }))
        }
        "starred_expression" => {
            let value = node
                .child(1)
                .and_then(|n| parse_expr(n, src))
                .map(Box::new)?;
            Some(Expr::Starred(ExprStarred {
                value,
                range: range_of(node),
            }))
        }
        "parenthesized_expression" => node.child(1).and_then(|n| parse_expr(n, src)),
        _ => None,
    }
}

fn parse_comparison(node: Node, src: &str) -> Option<Expr> {
    let mut cursor = node.walk();
    let children: Vec<Node> = node.children(&mut cursor).collect();

    if children.len() < 3 {
        return None;
    }

    let left = parse_expr(children[0], src).map(Box::new)?;
    let mut ops = Vec::new();
    let mut comparators = Vec::new();

    let mut i = 1;
    while i < children.len() {
        let op_text = text_of(children[i], src);
        let op = match op_text {
            "==" => CmpOp::Eq,
            "!=" => CmpOp::NotEq,
            "<" => CmpOp::Lt,
            "<=" => CmpOp::LtE,
            ">" => CmpOp::Gt,
            ">=" => CmpOp::GtE,
            "is" => {
                // check for "is not"
                if i + 1 < children.len() && text_of(children[i + 1], src) == "not" {
                    i += 1;
                    CmpOp::IsNot
                } else {
                    CmpOp::Is
                }
            }
            "in" => CmpOp::In,
            "not" => {
                // "not in"
                if i + 1 < children.len() && text_of(children[i + 1], src) == "in" {
                    i += 1;
                    CmpOp::NotIn
                } else {
                    return None;
                }
            }
            _ => return None,
        };
        ops.push(op);
        i += 1;
        if i < children.len() {
            if let Some(expr) = parse_expr(children[i], src) {
                comparators.push(expr);
            }
        }
        i += 1;
    }

    Some(Expr::Compare(ExprCompare {
        left,
        ops,
        comparators,
        range: range_of(node),
    }))
}

fn parse_arguments(node: Node, src: &str) -> Arguments {
    let mut args = Vec::new();
    let mut keywords = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "keyword_argument" => {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| text_of(n, src).to_string());
                let value = child
                    .child_by_field_name("value")
                    .and_then(|n| parse_expr(n, src));
                if let Some(v) = value {
                    keywords.push(Keyword {
                        arg: name,
                        value: v,
                        range: range_of(child),
                    });
                }
            }
            "dictionary_splat" => {
                if let Some(expr) = child.child(1).and_then(|n| parse_expr(n, src)) {
                    keywords.push(Keyword {
                        arg: None,
                        value: expr,
                        range: range_of(child),
                    });
                }
            }
            "list_splat" => {
                if let Some(expr) = child.child(1).and_then(|n| parse_expr(n, src)) {
                    args.push(Expr::Starred(ExprStarred {
                        value: Box::new(expr),
                        range: range_of(child),
                    }));
                }
            }
            "(" | ")" | "," => {}
            _ => {
                if let Some(expr) = parse_expr(child, src) {
                    args.push(expr);
                }
            }
        }
    }

    Arguments {
        args,
        keywords,
        range: range_of(node),
    }
}

fn parse_comprehension_generators(node: Node, src: &str) -> Vec<Comprehension> {
    let mut generators = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "for_in_clause" {
            let target = child
                .child_by_field_name("left")
                .and_then(|n| parse_expr(n, src))
                .unwrap_or(Expr::Name(ExprName {
                    id: "_".into(),
                    range: range_of(child),
                }));
            let iter = child
                .child_by_field_name("right")
                .and_then(|n| parse_expr(n, src))
                .unwrap_or(Expr::Name(ExprName {
                    id: "_".into(),
                    range: range_of(child),
                }));
            generators.push(Comprehension {
                target,
                iter,
                ifs: vec![],
                is_async: false,
                range: range_of(child),
            });
        } else if child.kind() == "if_clause" {
            let cond = child.child(1).and_then(|n| parse_expr(n, src));
            if let (Some(gen), Some(c)) = (generators.last_mut(), cond) {
                gen.ifs.push(c);
            }
        }
    }
    generators
}

fn parse_list_comp(node: Node, src: &str) -> Option<Expr> {
    let elt = node
        .child_by_field_name("body")
        .and_then(|n| parse_expr(n, src))
        .map(Box::new)?;
    let generators = parse_comprehension_generators(node, src);
    Some(Expr::ListComp(ExprListComp {
        elt,
        generators,
        range: range_of(node),
    }))
}

fn parse_set_comp(node: Node, src: &str) -> Option<Expr> {
    let elt = node
        .child_by_field_name("body")
        .and_then(|n| parse_expr(n, src))
        .map(Box::new)?;
    let generators = parse_comprehension_generators(node, src);
    Some(Expr::SetComp(ExprSetComp {
        elt,
        generators,
        range: range_of(node),
    }))
}

fn parse_dict_comp(node: Node, src: &str) -> Option<Expr> {
    let body = node.child_by_field_name("body");
    let (key, value) = if let Some(b) = body {
        let k = b
            .child_by_field_name("key")
            .and_then(|n| parse_expr(n, src))
            .map(Box::new);
        let v = b
            .child_by_field_name("value")
            .and_then(|n| parse_expr(n, src))
            .map(Box::new);
        (k, v)
    } else {
        (None, None)
    };
    let key = key?;
    let value = value?;
    let generators = parse_comprehension_generators(node, src);
    Some(Expr::DictComp(ExprDictComp {
        key,
        value,
        generators,
        range: range_of(node),
    }))
}

fn parse_generator(node: Node, src: &str) -> Option<Expr> {
    let elt = node
        .child_by_field_name("body")
        .and_then(|n| parse_expr(n, src))
        .map(Box::new)?;
    let generators = parse_comprehension_generators(node, src);
    Some(Expr::Generator(ExprGenerator {
        elt,
        generators,
        range: range_of(node),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_from_module_name() {
        let m = parse_python("from app.models import User\n").unwrap();
        let stmt = &m.body[0];
        if let Stmt::ImportFrom(imp) = stmt {
            assert_eq!(imp.module.as_deref(), Some("app.models"));
            assert_eq!(imp.names.len(), 1);
            assert_eq!(imp.names[0].name, "User");
            assert_eq!(imp.level, 0);
        } else {
            panic!("expected ImportFrom, got {:?}", stmt);
        }
    }

    #[test]
    fn import_from_multiple_names() {
        let m = parse_python("from os.path import join, exists\n").unwrap();
        if let Stmt::ImportFrom(imp) = &m.body[0] {
            assert_eq!(imp.module.as_deref(), Some("os.path"));
            assert_eq!(imp.names.len(), 2);
            assert_eq!(imp.names[0].name, "join");
            assert_eq!(imp.names[1].name, "exists");
        } else {
            panic!("expected ImportFrom");
        }
    }

    #[test]
    fn import_from_with_alias() {
        let m = parse_python("from x import y as z\n").unwrap();
        if let Stmt::ImportFrom(imp) = &m.body[0] {
            assert_eq!(imp.module.as_deref(), Some("x"));
            assert_eq!(imp.names.len(), 1);
            assert_eq!(imp.names[0].name, "y");
            assert_eq!(imp.names[0].asname.as_deref(), Some("z"));
        } else {
            panic!("expected ImportFrom");
        }
    }

    #[test]
    fn import_from_relative() {
        let m = parse_python("from . import foo\n").unwrap();
        if let Stmt::ImportFrom(imp) = &m.body[0] {
            assert!(imp.module.is_none());
            assert_eq!(imp.level, 1);
            assert_eq!(imp.names[0].name, "foo");
        } else {
            panic!("expected ImportFrom");
        }
    }

    #[test]
    fn import_from_wildcard() {
        let m = parse_python("from foo import *\n").unwrap();
        if let Stmt::ImportFrom(imp) = &m.body[0] {
            assert_eq!(imp.module.as_deref(), Some("foo"));
            assert_eq!(imp.names.len(), 1);
            assert_eq!(imp.names[0].name, "*");
        } else {
            panic!("expected ImportFrom");
        }
    }

    #[test]
    fn except_clause_with_type() {
        let m = parse_python("try:\n    pass\nexcept ValueError:\n    pass\n").unwrap();
        if let Stmt::Try(t) = &m.body[0] {
            assert_eq!(t.handlers.len(), 1);
            let h = &t.handlers[0];
            assert!(h.type_.is_some(), "except handler type_ should be Some");
            if let Some(Expr::Name(n)) = h.type_.as_deref() {
                assert_eq!(n.id, "ValueError");
            } else {
                panic!("expected Name expr for exception type, got {:?}", h.type_);
            }
            assert!(h.name.is_none());
        } else {
            panic!("expected Try");
        }
    }

    #[test]
    fn except_clause_with_type_and_name() {
        let m = parse_python("try:\n    pass\nexcept ValueError as e:\n    pass\n").unwrap();
        if let Stmt::Try(t) = &m.body[0] {
            let h = &t.handlers[0];
            assert!(h.type_.is_some());
            if let Some(Expr::Name(n)) = h.type_.as_deref() {
                assert_eq!(n.id, "ValueError");
            } else {
                panic!("expected Name expr, got {:?}", h.type_);
            }
            assert_eq!(h.name.as_deref(), Some("e"));
        } else {
            panic!("expected Try");
        }
    }

    #[test]
    fn except_bare() {
        let m = parse_python("try:\n    pass\nexcept:\n    pass\n").unwrap();
        if let Stmt::Try(t) = &m.body[0] {
            let h = &t.handlers[0];
            assert!(h.type_.is_none());
            assert!(h.name.is_none());
        } else {
            panic!("expected Try");
        }
    }

    #[test]
    fn string_literal_no_quotes() {
        let m = parse_python("x = \"__all__\"\n").unwrap();
        if let Stmt::Assign(a) = &m.body[0] {
            if let Expr::StringLiteral(s) = a.value.as_ref() {
                assert_eq!(s.value, "__all__");
            } else {
                panic!("expected StringLiteral, got {:?}", a.value);
            }
        } else {
            panic!("expected Assign");
        }
    }

    #[test]
    fn string_literal_single_quotes() {
        let m = parse_python("x = '__all__'\n").unwrap();
        if let Stmt::Assign(a) = &m.body[0] {
            if let Expr::StringLiteral(s) = a.value.as_ref() {
                assert_eq!(s.value, "__all__");
            } else {
                panic!("expected StringLiteral");
            }
        } else {
            panic!("expected Assign");
        }
    }

    #[test]
    fn class_with_base() {
        let m = parse_python("class MyForm(forms.ModelForm):\n    pass\n").unwrap();
        if let Stmt::ClassDef(c) = &m.body[0] {
            assert_eq!(c.name, "MyForm");
            let args = c.arguments.as_ref().expect("should have arguments");
            assert_eq!(args.args.len(), 1);
            if let Expr::Attribute(a) = &args.args[0] {
                assert_eq!(a.attr, "ModelForm");
            } else {
                panic!("expected Attribute, got {:?}", args.args[0]);
            }
        } else {
            panic!("expected ClassDef");
        }
    }
}
