use nalgebra_glm::Vec3;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use webgl2::gl;

use crate::{
    boids::Boid,
    boids_shader::BoidShader,
    camera::{Camera, ViewMatrix},
    info,
};

const COLOR_BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

#[wasm_bindgen(start)]
pub fn init() -> Result<(), JsValue> {
    info!("execute init");
    crate::utils::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
pub fn start_boids(canvas: HtmlCanvasElement) -> Result<(), JsValue> {
    info!("Starting boids");
    canvas.set_width(768);
    canvas.set_height(768);

    let boids = crate::boids::Boids::new(vec![Boid::new(Vec3::zeros(), Vec3::zeros())]);
    info!("{:?}", boids);

    let gl = get_webgl2_context(&canvas)?;
    let camera = Camera::default();
    let view = ViewMatrix::default();

    let bi = BoidShader::new(&gl, &Boid::zero())?;
    bi.use_program(&gl);
    bi.set_mvp(&gl, &camera, &view);
    bi.set_ambient(&gl, [1.0, 0.0, 0.0, 1.0]);
    bi.draw(&gl);

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
