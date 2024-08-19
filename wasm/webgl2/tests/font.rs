//! Test suite for the Web and headless browsers.
#![cfg(feature = "font")]
#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;

use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;
use webgl2::font::TextShader;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_fontshader() -> std::result::Result<(), JsValue> {
    let doc = web_sys::window()
        .ok_or("Failed to get Window")?
        .document()
        .ok_or("Failed to get Document")?;

    let canvas = doc
        .create_element("canvas")
        .expect("Could not create testing node");
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

    let ctx = webgl2::context::Context::new(canvas, webgl2::context::COLOR_BLACK)?;

    let _s = TextShader::new(&ctx)?;

    Ok(())
}
