use std::{rc::Rc, time::Duration};

use nalgebra::Matrix3;
use wasm_utils::error::*;
use web_sys::WebGlUniformLocation;
use webgl2::{
    context::Context,
    gl,
    program::Program,
    vertex::{Vao, VaoDefine},
    GlPoint1d, GlPoint2d, GlPoint4d,
};

#[derive(Clone)]
pub struct PlotParams {
    /// 点の色
    pub color: [f32; 4],
    /// 点の大きさ
    pub point_size: f32,
    /// プロットする点の数
    pub point_count: usize,
    /// plotのX軸の表示範囲
    pub time_window: Duration,
    /// plotのY軸の表示範囲
    pub y_range: (f32, f32),
}

impl PlotParams {
    pub const DEFAULT_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
    pub const DEFAULT_POINT_SIZE: f32 = 4.0;

    pub fn new(time_window: Duration, point_per_seconds: u32, y_range: (f32, f32)) -> Self {
        let point_count = (time_window.as_secs() as u32 * point_per_seconds) as usize;
        Self {
            color: Self::DEFAULT_COLOR,
            point_size: Self::DEFAULT_POINT_SIZE,
            point_count,
            time_window,
            y_range,
        }
    }

    pub fn point_per_seconds(&self) -> f32 {
        (self.point_count as f32) / self.time_window.as_secs() as f32
    }
}

impl Default for PlotParams {
    fn default() -> Self {
        Self {
            color: Self::DEFAULT_COLOR,
            point_size: Self::DEFAULT_POINT_SIZE,
            point_count: 100,
            time_window: Duration::from_secs(10),
            y_range: (-1.0, 1.0),
        }
    }
}

struct PlotState {
    len: usize,
    current_index: usize,
}

impl PlotState {
    fn new(len: usize) -> Self {
        Self {
            len,
            current_index: 0,
        }
    }

    fn next(&mut self) -> usize {
        let i = self.current_index;
        self.current_index = (self.current_index + 1) % self.len;
        i
    }
}

/// 時系列データをプロットするシェーダ
pub struct DotShader {
    program: Program,
    uniform: DotUniform,
    vao: Vao<DotVertexDefine>,
    vertex_len: i32,
    default_color: GlPoint4d,
    state: PlotState,
}

impl DotShader {
    // x方向は時間情報なので、表示範囲の指定にwindow_matを使う
    const VERT: &'static str = r#"#version 300 es
layout(location = 0) in vec2 position;
layout(location = 1) in vec4 color;
layout(location = 2) in float point_size;

// プロット表示の位置や大きさを調整するための行列
uniform mat3 local_mat;
// プロットはX方向にずれていくので表示位置を調整するための行列
uniform mat3 plot_mat;

out vec4 out_color;

void main() {
    mat3 m = local_mat * plot_mat;
    gl_Position = vec4((m * vec3(position.xy, 1.0)).xy, 0.0, 1.0);
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

    pub fn new(ctx: &Context, param: &PlotParams) -> Result<Self> {
        let program = ctx.program(Self::VERT, Self::FRAG)?;
        let mut vao = program.create_vao()?;
        let vertex_data = vec![GlPoint2d::new(0.0, 0.0); param.point_count];
        vao.buffer_data(DotVertexDefine::Position, &vertex_data, gl::DYNAMIC_DRAW);

        let color_data = vec![GlPoint4d::new(0.0, 0.0, 0.0, 0.0); param.point_count];
        vao.buffer_data(DotVertexDefine::Color, &color_data, gl::DYNAMIC_DRAW);

        let point_size_data = vec![GlPoint1d::new(param.point_size); param.point_count];
        vao.buffer_data(
            DotVertexDefine::PointSize,
            &point_size_data,
            gl::DYNAMIC_DRAW,
        );

        let uniform = DotUniform::new(&program)?;
        uniform.init();

        Ok(Self {
            program,
            uniform,
            vao,
            vertex_len: param.point_count as i32,
            default_color: GlPoint4d::from(param.color),
            state: PlotState::new(param.point_count),
        })
    }

    pub fn use_program(&self) {
        self.program.use_program();
    }

    pub fn uniform(&self) -> &DotUniform {
        &self.uniform
    }

    pub fn add_data(&mut self, p: GlPoint2d) {
        let i = self.state.next();
        self.vao
            .buffer_sub_data(DotVertexDefine::Position, &[p], i as i32);
        self.vao
            .buffer_sub_data(DotVertexDefine::Color, &[self.default_color], i as i32);
    }

    pub fn draw(&self) {
        self.vao.bind();
        self.program
            .gl()
            .draw_arrays(gl::POINTS, 0, self.vertex_len);
        self.vao.unbind();
    }
}

#[derive(Debug, PartialEq)]
pub enum DotVertexDefine {
    Position,
    Color,
    PointSize,
}

impl VaoDefine for DotVertexDefine {
    fn iter() -> std::slice::Iter<'static, Self> {
        use DotVertexDefine::*;
        static VAO: [DotVertexDefine; 3] = [Position, Color, PointSize];
        VAO.iter()
    }

