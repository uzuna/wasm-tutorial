use std::rc::Rc;

use wasm_utils::{error::*, info};
use web_sys::{WebGlBuffer, WebGlTexture, WebGlUniformLocation};
use webgl2::{
    gl, uniform_location,
    vertex::{buffer_data, buffer_data_f32, create_buffer, Vao, VaoDefine},
    GlPoint, GlPoint2d, GlPoint3d, GlPoint4d, Program,
};

/// Webgl1.0のシングルカラーシェーダー
pub struct SingleColorShaderGl1 {
    gl: Rc<gl>,
    program: Program,
    uniform: SingleColorUniform,
    position: u32,
}

impl SingleColorShaderGl1 {
    pub const VERT: &'static str = r#"attribute vec2 position;
uniform mat3 local_mat;
uniform mat3 global_mat;

void main(void){
    // 順序に意味がある。global=移動先にlocal=変形を先に適用してから頂点情報に適用する
	gl_Position = vec4((global_mat * local_mat * vec3(position, 1.0)).xy, 0.0, 1.0);
}
"#;

    pub const FRAG: &'static str = r#"precision mediump float;

uniform vec4 u_color;

void main(void){
    gl_FragColor = u_color;
}
"#;
    pub const UNIT_RECT: [GlPoint2d; 4] = [
        GlPoint2d::new(-1.0, 1.0),
        GlPoint2d::new(1.0, 1.0),
        GlPoint2d::new(-1.0, -1.0),
        GlPoint2d::new(1.0, -1.0),
    ];

    pub fn new(gl: Rc<gl>) -> Result<Self> {
        let program = Program::new(&gl, Self::VERT, Self::FRAG)?;
        program.use_program(&gl);

        let uniform = SingleColorUniform::new(gl.clone(), &program)?;
        uniform.init();
        // 初期カラーは赤
        uniform.set_color([1.0, 0.0, 0.0, 1.0]);

        let position = gl.get_attrib_location(program.program(), "position") as u32;

        let s = Self {
            gl,
            program,
            uniform,
            position,
        };
        Ok(s)
    }

    pub fn use_program(&self) {
        self.program.use_program(&self.gl);
    }

    pub fn uniform(&self) -> &SingleColorUniform {
        &self.uniform
    }

    pub fn create_vbo(&self, data: &[GlPoint2d; 4]) -> Result<WebGlBuffer> {
        let vbo = create_buffer(&self.gl)?;
        self.gl.bind_buffer(gl::ARRAY_BUFFER, Some(&vbo));
        buffer_data(&self.gl, gl::ARRAY_BUFFER, data, gl::STATIC_DRAW);
        self.gl.enable_vertex_attrib_array(self.position);
        self.gl.vertex_attrib_pointer_with_i32(
            self.position,
            GlPoint2d::size(),
            gl::FLOAT,
            false,
            0,
            0,
        );
        Ok(vbo)
    }

    pub fn draw(&self, vbo: &WebGlBuffer) {
        self.use_program();
        self.gl.bind_buffer(gl::ARRAY_BUFFER, Some(vbo));
        self.gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
    }
}

pub struct SingleColorUniform {
    gl: Rc<gl>,
    // 色の設定
    color: WebGlUniformLocation,
    // ローカル座標変換行列。配置の前に描画内容の変形移動を行う行列。オブジェクトに対する個別の変換
    local_mat: WebGlUniformLocation,
    // 座標空間全体のうち、どこに描画するかを決める行列。カメラ行列に相当
    global_mat: WebGlUniformLocation,
}

impl SingleColorUniform {
    pub fn new(gl: Rc<gl>, program: &Program) -> Result<Self> {
        let color = uniform_location(&gl, program, "u_color")?;
        let local_mat = uniform_location(&gl, program, "local_mat")?;
        let global_mat = uniform_location(&gl, program, "global_mat")?;
        Ok(Self {
            gl,
            color,
            local_mat,
            global_mat,
        })
    }

    pub fn init(&self) {
        self.set_color([0.0, 0.0, 0.0, 0.0]);
        self.set_local_mat(nalgebra::Matrix3::identity());
        self.set_global_mat(nalgebra::Matrix3::identity());
    }

    pub fn set_color(&self, color: [f32; 4]) {
        self.gl.uniform4fv_with_f32_array(Some(&self.color), &color);
    }

    pub fn set_local_mat(&self, mat: nalgebra::Matrix3<f32>) {
        self.gl
            .uniform_matrix3fv_with_f32_array(Some(&self.local_mat), false, mat.as_slice());
    }

    pub fn set_global_mat(&self, mat: nalgebra::Matrix3<f32>) {
        self.gl
            .uniform_matrix3fv_with_f32_array(Some(&self.global_mat), false, mat.as_slice());
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
        buffer_data_f32(gl, gl::ARRAY_BUFFER, data, gl::STATIC_DRAW);
        gl.bind_buffer(gl::ARRAY_BUFFER, None);
        info!("bind_buffer {}", gl.get_error());

        let data = bytemuck::cast_slice(&Self::RECT_COORD);
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(&self.coord));
        buffer_data_f32(gl, gl::ARRAY_BUFFER, data, gl::STATIC_DRAW);
        gl.bind_buffer(gl::ARRAY_BUFFER, None);

        let data = bytemuck::cast_slice(&Self::RECT_COLOR);
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(&self.color));
        buffer_data_f32(gl, gl::ARRAY_BUFFER, data, gl::STATIC_DRAW);
        gl.bind_buffer(gl::ARRAY_BUFFER, None);

        let data = bytemuck::cast_slice(&Self::RECT_INDEX);
        gl.bind_buffer(gl::ELEMENT_ARRAY_BUFFER, Some(&self.index));
        buffer_data_f32(gl, gl::ELEMENT_ARRAY_BUFFER, data, gl::STATIC_DRAW);
        gl.bind_buffer(gl::ARRAY_BUFFER, None);

        self.index_count = Self::RECT_INDEX.len() as i32;
    }
}

pub struct TextureShader {
    gl: Rc<gl>,
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
    pub fn new(gl: Rc<gl>) -> Result<Self> {
        let program = Program::new(&gl, Self::VERT, Self::FRAG)?;
        program.use_program(&gl);
        let uniform = TextureUniform::new(gl.clone(), &program)?;
        uniform.init();
        Ok(Self {
            gl,
            program,
            uniform,
        })
    }

    pub fn uniform(&self) -> &TextureUniform {
        &self.uniform
    }

    pub fn create_vao(&self, vert: &[GlPoint2d; 4]) -> Result<Vao<TextureVd>> {
        let vao = Vao::new(&self.gl, self.program.program())?;
        vao.buffer_data(&self.gl, TextureVd::Position, vert, gl::STATIC_DRAW);
        vao.buffer_data(
            &self.gl,
            TextureVd::Coord,
            &TextureVd::FRAG,
            gl::STATIC_DRAW,
        );
        Ok(vao)
    }

    pub fn draw(&self, vao: &Vao<TextureVd>) {
        self.program.use_program(&self.gl);
        vao.bind(&self.gl);
        self.gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
    }
}

pub struct TextureUniform {
    gl: Rc<gl>,
    local_mat: WebGlUniformLocation,
    texture: WebGlUniformLocation,
}

impl TextureUniform {
    pub fn new(gl: Rc<gl>, program: &Program) -> Result<Self> {
        let local_mat = uniform_location(&gl, program, "local_mat")?;
        let texture = uniform_location(&gl, program, "u_texture")?;
        Ok(Self {
            gl,
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
