#![allow(dead_code, unused_variables)]

use crate::ast::{self, DeclarationSpecifier};
use std::fmt::Debug;

#[derive(Debug, PartialEq, Clone)]
enum Type<'text> {
    Void,
    Int,
    Char,
    Float,
    Double,
    String,
    Pointer(Box<Type<'text>>),
    Array(Box<Type<'text>>, usize),
    Function {
        return_ty: Box<Type<'text>>,
        param_tys: Vec<Type<'text>>,
    },
    Struct {
        name: &'text str,
        members: Vec<(&'text str, Type<'text>)>,
    },
    TypeDef {
        name: &'text str,
        ty: Box<Type<'text>>,
    },
    SignedChar,
    UnSignedChar,
    Short,
    UnSignedShort,
    UnSigned,
    Long,
    UnSignedLong,
    LongLong,
    UnSignedLongLong,
    LongDouble,
}

#[derive(Debug)]
enum BinOp<'ast, 'text> {
    LogicalOr(&'ast ast::LogicalOrExpr<'text>),
    LogicalAnd(&'ast ast::LogicalAndExpr<'text>),
    BitOr(&'ast ast::BitOrExpr<'text>),
    XOR(&'ast ast::XORExpr<'text>),
    BitAnd(&'ast ast::BitAndExpr<'text>),
    Equality(&'ast ast::EqualityExpr<'text>),
    Comparision(&'ast ast::ComparisionExpr<'text>),
    Shift(&'ast ast::ShiftExpr<'text>),
    Additive(&'ast ast::AdditiveExpr<'text>),
    Multiplicative(&'ast ast::MultiplicativeExpr<'text>),
}

#[derive(Debug)]
enum SemanticError<'ast, 'text> {
    UndefinedVariable(&'text str),
    UndefinedLabel(&'text str),
    VariableRedeclaration(&'text str),
    InvalidBinaryOperands(BinOp<'ast, 'text>),
    TypeMismatch(Type<'text>, Type<'text>),
    UnexpectedType {
        expected: Type<'text>,
        actual: Type<'text>,
    },
    InvalidInitializer,
    InvalidPostfixOperand(&'ast ast::PostfixExpr<'text>),
    NotAFunction(&'ast ast::PostfixExpr<'text>),
    InvalidFnCall(&'ast ast::PostfixExpr<'text>),
    UndefinedMember {
        struct_name: &'text str,
        field: &'text str,
    },
    NotAStruct(&'ast ast::PostfixExpr<'text>),
    NotAPointerToStruct(&'ast ast::PostfixExpr<'text>),
    InvalidUnaryOperand(&'ast ast::UnaryExpr<'text>),
    InvalidDereferenceOperand(&'ast ast::UnaryExpr<'text>),
    InvalidTypeCast {
        from: Type<'text>,
        to: Type<'text>,
    },
    IllegalJump(&'ast ast::JumpStmt<'text>),
    ReturnTypeMismatch {
        expected: Type<'text>,
        actual: Type<'text>,
    },
    ReturnOutsideFn(&'ast ast::JumpStmt<'text>),
    CaseOutsideSwitch(&'ast ast::LabeledStmt<'text>),
    DefaultOutsideSwitch(&'ast ast::LabeledStmt<'text>),
    LabelRedeclaration(&'ast ast::LabeledStmt<'text>),
    InvalidSpecifierQualifiers(&'ast [ast::SpecifierQualifier<'text>]),
    InvalidFunctionDefinition(&'ast ast::FunctionDefinition<'text>),
    InvalidDSS(&'ast [ast::DeclarationSpecifier<'text>]),
}

enum Symbol<'text> {
    Var(Var<'text>),
    Label(Label<'text>),
    Enum(Enum<'text>),
}

struct Var<'text> {
    name: &'text str,
    ty: Type<'text>,
}

struct Label<'text>(&'text str);

struct Enum<'text> {
    name: &'text str,
    ty: Type<'text>,
}

struct Scope<'text> {
    symbols: Vec<Symbol<'text>>,
    kind: ScopeKind<'text>,
}

#[derive(PartialEq, Clone)]
enum ScopeKind<'text> {
    Regular,
    Fn(Type<'text>),
    Switch(Type<'text>),
    Loop,
}

struct SemanticContext<'text> {
    symbol_table: Vec<Scope<'text>>,
}

impl<'text> SemanticContext<'text> {
    fn new() -> SemanticContext<'text> {
        SemanticContext {
            symbol_table: vec![Scope {
                symbols: vec![],
                kind: ScopeKind::Regular,
            }],
        }
    }

    fn scoped<T>(&mut self, kind: ScopeKind<'text>, f: impl FnOnce(&mut Self) -> T) -> T {
        self.symbol_table.push(Scope {
            symbols: vec![],
            kind,
        });
        let out = f(self);
        self.symbol_table.pop();
        out
    }

    fn curr_scope(&self) -> &Scope<'text> {
        self.symbol_table
            .last()
            .expect("must have atleast one scope")
    }

    fn curr_scope_mut(&mut self) -> &mut Scope<'text> {
        self.symbol_table
            .last_mut()
            .expect("must have atleast one scope")
    }

    fn curr_fn_scope<'ctx>(&'ctx self) -> Option<(&'ctx Type<'text>, &'ctx [Symbol<'text>])> {
        self.symbol_table
            .iter()
            .rev()
            .find_map(|scope| match &scope.kind {
                ScopeKind::Fn(return_ty) => Some((return_ty, scope.symbols.as_slice())),
                _ => None,
            })
    }

    fn curr_switch_scope<'ctx>(&'ctx self) -> Option<(&'ctx Type<'text>, &'ctx [Symbol<'text>])> {
        self.symbol_table
            .iter()
            .rev()
            .find_map(|scope| match &scope.kind {
                ScopeKind::Switch(ty) => Some((ty, scope.symbols.as_slice())),
                _ => None,
            })
    }

    fn in_loop<'ctx>(&'ctx self) -> bool {
        self.symbol_table
            .iter()
            .rev()
            .any(|scope| scope.kind == ScopeKind::Loop)
    }

    fn in_switch<'ctx>(&'ctx self) -> bool {
        self.symbol_table
            .iter()
            .rev()
            .any(|scope| match scope.kind {
                ScopeKind::Switch(_) => true,
                _ => false,
            })
    }

    // fn curr_switch_scope<'ctx>

    fn declare_var(&mut self, var: Var<'text>) -> bool {
        let scope = self.curr_scope_mut();

        // cannot redeclare variable
        if scope
            .symbols
            .iter()
            .filter_map(|s| match s {
                Symbol::Var(v) => Some(v),
                _ => None,
            })
            .any(|var_| var_.name == var.name)
        {
            return false;
        }

        scope.symbols.push(Symbol::Var(var));
        true
    }

    fn find_var<'ctx>(&'ctx self, name: &'text str) -> Option<&'ctx Var<'text>> {
        self.symbol_table
            .iter()
            .rev()
            .flat_map(|scope| scope.symbols.iter().rev())
            .filter_map(|s| match s {
                Symbol::Var(v) => Some(v),
                _ => None,
            })
            .find(|var| var.name == name)
    }

    fn declare_label(&mut self, label: &'text str) -> bool {
        // labels are function scoped.
        // so checking for label just inside local scope is not enough
        if self
            .symbol_table
            .iter()
            .rev()
            .flat_map(|scope| scope.symbols.iter().rev())
            .filter_map(|s| match s {
                Symbol::Label(l) => Some(l),
                _ => None,
            })
            .any(|l| l.0 == label)
        {
            return false;
        }

        self.curr_scope_mut()
            .symbols
            .push(Symbol::Label(Label(label)));
        true
    }

    fn find_label<'ctx>(&'ctx self, label: &'text str) -> Option<&'ctx Label<'text>> {
        self.symbol_table
            .iter()
            .flat_map(|scope| scope.symbols.iter())
            .filter_map(|s| match s {
                Symbol::Label(label) => Some(label),
                _ => None,
            })
            .find(|Label(label_)| *label_ == label)
    }

    fn contains_label(&self, label: &'text str) -> bool {
        self.find_label(label).is_some()
    }

    fn declare_enum_invariant(&mut self, e: Enum<'text>) -> bool {
        let scope = self.curr_scope_mut();

        // cannot redeclare enum invariants
        if scope
            .symbols
            .iter()
            .filter_map(|s| match s {
                Symbol::Enum(e) => Some(e),
                _ => None,
            })
            .any(|e_| e_.name == e.name)
        {
            return false;
        }

        scope.symbols.push(Symbol::Enum(e));
        true
    }

    fn find_enum_invariant<'ctx>(&'ctx self, name: &'text str) -> Option<&'ctx Enum<'text>> {
        self.symbol_table
            .iter()
            .rev()
            .flat_map(
                /* C doesn't allow redeclaring enums.
                So, scope.iter() and scope.iter().rev() work the same.
                */
                |scope| scope.symbols.iter(),
            )
            .filter_map(|s| match s {
                Symbol::Enum(e) => Some(e),
                _ => None,
            })
            .find(|e| e.name == name)
    }

    // fn find_typedef<'ctx>(&'ctx self, name: &'text str) -> Option<>
}

