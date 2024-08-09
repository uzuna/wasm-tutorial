use std::time::Duration;

use nalgebra::Matrix3;
use wasm_utils::error::*;
use web_sys::WebGlUniformLocation;
use webgl2::{
    gl, uniform_location,
    vertex::{Vao, VertexVbo},
    GlPoint1d, GlPoint2d, GlPoint4d, Program,
};

#[derive(Clone)]
pub struct PlotParams {
    pub color: [f32; 4],
    pub point_size: f32,
    pub point_count: usize,
    /// plotのX軸の表示範囲
    pub time_window: Duration,
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
    window_mat: WebGlUniformLocation,
    vao: Vao,
    vertex: VertexVbo,
    color: VertexVbo,
    point_size: VertexVbo,

    default_color: GlPoint4d,

    state: PlotState,
}

impl DotShader {
    // x方向は時間情報なので、表示範囲の指定にwindow_matを使う
    const VERT: &'static str = r#"#version 300 es
layout(location = 0) in vec2 position;
layout(location = 1) in vec4 color;
layout(location = 2) in float point_size;

uniform mat3 window_mat;

out vec4 out_color;

void main() {
    gl_Position = vec4((window_mat * vec3(position, 1.0)).xy, 0.0, 1.0);
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

        let color_data = vec![GlPoint4d::new(0.0, 0.0, 0.0, 0.0); param.point_count];
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
            default_color: GlPoint4d::from(param.color),
            state: PlotState::new(param.point_count),
        };
        s.init(gl);
        Ok(s)
    }

    fn init(&self, gl: &gl) {
        self.program.use_program(gl);
        self.set_window_mat(gl, Matrix3::identity());
    }

    pub fn use_program(&self, gl: &gl) {
        self.program.use_program(gl);
    }

    pub fn set_window_mat(&self, gl: &gl, mat: Matrix3<f32>) {
        let ma: [[f32; 3]; 3] = mat.into();
        let mm = ma.iter().flat_map(|a| *a).collect::<Vec<_>>();
        gl.uniform_matrix3fv_with_f32_array(Some(&self.window_mat), false, &mm);
    }

    pub fn add_data(&mut self, gl: &gl, p: GlPoint2d) {
        let i = self.state.next();
        self.vertex.update_vertex_sub(gl, &[p], i as i32);
        self.color
            .update_vertex_sub(gl, &[self.default_color], i as i32)
    }

    pub fn set_color(&mut self, color: [f32; 4]) {
        self.default_color = GlPoint4d::from(color);
    }

    pub fn draw(&self, gl: &gl) {
        self.vao.bind(gl);
        gl.draw_arrays(gl::POINTS, 0, self.vertex.count());
        self.vao.unbind(gl);
    }
}

/// 平面を描くデバッグ用シェーダー
pub struct PlaneShader {
    program: Program,
    window_mat: WebGlUniformLocation,
    vao: Vao,
}

impl PlaneShader {
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

    const LOCATION_POSITION: u32 = 0;
    const LOCATION_COORD: u32 = 1;

    const RECT: [GlPoint2d; 6] = [
        GlPoint2d::new(-1.0, 1.0),
        GlPoint2d::new(-1.0, -1.0),
        GlPoint2d::new(1.0, 1.0),
        GlPoint2d::new(1.0, 1.0),
        GlPoint2d::new(-1.0, -1.0),
        GlPoint2d::new(1.0, -1.0),
    ];

    const RECT_COORD: [GlPoint2d; 6] = [
        GlPoint2d::new(0.0, 0.0),
        GlPoint2d::new(0.0, 1.0),
        GlPoint2d::new(1.0, 0.0),
        GlPoint2d::new(1.0, 0.0),
        GlPoint2d::new(0.0, 1.0),
        GlPoint2d::new(1.0, 1.0),
    ];

    pub fn new(gl: &gl) -> Result<Self> {
        let program = Program::new(gl, Self::VERT, Self::FRAG)?;

        let vao = Vao::new(gl)?;
        let _vertex = VertexVbo::new(gl, &Self::RECT, Self::LOCATION_POSITION)?;
        let _coord = VertexVbo::new(gl, &Self::RECT_COORD, Self::LOCATION_COORD)?;
        vao.unbind(gl);

        let window_mat = uniform_location(gl, &program, "window_mat")?;
        Ok(Self {
            program,
            window_mat,
            vao,
        })
    }

    pub fn set_mat(&self, gl: &gl, mat: Matrix3<f32>) {
        self.program.use_program(gl);
        let ma: [[f32; 3]; 3] = mat.into();
        let mm = ma.iter().flat_map(|a| *a).collect::<Vec<_>>();
        gl.uniform_matrix3fv_with_f32_array(Some(&self.window_mat), false, &mm);
    }

    pub fn draw(&self, gl: &gl, texture: &web_sys::WebGlTexture) {
        self.program.use_program(gl);
        gl.active_texture(gl::TEXTURE0);
        gl.bind_texture(gl::TEXTURE_2D, Some(texture));
        self.vao.bind(gl);
        gl.draw_arrays(gl::TRIANGLES, 0, 6);
        self.vao.unbind(gl);
    }
}
