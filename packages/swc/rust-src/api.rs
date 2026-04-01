use wasm_bindgen::prelude::*;

use crate::analyzer::analyze_module_to_result;

#[wasm_bindgen]
pub fn analyze(code: &str) -> String {
    let result = analyze_module_to_result(code);
    serde_json::to_string(&result).unwrap()
}