fn analyze_translation_unit<'ast, 'text>(
    translation_unit: &'ast ast::TranslationUnit<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<(), SemanticError<'ast, 'text>> {
    for external_declaration in &translation_unit.0 {
        analyze_external_declaration(external_declaration, ctx)?;
    }
    Ok(())
}

fn analyze_external_declaration<'ast, 'text>(
    external_declaration: &'ast ast::ExternalDeclaration<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<(), SemanticError<'ast, 'text>> {
    match external_declaration {
        ast::ExternalDeclaration::FunctionDefinition(f) => analyze_function_definition(f, ctx)?,
        ast::ExternalDeclaration::Declaration(d) => analyze_declaration(d, ctx)?,
    }
    Ok(())
}

fn analyze_function_definition<'ast, 'text>(
    f: &'ast ast::FunctionDefinition<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<(), SemanticError<'ast, 'text>> {
    use ast::StorageClassSpecifier as SCS;
    use ast::TypeSpecifier as TS;

    let mut pos = 0;

    let scs = match f.declaration_specifiers.get(pos) {
        None => None,
        Some(DeclarationSpecifier::StorageClassSpecifier(scs))
            if scs == &SCS::Static || scs == &SCS::Extern =>
        {
            pos += 1;
            Some(scs)
        }
        _ => return Err(SemanticError::InvalidDSS(&f.declaration_specifiers)),
    };

    let mut tss = Vec::with_capacity(f.declaration_specifiers.len());
    for ds in &f.declaration_specifiers[pos..] {
        match ds {
            DeclarationSpecifier::TypeSpecifier(ts) => tss.push(ts),
            _ => return Err(SemanticError::InvalidDSS(&f.declaration_specifiers)),
        }
    }

    let a = match tss.as_slice() {
        [TS::Void] => Ok(Type::Void),
        [TS::Char] => Ok(Type::Char),
        [TS::Signed, TS::Char] => Ok(Type::SignedChar),
        [TS::UnSigned, TS::Char] => Ok(Type::UnSignedChar),
        [TS::Short]
        | [TS::Signed, TS::Short]
        | [TS::Short, TS::Int]
        | [TS::Signed, TS::Short, TS::Int] => Ok(Type::Short),
        [TS::UnSigned, TS::Short] | [TS::UnSigned, TS::Short, TS::Int] => Ok(Type::UnSignedShort),
        [TS::Int] | [TS::Signed] | [TS::Signed, TS::Int] => Ok(Type::Int),
        [TS::UnSigned] | [TS::UnSigned, TS::Int] => Ok(Type::UnSigned),
        [TS::Long]
        | [TS::Signed, TS::Long]
        | [TS::Long, TS::Int]
        | [TS::Signed, TS::Long, TS::Int] => Ok(Type::Long),
        [TS::UnSigned, TS::Long] | [TS::UnSigned, TS::Long, TS::Int] => Ok(Type::UnSignedLong),
        [TS::Long, TS::Long]
        | [TS::Signed, TS::Long, TS::Long]
        | [TS::Long, TS::Long, TS::Int]
        | [TS::Signed, TS::Long, TS::Long, TS::Int] => Ok(Type::LongLong),
        [TS::UnSigned, TS::Long, TS::Long] | [TS::UnSigned, TS::Long, TS::Long, TS::Int] => {
            Ok(Type::UnSignedLongLong)
        }
        [TS::Float] => Ok(Type::Float),
        [TS::Double] => Ok(Type::Double),
        [TS::Long, TS::Double] => Ok(Type::LongDouble),
        [TS::StructOrUnionSpecifier(sou)] => analyze_struct_or_union_specifier(sou, ctx),
        [TS::EnumSpecifier(e)] => analyze_enum_specifier(e, ctx),
        [TS::TypeDefName(name)] => {
            // ctx.find_typedef(name);
            todo!()
        }
        _ => todo!(),
    };

    todo!()
}

fn analyze_declaration<'ast, 'text>(
    declaration: &'ast ast::Declaration<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<(), SemanticError<'ast, 'text>> {
    let ty = analyze_declaration_specifiers(&declaration.declaration_specifiers, ctx)?;

    for init_d in &declaration.init_declarators {
        let d_ty = analyze_init_declarator(init_d, ctx)?;
        // if ty != d_ty {
        //     return Err(SemanticError::TypeMismatch(ty, d_ty));
        // }
    }

    Ok(())
}

fn analyze_declaration_specifiers<'ast, 'text>(
    dss: &'ast [ast::DeclarationSpecifier<'text>],
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    todo!()
}

fn analyze_init_declarator<'ast, 'text>(
    init_d: &'ast ast::InitDeclarator<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match init_d {
        ast::InitDeclarator::Declared(d) => analyze_declarator(d, ctx),
        ast::InitDeclarator::Initialized(d, init) => {
            let d_ty = analyze_declarator(d, ctx)?;
            let init_ty = analyze_initializer(init, ctx)?;
            match d_ty == init_ty {
                true => Ok(d_ty),
                false => Err(SemanticError::TypeMismatch(d_ty, init_ty)),
            }
        }
    }
}

fn analyze_declarator<'ast, 'text>(
    declarator: &'ast ast::Declarator<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    if let Some(pointer) = &declarator.pointer {
        analyze_pointer(pointer, ctx)?;
    }
    analyze_direct_declarator(&declarator.d_declarator, ctx)
}

fn analyze_initializer<'ast, 'text>(
    init: &'ast ast::Initializer<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match init {
        ast::Initializer::Assignment(expr) => analyze_assignment_expr(expr, ctx),
        ast::Initializer::InitializerList(inits) => todo!(),
    }
}

fn analyze_pointer<'ast, 'text>(
    pointer: &'ast ast::Pointer,
    ctx: &mut SemanticContext<'text>,
) -> Result<(), SemanticError<'ast, 'text>> {
    Ok(())
}

fn analyze_direct_declarator<'ast, 'text>(
    d_declarator: &'ast ast::DirectDeclarator<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    todo!()
}

fn analyze_type_name<'ast, 'text>(
    type_name: &'ast ast::TypeName<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    let base_ty = analyze_specifier_qualifiers(&type_name.specifier_qualifiers, ctx)?;

    match type_name.abstract_declarator.as_ref() {
        Some(ad) => {
            let declarator_ty = analyze_abstract_declarator(ad, ctx)?;

            // let a = match (base_ty, declarator_ty) {
            // }

            todo!()

            // match (base_ty, declarator_ty) {
            // }
        }
        None => Ok(base_ty),
    }
}

