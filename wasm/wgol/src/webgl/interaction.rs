use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::{WebGlFramebuffer, WebGlTexture, WebGlUniformLocation};

use webgl2::{
    context::Context,
    gl,
    program::Program,
    vertex::{Vao, VaoDefine},
    GlEnum, GlPoint2d, GlPoint3d,
};

use crate::error::Result;

#[derive(Debug, PartialEq)]
pub enum ParticleVd {
    Position,
}

impl VaoDefine for ParticleVd {
    fn iter() -> std::slice::Iter<'static, Self> {
        [ParticleVd::Position].iter()
    }

    fn name(&self) -> &'static str {
        match self {
            ParticleVd::Position => "position",
        }
    }

    fn size_of(&self) -> i32 {
        2
    }
}

pub struct ParticleShader {
    program: Program,
    particle: Particle,
    vao: Vao<ParticleVd>,
    vertex_len: i32,
    uniform: ParticleUniform,
}

impl ParticleShader {
    /// reference: https://wgld.org/d/webgl/w082.html
    const VERT: &'static str = r#"#version 300 es

layout(location = 0) in vec2 position;

uniform float pointSize;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    gl_PointSize = pointSize;
}
"#;

    const FRAG: &'static str = r#"#version 300 es

precision mediump float;

uniform vec4 pointColor;
out vec4 fragmentColor;

void main() {
    fragmentColor = pointColor;
}
"#;

    pub fn new(ctx: &Context, res: Resolution, ctrl: ParticleControl) -> Result<Self> {
        let program = ctx.program(Self::VERT, Self::FRAG)?;
        let particle = Particle::new(res, ctrl);
        let mut vao = program.create_vao()?;
        vao.buffer_data(ParticleVd::Position, &particle.position, gl::DYNAMIC_DRAW);
        let vertex_len = particle.position.len() as i32;

        program.use_program();
        let uniform = ParticleUniform::new(&program)?;
        uniform.init();
        Ok(Self {
            program,
            particle,
            vao,
            vertex_len,
            uniform,
        })
    }

    pub fn set_color(&self, color: [f32; 4]) {
        self.uniform.set_color(color);
    }

    pub fn update(&mut self, target: Point, vector_update: bool) {
        self.particle.update(target, vector_update);
        self.vao
            .buffer_sub_data(ParticleVd::Position, &self.particle.position, 0);
        self.uniform.set_size(self.particle.current_size);
    }

    pub fn draw(&self) {
        self.program.use_program();
        self.vao.bind();
        self.program
            .gl()
            .draw_arrays(gl::POINTS, 0, self.vertex_len);
    }
}

struct ParticleUniform {
    gl: Rc<gl>,
    size: WebGlUniformLocation,
    color: WebGlUniformLocation,
}

impl ParticleUniform {
    pub fn new(program: &Program) -> Result<Self> {
        let size = program.uniform_location("pointSize")?;
        let color = program.uniform_location("pointColor")?;
        let gl = program.gl().clone();
        Ok(Self { gl, size, color })
    }

    pub fn init(&self) {
        self.set_size(1.0);
        self.set_color([1.0, 1.0, 1.0, 1.0]);
    }

    pub fn set_size(&self, size: f32) {
        self.gl.uniform1f(Some(&self.size), size);
    }

