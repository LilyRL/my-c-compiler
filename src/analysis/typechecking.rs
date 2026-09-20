use std::{collections::HashMap, fmt::Display};

use strum::{EnumDiscriminants, EnumIs, EnumTryAs};

use crate::{
    analysis::declarations::Scope,
    diagnostics::Diagnostics,
    parser::{
        BlockItem, Constant, ConstantType, Declaration, ExprKind, Expression, ForInit,
        FunctionDeclaration, Identifier, Program, Statement, StmtKind, Switch, UnaryOperator,
        VariableDeclaration,
    },
};

#[derive(Clone, PartialEq, EnumIs, Debug, EnumTryAs)]
pub enum Type {
    Int,
    Long,
    Function(Box<FunctionType>),
}

impl Type {
    pub fn alignment(&self) -> u32 {
        match self {
            Type::Int => 4,
            Type::Long => 8,
            Type::Function(_) => 8,
        }
    }

    pub fn to_constant(&self) -> Option<ConstantType> {
        match self {
            Type::Int => Some(ConstantType::Int),
            Type::Long => Some(ConstantType::Long),
            _ => None,
        }
    }

    pub fn to_asm_type(&self) -> Option<crate::codegen::AssemblyType> {
        match self {
            Type::Int => Some(crate::codegen::AssemblyType::Longword),
            Type::Long => Some(crate::codegen::AssemblyType::Quadword),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, EnumIs)]
pub enum IdentifierAttributes {
    Function { defined: bool, global: bool },
    Static { init: InitialValue, global: bool },
    Local,
}

impl IdentifierAttributes {
    pub fn global(&self) -> bool {
        match self {
            IdentifierAttributes::Function { global, .. } => *global,
            IdentifierAttributes::Static { global, .. } => *global,
            IdentifierAttributes::Local => false,
        }
    }

    pub fn initial_value(&self) -> Option<InitialValue> {
        match self {
            IdentifierAttributes::Function { .. } => None,
            IdentifierAttributes::Static { init, .. } => Some(init.clone()),
            IdentifierAttributes::Local => None,
        }
    }
}

#[derive(Clone, Debug, EnumIs)]
pub enum InitialValue {
    Tentitive,
    Constant(StaticInit),
    None,
}

#[derive(Clone, Debug, EnumDiscriminants, Copy)]
#[strum_discriminants(name(StaticIntType))]
pub enum StaticInit {
    Int(i32),
    Long(i64),
}

impl StaticInit {
    pub fn size_bytes(&self) -> usize {
        match self {
            StaticInit::Int(_) => 4,
            StaticInit::Long(_) => 8,
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            StaticInit::Int(i) => *i == 0,
            StaticInit::Long(l) => *l == 0,
        }
    }

    pub fn from_constant(constant: &Constant) -> Option<Self> {
        match constant {
            Constant::Int(i) => Some(StaticInit::Int(*i)),
            Constant::Long(l) => Some(StaticInit::Long(*l)),
        }
    }

    pub fn to_constant(&self) -> Constant {
        match self {
            StaticInit::Int(i) => Constant::Int(*i),
            StaticInit::Long(l) => Constant::Long(*l),
        }
    }

    pub fn cast(self, ty: StaticIntType) -> Self {
        match (self, ty) {
            (StaticInit::Int(i), StaticIntType::Int) => StaticInit::Int(i),
            (StaticInit::Int(i), StaticIntType::Long) => StaticInit::Long(i as i64),
            (StaticInit::Long(l), StaticIntType::Int) => StaticInit::Int(l as i32),
            (StaticInit::Long(l), StaticIntType::Long) => StaticInit::Long(l),
        }
    }
}

#[derive(Debug)]
pub struct Symbol {
    pub attributes: IdentifierAttributes,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionType {
    pub parameters: Vec<Type>,
    pub defined: bool,
    pub return_type: Type,
}

impl FunctionType {
    pub fn num_parameters(&self) -> usize {
        self.parameters.len()
    }
}

pub type Symbols = HashMap<Identifier, Symbol>;
sge_global::global!(Symbols, symbols);

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Long => write!(f, "long"),
            Type::Function(func) => {
                write!(f, "int(")?;

                let mut n = func.num_parameters();

                if n != 0 {
                    write!(f, "int")?;
                    n -= 1;

                    for _ in 0..n {
                        write!(f, ", int")?;
                    }
                }

                write!(f, ")")
            }
        }
    }
}