fn analyze_specifier_qualifiers<'ast, 'text>(
    sqs: &'ast [ast::SpecifierQualifier<'text>],
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    // TODO: check TypeQualifiers (const, volatile)

    use ast::TypeSpecifier as TS;

    let mut tss = sqs
        .iter()
        .filter_map(|sq| match sq {
            ast::SpecifierQualifier::TypeSpecifier(ts) => Some(ts),
            _ => None,
        })
        .collect::<Vec<&ast::TypeSpecifier<'text>>>();

    tss.sort_by_key(|ts| match ts {
        TS::Void => 2,
        TS::Signed => 3,
        TS::UnSigned => 4,
        TS::Short => 5,
        TS::Long => 6,
        TS::Int => 7,
        TS::Char => 8,
        TS::Float => 9,
        TS::Double => 10,
        TS::StructOrUnionSpecifier(_) => 11,
        TS::EnumSpecifier(_) => 12,
        TS::TypeDefName(_) => 13,
    });

    match tss.as_slice() {
        [TS::Void] => Ok(Type::Void),
        [TS::Char] => Ok(Type::Char),
        [TS::Signed, TS::Char] => Ok(Type::SignedChar),
        [TS::UnSigned, TS::Char] => Ok(Type::UnSignedChar),
        [TS::Short]
        | [TS::Signed, TS::Short]
        | [TS::Short, TS::Int]
        | [TS::Signed, TS::Short, TS::Int] => Ok(Type::Short),
        [TS::UnSigned, TS::Short] | [TS::UnSigned, TS::Short, TS::Int] => Ok(Type::UnSignedShort),
        [TS::Int] | [TS::Signed] | [TS::Signed, TS::Int] => Ok(Type::Int),
        [TS::UnSigned] | [TS::UnSigned, TS::Int] => Ok(Type::UnSigned),
        [TS::Long]
        | [TS::Signed, TS::Long]
        | [TS::Long, TS::Int]
        | [TS::Signed, TS::Long, TS::Int] => Ok(Type::Long),
        [TS::UnSigned, TS::Long] | [TS::UnSigned, TS::Long, TS::Int] => Ok(Type::UnSignedLong),
        [TS::Long, TS::Long]
        | [TS::Signed, TS::Long, TS::Long]
        | [TS::Long, TS::Long, TS::Int]
        | [TS::Signed, TS::Long, TS::Long, TS::Int] => Ok(Type::LongLong),
        [TS::UnSigned, TS::Long, TS::Long] | [TS::UnSigned, TS::Long, TS::Long, TS::Int] => {
            Ok(Type::UnSignedLongLong)
        }
        [TS::Float] => Ok(Type::Float),
        [TS::Double] => Ok(Type::Double),
        [TS::Long, TS::Double] => Ok(Type::LongDouble),
        [TS::StructOrUnionSpecifier(sou)] => analyze_struct_or_union_specifier(sou, ctx),
        [TS::EnumSpecifier(e)] => analyze_enum_specifier(e, ctx),
        [TS::TypeDefName(name)] => {
            // ctx.find_typedef(name);
            todo!()
        }
        _ => Err(SemanticError::InvalidSpecifierQualifiers(sqs)),
    }
}

fn analyze_struct_or_union_specifier<'ast, 'text>(
    sou: &ast::StructOrUnionSpecifier<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    todo!()
}

fn analyze_enum_specifier<'ast, 'text>(
    e: &ast::EnumSpecifier<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    todo!()
}

fn analyze_abstract_declarator<'ast, 'text>(
    ad: &ast::AbstractDeclarator<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match ad {
        ast::AbstractDeclarator::Pointer(p) => todo!(),
        ast::AbstractDeclarator::PointerWithDirect(p, dad) => todo!(),
        ast::AbstractDeclarator::Direct(dad) => todo!(),
    }
}

fn analyze_stmt<'ast, 'text>(
    stmt: &'ast ast::Stmt<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<(), SemanticError<'ast, 'text>> {
    match stmt {
        ast::Stmt::EmptyStmt => Ok(()),
        ast::Stmt::Labeled(stmt) => analyze_labeled_stmt(stmt, ctx),
        ast::Stmt::Expr(expr) => analyze_assignment_expr(expr, ctx).map(|_| ()),
        ast::Stmt::Compound(stmt) => analyze_compound_stmt(stmt, ctx),
        ast::Stmt::Selection(stmt) => analyze_selection_stmt(stmt, ctx),
        ast::Stmt::Iteration(stmt) => analyze_iteration_stmt(stmt, ctx),
        ast::Stmt::Jump(stmt) => analyze_jump_stmt(stmt, ctx),
    }
}

fn analyze_labeled_stmt<'ast, 'text>(
    stmt: &'ast ast::LabeledStmt<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<(), SemanticError<'ast, 'text>> {
    match stmt {
        ast::LabeledStmt::Ident(label, inner_stmt) => {
            if !ctx.declare_label(label) {
                return Err(SemanticError::LabelRedeclaration(stmt));
            }
            analyze_stmt(inner_stmt, ctx)
        }
        ast::LabeledStmt::Case(expr, inner_stmt) => match ctx.curr_switch_scope() {
            Some((switch_ty, _)) => {
                match (switch_ty.clone(), analyze_conditional_expr(expr, ctx)?) {
                    (Type::Int, Type::Int) | (Type::Char, Type::Char) => {
                        analyze_stmt(inner_stmt, ctx)
                    }
                    (switch_ty, expr_ty) => Err(SemanticError::UnexpectedType {
                        expected: switch_ty,
                        actual: expr_ty,
                    }),
                }
            }
            None => Err(SemanticError::CaseOutsideSwitch(stmt)),
        },
        ast::LabeledStmt::Default(inner_stmt) => match ctx.in_switch() {
            true => analyze_stmt(inner_stmt, ctx),
            false => Err(SemanticError::DefaultOutsideSwitch(stmt)),
        },
    }
}

fn analyze_compound_stmt<'ast, 'text>(
    stmt: &'ast ast::CompoundStmt<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<(), SemanticError<'ast, 'text>> {
    ctx.scoped(ScopeKind::Regular, |ctx| {
        stmt.0
            .iter()
            .map(|item| match item {
                ast::BlockItem::Declaration(d) => analyze_declaration(d, ctx),
                ast::BlockItem::Statement(stmt) => analyze_stmt(stmt, ctx),
            })
            .find(Result::is_err)
            .unwrap_or(Ok(()))
    })
}

fn analyze_selection_stmt<'ast, 'text>(
    stmt: &'ast ast::SelectionStmt<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<(), SemanticError<'ast, 'text>> {
    match stmt {
        ast::SelectionStmt::If { test, pass } => {
            let test_ty = analyze_assignment_expr(test, ctx)?;
            if test_ty != Type::Int {
                return Err(SemanticError::UnexpectedType {
                    expected: Type::Int,
                    actual: test_ty,
                });
            }

            ctx.scoped(ScopeKind::Regular, |ctx| analyze_stmt(pass, ctx))
        }
        ast::SelectionStmt::IfElse { test, pass, fail } => {
            let test_ty = analyze_assignment_expr(test, ctx)?;
            if test_ty != Type::Int {
                return Err(SemanticError::UnexpectedType {
                    expected: Type::Int,
                    actual: test_ty,
                });
            }

            ctx.scoped(ScopeKind::Regular, |ctx| analyze_stmt(pass, ctx))?;
            ctx.scoped(ScopeKind::Regular, |ctx| analyze_stmt(fail, ctx))
        }
        ast::SelectionStmt::Switch { test, pass } => {
            let test_ty = analyze_assignment_expr(test, ctx)?;
            if test_ty != Type::Int {
                return Err(SemanticError::UnexpectedType {
                    expected: Type::Int,
                    actual: test_ty,
                });
            }

            ctx.scoped(ScopeKind::Switch(test_ty), |ctx| analyze_stmt(pass, ctx))
        }
    }
}

fn analyze_iteration_stmt<'ast, 'text>(
    stmt: &'ast ast::IterationStmt<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<(), SemanticError<'ast, 'text>> {
    match stmt {
        ast::IterationStmt::While { test, body } => {
            let test_ty = analyze_assignment_expr(test, ctx)?;
            if test_ty != Type::Int {
                return Err(SemanticError::UnexpectedType {
                    expected: Type::Int,
                    actual: test_ty,
                });
            }
            ctx.scoped(ScopeKind::Loop, |ctx| analyze_stmt(body, ctx))
        }
        ast::IterationStmt::DoWhile { test, body } => {
            ctx.scoped(ScopeKind::Loop, |ctx| analyze_stmt(body, ctx))?;

            let test_ty = analyze_assignment_expr(test, ctx)?;
            if test_ty != Type::Int {
                return Err(SemanticError::UnexpectedType {
                    expected: Type::Int,
                    actual: test_ty,
                });
            }

            Ok(())
        }
        ast::IterationStmt::For {
            init,
            test,
            update,
            body,
        } => {
            if let Some(init) = init {
                analyze_assignment_expr(init, ctx)?;
            }

            if let Some(test) = test {
                let test_type = analyze_assignment_expr(test, ctx)?;
                if test_type != Type::Int {
                    return Err(SemanticError::UnexpectedType {
                        expected: Type::Int,
                        actual: test_type,
                    });
                }
            }

            if let Some(update) = update {
                analyze_assignment_expr(update, ctx)?;
            }

            ctx.scoped(ScopeKind::Loop, |ctx| analyze_stmt(body, ctx))
        }
    }
}

