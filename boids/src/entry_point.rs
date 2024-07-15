use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use webgl2::gl;

use crate::{
    animation,
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

    let mut boids = crate::boids::Boids::new_circle(180, 0.5, 0.01);
    info!("{:?}", boids);

    let gl = get_webgl2_context(&canvas)?;
    let camera = Camera::default();
    let view = ViewMatrix::default();

    let boid_size = 0.01;
    let mut boids_shaders: Vec<BoidShader> = vec![];
    for b in boids.boids.iter() {
        let bi = BoidShader::new(&gl, b, boid_size)?;
        bi.use_program(&gl);
        bi.set_mvp(&gl, &camera, &view);
        bi.set_ambient(&gl, [1.0, 0.0, 0.0, 1.0]);
        bi.draw(&gl);
        boids_shaders.push(bi);
    }

    let a = animation::AnimationLoop::new(move |_| {
        gl_clear_color(&gl, COLOR_BLACK);
        for (b, s) in boids.boids.iter().zip(boids_shaders.iter_mut()) {
            s.use_program(&gl);
            s.update(&gl, b);
            s.draw(&gl);
        }
        boids.update();
        Ok(())
    });
    a.start()?;

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
