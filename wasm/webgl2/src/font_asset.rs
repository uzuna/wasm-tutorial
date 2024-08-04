//! フォントデータの埋め込み

use wasm_bindgen::JsError;

use crate::{
    error::*,
    font::{Font, FontTextureDetail},
    gl,
};

const FONT_DXT1: &[u8] = include_bytes!("../testdata/Ubuntu_Mono_64px.dxt1");
const FONT_JSON: &str = include_str!("../testdata/Ubuntu_Mono_64px.json");

pub fn load(gl: &gl) -> Result<Font> {
    let detail: FontTextureDetail = serde_json::from_str(FONT_JSON).unwrap();
    let texture = gl
        .create_texture()
        .ok_or(JsError::new("Failed to create texture"))?;
    gl.bind_texture(gl::TEXTURE_2D, Some(&texture));
    gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
    gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
    gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

    gl.pixel_storei(gl::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1);
    // gl.pixel_storei(gl::UNPACK_COLORSPACE_CONVERSION_WEBGL, gl::NONE as i32);
    let level = 0;
    let internal_format = gl::RGBA;
    let width = 1;
    let height = 1;
    let border = 0;
    let src_format = gl::RGBA;
    let src_type = gl::UNSIGNED_BYTE;
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        gl::TEXTURE_2D,
        level,
        internal_format as i32,
        width,
        height,
        border,
        src_format,
        src_type,
        Some(FONT_DXT1),
    )
    .map_err(|_| JsError::new("Failed to set texture image"))?;

    Ok(Font::new(texture, detail))
}
