use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::log;

#[wasm_bindgen]
pub fn start_boids(canvas: HtmlCanvasElement) -> Result<(), JsValue> {
    log!("Starting boids");
    Ok(())
}
