use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
struct AnalyzeResult {
    ok: bool,
    message: String,
}

#[wasm_bindgen]
pub fn analyze(_code: &str) -> String {
    let result = AnalyzeResult {
        ok: false,
        message: "SWC analyzer scaffold is ready, but AST mapping is not implemented yet.".to_string(),
    };

    serde_json::to_string(&result).unwrap()
}
