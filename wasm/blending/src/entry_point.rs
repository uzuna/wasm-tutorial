use std::rc::Rc;

use nalgebra::{Matrix3, Vector2};
use wasm_bindgen::prelude::*;
use wasm_utils::{error::*, info};
use web_sys::{HtmlCanvasElement, WebGlBuffer, WebGlProgram};
use webgl2::{gl, vertex::buffer_data_f32, GlPoint2d, Program};

use crate::shader::{color_texture, SingleColorShaderGl1, TextureShader};

#[wasm_bindgen(start)]
pub fn init() -> Result<()> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

/// ローカル座標変換行列
pub struct LocalMat {
    // windowによる形状の変化はローカルの時点で考慮する
    window_scaling: Vector2<f32>,
}

impl LocalMat {
    fn new(aspect: f32) -> Self {
        let window_scaling = Vector2::new(1.0 / aspect, 1.0);
        Self { window_scaling }
    }

    fn with_translation(&self, x: f32, y: f32) -> Matrix3<f32> {
        Matrix3::identity()
            .append_translation(&Vector2::new(x, y))
            .append_nonuniform_scaling(&self.window_scaling)
    }
}

#[wasm_bindgen]
pub fn start(canvas: HtmlCanvasElement) -> std::result::Result<(), JsValue> {
    let width = 500;
    let height = 300;
    canvas.set_width(width);
    canvas.set_height(height);
    let local_mat = LocalMat::new(width as f32 / height as f32);

    let gl = webgl2::context::get_context(&canvas, [0.0, 0.0, 0.0, 1.0])?;

    // 画面クリア
    gl.clear_color(0.0, 0.0, 0.75, 1.0);
    gl.clear_depth(1.0);
    gl.clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

    let gl = Rc::new(gl);
    let s = SingleColorShaderGl1::new(gl.clone())?;
    let v1: WebGlBuffer = s.create_vbo(&SingleColorShaderGl1::UNIT_RECT)?;
    let u = s.uniform();
    u.set_local_mat(local_mat.with_translation(-0.5, -0.5));

    u.set_color([1.0, 0.0, 0.0, 0.5]);
    s.draw(&v1);

    let v2 = s.create_vbo(&SingleColorShaderGl1::UNIT_RECT)?;
    u.set_local_mat(local_mat.with_translation(0.5, 0.5));
    u.set_color([0.0, 1.0, 0.0, 0.5]);
    s.draw(&v2);

    Ok(())
}

#[wasm_bindgen]
pub fn start_webgl2_gradiation(canvas: HtmlCanvasElement) -> std::result::Result<(), JsValue> {
    canvas.set_width(500);
    canvas.set_height(300);

    let gl = webgl2::context::get_context(&canvas, [0.0, 0.0, 0.0, 1.0])?;
    let gl = Rc::new(gl);

    // 画面クリア
    gl.clear_color(0.0, 0.0, 0.75, 1.0);
    gl.clear_depth(1.0);
    gl.clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

    let rect_left = [
        GlPoint2d::new(-1.0, 0.5),
        GlPoint2d::new(0.5, 0.5),
        GlPoint2d::new(-1.0, -1.0),
        GlPoint2d::new(0.5, -1.0),
    ];

    let t = color_texture(&gl, [255, 0, 0, 128]);
    let s = TextureShader::new(gl.clone())?;
    let vao = s.create_vao(&rect_left)?;
    gl.bind_texture(gl::TEXTURE_2D, Some(&t));
    s.draw(&vao);

    let rect_left = [
        GlPoint2d::new(-0.5, 1.0),
        GlPoint2d::new(1.0, 1.0),
        GlPoint2d::new(-0.5, -0.5),
        GlPoint2d::new(1.0, -0.5),
    ];

    let t = color_texture(&gl, [0, 255, 0, 128]);
    let s = TextureShader::new(gl.clone())?;
    let vao = s.create_vao(&rect_left)?;
    gl.bind_texture(gl::TEXTURE_2D, Some(&t));
    s.draw(&vao);

    Ok(())
}

#[wasm_bindgen]
pub fn get_context_rs(canvas: HtmlCanvasElement) -> Result<gl> {
    let gl = canvas
        .get_context("experimental-webgl")
        .map_err(|_| JsError::new("Failed to get_context(webgl)"))?
        .ok_or(JsError::new("Failed to get WebGlRenderingContext Object"))?
        .dyn_into::<gl>()
        .map_err(|_| JsError::new("Failed to cast to WebGlRenderingContext"))?;
    Ok(gl)
}

#[wasm_bindgen]
pub fn create_program_rs(gl: gl) -> std::result::Result<WebGlProgram, JsValue> {
    let p = Program::new(&gl, SingleColorShaderGl1::VERT, SingleColorShaderGl1::FRAG)?;
    p.use_program(&gl);
    Ok(p.into_program())
}

#[wasm_bindgen]
pub fn get_attr_location_rs(gl: gl, prg: &WebGlProgram, name: &str) -> i32 {
    gl.get_attrib_location(prg, name)
}

#[wasm_bindgen]
pub fn create_vbo_rs(gl: gl, data: &[f32]) -> std::result::Result<WebGlBuffer, JsValue> {
    let vbo = gl.create_buffer().ok_or("failed to create buffer")?;
    gl.bind_buffer(gl::ARRAY_BUFFER, Some(&vbo));
    info!("bind_buffer {:?}", gl.get_error());
    buffer_data_f32(&gl, gl::ARRAY_BUFFER, data, gl::STATIC_DRAW);
    info!("buffer_data_with_array_buffer_view {:?}", gl.get_error());
    gl.bind_buffer(gl::ARRAY_BUFFER, None);
    Ok(vbo)
}

#[wasm_bindgen]
pub fn bind_buffer_rs(gl: gl, attr: u32, vbo: &WebGlBuffer) -> std::result::Result<(), JsValue> {
    gl.bind_buffer(gl::ARRAY_BUFFER, Some(vbo));
    gl.enable_vertex_attrib_array(attr);
    gl.vertex_attrib_pointer_with_i32(attr, 2, gl::FLOAT, false, 0, 0);
    Ok(())
}

#[wasm_bindgen]
pub fn clear_canvas_rs(gl: gl) -> std::result::Result<(), JsValue> {
    gl.clear_color(0.0, 0.0, 0.75, 1.0);
    gl.clear_depth(1.0);
    gl.clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    Ok(())
}

#[wasm_bindgen]
pub fn set_uniform_color_rs(
    gl: gl,
    prg: &WebGlProgram,
    color: js_sys::Float32Array,
) -> std::result::Result<(), JsValue> {
    let u_color = gl.get_uniform_location(prg, "u_color");
    gl.uniform4fv_with_f32_sequence(u_color.as_ref(), &color);
    Ok(())
}

#[wasm_bindgen]
pub fn draw_rs(gl: gl) -> std::result::Result<(), JsValue> {
    gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
    Ok(())
}

#[wasm_bindgen]
pub fn setup_depth_test_rs(gl: gl) {
    gl.enable(gl::DEPTH_TEST);
    gl.depth_func(gl::LEQUAL);
}
