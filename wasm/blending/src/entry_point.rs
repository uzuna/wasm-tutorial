use std::{cell::RefCell, rc::Rc};

use nalgebra::{Matrix3, Vector2};
use wasm_bindgen::{convert::IntoWasmAbi, prelude::*};
use wasm_utils::{animation::AnimationLoop, error::*, info};
use web_sys::{HtmlCanvasElement, WebGlBuffer, WebGlProgram};
use webgl2::{
    blend::BlendMode, context::gl_clear_color, gl, vertex::buffer_data_f32, Program,
};

use crate::shader::{color_texture, SingleColorShaderGl1, TextureShader};

const BG_COLOR: [f32; 4] = [0.0, 0.2, 0.2, 1.0];

#[wasm_bindgen(start)]
pub fn init() -> Result<()> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

#[derive(Debug, Clone, Copy)]
#[wasm_bindgen]
pub enum GlBlendMode {
    Alpha,
    Add,
    Sub,
    Mul,
    Screen,
    Lighten,
    Darken,
}

impl From<GlBlendMode> for BlendMode {
    fn from(val: GlBlendMode) -> Self {
        match val {
            GlBlendMode::Alpha => BlendMode::Alpha,
            GlBlendMode::Add => BlendMode::Add,
            GlBlendMode::Sub => BlendMode::Sub,
            GlBlendMode::Mul => BlendMode::Mul,
            GlBlendMode::Screen => BlendMode::Screen,
            GlBlendMode::Lighten => BlendMode::Lighten,
            GlBlendMode::Darken => BlendMode::Darken,
        }
    }
}

impl GlBlendMode {
    const VARIABLES: &'static [Self] = &[
        GlBlendMode::Alpha,
        GlBlendMode::Add,
        GlBlendMode::Sub,
        GlBlendMode::Mul,
        GlBlendMode::Screen,
        GlBlendMode::Lighten,
        GlBlendMode::Darken,
    ];
}

