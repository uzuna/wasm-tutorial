pub use web_sys::WebGl2RenderingContext as gl;
use web_sys::{WebGlProgram, WebGlShader};

use crate::error::{Error, Result};

pub type GlEnum = u32;
pub type GlInt = i32;

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
pub struct GlPoint2D {
    pub x: f32,
    pub y: f32,
}

impl GlPoint2D {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn norm(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

impl GlPoint for GlPoint2D {
    fn size() -> GlInt {
        2
    }
}

impl std::ops::Sub for GlPoint2D {
    type Output = GlPoint2D;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
impl std::ops::Mul<f32> for GlPoint2D {
    type Output = GlPoint2D;
    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl std::ops::DivAssign<f32> for GlPoint2D {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl std::ops::AddAssign for GlPoint2D {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
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

    pub fn program(&self) -> &WebGlProgram {
        &self.program
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