fn analyze_jump_stmt<'ast, 'text>(
    stmt: &'ast ast::JumpStmt<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<(), SemanticError<'ast, 'text>> {
    match stmt {
        ast::JumpStmt::Goto(label) => match ctx.contains_label(label) {
            true => Ok(()),
            false => Err(SemanticError::UndefinedLabel(label)),
        },
        ast::JumpStmt::Continue => match ctx.in_loop() {
            true => Ok(()),
            false => Err(SemanticError::IllegalJump(stmt)),
        },
        ast::JumpStmt::Break => match ctx.in_loop() || ctx.in_switch() {
            true => Ok(()),
            false => Err(SemanticError::IllegalJump(stmt)),
        },
        ast::JumpStmt::Return(expr) => {
            let Some((return_ty, _)) = ctx.curr_fn_scope() else {
                // The parse won't allow return statement outside a function
                // so this check is redundant and can be safely unwrapped
                // instead of returning a Result::Err
                return Err(SemanticError::ReturnOutsideFn(stmt));
            };

            match (return_ty.clone(), expr) {
                (Type::Void, None) => Ok(()),
                (Type::Void, Some(expr)) => Err(SemanticError::ReturnTypeMismatch {
                    expected: Type::Void,
                    actual: analyze_assignment_expr(expr, ctx)?,
                }),
                (return_ty, None) => Err(SemanticError::ReturnTypeMismatch {
                    expected: return_ty,
                    actual: Type::Void,
                }),
                (return_ty, Some(expr)) => {
                    let expr_ty = analyze_assignment_expr(expr, ctx)?;
                    match return_ty == expr_ty {
                        true => Ok(()),
                        false => Err(SemanticError::ReturnTypeMismatch {
                            expected: return_ty.clone(),
                            actual: expr_ty,
                        }),
                    }
                }
            }
        }
    }
}

fn analyze_assignment_expr<'ast, 'text>(
    expr: &'ast ast::AssignmentExpr<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::AssignmentExpr::ConditionalExpr(expr) => analyze_conditional_expr(expr, ctx),
        ast::AssignmentExpr::Assign(lhs, rhs)
        | ast::AssignmentExpr::MulAssign(lhs, rhs)
        | ast::AssignmentExpr::DivAssign(lhs, rhs)
        | ast::AssignmentExpr::ModAssign(lhs, rhs)
        | ast::AssignmentExpr::AddAssign(lhs, rhs)
        | ast::AssignmentExpr::SubAssign(lhs, rhs)
        | ast::AssignmentExpr::ShiftLeftAssign(lhs, rhs)
        | ast::AssignmentExpr::ShiftRightAssign(lhs, rhs)
        | ast::AssignmentExpr::BitAndAssign(lhs, rhs)
        | ast::AssignmentExpr::XORAssign(lhs, rhs)
        | ast::AssignmentExpr::BitOrAssign(lhs, rhs) => {
            let lhs_ty = analyze_unary_expr(lhs, ctx)?;
            let rhs_ty = analyze_assignment_expr(rhs, ctx)?;
            match lhs_ty == rhs_ty {
                true => Ok(lhs_ty),
                false => Err(SemanticError::TypeMismatch(lhs_ty, rhs_ty)),
            }
        }
    }
}

fn analyze_conditional_expr<'ast, 'text>(
    expr: &'ast ast::ConditionalExpr<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::ConditionalExpr::LogicalOrExpr(expr) => analyze_logicalor_expr(expr, ctx),
        ast::ConditionalExpr::Ternary { test, pass, fail } => {
            let ty = analyze_logicalor_expr(test, ctx)?;
            if ty != Type::Int {
                return Err(SemanticError::UnexpectedType {
                    expected: Type::Int,
                    actual: ty,
                });
            }
            let pass_ty = analyze_assignment_expr(pass, ctx)?;
            let fail_ty = analyze_conditional_expr(fail, ctx)?;
            match pass_ty == fail_ty {
                true => Ok(pass_ty),
                false => Err(SemanticError::TypeMismatch(pass_ty, fail_ty)),
            }
        }
    }
}

fn analyze_logicalor_expr<'ast, 'text>(
    expr: &'ast ast::LogicalOrExpr<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::LogicalOrExpr::LogicalAndExpr(expr) => todo!(),
        ast::LogicalOrExpr::LogicalOr(lhs, rhs) => match (
            analyze_logicalor_expr(lhs, ctx)?,
            analyze_logicaland_expr(rhs, ctx)?,
        ) {
            (Type::Int, Type::Int) | (Type::Char, Type::Char) => Ok(Type::Int),
            _ => Err(SemanticError::InvalidBinaryOperands(BinOp::LogicalOr(expr))),
        },
    }
}

fn analyze_logicaland_expr<'ast, 'text>(
    expr: &'ast ast::LogicalAndExpr<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::LogicalAndExpr::BitOrExpr(expr) => analyze_bitor_expr(expr, ctx),
        ast::LogicalAndExpr::LogicalAnd(lhs, rhs) => match (
            analyze_logicaland_expr(lhs, ctx)?,
            analyze_bitor_expr(rhs, ctx)?,
        ) {
            (Type::Int, Type::Int) | (Type::Char, Type::Char) => Ok(Type::Int),
            _ => Err(SemanticError::InvalidBinaryOperands(BinOp::LogicalAnd(
                expr,
            ))),
        },
    }
}

fn analyze_bitor_expr<'ast, 'text>(
    expr: &'ast ast::BitOrExpr<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::BitOrExpr::XORExpr(xor_expr) => analyze_xor_expr(xor_expr, ctx),
        ast::BitOrExpr::BitOr(lhs, rhs) => {
            match (analyze_bitor_expr(lhs, ctx)?, analyze_xor_expr(rhs, ctx)?) {
                (Type::Int, Type::Int) | (Type::Char, Type::Char) => Ok(Type::Int),
                _ => Err(SemanticError::InvalidBinaryOperands(BinOp::BitOr(expr))),
            }
        }
    }
}

fn analyze_xor_expr<'ast, 'text>(
    expr: &'ast ast::XORExpr<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::XORExpr::BitAndExpr(expr) => analyze_bitand_expr(expr, ctx),
        ast::XORExpr::XOR(lhs, rhs) => {
            match (analyze_xor_expr(lhs, ctx)?, analyze_bitand_expr(rhs, ctx)?) {
                (Type::Int, Type::Int) | (Type::Char, Type::Char) => Ok(Type::Int),
                _ => Err(SemanticError::InvalidBinaryOperands(BinOp::XOR(expr))),
            }
        }
    }
}

fn analyze_bitand_expr<'ast, 'text>(
    expr: &'ast ast::BitAndExpr<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::BitAndExpr::EqualityExpr(expr) => analyze_equality_expr(expr, ctx),
        ast::BitAndExpr::BitAnd(lhs, rhs) => match (
            analyze_bitand_expr(lhs, ctx)?,
            analyze_equality_expr(rhs, ctx)?,
        ) {
            (Type::Int, Type::Int) | (Type::Char, Type::Char) => Ok(Type::Int),
            _ => Err(SemanticError::InvalidBinaryOperands(BinOp::BitAnd(expr))),
        },
    }
}

fn analyze_equality_expr<'ast, 'text>(
    expr: &'ast ast::EqualityExpr<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::EqualityExpr::ComparisionExpr(expr) => analyze_comparision_expr(expr, ctx),
        ast::EqualityExpr::EQ(lhs, rhs) | ast::EqualityExpr::NE(lhs, rhs) => match (
            analyze_equality_expr(lhs, ctx)?,
            analyze_comparision_expr(rhs, ctx)?,
        ) {
            (Type::Int, Type::Int)
            | (Type::Char, Type::Char)
            | (Type::Float, Type::Float)
            | (Type::Double, Type::Double) => Ok(Type::Int),
            _ => Err(SemanticError::InvalidBinaryOperands(BinOp::Equality(expr))),
        },
    }
}

fn analyze_comparision_expr<'ast, 'text>(
    expr: &'ast ast::ComparisionExpr<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::ComparisionExpr::ShiftExpr(shift_expr) => analyze_shift_expr(shift_expr, ctx),
        ast::ComparisionExpr::LT(lhs, rhs)
        | ast::ComparisionExpr::GT(lhs, rhs)
        | ast::ComparisionExpr::LE(lhs, rhs)
        | ast::ComparisionExpr::GE(lhs, rhs) => {
            let lhs_ty = analyze_comparision_expr(lhs, ctx)?;
            let rhs_ty = analyze_shift_expr(rhs, ctx)?;

            // only same types; avoid implicit typecasts for simplicity
            match (&lhs_ty, &rhs_ty) {
                (Type::Int, Type::Int)
                | (Type::Char, Type::Char)
                | (Type::Float, Type::Float)
                | (Type::Double, Type::Double) => Ok(lhs_ty),
                _ => Err(SemanticError::InvalidBinaryOperands(BinOp::Comparision(
                    expr,
                ))),
            }
        }
    }
}

