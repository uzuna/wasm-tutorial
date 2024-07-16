use web_sys::WebGlUniformLocation;
use webgl2::{gl, uniform_location, vertex::VertexVbo, GlPoint3D, Program};

use crate::{
    boids::Boid,
    camera::{Camera, ViewMatrix},
    error::*,
};

pub struct BoidShader {
    program: Program,
    mvp: WebGlUniformLocation,
    ambient: WebGlUniformLocation,
    vbo: VertexVbo,
    size: f32,
    history: BoidHistoryShader,
}

impl BoidShader {
    // TODO: mvpはUniformBufferObjectにする
    const VERT: &'static str = r#"#version 300 es
layout(location = 0) in vec3 position;
uniform mat4 mvp;

void main() {
    gl_Position = mvp * vec4(position, 1.0);
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

    const LOCATION_POSITION: u32 = 0;

    // for TRIANGLE STRIP
    // Z方向は無視していたポリゴンで描画する
    fn rect(b: &Boid, size: f32) -> [GlPoint3D; 4] {
        let p = b.pos();
        [
            GlPoint3D::new(p.x - size, p.y - size, p.z),
            GlPoint3D::new(p.x + size, p.y - size, p.z),
            GlPoint3D::new(p.x - size, p.y + size, p.z),
            GlPoint3D::new(p.x + size, p.y + size, p.z),
        ]
    }

    pub fn new(gl: &gl, b: &Boid, size: f32, hist_len: usize) -> Result<Self> {
        let program = Program::new(gl, Self::VERT, Self::FRAG)?;
        let mvp = uniform_location!(gl, &program, "mvp")?;
        let ambient = uniform_location!(gl, &program, "ambient")?;
        let vbo = VertexVbo::new(gl, &Self::rect(b, size), BoidShader::LOCATION_POSITION)?;
        let history = BoidHistoryShader::new(gl, b, hist_len)?;
        Ok(Self {
            program,
            mvp,
            ambient,
            vbo,
            size,
            history,
        })
    }

    pub fn use_program(&self, gl: &gl) {
        self.program.use_program(gl);
    }

    pub fn update(&mut self, gl: &gl, b: &Boid) {
        self.vbo.update_vertex(gl, &Self::rect(b, self.size));
    }

    pub fn set_mvp(&self, gl: &gl, camera: &Camera, view: &ViewMatrix) {
        let mvp = camera.perspective() * view.look_at();
        let mvp_arrays: [[f32; 4]; 4] = mvp.into();
        let mvp_matrices = mvp_arrays.iter().flat_map(|a| *a).collect::<Vec<_>>();

        gl.uniform_matrix4fv_with_f32_array_and_src_offset_and_src_length(
            Some(&self.mvp),
            false,
            &mvp_matrices,
            0,
            0,
        );
    }

    pub fn set_ambient(&self, gl: &gl, ambient: [f32; 4]) {
        gl.uniform4f(
            Some(&self.ambient),
            ambient[0],
            ambient[1],
            ambient[2],
            ambient[3],
        );
    }

    pub fn draw(&self, gl: &gl) {
        self.vbo.bind(gl);
        gl.draw_arrays(gl::TRIANGLE_STRIP, 0, self.vbo.count());
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
    mvp: WebGlUniformLocation,
    ambient: WebGlUniformLocation,
    point_size: WebGlUniformLocation,
    vbo: VertexVbo,

    // 書き込む頂点位置の調整
    current_index: i32,
    vbo_len: i32,
}

impl BoidHistoryShader {
    // TODO: mvpはUniformBufferObjectにする
    const VERT: &'static str = r#"#version 300 es
layout(location = 0) in vec3 position;
uniform mat4 mvp;
uniform float pointSize;

void main() {
    gl_Position = mvp * vec4(position, 1.0);
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

    const LOCATION_POSITION: u32 = 0;

    fn new(gl: &gl, b: &Boid, hist_len: usize) -> Result<Self> {
        let program = Program::new(gl, Self::VERT, Self::FRAG)?;
        let mvp = uniform_location!(gl, &program, "mvp")?;
        let ambient = uniform_location!(gl, &program, "ambient")?;
        let point_size = uniform_location!(gl, &program, "pointSize")?;

        let vbo_len = hist_len.next_power_of_two();
        let pos = b.pos();
        let pos = GlPoint3D::new(pos.x, pos.y, pos.z);
        let v = vec![pos; vbo_len];
        let vbo = VertexVbo::new(gl, &v, BoidHistoryShader::LOCATION_POSITION)?;
        Ok(Self {
            program,
            mvp,
            ambient,
            point_size,
            vbo,
            current_index: 0,
            vbo_len: vbo_len as i32,
        })
    }

    // 長さが2の倍数であることを前提に位置計算
    fn index(&self, pos: i32) -> i32 {
        pos & (self.vbo_len - 1)
    }

    pub fn use_program(&self, gl: &gl) {
        self.program.use_program(gl);
    }

    pub fn update(&mut self, gl: &gl, b: &Boid) {
        let next = self.index(self.current_index + 1);
        let pos = GlPoint3D::new(b.pos().x, b.pos().y, b.pos().z);
        self.vbo.update_vertex_sub(gl, &[pos], next);
        self.current_index = next;
    }

    pub fn set_mvp(&self, gl: &gl, camera: &Camera, view: &ViewMatrix) {
        let mvp = camera.perspective() * view.look_at();
        let mvp_arrays: [[f32; 4]; 4] = mvp.into();
        let mvp_matrices = mvp_arrays.iter().flat_map(|a| *a).collect::<Vec<_>>();

        gl.uniform_matrix4fv_with_f32_array_and_src_offset_and_src_length(
            Some(&self.mvp),
            false,
            &mvp_matrices,
            0,
            0,
        );
    }

    pub fn set_ambient(&self, gl: &gl, ambient: [f32; 4]) {
        gl.uniform4f(
            Some(&self.ambient),
            ambient[0],
            ambient[1],
            ambient[2],
            ambient[3],
        );
    }

    pub fn set_point_size(&self, gl: &gl, size: f32) {
        gl.uniform1f(Some(&self.point_size), size);
    }

    pub fn draw(&self, gl: &gl) {
        self.vbo.bind(gl);
        gl.draw_arrays(gl::POINTS, 0, self.vbo.count());
    }
}
