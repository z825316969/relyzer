use crate::analyzer::analyze_program_to_plan;
use crate::plan::PluginConfig;
use crate::transform::RelyzerTransform;
use swc_ecma_ast::Program;
use swc_ecma_visit::VisitMutWith;

#[cfg(target_arch = "wasm32")]
use swc_plugin_macro::plugin_transform;
#[cfg(target_arch = "wasm32")]
use swc_plugin_proxy::TransformPluginProgramMetadata;

#[cfg(target_arch = "wasm32")]
#[plugin_transform]
pub fn process_transform(mut program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let config: PluginConfig = metadata
        .get_transform_plugin_config()
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default();

    let plan = analyze_program_to_plan(&program, &config);
    let mut transformer = RelyzerTransform::new(config, plan);
    program.visit_mut_with(&mut transformer);
    program
}
