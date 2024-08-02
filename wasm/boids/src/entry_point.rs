use tokio::sync::mpsc;
use wasm_bindgen::prelude::*;
use wasm_utils::info;
use web_sys::HtmlCanvasElement;
use webgl2::gl;

use crate::{
    boids_shader::BoidsShaderBuilder,
    camera::{Camera, ViewMatrix},
    utils::{merge_events, Mergeable},
    ws::start_websocket,
};

const COLOR_BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

#[wasm_bindgen(start)]
pub fn init() -> Result<(), JsValue> {
    info!("execute init");
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

#[wasm_bindgen(inspectable)]
pub struct BoidsInitializeParam {
    pub boid_num: u32,
    pub boid_size: f32,
    pub history_len: usize,
    pub history_size: f32,
    pub history_alpha: f32,
}

#[wasm_bindgen]
impl BoidsInitializeParam {
    pub fn init() -> Self {
        Self {
            boid_num: 100,
            boid_size: 0.01,
            history_len: 200,
            history_size: 1.0,
            history_alpha: 0.25,
        }
    }
}

#[wasm_bindgen]
pub fn start_boids(
    canvas: HtmlCanvasElement,
    ip: BoidsInitializeParam,
) -> Result<BoidController, JsValue> {
    info!("Starting boids");
    canvas.set_width(768);
    canvas.set_height(768);

    let mut boids = crate::boids::Boids::new_circle(ip.boid_num, 0.5, 0.01);
    let mut buillder = BoidsShaderBuilder::new();

    let gl = get_webgl2_context(&canvas)?;
    let camera = Camera::default();
    let mut view = ViewMatrix::default();

    buillder.boid_size = ip.boid_size;
    buillder.history_size = ip.history_size;
    buillder.history_len = ip.history_len;
    buillder.history_color = [0.0, 0.5, 0.4, ip.history_alpha];

    let mut boids_shader = buillder.build(&gl, &boids.boids, &camera, &view)?;

    let (tx, mut rx) = mpsc::unbounded_channel();
    let (c_tx, mut c_rx) = mpsc::unbounded_channel();
    let ctrl = BoidController::new(tx, c_tx);

    let a = wasm_utils::animation::AnimationLoop::new(move |_| {
        if let Some(event) = merge_events(&mut rx) {
            for b in boids.boids.iter_mut() {
                event.apply(b);
            }
        }
        if let Some(event) = merge_events(&mut c_rx) {
            view.eye.x = event.x;
            view.eye.y = event.y;
            view.eye.z = event.z;
            boids_shader.camera.update_mvp(&gl, &camera, &view);
        }

        gl_clear_color(&gl, COLOR_BLACK);
        for (b, s) in boids.boids.iter().zip(boids_shader.boids.iter_mut()) {
            s.use_program(&gl);
            s.update(&gl, b);
            s.draw(&gl);
            let hist = s.history_mut();
            hist.use_program(&gl);
            hist.update(&gl, b);
            hist.draw(&gl);
        }
        boids.update();
        Ok(())
    });
    a.start();
    a.forget();
    // 初期値送信
    ctrl.init();

    // start ws
    start_websocket("ws://localhost:8080/api/ws/boid/gen_stream")?;
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
#[derive(Debug, Clone, Copy)]
pub struct BoidParamSetter {
    pub visual_range: Option<f32>,
    pub center_factor: Option<f32>,
    pub alignment_factor: Option<f32>,
    pub avoid_distance: Option<f32>,
    pub avoid_factor: Option<f32>,
    // WASMではタプルが使えないので、min, maxを別々に設定する
    pub speed_min: Option<f32>,
    pub speed_max: Option<f32>,
}

// 別々に発火させるためにMergeableを実装したが、Controllerで保持するようになったからいらないかも
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
        if let Some(v) = self.alignment_factor {
            p.set_alignment_factor(v);
        }
        if let Some(v) = self.avoid_distance {
            p.set_avoid_distance(v);
        }
        if let Some(v) = self.avoid_factor {
            p.set_avoid_factor(v);
        }
        if let Some(v) = self.speed_min {
            p.set_speed_min(v);
        }
        if let Some(v) = self.speed_max {
            p.set_speed_max(v);
        }
    }
}

