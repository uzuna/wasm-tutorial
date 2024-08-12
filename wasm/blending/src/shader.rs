use std::rc::Rc;

use webgl2::{gl, uniform_location, GlPoint, GlPoint2d, GlPoint3d, GlPoint4d, Program};
use bytemuck::NoUninit;
use wasm_bindgen::JsError;
use wasm_utils::{error::*, info};
use web_sys::{WebGlBuffer, WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject};

fn create_buffer(gl: &gl) -> Result<web_sys::WebGlBuffer> {
    gl.create_buffer()
        .ok_or(JsError::new("Failed to create_buffer"))
}

pub struct SimpleShader {
    gl: Rc<gl>,
    program: Program,
    color: WebGlUniformLocation,
    position: u32,
    vbo: WebGlBuffer,
}

impl SimpleShader {
    pub const VERT: &'static str = r#"attribute vec2 position;
void main(void){
	gl_Position = vec4(position.xy, 0.0, 1.0);
}
"#;

    pub const FRAG: &'static str = r#"precision mediump float;

uniform vec4 u_color;

void main(void){
    gl_FragColor = u_color;
}
"#;

    pub fn new(gl: Rc<gl>, data: &[f32]) -> Result<Self> {
        let program = Program::new(&gl, Self::VERT, Self::FRAG)?;
        program.use_program(&gl);
        let color = uniform_location(&gl, &program, "u_color")?;
        let position = gl.get_attrib_location(program.program(), "position") as u32;
        info!(
            "get_attrib_location = {}, error: {}",
            position,
            gl.get_error()
        );
        let vbo = create_buffer(&gl)?;
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(&vbo));
        #[rustfmt::skip]
        VertexObject::buffer_data(
            &gl,
            gl::ARRAY_BUFFER,
            data,
            gl::STATIC_DRAW,
        );

        info!("buffer_data {}", gl.get_error());

        gl.enable_vertex_attrib_array(position);
        info!("enable_vertex_attrib_array {}", gl.get_error());

        gl.vertex_attrib_pointer_with_i32(position, 2, gl::FLOAT, false, 0, 0);
        info!("vertex_attrib_pointer_with_i32 {}", gl.get_error());

        let s = Self {
            gl,
            program,
            color,
            position,
            vbo,
        };
        s.init();
        Ok(s)
    }

    pub fn init(&self) {
        self.set_color([1.0, 0.0, 0.0, 1.0]);
    }

    pub fn use_program(&self) {
        self.program.use_program(&self.gl);
    }

    pub fn set_color(&self, color: [f32; 4]) {
        self.gl.uniform4fv_with_f32_array(Some(&self.color), &color);
    }

    pub fn draw(&self) {
        self.use_program();
        self.gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
        info!("draw_arrays {}", self.gl.get_error());
    }
}

pub struct VertexObject {
    gl: Rc<gl>,
    vertex: web_sys::WebGlBuffer,
    coord: web_sys::WebGlBuffer,
    color: web_sys::WebGlBuffer,
    index: web_sys::WebGlBuffer,
    index_count: i32,
}

impl VertexObject {
    const RECT_VERTEX: [GlPoint3d; 4] = [
        GlPoint3d::new(-1.0, 1.0, 0.0),
        GlPoint3d::new(1.0, 1.0, 0.0),
        GlPoint3d::new(-1.0, -1.0, 0.0),
        GlPoint3d::new(1.0, -1.0, 0.0),
    ];

    const RECT_COORD: [GlPoint2d; 4] = [
        GlPoint2d::new(0.0, 0.0),
        GlPoint2d::new(1.0, 0.0),
        GlPoint2d::new(0.0, 1.0),
        GlPoint2d::new(1.0, 1.0),
    ];

    const RECT_COLOR: [GlPoint4d; 4] = [
        GlPoint4d::new(1.0, 0.0, 0.0, 1.0),
        GlPoint4d::new(0.0, 1.0, 0.0, 1.0),
        GlPoint4d::new(0.0, 0.0, 1.0, 1.0),
        GlPoint4d::new(1.0, 1.0, 1.0, 1.0),
    ];

