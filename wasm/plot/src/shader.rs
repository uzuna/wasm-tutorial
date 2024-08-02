use nalgebra::Matrix2;
use wasm_utils::error::*;
use web_sys::WebGlUniformLocation;
use webgl2::{
    gl, uniform_location,
    vertex::{Vao, VertexVbo},
    GlPoint1d, GlPoint2d, Program,
};

pub struct PlotParams {
    pub color: [f32; 4],
    pub point_size: f32,
    pub point_count: usize,
}

impl Default for PlotParams {
    fn default() -> Self {
        Self {
            color: [1.0, 0.0, 0.0, 1.0],
            point_size: 2.0,
            point_count: 100,
        }
    }
}

/// 時系列データをプロットするシェーダ
pub struct PlotShader {
    program: Program,
    window_mat: WebGlUniformLocation,
    vao: Vao,
    vertex: VertexVbo,
    color: VertexVbo,
    point_size: VertexVbo,
}

impl PlotShader {
    // x方向は時間情報なので、表示範囲の指定にwindow_matを使う
    const VERT: &'static str = r#"#version 300 es
layout(location = 0) in vec2 position;
layout(location = 1) in vec4 color;
layout(location = 2) in float point_size;

uniform mat2 window_mat;

out vec4 out_color;

void main() {
    gl_Position = vec4(window_mat * position, 0.0, 1.0);
    out_color = color;
    gl_PointSize = point_size;
}
"#;

    const FRAG: &'static str = r#"#version 300 es
precision mediump float;
in vec4 out_color;
out vec4 fragmentColor;

void main() {
    fragmentColor = out_color;
}
"#;

    const LOCATION_POSITION: u32 = 0;
    const LOCATION_COLOR: u32 = 1;
    const LOCATION_POINT_SIZE: u32 = 2;

    pub fn new(gl: &gl, param: &PlotParams) -> Result<Self> {
        let program = Program::new(gl, Self::VERT, Self::FRAG)?;

        let vao = Vao::new(gl)?;
        let vertex_data = vec![GlPoint2d::new(0.0, 0.0); param.point_count];
        let vertex = VertexVbo::new(gl, &vertex_data, Self::LOCATION_POSITION)?;

        let color_data = vec![GlPoint2d::new(param.color[0], param.color[1]); param.point_count];
        let color = VertexVbo::new(gl, &color_data, Self::LOCATION_COLOR)?;

        let point_size_data = vec![GlPoint1d::new(param.point_size); param.point_count];
        let point_size = VertexVbo::new(gl, &point_size_data, Self::LOCATION_POINT_SIZE)?;
        vao.unbind(gl);

        let window_mat = uniform_location(gl, &program, "window_mat")?;
        let s = Self {
            program,
            window_mat,
            vao,
            vertex,
            color,
            point_size,
        };
        s.init(gl);
        Ok(s)
    }

    fn init(&self, gl: &gl) {
        self.program.use_program(gl);
        self.set_window_mat(gl, Matrix2::identity());
    }

    pub fn set_window_mat(&self, gl: &gl, mat: Matrix2<f32>) {
        let ma: [[f32; 2]; 2] = mat.into();
        let mm = ma.iter().flat_map(|a| *a).collect::<Vec<_>>();
        gl.uniform_matrix2fv_with_f32_array(Some(&self.window_mat), false, &mm);
    }

    pub fn draw(&self, gl: &gl) {
        self.vao.bind(gl);
        gl.draw_arrays(gl::POINTS, 0, self.vertex.count());
        self.vao.unbind(gl);
    }
}
