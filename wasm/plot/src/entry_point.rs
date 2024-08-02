use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use webgl2::gl;

use crate::shader::PlotParams;

#[wasm_bindgen(start)]
pub fn init() -> Result<(), JsValue> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
pub fn start(canvas: HtmlCanvasElement) -> Result<(), JsValue> {
    canvas.set_width(1024);
    canvas.set_height(768);

    let gl = webgl2::context::get_webgl2_context(&canvas, webgl2::context::COLOR_BLACK)?;
    gl.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    let mut prop = PlotParams::default();
    prop.point_size = 10.0;
    let shader = crate::shader::PlotShader::new(&gl, &prop)?;
    shader.draw(&gl);
    Ok(())
}
