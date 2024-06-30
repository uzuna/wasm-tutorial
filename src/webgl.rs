use nalgebra_glm::{TMat4, Vec3};
use web_sys::{WebGl2RenderingContext as gl, *};

use crate::{
    error::{Error, Result},
    log,
};

type GlEnum = u32;
type GlInt = i32;

/// OpenGLに渡す頂点情報を扱いやすくするためのトレイト
///
/// OpenGLの頂点情報は、`f32`の配列として扱う。
/// `1点あたりのデータ数 * 頂点数の長さ`の配列を渡す必要があり、単純なf32配列で扱うので認知負荷が高い。
/// 点数毎に与える引数はわかっているのでトレイトで表現する
pub trait GlPoint {
    /// 1点あたりのデータ数
    fn size() -> GlInt;
    /// 頂点情報の型。精度はf32で十分
    fn type_() -> GlEnum {
        gl::FLOAT
    }
}

/// OpenGLに渡す2次元の点の情報
///
/// 連続する2つの`f32`のデータとして見えなければならないのでCの構造体として定義する  
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[repr(C)]
pub struct GlPoint3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl GlPoint3D {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

impl GlPoint for GlPoint3D {
    fn size() -> GlInt {
        3
    }
}

/// OpenGLに渡す2次元の点の情報
///
/// 連続する2つの`f32`のデータとして見えなければならないのでCの構造体として定義する  
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[repr(C)]
pub struct GlPoint4D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl GlPoint4D {
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

impl GlPoint for GlPoint4D {
    fn size() -> GlInt {
        4
    }
}

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
            .get_uniform_location(&program.program, "mvp")
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

pub(crate) struct Program {
    program: WebGlProgram,
    vertex: WebGlShader,
    fragment: WebGlShader,
}

impl Program {
    pub fn new(gl: &gl, vert: &str, frag: &str) -> Result<Self> {
        let vertex = compile_vertex(gl, vert)?;
        let fragment = compile_fragment(gl, frag)?;

        // Link shaders
        let program = gl
            .create_program()
            .ok_or(Error::gl("Failed to create program object".into()))?;
        gl.attach_shader(&program, &vertex);
        gl.attach_shader(&program, &fragment);
        gl.link_program(&program);

        if gl
            .get_program_parameter(&program, gl::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(Self {
                program,
                vertex,
                fragment,
            })
        } else {
            let log = gl
                .get_program_info_log(&program)
                .unwrap_or(String::from("Failed to link program"));
            gl.delete_program(Some(&program));
            Err(Error::gl(log))
        }
    }

    pub fn use_program(&self, gl: &gl) {
        gl.use_program(Some(&self.program));
    }
}

/// シェーダースクリプトの種類の宣言
#[derive(Debug)]
pub enum ShaderType {
    Vertex,
    Fragment,
}

impl ShaderType {
    pub fn to_glenum(&self) -> u32 {
        match self {
            ShaderType::Vertex => gl::VERTEX_SHADER,
            ShaderType::Fragment => gl::FRAGMENT_SHADER,
        }
    }
}

/// 頂点シェーダーをコンパイルする
pub fn compile_vertex(gl: &gl, vertex: &str) -> Result<WebGlShader> {
    let s = unsafe { compile_shader(gl, vertex, ShaderType::Vertex)? };
    Ok(s)
}

/// フラグメントシェーダーをコンパイルする
pub fn compile_fragment(gl: &gl, fragment: &str) -> Result<WebGlShader> {
    let s = unsafe { compile_shader(gl, fragment, ShaderType::Fragment)? };
    Ok(s)
}

// Shaderのコンパイルする
unsafe fn compile_shader(gl: &gl, shader_script: &str, type_: ShaderType) -> Result<WebGlShader> {
    let shader = gl
        .create_shader(type_.to_glenum())
        .ok_or(Error::gl("Failed to create shader object".into()))?;
    gl.shader_source(&shader, shader_script);
    gl.compile_shader(&shader);

    if gl
        .get_shader_parameter(&shader, gl::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        let log = gl
            .get_shader_info_log(&shader)
            .unwrap_or(String::from("Failed to compile shader"));
        gl.delete_shader(Some(&shader));
        Err(Error::gl(log))
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
            let view = js_sys::Float32Array::view(&data);
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
