use wasm_bindgen::prelude::*;
use wasm_utils::error::*;

/// エレメント取得のラッパー
pub fn get_element<T>(id: &str) -> Result<T>
where
    T: JsCast,
{
    web_sys::window()
        .ok_or(JsError::new("Failed to get window"))?
        .document()
        .ok_or(JsError::new("Failed to get document"))?
        .get_element_by_id(id)
        .ok_or(JsError::new(&format!("Failed to get element: {id}")))?
        .dyn_into::<T>()
        .map_err(|_| JsError::new(&format!("Failed to convert Element: {id}")))
}