fn analyze_shift_expr<'ast, 'text>(
    expr: &'ast ast::ShiftExpr<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::ShiftExpr::AdditiveExpr(additive_expr) => analyze_additive_expr(additive_expr, ctx),
        ast::ShiftExpr::ShiftLeft(lhs, rhs) | ast::ShiftExpr::ShiftRight(lhs, rhs) => match (
            analyze_shift_expr(lhs, ctx)?,
            analyze_additive_expr(rhs, ctx)?,
        ) {
            (Type::Int, Type::Int) => Ok(Type::Int),
            _ => Err(SemanticError::InvalidBinaryOperands(BinOp::Shift(expr))),
        },
    }
}

fn analyze_additive_expr<'ast, 'text>(
    expr: &'ast ast::AdditiveExpr<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::AdditiveExpr::MultiplicativeExpr(multiplicative_expr) => {
            analyze_multiplicative_expr(multiplicative_expr, ctx)
        }
        ast::AdditiveExpr::Add(lhs, rhs) | ast::AdditiveExpr::Sub(lhs, rhs) => {
            let lhs_ty = analyze_additive_expr(lhs, ctx)?;
            let rhs_ty = analyze_multiplicative_expr(rhs, ctx)?;

            // only same types; avoid implicit typecasts for simplicity
            match (&lhs_ty, &rhs_ty) {
                (Type::Int, Type::Int)
                | (Type::Float, Type::Float)
                | (Type::Double, Type::Double) => Ok(lhs_ty),
                _ => Err(SemanticError::InvalidBinaryOperands(BinOp::Additive(expr))),
            }
        }
    }
}

fn analyze_multiplicative_expr<'ast, 'text>(
    expr: &'ast ast::MultiplicativeExpr<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::MultiplicativeExpr::CastExpr(cast_expr) => analyze_cast_expr(cast_expr, ctx),
        ast::MultiplicativeExpr::Mul(lhs, rhs) | ast::MultiplicativeExpr::Div(lhs, rhs) => {
            let lhs_ty = analyze_multiplicative_expr(lhs, ctx)?;
            let rhs_ty = analyze_cast_expr(rhs, ctx)?;

            // only same types; avoid implicit typecasts for simplicity
            match (&lhs_ty, &rhs_ty) {
                (Type::Int, Type::Int)
                | (Type::Float, Type::Float)
                | (Type::Double, Type::Double) => Ok(lhs_ty),
                _ => Err(SemanticError::InvalidBinaryOperands(BinOp::Multiplicative(
                    expr,
                ))),
            }
        }
        ast::MultiplicativeExpr::Mod(lhs, rhs) => match (
            analyze_multiplicative_expr(lhs, ctx)?,
            analyze_cast_expr(rhs, ctx)?,
        ) {
            (Type::Int, Type::Int) => Ok(Type::Int),
            _ => Err(SemanticError::InvalidBinaryOperands(BinOp::Multiplicative(
                expr,
            ))),
        },
    }
}

fn analyze_cast_expr<'ast, 'text>(
    expr: &'ast ast::CastExpr<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::CastExpr::UnaryExpr(unary_expr) => analyze_unary_expr(unary_expr, ctx),
        ast::CastExpr::Cast(type_name, sub_expr) => {
            let target_type = analyze_type_name(type_name, ctx)?;
            let curr_type = analyze_cast_expr(sub_expr, ctx)?;

            match (&target_type, &curr_type) {
                (Type::Int, Type::Char)
                | (Type::Int, Type::Float)
                | (Type::Int, Type::Double)
                | (Type::Char, Type::Int)
                | (Type::Float, Type::Int)
                | (Type::Double, Type::Int) => Ok(target_type),
                (Type::Pointer(_), Type::Pointer(_)) => Ok(target_type),
                _ => Err(SemanticError::InvalidTypeCast {
                    from: curr_type,
                    to: target_type,
                }),
            }
        }
    }
}

fn analyze_unary_expr<'ast, 'text>(
    expr: &'ast ast::UnaryExpr<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::UnaryExpr::PostfixExpr(postfix_expr) => analyze_postfix_expr(postfix_expr, ctx),
        ast::UnaryExpr::PreIncr(inner_expr) | ast::UnaryExpr::PreDecr(inner_expr) => {
            let ty = analyze_unary_expr(inner_expr, ctx)?;
            match ty {
                Type::Int | Type::Char | Type::Float | Type::Double | Type::Pointer(_) => Ok(ty),
                _ => Err(SemanticError::InvalidUnaryOperand(expr)),
            }
        }
        ast::UnaryExpr::Ref(inner_expr) => {
            Ok(Type::Pointer(Box::new(analyze_cast_expr(inner_expr, ctx)?)))
        }
        ast::UnaryExpr::Deref(inner_expr) => match analyze_cast_expr(inner_expr, ctx)? {
            Type::Pointer(ty) => Ok(*ty),
            _ => Err(SemanticError::InvalidDereferenceOperand(expr)),
        },
        ast::UnaryExpr::UnaryAdd(inner_expr) | ast::UnaryExpr::UnarySub(inner_expr) => {
            let ty = analyze_cast_expr(inner_expr, ctx)?;
            match ty {
                Type::Int | Type::Char | Type::Float | Type::Double => Ok(ty),
                _ => Err(SemanticError::InvalidUnaryOperand(expr)),
            }
        }
        ast::UnaryExpr::OnesComplement(inner_expr) => {
            let ty = analyze_cast_expr(inner_expr, ctx)?;
            match ty {
                Type::Int | Type::Char => Ok(ty),
                _ => Err(SemanticError::InvalidUnaryOperand(expr)),
            }
        }
        ast::UnaryExpr::Not(inner_expr) => {
            let ty = analyze_cast_expr(inner_expr, ctx)?;
            match ty {
                Type::Int => Ok(ty),
                _ => Err(SemanticError::InvalidUnaryOperand(expr)),
            }
        }
        ast::UnaryExpr::SizeofExpr(inner_expr) => {
            analyze_unary_expr(inner_expr, ctx)?;
            Ok(Type::Int) // Sizeof always results in an integer value
        }
        ast::UnaryExpr::SizeofTypeName(inner_expr) => {
            analyze_type_name(inner_expr, ctx)?;
            Ok(Type::Int) // Sizeof always results in an integer value
        }
    }
}

fn analyze_postfix_expr<'ast, 'text>(
    expr: &'ast ast::PostfixExpr<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::PostfixExpr::Primary(expr) => analyze_primary_expr(expr, ctx),
        ast::PostfixExpr::ArrayAccess(inner_expr, access) => match (
            analyze_postfix_expr(inner_expr, ctx)?,
            analyze_assignment_expr(access, ctx)?,
        ) {
            (Type::Pointer(ty) | Type::Array(ty, _), Type::Int) => Ok(*ty),
            _ => Err(SemanticError::InvalidPostfixOperand(expr)),
        },
        ast::PostfixExpr::FunctionCall(inner_expr, args) => {
            match analyze_postfix_expr(inner_expr, ctx)? {
                Type::Function {
                    return_ty,
                    param_tys,
                } => {
                    if args.len() != param_tys.len() {
                        return Err(SemanticError::InvalidFnCall(expr));
                    }

                    for (arg, param_ty) in args.iter().zip(param_tys.iter()) {
                        let arg_ty = analyze_assignment_expr(arg, ctx)?;
                        if &arg_ty != param_ty {
                            return Err(SemanticError::InvalidFnCall(expr));
                        }
                    }
                    Ok(*return_ty)
                }
                Type::Pointer(ty) => match *ty {
                    Type::Function {
                        return_ty,
                        param_tys,
                    } => {
                        if args.len() != param_tys.len() {
                            return Err(SemanticError::InvalidFnCall(expr));
                        }

                        for (arg, param_ty) in args.iter().zip(param_tys.iter()) {
                            let arg_ty = analyze_assignment_expr(arg, ctx)?;
                            if &arg_ty != param_ty {
                                return Err(SemanticError::InvalidFnCall(expr));
                            }
                        }
                        Ok(*return_ty)
                    }
                    _ => Err(SemanticError::NotAFunction(inner_expr)),
                },
                _ => Err(SemanticError::NotAFunction(inner_expr)),
            }
        }
        ast::PostfixExpr::MemberAccess(inner_expr, field) => {
            match analyze_postfix_expr(inner_expr, ctx)? {
                Type::Struct { name, members } => members
                    .into_iter()
                    .find(|(name, _)| name == field)
                    .map(|(_, ty)| ty)
                    .ok_or(SemanticError::UndefinedMember {
                        struct_name: name,
                        field,
                    }),
                _ => Err(SemanticError::NotAStruct(inner_expr)),
            }
        }
        ast::PostfixExpr::PointerMemberAccess(inner_expr, field) => {
            match analyze_postfix_expr(inner_expr, ctx)? {
                Type::Pointer(inner_ty) => match *inner_ty {
                    Type::Struct { name, members } => members
                        .into_iter()
                        .find(|(name, _)| name == field)
                        .map(|(_, ty)| ty)
                        .ok_or(SemanticError::UndefinedMember {
                            struct_name: name,
                            field,
                        }),
                    _ => Err(SemanticError::NotAStruct(inner_expr)),
                },
                _ => Err(SemanticError::NotAPointerToStruct(inner_expr)),
            }
        }

        ast::PostfixExpr::PostIncr(inner_expr) | ast::PostfixExpr::PostDecr(inner_expr) => {
            let ty = analyze_postfix_expr(inner_expr, ctx)?;
            match ty {
                Type::Int | Type::Char | Type::Float | Type::Double | Type::Pointer(_) => Ok(ty),
                _ => Err(SemanticError::InvalidPostfixOperand(expr)),
            }
        }
    }
}

