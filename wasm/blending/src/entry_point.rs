use std::{cell::RefCell, rc::Rc};

use nalgebra::{Matrix3, Vector2};
use wasm_bindgen::{convert::IntoWasmAbi, prelude::*};
use wasm_utils::{animation::AnimationLoop, error::*};
use web_sys::HtmlCanvasElement;
use webgl2::{blend::BlendMode, context::gl_clear_color};

use crate::shader::SingleColorShaderGl1;

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
    blend: Rc<RefCell<BlendMode>>,
}

impl GlContext {
    fn new(blend: BlendMode) -> Self {
        Self {
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

    let ctx = webgl2::context::Context::new(canvas, BG_COLOR)?;
    let gl = ctx.gl().clone();
    let s = SingleColorShaderGl1::new(&ctx)?;
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

    let ctx = GlContext::new(BlendMode::Alpha);
    let ctx_clone = ctx.clone();
    let mut a = AnimationLoop::new(move |time| {
        let t = time as f32 / 500.0;
        let x = t.sin() * 0.5;
        let y = t.cos() * 0.5;
        let gt = time as f32 / 700.0;
        let gx = gt.sin() * 0.5;
        let gs = gt.cos() * 0.1;
        let u = s.uniform();

        let global_mat = Matrix3::identity()
            .append_translation(&Vector2::new(0.0, gx))
            .append_scaling(gs + 0.8);

        // 背景色を描画。Canvasの影響を可視化するために青線をAlphaブレンドで描画
        BlendMode::Alpha.enable(&gl);
        gl_clear_color(&gl, BG_COLOR);
        u.set_local_mat(Matrix3::identity().append_nonuniform_scaling(&Vector2::new(1.0, 0.1)));
        u.set_global_mat(global_mat);
        u.set_color([0.0, 0.0, 1.0, 1.0]);
        s.draw(&v0);

        // 指定のブレンドモードで、赤と緑の矩形を描画
        ctx_clone.blend.borrow().enable(&gl);
        u.set_local_mat(local_mat.with_translation(x, y));
        u.set_global_mat(global_mat);
        u.set_color([1.0, 0.0, 0.0, x.abs() + 0.1]);
        s.draw(&v0);
        u.set_local_mat(local_mat.with_translation(-x, -y));
        u.set_global_mat(global_mat);
        u.set_color([0.0, 1.0, 0.0, y.abs() + 0.1]);
        s.draw(&v0);
        Ok(())
    });
    a.start();
    a.forget();

    Ok(ctx)
}
