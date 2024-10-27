use wasm_bindgen::prelude::*;

use crate::error::Result;

/// エレメント取得のラッパー
pub fn get_element<T>(id: impl AsRef<str>) -> Result<T>
where
    T: wasm_bindgen::JsCast,
{
    let id = id.as_ref();
    web_sys::window()
        .ok_or(JsError::new("Failed to get window"))?
        .document()
        .ok_or(JsError::new("Failed to get document"))?
        .get_element_by_id(id)
        .ok_or(JsError::new(&format!("Failed to get element: {id}")))?
        .dyn_into::<T>()
        .map_err(|_| JsError::new(&format!("Failed to convert Element: {id}")))
}

/// エレメントを作成のラッパー
pub fn create_element<T>(tag: impl AsRef<str>) -> Result<T>
where
    T: wasm_bindgen::JsCast,
{
    web_sys::window()
        .ok_or(JsError::new("window is None"))?
        .document()
        .ok_or(JsError::new("document is None"))?
        .create_element(tag.as_ref())
        .map_err(|_| JsError::new("cannot create element"))?
        .dyn_into::<T>()
        .map_err(|_| JsError::new("cannot convert to HtmlElement"))
}

pub fn get_body() -> Result<web_sys::HtmlElement> {
    web_sys::window()
        .ok_or(JsError::new("window is None"))?
        .document()
        .ok_or(JsError::new("document is None"))?
        .body()
        .ok_or(JsError::new("body is None"))?
        .dyn_into::<web_sys::HtmlElement>()
        .map_err(|_| JsError::new("cannot convert to HtmlElement"))
}

pub fn get_window() -> Result<web_sys::Window> {
    web_sys::window().ok_or(JsError::new("window is None"))
}
