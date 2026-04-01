use swc_common::DUMMY_SP;
use swc_ecma_ast::*;

use crate::plan::ComponentPlan;

pub fn private_ident(name: &str) -> Ident {
    Ident::new(name.into(), DUMMY_SP)
}

pub fn make_use_relyzer_import(local: Ident) -> ModuleItem {
    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        span: DUMMY_SP,
        specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
            span: DUMMY_SP,
            local,
            imported: Some(ModuleExportName::Ident(Ident::new("useRelyzer".into(), DUMMY_SP))),
            is_type_only: false,
        })],
        src: Box::new(Str {
            span: DUMMY_SP,
            value: "@relyzer/runtime".into(),
            raw: None,
        }),
        type_only: false,
        with: None,
        phase: Default::default(),
    }))
}

pub fn make_metadata_object(plan: &ComponentPlan) -> Expr {
    let mut props = vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(Ident::new("id".into(), DUMMY_SP)),
            value: Box::new(Expr::Lit(Lit::Str(Str { span: DUMMY_SP, value: "SWC_PLUGIN_TODO".into(), raw: None }))),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(Ident::new("code".into(), DUMMY_SP)),
            value: Box::new(Expr::Lit(Lit::Str(Str { span: DUMMY_SP, value: plan.code.clone().into(), raw: None }))),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(Ident::new("observedList".into(), DUMMY_SP)),
            value: Box::new(Expr::Array(ArrayLit { span: DUMMY_SP, elems: vec![] })),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(Ident::new("shouldDetectCallStack".into(), DUMMY_SP)),
            value: Box::new(Expr::Lit(Lit::Bool(Bool { span: DUMMY_SP, value: plan.should_detect_call_stack }))),
        }))),
    ];

    if let Some(loc) = &plan.loc {
        props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(Ident::new("loc".into(), DUMMY_SP)),
            value: Box::new(Expr::Lit(Lit::Str(Str { span: DUMMY_SP, value: loc.clone().into(), raw: None }))),
        }))));
    }

    Expr::Object(ObjectLit { span: DUMMY_SP, props })
}

pub fn make_use_relyzer_decl(runtime_ident: &Ident, perf_ident: &Ident, plan: &ComponentPlan) -> Stmt {
    Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent { id: perf_ident.clone(), type_ann: None }),
            init: Some(Box::new(Expr::Call(CallExpr {
                span: DUMMY_SP,
                callee: Callee::Expr(Box::new(Expr::Ident(runtime_ident.clone()))),
                args: vec![ExprOrSpread { spread: None, expr: Box::new(make_metadata_object(plan)) }],
                type_args: None,
            }))),
            definite: false,
        }],
    })))
}

pub fn make_perf_call_expr(perf_ident: &Ident, expr: Box<Expr>, index: i32) -> Expr {
    Expr::Call(CallExpr {
        span: DUMMY_SP,
        callee: Callee::Expr(Box::new(Expr::Ident(perf_ident.clone()))),
        args: vec![
            ExprOrSpread { spread: None, expr },
            ExprOrSpread { spread: None, expr: Box::new(Expr::Lit(Lit::Num(Number { span: DUMMY_SP, value: index as f64, raw: None }))) },
        ],
        type_args: None,
    })
}

pub fn make_perf_call_stmt(perf_ident: &Ident, expr: Box<Expr>, index: i32) -> Stmt {
    Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(make_perf_call_expr(perf_ident, expr, index)),
    })
}
