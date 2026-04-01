mod api;
mod analyzer;
mod plan;
mod runtime_ast;
mod transform;
#[cfg(target_arch = "wasm32")]
mod plugin;

pub use api::analyze;
pub use plan::{AnalyzeResult, ComponentMetaData, ImportMeta, ObservedMeta, PluginConfig, TransformPlan, ComponentPlan, PropsMode, InjectPoint};
