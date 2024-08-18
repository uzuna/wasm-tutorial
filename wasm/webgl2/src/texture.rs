//! テクスチャの作成と管理を行うモジュールです。

use std::{rc::Rc, sync::atomic::AtomicU64, sync::atomic::Ordering::Relaxed};

use wasm_bindgen::JsError;
use web_sys::WebGlTexture;

use crate::{
    context::{Context, ContextInner},
    error::Result,
    gl,
};

/// テクスチャの設定
pub struct Texture2dConfig {
    /// 画像サイズ
    pub width: i32,
    pub height: i32,
    /// テクスチャの内部フォーマット
    pub inner_format: i32,
    /// 渡す画像のフォーマット
    pub format: u32,
    /// テクスチャのフィルター設定
    pub filter: TextureFilter,
}

impl Texture2dConfig {
    pub fn new_rgba(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            inner_format: gl::RGBA as i32,
            format: gl::RGBA,
            filter: TextureFilter::default(),
        }
    }

    pub fn new_luminance(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            inner_format: gl::LUMINANCE as i32,
            format: gl::LUMINANCE,
            filter: TextureFilter::default(),
        }
    }

    pub fn create_from_byte(&self, gl: &gl, body: Option<&[u8]>) -> Result<WebGlTexture> {
        let texture = create_texture_inner(gl)?;
        gl.bind_texture(gl::TEXTURE_2D, Some(&texture));
        self.filter.apply(gl);
        // フォーマットが4の倍数でない場合は読み出しのアライメントを1に設定する
        if Self::format_sizeof(self.inner_format as u32) != 4 {
            gl.pixel_storei(gl::UNPACK_ALIGNMENT, 1);
        }
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            gl::TEXTURE_2D,
            0,
            self.inner_format,
            self.width,
            self.height,
            0,
            self.format,
            gl::UNSIGNED_BYTE,
            body,
        )
        .map_err(|e| {
            JsError::new(&format!(
                "Failed to call texImage2D from bytes: {:?}",
                e.as_string()
            ))
        })?;
        Ok(texture)
    }

    // フォーマットに応じたバイト数を返す
    fn format_sizeof(inner_format: u32) -> u64 {
        match inner_format {
            gl::RGB => 3,
            gl::RGBA => 4,
            gl::LUMINANCE => 1,
            gl::LUMINANCE_ALPHA => 2,
            // 他のフォーマットは仮で4とする
            _ => 4,
        }
    }

    /// テクスチャの保持に必要なバイト数を推定する
    pub fn bytes(&self) -> u64 {
        self.width as u64 * self.height as u64 * Self::format_sizeof(self.inner_format as u32)
    }
}

/// テクスチャの折返しや拡大縮小に関する設定。
pub struct TextureFilter {
    pub min: i32,
    pub mag: i32,
    pub wrap_s: i32,
    pub wrap_t: i32,
}

impl Default for TextureFilter {
    fn default() -> Self {
        Self {
            min: gl::LINEAR as i32,
            mag: gl::LINEAR as i32,
            wrap_s: gl::CLAMP_TO_EDGE as i32,
            wrap_t: gl::CLAMP_TO_EDGE as i32,
        }
    }
}

impl TextureFilter {
    fn apply(&self, gl: &gl) {
        gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, self.min);
        gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, self.mag);
        gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, self.wrap_s);
        gl.tex_parameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, self.wrap_t);
    }
}

/// 1x1pxの色のテクスチャを作成する
pub fn color_texture(gl: &gl, color: [u8; 4]) -> Result<WebGlTexture> {
    let config = Texture2dConfig::new_rgba(1, 1);
    config.create_from_byte(gl, Some(&color))
}

/// RGBAデータからテクスチャを作成する
pub fn create_texture(
    gl: &gl,
    config: &Texture2dConfig,
    body: Option<&[u8]>,
) -> Result<WebGlTexture> {
    config.create_from_byte(gl, body)
}

/// 画像要素からテクスチャを作成する
pub fn create_texture_image_element(
    gl: &gl,
    filter: &TextureFilter,
    element: &web_sys::HtmlImageElement,
) -> Result<WebGlTexture> {
    let texture = create_texture_inner(gl)?;
    gl.bind_texture(gl::TEXTURE_2D, Some(&texture));
    filter.apply(gl);
    gl.tex_image_2d_with_u32_and_u32_and_html_image_element(
        gl::TEXTURE_2D,
        0,
        gl::RGBA as i32,
        gl::RGBA,
        gl::UNSIGNED_BYTE,
        element,
    )
    .map_err(|e| {
        JsError::new(&format!(
            "Failed to call texImage2D from element: {:?}",
            e.as_string()
        ))
    })?;
    Ok(texture)
}

/// 空のテクスチャを作成する
pub fn crate_blank_texture(gl: &gl) -> Result<WebGlTexture> {
    let texture = create_texture_inner(gl)?;
    gl.bind_texture(gl::TEXTURE_2D, Some(&texture));
    TextureFilter::default().apply(gl);
    Ok(texture)
}

