use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_utils::{error::*, info};
use web_sys::HtmlCanvasElement;

use crate::{
    camera::{Camera, ViewMatrix},
    shader::{PlaneShader, SimpleShader, VertexObject},
    webgl::gl,
};

#[wasm_bindgen(start)]
pub fn init() -> Result<()> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
pub fn start(canvas: HtmlCanvasElement) -> std::result::Result<(), JsValue> {
    canvas.set_width(1024);
    canvas.set_height(768);

    let aspect = 1024.0 / 768.0;
    let gl = crate::webgl::get_context(&canvas, [0.0, 0.0, 0.0, 1.0])?;

    // アルファブレンドを有効にする
    gl.enable(gl::BLEND);
    gl.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

    // 深度テストを有効にする
    gl.enable(gl::DEPTH_TEST);
    gl.depth_func(gl::LEQUAL);

    // 画面クリア
    gl.clear_color(0.0, 0.75, 0.75, 1.0);
    gl.clear_depth(1.0);
    gl.clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

    let gl = Rc::new(gl);
    let s = SimpleShader::new(gl.clone())?;
    s.draw();

    // let camera = Camera::default();
    // let view = ViewMatrix::default();
    // let mvp = camera.perspective().as_matrix() * view.look_at();
    // let s = PlaneShader::new(gl.clone())?;
    // let v = VertexObject::rect(gl.clone())?;
    // let u = s.uniforms();
    // u.set_mvp(mvp);
    // s.draw(&v);

    // info!("gl error {}", gl.get_error());

    Ok(())
}
