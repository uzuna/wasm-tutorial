use wasm_bindgen::prelude::*;
use wasm_utils::error::*;
use web_sys::HtmlCanvasElement;
use webgl2::context::{Context, COLOR_BLACK};

#[wasm_bindgen(start)]
pub fn init() -> Result<()> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}
impl WindowSize {
    const DEFAULT: Self = Self {
        width: 1024,
        height: 768,
    };
}

#[wasm_bindgen]
impl WindowSize {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn default() -> Self {
        Self::DEFAULT
    }
}

#[wasm_bindgen]
pub fn start(canvas: HtmlCanvasElement, ws: WindowSize) -> std::result::Result<(), JsValue> {
    canvas.set_width(ws.width);
    canvas.set_height(ws.height);

    // テキスト表示
    let ctx = Context::new(canvas, COLOR_BLACK)?;
    let _gl = ctx.gl().clone();
    let _viewport = ctx.viewport();

    Ok(())
}
