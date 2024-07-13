use wasm_bindgen::prelude::*;
use web_sys::{WebGlBuffer, WebGlUniformLocation};

use super::program::{gl, GlEnum, GlPoint, GlPoint2D, Program};

use crate::error::{Error, Result};

pub struct ParticleShader {
    program: Program,
    particle: Particle,
    vbo: VertexVbo,
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

    pub fn new(gl: &gl, res: Resolution, ctrl: ParticleControl) -> Result<Self> {
        let program = Program::new(gl, Self::VERT, Self::FRAG)?;
        let particle = Particle::new(res, ctrl);
        let vbo = VertexVbo::new(gl, &particle.position, 0)?;
        program.use_program(gl);
        let uniform = ParticleUniform::new(gl, &program)?;
        uniform.init(gl);
        Ok(Self {
            program,
            particle,
            vbo,
            uniform,
        })
    }

    pub fn set_color(&self, gl: &gl, color: [f32; 4]) {
        self.uniform.set_color(gl, color);
    }

    pub fn update(&mut self, gl: &gl, target: Point, vector_update: bool) {
        self.particle.update(target, vector_update);
        self.vbo.update_vertex(gl, &self.particle.position);
        self.uniform.set_size(gl, self.particle.current_size);
    }

    pub fn draw(&self, gl: &gl) {
        self.program.use_program(gl);
        gl.draw_arrays(gl::POINTS, 0, self.particle.position.len() as i32);
    }
}

struct ParticleUniform {
    size: WebGlUniformLocation,
    color: WebGlUniformLocation,
}

impl ParticleUniform {
    pub fn new(gl: &gl, program: &Program) -> Result<Self> {
        let size = gl
            .get_uniform_location(program.program(), "pointSize")
            .ok_or(Error::gl("Failed to get uniform location".into()))?;
        let color = gl
            .get_uniform_location(program.program(), "pointColor")
            .ok_or(Error::gl("Failed to get uniform location".into()))?;
        Ok(Self { size, color })
    }

    pub fn init(&self, gl: &gl) {
        gl.uniform1f(Some(&self.size), 1.0);
        gl.uniform4f(Some(&self.color), 1.0, 1.0, 1.0, 1.0);
    }

    pub fn set_size(&self, gl: &gl, size: f32) {
        gl.uniform1f(Some(&self.size), size);
    }

    pub fn set_color(&self, gl: &gl, color: [f32; 4]) {
        gl.uniform4f(Some(&self.color), color[0], color[1], color[2], color[3]);
    }
}

pub struct Resolution {
    pub x: u32,
    pub y: u32,
}

impl Resolution {
    pub const DEFAULT: Self = Self { x: 64, y: 64 };
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

impl From<Point> for GlPoint2D {
    fn from(p: Point) -> GlPoint2D {
        GlPoint2D::new(p.x, p.y)
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
        handle_rate: 5.0,
    };
}

#[wasm_bindgen]
impl ParticleControl {
    pub fn default() -> Self {
        Self::DEFAULT
    }
}

pub struct Particle {
    position: Vec<GlPoint2D>,
    vector: Vec<GlPoint2D>,
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
                position.push(GlPoint2D::new(
                    x as f32 * ix * 2.0 - 1.0,
                    y as f32 * iy * 2.0 - 1.0,
                ));
                vector.push(GlPoint2D::new(0.0, 0.0));
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
    fn update_vector(&self, pos: GlPoint2D, target: Point, vector: GlPoint2D) -> GlPoint2D {
        let mut delta = GlPoint2D::from(target) - pos;
        // ベクトルに対する加算量を計算
        let r = delta.norm() * self.ctrl.handle_rate;
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

pub struct VertexVbo {
    vbo: WebGlBuffer,
    location: u32,
}

impl VertexVbo {
    const TARGET: GlEnum = gl::ARRAY_BUFFER;
    pub fn new(gl: &gl, data: &[GlPoint2D], location: u32) -> Result<Self> {
        let vbo = Self::create_vertex_buffer(
            gl,
            unsafe {
                std::slice::from_raw_parts(
                    data.as_ptr() as *const f32,
                    data.len() * GlPoint2D::size() as usize,
                )
            },
            location,
            gl::DYNAMIC_DRAW,
        )?;

        Ok(Self { vbo, location })
    }

    fn create_vertex_buffer(
        gl: &gl,
        data: &[f32],
        location: u32,
        usage: GlEnum,
    ) -> Result<WebGlBuffer> {
        let buffer = gl
            .create_buffer()
            .ok_or(Error::gl("Failed to create buffer object".into()))?;
        gl.bind_buffer(Self::TARGET, Some(&buffer));
        unsafe {
            let view = js_sys::Float32Array::view(data);
            gl.buffer_data_with_array_buffer_view(Self::TARGET, &view, usage);
        }
        gl.enable_vertex_attrib_array(location);
        gl.vertex_attrib_pointer_with_i32(location, GlPoint2D::size(), gl::FLOAT, false, 0, 0);

        // GLES2.0と違ってVAOにつなぐのでunbing不要
        Ok(buffer)
    }

    // VBOの更新
    pub fn update_vertex(&self, gl: &gl, data: &[GlPoint2D]) {
        let data = unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const f32,
                data.len() * GlPoint2D::size() as usize,
            )
        };
        gl.bind_buffer(Self::TARGET, Some(&self.vbo));
        unsafe {
            let view = js_sys::Float32Array::view(data);
            gl.buffer_sub_data_with_i32_and_array_buffer_view(Self::TARGET, 0, &view);
        }
        gl.enable_vertex_attrib_array(self.location);
        gl.vertex_attrib_pointer_with_i32(self.location, GlPoint2D::size(), gl::FLOAT, false, 0, 0);
    }
}

pub struct ParticleGpgpuShader {
    point: Program,
    velocity: Program,
    program: Program,
}

impl ParticleGpgpuShader {
    // 頂点の位置を保持するシェーダー。テクスチャにある頂点情報を取り出してそのまま出力
    const POINT_VERT: &'static str = r#"#version 300 es
layout(location = 0) in float index;
uniform vec2 resolution;
uniform sampler2D u_texture;
uniform float pointScale;

void main(){
    // index値から頂点データの位置を算出
    vec2 p = vec2(
        mod(index, resolution.x) / resolution.x,
        floor(index / resolution.x) / resolution.y
    );
    vec4 t = texture(u_texture, p);
    gl_Position = vec4(t.xy, 0.0, 1.0);
    gl_PointSize = 0.1 + pointScale;
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
    fragmentColor = vec4(p, 0.0, 0.0);
}
"#;

    pub fn new(gl: &gl, res: Resolution, ctrl: ParticleControl) -> Result<Self> {
        let point = Program::new(gl, Self::POINT_VERT, Self::POINT_FRAG)?;
        let velocity = Program::new(gl, Self::VELOCITY_VERT, Self::VELOCITY_FRAG)?;
        let program = Program::new(gl, Self::VERT, Self::FRAG)?;
        Ok(Self {
            point,
            velocity,
            program,
        })
    }
}
