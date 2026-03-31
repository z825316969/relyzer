use serde::{Deserialize, Serialize};
use swc_common::{comments::{Comments, SingleThreadedComments}, sync::Lrc, FileName, SourceMap, SourceMapper, Span, Spanned};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};
use swc_ecma_visit::{Visit, VisitWith};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ObservedMeta {
    pub name: String,
    pub loc: String,
    #[serde(rename = "type")]
    pub observed_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ComponentMetaData {
    pub id: String,
    pub name: Option<String>,
    pub code: String,
    pub loc: Option<String>,
    pub observed_list: Vec<ObservedMeta>,
    pub should_detect_call_stack: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportMeta {
    pub source: String,
    pub specifiers: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeResult {
    pub ok: bool,
    pub imports: Vec<ImportMeta>,
    pub components: Vec<ComponentMetaData>,
    pub errors: Vec<String>,
}

fn span_to_loc(cm: &Lrc<SourceMap>, span: Span) -> Option<String> {
    if span.is_dummy() {
        return None;
    }
    let start = cm.lookup_char_pos(span.lo);
    let end = cm.lookup_char_pos(span.hi);
    Some(format!("{},{},{},{}", start.line, start.col_display, end.line, end.col_display))
}

fn relative_loc(cm: &Lrc<SourceMap>, base: Span, child: Span) -> Option<String> {
    if base.is_dummy() || child.is_dummy() {
        return None;
    }
    let base_start = cm.lookup_char_pos(base.lo);
    let child_start = cm.lookup_char_pos(child.lo);
    let child_end = cm.lookup_char_pos(child.hi);

    let start_col = if child_start.line == base_start.line {
        child_start.col_display.saturating_sub(base_start.col_display)
    } else {
        child_start.col_display
    };

    let end_col = if child_end.line == base_start.line {
        child_end.col_display.saturating_sub(base_start.col_display)
    } else {
        child_end.col_display
    };

    Some(format!(
        "{},{},{},{}",
        child_start.line.saturating_sub(base_start.line),
        start_col,
        child_end.line.saturating_sub(base_start.line),
        end_col
    ))
}

fn extract_code(cm: &Lrc<SourceMap>, span: Span) -> String {
    cm.span_to_snippet(span).unwrap_or_default()
}

fn random_id() -> String {
    format!("SWC_{:x}", js_sys::Date::now() as u64)
}

fn is_first_cap(name: &str) -> bool {
    name.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false)
}

fn has_component_comment(comments: &SingleThreadedComments, span: Span) -> bool {
    if let Some(items) = comments.get_leading(span.lo()) {
        items.iter().any(|c| c.text.contains("@component"))
    } else {
        false
    }
}

fn has_relyzer_directive(body: &BlockStmt) -> bool {
    body.stmts.iter().any(|stmt| match stmt {
        Stmt::Expr(expr) => match &*expr.expr {
            Expr::Lit(Lit::Str(s)) => s.value.as_ref() == "use relyzer",
            _ => false,
        },
        _ => false,
    })
}

fn fn_name_from_decl(decl: &FnDecl) -> Option<String> {
    Some(decl.ident.sym.to_string())
}

fn fn_name_from_var_declarator(var: &VarDeclarator) -> Option<String> {
    match &var.name {
        Pat::Ident(id) => Some(id.id.sym.to_string()),
        _ => None,
    }
}

fn expr_name(expr: &Expr) -> String {
    match expr {
        Expr::Ident(id) => id.sym.to_string(),
        _ => format!("span:{:?}", expr.span()),
    }
}

fn collect_pat_idents(pat: &Pat, out: &mut Vec<(String, Span)>) {
    match pat {
        Pat::Ident(id) => out.push((id.id.sym.to_string(), id.id.span)),
        Pat::Array(arr) => {
            for elem in &arr.elems {
                if let Some(p) = elem {
                    collect_pat_idents(p, out);
                }
            }
        }
        Pat::Object(obj) => {
            for prop in &obj.props {
                match prop {
                    ObjectPatProp::Assign(assign) => out.push((assign.key.sym.to_string(), assign.key.span)),
                    ObjectPatProp::KeyValue(kv) => collect_pat_idents(&kv.value, out),
                    ObjectPatProp::Rest(rest) => collect_pat_idents(&rest.arg, out),
                }
            }
        }
        Pat::Assign(assign) => collect_pat_idents(&assign.left, out),
        Pat::Rest(rest) => collect_pat_idents(&rest.arg, out),
        _ => {}
    }
}

struct ComponentCollector {
    cm: Lrc<SourceMap>,
    component_span: Span,
    observed: Vec<ObservedMeta>,
}

impl ComponentCollector {
    fn new(cm: Lrc<SourceMap>, component_span: Span) -> Self {
        Self {
            cm,
            component_span,
            observed: vec![],
        }
    }

    fn push_observed(&mut self, observed_type: &str, name: String, span: Span) {
        if let Some(loc) = relative_loc(&self.cm, self.component_span, span) {
            if !self.observed.iter().any(|item| item.loc == loc) {
                self.observed.push(ObservedMeta {
                    name,
                    loc,
                    observed_type: observed_type.to_string(),
                });
            }
        }
    }
}

impl Visit for ComponentCollector {
    fn visit_var_declarator(&mut self, n: &VarDeclarator) {
        let mut idents = vec![];
        collect_pat_idents(&n.name, &mut idents);
        for (name, span) in idents {
            self.push_observed("var", name, span);
        }
        n.visit_children_with(self);
    }

    fn visit_call_expr(&mut self, n: &CallExpr) {
        if let Callee::Expr(callee) = &n.callee {
            if let Expr::Ident(id) = &**callee {
                if (id.sym.as_ref() == "useCallback" || id.sym.as_ref() == "useMemo") && n.args.len() > 1 {
                    if let Expr::Array(arr) = &*n.args[1].expr {
                        for expr_or_spread in &arr.elems {
                            if let Some(expr_or_spread) = expr_or_spread {
                                self.push_observed(
                                    "dep",
                                    expr_name(&expr_or_spread.expr),
                                    expr_or_spread.expr.span(),
                                );
                            }
                        }
                    }
                }
            }
        }
        n.visit_children_with(self);
    }

    fn visit_jsx_opening_element(&mut self, n: &JSXOpeningElement) {
        let is_native = matches!(&n.name, JSXElementName::Ident(id) if id.sym.as_ref().chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false));
        if !is_native {
            for attr_or_spread in &n.attrs {
                if let JSXAttrOrSpread::JSXAttr(attr) = attr_or_spread {
                    let attr_name = match &attr.name {
                        JSXAttrName::Ident(id) => id.sym.to_string(),
                        JSXAttrName::JSXNamespacedName(ns) => format!("{}:{}", ns.ns.sym, ns.name.sym),
                    };

                    if let Some(value) = &attr.value {
                        match value {
                            JSXAttrValue::Lit(Lit::Str(_)) => {}
                            JSXAttrValue::JSXExprContainer(container) => {
                                if let JSXExpr::Expr(expr) = &container.expr {
                                    self.push_observed("attr", attr_name, expr.span());
                                }
                            }
                            JSXAttrValue::JSXElement(el) => self.push_observed("attr", attr_name, el.span),
                            JSXAttrValue::JSXFragment(frag) => self.push_observed("attr", attr_name, frag.span),
                            _ => {}
                        }
                    }
                }
            }
        }

        n.visit_children_with(self);
    }
}

struct Analyzer {
    cm: Lrc<SourceMap>,
    comments: SingleThreadedComments,
    imports: Vec<ImportMeta>,
    components: Vec<ComponentMetaData>,
}

impl Analyzer {
    fn push_component(&mut self, name: Option<String>, span: Span, body: Option<&BlockStmt>, auto_detected: bool) {
        let explicit = has_component_comment(&self.comments, span)
            || body.map(has_relyzer_directive).unwrap_or(false);
        let inferred = name.as_ref().map(|n| is_first_cap(n)).unwrap_or(false);
        let is_component = explicit || (auto_detected && inferred);

        if !is_component {
            return;
        }

        let mut observed_list = vec![];

        if let Some(block) = body {
            let mut collector = ComponentCollector::new(self.cm.clone(), span);
            collector.visit_block_stmt(block);
            observed_list = collector.observed;
        }

        self.components.push(ComponentMetaData {
            id: random_id(),
            name,
            code: extract_code(&self.cm, span),
            loc: span_to_loc(&self.cm, span),
            observed_list,
            should_detect_call_stack: auto_detected,
        });
    }
}

impl Visit for Analyzer {
    fn visit_import_decl(&mut self, n: &ImportDecl) {
        let specifiers = n
            .specifiers
            .iter()
            .map(|s| match s {
                ImportSpecifier::Named(named) => named.local.sym.to_string(),
                ImportSpecifier::Default(default) => default.local.sym.to_string(),
                ImportSpecifier::Namespace(ns) => ns.local.sym.to_string(),
            })
            .collect();

        self.imports.push(ImportMeta {
            source: n.src.value.to_string(),
            specifiers,
        });
    }

    fn visit_fn_decl(&mut self, n: &FnDecl) {
        self.push_component(fn_name_from_decl(n), n.function.span, n.function.body.as_ref(), true);
        n.visit_children_with(self);
    }

    fn visit_var_declarator(&mut self, n: &VarDeclarator) {
        let name = fn_name_from_var_declarator(n);
        if let Some(init) = &n.init {
            match &**init {
                Expr::Fn(fn_expr) => {
                    self.push_component(name.clone(), fn_expr.function.span, fn_expr.function.body.as_ref(), true);
                }
                Expr::Arrow(arrow) => {
                    let body = match &*arrow.body {
                        BlockStmtOrExpr::BlockStmt(block) => Some(block),
                        BlockStmtOrExpr::Expr(_) => None,
                    };
                    self.push_component(name.clone(), arrow.span, body, true);
                }
                Expr::Call(call) => {
                    if let Some(first) = call.args.first() {
                        match &*first.expr {
                            Expr::Fn(fn_expr) => self.push_component(name.clone(), fn_expr.function.span, fn_expr.function.body.as_ref(), true),
                            Expr::Arrow(arrow) => {
                                let body = match &*arrow.body {
                                    BlockStmtOrExpr::BlockStmt(block) => Some(block),
                                    BlockStmtOrExpr::Expr(_) => None,
                                };
                                self.push_component(name.clone(), arrow.span, body, true);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        n.visit_children_with(self);
    }
}

#[wasm_bindgen]
pub fn analyze(code: &str) -> String {
    let cm: Lrc<SourceMap> = Default::default();
    let comments = SingleThreadedComments::default();
    let fm = cm.new_source_file(FileName::Custom("input.tsx".into()).into(), code.to_string());

    let lexer = Lexer::new(
        Syntax::Typescript(TsConfig {
            tsx: true,
            decorators: true,
            dts: false,
            no_early_errors: true,
            disallow_ambiguous_jsx_like: false,
        }),
        EsVersion::Es2022,
        StringInput::from(&*fm),
        Some(&comments),
    );

    let mut parser = Parser::new_from(lexer);
    let mut errors = vec![];

    let module = match parser.parse_module() {
        Ok(module) => module,
        Err(err) => {
            errors.push(format!("parse error: {err:?}"));
            return serde_json::to_string(&AnalyzeResult {
                ok: false,
                imports: vec![],
                components: vec![],
                errors,
            }).unwrap();
        }
    };

    for err in parser.take_errors() {
        errors.push(format!("parse warning: {err:?}"));
    }

    let mut analyzer = Analyzer {
        cm,
        comments,
        imports: vec![],
        components: vec![],
    };

    module.visit_with(&mut analyzer);

    serde_json::to_string(&AnalyzeResult {
        ok: errors.is_empty(),
        imports: analyzer.imports,
        components: analyzer.components,
        errors,
    }).unwrap()
}
