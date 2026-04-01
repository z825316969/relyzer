use swc_common::Span;
use swc_ecma_ast::*;
use swc_ecma_visit::{VisitMut, VisitMutWith};

use crate::plan::{ComponentPlan, PluginConfig, TransformPlan};
use crate::runtime_ast::{make_use_relyzer_decl, make_use_relyzer_import, private_ident};

pub struct RelyzerTransform {
    pub config: PluginConfig,
    pub plan: TransformPlan,
    pub runtime_ident: Option<Ident>,
}

impl RelyzerTransform {
    pub fn new(config: PluginConfig, plan: TransformPlan) -> Self {
        Self {
            config,
            plan,
            runtime_ident: None,
        }
    }

    fn component_plan_for_span(&self, span: Span) -> Option<&ComponentPlan> {
        self.plan.components.iter().find(|p| p.component_span == span)
    }

    fn ensure_runtime_import(&mut self, module: &mut Module) {
        if self.runtime_ident.is_some() {
            return;
        }

        let local = private_ident("_useRelyzer");
        module.body.insert(0, make_use_relyzer_import(local.clone()));
        self.runtime_ident = Some(local);
    }
}

fn inject_into_function(function: &mut Function, plan: &ComponentPlan, runtime_ident: &Ident) {
    let body = match &mut function.body {
        Some(body) => body,
        None => return,
    };

    let perf_ident = private_ident("_p");
    let prepended = vec![make_use_relyzer_decl(runtime_ident, &perf_ident, plan)];
    body.stmts.splice(0..0, prepended);
}

impl VisitMut for RelyzerTransform {
    fn visit_mut_module(&mut self, module: &mut Module) {
        if self.plan.import_runtime || !self.plan.components.is_empty() {
            self.ensure_runtime_import(module);
        }
        module.visit_mut_children_with(self);
    }

    fn visit_mut_fn_decl(&mut self, n: &mut FnDecl) {
        n.visit_mut_children_with(self);
        if let (Some(plan), Some(runtime_ident)) = (self.component_plan_for_span(n.function.span), self.runtime_ident.as_ref()) {
            inject_into_function(&mut n.function, plan, runtime_ident);
        }
    }

    fn visit_mut_fn_expr(&mut self, n: &mut FnExpr) {
        n.visit_mut_children_with(self);
        if let (Some(plan), Some(runtime_ident)) = (self.component_plan_for_span(n.function.span), self.runtime_ident.as_ref()) {
            inject_into_function(&mut n.function, plan, runtime_ident);
        }
    }

    fn visit_mut_arrow_expr(&mut self, n: &mut ArrowExpr) {
        n.visit_mut_children_with(self);
        if let (Some(plan), Some(runtime_ident)) = (self.component_plan_for_span(n.span), self.runtime_ident.as_ref()) {
            if let BlockStmtOrExpr::BlockStmt(block) = &mut *n.body {
                let perf_ident = private_ident("_p");
                let prepended = vec![make_use_relyzer_decl(runtime_ident, &perf_ident, plan)];
                block.stmts.splice(0..0, prepended);
            }
        }
    }
}
