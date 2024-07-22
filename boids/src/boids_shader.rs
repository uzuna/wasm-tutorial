use web_sys::{js_sys, WebGlBuffer, WebGlUniformLocation};
use webgl2::{gl, uniform_block_binding, uniform_location, vertex::VertexVbo, GlPoint3D, Program};

use crate::{
    boids::Boid,
    camera::{Camera, ViewMatrix},
    error::*,
    info,
};

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
        gl: &gl,
        boids: &[Boid],
        camera: &Camera,
        view: &ViewMatrix,
    ) -> Result<BoidsShader> {
        let camera_ubo = CameraUBO::new(gl, camera, view)?;
        let mut boids_shaders: Vec<BoidShader> = vec![];
        for b in boids {
            let bi = BoidShader::new(gl, b, self.boid_size, self.history_len, &camera_ubo)?;
            bi.use_program(gl);
            bi.set_ambient(gl, self.color);
            bi.draw(gl);
            let hist = bi.history();
            hist.use_program(gl);
            hist.set_ambient(gl, self.history_color);
            hist.set_point_size(gl, self.history_size);
            hist.draw(gl);
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
    pub camera: CameraUBO,
}

pub struct CameraUBO {
    ubo: WebGlBuffer,
}

impl CameraUBO {
    fn new(gl: &gl, camera: &Camera, view: &ViewMatrix) -> Result<Self> {
        let ubo = gl
            .create_buffer()
            .ok_or(Error::gl("failed to create buffer".into()))?;
        let mvp = Self::gen_matrix(camera, view);
        info!("CameraUBO: mvp: {:?}", mvp);

        gl.bind_buffer(gl::UNIFORM_BUFFER, Some(&ubo));
        unsafe {
            let view = js_sys::Float32Array::view(&mvp);
            gl.buffer_data_with_array_buffer_view(gl::UNIFORM_BUFFER, &view, gl::DYNAMIC_DRAW);
        }
        gl.bind_buffer(gl::UNIFORM_BUFFER, None);
        Ok(Self { ubo })
    }

    fn gen_matrix(camera: &Camera, view: &ViewMatrix) -> Vec<f32> {
        let mvp = camera.perspective() * view.look_at();
        info!("perspective: {:?}", camera.perspective());
        info!("lookat: {:?}", view.look_at());
        let mvp_arrays: [[f32; 4]; 4] = mvp.into();
        mvp_arrays.iter().flat_map(|a| *a).collect::<Vec<_>>()
    }

    pub fn update_mvp(&self, gl: &gl, camera: &Camera, view: &ViewMatrix) {
        let mvp = Self::gen_matrix(camera, view);

        gl.bind_buffer(gl::UNIFORM_BUFFER, Some(&self.ubo));
        unsafe {
            let view = js_sys::Float32Array::view(&mvp);
            gl.buffer_sub_data_with_i32_and_array_buffer_view(gl::UNIFORM_BUFFER, 0, &view);
        }
        gl.bind_buffer(gl::UNIFORM_BUFFER, None);
    }
}

pub struct BoidShader {
    program: Program,
    ambient: WebGlUniformLocation,
    vbo: VertexVbo,
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

    const LOCATION_POSITION: u32 = 0;
    const MVP_UBI: u32 = 0;

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

    pub fn new(gl: &gl, b: &Boid, size: f32, hist_len: usize, camera: &CameraUBO) -> Result<Self> {
        let program = Program::new(gl, Self::VERT, Self::FRAG)?;
        uniform_block_binding!(gl, &program, "matrix", Self::MVP_UBI);
        gl.bind_buffer_base(gl::UNIFORM_BUFFER, Self::MVP_UBI, Some(&camera.ubo));
        let ambient = uniform_location!(gl, &program, "ambient")?;
        let vbo = VertexVbo::new(gl, &Self::rect(b, size), BoidShader::LOCATION_POSITION)?;
        let history = BoidHistoryShader::new(gl, b, hist_len, camera)?;
        Ok(Self {
            program,
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

    const LOCATION_POSITION: u32 = 0;
    const MVP_UBI: u32 = 0;

    fn new(gl: &gl, b: &Boid, hist_len: usize, camera: &CameraUBO) -> Result<Self> {
        let program = Program::new(gl, Self::VERT, Self::FRAG)?;

        uniform_block_binding!(gl, &program, "matrix", Self::MVP_UBI);
        gl.bind_buffer_base(gl::UNIFORM_BUFFER, Self::MVP_UBI, Some(&camera.ubo));

        let ambient = uniform_location!(gl, &program, "ambient")?;
        let point_size = uniform_location!(gl, &program, "pointSize")?;

        let vbo_len = hist_len.next_power_of_two();
        let pos = b.pos();
        let pos = GlPoint3D::new(pos.x, pos.y, pos.z);
        let v = vec![pos; vbo_len];
        let vbo = VertexVbo::new(gl, &v, BoidHistoryShader::LOCATION_POSITION)?;
        Ok(Self {
            program,
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
