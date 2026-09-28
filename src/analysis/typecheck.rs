use crate::{
    analysis::declarations::Scope,
    core::{
        Constant, FunctionType, IdentifierAttributes, InitialValue, StaticInit, Symbol, Symbols,
        Type,
    },
    diagnostics::Diagnostics,
    syntax::{
        BlockItem, Declaration, ExprKind, Expression, ForInit, FunctionDeclaration, Program,
        Statement, StmtKind, Switch, UnaryOperator, VariableDeclaration,
    },
};

pub fn check_all_types(program: &mut Program, diagnostics: &mut Diagnostics) -> Symbols {
    let mut symbols = Symbols::new();

    for declaration in &mut program.0 {
        match declaration {
            Declaration::Var(decl) => {
                check_variable_declaration(decl, &mut symbols, diagnostics, Scope::Global)
            }
            Declaration::Func(decl) => check_function_declaration(decl, &mut symbols, diagnostics),
        }
    }

    symbols
}

fn check_variable_declaration(
    decl: &mut VariableDeclaration,
    symbols: &mut Symbols,
    diagnostics: &mut Diagnostics,
    scope: Scope,
) {
    match scope {
        Scope::Global => check_global_variable_declaration(decl, symbols, diagnostics),
        Scope::Local => check_local_variable(decl, symbols, diagnostics),
    }
}

fn check_local_variable(
    decl: &mut VariableDeclaration,
    symbols: &mut Symbols,
    diagnostics: &mut Diagnostics,
) {
    if decl.storage_class.is_extern() {
        if decl.init.is_some() {
            diagnostics.analysis_error(
                decl.span.clone(),
                "initializer on local extern variable declaration",
            );
        }

        if let Some(old_decl) = symbols.get(&decl.name) {
            if old_decl.ty != decl.ty {
                diagnostics
                    .analysis_error(
                        decl.span.clone(),
                        format!(
                            "redeclaration of '{}' with a different type: '{}' here, but '{}' previously",
                            decl.name.1, decl.ty, old_decl.ty
                        ),
                    )
                    .and_label(old_decl.declared_at.clone(), "previous declaration here");
            }
        } else {
            symbols.insert(
                decl.name.clone(),
                Symbol {
                    attributes: IdentifierAttributes::Static {
                        init: InitialValue::None,
                        global: true,
                    },
                    ty: decl.ty.clone(),
                    declared_at: decl.span.clone(),
                },
            );
        }
    } else if decl.storage_class.is_static() {
        let initial_value = match resolve_init(decl, diagnostics, Scope::Local) {
            Some(value) => value,
            None => return,
        };

        symbols.insert(
            decl.name.clone(),
            Symbol {
                attributes: IdentifierAttributes::Static {
                    init: initial_value,
                    global: false,
                },
                ty: decl.ty.clone(),
                declared_at: decl.span.clone(),
            },
        );
    } else {
        symbols.insert(
            decl.name.clone(),
            Symbol {
                attributes: IdentifierAttributes::Local,
                ty: decl.ty.clone(),
                declared_at: decl.span.clone(),
            },
        );

        if let Some(init) = &mut decl.init {
            check_expression(init, symbols, diagnostics);
            convert_to(init, decl.ty.clone());
        }
    }
}

