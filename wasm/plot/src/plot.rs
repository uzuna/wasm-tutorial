use std::{collections::VecDeque, rc::Rc};

use nalgebra::Vector2;
use wasm_utils::error::*;
use webgl2::{context::Context, gl, viewport::LocalView, GlPoint2d};

use crate::shader::PlotParams;

/// チャート全体を描画するための構造体
pub struct Chart {
    gl: Rc<gl>,
    // 画面全体のうちこのチャートが使っても良い領域
    localview: LocalView,
    // データ系列
    series: Vec<SeriesRenderer>,
    // データ系列のラベル
    labels: Vec<String>,
}

impl Chart {
    pub fn new(ctx: &Context, localview: LocalView) -> Result<Self> {
        Ok(Self {
            gl: ctx.gl().clone(),
            localview,
            series: Vec::new(),
            labels: Vec::new(),
        })
    }

    pub fn add_series(&mut self, ctx: &Context, prop: PlotParams, label: &str) -> Result<usize> {
        let mut series = SeriesRenderer::new(ctx, prop)?;
        // 表示スケールの設定
        let local_mat = self.localview.local_mat();
        series.set_local_mat(local_mat);

        let index = self.series.len();
        self.series.push(series);
        self.labels.push(label.to_string());
        Ok(index)
    }

    pub fn add_data(&mut self, index: usize, time: f32, value: f32) {
        if let Some(series) = self.series.get_mut(index) {
            series.add_data(time, value);
        }
    }

    pub fn draw(&mut self, current_time: f32) {
        self.localview.scissor(&self.gl);
        for series in self.series.iter_mut() {
            series.update_window(current_time);
            series.draw();
        }
    }

    pub fn series(&self, index: usize) -> Option<&SeriesRenderer> {
        self.series.get(index)
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
    pub fn new(ctx: &Context, prop: PlotParams) -> Result<Self> {
        let dot_shader = crate::shader::DotShader::new(ctx, &prop)?;
        let buffer = DataBuffer {
            time: VecDeque::new(),
            value: VecDeque::new(),
            max_len: prop.point_count,
        };
        let plane_shader = crate::shader::PlaneShader::new(ctx, [0.5, 0.5, 0.5, 1.0])?;
        Ok(Self {
            params: prop,
            dot_shader,
            buffer,
            plane_shader,
        })
    }

    /// ローカル座標変換行列を設定。
    pub fn set_local_mat(&mut self, mat: nalgebra::Matrix3<f32>) {
        self.dot_shader.use_program();
        self.dot_shader.uniform().local_mat(mat);
        self.plane_shader.use_program();
        self.plane_shader.uniform().local_mat(mat);
    }

    pub fn add_data(&mut self, time: f32, value: f32) {
        if self.buffer.time.len() >= self.buffer.max_len {
            self.buffer.time.pop_front();
            self.buffer.value.pop_front();
        }
        self.buffer.time.push_back(time);
        self.buffer.value.push_back(value);
        self.dot_shader.add_data(GlPoint2d::new(time, value));
    }

    pub fn update_window(&mut self, current_time: f32) {
        // 画面いっぱいにプロットするために時間長をOpenGL空間の横幅2.0に合わせる
        let window_width_scale = self.params.time_window.as_secs_f32() * 0.5;

        let height = (self.params.y_range.1 - self.params.y_range.0) * 0.5;
        let y_trans = self.params.y_range.0 + height;

        // 新しいプロットの位置はどのように決定する?
        // OpenGL Unit範囲に表示すると考えたときに、この座標はどの程度動かせば良い?
        let mat = nalgebra::Matrix3::identity()
            .append_translation(&Vector2::new(-current_time + window_width_scale, -y_trans))
            .append_nonuniform_scaling(&Vector2::new(1.0 / window_width_scale, 1.0 / height));

        self.dot_shader.use_program();
        self.dot_shader.uniform().plot_mat(mat);
    }

    pub fn draw(&self) {
        self.dot_shader.draw();
        self.plane_shader.draw();
    }

    pub fn last(&self) -> Option<(f32, f32)> {
        self.buffer
            .time
            .back()
            .map(|&t| (t, *self.buffer.value.back().unwrap()))
    }
}

// 統計値を算出するためにデータを保持する
// 統計値は表示レンジの調整などにも使う
struct DataBuffer {
    time: VecDeque<f32>,
    value: VecDeque<f32>,
    max_len: usize,
}
