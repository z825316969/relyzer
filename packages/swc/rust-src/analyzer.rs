use serde_json;
use swc_common::{comments::{Comments, SingleThreadedComments}, sync::Lrc, FileName, SourceMap, SourceMapper, Span, Spanned};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};
use swc_ecma_visit::{Visit, VisitWith};

use crate::plan::{AnalyzeResult, ComponentMetaData, ComponentPlan, ImportMeta, InjectPoint, ObservedMeta, PluginConfig, PropsMode, TransformPlan};

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
    let start_col = if child_start.line == base_start.line { child_start.col_display.saturating_sub(base_start.col_display) } else { child_start.col_display };
    let end_col = if child_end.line == base_start.line { child_end.col_display.saturating_sub(base_start.col_display) } else { child_end.col_display };
    Some(format!("{},{},{},{}", child_start.line.saturating_sub(base_start.line), start_col, child_end.line.saturating_sub(base_start.line), end_col))
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

fn has_component_comment_for_any(comments: &SingleThreadedComments, spans: &[Span]) -> bool {
    spans.iter().any(|span| {
        !span.is_dummy()
            && comments
                .get_leading(span.lo())
                .map(|items| items.iter().any(|c| c.text.contains("@component")))
                .unwrap_or(false)
    })
}

fn jsx_attr_name_span(name: &JSXAttrName) -> Span {
    match name {
        JSXAttrName::Ident(id) => id.span,
        JSXAttrName::JSXNamespacedName(ns) => Span::new(ns.ns.span.lo, ns.name.span.hi, Default::default()),
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

fn props_name_from_pat(first: Option<&Pat>) -> Option<String> {
    match first {
        Some(Pat::Ident(id)) => Some(id.id.sym.to_string()),
        Some(Pat::Object(_)) | None => Some("props".to_string()),
        _ => None,
    }
}

fn props_mode_from_pat(first: Option<&Pat>) -> PropsMode {
    match first {
        None => PropsMode::None,
        Some(Pat::Ident(id)) => PropsMode::Ident { name: id.id.sym.to_string() },
        Some(Pat::Object(obj)) => PropsMode::ObjectPattern { temp_name: "_props".to_string(), original: Pat::Object(obj.clone()) },
        _ => PropsMode::None,
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
    inject_points: Vec<InjectPoint>,
}

impl ComponentCollector {
    fn new(cm: Lrc<SourceMap>, component_span: Span) -> Self {
        Self { cm, component_span, observed: vec![], inject_points: vec![] }
    }

    fn push_observed(&mut self, observed_type: &str, name: String, span: Span) {
        if let Some(loc) = relative_loc(&self.cm, self.component_span, span) {
            if !self.observed.iter().any(|item| item.loc == loc) {
                let index = self.observed.len();
                self.observed.push(ObservedMeta { name: name.clone(), loc, observed_type: observed_type.to_string() });
                match observed_type {
                    "dep" => self.inject_points.push(InjectPoint::DepExpr { expr_span: span, name, index }),
                    "attr" => self.inject_points.push(InjectPoint::AttrExpr { expr_span: span, attr_name: name, index }),
                    _ => {}
                }
            }
        }
    }
}

impl Visit for ComponentCollector {
    fn visit_var_declarator(&mut self, n: &VarDeclarator) {
        let mut idents = vec![];
        collect_pat_idents(&n.name, &mut idents);
        for (name, span) in idents {
            if let Some(loc) = relative_loc(&self.cm, self.component_span, span) {
                if !self.observed.iter().any(|item| item.loc == loc) {
                    let index = self.observed.len();
                    self.observed.push(ObservedMeta { name: name.clone(), loc, observed_type: "var".to_string() });
                    self.inject_points.push(InjectPoint::Var { stmt_span: n.span, name, index });
                }
            }
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
                                self.push_observed("dep", expr_name(&expr_or_spread.expr), expr_or_spread.expr.span());
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
                                if let JSXExpr::Expr(_expr) = &container.expr {
                                    self.push_observed("attr", attr_name, jsx_attr_name_span(&attr.name));
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
    plans: Vec<ComponentPlan>,
}

impl Analyzer {
    fn push_component(
        &mut self,
        name: Option<String>,
        span: Span,
        body: Option<&BlockStmt>,
        props_name: Option<String>,
        props_mode: PropsMode,
        comment_spans: Vec<Span>,
        is_auto_detected: bool,
    ) {
        let explicit = has_component_comment_for_any(&self.comments, &comment_spans) || body.map(has_relyzer_directive).unwrap_or(false);
        let inferred = name.as_ref().map(|n| is_first_cap(n)).unwrap_or(false);
        let is_auto_component = is_auto_detected && inferred;
        let is_component = explicit || is_auto_component;
        if !is_component { return; }

        let mut collector = ComponentCollector::new(self.cm.clone(), span);
        let _ = props_name;
        if let Some(block) = body { collector.visit_block_stmt(block); }

        let code = extract_code(&self.cm, span);
        let loc = span_to_loc(&self.cm, span);
        let normalized_name = if is_auto_component { name } else { None };

        if self.components.iter().any(|item| item.code == code && item.loc == loc && item.should_detect_call_stack == is_auto_component) {
            return;
        }

        self.components.push(ComponentMetaData {
            id: random_id(),
            name: normalized_name,
            code: code.clone(),
            loc: loc.clone(),
            observed_list: collector.observed.clone(),
            should_detect_call_stack: is_auto_component,
        });

        self.plans.push(ComponentPlan {
            component_span: span,
            body_span: body.map(|b| b.span),
            code,
            loc,
            observed_list: collector.observed,
            should_detect_call_stack: is_auto_component,
            is_explicit: explicit,
            is_auto_detected: is_auto_component,
            props_mode,
            inject_points: collector.inject_points,
        });
    }
}

impl Visit for Analyzer {
    fn visit_module_decl(&mut self, n: &ModuleDecl) {
        if let ModuleDecl::ExportDecl(export_decl) = n {
            if let Decl::Fn(fn_decl) = &export_decl.decl {
                let explicit = has_component_comment_for_any(&self.comments, &[export_decl.span, fn_decl.ident.span, fn_decl.function.span])
                    || fn_decl.function.body.as_ref().map(has_relyzer_directive).unwrap_or(false);
                if explicit {
                    self.push_component(
                        None,
                        fn_decl.function.span,
                        fn_decl.function.body.as_ref(),
                        props_name_from_pat(fn_decl.function.params.first().map(|p| &p.pat)),
                        props_mode_from_pat(fn_decl.function.params.first().map(|p| &p.pat)),
                        vec![export_decl.span, fn_decl.ident.span, fn_decl.function.span],
                        false,
                    );
                }
            }
        }
        n.visit_children_with(self);
    }

    fn visit_import_decl(&mut self, n: &ImportDecl) {
        let specifiers = n.specifiers.iter().map(|s| match s {
            ImportSpecifier::Named(named) => named.local.sym.to_string(),
            ImportSpecifier::Default(default) => default.local.sym.to_string(),
            ImportSpecifier::Namespace(ns) => ns.local.sym.to_string(),
        }).collect();
        self.imports.push(ImportMeta { source: n.src.value.to_string(), specifiers });
    }

    fn visit_fn_decl(&mut self, n: &FnDecl) {
        let name = fn_name_from_decl(n);
        self.push_component(
            name.clone(),
            n.function.span,
            n.function.body.as_ref(),
            props_name_from_pat(n.function.params.first().map(|p| &p.pat)),
            props_mode_from_pat(n.function.params.first().map(|p| &p.pat)),
            vec![n.ident.span, n.function.span],
            name.as_ref().map(|n| is_first_cap(n)).unwrap_or(false),
        );
        n.visit_children_with(self);
    }

    fn visit_var_declarator(&mut self, n: &VarDeclarator) {
        let name = fn_name_from_var_declarator(n);
        let mut base_comment_spans = vec![n.name.span(), n.span];
        if let Some(init) = &n.init {
            match &**init {
                Expr::Fn(fn_expr) => {
                    let mut comment_spans = base_comment_spans.clone();
                    comment_spans.push(fn_expr.function.span);
                    self.push_component(
                        name.clone(),
                        fn_expr.function.span,
                        fn_expr.function.body.as_ref(),
                        props_name_from_pat(fn_expr.function.params.first().map(|p| &p.pat)),
                        props_mode_from_pat(fn_expr.function.params.first().map(|p| &p.pat)),
                        comment_spans,
                        name.as_ref().map(|n| is_first_cap(n)).unwrap_or(false),
                    );
                }
                Expr::Arrow(arrow) => {
                    let body = match &*arrow.body { BlockStmtOrExpr::BlockStmt(block) => Some(block), BlockStmtOrExpr::Expr(_) => None };
                    let mut comment_spans = base_comment_spans.clone();
                    comment_spans.push(arrow.span);
                    self.push_component(
                        name.clone(),
                        arrow.span,
                        body,
                        props_name_from_pat(arrow.params.first()),
                        props_mode_from_pat(arrow.params.first()),
                        comment_spans,
                        name.as_ref().map(|n| is_first_cap(n)).unwrap_or(false),
                    );
                }
                Expr::Call(call) => {
                    base_comment_spans.push(call.span);
                    if let Some(first) = call.args.first() {
                        match &*first.expr {
                            Expr::Fn(fn_expr) => {
                                let mut comment_spans = base_comment_spans.clone();
                                comment_spans.push(fn_expr.function.span);
                                self.push_component(
                                    name.clone(),
                                    fn_expr.function.span,
                                    fn_expr.function.body.as_ref(),
                                    props_name_from_pat(fn_expr.function.params.first().map(|p| &p.pat)),
                                    props_mode_from_pat(fn_expr.function.params.first().map(|p| &p.pat)),
                                    comment_spans,
                                    name.as_ref().map(|n| is_first_cap(n)).unwrap_or(false),
                                )
                            }
                            Expr::Arrow(arrow) => {
                                let body = match &*arrow.body { BlockStmtOrExpr::BlockStmt(block) => Some(block), BlockStmtOrExpr::Expr(_) => None };
                                let mut comment_spans = base_comment_spans.clone();
                                comment_spans.push(arrow.span);
                                self.push_component(
                                    name.clone(),
                                    arrow.span,
                                    body,
                                    props_name_from_pat(arrow.params.first()),
                                    props_mode_from_pat(arrow.params.first()),
                                    comment_spans,
                                    name.as_ref().map(|n| is_first_cap(n)).unwrap_or(false),
                                );
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

fn parse_module(code: &str) -> (Option<(Module, Lrc<SourceMap>, SingleThreadedComments)>, Vec<String>) {
    let cm: Lrc<SourceMap> = Default::default();
    let comments = SingleThreadedComments::default();
    let fm = cm.new_source_file(FileName::Custom("input.tsx".into()).into(), code.to_string());
    let lexer = Lexer::new(
        Syntax::Typescript(TsConfig { tsx: true, decorators: true, dts: false, no_early_errors: true, disallow_ambiguous_jsx_like: false }),
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
            return (None, errors);
        }
    };
    for err in parser.take_errors() {
        errors.push(format!("parse warning: {err:?}"));
    }
    (Some((module, cm, comments)), errors)
}

pub fn analyze_module_to_result(code: &str) -> AnalyzeResult {
    let (parsed, errors) = parse_module(code);
    let Some((module, cm, comments)) = parsed else {
        return AnalyzeResult { ok: false, imports: vec![], components: vec![], errors };
    };

    let mut analyzer = Analyzer { cm, comments, imports: vec![], components: vec![], plans: vec![] };
    module.visit_with(&mut analyzer);
    AnalyzeResult { ok: errors.is_empty(), imports: analyzer.imports, components: analyzer.components, errors }
}

pub fn analyze_program_to_plan(program: &Program, _config: &PluginConfig) -> TransformPlan {
    match program {
        Program::Module(module) => {
            let code = serde_json::to_string(module).unwrap_or_default();
            let result = analyze_module_to_result(&code);
            TransformPlan {
                import_runtime: !result.components.is_empty(),
                components: vec![],
            }
        }
        Program::Script(_) => TransformPlan { import_runtime: false, components: vec![] },
    }
}
