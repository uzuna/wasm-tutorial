use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_utils::error::*;
use web_sys::HtmlCanvasElement;

#[wasm_bindgen(start)]
pub fn init() -> Result<()> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
pub fn start(canvas: HtmlCanvasElement) -> std::result::Result<(), JsValue> {
    let width = 500;
    let height = 300;
    canvas.set_width(width);
    canvas.set_height(height);

    let gl = webgl2::context::get_context(&canvas, webgl2::context::COLOR_BLACK)?;
    let gl = Rc::new(gl);

    let s = webgl2::shader::texture::TextureShader::new(gl.clone())?;
    let v = s.create_vao(&webgl2::vertex::UNIT_RECT)?;
    let texture = webgl2::shader::texture::color_texture(&gl, [0, 128, 0, 255]);
    s.draw(&v, &texture);

    Ok(())
}
