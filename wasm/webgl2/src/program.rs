//! シェーダープログラムを扱うモジュール

use std::rc::Rc;

use web_sys::{WebGlProgram, WebGlShader, WebGlUniformLocation};

use crate::{error::Result, gl, JsError};

/// 2つのコンパイル済みシェーダーを渡してプログラムを作成する
pub fn link_program(gl: &gl, vertex: &WebGlShader, fragment: &WebGlShader) -> Result<WebGlProgram> {
    let program = gl
        .create_program()
        .ok_or(JsError::new("Failed to create program object"))?;
    gl.attach_shader(&program, vertex);
    gl.attach_shader(&program, fragment);
    gl.link_program(&program);

    if gl
        .get_program_parameter(&program, gl::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        let log = gl
            .get_program_info_log(&program)
            .unwrap_or(String::from("Failed to link program"));
        gl.delete_program(Some(&program));
        Err(JsError::new(&log))
    }
}

/// 頂点シェーダーをコンパイルする
pub fn compile_vertex(gl: &gl, vertex: &str) -> Result<WebGlShader> {
    let s = compile_shader(gl, vertex, ShaderType::Vertex)?;
    Ok(s)
}

/// フラグメントシェーダーをコンパイルする
pub fn compile_fragment(gl: &gl, fragment: &str) -> Result<WebGlShader> {
    let s = compile_shader(gl, fragment, ShaderType::Fragment)?;
    Ok(s)
}

pub fn compile_program(gl: &gl, vertex: &str, fragment: &str) -> Result<WebGlProgram> {
    let vertex = compile_vertex(gl, vertex)?;
    let fragment = compile_fragment(gl, fragment)?;
    link_program(gl, &vertex, &fragment)
}

pub fn uniform_location(
    gl: &gl,
    program: &WebGlProgram,
    name: &str,
) -> Result<WebGlUniformLocation> {
    gl.get_uniform_location(program, name)
        .ok_or(JsError::new(&format!(
            "Failed to get uniform location {}",
            name
        )))
}

pub fn uniform_block_binding(gl: &gl, program: &WebGlProgram, name: &str, index: u32) {
    gl.uniform_block_binding(program, gl.get_uniform_block_index(program, name), index);
}

/// シェーダースクリプトの種類
#[derive(Debug)]
enum ShaderType {
    /// 頂点シェーダー
    Vertex,
    /// フラグメントシェーダー
    Fragment,
}

impl ShaderType {
    fn to_glenum(&self) -> u32 {
        match self {
            ShaderType::Vertex => gl::VERTEX_SHADER,
            ShaderType::Fragment => gl::FRAGMENT_SHADER,
        }
    }
}

// Shaderのコンパイルする
fn compile_shader(gl: &gl, shader_script: &str, type_: ShaderType) -> Result<WebGlShader> {
    let shader = gl
        .create_shader(type_.to_glenum())
        .ok_or(JsError::new("Failed to create shader object"))?;
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
        Err(JsError::new(&log))
    }
}

/// WebGLコンテキストに結びついたシェーダープログラムの構造体
#[cfg(feature = "context")]
pub struct Program {
    ctx: Rc<crate::context::ContextInner>,
    program: WebGlProgram,
    vertex: WebGlShader,
    fragment: WebGlShader,
}

#[cfg(feature = "context")]
impl Program {
    pub(crate) fn new(
        ctx: Rc<crate::context::ContextInner>,
        vert: &str,
        frag: &str,
    ) -> Result<Self> {
        let gl = ctx.gl();
        let vertex = compile_vertex(gl, vert)?;
        let fragment = compile_fragment(gl, frag)?;

        // Link shaders
        let program = link_program(gl, &vertex, &fragment)?;
        #[cfg(feature = "metrics")]
        ctx.metrics().shader.inc_shader(1);
        Ok(Self {
            ctx,
            program,
            vertex,
            fragment,
        })
    }

    pub(crate) fn ctx(&self) -> Rc<crate::context::ContextInner> {
        self.ctx.clone()
    }

    /// 生のWebGL2RenderingContextを取得する
    pub fn gl(&self) -> &Rc<gl> {
        self.ctx.gl()
    }

    /// プログラムを有効にする
    pub fn use_program(&self) {
        self.ctx.gl().use_program(Some(&self.program));
    }

    /// 生のプログラムを取得する
    pub fn program(&self) -> &WebGlProgram {
        &self.program
    }

    /// uniform変数の位置を取得する
    pub fn uniform_location(&self, name: &str) -> Result<WebGlUniformLocation> {
        self.ctx
            .gl()
            .get_uniform_location(&self.program, name)
            .ok_or(JsError::new(&format!(
                "Failed to get uniform location {}",
                name
            )))
    }
}

#[cfg(feature = "context")]
impl Drop for Program {
    fn drop(&mut self) {
        let gl = self.ctx.gl();
        gl.delete_program(Some(&self.program));
        gl.delete_shader(Some(&self.vertex));
        gl.delete_shader(Some(&self.fragment));
        #[cfg(feature = "metrics")]
        self.ctx.metrics().shader.sub_shader(1);
    }
}