fn check_global_variable_declaration(
    decl: &VariableDeclaration,
    symbols: &mut Symbols,
    diagnostics: &mut Diagnostics,
) {
    let mut initial_value = match resolve_init(decl, diagnostics, Scope::Global) {
        Some(value) => value,
        None => return,
    };

    let mut is_global = !decl.storage_class.is_static();

    if let Some(old_decl) = symbols.get(&decl.name) {
        if old_decl.ty.is_function() {
            diagnostics
                .analysis_error(
                    decl.span.clone(),
                    format!(
                        "'{}' redeclared as a variable but was previously a function",
                        decl.name.1
                    ),
                )
                .and_label(old_decl.declared_at.clone(), "previous declaration here");
        } else if old_decl.ty != decl.ty {
            diagnostics
                .analysis_error(
                    decl.span.clone(),
                    format!(
                        "conflicting types for '{}': '{}' here, but '{}' previously",
                        decl.name.1, decl.ty, old_decl.ty
                    ),
                )
                .and_label(old_decl.declared_at.clone(), "previous declaration here");
        }

        if decl.storage_class.is_extern() {
            is_global = old_decl.attributes.global();
        } else if old_decl.attributes.global() != is_global {
            diagnostics
                .analysis_error(
                    decl.span.clone(),
                    format!("conflicting linkage for variable '{}'", decl.name.1),
                )
                .and_label(old_decl.declared_at.clone(), "previous declaration here");
        }

        if let Some(old_init) = old_decl.attributes.initial_value() {
            if old_init.is_constant() {
                if initial_value.is_constant() {
                    diagnostics
                        .analysis_error(
                            decl.span.clone(),
                            format!("conflicting file-scope definitions for '{}'", decl.name.1),
                        )
                        .and_label(old_decl.declared_at.clone(), "previous definition here");
                } else {
                    initial_value = old_init;
                }
            } else if !initial_value.is_constant() && old_init.is_tentitive() {
                initial_value = InitialValue::Tentitive;
            }
        }
    }

    let attributes = IdentifierAttributes::Static {
        init: initial_value,
        global: is_global,
    };

    let declared_at = symbols
        .get(&decl.name)
        .map_or_else(|| decl.span.clone(), |old| old.declared_at.clone());

    symbols.insert(
        decl.name.clone(),
        Symbol {
            attributes,
            ty: decl.ty.clone(),
            declared_at,
        },
    );
}

fn check_function_declaration(
    decl: &mut FunctionDeclaration,
    symbols: &mut Symbols,
    diagnostics: &mut Diagnostics,
) {
    let has_body = decl.body.is_some();
    let mut new = FunctionType {
        parameters: decl.params.iter().map(|p| p.ty.clone()).collect(),
        return_type: decl.return_type.clone(),
        defined: has_body,
    };
    let mut is_global = !decl.storage_class.is_static();

    if let Some(old_decl) = symbols.get(&decl.name) {
        match &old_decl.ty {
            Type::Function(old) => {
                new.defined = old.defined || has_body;

                if old.parameters != new.parameters || old.return_type != new.return_type {
                    let which = if old.parameters != new.parameters {
                        String::from("different parameters")
                    } else {
                        format!(
                            "return type '{}' rather than '{}'",
                            decl.return_type, old.return_type
                        )
                    };
                    diagnostics
                        .analysis_error(
                            decl.span.clone(),
                            format!(
                                "conflicting types for '{}': declaration has {}",
                                decl.name.1, which
                            ),
                        )
                        .and_label(old_decl.declared_at.clone(), "previous declaration here");
                }

                if old.defined && has_body {
                    diagnostics
                        .analysis_error(
                            decl.span.clone(),
                            format!("redefinition of function '{}'", decl.name.1),
                        )
                        .and_label(old_decl.declared_at.clone(), "previous definition here");
                }

                if old_decl.attributes.global() && decl.storage_class.is_static() {
                    diagnostics
                        .analysis_error(
                            decl.span.clone(),
                            format!(
                                "static declaration of '{}' follows non-static declaration",
                                decl.name.1
                            ),
                        )
                        .and_label(old_decl.declared_at.clone(), "previous declaration here");
                }

                is_global = old_decl.attributes.global();
            }
            _ => {
                diagnostics
                    .analysis_error(
                        decl.span.clone(),
                        format!(
                            "'{}' redeclared as a function but was previously declared as a variable",
                            decl.name.1
                        ),
                    )
                    .and_label(old_decl.declared_at.clone(), "previous declaration here");
            }
        }
    }

    let attributes = IdentifierAttributes::Function {
        defined: new.defined,
        global: is_global,
    };

    let declared_at = symbols
        .get(&decl.name)
        .map_or_else(|| decl.span.clone(), |old| old.declared_at.clone());

    symbols.insert(
        decl.name.clone(),
        Symbol {
            attributes,
            ty: Type::Function(Box::new(new)),
            declared_at,
        },
    );

    if let Some(body) = &mut decl.body {
        for param in &decl.params {
            symbols.insert(
                param.name.clone(),
                Symbol {
                    attributes: IdentifierAttributes::Local,
                    ty: param.ty.clone(),
                    declared_at: param.span.clone(),
                },
            );
        }

        check_block(body, symbols, diagnostics, &decl.return_type);
    }
}

