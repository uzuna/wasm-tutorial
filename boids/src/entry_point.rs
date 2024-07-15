use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use webgl2::gl;

use crate::info;

const COLOR_BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

#[wasm_bindgen]
pub fn start_boids(canvas: HtmlCanvasElement) -> Result<(), JsValue> {
    info!("Starting boids");
    canvas.set_width(256);
    canvas.set_height(256);

    let _gl = get_webgl2_context(&canvas)?;

    Ok(())
}

fn get_webgl2_context(canvas: &HtmlCanvasElement) -> Result<gl, JsValue> {
    let gl = canvas
        .get_context("webgl2")?
        .ok_or("Failed to get WebGl2RenderingContext")?
        .dyn_into::<gl>()?;

    gl.enable(gl::DEPTH_TEST);
    gl.depth_func(gl::LEQUAL);
    gl.enable(gl::CULL_FACE);

    gl_clear_color(&gl, COLOR_BLACK);
    gl.clear_depth(1.0);
    gl.clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    Ok(gl)
}

#[inline]
fn gl_clear_color(gl: &gl, color: [f32; 4]) {
    gl.clear_color(color[0], color[1], color[2], color[3]);
    gl.clear(gl::COLOR_BUFFER_BIT);
}
