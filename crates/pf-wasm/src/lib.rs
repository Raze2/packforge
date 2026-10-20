//! WebAssembly bindings. Data crosses the boundary as JSON strings (and bytes for files),
//! which keeps the JS API small and framework-agnostic.
//!
//! Errors are thrown as JSON `{"code": "...", "message": "..."}` strings.

use pf_core::{CoreError, Format, Project};
use serde::Serialize;
use wasm_bindgen::prelude::*;

fn to_json<T: Serialize>(v: &T) -> Result<String, JsValue> {
    serde_json::to_string(v).map_err(|e| error("serialization_error", &e.to_string()))
}

fn error(code: &str, message: &str) -> JsValue {
    JsValue::from_str(&serde_json::json!({ "code": code, "message": message }).to_string())
}

fn core_err(e: CoreError) -> JsValue {
    error(e.code(), &e.to_string())
}

fn parse_project(json: &str) -> Result<Project, JsValue> {
    serde_json::from_str(json).map_err(|e| error("invalid_project", &e.to_string()))
}

/// Crate version, e.g. for "About" dialogs.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// JSON array of component summaries.
#[wasm_bindgen(js_name = listComponents)]
pub fn list_components() -> Result<String, JsValue> {
    to_json(&pf_core::list_components().map_err(core_err)?)
}

/// JSON parameter schema of one component.
#[wasm_bindgen]
pub fn describe(component_id: &str) -> Result<String, JsValue> {
    to_json(&pf_core::describe(component_id).map_err(core_err)?)
}

/// JSON array of board presets.
#[wasm_bindgen]
pub fn boards() -> Result<String, JsValue> {
    to_json(&pf_core::boards())
}

/// Project JSON in, `RenderResult` JSON out.
#[wasm_bindgen]
pub fn render(project_json: &str) -> Result<String, JsValue> {
    let p = parse_project(project_json)?;
    to_json(&pf_core::render(&p).map_err(core_err)?)
}

/// Project JSON + format (`"svg"` | `"dxf"`) in, file bytes out.
#[wasm_bindgen(js_name = exportFile)]
pub fn export_file(project_json: &str, format: &str) -> Result<Vec<u8>, JsValue> {
    let p = parse_project(project_json)?;
    let f = Format::parse(format)
        .ok_or_else(|| error("unknown_format", &format!("unsupported format '{format}'")))?;
    Ok(pf_core::export(&p, f).map_err(core_err)?.bytes)
}