fn check_expression(
    expr: &mut Expression,
    symbols: &mut Symbols,
    diagnostics: &mut Diagnostics,
) -> Option<()> {
    match &mut expr.kind {
        ExprKind::Var(i) => {
            let v_ty = symbols.get(i).unwrap().ty.clone();

            if v_ty.is_function() {
                diagnostics.analysis_error(expr.span.clone(), "function name used as variable");
                return None;
            }

            expr.ty = v_ty.clone();
        }
        ExprKind::Constant(c) => {
            expr.ty = c.ty();
        }
        ExprKind::Cast {
            target_type,
            expr: inner,
        } => {
            check_expression(inner, symbols, diagnostics)?;
            expr.ty = target_type.clone();
        }
        ExprKind::Unary {
            operator,
            expr: inner,
        } => {
            check_expression(inner, symbols, diagnostics)?;

            expr.ty = match operator {
                UnaryOperator::Not => Type::Int,
                _ => inner.ty.clone(),
            };
        }
        ExprKind::Binary { operator, lhs, rhs } => {
            check_expression(lhs, symbols, diagnostics)?;
            check_expression(rhs, symbols, diagnostics)?;

            if operator.is_logical() {
                expr.ty = Type::Int;
                return Some(());
            }

            if operator.is_shift() {
                expr.ty = lhs.ty.clone();
                return Some(());
            }

            let common_type = get_common_type(lhs.ty.clone(), rhs.ty.clone());
            convert_to(lhs, common_type.clone());
            convert_to(rhs, common_type.clone());

            if operator.is_comparison() {
                expr.ty = Type::Int;
            } else {
                expr.ty = common_type;
            }
        }
        ExprKind::Prefix(_, inner) | ExprKind::Postfix(_, inner) => {
            check_expression(inner, symbols, diagnostics)?;
            expr.ty = inner.ty.clone();
        }
        ExprKind::Conditional(cond, then, else_) => {
            check_expression(cond, symbols, diagnostics)?;
            check_expression(then, symbols, diagnostics)?;
            check_expression(else_, symbols, diagnostics)?;

            if then.ty != else_.ty {
                let common_type = get_common_type(then.ty.clone(), else_.ty.clone());
                convert_to(then, common_type.clone());
                convert_to(else_, common_type.clone());
                expr.ty = common_type;
            } else {
                expr.ty = then.ty.clone();
            }
        }
        ExprKind::Assignment(lhs, rhs) => {
            check_expression(lhs, symbols, diagnostics)?;
            check_expression(rhs, symbols, diagnostics)?;
            convert_to(rhs, lhs.ty.clone());
            expr.ty = lhs.ty.clone();
        }
        ExprKind::CompoundAssign { .. } => unreachable!(),
        ExprKind::FunctionCall { name, args } => {
            let function_type = symbols.get(name).unwrap().ty.clone();

            match function_type {
                Type::Function(f) => {
                    let FunctionType {
                        parameters,
                        return_type,
                        ..
                    } = *f;
                    if parameters.len() != args.len() {
                        diagnostics.analysis_error(
                            expr.span.clone(),
                            format!(
                                "wrong number of arguments: expected {}, found {}",
                                parameters.len(),
                                args.len()
                            ),
                        );
                        return None;
                    }

                    for (arg, param_type) in args.iter_mut().zip(parameters) {
                        check_expression(arg, symbols, diagnostics)?;
                        convert_to(arg, param_type);
                    }

                    expr.ty = return_type;
                }
                _ => {
                    diagnostics.analysis_error(
                        expr.span.clone(),
                        format!("variable '{}' used as a function", name.1),
                    );
                    return None;
                }
            }
        }
    }

    Some(())
}

// this function is so confusing
fn convert_to(expr: &mut Expression, ty: Type) {
    if expr.ty != ty {
        let span = expr.span.clone();

        let placeholder = Expression {
            kind: ExprKind::Constant(Constant::Int(0)),
            span,
            ty: ty.clone(),
        };

        let old = std::mem::replace(expr, placeholder);

        expr.kind = ExprKind::Cast {
            target_type: ty,
            expr: Box::new(old),
        };
    }
}

