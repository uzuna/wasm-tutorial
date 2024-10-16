use std::cell::RefCell;

use fxhash::FxHashMap;
use wasm_bindgen::prelude::*;
use wasm_utils::error::*;

thread_local! {
    /// JSに登録するClosureはそのままではWASM空間内ではライフタイムが切れてしまう
    /// forgetだと削除ができなくなるので、thread_localで保持する
    #[allow(clippy::type_complexity)]
    pub(super) static SELECT_CLOSURES: RefCell<FxHashMap<String,Closure<dyn FnMut()>>> = RefCell::new(FxHashMap::default());
}

/// DOMidに対するクロージャ登録があるかどうか
pub(super) fn contains(id: &str) -> bool {
    SELECT_CLOSURES.with_borrow(|closures| closures.contains_key(id))
}

/// イベントリスナー登録したクロージャをスレッドローカルメモリに登録する
pub(super) fn insert(id: &str, closure: Closure<dyn FnMut()>) {
    SELECT_CLOSURES.with(|closures| {
        closures.borrow_mut().insert(id.to_string(), closure);
    });
}

/// エレメント取得のラッパー
pub(super) fn get_element<T>(id: &str) -> Result<T>
where
    T: wasm_bindgen::JsCast,
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

/// エレメントを作成のラッパー
pub(super) fn create_element<T>(tag: &str) -> Result<T>
where
    T: wasm_bindgen::JsCast,
{
    web_sys::window()
        .ok_or(JsError::new("window is None"))?
        .document()
        .ok_or(JsError::new("document is None"))?
        .create_element(tag)
        .map_err(|_| JsError::new("cannot create element"))?
        .dyn_into::<T>()
        .map_err(|_| JsError::new("cannot convert to HtmlElement"))
}

/// イベントリスナーを登録する
pub(super) fn add_event_listener(
    element: &web_sys::EventTarget,
    event: &str,
    callback: &JsValue,
) -> Result<()> {
    element
        .add_event_listener_with_callback(event, callback.unchecked_ref())
        .map_err(|_| JsError::new("Failed to add event listener"))?;
    Ok(())
}

pub(super) fn remove_closure(id: &str) {
    SELECT_CLOSURES.with(|closures| {
        closures.borrow_mut().remove(id);
    });
}
