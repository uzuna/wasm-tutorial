use wasm_utils::error::*;
use web_sys::WebGlUniformLocation;
use webgl2::{
    camera::{Camera, CameraUbo, ViewMatrix},
    context::Context,
    gl,
    program::Program,
    vertex::{Vao, VaoDefine},
    GlPoint3d,
};

use crate::boids::Boid;

pub struct BoidsShaderBuilder {
    /// ボイドの描画サイズ(Gl空間サイズ)
    pub boid_size: f32,
    /// ボイドの色
    pub color: [f32; 4],
    /// ボイド履歴の色
    pub history_color: [f32; 4],
    /// ボイド履歴の描画ポイントサイズ
    pub history_size: f32,
    /// ボイドの履歴を残す数
    pub history_len: usize,
}

impl BoidsShaderBuilder {
    pub fn new() -> Self {
        Self {
            boid_size: 0.01,
            color: [1.0, 0.0, 0.0, 1.0],
            history_color: [0.0, 0.5, 0.4, 1.0],
            history_size: 1.0,
            history_len: 200,
        }
    }

    pub fn build(
        self,
        ctx: &Context,
        boids: &[Boid],
        camera: &Camera,
        view: &ViewMatrix,
    ) -> Result<BoidsShader> {
        let gl = ctx.gl();
        let camera_ubo = CameraUbo::new(gl, camera, view)?;
        let mut boids_shaders: Vec<BoidShader> = vec![];
        for b in boids {
            let bi = BoidShader::new(ctx, b, self.boid_size, self.history_len, &camera_ubo)?;
            bi.use_program();
            bi.set_ambient(self.color);
            bi.draw();
            let hist = bi.history();
            hist.use_program();
            hist.set_ambient(self.history_color);
            hist.set_point_size(self.history_size);
            hist.draw();
            boids_shaders.push(bi);
        }
        Ok(BoidsShader {
            boids: boids_shaders,
            camera: camera_ubo,
        })
    }
}

pub struct BoidsShader {
    pub boids: Vec<BoidShader>,
    pub camera: CameraUbo,
}

#[derive(Debug, PartialEq)]
pub enum BoidVd {
    Position,
}

impl VaoDefine for BoidVd {
    fn iter() -> std::slice::Iter<'static, Self> {
        [BoidVd::Position].iter()
    }

    fn name(&self) -> &'static str {
        match self {
            BoidVd::Position => "position",
        }
    }

    fn size_of(&self) -> i32 {
        match self {
            BoidVd::Position => 3,
        }
    }
}

pub struct BoidShader {
    program: Program,
    ambient: WebGlUniformLocation,
    vao: Vao<BoidVd>,
    vertex_len: i32,
    size: f32,
    history: BoidHistoryShader,
}

impl BoidShader {
    // TODO: mvpはUniformBufferObjectにする
    const VERT: &'static str = r#"#version 300 es
layout(location = 0) in vec3 position;
layout (std140) uniform matrix {
    mat4 mvp;
} mat;

void main() {
    gl_Position = mat.mvp * vec4(position, 1.0);
}
"#;

    const FRAG: &'static str = r#"#version 300 es
precision mediump float;

uniform vec4 ambient;
out vec4 fragmentColor;

void main() {
    fragmentColor = ambient;
}
"#;

    // for TRIANGLE STRIP
    // Z方向は無視していたポリゴンで描画する
    fn rect(b: &Boid, size: f32) -> [GlPoint3d; 4] {
        let p = b.pos();
        [
            GlPoint3d::new(p.x - size, p.y - size, p.z),
            GlPoint3d::new(p.x + size, p.y - size, p.z),
            GlPoint3d::new(p.x - size, p.y + size, p.z),
            GlPoint3d::new(p.x + size, p.y + size, p.z),
        ]
    }

