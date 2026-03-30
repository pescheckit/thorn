use crate::ast::*;

/// Trait for walking a Python AST. Override the methods you care about
/// and call `walk_*` to continue traversal into child nodes.
pub trait Visitor {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &Expr) {
        walk_expr(self, expr);
    }

    fn visit_body(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.visit_stmt(stmt);
        }
    }
}

pub fn walk_stmt(v: &mut (impl Visitor + ?Sized), stmt: &Stmt) {
    match stmt {
        Stmt::FunctionDef(s) => {
            for decorator in &s.decorator_list {
                v.visit_expr(decorator);
            }
            for param in &s.parameters.args {
                if let Some(ann) = &param.annotation {
                    v.visit_expr(ann);
                }
                if let Some(default) = &param.default {
                    v.visit_expr(default);
                }
            }
            for param in &s.parameters.kwonlyargs {
                if let Some(ann) = &param.annotation {
                    v.visit_expr(ann);
                }
                if let Some(default) = &param.default {
                    v.visit_expr(default);
                }
            }
            for param in &s.parameters.posonlyargs {
                if let Some(ann) = &param.annotation {
                    v.visit_expr(ann);
                }
                if let Some(default) = &param.default {
                    v.visit_expr(default);
                }
            }
            if let Some(vararg) = &s.parameters.vararg {
                if let Some(ann) = &vararg.annotation {
                    v.visit_expr(ann);
                }
            }
            if let Some(kwarg) = &s.parameters.kwarg {
                if let Some(ann) = &kwarg.annotation {
                    v.visit_expr(ann);
                }
            }
            if let Some(returns) = &s.returns {
                v.visit_expr(returns);
            }
            v.visit_body(&s.body);
        }
        Stmt::ClassDef(s) => {
            for decorator in &s.decorator_list {
                v.visit_expr(decorator);
            }
            if let Some(args) = &s.arguments {
                for arg in &args.args {
                    v.visit_expr(arg);
                }
                for kw in &args.keywords {
                    v.visit_expr(&kw.value);
                }
            }
            v.visit_body(&s.body);
        }
        Stmt::Return(s) => {
            if let Some(val) = &s.value {
                v.visit_expr(val);
            }
        }
        Stmt::Assign(s) => {
            for target in &s.targets {
                v.visit_expr(target);
            }
            v.visit_expr(&s.value);
        }
        Stmt::AnnAssign(s) => {
            v.visit_expr(&s.target);
            v.visit_expr(&s.annotation);
            if let Some(val) = &s.value {
                v.visit_expr(val);
            }
        }
        Stmt::AugAssign(s) => {
            v.visit_expr(&s.target);
            v.visit_expr(&s.value);
        }
        Stmt::For(s) => {
            v.visit_expr(&s.target);
            v.visit_expr(&s.iter);
            v.visit_body(&s.body);
            v.visit_body(&s.orelse);
        }
        Stmt::While(s) => {
            v.visit_expr(&s.test);
            v.visit_body(&s.body);
            v.visit_body(&s.orelse);
        }
        Stmt::If(s) => {
            v.visit_expr(&s.test);
            v.visit_body(&s.body);
            for clause in &s.elif_else_clauses {
                if let Some(test) = &clause.test {
                    v.visit_expr(test);
                }
                v.visit_body(&clause.body);
            }
        }
        Stmt::With(s) => {
            for item in &s.items {
                v.visit_expr(&item.context_expr);
                if let Some(vars) = &item.optional_vars {
                    v.visit_expr(vars);
                }
            }
            v.visit_body(&s.body);
        }
        Stmt::Raise(s) => {
            if let Some(exc) = &s.exc {
                v.visit_expr(exc);
            }
            if let Some(cause) = &s.cause {
                v.visit_expr(cause);
            }
        }
        Stmt::Try(s) => {
            v.visit_body(&s.body);
            for handler in &s.handlers {
                if let Some(ty) = &handler.type_ {
                    v.visit_expr(ty);
                }
                v.visit_body(&handler.body);
            }
            v.visit_body(&s.orelse);
            v.visit_body(&s.finalbody);
        }
        Stmt::Assert(s) => {
            v.visit_expr(&s.test);
            if let Some(msg) = &s.msg {
                v.visit_expr(msg);
            }
        }
        Stmt::Import(_) => {}
        Stmt::ImportFrom(_) => {}
        Stmt::Global(_) => {}
        Stmt::Nonlocal(_) => {}
        Stmt::Expr(s) => {
            v.visit_expr(&s.value);
        }
        Stmt::Pass(_) | Stmt::Break(_) | Stmt::Continue(_) => {}
        Stmt::Delete(s) => {
            for target in &s.targets {
                v.visit_expr(target);
            }
        }
    }
}

