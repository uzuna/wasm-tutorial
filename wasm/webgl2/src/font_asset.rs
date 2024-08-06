//! フォントデータの埋め込み

use wasm_bindgen::JsError;

use crate::{
    error::*,
    font::{Font, FontTextureDetail},
    gl,
};

// フォント画像と位置情報のJSONを埋め込む
// bmpだと400KB程度だが、DSS圧縮で60KB程度になることが期待される
// 輝度情報だけなら100KB程度
const FONT_IMAGE: &[u8] = include_bytes!("../testdata/Ubuntu_Mono_64px.lum");
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

    gl.pixel_storei(gl::UNPACK_ALIGNMENT, 1);
    let level = 0;
    let width = detail.width() as i32;
    let height = detail.height() as i32;
    let border = 0;
    // TODO use compressed texture
    // reference: https://developer.mozilla.org/en-US/docs/Web/API/WEBGL_compressed_texture_s3tc#ext.compressed_rgb_s3tc_dxt1_ext
    // gl.compressed_tex_image_2d_with_u8_array(
    //     gl::TEXTURE_2D,
    //     level,
    //     web_sys::WebglCompressedTextureS3tc::COMPRESSED_RGBA_S3TC_DXT5_EXT,
    //     width,
    //     height,
    //     border,
    //     FONT_DXT1,
    // );
    // 輝度情報のみなのでLUMINANCEを使う
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        gl::TEXTURE_2D,
        level,
        gl::LUMINANCE as i32,
        width,
        height,
        border,
        gl::LUMINANCE,
        gl::UNSIGNED_BYTE,
        Some(FONT_IMAGE),
    )
    .expect("Failed to set texture image");

    Ok(Font::new(texture, detail))
}