impl Default for BoidParamSetter {
    fn default() -> Self {
        Self {
            visual_range: Some(0.16),
            center_factor: Some(0.0014),
            alignment_factor: Some(0.0224),
            avoid_distance: Some(0.045),
            avoid_factor: Some(0.017),
            speed_min: Some(0.0014),
            speed_max: Some(0.01),
        }
    }
}

/// Js側に露出して操作を受け付け、WASM側に指示を送るための構造体
#[wasm_bindgen]
pub struct BoidController {
    param_ch: mpsc::UnboundedSender<BoidParamSetter>,
    last: BoidParamSetter,
    camera_ch: mpsc::UnboundedSender<CameraParamSetter>,
    camera_last: CameraParamSetter,
}

impl BoidController {
    pub fn new(
        tx: mpsc::UnboundedSender<BoidParamSetter>,
        c_tx: mpsc::UnboundedSender<CameraParamSetter>,
    ) -> Self {
        Self {
            param_ch: tx,
            last: BoidParamSetter::default(),
            camera_ch: c_tx,
            camera_last: CameraParamSetter::DEFAULT,
        }
    }
}

#[wasm_bindgen]
impl BoidController {
    fn init(&self) {
        self.param_ch.send(self.last).unwrap();
    }

    pub fn param(&self) -> BoidParamSetter {
        self.last
    }

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

    /// 群れの進む方向に揃える力の強さを設定する
    pub fn set_alignment_factor(&mut self, alignment_factor: f32) {
        self.last.alignment_factor = Some(alignment_factor);
        self.param_ch.send(self.last).unwrap();
    }

    /// 避ける対象となる距離を設定する
    pub fn set_avoid_distance(&mut self, avoid_distance: f32) {
        self.last.avoid_distance = Some(avoid_distance);
        self.param_ch.send(self.last).unwrap();
    }

    /// 避ける力の強さを設定する
    pub fn set_avoid_factor(&mut self, avoid_factor: f32) {
        self.last.avoid_factor = Some(avoid_factor);
        self.param_ch.send(self.last).unwrap();
    }

    /// 速度の最小値を設定する
    pub fn set_speed_min(&mut self, speed_min: f32) {
        self.last.speed_min = Some(speed_min);
        self.param_ch.send(self.last).unwrap();
    }

    /// 速度の最大値を設定する
    pub fn set_speed_max(&mut self, speed_max: f32) {
        self.last.speed_max = Some(speed_max);
        self.param_ch.send(self.last).unwrap();
    }

    pub fn camera(&self) -> CameraParamSetter {
        self.camera_last
    }

    pub fn set_camera_x(&mut self, x: f32) {
        self.camera_last.x = x;
        self.camera_ch.send(self.camera_last).unwrap();
    }

    pub fn set_camera_y(&mut self, y: f32) {
        self.camera_last.y = y;
        self.camera_ch.send(self.camera_last).unwrap();
    }

    pub fn set_camera_z(&mut self, z: f32) {
        self.camera_last.z = z;
        self.camera_ch.send(self.camera_last).unwrap();
    }

    pub fn reset_camera_position(&mut self) {
        self.camera_ch.send(CameraParamSetter::DEFAULT).unwrap();
    }
}

#[wasm_bindgen(inspectable)]
#[derive(Debug, Clone, Copy)]
pub struct CameraParamSetter {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl CameraParamSetter {
    const DEFAULT: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 3.0,
    };
}

impl Mergeable for CameraParamSetter {
    fn merge(&mut self, other: Self) {
        self.x = other.x;
        self.y = other.y;
        self.z = other.z;
    }
}
