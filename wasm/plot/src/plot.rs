use std::collections::VecDeque;

use nalgebra::Vector2;
use wasm_utils::error::*;
use webgl2::{gl, GlPoint2d};

use crate::shader::PlotParams;

pub struct Plot {
    params: PlotParams,
    dot_shader: crate::shader::DotShader,
    // view_port: ViewPort,
    buffer: DataBuffer,
    y_range: (f32, f32),
}

impl Plot {
    pub fn new(gl: &gl, prop: PlotParams) -> Result<Self> {
        let dot_shader = crate::shader::DotShader::new(gl, &prop)?;
        let buffer = DataBuffer {
            time: VecDeque::new(),
            value: VecDeque::new(),
            max_len: prop.point_count,
        };
        Ok(Self {
            params: prop,
            dot_shader,
            buffer,
            y_range: (-1.0, 1.0),
        })
    }

    pub fn add_data(&mut self, gl: &gl, time: f32, value: f32) {
        if self.buffer.time.len() >= self.buffer.max_len {
            self.buffer.time.pop_front();
            self.buffer.value.pop_front();
        }
        self.buffer.time.push_back(time);
        self.buffer.value.push_back(value);
        self.dot_shader.add_data(gl, GlPoint2d::new(time, value));
    }

    pub fn update_window(&mut self, gl: &gl, current_time: f32) {
        // 画面いっぱいにプロットするために時間長をOpenGL空間の横幅2.0に合わせる
        let window_width_scale = self.params.time_window.as_secs_f32() * 0.5;

        let height = (self.y_range.1 - self.y_range.0) * 0.5;
        let y_trans = self.y_range.0 + height;
        let mat = nalgebra::Matrix3::identity()
            .append_translation(&Vector2::new(-current_time + window_width_scale, -y_trans))
            .append_nonuniform_scaling(&Vector2::new(1.0 / window_width_scale, 1.0 / height));

        self.dot_shader.use_program(gl);
        self.dot_shader.set_window_mat(gl, mat);
    }

    pub fn set_y_range(&mut self, y_range: (f32, f32)) {
        self.y_range = y_range;
    }

    pub fn set_color(&mut self, color: [f32; 4]) {
        self.params.color = color;
        self.dot_shader.set_color(color);
    }

    pub fn draw(&self, gl: &gl) {
        self.dot_shader.use_program(gl);
        self.dot_shader.draw(gl);
    }
}

// OpenGL画面のうち、描画する範囲を表す
struct ViewPort {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

// 統計値を算出するためにデータを保持する
// 統計値は表示レンジの調整などにも使う
struct DataBuffer {
    time: VecDeque<f32>,
    value: VecDeque<f32>,
    max_len: usize,
}

impl DataBuffer {
    fn min_max(&self) -> (f32, f32) {
        let min = self.value.iter().fold(f32::MAX, |a, &b| a.min(b));
        let max = self.value.iter().fold(f32::MIN, |a, &b| a.max(b));
        (min, max)
    }
}