fn analyze_primary_expr<'ast, 'text>(
    expr: &'ast ast::Primary<'text>,
    ctx: &mut SemanticContext<'text>,
) -> Result<Type<'text>, SemanticError<'ast, 'text>> {
    match expr {
        ast::Primary::Ident(ident) => match ctx.find_var(ident) {
            Some(var) => Ok(var.ty.clone()),
            None => Err(SemanticError::UndefinedVariable(ident)),
        },
        ast::Primary::Int(_) => Ok(Type::Int),
        ast::Primary::Char(_) => Ok(Type::Char),
        ast::Primary::Float(_) => Ok(Type::Float),
        ast::Primary::EnumConstant(ident) => match ctx.find_enum_invariant(ident) {
            Some(e) => Ok(e.ty.clone()),
            None => Err(SemanticError::UndefinedVariable(ident)), // there is no such thing as undefind enum
        },
        ast::Primary::String(_) => Ok(Type::String),
        ast::Primary::Parens(expr) => analyze_assignment_expr(expr, ctx),
    }
}

//     let return_type = analyze_declaration_specifiers(&ast.return_type, ctx)?;

//     // Add function parameters to context
//     let params = analyze_parameter_list(&ast.declarator, ctx)?;

//     // Create a new context for the function body with parameters

//     ctx.scoped(|ctx| {
//         for param in &params {
//             let a = ctx.declare_var(&param.name, param.ty.clone());

//             ctx.declare_var(&param.name, param.ty.clone())?;
//         }

//         // Analyze function body
//         analyze_compound_statement(&ast.body, &mut ctx)?;
//     });

//     Ok(())
// }

// fn analyze_parameter_list<'ast, 'text>(
//     params: &ParameterTypeList<'text>,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<Vec<ParameterTypeList<'text>>, SemanticError<'ast, 'text>> {
//     match params {
//         ParameterTypeList::ParameterList(param_decls) => {
//             let mut analyzed_params = Vec::new();
//             for param_decl in param_decls {
//                 let param = analyze_parameter_declaration(param_decl, ctx)?;
//                 analyzed_params.push(param);
//             }
//             Ok(analyzed_params)
//         }
//         ParameterTypeList::VariadicParameterList(_) => {
//             // Handle variadic parameters if needed
//             unimplemented!("Variadic functions are not supported in this example");
//         }
//     }
// }

// fn analyze_parameter_declaration<'ast, 'text>(
//     param_decl: &ParameterDeclaration<'text>,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<Parameter<'text>, SemanticError<'ast, 'text>> {
//     match param_decl {
//         ParameterDeclaration::WithDeclarator(decl_specifiers, declarator) => {
//             let param_ty = analyze_declaration_specifiers(decl_specifiers, ctx)?;
//             let param_name = get_param_name_from_declarator(declarator);
//             Ok(Parameter {
//                 name: param_name,
//                 ty: param_ty,
//             })
//         }
//         ParameterDeclaration::WithAbstractDeclarator(_, _) => {
//             // Handle abstract declarators if needed
//             unimplemented!("Abstract declarators are not supported in this example");
//         }
//         ParameterDeclaration::OnlySpecifiers(_) => {
//             // Handle cases where the parameter has no name
//             unimplemented!("Parameter with no name is not supported in this example");
//         }
//     }
// }

// fn get_param_name_from_declarator<'text>(declarator: &Declarator<'ast, 'text>) -> &'text str {
//     // Extract the parameter name from the declarator
//     // Implement the logic to extract the name based on your language's rules
//     // For example, if the declarator is an identifier, return its name
//     // This is a placeholder and needs to be adapted based on your specific language
//     match &declarator.d_declarator {
//         DirectDeclarator::Ident(name, _) => name,
//         _ => unimplemented!("Unsupported declarator for function parameter"),
//     }
// }

// fn analyze_compound_statement<'ast, 'text>(
//     compound_stmt: &CompoundStmt<'text>,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     ctx.scoped(|ctx| {
//         for block_item in &compound_stmt.0 {
//             match block_item {
//                 BlockItem::Declaration(d) => analyze_declaration(d, ctx)?,
//                 BlockItem::Statement(stmt) => analyze_stmt(stmt, ctx)?,
//             }
//         }
//         Ok(())
//     })
// }

// fn analyze_stmt<'ast, 'text>(
//     stmt: &Stmt<'text>,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match stmt {
//         Stmt::EmptyStmt => { /* nothing to analyze */ }
//         Stmt::Expr(expr) => analyze_expr(expr, ctx)?,
//         Stmt::Labeled(stmt) => analyze_labeled_stmt(stmt, ctx)?,
//         Stmt::Compound(stmt) => analyze_compound_statement(stmt, ctx)?,
//         Stmt::Selection(stmt) => analyze_selection_stmt(stmt, ctx)?,
//         Stmt::Iteration(stmt) => analyze_iteration_stmt(stmt, ctx)?,
//         Stmt::Jump(stmt) => analyze_jump_stmt(stmt, ctx)?,
//     }
//     Ok(())
// }

// // Implement other semantic analysis functions as needed...

// fn analyze_labeled_stmt<'ast, 'text>(
//     stmt: &LabeledStmt<'text>,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match stmt {
//         LabeledStmt::Ident(label, stmt) => {
//             ctx.declare_label(label);
//             analyze_stmt(stmt, ctx)?;
//         }
//         LabeledStmt::Case(expr, stmt) => {
//             // TODO: in ctx specify that we are currently inside a switch stmt
//             // because case is only allowed in switch

//             // Analyze the statement following a case label
//             analyze_stmt(stmt, ctx)?;
//         }
//         LabeledStmt::Default(stmt) => {
//             // TODO: in ctx specify that we are currently inside a switch stmt
//             // because default is only allowed in switch

//             // Analyze the statement following the default label
//             analyze_stmt(stmt, ctx)?;
//         }
//     }

//     Ok(())
// }

// fn analyze_selection_stmt<'ast, 'text>(
//     selection_stmt: &SelectionStmt<'text>,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match selection_stmt {
//         SelectionStmt::If { test, pass } => {
//             analyze_expr(test, &Type::Int, ctx)?;
//             analyze_stmt(pass, ctx)?;
//         }
//         SelectionStmt::IfElse { test, pass, fail } => {
//             analyze_expr(test, &Type::Int, ctx)?;
//             analyze_stmt(pass, ctx)?;
//             analyze_stmt(fail, ctx)?;
//         }
//         SelectionStmt::Switch { test, pass } => {
//             // TODO: in ctx specify that we are currently inside a switch stmt
//             // because case and default is only allowed in switch
//             // and we need a way to figure out if we are inside a switch stmt
//             // when analyzing case and default

//             analyze_expr(test, &Type::Int, ctx)?;
//             analyze_stmt(pass, ctx)?;
//         }
//     }
//     Ok(())
// }

// fn analyze_iteration_stmt<'ast, 'text>(
//     stmt: &IterationStmt<'text>,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match stmt {
//         IterationStmt::While { test, body } => {
//             analyze_expr(test, ctx)?;

