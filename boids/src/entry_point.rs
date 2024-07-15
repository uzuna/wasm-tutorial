use tokio::sync::mpsc;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use webgl2::gl;

use crate::{
    animation,
    boids_shader::BoidShader,
    camera::{Camera, ViewMatrix},
    info,
    utils::{merge_events, Mergeable},
};

const COLOR_BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

#[wasm_bindgen(start)]
pub fn init() -> Result<(), JsValue> {
    info!("execute init");
    crate::utils::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
pub fn start_boids(canvas: HtmlCanvasElement) -> Result<BoidController, JsValue> {
    info!("Starting boids");
    canvas.set_width(768);
    canvas.set_height(768);

    let mut boids = crate::boids::Boids::new_circle(180, 0.5, 0.01);

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

    let (tx, mut rx) = mpsc::unbounded_channel();
    let ctrl = BoidController::new(tx);

    let a = animation::AnimationLoop::new(move |_| {
        if let Some(event) = merge_events(&mut rx) {
            for b in boids.boids.iter_mut() {
                event.apply(b);
            }
        }

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

    Ok(ctrl)
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

/// Boidのパラメータを設定するための構造体
///
/// 既定値で動作しているので、必要な値だけ設定して渡すことができる。
#[wasm_bindgen(inspectable)]
#[derive(Debug, Clone, Copy, Default)]
pub struct BoidParamSetter {
    pub visual_range: Option<f32>,
    pub center_factor: Option<f32>,
}

impl Mergeable for BoidParamSetter {
    fn merge(&mut self, other: Self) {
        if let Some(v) = other.visual_range {
            self.visual_range = Some(v);
        }
        if let Some(v) = other.center_factor {
            self.center_factor = Some(v);
        }
    }
}

impl BoidParamSetter {
    fn apply(&self, b: &mut crate::boids::Boid) {
        let p = b.get_param_mut();
        if let Some(v) = self.visual_range {
            p.set_visual_range(v);
        }
        if let Some(v) = self.center_factor {
            p.set_center_factor(v);
        }
    }
}

#[wasm_bindgen]
pub struct BoidController {
    param_ch: mpsc::UnboundedSender<BoidParamSetter>,
    last: BoidParamSetter,
}

impl BoidController {
    pub fn new(tx: mpsc::UnboundedSender<BoidParamSetter>) -> Self {
        Self {
            param_ch: tx,
            last: BoidParamSetter::default(),
        }
    }
}

#[wasm_bindgen]
impl BoidController {
    /// boidsが周辺の個体を群れとして扱う範囲を設定する
    pub fn set_visual_range(&mut self, visual_range: f32) {
        self.last.visual_range = Some(visual_range);
        self.param_ch.send(self.last).unwrap();
    }

    /// 群れの中心に向かう力の強さを設定する
    pub fn set_center_factor(&mut self, center_factor: f32) {
        self.last.center_factor = Some(center_factor);
        self.param_ch.send(self.last).unwrap();
    }
}
