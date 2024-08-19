use wasm_bindgen::JsError;
use wasm_utils::error::*;
use web_sys::Storage;

pub fn local_storage() -> Result<Storage> {
    let storage = web_sys::window()
        .ok_or(JsError::new("Failed to get Window"))?
        .local_storage()
        .map_err(|_| JsError::new("Failed to get LocalStorage"))?
        .ok_or(JsError::new("LocalStorage response is None"))?;

    Ok(storage)
}

pub fn document() -> Result<web_sys::Document> {
    web_sys::window()
        .ok_or(JsError::new("Failed to get Window"))?
        .document()
        .ok_or(JsError::new("Failed to get Document"))
}
