#![cfg(feature = "context")]
#![cfg(feature = "metrics")]
#![cfg(feature = "texture")]
#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;

use std::{assert_eq, sync::atomic::Ordering::Relaxed};

use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// メトリクスがRAIIにできている。
#[wasm_bindgen_test]
fn test_metrics_texture() -> std::result::Result<(), JsValue> {
    let doc = web_sys::window()
        .ok_or("Failed to get Window")?
        .document()
        .ok_or("Failed to get Document")?;

    let canvas = doc
        .create_element("canvas")
        .expect("Could not create testing node");
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

    let ctx = webgl2::context::Context::new(canvas, webgl2::context::COLOR_BLACK)?;
    let mut textures = vec![];
    let len = 1000;
    for _ in 0..len {
        textures.push(ctx.create_blank_texture()?);
    }
    let metrics = ctx.metrics();

    assert_eq!(len, metrics.texture.texture_count.load(Relaxed));
    assert_eq!(len as u64 * 4, metrics.texture.bytes_count.load(Relaxed));

    drop(textures);

    assert_eq!(0, metrics.texture.texture_count.load(Relaxed));
    assert_eq!(0, metrics.texture.bytes_count.load(Relaxed));

    Ok(())
}
