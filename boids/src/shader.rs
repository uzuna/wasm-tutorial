use web_sys::WebGlUniformLocation;
use webgl2::{gl, vertex::VertexVbo, GlPoint3D, Program};

use crate::{
    camera::{Camera, ViewMatrix},
    error::*,
};

pub struct BoidsMemory {
    pub positions: Vec<GlPoint3D>,
}

impl BoidsMemory {
    pub fn new(len: usize) -> Self {
        Self {
            positions: vec![GlPoint3D::new(0.0, 0.0, 0.0); len],
        }
    }
}

pub struct Shader {
    program: Program,
    mvp: WebGlUniformLocation,
    boids: BoidsMemory,
    vbo: VertexVbo,
}

impl Shader {
    // versionは開業よりも前になければならない。
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

    pub fn new(gl: &gl, boids: BoidsMemory) -> Result<Self> {
        let program = Program::new(gl, Self::VERT, Self::FRAG)?;
        let mvp = gl
            .get_uniform_location(program.program(), "mvp")
            .ok_or(Error::gl("Failed to get uniform location".into()))?;
        let vbo = VertexVbo::new(gl, &boids.positions, 0)?;
        Ok(Self {
            program,
            mvp,
            boids,
            vbo,
        })
    }

    pub fn use_program(&self, gl: &gl) {
        self.program.use_program(gl);
    }

    pub fn set_mvp(&self, gl: &gl, camera: &Camera, view: &ViewMatrix) {
        let mvp = camera.perspective() * view.look_at();
        // gl.uniform_matrix4fv_with_f32_array(Some(&self.mvp), false, mvp.as_slice());
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
}
