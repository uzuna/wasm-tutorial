use wasm_bindgen::JsError;

pub type Result<T> = std::result::Result<T, JsError>;