pub fn walk_expr(v: &mut (impl Visitor + ?Sized), expr: &Expr) {
    match expr {
        Expr::BoolOp(e) => {
            for val in &e.values {
                v.visit_expr(val);
            }
        }
        Expr::Named(e) => {
            v.visit_expr(&e.target);
            v.visit_expr(&e.value);
        }
        Expr::BinOp(e) => {
            v.visit_expr(&e.left);
            v.visit_expr(&e.right);
        }
        Expr::UnaryOp(e) => {
            v.visit_expr(&e.operand);
        }
        Expr::Lambda(e) => {
            v.visit_expr(&e.body);
        }
        Expr::If(e) => {
            v.visit_expr(&e.test);
            v.visit_expr(&e.body);
            v.visit_expr(&e.orelse);
        }
        Expr::Dict(e) => {
            for item in &e.items {
                if let Some(key) = &item.key {
                    v.visit_expr(key);
                }
                v.visit_expr(&item.value);
            }
        }
        Expr::Set(e) => {
            for elt in &e.elts {
                v.visit_expr(elt);
            }
        }
        Expr::ListComp(e) => {
            v.visit_expr(&e.elt);
            for gen in &e.generators {
                v.visit_expr(&gen.target);
                v.visit_expr(&gen.iter);
                for cond in &gen.ifs {
                    v.visit_expr(cond);
                }
            }
        }
        Expr::SetComp(e) => {
            v.visit_expr(&e.elt);
            for gen in &e.generators {
                v.visit_expr(&gen.target);
                v.visit_expr(&gen.iter);
                for cond in &gen.ifs {
                    v.visit_expr(cond);
                }
            }
        }
        Expr::DictComp(e) => {
            v.visit_expr(&e.key);
            v.visit_expr(&e.value);
            for gen in &e.generators {
                v.visit_expr(&gen.target);
                v.visit_expr(&gen.iter);
                for cond in &gen.ifs {
                    v.visit_expr(cond);
                }
            }
        }
        Expr::Generator(e) => {
            v.visit_expr(&e.elt);
            for gen in &e.generators {
                v.visit_expr(&gen.target);
                v.visit_expr(&gen.iter);
                for cond in &gen.ifs {
                    v.visit_expr(cond);
                }
            }
        }
        Expr::Await(e) => {
            v.visit_expr(&e.value);
        }
        Expr::Yield(e) => {
            if let Some(val) = &e.value {
                v.visit_expr(val);
            }
        }
        Expr::YieldFrom(e) => {
            v.visit_expr(&e.value);
        }
        Expr::Compare(e) => {
            v.visit_expr(&e.left);
            for comp in &e.comparators {
                v.visit_expr(comp);
            }
        }
        Expr::Call(e) => {
            v.visit_expr(&e.func);
            for arg in &e.arguments.args {
                v.visit_expr(arg);
            }
            for kw in &e.arguments.keywords {
                v.visit_expr(&kw.value);
            }
        }
        Expr::FString(e) => {
            for part in &e.parts {
                if let FStringPart::Expression(expr_part) = part {
                    v.visit_expr(&expr_part.value);
                }
            }
        }
        Expr::Attribute(e) => {
            v.visit_expr(&e.value);
        }
        Expr::Subscript(e) => {
            v.visit_expr(&e.value);
            v.visit_expr(&e.slice);
        }
        Expr::Starred(e) => {
            v.visit_expr(&e.value);
        }
        Expr::List(e) => {
            for elt in &e.elts {
                v.visit_expr(elt);
            }
        }
        Expr::Tuple(e) => {
            for elt in &e.elts {
                v.visit_expr(elt);
            }
        }
        Expr::StringLiteral(_)
        | Expr::BytesLiteral(_)
        | Expr::NumberLiteral(_)
        | Expr::BooleanLiteral(_)
        | Expr::NoneLiteral(_)
        | Expr::EllipsisLiteral(_)
        | Expr::Name(_) => {}
    }
}