pub fn check_all_types(program: &mut Program, diagnostics: &mut Diagnostics) {
    let mut symbols = Symbols::new();

    for declaration in &mut program.0 {
        match declaration {
            Declaration::Var(decl) => {
                check_variable_declaration(decl, &mut symbols, diagnostics, Scope::Global)
            }
            Declaration::Func(decl) => check_function_declaration(decl, &mut symbols, diagnostics),
        }
    }

    set_symbols(symbols);
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
                    .analysis_error(decl.span.clone(), "redeclaration with a different type");
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
                },
            );
        }
    } else if decl.storage_class.is_static() {
        let initial_value;
        if let Some(Some(constant)) = decl.init.as_ref().map(|i| i.eval())
            && let Some(static_int) = StaticInit::from_constant(&constant)
        {
            let const_ty = match decl.ty {
                Type::Int => StaticIntType::Int,
                Type::Long => StaticIntType::Long,
                Type::Function(_) => unreachable!("local var can't have function type"),
            };
            initial_value = InitialValue::Constant(static_int.cast(const_ty));
        } else if decl.init.is_none() {
            initial_value = InitialValue::Tentitive;
        } else {
            diagnostics.analysis_error(decl.span.clone(), "initializer is not a constant integer");
            return;
        }

        symbols.insert(
            decl.name.clone(),
            Symbol {
                attributes: IdentifierAttributes::Static {
                    init: initial_value,
                    global: false,
                },
                ty: decl.ty.clone(),
            },
        );
    } else {
        symbols.insert(
            decl.name.clone(),
            Symbol {
                attributes: IdentifierAttributes::Local,
                ty: decl.ty.clone(),
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
    let mut initial_value;
    if let Some(init) = &decl.init {
        if let Some(constant) = init.eval()
            && let Some(static_int) = StaticInit::from_constant(&constant)
        {
            let const_ty = match decl.ty {
                Type::Int => StaticIntType::Int,
                Type::Long => StaticIntType::Long,
                Type::Function(_) => unreachable!("global var can't have function type"),
            };
            initial_value = InitialValue::Constant(static_int.cast(const_ty));
        } else {
            diagnostics.analysis_error(decl.span.clone(), "initializer is not a constant int");
            return;
        }
    } else if decl.storage_class.is_extern() {
        initial_value = InitialValue::None;
    } else {
        initial_value = InitialValue::Tentitive;
    }

    let mut is_global = !decl.storage_class.is_static();

    if let Some(old_decl) = symbols.get(&decl.name) {
        if old_decl.ty.is_function() {
            diagnostics.analysis_error(
                decl.span.clone(),
                format!(
                    "'{}' redeclared as a variable but was previously a function",
                    decl.name.1
                ),
            );
        } else if old_decl.ty != decl.ty {
            diagnostics.analysis_error(
                decl.span.clone(),
                format!(
                    "conflicting types for '{}': '{}' vs '{}'",
                    decl.name.1, old_decl.ty, decl.ty
                ),
            );
        }

        if decl.storage_class.is_extern() {
            is_global = old_decl.attributes.global();
        } else if old_decl.attributes.global() != is_global {
            diagnostics.analysis_error(
                decl.span.clone(),
                format!("conflicting linkage for variable '{}'", decl.name.1),
            );
        }

        if let Some(old_init) = old_decl.attributes.initial_value() {
            if old_init.is_constant() {
                if initial_value.is_constant() {
                    diagnostics.analysis_error(
                        decl.span.clone(),
                        format!("conflicting file-scope definitions for '{}'", decl.name.1),
                    );
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

    symbols.insert(
        decl.name.clone(),
        Symbol {
            attributes,
            ty: decl.ty.clone(),
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

                if old.parameters != new.parameters {
                    diagnostics.analysis_error(
                        decl.span.clone(),
                        format!(
                            "incompatible function declarations: '{}' vs '{}'",
                            Type::Function(old.clone()),
                            Type::Function(Box::new(new.clone()))
                        ),
                    );
                }

                if old.defined && has_body {
                    diagnostics.analysis_error(
                        decl.span.clone(),
                        format!("redefinition of function '{}'", decl.name.1),
                    );
                }

                if old_decl.attributes.global() && decl.storage_class.is_static() {
                    diagnostics.analysis_error(
                        decl.span.clone(),
                        format!(
                            "static declaration of '{}' follows non-static declaration",
                            decl.name.1
                        ),
                    );
                }

                is_global = old_decl.attributes.global();
            }
            _ => {
                diagnostics.analysis_error(
                    decl.span.clone(),
                    format!(
                        "'{}' redeclared as a function but was previously declared as a variable",
                        decl.name.1
                    ),
                );
            }
        }
    }

    let attributes = IdentifierAttributes::Function {
        defined: new.defined,
        global: is_global,
    };

    symbols.insert(
        decl.name.clone(),
        Symbol {
            attributes,
            ty: Type::Function(Box::new(new)),
        },
    );

    if let Some(body) = &mut decl.body {
        for param in &decl.params {
            symbols.insert(
                param.name.clone(),
                Symbol {
                    attributes: IdentifierAttributes::Local,
                    ty: param.ty.clone(),
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
        ExprKind::Constant(c) => match c {
            Constant::Int(_) => expr.ty = Type::Int,
            Constant::Long(_) => expr.ty = Type::Long,
        },
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
    if a == b { a } else { Type::Long }
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