    fn name(&self) -> &'static str {
        use DotVertexDefine::*;
        match self {
            Position => "position",
            Color => "color",
            PointSize => "point_size",
        }
    }

    fn size_of(&self) -> i32 {
        use DotVertexDefine::*;
        match self {
            Position => 2,
            Color => 4,
            PointSize => 1,
        }
    }
}

pub struct DotUniform {
    gl: Rc<gl>,
    pub local_mat: WebGlUniformLocation,
    pub plot_mat: WebGlUniformLocation,
}

impl DotUniform {
    pub fn new(program: &Program) -> Result<Self> {
        let local_mat = program.uniform_location("local_mat")?;
        let plot_mat = program.uniform_location("plot_mat")?;
        Ok(Self {
            gl: program.gl().clone(),
            local_mat,
            plot_mat,
        })
    }

    pub fn init(&self) {
        self.local_mat(Matrix3::identity());
        self.plot_mat(Matrix3::identity());
    }

    pub fn local_mat(&self, mat: Matrix3<f32>) {
        self.gl
            .uniform_matrix3fv_with_f32_array(Some(&self.local_mat), false, mat.as_slice());
    }

    pub fn plot_mat(&self, mat: Matrix3<f32>) {
        self.gl
            .uniform_matrix3fv_with_f32_array(Some(&self.plot_mat), false, mat.as_slice());
    }
}

#[derive(Debug, PartialEq)]
pub enum PlaneVertexDefine {
    Position,
}

impl VaoDefine for PlaneVertexDefine {
    fn iter() -> std::slice::Iter<'static, Self> {
        use PlaneVertexDefine::*;
        static VAO: [PlaneVertexDefine; 1] = [Position];
        VAO.iter()
    }

    fn name(&self) -> &'static str {
        use PlaneVertexDefine::*;
        match self {
            Position => "position",
        }
    }

    fn size_of(&self) -> i32 {
        use PlaneVertexDefine::*;
        match self {
            Position => 2,
        }
    }
}

pub struct PlaneShader {
    program: Program,
    uniform: PlaneUniform,
    vao: Vao<PlaneVertexDefine>,
    vertex_len: i32,
}

impl PlaneShader {
    // x方向は時間情報なので、表示範囲の指定にwindow_matを使う
    const VERT: &'static str = r#"#version 300 es
layout(location = 0) in vec2 position;

uniform mat3 local_mat;

void main() {
    gl_Position = vec4((local_mat * vec3(position.xy, 1.0)).xy, 0.0, 1.0);
}
"#;

    const FRAG: &'static str = r#"#version 300 es
precision mediump float;

uniform vec4 u_color;
out vec4 fragmentColor;

void main() {
    fragmentColor = u_color;
}
"#;
    const TRIANGLE: [GlPoint2d; 4] = [
        GlPoint2d::new(-1.0, -1.0),
        GlPoint2d::new(1.0, -1.0),
        GlPoint2d::new(-1.0, 1.0),
        GlPoint2d::new(1.0, 1.0),
    ];

    pub fn new(ctx: &Context, color: [f32; 4]) -> Result<Self> {
        let program = ctx.program(Self::VERT, Self::FRAG)?;
        program.use_program();

        let mut vao = program.create_vao()?;
        vao.buffer_data(
            PlaneVertexDefine::Position,
            &Self::TRIANGLE,
            gl::DYNAMIC_DRAW,
        );

        let uniform = PlaneUniform::new(&program)?;
        uniform.init();
        uniform.color(color);

        Ok(Self {
            program,
            uniform,
            vao,
            vertex_len: Self::TRIANGLE.len() as i32,
        })
    }

    pub fn use_program(&self) {
        self.program.use_program();
    }

    pub fn uniform(&self) -> &PlaneUniform {
        &self.uniform
    }

    pub fn draw(&self) {
        self.use_program();
        self.vao.bind();
        self.program
            .gl()
            .draw_arrays(gl::LINE_LOOP, 0, self.vertex_len);
        self.vao.unbind();
    }
}

pub struct PlaneUniform {
    gl: Rc<gl>,
    pub local_mat: WebGlUniformLocation,
    pub color: WebGlUniformLocation,
}

impl PlaneUniform {
    pub fn new(program: &Program) -> Result<Self> {
        let local_mat = program.uniform_location("local_mat")?;
        let color = program.uniform_location("u_color")?;
        let gl = program.gl().clone();
        Ok(Self {
            gl,
            local_mat,
            color,
        })
    }

    pub fn init(&self) {
        self.local_mat(Matrix3::identity());
    }

    pub fn color(&self, color: [f32; 4]) {
        self.gl.uniform4fv_with_f32_array(Some(&self.color), &color);
    }

    pub fn local_mat(&self, mat: Matrix3<f32>) {
        self.gl
            .uniform_matrix3fv_with_f32_array(Some(&self.local_mat), false, mat.as_slice());
    }
}