    pub fn set_color(&self, color: [f32; 4]) {
        self.gl
            .uniform4f(Some(&self.color), color[0], color[1], color[2], color[3]);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Resolution {
    pub x: u32,
    pub y: u32,
}

impl Resolution {
    pub const DEFAULT: Self = Self { x: 64, y: 64 };

    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<Point> for GlPoint2d {
    fn from(p: Point) -> GlPoint2d {
        GlPoint2d::new(p.x, p.y)
    }
}

impl std::ops::Sub for Point {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Div for Point {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl std::ops::Div<f32> for Point {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl std::ops::Mul<f32> for Point {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

/// パーティクルに関する操作
#[wasm_bindgen(inspectable)]
#[derive(Debug, Clone, Copy)]
pub struct ParticleControl {
    // 追従移動速度の係数。小さいと実際の移動量が小さくなり、速度も遅いがマウスに追従しやすくなる
    // 大きくするとオーバーシュートが増える
    pub speed: f32,
    // mouseup時のパーティクル移動速度の減衰係数
    // 大きいほどすぐに止まる
    pub speed_decay: f32,
    // パーティクルの大きさの減衰係数
    // 大きいほどすぐに見えない大きさに鳴る
    pub size_decay: f32,
    // 最大速度係数。speedとほぼ同じだが、こちらは減速速度にも影響する
    pub max_velocity: f32,
    // 最大パーティクルサイズ
    pub max_size: f32,
    // ベクトル更新レートの逆数
    // これが大きいほど、パーティクルの方向転換が遅くなる = オーバーシュートしやすくなる
    pub handle_rate: f32,
}

impl ParticleControl {
    pub const DEFAULT: Self = Self {
        speed: 0.02,
        speed_decay: 0.95,
        size_decay: 0.98,
        max_velocity: 2.0,
        max_size: 4.0,
        handle_rate: 1.0 / 5.0,
    };
}

#[wasm_bindgen]
impl ParticleControl {
    pub fn default() -> Self {
        Self::DEFAULT
    }
}

pub struct Particle {
    position: Vec<GlPoint2d>,
    vector: Vec<GlPoint2d>,
    res: Resolution,
    current_velocity: f32,
    current_size: f32,
    ctrl: ParticleControl,
}

impl Particle {
    pub fn new(res: Resolution, ctrl: ParticleControl) -> Self {
        let mut position = Vec::new();
        let mut vector = Vec::new();
        // OpenGL空間を指定解像度で分割して点を配置
        let (ix, iy) = (1. / res.x as f32, 1. / res.y as f32);
        for y in 0..res.y {
            for x in 0..res.x {
                position.push(GlPoint2d::new(
                    x as f32 * ix * 2.0 - 1.0,
                    y as f32 * iy * 2.0 - 1.0,
                ));
                vector.push(GlPoint2d::new(0.0, 0.0));
            }
        }
        Self {
            position,
            vector,
            res,
            current_velocity: 0.0,
            current_size: 1.0,
            ctrl,
        }
    }

    fn index(&self, x: u32, y: u32) -> usize {
        y as usize * self.res.x as usize + x as usize
    }

    // 移動ベクトルの更新
    fn update_vector(&self, pos: GlPoint2d, target: Point, vector: GlPoint2d) -> GlPoint2d {
        let mut delta = GlPoint2d::from(target) - pos;
        // ベクトルに対する加算量を計算
        let r = delta.norm() / self.ctrl.handle_rate;
        if r != 0.0 {
            delta /= r;
        }
        delta += vector;
        // ベクトルの長さが1.0に収束するように正規化
        let r = delta.norm();
        if r != 0.0 {
            delta /= r;
        }
        delta
    }

    // 目標点に向かって移動
    pub fn update(&mut self, target: Point, vector_update: bool) {
        match vector_update {
            true => {
                self.current_velocity = self.ctrl.max_velocity;
                self.current_size = self.ctrl.max_size
            }
            false => {
                self.current_velocity *= self.ctrl.speed_decay;
                self.current_size *= self.ctrl.size_decay;
            }
        }
        for x in 0..self.res.x {
            for y in 0..self.res.y {
                let i = self.index(x, y);
                if vector_update {
                    self.vector[i] = self.update_vector(self.position[i], target, self.vector[i]);
                }
                self.position[i] += self.vector[i] * self.current_velocity * self.ctrl.speed;
            }
        }
    }
}

#[derive(Debug, PartialEq)]

pub enum IndexVd {
    Position,
}

impl VaoDefine for IndexVd {
    fn iter() -> std::slice::Iter<'static, Self> {
        [IndexVd::Position].iter()
    }

    fn name(&self) -> &'static str {
        match self {
            IndexVd::Position => "position",
        }
    }

    fn size_of(&self) -> i32 {
        3
    }
}

pub struct ParticleGpgpuShader {
    res: Resolution,
    point: Program,
    velocity: Program,
    index: Program,
    u_point: ParticleGpgpuPointUniform,
    u_velocity: ParticleGpgpuVelocityUniform,
    _u_index: ParticleGpgpuIndexUniform,
    point_vao: Vao<ParticleVd>,
    point_vlen: i32,
    index_vao: Vao<IndexVd>,
    fbos: [TextureFBO; 2],
    fbo_prev_index: usize,
    state: ParticleGpgpuState,
}

impl ParticleGpgpuShader {
    // 頂点の位置を保持するシェーダー。テクスチャにある頂点情報を取り出してそのまま出力
    const POINT_VERT: &'static str = r#"#version 300 es
layout(location = 0) in vec2 position;
uniform sampler2D u_texture;
uniform float pointSize;

void main(){
    vec4 t = texture(u_texture, position);
    gl_Position = vec4(t.xy, 0.0, 1.0);
    gl_PointSize = pointSize;
}
"#;
    // 頂点の色はuniformから指定
    const POINT_FRAG: &'static str = r#"#version 300 es
precision mediump float;
uniform vec4 ambient;
out vec4 fragmentColor;
void main(){
	fragmentColor = ambient;
}
"#;

    // 何が入っている?
    const VELOCITY_VERT: &'static str = r#"#version 300 es
layout(location = 0) in vec3 position;
void main(){
    gl_Position = vec4(position, 1.0);
}
"#;
    // テクスチャから現在のVelocityを取り出して更新するロジック
    const VELOCITY_FRAG: &'static str = r#"#version 300 es
precision mediump float;

uniform vec2 resolution;
uniform sampler2D u_texture;
uniform vec2 target;
uniform bool vectorUpdate;
uniform float velocity;
uniform float speed;
uniform float handleRate;

out vec4 fragmentColor;
void main(){
    vec2 p = gl_FragCoord.xy / resolution;
    vec4 t = texture(u_texture, p);
    vec2 v = normalize(target - t.xy) * handleRate;
    vec2 w = normalize(v + t.zw);
    vec4 destColor = vec4(t.xy + w * speed * velocity, w);
    if(!vectorUpdate){destColor.zw = t.zw;}
    fragmentColor = destColor;
}
"#;

    // 初期状態を作るシェーダープログラム
    const VERT: &'static str = r#"#version 300 es
layout(location = 0) in vec3 position;
void main(){
    gl_Position = vec4(position, 1.0);
}
"#;

    const FRAG: &'static str = r#"#version 300 es
precision mediump float;
uniform vec2 resolution;
out vec4 fragmentColor;
void main(){
    vec2 p = (gl_FragCoord.xy / resolution) * 2.0 - 1.0;
    // RGは位置情報、BAは速度情報
    // 初期位置は位置をもとに。速度は0
    fragmentColor = vec4(p, 0.0, 0.0);
}
"#;

    // 画面全体を覆うポリゴンの頂点情報
    const TEXTURE_VERTEX: [GlPoint3d; 4] = [
        GlPoint3d::new(-1.0, 1.0, 0.0),
        GlPoint3d::new(-1.0, -1.0, 0.0),
        GlPoint3d::new(1.0, 1.0, 0.0),
        GlPoint3d::new(1.0, -1.0, 0.0),
    ];

    pub fn new(ctx: &Context, res: Resolution, ctrl: ParticleControl) -> Result<Self> {
        let point = ctx.program(Self::POINT_VERT, Self::POINT_FRAG)?;
        let velocity = ctx.program(Self::VELOCITY_VERT, Self::VELOCITY_FRAG)?;
        let index_map = ctx.program(Self::VERT, Self::FRAG)?;

        let state = ParticleGpgpuState::new(ctrl);

        point.use_program();
        let u_point = ParticleGpgpuPointUniform::new(&point)?;
        u_point.init(&state);

        velocity.use_program();
        let u_velocity = ParticleGpgpuVelocityUniform::new(&velocity)?;
        u_velocity.init(&res, &state);

        index_map.use_program();
        let u_index = ParticleGpgpuIndexUniform::new(&index_map)?;
        u_index.init(&res);

        // 最終描画する頂点情報を作成
        // この頂点はデータのサンプリングの位置を示すだけなので更新することはない
        let point_vert = Self::point_vert(res.x, res.y);
        let mut point_vao = point.create_vao()?;
        point_vao.buffer_data(ParticleVd::Position, &point_vert, gl::STATIC_DRAW);

        let mut index_vao = index_map.create_vao()?;
        index_vao.buffer_data(IndexVd::Position, &Self::TEXTURE_VERTEX, gl::STATIC_DRAW);

        let gl = ctx.gl();
        // 位置と速度の情報は2つのバッファを使って交互に更新する
        let fbos = [
            TextureFBO::new_float_vec4(gl.clone(), res)?,
            TextureFBO::new_float_vec4(gl.clone(), res)?,
        ];
        index_vao.unbind();

        let s = Self {
            res,
            point,
            velocity,
            index: index_map,
            u_point,
            u_velocity,
            _u_index: u_index,
            point_vao,
            point_vlen: point_vert.len() as i32,
            index_vao,
            fbos,
            fbo_prev_index: 0,
            state,
        };
        s.draw_init();

        Ok(s)
    }

    // 取り出すテクスチャ座標の位置
    fn point_vert(x: u32, y: u32) -> Vec<GlPoint2d> {
        let (ix, iy) = (1. / x as f32, 1. / y as f32);
        let mut position = Vec::new();
        for y in 0..y {
            for x in 0..x {
                position.push(GlPoint2d::new(x as f32 * ix, y as f32 * iy));
            }
        }
        position
    }

    fn next_fbo_index(&self) -> usize {
        (self.fbo_prev_index + 1) % 2
    }

    // 0番目のFBOに初期状態を書き込む
    fn draw_init(&self) {
        let gl = self.point.gl();
        gl.disable(gl::BLEND);
        gl.blend_func(gl::ONE, gl::ONE);
        self.fbos[0].bind();
        gl.viewport(0, 0, self.res.x as i32, self.res.y as i32);
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear(gl::COLOR_BUFFER_BIT);
        self.index.use_program();
        self.index_vao.bind();
        gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
        self.fbos[0].unbind();
    }

    /// 画面全体にインデックスを描画。インデックス確認用
    #[allow(dead_code)]
    pub fn draw_index(&self, target_res: &Resolution) {
        let gl = self.point.gl();
        gl.disable(gl::BLEND);
        gl.blend_func(gl::ONE, gl::ONE);
        gl.viewport(0, 0, target_res.x as i32, target_res.y as i32);
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(gl::COLOR_BUFFER_BIT);
        self.index.use_program();
        self.index_vao.bind();
        gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
    }

    pub fn update(&mut self, target: Point, vector_update: bool, color: [f32; 4]) {
        self.state.update(target, vector_update);
        self.state.ambient = color;

        // 移動制御uniformを更新
        self.velocity.use_program();
        self.u_velocity.set_target(self.state.target);
        self.u_velocity.set_velocity(self.state.velocity);
        self.u_velocity.set_vector_update(self.state.vector_update);

        // 描画uniformを更新
        self.point.use_program();
        self.point_vao.bind();
        self.u_point.set_ambient(self.state.ambient);
        self.u_point.set_point_size(self.state.size);
    }

    pub fn draw(&mut self, target_res: &Resolution) {
        let gl = self.point.gl();
        // FBOは交互に使うので、インデックスを切り替える
        let next = self.next_fbo_index();
        let fbos = [&self.fbos[self.fbo_prev_index], &self.fbos[next]];
        // ブレンドは無効化
        gl.disable(gl::BLEND);
        self.index_vao.bind();

        // 次のFBOに位置と速度を書き込む
        fbos[1].bind();
        gl.viewport(0, 0, self.res.x as i32, self.res.y as i32);
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear(gl::COLOR_BUFFER_BIT);

        self.velocity.use_program();
        gl.active_texture(gl::TEXTURE0);
        // 前のFBOの状態をテクスチャの仕組みで取得
        gl.bind_texture(gl::TEXTURE_2D, Some(&fbos[0].texture));
        gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
        fbos[0].unbind();

        // FBOをもとに描画
        gl.viewport(0, 0, target_res.x as i32, target_res.y as i32);
        gl.enable(gl::BLEND);
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(gl::COLOR_BUFFER_BIT);

        // 上で描画したテクスチャをバインド
        gl.bind_texture(gl::TEXTURE_2D, Some(&fbos[1].texture));

        // Debug: パラメータをそのまま描画
        // gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);

        // ポイントで描画
        self.point.use_program();
        self.point_vao.bind();
        gl.draw_arrays(gl::POINTS, 0, self.point_vlen);
        self.point_vao.unbind();

        gl.flush();

        // 次のフレームのためにインデックスを更新
        self.fbo_prev_index = next;
    }
}

struct ParticleGpgpuState {
    ctrl: ParticleControl,
    velocity: f32,
    size: f32,
    vector_update: bool,
    ambient: [f32; 4],
    target: Point,
}

impl ParticleGpgpuState {
    fn new(ctrl: ParticleControl) -> Self {
        Self {
            ctrl,
            velocity: 0.0,
            size: 0.0,
            vector_update: false,
            ambient: [1.0, 1.0, 1.0, 1.0],
            target: Point::new(0.0, 0.0),
        }
    }

    fn update(&mut self, target: Point, vector_update: bool) {
        self.vector_update = vector_update;
        self.target = target;
        match vector_update {
            true => {
                self.velocity = self.ctrl.max_velocity;
                self.size = self.ctrl.max_size
            }
            false => {
                self.velocity *= self.ctrl.speed_decay;
                self.size *= self.ctrl.size_decay;
            }
        }
    }
}

struct ParticleGpgpuPointUniform {
    gl: Rc<gl>,
    point_size: WebGlUniformLocation,
    u_texture: WebGlUniformLocation,
    ambient: WebGlUniformLocation,
}

impl ParticleGpgpuPointUniform {
    pub fn new(program: &Program) -> Result<Self> {
        let point_size = program.uniform_location("pointSize")?;
        let u_texture = program.uniform_location("u_texture")?;
        let ambient = program.uniform_location("ambient")?;
        let gl = program.gl().clone();
        Ok(Self {
            gl,
            point_size,
            u_texture,
            ambient,
        })
    }

    fn init(&self, state: &ParticleGpgpuState) {
        self.set_ambient(state.ambient);
        self.set_point_size(20.0)
    }
    #[allow(dead_code)]
    pub fn set_texture_unit(&self, texture_unit: i32) {
        self.gl.uniform1i(Some(&self.u_texture), texture_unit);
    }

    pub fn set_ambient(&self, color: [f32; 4]) {
        self.gl
            .uniform4f(Some(&self.ambient), color[0], color[1], color[2], color[3]);
    }

    pub fn set_point_size(&self, size: f32) {
        self.gl.uniform1f(Some(&self.point_size), size);
    }
}

struct ParticleGpgpuVelocityUniform {
    gl: Rc<gl>,
    resolution: WebGlUniformLocation,
    u_texture: WebGlUniformLocation,
    target: WebGlUniformLocation,
    vector_update: WebGlUniformLocation,
    velocity: WebGlUniformLocation,
    speed: WebGlUniformLocation,
    handle_rate: WebGlUniformLocation,
}

impl ParticleGpgpuVelocityUniform {
    pub fn new(program: &Program) -> Result<Self> {
        let resolution = program.uniform_location("resolution")?;
        let u_texture = program.uniform_location("u_texture")?;
        let target = program.uniform_location("target")?;
        let vector_update = program.uniform_location("vectorUpdate")?;
        let velocity = program.uniform_location("velocity")?;
        let speed = program.uniform_location("speed")?;
        let handle_rate = program.uniform_location("handleRate")?;
        let gl = program.gl().clone();
        Ok(Self {
            gl,
            resolution,
            u_texture,
            target,
            vector_update,
            velocity,
            speed,
            handle_rate,
        })
    }

    fn init(&self, res: &Resolution, state: &ParticleGpgpuState) {
        self.set_resolution(res);
        self.set_target(state.target);
        self.set_vector_update(state.vector_update);
        self.set_velocity(state.velocity);
        self.set_speed(state.ctrl.speed);
        self.set_handle_rate(state.ctrl.handle_rate);
    }

    #[allow(dead_code)]
    pub fn set_texture_unit(&self, texture_unit: i32) {
        self.gl.uniform1i(Some(&self.u_texture), texture_unit);
    }

    pub fn set_resolution(&self, res: &Resolution) {
        self.gl
            .uniform2f(Some(&self.resolution), res.x as f32, res.y as f32);
    }

    pub fn set_target(&self, target: Point) {
        self.gl.uniform2f(Some(&self.target), target.x, target.y);
    }

    pub fn set_vector_update(&self, update: bool) {
        self.gl.uniform1i(Some(&self.vector_update), update as i32);
    }

    pub fn set_velocity(&self, velocity: f32) {
        self.gl.uniform1f(Some(&self.velocity), velocity);
    }

    pub fn set_speed(&self, speed: f32) {
        self.gl.uniform1f(Some(&self.speed), speed);
    }

    pub fn set_handle_rate(&self, rate: f32) {
        self.gl.uniform1f(Some(&self.handle_rate), rate);
    }
}

struct ParticleGpgpuIndexUniform {
    gl: Rc<gl>,
    resolution: WebGlUniformLocation,
}

impl ParticleGpgpuIndexUniform {
    pub fn new(program: &Program) -> Result<Self> {
        let resolution = program.uniform_location("resolution")?;
        let gl = program.gl().clone();
        Ok(Self { gl, resolution })
    }

    fn init(&self, res: &Resolution) {
        self.set_resolution(res);
    }

    pub fn set_resolution(&self, res: &Resolution) {
        self.gl
            .uniform2f(Some(&self.resolution), res.x as f32, res.y as f32);
    }
}

struct TextureFBO {
    gl: Rc<gl>,
    fbo: WebGlFramebuffer,
    texture: WebGlTexture,
}
impl TextureFBO {
    #[inline]
    #[allow(dead_code)]
    fn new_rgba(gl: Rc<gl>, res: Resolution) -> Result<Self> {
        Self::new_inner(gl, res, gl::RGBA, gl::RGBA, gl::UNSIGNED_BYTE)
    }

    #[inline]
    #[allow(dead_code)]
    fn new_half_float(gl: Rc<gl>, res: Resolution) -> Result<Self> {
        Self::new_inner(gl, res, gl::R16F, gl::RED, gl::FLOAT)
    }

    #[inline]
    #[allow(dead_code)]
    fn new_float_vec2(gl: Rc<gl>, res: Resolution) -> Result<Self> {
        Self::new_inner(gl, res, gl::RG32F, gl::RG, gl::FLOAT)
    }

    #[inline]
    #[allow(dead_code)]
    fn new_float_vec4(gl: Rc<gl>, res: Resolution) -> Result<Self> {
        Self::new_inner(gl, res, gl::RGBA32F, gl::RGBA, gl::FLOAT)
    }

    fn new_inner(
        gl: Rc<gl>,
        res: Resolution,
        internal_format: GlEnum,
        src_format: GlEnum,
        type_: GlEnum,
    ) -> Result<Self> {
        // フレームバッファにテクスチャ用の領域を確保
        let texture = gl
            .create_texture()
            .ok_or(JsError::new("Failed to create texture"))?;
        gl.bind_texture(gl::TEXTURE_2D, Some(&texture));
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            gl::TEXTURE_2D,
            0,
            internal_format as i32,
            res.x as i32,
            res.y as i32,
            0,
            src_format,
            type_,
            None,
        )
        .map_err(|e| JsError::new(&format!("Failed to tex_image_2d: {:?}", e)))?;

        gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

        let fbo = gl
            .create_framebuffer()
            .ok_or(JsError::new("Failed to create framebuffer"))?;
        gl.bind_framebuffer(gl::FRAMEBUFFER, Some(&fbo));

        // フレームバッファにテクスチャをアタッチ
        gl.framebuffer_texture_2d(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            Some(&texture),
            0,
        );

        // フレームバッファの状態を確認
        if gl.check_framebuffer_status(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
            return Err(JsError::new(&format!(
                "Framebuffer is not complete. code={}",
                gl.get_error()
            )));
        }

        // バインド解除
        gl.bind_texture(gl::TEXTURE_2D, None);
        gl.bind_framebuffer(gl::FRAMEBUFFER, None);

        Ok(Self { gl, fbo, texture })
    }

    fn bind(&self) {
        self.gl.bind_framebuffer(gl::FRAMEBUFFER, Some(&self.fbo));
    }

    fn unbind(&self) {
        self.gl.bind_framebuffer(gl::FRAMEBUFFER, None);
    }
}