/// テクスチャの画像データを更新する
pub fn update_texture_image_element(
    gl: &gl,
    texture: &WebGlTexture,
    element: &web_sys::HtmlImageElement,
) {
    gl.bind_texture(gl::TEXTURE_2D, Some(texture));
    gl.tex_image_2d_with_u32_and_u32_and_html_image_element(
        gl::TEXTURE_2D,
        0,
        gl::RGBA as i32,
        gl::RGBA,
        gl::UNSIGNED_BYTE,
        element,
    )
    .expect("Failed to set texture image");
}

fn create_texture_inner(gl: &gl) -> Result<WebGlTexture> {
    gl.create_texture()
        .ok_or(JsError::new("Failed to create texture"))
}

impl Context {
    /// 画像のバイト列からテクスチャを作成する
    pub fn create_texture(&self, config: &Texture2dConfig, body: Option<&[u8]>) -> Result<Texture> {
        Texture::new_from_bytes(self.ctx.clone(), config, body)
    }

    /// 空のテクスチャを作成する
    pub fn create_blank_texture(&self) -> Result<Texture> {
        Texture::new_from_bytes(self.ctx.clone(), &Texture2dConfig::new_rgba(1, 1), None)
    }

    /// 画像要素からテクスチャを作成する
    pub fn create_texture_image_element(
        &self,
        filter: &TextureFilter,
        element: &web_sys::HtmlImageElement,
    ) -> Result<Texture> {
        Texture::new_from_image_element(self.ctx.clone(), filter, element)
    }
}

struct TextureInner {
    ctx: Rc<ContextInner>,
    texture: Rc<WebGlTexture>,
    bytes: AtomicU64,
}

impl TextureInner {
    fn new(ctx: Rc<ContextInner>, texture: WebGlTexture, bytes: u64) -> Result<Self> {
        let texture = Rc::new(texture);
        let bytes = AtomicU64::new(bytes);
        #[cfg(feature = "metrics")]
        {
            let texture = &ctx.metrics().texture;
            texture.inc_texture(1);
            texture.inc_bytes(bytes.load(Relaxed));
        }
        Ok(Self {
            ctx,
            texture,
            bytes,
        })
    }

    fn bind(&self) {
        self.ctx
            .gl()
            .bind_texture(gl::TEXTURE_2D, Some(&self.texture));
    }

    fn update_bytes(&self, bytes: u64) {
        let _old = self.bytes.swap(bytes, Relaxed);
        #[cfg(feature = "metrics")]
        {
            let texture = &self.ctx.metrics().texture;
            texture.sub_bytes(_old);
            texture.inc_bytes(bytes);
        }
    }
}

impl Drop for TextureInner {
    fn drop(&mut self) {
        self.ctx.gl().delete_texture(Some(&self.texture));
        #[cfg(feature = "metrics")]
        {
            let texture = &self.ctx.metrics().texture;
            texture.sub_texture(1);
            texture.sub_bytes(self.bytes.load(Relaxed));
        }
    }
}

#[derive(Clone)]
pub struct Texture {
    inner: Rc<TextureInner>,
}

impl Texture {
    pub(crate) fn new_from_bytes(
        ctx: Rc<ContextInner>,
        config: &Texture2dConfig,
        body: Option<&[u8]>,
    ) -> Result<Self> {
        let texture = create_texture(ctx.gl(), config, body)?;
        let bytes = config.bytes();
        let inner = TextureInner::new(ctx, texture, bytes)?;
        Ok(Self {
            inner: Rc::new(inner),
        })
    }

    pub(crate) fn new_from_image_element(
        ctx: Rc<ContextInner>,
        filter: &TextureFilter,
        element: &web_sys::HtmlImageElement,
    ) -> Result<Self> {
        let texture = create_texture_image_element(ctx.gl(), filter, element)?;
        let bytes = predict_bytes_from_element(element);
        let inner = TextureInner::new(ctx, texture, bytes)?;
        Ok(Self {
            inner: Rc::new(inner),
        })
    }

    /// 生のWebGLテクスチャを取得する
    pub fn texture(&self) -> &Rc<WebGlTexture> {
        &self.inner.texture
    }

    /// テクスチャをバインドする
    pub fn bind(&self) {
        self.inner.bind();
    }

    /// 画像要素からテクスチャを更新する
    pub fn update_texture_image_element(&self, element: &web_sys::HtmlImageElement) {
        update_texture_image_element(self.inner.ctx.gl(), &self.inner.texture, element);
        self.inner.update_bytes(predict_bytes_from_element(element));
    }
}

// 画像要素からテクスチャのバイト数を推定する
fn predict_bytes_from_element(element: &web_sys::HtmlImageElement) -> u64 {
    let width = element.width();
    let height = element.height();
    width as u64 * height as u64 * Texture2dConfig::format_sizeof(gl::RGBA)
}
