//! テクスチャを描画するためのシェーダー

use std::rc::Rc;

use web_sys::{WebGlTexture, WebGlUniformLocation};

use crate::{
    context::Context,
    error::Result,
    gl,
    program::Program,
    vertex::{Vao, VaoDefine},
    GlPoint, GlPoint2d,
};

/// シンプルなテクスチャ描画用のシェーダー
pub struct TextureShader {
    program: Program,
    uniform: TextureUniform,
}

impl TextureShader {
    const VERT: &'static str = r#"#version 300 es

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 coord;

uniform mat3 local_mat;

out vec2 tex_coord;

void main() {
    gl_Position = vec4((local_mat * vec3(position, 1.0)).xy, 0.0, 1.0);
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
    pub fn new(ctx: &Context) -> Result<Self> {
        let program = ctx.program(Self::VERT, Self::FRAG)?;
        program.use_program();
        let uniform = TextureUniform::new(&program)?;
        uniform.init();
        Ok(Self { program, uniform })
    }

    pub fn uniform(&self) -> &TextureUniform {
        &self.uniform
    }

    pub fn create_vao(&self, vert: &[GlPoint2d; 4]) -> Result<Vao<TextureVd>> {
        let mut vao = self.program.create_vao()?;
        vao.buffer_data(TextureVd::Position, vert, gl::STATIC_DRAW);
        vao.buffer_data(TextureVd::Coord, &TextureVd::FRAG, gl::STATIC_DRAW);
        Ok(vao)
    }

    /// テクスチャを描画する
    pub fn draw(&self, vao: &Vao<TextureVd>, texture: &WebGlTexture) {
        self.program.use_program();
        let gl = self.program.gl();
        gl.active_texture(gl::TEXTURE0);
        gl.bind_texture(gl::TEXTURE_2D, Some(texture));
        vao.bind();
        gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
    }
}

pub struct TextureUniform {
    gl: Rc<gl>,
    local_mat: WebGlUniformLocation,
    texture: WebGlUniformLocation,
}

impl TextureUniform {
    pub fn new(program: &Program) -> Result<Self> {
        let local_mat = program.uniform_location("local_mat")?;
        let texture = program.uniform_location("u_texture")?;

        Ok(Self {
            gl: program.gl().clone(),
            local_mat,
            texture,
        })
    }

    pub fn init(&self) {
        self.set_mat(nalgebra::Matrix3::identity());
        self.set_texture(0);
    }

    pub fn set_mat(&self, mat: nalgebra::Matrix3<f32>) {
        self.gl
            .uniform_matrix3fv_with_f32_array(Some(&self.local_mat), false, mat.as_slice());
    }

    pub fn set_texture(&self, texture: i32) {
        self.gl.uniform1i(Some(&self.texture), texture);
    }
}

#[derive(Debug, PartialEq)]
#[repr(u32)]
pub enum TextureVd {
    Position,
    Coord,
}

impl TextureVd {
    const FRAG: [GlPoint2d; 4] = [
        GlPoint2d::new(0.0, 0.0),
        GlPoint2d::new(1.0, 0.0),
        GlPoint2d::new(0.0, 1.0),
        GlPoint2d::new(1.0, 1.0),
    ];
}

impl VaoDefine for TextureVd {
    fn name(&self) -> &'static str {
        match self {
            TextureVd::Position => "position",
            TextureVd::Coord => "coord",
        }
    }

    fn iter() -> std::slice::Iter<'static, Self> {
        static VD: [TextureVd; 2] = [TextureVd::Position, TextureVd::Coord];
        VD.iter()
    }

    fn size_of(&self) -> i32 {
        match self {
            TextureVd::Position | TextureVd::Coord => GlPoint2d::size(),
        }
    }
}