#[wasm_bindgen]
pub fn create_blendmode_option(select_element: web_sys::HtmlSelectElement) -> Result<()> {
    for mode in GlBlendMode::VARIABLES {
        let option = web_sys::window()
            .ok_or(JsError::new("Failed to get window"))?
            .document()
            .ok_or(JsError::new("Failed to get document"))?
            .create_element("option")
            .map_err(|_| JsError::new("Failed to create option element"))?;
        option.set_text_content(Some(&format!("{:?}", mode)));
        option
            .set_attribute("value", &format!("{:?}", mode.into_abi()))
            .map_err(|_| JsError::new("Failed to set value attribute to option element"))?;
        select_element
            .append_child(&option)
            .map_err(|_| JsError::new("Failed to append option element"))?;
    }

    Ok(())
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct GlContext {
    gl: Rc<gl>,
    blend: Rc<RefCell<BlendMode>>,
}

impl GlContext {
    fn new(gl: Rc<gl>, blend: BlendMode) -> Self {
        Self {
            gl,
            blend: Rc::new(RefCell::new(blend)),
        }
    }
}

#[wasm_bindgen]
impl GlContext {
    pub fn set_blend_mode(&self, mode: GlBlendMode) {
        self.blend.replace(mode.into());
    }
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
pub fn start(canvas: HtmlCanvasElement) -> std::result::Result<GlContext, JsValue> {
    let width = 500;
    let height = 300;
    canvas.set_width(width);
    canvas.set_height(height);
    let local_mat = LocalMat::new(width as f32 / height as f32);

    let gl = webgl2::context::get_context(&canvas, BG_COLOR)?;

    let gl = Rc::new(gl);
    let s = SingleColorShaderGl1::new(gl.clone())?;
    let u = s.uniform();

    // 背景色を描画
    BlendMode::Alpha.enable(&gl);
    let v0 = s.create_vbo(&SingleColorShaderGl1::UNIT_RECT)?;
    u.set_local_mat(Matrix3::identity().append_nonuniform_scaling(&Vector2::new(1.0, 0.1)));
    u.set_color([0.0, 0.0, 1.0, 1.0]);
    s.draw(&v0);

    u.set_local_mat(local_mat.with_translation(-0.5, -0.5));
    u.set_color([1.0, 0.0, 0.0, 0.5]);
    s.draw(&v0);

    u.set_local_mat(local_mat.with_translation(0.5, 0.5));
    u.set_color([0.0, 1.0, 0.0, 0.5]);
    s.draw(&v0);

    let ctx = GlContext::new(gl.clone(), BlendMode::Alpha);
    let ctx_clone = ctx.clone();
    let mut a = AnimationLoop::new(move |t| {
        let t = t as f32 / 500.0;
        let x = t.sin() * 0.5;
        let y = t.cos() * 0.5;
        let u = s.uniform();

        // 背景色を描画。Canvasの影響を可視化するために青線をAlphaブレンドで描画
        BlendMode::Alpha.enable(&gl);
        gl_clear_color(&gl, BG_COLOR);
        u.set_local_mat(Matrix3::identity().append_nonuniform_scaling(&Vector2::new(1.0, 0.1)));
        u.set_color([0.0, 0.0, 1.0, 1.0]);
        s.draw(&v0);

        // 指定のブレンドモードで、赤と緑の矩形を描画
        ctx_clone.blend.borrow().enable(&gl);
        u.set_local_mat(local_mat.with_translation(x, y));
        u.set_color([1.0, 0.0, 0.0, x.abs() + 0.1]);
        s.draw(&v0);
        u.set_local_mat(local_mat.with_translation(-x, -y));
        u.set_color([0.0, 1.0, 0.0, y.abs() + 0.1]);
        s.draw(&v0);
        Ok(())
    });
    a.start();
    a.forget();

    Ok(ctx)
}

#[wasm_bindgen]
pub fn start_webgl2_texture(canvas: HtmlCanvasElement) -> std::result::Result<GlContext, JsValue> {
    let width = 500;
    let height = 300;
    canvas.set_width(width);
    canvas.set_height(height);
    let local_mat = LocalMat::new(width as f32 / height as f32);

    let gl = webgl2::context::get_context(&canvas, BG_COLOR)?;
    let gl = Rc::new(gl);
    let s = TextureShader::new(gl.clone())?;

    let r = SingleColorShaderGl1::UNIT_RECT;
    let vao = s.create_vao(&r)?;

    let t_r = color_texture(&gl, [255, 0, 0, 128]);
    let t_g = color_texture(&gl, [0, 255, 0, 128]);
    let t_b = color_texture(&gl, [0, 0, 255, 255]);

    let u = s.uniform();

    u.set_mat(Matrix3::identity().append_nonuniform_scaling(&Vector2::new(1.0, 0.1)));
    gl.bind_texture(gl::TEXTURE_2D, Some(&t_b));
    s.draw(&vao);

    u.set_mat(local_mat.with_translation(-0.5, -0.5));
    gl.bind_texture(gl::TEXTURE_2D, Some(&t_r));
    s.draw(&vao);

    u.set_mat(local_mat.with_translation(0.5, 0.5));
    gl.bind_texture(gl::TEXTURE_2D, Some(&t_g));
    s.draw(&vao);

    let ctx = GlContext::new(gl.clone(), BlendMode::Alpha);
    let ctx_clone = ctx.clone();

    let mut a = AnimationLoop::new(move |_| {
        let u = s.uniform();

        // 背景色を描画。Canvasの影響を可視化するために青線をAlphaブレンドで描画
        BlendMode::Alpha.enable(&gl);
        gl_clear_color(&gl, BG_COLOR);
        u.set_mat(Matrix3::identity().append_nonuniform_scaling(&Vector2::new(1.0, 0.1)));
        gl.bind_texture(gl::TEXTURE_2D, Some(&t_b));
        s.draw(&vao);

        ctx_clone.blend.borrow().enable(&gl);
        u.set_mat(local_mat.with_translation(-0.5, -0.5));
        gl.bind_texture(gl::TEXTURE_2D, Some(&t_r));
        s.draw(&vao);

        u.set_mat(local_mat.with_translation(0.5, 0.5));
        gl.bind_texture(gl::TEXTURE_2D, Some(&t_g));
        s.draw(&vao);
        Ok(())
    });
    a.start();
    a.forget();

    Ok(ctx)
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