//             // Ensure the test expression has type 'int'
//             if test_type != Type::Int {
//                 return Err(SemanticError::InvalidIterationTestType);
//             }

//             // Analyze the body of the while loop
//             analyze_stmt(body, ctx)?;
//         }
//         IterationStmt::DoWhile { test, body } => {
//             // Analyze the body of the do-while loop
//             analyze_stmt(body, ctx)?;

//             let test_type = analyze_expr(test, ctx)?;

//             // Ensure the test expression has type 'int'
//             if test_type != Type::Int {
//                 return Err(SemanticError::InvalidIterationTestType);
//             }
//         }
//         IterationStmt::For {
//             init,
//             test,
//             update,
//             body,
//         } => {
//             // Analyze the initialization expression
//             if let Some(init_expr) = init {
//                 analyze_expr(init_expr, ctx)?;
//             }

//             // Analyze the test expression
//             if let Some(test_expr) = test {
//                 let test_type = analyze_expr(test_expr, ctx)?;

//                 // Ensure the test expression has type 'int'
//                 if test_type != Type::Int {
//                     return Err(SemanticError::InvalidIterationTestType);
//                 }
//             }

//             // Analyze the update expression
//             if let Some(update_expr) = update {
//                 analyze_expr(update_expr, ctx)?;
//             }

//             // Analyze the body of the for loop
//             analyze_stmt(body, ctx)?;
//         }
//     }

//     Ok(())
// }

// fn analyze_jump_stmt<'ast, 'text>(
//     stmt: &JumpStmt<'text>,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match stmt {
//         JumpStmt::Goto(label) => {
//             // Check if the label is declared
//             if !ctx.is_label_declared(label) {
//                 return Err(SemanticError::UndeclaredLabel(label.to_string()));
//             }
//         }
//         JumpStmt::Return(expr) => {
//             // Analyze the return expression if present
//             if let Some(expr) = expr {
//                 analyze_expr(expr, ctx)?;
//             }
//         }
//         // 'continue' and 'break' do not require additional analysis
//         JumpStmt::Continue | JumpStmt::Break => {}
//     }

//     Ok(())
// }

