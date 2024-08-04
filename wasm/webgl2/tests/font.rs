//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;

use std::assert_eq;

use wasm_bindgen::{prelude::*, JsError};
use wasm_bindgen_test::*;
use web_sys::WebGlUniformLocation;
use webgl2::{font::TextShader, gl, Program};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_fontshader() -> std::result::Result<(), JsValue> {
    let doc = web_sys::window()
        .ok_or("Failed to get Window")?
        .document()
        .ok_or("Failed to get Document")?;
    let body = doc.body().ok_or("Failed to create Body")?;

    let canvas = doc
        .create_element("canvas")
        .expect("Could not create testing node");
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

    let gl = canvas
        .get_context("webgl2")?
        .ok_or("Failed to get WebGl2RenderingContext")?
        .dyn_into::<gl>()?;

    let _s = TextShader::new(&gl)?;

    Ok(())
}
