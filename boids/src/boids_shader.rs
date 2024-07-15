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
}

impl BoidShader {
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
    fn rect(b: &Boid) -> [GlPoint3D; 4] {
        let p = b.pos();
        [
            GlPoint3D::new(p.x - 0.5, p.y - 0.5, p.z),
            GlPoint3D::new(p.x + 0.5, p.y - 0.5, p.z),
            GlPoint3D::new(p.x - 0.5, p.y + 0.5, p.z),
            GlPoint3D::new(p.x + 0.5, p.y + 0.5, p.z),
        ]
    }

    pub fn new(gl: &gl, b: &Boid) -> Result<Self> {
        let program = Program::new(gl, Self::VERT, Self::FRAG)?;
        let mvp = uniform_location!(gl, &program, "mvp")?;
        let ambient = uniform_location!(gl, &program, "ambient")?;
        let vbo = VertexVbo::new(gl, &Self::rect(b), BoidShader::LOCATION_POSITION)?;
        Ok(Self {
            program,
            mvp,
            ambient,
            vbo,
        })
    }

    pub fn use_program(&self, gl: &gl) {
        self.program.use_program(gl);
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
}
