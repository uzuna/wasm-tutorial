use nalgebra::Vector2;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use webgl2::{gl, GlPoint2d};

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
    let mut shader = crate::shader::PlotShader::new(&gl, &prop)?;

    let a = wasm_utils::animation::AnimationLoop::new(move |time| {
        let time = (time / 500.0) as f32;
        let p = GlPoint2d::new(time.sin() / 2.0, time.cos() / 2.0);
        let mat = nalgebra::Matrix3::identity()
            .scale(time.sin() + 0.5)
            .append_translation(&Vector2::new(time.sin() / 2.0, 0.0));

        shader.set_window_mat(&gl, mat);
        shader.add_data(&gl, p);
        webgl2::context::gl_clear_color(&gl, webgl2::context::COLOR_BLACK);
        shader.draw(&gl);
        Ok(())
    });
    a.start();
    a.forget();
    Ok(())
}