    pub fn new(
        ctx: &Context,
        b: &Boid,
        size: f32,
        hist_len: usize,
        camera: &CameraUbo,
    ) -> Result<Self> {
        let program = ctx.program(Self::VERT, Self::FRAG)?;
        let gl = ctx.gl();
        camera.bind_ubo(gl, &program);

        let ambient = program.uniform_location("ambient")?;
        let mut vao = program.create_vao()?;
        let vert = Self::rect(b, size);
        vao.buffer_data(BoidVd::Position, &vert, gl::DYNAMIC_DRAW);

        let history = BoidHistoryShader::new(ctx, b, hist_len, camera)?;
        Ok(Self {
            program,
            ambient,
            vao,
            vertex_len: vert.len() as i32,
            size,
            history,
        })
    }

    pub fn use_program(&self) {
        self.program.use_program();
    }

    pub fn update(&mut self, b: &Boid) {
        self.vao
            .buffer_sub_data(BoidVd::Position, &Self::rect(b, self.size), 0);
    }

    pub fn set_ambient(&self, ambient: [f32; 4]) {
        self.program.gl().uniform4f(
            Some(&self.ambient),
            ambient[0],
            ambient[1],
            ambient[2],
            ambient[3],
        );
    }

    pub fn draw(&self) {
        self.vao.bind();
        self.program
            .gl()
            .draw_arrays(gl::TRIANGLE_STRIP, 0, self.vertex_len);
    }

    pub fn history(&self) -> &BoidHistoryShader {
        &self.history
    }

    pub fn history_mut(&mut self) -> &mut BoidHistoryShader {
        &mut self.history
    }
}

/// posの記録を行うシェーダー
pub struct BoidHistoryShader {
    program: Program,
    ambient: WebGlUniformLocation,
    point_size: WebGlUniformLocation,
    vao: Vao<BoidVd>,
    vertex_len: i32,

    // 書き込む頂点位置の調整
    current_index: i32,
    vbo_len: i32,
}

impl BoidHistoryShader {
    // TODO: mvpはUniformBufferObjectにする
    const VERT: &'static str = r#"#version 300 es
layout(location = 0) in vec3 position;
layout (std140) uniform matrix {
    mat4 mvp;
} mat;
uniform float pointSize;

void main() {
    gl_Position = mat.mvp * vec4(position, 1.0);
    gl_PointSize = pointSize;
}
"#;

    const FRAG: &'static str = r#"#version 300 es
precision mediump float;

uniform vec4 ambient;
out vec4 fragmentColor;

void main() {
    fragmentColor = ambient;
}
"#;

    fn new(ctx: &Context, b: &Boid, hist_len: usize, camera: &CameraUbo) -> Result<Self> {
        let program = ctx.program(Self::VERT, Self::FRAG)?;
        let gl = ctx.gl();
        camera.bind_ubo(gl, &program);

        let ambient = program.uniform_location("ambient")?;
        let point_size = program.uniform_location("pointSize")?;

        let mut vao = program.create_vao()?;

        let vbo_len = hist_len.next_power_of_two();
        let pos = b.pos();
        let pos = GlPoint3d::new(pos.x, pos.y, pos.z);
        let v = vec![pos; vbo_len];
        vao.buffer_data(BoidVd::Position, &v, gl::DYNAMIC_DRAW);

        Ok(Self {
            program,
            ambient,
            point_size,
            vao,
            vertex_len: v.len() as i32,
            current_index: 0,
            vbo_len: vbo_len as i32,
        })
    }

    // 長さが2の倍数であることを前提に位置計算
    fn index(&self, pos: i32) -> i32 {
        pos & (self.vbo_len - 1)
    }

    pub fn use_program(&self) {
        self.program.use_program();
    }

    pub fn update(&mut self, b: &Boid) {
        let next = self.index(self.current_index + 1);
        let pos = GlPoint3d::new(b.pos().x, b.pos().y, b.pos().z);
        self.vao.buffer_sub_data(BoidVd::Position, &[pos], next);
        self.current_index = next;
    }

    pub fn set_ambient(&self, ambient: [f32; 4]) {
        self.program.gl().uniform4f(
            Some(&self.ambient),
            ambient[0],
            ambient[1],
            ambient[2],
            ambient[3],
        );
    }

    pub fn set_point_size(&self, size: f32) {
        self.program.gl().uniform1f(Some(&self.point_size), size);
    }

    pub fn draw(&self) {
        self.vao.bind();
        self.program
            .gl()
            .draw_arrays(gl::POINTS, 0, self.vertex_len);
    }
}