// fn analyze_declaration_specifiers<'ast, 'text>(
//     declaration_specifiers: &[DeclarationSpecifier<'text>],
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<Type, SemanticError<'ast, 'text>> {
//     // Implement logic to interpret declaration specifiers and return the corresponding type
//     // ...

//     Ok(Type::Int) // Placeholder, replace with actual logic
// }

// fn analyze_init_declarator<'ast, 'text>(
//     init_declarator: &InitDeclarator<'text>,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<Type, SemanticError<'ast, 'text>> {
//     match init_declarator {
//         InitDeclarator::Declared(declarator) => {
//             // Analyze the declared variable and update the context
//             analyze_declarator(declarator, ty, ctx)?;
//         }
//         InitDeclarator::Initialized(declarator, initializer) => {
//             // Analyze the declared variable and update the context
//             analyze_declarator(declarator, ty, ctx)?;

//             // Analyze the initializer expression
//             analyze_initializer(initializer, ty, ctx)?;
//         }
//     }

//     Ok(())
// }

// fn analyze_declarator<'ast, 'text>(
//     declarator: &Declarator<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     // Implement logic to analyze the declarator and update the context
//     // ...

//     Ok(())
// }

// fn analyze_initializer<'ast, 'text>(
//     initializer: &Initializer<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match initializer {
//         Initializer::Assignment(assignment_expr) => {
//             // Analyze the assignment expression
//             analyze_assignment_expr(assignment_expr, ty, ctx)?;
//         }
//         Initializer::InitializerList(initializer_list) => {
//             // Analyze each initializer in the list
//             for init in initializer_list {
//                 analyze_initializer(init, ty, ctx)?;
//             }
//         }
//     }

//     Ok(())
// }

// fn analyze_assignment_expr<'ast, 'text>(
//     assignment_expr: &AssignmentExpr<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match assignment_expr {
//         AssignmentExpr::ConditionalExpr(cond_expr) => {
//             // Analyze the conditional expression
//             analyze_conditional_expr(cond_expr, ty, ctx)?;
//         }
//         AssignmentExpr::Assign(lhs, rhs) => {
//             // Analyze the left-hand side (lhs) and right-hand side (rhs) of the assignment
//             analyze_unary_expr(lhs, ty, ctx)?;
//             analyze_assignment_expr(rhs, ty, ctx)?;

//             // Perform additional checks if needed
//             // ...

//             // Update the context if necessary
//             // ...
//         }
//         // Implement other assignment operators as needed...
//         _ => unimplemented!("Semantic analysis not implemented for this assignment expression"),
//     }

//     Ok(())
// }

// // Continue with similar functions for other AST nodes...

// fn analyze_conditional_expr<'ast, 'text>(
//     cond_expr: &ConditionalExpr<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     // Implement logic to analyze the conditional expression
//     // ...

//     Ok(())
// }

// fn analyze_expr<'ast, 'text>(
//     expr: &Expr<'text>,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match expr {
//         // Expr::AssignmentExpr(assignment_expr) => {
//         //     // Analyze the assignment expression
//         //     analyze_assignment_expr(assignment_expr, expected_type, ctx)?;
//         // }
//         // Expr::LogicalOrExpr(logical_or_expr) => {
//         //     // Analyze the logical OR expression
//         //     analyze_logical_or_expr(logical_or_expr, expected_type, ctx)?;
//         // }
//         // // Implement other expression types as needed...
//         // _ => unimplemented!("Semantic analysis not implemented for this expression"),
//         AssignmentExpr::ConditionalExpr(_) => analyze_conditional_expr(),
//         AssignmentExpr::Assign(_, _) => todo!(),
//         AssignmentExpr::MulAssign(_, _) => todo!(),
//         AssignmentExpr::DivAssign(_, _) => todo!(),
//         AssignmentExpr::ModAssign(_, _) => todo!(),
//         AssignmentExpr::AddAssign(_, _) => todo!(),
//         AssignmentExpr::SubAssign(_, _) => todo!(),
//         AssignmentExpr::ShiftLeftAssign(_, _) => todo!(),
//         AssignmentExpr::ShiftRightAssign(_, _) => todo!(),
//         AssignmentExpr::BitAndAssign(_, _) => todo!(),
//         AssignmentExpr::XORAssign(_, _) => todo!(),
//         AssignmentExpr::BitOrAssign(_, _) => todo!(),
//     };

//     Ok(())
// }

// fn analyze_logical_or_expr<'ast, 'text>(
//     logical_or_expr: &LogicalOrExpr<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match logical_or_expr {
//         LogicalOrExpr::LogicalAndExpr(logical_and_expr) => {
//             // Analyze the logical AND expression
//             analyze_logical_and_expr(logical_and_expr, ty, ctx)?;
//         }
//         LogicalOrExpr::LogicalOr(lhs, rhs) => {
//             // Analyze the left-hand side (lhs) and right-hand side (rhs) of the logical OR
//             analyze_logical_or_expr(lhs, ty, ctx)?;
//             analyze_logical_and_expr(rhs, ty, ctx)?;

//             // Perform additional checks if needed
//             // ...

//             // Update the context if necessary
//             // ...
//         }
//         // Implement other logical OR operations as needed...
//         _ => unimplemented!("Semantic analysis not implemented for this logical OR expression"),
//     }

//     Ok(())
// }

// fn analyze_logical_and_expr<'ast, 'text>(
//     logical_and_expr: &LogicalAndExpr<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match logical_and_expr {
//         LogicalAndExpr::BitOrExpr(bit_or_expr) => {
//             // Analyze the bitwise OR expression
//             analyze_bit_or_expr(bit_or_expr, ty, ctx)?;
//         }
//         LogicalAndExpr::LogicalAnd(lhs, rhs) => {
//             // Analyze the left-hand side (lhs) and right-hand side (rhs) of the logical AND
//             analyze_logical_and_expr(lhs, ty, ctx)?;
//             analyze_bit_or_expr(rhs, ty, ctx)?;

//             // Perform additional checks if needed
//             // ...

//             // Update the context if necessary
//             // ...
//         }
//         // Implement other logical AND operations as needed...
//         _ => unimplemented!("Semantic analysis not implemented for this logical AND expression"),
//     }

//     Ok(())
// }

// fn analyze_bit_or_expr<'ast, 'text>(
//     bit_or_expr: &BitOrExpr<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match bit_or_expr {
//         BitOrExpr::XORExpr(xor_expr) => {
//             // Analyze the XOR expression
//             analyze_xor_expr(xor_expr, ty, ctx)?;
//         }
//         BitOrExpr::BitOr(lhs, rhs) => {
//             // Analyze the left-hand side (lhs) and right-hand side (rhs) of the bitwise OR
//             analyze_bit_or_expr(lhs, ty, ctx)?;
//             analyze_xor_expr(rhs, ty, ctx)?;

//             // Perform additional checks if needed
//             // ...

//             // Update the context if necessary
//             // ...
//         }
//         // Implement other bitwise OR operations as needed...
//         _ => unimplemented!("Semantic analysis not implemented for this bitwise OR expression"),
//     }

//     Ok(())
// }

// fn analyze_xor_expr<'ast, 'text>(
//     xor_expr: &XORExpr<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match xor_expr {
//         XORExpr::BitAndExpr(bit_and_expr) => {
//             // Analyze the bitwise AND expression
//             analyze_bit_and_expr(bit_and_expr, ty, ctx)?;
//         }
//         XORExpr::XOR(lhs, rhs) => {
//             // Analyze the left-hand side (lhs) and right-hand side (rhs) of the XOR
//             analyze_xor_expr(lhs, ty, ctx)?;
//             analyze_bit_and_expr(rhs, ty, ctx)?;

//             // Perform additional checks if needed
//             // ...

//             // Update the context if necessary
//             // ...
//         }
//         // Implement other XOR operations as needed...
//         _ => unimplemented!("Semantic analysis not implemented for this XOR expression"),
//     }

//     Ok(())
// }

// // Continue with similar functions for other expression types...

// fn analyze_bit_and_expr<'ast, 'text>(
//     bit_and_expr: &BitAndExpr<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     // Implement logic to analyze the bitwise AND expression
//     // ...

//     Ok(())
// }

// // Continue with similar functions for other expression types...

// fn analyze_shift_expr<'ast, 'text>(
//     shift_expr: &ShiftExpr<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     // Implement logic to analyze the shift expression
//     // ...

//     Ok(())
// }

// // Continue with similar functions for other expression types...

// fn analyze_additive_expr<'ast, 'text>(
//     additive_expr: &AdditiveExpr<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     // Implement logic to analyze the additive expression
//     // ...

//     Ok(())
// }

// // Continue with similar functions for other expression types...

// fn analyze_multiplicative_expr<'ast, 'text>(
//     multiplicative_expr: &MultiplicativeExpr<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     // Implement logic to analyze the multiplicative expression
//     // ...

//     Ok(())
// }

// // Continue with similar functions for other expression types...

// fn analyze_cast_expr<'ast, 'text>(
//     cast_expr: &CastExpr<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match cast_expr {
//         CastExpr::UnaryExpr(unary_expr) => {
//             // Analyze the unary expression
//             analyze_unary_expr(unary_expr, ty, ctx)?;
//         }
//         CastExpr::Cast(type_name, expr) => {
//             // Analyze the type name and the expression
//             let cast_type = analyze_type_name(type_name, ctx)?;
//             analyze_expr(expr, &cast_type, ctx)?;

//             // Perform additional checks if needed
//             // ...

//             // Update the context if necessary
//             // ...
//         }
//         // Implement other cast operations as needed...
//         _ => unimplemented!("Semantic analysis not implemented for this cast expression"),
//     }

//     Ok(())
// }

// fn analyze_type_name<'ast, 'text>(
//     type_name: &TypeName<'text>,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<Type, SemanticError<'ast, 'text>> {
//     // Implement logic to analyze the type name and return the corresponding type
//     // ...

//     Ok(Type::Int) // Placeholder, replace with actual logic
// }

// // Continue with similar functions for other expression types...

// fn analyze_unary_expr<'ast, 'text>(
//     unary_expr: &UnaryExpr<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match unary_expr {
//         UnaryExpr::PostfixExpr(postfix_expr) => {
//             // Analyze the postfix expression
//             analyze_postfix_expr(postfix_expr, ty, ctx)?;
//         }
//         UnaryExpr::PreIncr(expr)
//         | UnaryExpr::PreDecr(expr)
//         | UnaryExpr::Ref(expr)
//         | UnaryExpr::Deref(expr)
//         | UnaryExpr::UnaryAdd(expr)
//         | UnaryExpr::UnarySub(expr)
//         | UnaryExpr::OnesComplement(expr)
//         | UnaryExpr::Not(expr) => {
//             // Analyze the sub-expression
//             analyze_cast_expr(expr, ty, ctx)?;
//         }
//         UnaryExpr::SizeofExpr(expr) => {
//             // Analyze the expression inside sizeof
//             analyze_unary_expr(expr, ty, ctx)?;

//             // Perform additional checks if needed
//             // ...

//             // Update the context if necessary
//             // ...
//         }
//         UnaryExpr::SizeofTypeName(type_name) => {
//             // Analyze the type name inside sizeof
//             analyze_type_name(type_name, ctx)?;

//             // Perform additional checks if needed
//             // ...

//             // Update the context if necessary
//             // ...
//         }
//         // Implement other unary operations as needed...
//         _ => unimplemented!("Semantic analysis not implemented for this unary expression"),
//     }

//     Ok(())
// }

// // Continue with similar functions for other expression types...

// fn analyze_postfix_expr<'ast, 'text>(
//     postfix_expr: &PostfixExpr<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match postfix_expr {
//         PostfixExpr::Primary(primary) => {
//             // Analyze the primary expression
//             analyze_primary(primary, ty, ctx)?;
//         }
//         PostfixExpr::ArrayAccess(expr, index_expr) => {
//             // Analyze the base expression and the index expression
//             analyze_postfix_expr(expr, ty, ctx)?;
//             analyze_expr(index_expr, &Type::Int, ctx)?;

//             // Perform additional checks if needed
//             // ...

//             // Update the context if necessary
//             // ...
//         }
//         PostfixExpr::FunctionCall(expr, args) => {
//             // Analyze the base expression and the function arguments
//             analyze_postfix_expr(expr, ty, ctx)?;
//             for arg in args {
//                 analyze_assignment_expr(arg, ty, ctx)?;
//             }

//             // Perform additional checks if needed
//             // ...

//             // Update the context if necessary
//             // ...
//         }
//         PostfixExpr::MemberAccess(expr, member)
//         | PostfixExpr::PointerMemberAccess(expr, member) => {
//             // Analyze the base expression
//             analyze_postfix_expr(expr, ty, ctx)?;

//             // Perform additional checks if needed
//             // ...

//             // Update the context if necessary
//             // ...
//         }
//         PostfixExpr::PostIncr(expr) | PostfixExpr::PostDecr(expr) => {
//             // Analyze the sub-expression
//             analyze_postfix_expr(expr, ty, ctx)?;

//             // Perform additional checks if needed
//             // ...

//             // Update the context if necessary
//             // ...
//         }
//         // Implement other postfix operations as needed...
//         _ => unimplemented!("Semantic analysis not implemented for this postfix expression"),
//     }

//     Ok(())
// }

// fn analyze_primary<'ast, 'text>(
//     primary: &Primary<'text>,
//     ty: &Type,
//     ctx: &mut SemanticContext<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     match primary {
//         Primary::Ident(ident) => {
//             // Check if the identifier is declared in the current context
//             ctx.find_var(ident)
//                 .ok_or(SemanticError::UndefinedVariable(ident))?;
//         }
//         Primary::Int(_) | Primary::Char(_) | Primary::Float(_) => {
//             // Primary literals, nothing to analyze
//         }
//         Primary::EnumConstant(enum_const) => {
//             // Check if the enum constant is declared in the current context
//             // ...

//             // Update the context if necessary
//             // ...
//         }
//         Primary::String(_) => {
//             // String literal, nothing to analyze
//         }
//         Primary::Parens(expr) => {
//             // Analyze the expression inside parentheses
//             analyze_expr(expr, ty, ctx)?;
//         }
//     }

//     Ok(())
// }

// // Implement similar functions for other AST nodes...

// // Finally, you can create a top-level function to start semantic analysis
// pub fn perform_semantic_analysis<'ast, 'text>(
//     translation_unit: &TranslationUnit<'text>,
// ) -> Result<(), SemanticError<'ast, 'text>> {
//     let mut ctx = SemanticContext::new();

//     // Analyze the translation unit
//     analyze_translation_unit(translation_unit, &mut ctx)?;

//     Ok(())
// }
