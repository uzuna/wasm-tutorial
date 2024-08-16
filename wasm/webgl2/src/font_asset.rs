//! フォントデータの埋め込み

use wasm_bindgen::JsError;
use web_sys::WebGlTexture;

use crate::{error::*, font::Font, gl};

#[cfg(not(feature = "font-asset-compress"))]
mod inner {
    use crate::{error::*, font::FontTextureDetail};
    // フォント画像と位置情報のJSONを埋め込む
    // bmpだと400KB程度だが、DSS圧縮で60KB程度になることが期待される
    // 輝度情報だけなら100KB程度
    const FONT_IMAGE: &[u8] = include_bytes!("../testdata/Ubuntu_Mono_64px.lum");
    const FONT_JSON: &str = include_str!("../testdata/Ubuntu_Mono_64px.json");

    pub(crate) fn load() -> Result<(FontTextureDetail, &'static [u8])> {
        let detail: FontTextureDetail = serde_json::from_str(FONT_JSON)?;
        Ok((detail, FONT_IMAGE))
    }
}

// zstd圧縮を使う。
// ただし、200KB程度の画像データの場合はほとんど圧縮効果がなく、zstd実装分増えた
// plotバイナリで比較。261763B -> 264052B
#[cfg(feature = "font-asset-compress")]
mod inner {
    use crate::{error::*, font::FontTextureDetail};
    pub(crate) fn load() -> Result<(FontTextureDetail, Vec<u8>)> {
        let detail: FontTextureDetail = serde_json::from_slice(
            &include_bytes_zstd::include_bytes_zstd!("testdata/Ubuntu_Mono_64px.json", 19),
        )?;
        Ok((
            detail,
            include_bytes_zstd::include_bytes_zstd!("testdata/Ubuntu_Mono_64px.lum", 19),
        ))
    }
}

pub fn load(gl: &gl) -> Result<Font> {
    let (detail, image) = inner::load()?;

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
        Some(image),
    )
    .expect("Failed to set texture image");

    Ok(Font::new(texture, detail))
}

/// 1x1pxの色のテクスチャを作成する
pub fn color_texture(gl: &gl, color: [u8; 4]) -> WebGlTexture {
    let texture = gl.create_texture().expect("Failed to create texture");
    gl.bind_texture(gl::TEXTURE_2D, Some(&texture));
    gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
    gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
    gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        gl::TEXTURE_2D,
        0,
        gl::RGBA as i32,
        1,
        1,
        0,
        gl::RGBA,
        gl::UNSIGNED_BYTE,
        Some(&color),
    )
    .expect("Failed to set texture image");
    texture
}
