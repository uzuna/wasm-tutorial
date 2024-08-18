use std::rc::Rc;

use wasm_utils::error::*;
use web_sys::{WebGlBuffer, WebGlUniformLocation};
use webgl2::{
    context::Context,
    gl,
    program::Program,
    vertex::{buffer_data, create_buffer},
    GlPoint, GlPoint2d,
};

/// Webgl1.0のシングルカラーシェーダー
pub struct SingleColorShaderGl1 {
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

    pub fn new(ctx: &Context) -> Result<Self> {
        let program = ctx.program(Self::VERT, Self::FRAG)?;
        program.use_program();

        let uniform = SingleColorUniform::new(&program)?;
        uniform.init();
        // 初期カラーは赤
        uniform.set_color([1.0, 0.0, 0.0, 1.0]);

        let position = ctx.gl().get_attrib_location(program.program(), "position") as u32;

        let s = Self {
            program,
            uniform,
            position,
        };
        Ok(s)
    }

    pub fn use_program(&self) {
        self.program.use_program();
    }

    pub fn uniform(&self) -> &SingleColorUniform {
        &self.uniform
    }

    pub fn create_vbo(&self, data: &[GlPoint2d; 4]) -> Result<WebGlBuffer> {
        let gl = self.program.gl();
        let vbo = create_buffer(gl)?;
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(&vbo));
        buffer_data(gl, gl::ARRAY_BUFFER, data, gl::STATIC_DRAW);
        gl.enable_vertex_attrib_array(self.position);
        gl.vertex_attrib_pointer_with_i32(self.position, GlPoint2d::size(), gl::FLOAT, false, 0, 0);
        Ok(vbo)
    }

    pub fn draw(&self, vbo: &WebGlBuffer) {
        self.use_program();
        let gl = &self.program.gl();
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(vbo));
        gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
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
    pub fn new(program: &Program) -> Result<Self> {
        let color = program.uniform_location("u_color")?;
        let local_mat = program.uniform_location("local_mat")?;
        let global_mat = program.uniform_location("global_mat")?;
        Ok(Self {
            gl: program.gl().clone(),
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
