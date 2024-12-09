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

/// Bodyを取得のラッパー
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

/// ウィンドウを取得のラッパー
pub fn get_window() -> Result<web_sys::Window> {
    web_sys::window().ok_or(JsError::new("window is None"))
}

/// パフォーマンスを取得のラッパー
pub fn get_performance() -> Result<web_sys::Performance> {
    web_sys::window()
        .ok_or(JsError::new("Failed to get window"))?
        .performance()
        .ok_or(JsError::new("Failed to get performance"))
}

/// エレメントリストを取得のラッパー
pub fn get_elements<T>(class_name: impl AsRef<str>) -> Result<Vec<T>>
where
    T: wasm_bindgen::JsCast,
{
    let class_name = class_name.as_ref();
    let elements = web_sys::window()
        .ok_or(JsError::new("Failed to get window"))?
        .document()
        .ok_or(JsError::new("Failed to get document"))?
        .get_elements_by_class_name(class_name);
    let mut result = Vec::new();
    for i in 0..elements.length() {
        let element = elements
            .item(i)
            .ok_or(JsError::new("Failed to get element"))?
            .dyn_into::<T>()
            .map_err(|_| JsError::new("Failed to convert to T"))?;
        result.push(element);
    }
    Ok(result)
}

/// イベントリスナーを登録する
pub fn add_event_listener(
    element: &web_sys::EventTarget,
    event: &str,
    callback: &JsValue,
) -> Result<()> {
    element
        .add_event_listener_with_callback(event, callback.unchecked_ref())
        .map_err(|_| JsError::new("Failed to add event listener"))?;
    Ok(())
}

/// イベントリスナーを削除する
pub fn remove_event_listener(
    element: &web_sys::EventTarget,
    event: &str,
    callback: &JsValue,
) -> Result<()> {
    element
        .remove_event_listener_with_callback(event, callback.unchecked_ref())
        .map_err(|_| JsError::new("Failed to remove event listener"))?;
    Ok(())
}
