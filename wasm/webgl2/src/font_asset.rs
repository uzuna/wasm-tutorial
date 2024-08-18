//! フォントデータの埋め込み

use crate::{context::Context, error::*, font::Font};

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

pub fn load(ctx: &Context) -> Result<Font> {
    let (detail, image) = inner::load()?;

    let config = crate::texture::Texture2dConfig::new_luminance(
        detail.width() as i32,
        detail.height() as i32,
    );
    let texture = ctx.create_texture(&config, Some(image))?;
    Ok(Font::new(texture, detail))
}