    const RECT_INDEX: [u16; 6] = [0, 1, 2, 3, 2, 1];
    pub fn new(gl: Rc<gl>) -> Result<Self> {
        let vertex = create_buffer(&gl)?;
        let coord = create_buffer(&gl)?;
        let color = create_buffer(&gl)?;
        let index = create_buffer(&gl)?;

        Ok(Self {
            gl,
            vertex,
            coord,
            color,
            index,
            index_count: 0,
        })
    }

    pub fn rect(gl: Rc<gl>) -> Result<Self> {
        let mut v = Self::new(gl)?;
        v.rect_inner();
        Ok(v)
    }

    fn rect_inner(&mut self) {
        let gl = &self.gl;
        let data = bytemuck::cast_slice(&Self::RECT_VERTEX);
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(&self.vertex));
        Self::buffer_data(gl, gl::ARRAY_BUFFER, data, gl::STATIC_DRAW);
        gl.bind_buffer(gl::ARRAY_BUFFER, None);
        info!("bind_buffer {}", gl.get_error());

        let data = bytemuck::cast_slice(&Self::RECT_COORD);
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(&self.coord));
        Self::buffer_data(gl, gl::ARRAY_BUFFER, data, gl::STATIC_DRAW);
        gl.bind_buffer(gl::ARRAY_BUFFER, None);

        let data = bytemuck::cast_slice(&Self::RECT_COLOR);
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(&self.color));
        Self::buffer_data(gl, gl::ARRAY_BUFFER, data, gl::STATIC_DRAW);
        gl.bind_buffer(gl::ARRAY_BUFFER, None);

        let data = bytemuck::cast_slice(&Self::RECT_INDEX);
        gl.bind_buffer(gl::ELEMENT_ARRAY_BUFFER, Some(&self.index));
        Self::buffer_data(gl, gl::ELEMENT_ARRAY_BUFFER, data, gl::STATIC_DRAW);
        gl.bind_buffer(gl::ARRAY_BUFFER, None);

        self.index_count = Self::RECT_INDEX.len() as i32;
    }

    pub fn buffer_data(gl: &gl, target: u32, data: &[f32], usage: u32) {
        unsafe {
            let view = js_sys::Float32Array::view(data);
            gl.buffer_data_with_array_buffer_view(target, &view, usage);
        }
    }
}

pub struct TextureShader {
    gl: Rc<gl>,
    program: Program,
    uniform: GradUniform,
}

impl TextureShader {
    // x方向は時間情報なので、表示範囲の指定にwindow_matを使う
    const VERT: &'static str = r#"#version 300 es

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 coord;

uniform mat3 window_mat;

out vec2 tex_coord;

void main() {
    gl_Position = vec4((window_mat * vec3(position, 1.0)).xy, 0.0, 1.0);
    tex_coord = coord;
}
"#;

    const FRAG: &'static str = r#"#version 300 es

precision mediump float;
uniform sampler2D u_texture;
in vec2 tex_coord;

out vec4 fragmentColor;

void main() {
    fragmentColor = texture(u_texture, tex_coord);
}
"#;
    pub fn new(gl: Rc<gl>) -> Result<Self> {
        let program = Program::new(&gl, Self::VERT, Self::FRAG)?;
        program.use_program(&gl);
        let uniform = GradUniform::new(gl.clone(), &program)?;
        uniform.init();
        Ok(Self {
            gl,
            program,
            uniform,
        })
    }

    pub fn uniform(&self) -> &GradUniform {
        &self.uniform
    }

    pub fn create_vao(&self, vert: &[GlPoint2d; 4]) -> Result<GradVao> {
        let locs = ["position", "coord"]
            .map(|s| self.gl.get_attrib_location(self.program.program(), s) as u32);
        GradVao::new(self.gl.clone(), vert, locs)
    }

