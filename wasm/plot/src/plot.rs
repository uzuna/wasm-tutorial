use std::{collections::VecDeque, rc::Rc};

use nalgebra::Vector2;
use wasm_utils::error::*;
use webgl2::{gl, GlPoint2d};

use crate::{shader::PlotParams, viewport::LocalView};

/// チャート全体を描画するための構造体
pub struct Chart {
    // 画面全体のうちこのチャートが使っても良い領域
    localview: LocalView,
    // データ系列
    series: Vec<SeriesRenderer>,
    // データ系列のラベル
    labels: Vec<String>,
}

impl Chart {
    pub fn new(localview: LocalView) -> Result<Self> {
        Ok(Self {
            localview,
            series: Vec::new(),
            labels: Vec::new(),
        })
    }

    pub fn add_series(&mut self, gl: Rc<gl>, prop: PlotParams, label: &str) -> Result<usize> {
        let mut series = SeriesRenderer::new(gl.clone(), prop)?;
        // 表示スケールの設定
        let local_mat = self.localview.local_mat();
        series.set_local_mat(&gl, local_mat);

        let index = self.series.len();
        self.series.push(series);
        self.labels.push(label.to_string());
        Ok(index)
    }

    pub fn add_data(&mut self, gl: &gl, index: usize, time: f32, value: f32) {
        if let Some(series) = self.series.get_mut(index) {
            series.add_data(gl, time, value);
        }
    }

    pub fn draw(&mut self, gl: &gl, current_time: f32) {
        self.localview.scissor(gl);
        for series in self.series.iter_mut() {
            series.update_window(gl, current_time);
            series.draw(gl);
        }
    }
}

/// 1データ系列を描画するための構造体
pub struct SeriesRenderer {
    // 描画パラメータ
    params: PlotParams,
    // ドット描画用のシェーダ。描画メモリも持つ
    dot_shader: crate::shader::DotShader,
    // 描画とは別にデータを保持
    buffer: DataBuffer,
    // 表示範囲確認
    plane_shader: crate::shader::PlaneShader,
}

impl SeriesRenderer {
    pub fn new(gl: Rc<gl>, prop: PlotParams) -> Result<Self> {
        let dot_shader = crate::shader::DotShader::new(&gl, &prop)?;
        let buffer = DataBuffer {
            time: VecDeque::new(),
            value: VecDeque::new(),
            max_len: prop.point_count,
        };
        let plane_shader = crate::shader::PlaneShader::new(gl, [0.5, 0.5, 0.5, 1.0])?;
        Ok(Self {
            params: prop,
            dot_shader,
            buffer,
            plane_shader,
        })
    }

    /// ローカル座標変換行列を設定。
    pub fn set_local_mat(&mut self, gl: &gl, mat: nalgebra::Matrix3<f32>) {
        self.dot_shader.use_program(gl);
        self.dot_shader.uniform().local_mat(gl, mat);
        self.plane_shader.use_program();
        self.plane_shader.uniform().local_mat(gl, mat);
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

        let height = (self.params.y_range.1 - self.params.y_range.0) * 0.5;
        let y_trans = self.params.y_range.0 + height;

        // 新しいプロットの位置はどのように決定する?
        // OpenGL Unit範囲に表示すると考えたときに、この座標はどの程度動かせば良い?
        let mat = nalgebra::Matrix3::identity()
            .append_translation(&Vector2::new(-current_time + window_width_scale, -y_trans))
            .append_nonuniform_scaling(&Vector2::new(1.0 / window_width_scale, 1.0 / height));

        self.dot_shader.use_program(gl);
        self.dot_shader.uniform().plot_mat(gl, mat);
    }

    pub fn set_y_range(&mut self, y_range: (f32, f32)) {
        self.params.y_range = y_range;
    }

    pub fn set_color(&mut self, color: [f32; 4]) {
        self.params.color = color;
        self.dot_shader.set_color(color);
    }

    pub fn draw(&self, gl: &gl) {
        self.dot_shader.draw(gl);
        self.plane_shader.draw();
    }
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