fn get_common_type(a: Type, b: Type) -> Type {
    if a == b {
        a
    } else if a.size_bytes() == b.size_bytes() {
        if a.is_signed() { b } else { a }
    } else {
        if a.size_bytes() > b.size_bytes() {
            a
        } else {
            b
        }
    }
}

fn check_block(
    block: &mut Vec<BlockItem>,
    symbols: &mut Symbols,
    diagnostics: &mut Diagnostics,
    function_return_type: &Type,
) {
    for item in block {
        match item {
            BlockItem::Stmt(s) => check_statement(s, symbols, diagnostics, function_return_type),
            BlockItem::Decl(decl) => match decl {
                Declaration::Var(v) => {
                    check_variable_declaration(v, symbols, diagnostics, Scope::Local)
                }
                Declaration::Func(f) => check_function_declaration(f, symbols, diagnostics),
            },
        }
    }
}

fn check_statement(
    stmt: &mut Statement,
    symbols: &mut Symbols,
    diagnostics: &mut Diagnostics,
    function_return_type: &Type,
) {
    match &mut stmt.kind {
        StmtKind::Compound(items) => {
            for item in items {
                match item {
                    BlockItem::Stmt(s) => {
                        check_statement(s, symbols, diagnostics, function_return_type)
                    }
                    BlockItem::Decl(d) => match d {
                        Declaration::Var(v) => {
                            check_variable_declaration(v, symbols, diagnostics, Scope::Local)
                        }
                        Declaration::Func(f) => check_function_declaration(f, symbols, diagnostics),
                    },
                }
            }
        }
        StmtKind::Switch(Switch { value, body, .. }) => {
            check_expression(value, symbols, diagnostics);
            check_statement(body, symbols, diagnostics, function_return_type);
        }
        StmtKind::If { cond, then, else_ } => {
            check_expression(cond, symbols, diagnostics);
            check_statement(then, symbols, diagnostics, function_return_type);
            if let Some(else_) = else_ {
                check_statement(else_, symbols, diagnostics, function_return_type);
            }
        }
        StmtKind::While { cond, body, .. } | StmtKind::DoWhile { body, cond, .. } => {
            check_expression(cond, symbols, diagnostics);
            check_statement(body, symbols, diagnostics, function_return_type);
        }
        StmtKind::For {
            init,
            condition,
            post,
            body,
            ..
        } => {
            match init {
                ForInit::Decl(d) => {
                    check_variable_declaration(d, symbols, diagnostics, Scope::Local)
                }
                ForInit::Expr(e) => {
                    check_expression(e, symbols, diagnostics);
                }
                ForInit::None => {}
            }
            if let Some(condition) = condition {
                check_expression(condition, symbols, diagnostics);
            }
            if let Some(post) = post {
                check_expression(post, symbols, diagnostics);
            }
            check_statement(body, symbols, diagnostics, function_return_type);
        }
        StmtKind::Expression(e) => {
            check_expression(e, symbols, diagnostics);
        }
        StmtKind::Return(e) => {
            check_expression(e, symbols, diagnostics);
            convert_to(e, function_return_type.clone());
        }
        StmtKind::Label(_, stmt)
        | StmtKind::Case { stmt, .. }
        | StmtKind::DefaultCase { stmt, .. } => {
            check_statement(stmt, symbols, diagnostics, function_return_type)
        }
        StmtKind::Null | StmtKind::Break(_) | StmtKind::Continue(_) | StmtKind::Goto(_) => {}
    }
}

fn resolve_init(
    decl: &VariableDeclaration,
    diagnostics: &mut Diagnostics,
    scope: Scope,
) -> Option<InitialValue> {
    if let Some(expr) = &decl.init {
        match expr.eval() {
            Ok(constant) => {
                let const_ty = decl.ty.to_constant().unwrap_or_else(|| {
                    unreachable!(
                        "{} var can't have function type",
                        if scope.is_local() { "local" } else { "global" }
                    )
                });
                Some(InitialValue::Constant(
                    StaticInit::from_constant(constant).cast(const_ty),
                ))
            }
            Err(e) => {
                diagnostics.analysis_error(decl.span.clone(), e);
                None
            }
        }
    } else if scope.is_global() && decl.storage_class.is_extern() {
        Some(InitialValue::None)
    } else {
        Some(InitialValue::Tentitive)
    }
}
