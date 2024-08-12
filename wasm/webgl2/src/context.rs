use crate::{blend::BlendMode, error::Result};
use wasm_bindgen::*;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext as gl};

pub const COLOR_BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// refer: https://developer.mozilla.org/en-US/docs/Web/API/HTMLCanvasElement/getContext
/// jsでの定義似合わせtえcamelCaseで定義
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WebGL2ContextOption {
    // 代表的なアルファブレンディング計算式は `scr_color * scr_alpha + dst_color * (1 - scr_alpha)` となる
    // 計算コストの重い乗算が2回は出現してしまう。
    // この計算回数を減らすために、事前にアルファ値を乗算してメモリに保持するのがこのオプション
    premultiplied_alpha: bool,
    // 表示するcanvasタグに設定された背景色を透明にするかどうか
    alpha: bool,
    // 画像のアンチエイリアスを有効にするかどうか
    antialias: bool,
    // 描画バッファに16bitの深度バッファが必要であることを示す
    depth: bool,
    // 描画バッファに8bitのステンシルバッファが必要であることを示す
    stencil: bool,
}

impl WebGL2ContextOption {
    const DEFAULT: Self = Self {
        // alphaを保持してほしいのでfalse
        premultiplied_alpha: false,
        // バックバッファがアルファを含む場合、Canvasの色がでてしまうため、アルファを無効にする
        // ONE_MINUS_DST_COLORなどDSTを使うブレンドをすると、アルファを無視して合成してしまうので注意
        alpha: false,
        antialias: true,
        depth: true,
        stencil: true,
    };
}

pub fn get_context(canvas: &HtmlCanvasElement, color: [f32; 4]) -> Result<gl> {
    use wasm_bindgen::JsCast;
    let options = serde_wasm_bindgen::to_value(&WebGL2ContextOption::DEFAULT)?;

    let gl = canvas
        .get_context_with_context_options("webgl2", &options)
        .map_err(|_| JsError::new("Failed to get_context(webgl2)"))?
        .ok_or(JsError::new("Failed to get WebGl2RenderingContext Object"))?
        .dyn_into::<gl>()
        .map_err(|_| JsError::new("Failed to cast to WebGl2RenderingContext"))?;

    // 手前にあるものだけを描画して負荷を下げる
    gl.enable(gl::DEPTH_TEST);
    // 震度バッファ評価方法。デフォルトのGL_LESSは入力値が"未満"の場合にパスするので、平面表示が先勝ちになる
    // 後勝ちにするためには、入力値が"以下"の場合にパスするLEQUALを使う
    gl.depth_func(gl::LEQUAL);
    // テクスチャの表面だけを描画する
    // gl.enable(gl::CULL_FACE);
    // アルファブレンドを有効にする
    BlendMode::Alpha.enable(&gl);

    gl_clear_color(&gl, color);
    gl.clear_depth(1.0);
    gl.clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    Ok(gl)
}

#[inline]
pub fn gl_clear_color(gl: &gl, color: [f32; 4]) {
    gl.clear_color(color[0], color[1], color[2], color[3]);
    gl.clear(gl::COLOR_BUFFER_BIT);
}
