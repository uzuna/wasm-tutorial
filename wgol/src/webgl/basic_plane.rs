use nalgebra_glm::{TMat4, Vec3};
use web_sys::{WebGlBuffer, WebGlUniformLocation, WebGlVertexArrayObject};

use crate::error::{Error, Result};
use webgl2::{gl, GlEnum, GlInt, GlPoint, GlPoint3D, GlPoint4D, Program};

pub struct Shader {
    program: Program,
    mvp: WebGlUniformLocation,
    vao: ColorVertexVao,
}

impl Shader {
    // versionは開業よりも前になければならない。
    const VERT: &'static str = r#"#version 300 es

layout(location = 0) in vec3 position;
layout(location = 1) in vec4 color;

uniform mat4 mvp;

out vec4 vertexColor;

void main() {
    vertexColor = color;
    gl_Position = mvp * vec4(position, 1.0);
}
"#;

    const FRAG: &'static str = r#"#version 300 es

precision highp float;

in vec4 vertexColor;
out vec4 fragmentColor;

void main() {
    fragmentColor = vertexColor;
}
"#;

    pub const LOCATIONS: [u32; 2] = [0, 1];

    pub fn new(gl: &gl) -> Result<Self> {
        let program = Program::new(gl, Self::VERT, Self::FRAG)?;
        let mvp = gl
            .get_uniform_location(program.program(), "mvp")
            .ok_or(Error::gl("Failed to get uniform location".into()))?;
        let data = ColorVertexData::rect();
        let vao = ColorVertexVao::new(gl, &data, Self::LOCATIONS)?;

        Ok(Self { program, mvp, vao })
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

    pub fn draw(&self, gl: &gl) {
        gl.bind_vertex_array(Some(&self.vao.vao));
        gl.draw_elements_with_i32(gl::TRIANGLES, self.vao.index_count, gl::UNSIGNED_SHORT, 0);
    }
}

pub struct ViewMatrix {
    eye: Vec3,
    center: Vec3,
    up: Vec3,
}

impl ViewMatrix {
    pub const DEFAULT: Self = Self {
        eye: Vec3::new(0.0, 0.0, 3.0),
        center: Vec3::new(0.0, 0.0, 0.0),
        up: Vec3::new(0.0, 1.0, 0.0),
    };

    pub const fn new(eye: Vec3, center: Vec3, up: Vec3) -> Self {
        Self { eye, center, up }
    }

    pub fn look_at(&self) -> TMat4<f32> {
        nalgebra_glm::look_at(&self.eye, &self.center, &self.up)
    }
}

impl Default for ViewMatrix {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub struct Camera {
    aspect: f32,
    fovy: f32,
    near: f32,
    far: f32,
}

impl Camera {
    const DEFAULT: Self = Self {
        aspect: 1.0,
        fovy: 45.0,
        near: 0.1,
        far: 100.0,
    };

    const fn new(aspect: f32, fovy: f32, near: f32, far: f32) -> Self {
        Self {
            aspect,
            fovy,
            near,
            far,
        }
    }

    fn perspective(&self) -> TMat4<f32> {
        nalgebra_glm::perspective(
            self.aspect,
            self.fovy * std::f32::consts::PI / 180.0,
            self.near,
            self.far,
        )
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub struct ColorVertexData {
    pub vertex: Vec<GlPoint3D>,
    pub color: Vec<GlPoint4D>,
    pub index: Vec<u16>,
}

impl ColorVertexData {
    const VERT_RECT: &'static [GlPoint3D] = &[
        GlPoint3D::new(-1.0, -1.0, 0.0),
        GlPoint3D::new(1.0, -1.0, 0.0),
        GlPoint3D::new(-1.0, 1.0, 0.0),
        GlPoint3D::new(1.0, 1.0, 0.0),
    ];

    const COLOR_COORD: &'static [GlPoint4D] = &[
        GlPoint4D::new(1.0, 0.0, 0.0, 1.0),
        GlPoint4D::new(0.0, 1.0, 0.0, 1.0),
        GlPoint4D::new(0.0, 0.0, 1.0, 1.0),
        GlPoint4D::new(1.0, 1.0, 1.0, 1.0),
    ];

    const INDEX: &'static [u16] = &[0, 1, 2, 2, 1, 3];

    pub fn rect() -> Self {
        Self {
            vertex: Self::VERT_RECT.to_vec(),
            color: Self::COLOR_COORD.to_vec(),
            index: Self::INDEX.to_vec(),
        }
    }
}

pub struct ColorVertexVao {
    vao: WebGlVertexArrayObject,
    index_count: i32,
}

impl ColorVertexVao {
    pub fn new(gl: &gl, data: &ColorVertexData, locations: [u32; 2]) -> Result<Self> {
        let vao = gl
            .create_vertex_array()
            .ok_or(Error::gl("Failed to create vertex array object".into()))?;
        gl.bind_vertex_array(Some(&vao));

        let _vertex = Self::create_vertex_buffer(
            gl,
            unsafe {
                std::slice::from_raw_parts(
                    data.vertex.as_ptr() as *const f32,
                    data.vertex.len() * GlPoint3D::size() as usize,
                )
            },
            GlPoint3D::size(),
            locations[0],
            gl::ARRAY_BUFFER,
            gl::STATIC_DRAW,
        )?;
        let _color = Self::create_vertex_buffer(
            gl,
            unsafe {
                std::slice::from_raw_parts(
                    data.color.as_ptr() as *const f32,
                    data.color.len() * GlPoint4D::size() as usize,
                )
            },
            GlPoint4D::size(),
            locations[1],
            gl::ARRAY_BUFFER,
            gl::STATIC_DRAW,
        )?;

        let _index = Self::create_index_buffer(gl, &data.index)?;
        gl.bind_vertex_array(None);
        let index_count = data.index.len() as i32;

        Ok(Self { vao, index_count })
    }

    fn create_vertex_buffer(
        gl: &gl,
        data: &[f32],
        size: GlInt,
        location: u32,
        target: GlEnum,
        usage: GlEnum,
    ) -> Result<WebGlBuffer> {
        let buffer = gl
            .create_buffer()
            .ok_or(Error::gl("Failed to create buffer object".into()))?;
        gl.bind_buffer(target, Some(&buffer));
        unsafe {
            let view = js_sys::Float32Array::view(data);
            gl.buffer_data_with_array_buffer_view(target, &view, usage);
        }
        gl.enable_vertex_attrib_array(location);
        gl.vertex_attrib_pointer_with_i32(location, size, gl::FLOAT, false, 0, 0);

        // GLES2.0と違ってVAOにつなぐのでunbing不要
        Ok(buffer)
    }

    fn create_index_buffer(gl: &gl, data: &[u16]) -> Result<WebGlBuffer> {
        let ibo = gl
            .create_buffer()
            .ok_or(Error::gl("Failed to create buffer".into()))?;
        gl.bind_buffer(gl::ELEMENT_ARRAY_BUFFER, Some(&ibo));
        unsafe {
            let view = js_sys::Uint16Array::view(data);
            gl.buffer_data_with_array_buffer_view(gl::ELEMENT_ARRAY_BUFFER, &view, gl::STATIC_DRAW);
        }
        Ok(ibo)
    }
}
