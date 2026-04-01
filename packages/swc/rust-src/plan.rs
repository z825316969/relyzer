use serde::{Deserialize, Serialize};
use swc_common::Span;
use swc_ecma_ast::Pat;

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

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfig {
    pub auto_detect: Option<bool>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct TransformPlan {
    pub import_runtime: bool,
    pub components: Vec<ComponentPlan>,
}

#[derive(Debug, Clone)]
pub struct ComponentPlan {
    pub component_span: Span,
    pub body_span: Option<Span>,
    pub code: String,
    pub loc: Option<String>,
    pub observed_list: Vec<ObservedMeta>,
    pub should_detect_call_stack: bool,
    pub is_explicit: bool,
    pub is_auto_detected: bool,
    pub props_mode: PropsMode,
    pub inject_points: Vec<InjectPoint>,
}

#[derive(Debug, Clone)]
pub enum PropsMode {
    None,
    Ident { name: String },
    ObjectPattern { temp_name: String, original: Pat },
}

#[derive(Debug, Clone)]
pub enum InjectPoint {
    Var { stmt_span: Span, name: String, index: usize },
    DepExpr { expr_span: Span, name: String, index: usize },
    AttrExpr { expr_span: Span, attr_name: String, index: usize },
}
