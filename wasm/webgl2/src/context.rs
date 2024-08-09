use crate::error::Result;
use wasm_bindgen::*;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext as gl};

pub const COLOR_BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WebGL2ContextOption {
    premultiplied_alpha: bool,
    alpha: bool,
}

impl WebGL2ContextOption {
    const DEFAULT: Self = Self {
        premultiplied_alpha: false,
        alpha: false,
    };
}

pub fn get_webgl2_context(canvas: &HtmlCanvasElement, color: [f32; 4]) -> Result<gl> {
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
    // テクスチャの表面だけを描画する
    gl.enable(gl::CULL_FACE);
    // アルファブレンドを有効にする
    gl.enable(gl::BLEND);
    // アルファブレンドは、srcのアルファを使ってdstの値を割り引いてブレンドする
    gl.blend_func_separate(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA, gl::ONE, gl::ONE);

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