    pub fn draw(&self, vao: &GradVao) {
        self.program.use_program(&self.gl);
        self.gl.bind_vertex_array(Some(&vao.vao));
        self.gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
    }
}

pub struct GradUniform {
    gl: Rc<gl>,
    mat: WebGlUniformLocation,
    texture: WebGlUniformLocation,
}

impl GradUniform {
    pub fn new(gl: Rc<gl>, program: &Program) -> Result<Self> {
        let mat = uniform_location(&gl, program, "window_mat")?;
        let texture = uniform_location(&gl, program, "u_texture")?;
        Ok(Self { gl, mat, texture })
    }

    pub fn init(&self) {
        self.set_mat(nalgebra::Matrix3::identity());
        self.set_texture(0);
    }

    pub fn set_mat(&self, mat: nalgebra::Matrix3<f32>) {
        self.gl
            .uniform_matrix3fv_with_f32_array(Some(&self.mat), false, mat.as_slice());
    }

    pub fn set_texture(&self, texture: i32) {
        self.gl.uniform1i(Some(&self.texture), texture);
    }
}

pub struct GradVao {
    gl: Rc<gl>,
    vao: WebGlVertexArrayObject,
    vertex: WebGlBuffer,
    coord: WebGlBuffer,
}

impl GradVao {
    const VERT: [GlPoint2d; 4] = [
        GlPoint2d::new(-1.0, 1.0),
        GlPoint2d::new(1.0, 1.0),
        GlPoint2d::new(-1.0, -1.0),
        GlPoint2d::new(1.0, -1.0),
    ];

    const FRAG: [GlPoint2d; 4] = [
        GlPoint2d::new(0.0, 0.0),
        GlPoint2d::new(1.0, 0.0),
        GlPoint2d::new(0.0, 1.0),
        GlPoint2d::new(1.0, 1.0),
    ];

    fn new(gl: Rc<gl>, rect: &[GlPoint2d; 4], locs: [u32; 2]) -> Result<Self> {
        let vao = gl
            .create_vertex_array()
            .ok_or(JsError::new("Failed to create vao"))?;
        gl.bind_vertex_array(Some(&vao));
        let vertex = create_buffer(&gl)?;
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(&vertex));
        Self::buffer_data(&gl, gl::ARRAY_BUFFER, rect, gl::STATIC_DRAW);
        gl.enable_vertex_attrib_array(locs[0]);
        gl.vertex_attrib_pointer_with_i32(locs[0], GlPoint2d::size(), gl::FLOAT, false, 0, 0);

        let coord = create_buffer(&gl)?;
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(&coord));
        Self::buffer_data(&gl, gl::ARRAY_BUFFER, &Self::FRAG, gl::STATIC_DRAW);
        gl.enable_vertex_attrib_array(locs[1]);
        gl.vertex_attrib_pointer_with_i32(locs[1], GlPoint2d::size(), gl::FLOAT, false, 0, 0);

        gl.bind_vertex_array(None);
        Ok(Self {
            gl,
            vao,
            vertex,
            coord,
        })
    }

    pub fn buffer_data<P: GlPoint + NoUninit>(gl: &gl, target: u32, data: &[P], usage: u32) {
        let data = bytemuck::cast_slice(data);
        unsafe {
            let view = js_sys::Float32Array::view(data);
            gl.buffer_data_with_array_buffer_view(target, &view, usage);
        }
    }
}

/// 1x1pxの色のテクスチャを作成する
pub fn color_texture(gl: &gl, color: [u8; 4]) -> WebGlTexture {
    let texture = gl.create_texture().expect("Failed to create texture");
    gl.bind_texture(gl::TEXTURE_2D, Some(&texture));
    gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
    gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
    gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        gl::TEXTURE_2D,
        0,
        gl::RGBA as i32,
        1,
        1,
        0,
        gl::RGBA,
        gl::UNSIGNED_BYTE,
        Some(&color),
    )
    .expect("Failed to set texture image");
    texture
}
