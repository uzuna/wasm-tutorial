//! フォントレンダリング機能を提供します。

use std::rc::Rc;

use web_sys::WebGlUniformLocation;

use crate::{
    context::Context,
    error::Result,
    gl,
    texture::Texture,
    vertex::{Vao, VaoDefine},
    GlPoint2d, Program,
};

pub struct TextShader {
    program: Program,
    local_mat: WebGlUniformLocation,
}

impl TextShader {
    const VERT: &'static str = r#"#version 300 es
layout(location = 0) in vec2 position;
layout(location = 1) in vec2 coord;

uniform mat3 local_mat;

out vec2 tex_coord;

void main() {
    gl_Position = vec4((local_mat * vec3(position, 1.0)).xy, 0.0, 1.0);
    tex_coord = coord;
}
"#;

    // copy from: https://github.com/evanw/font-texture-generator/blob/gh-pages/example-webgl/index.html#L246-L268
    const FRAG: &'static str = r#"#version 300 es
precision mediump float;

uniform sampler2D u_texture;
in vec2 tex_coord;

out vec4 outColor;

void main() {
    // 白黒情報を抽出
    vec4 tex_color = texture(u_texture, tex_coord);

    // 単純表示ではなくなにか処理を掛けているけどわからない
    float scale = 1.0 / fwidth(tex_color.r);
    float signedDistance = (tex_color.r - 0.5) * scale;

    float color = clamp(signedDistance + 0.5, 0.0, 1.0);
    float alpha = clamp(signedDistance + scale * 0.125, 0.0, 1.0);
    outColor = vec4(color, color, color, alpha);
}
"#;

    pub fn new(ctx: &Context) -> Result<Self> {
        let program = ctx.program(Self::VERT, Self::FRAG)?;
        let local_mat = program.uniform_location("local_mat")?;

        Ok(Self { program, local_mat })
    }

    /// テキストの描画に使うVBOを作成する
    pub fn create_vbo(&self, v_text: &TextVertex) -> Result<TextVao> {
        self.program.use_program();
        let v = &v_text.vertex;

        let mut vao = self.program.create_vao()?;
        vao.buffer_data(TextVaoDefine::Vertex, &v.positions, gl::STATIC_DRAW);
        vao.buffer_data(TextVaoDefine::Uv, &v.uvs, gl::DYNAMIC_DRAW);
        Ok(TextVao {
            texture: v_text.font.texture.clone(),
            vao,
            vertex_size: v.positions.len() as i32,
        })
    }

    pub fn local_mat(&self, mat: &nalgebra::Matrix3<f32>) {
        self.program.use_program();
        self.program.gl().uniform_matrix3fv_with_f32_array(
            Some(&self.local_mat),
            false,
            mat.as_slice(),
        );
    }

    pub fn draw(&self, vao: &TextVao) {
        self.program.use_program();
        let gl = self.program.gl();
        gl.active_texture(gl::TEXTURE0);
        vao.bind();
        gl.draw_arrays(gl::TRIANGLES, 0, vao.vertex_size);
        vao.unbind();
    }
}

/// テキスト描画情報と、その更新方法を提供する構造体
pub struct TextVertex {
    font: Rc<FontInner>,
    vertex: TextVertexInner,
}

impl TextVertex {
    /// テキストを更新する。最初に作った時以上の文字列は無視される
    pub fn update_text(&mut self, text: &str) {
        self.font.update_text(&mut self.vertex, text);
    }

    /// 頂点情報をVBOに適用する
    pub fn apply_to_vao(&self, vao: &TextVao) {
        self.vertex.update(vao);
    }
}

/// 画面に対して文字列を表示するための頂点情報
///
/// 三次元空間内での文字描画は想定しない
#[derive(Debug, PartialEq)]
struct TextVertexInner {
    // テキストのピクセルサイズで頂点を作る
    positions: Vec<GlPoint2d>,
    uvs: Vec<GlPoint2d>,
    // テキストの標準サイズ
    text_pt: f32,
    // 文字列長の最大値と現在値
    capacity: usize,
    len: usize,
    align: Align,
}

impl TextVertexInner {
    // 数字以外の文字列を描画する場合。文字によって位置が変わるのでpositionも変更する
    //
    // TODO: 数字だけなどならuv更新に限定するなど効率化する余地がある
    fn update(&self, vao: &TextVao) {
        vao.vao
            .buffer_sub_data(TextVaoDefine::Vertex, &self.positions, 0);
        vao.vao.buffer_sub_data(TextVaoDefine::Uv, &self.uvs, 0);
    }
}

/// テキスト描画用のVBOの定義
#[derive(Debug, PartialEq)]
enum TextVaoDefine {
    Vertex,
    Uv,
}

impl VaoDefine for TextVaoDefine {
    fn iter() -> std::slice::Iter<'static, Self> {
        static VAO: [TextVaoDefine; 2] = [TextVaoDefine::Vertex, TextVaoDefine::Uv];
        VAO.iter()
    }

    fn name(&self) -> &'static str {
        match self {
            TextVaoDefine::Vertex => "position",
            TextVaoDefine::Uv => "coord",
        }
    }

    fn size_of(&self) -> i32 {
        use crate::GlPoint;
        GlPoint2d::size()
    }
}

/// テキスト描画用のVAO
///
/// GPUメモリ上に保持されたテキストの頂点とテクスチャを保持している。
/// この構造体があれば[Font]や[TextVertex]が無くても描画自体は可能
pub struct TextVao {
    texture: Texture,
    vao: Vao<TextVaoDefine>,
    // draw時に頂点数を渡す
    vertex_size: i32,
}

impl TextVao {
    pub fn bind(&self) {
        let gl = self.vao.gl();
        gl.bind_texture(gl::TEXTURE_2D, Some(self.texture.texture()));
        self.vao.bind();
    }
    pub fn unbind(&self) {
        self.vao.unbind();
    }
}

/// フォントテクスチャと切り出し情報を保持する構造体
pub struct Font {
    inner: Rc<FontInner>,
}

impl Font {
    /// フォントテクスチャと切り出し情報を保持する構造体を作成する
    pub fn new(texture: Texture, detail: FontTextureDetail) -> Self {
        Self {
            inner: Rc::new(FontInner::new(texture, detail)),
        }
    }

    /// 文字列と整列情報から頂点とテキスト編集構造体を作成する
    pub fn text(&self, text: &str, align: Align) -> TextVertex {
        let vi = self.inner.create_text_vertex(text, align);
        TextVertex {
            font: self.inner.clone(),
            vertex: vi,
        }
    }

    /// 文字数を指定して、空のテキスト編集構造体を作成する
    #[inline]
    pub fn text_by_capacity(&self, text_len: u32, align: Align) -> TextVertex {
        let text = " ".repeat(text_len as usize);
        self.text(&text, align)
    }
}

struct FontInner {
    // テクスチャはTextVaoがある限り描画可能にするためRcで包む
    texture: Texture,
    detail: FontTextureDetail,
}

impl FontInner {
    const CHAR_VERTEX_COUNT: usize = 6;

    fn new(texture: Texture, detail: FontTextureDetail) -> Self {
        Self { texture, detail }
    }

    fn aling_position(&self, align: Align, total_advance: f32) -> (f32, f32) {
        // 現在の文字の配置位置。中央揃えを想定
        let pos_x = match align.text {
            TextAlign::Left => 0.0,
            TextAlign::Center => -total_advance / 2.,
            TextAlign::Right => -total_advance,
        };
        // 7座標も0,0原点が中心に来るように配置
        let pos_y = match align.vertical {
            VerticalAlign::Top => -(self.detail.size as f32),
            VerticalAlign::Middle => -(self.detail.size as f32 / 2.),
            VerticalAlign::Bottom => 0.0,
        };
        (pos_x, pos_y)
    }

    // テキストの全幅
    fn total_advance(&self, text: &str) -> f32 {
        text.chars()
            .map(|c| {
                self.detail
                    .characters
                    .get(&c)
                    .map(|ch| ch.advance)
                    .unwrap_or(0)
            })
            .sum::<i32>() as f32
    }

    /// 文字列情報から頂点情報を作成する
    ///
    /// 高さが2.0の大きさの頂点データが作られる
    fn create_text_vertex(&self, text: &str, align: Align) -> TextVertexInner {
        let mut positions = vec![];
        let mut uvs = vec![];

        // 描画開始位置の決定
        let total_advance = self.total_advance(text);
        let (mut pos_x, pos_y) = self.aling_position(align, total_advance);

        // フォントサイズに関わらず高さを2.0に合わせる
        let scale = 2.0 / self.detail.size as f32;

        // (x0,y1) --- (x1,y1)
        // |         /  |
        // |     /      |
        // |  /         |
        // (x0,y0) --- (x1,y0)
        for c in text.chars() {
            if let Some(ch) = self.detail.characters.get(&c) {
                // 4つの頂点を作る
                let x0 = (pos_x - ch.origin_x as f32) * scale;
                let y0 = (pos_y + ch.origin_y as f32 - ch.height as f32) * scale;
                let x1 = x0 + (ch.width as f32 * scale);
                let y1 = y0 + (ch.height as f32 * scale);

                positions.push(GlPoint2d::new(x0, y1));
                positions.push(GlPoint2d::new(x0, y0));
                positions.push(GlPoint2d::new(x1, y1));
                positions.push(GlPoint2d::new(x1, y1));
                positions.push(GlPoint2d::new(x0, y0));
                positions.push(GlPoint2d::new(x1, y0));

                // UV座標。位置は元の画像の大きさから0-1.0空間にマップされている
                // 左下が0,0で右上が1,1で、画像のpxとはy軸が逆
                let u0 = ch.x as f32 / self.detail.width as f32;
                let v1 = ch.y as f32 / self.detail.height as f32;
                let u1 = (ch.x + ch.width) as f32 / self.detail.width as f32;
                let v0 = (ch.y + ch.height) as f32 / self.detail.height as f32;

                uvs.push(GlPoint2d::new(u0, v1));
                uvs.push(GlPoint2d::new(u0, v0));
                uvs.push(GlPoint2d::new(u1, v1));
                uvs.push(GlPoint2d::new(u1, v1));
                uvs.push(GlPoint2d::new(u0, v0));
                uvs.push(GlPoint2d::new(u1, v0));

                // 頂点位置を進める
                pos_x += ch.advance as f32;
            }
        }
        TextVertexInner {
            positions,
            uvs,
            text_pt: self.detail.size as f32,
            capacity: text.len(),
            len: text.len(),
            align,
        }
    }

    fn set_uv(&self, uvs: &mut [GlPoint2d], ch: &Character) {
        let u0 = ch.x as f32 / self.detail.width as f32;
        let v1 = ch.y as f32 / self.detail.height as f32;
        let u1 = (ch.x + ch.width) as f32 / self.detail.width as f32;
        let v0 = (ch.y + ch.height) as f32 / self.detail.height as f32;

        uvs[0] = GlPoint2d::new(u0, v1);
        uvs[1] = GlPoint2d::new(u0, v0);
        uvs[2] = GlPoint2d::new(u1, v1);
        uvs[3] = GlPoint2d::new(u1, v1);
        uvs[4] = GlPoint2d::new(u0, v0);
        uvs[5] = GlPoint2d::new(u1, v0);
    }

    fn set_vertex(&self, vs: &mut [GlPoint2d], ch: &Character, pos_x: f32, pos_y: f32) {
        let scale = 2.0 / self.detail.size as f32;

        // 4つの頂点を作る
        let x0 = (pos_x - ch.origin_x as f32) * scale;
        let y0 = (pos_y + ch.origin_y as f32 - ch.height as f32) * scale;
        let x1 = x0 + (ch.width as f32 * scale);
        let y1 = y0 + (ch.height as f32 * scale);

        vs[0] = GlPoint2d::new(x0, y1);
        vs[1] = GlPoint2d::new(x0, y0);
        vs[2] = GlPoint2d::new(x1, y1);
        vs[3] = GlPoint2d::new(x1, y1);
        vs[4] = GlPoint2d::new(x0, y0);
        vs[5] = GlPoint2d::new(x1, y0);
    }

    fn update_text(&self, v: &mut TextVertexInner, text: &str) {
        let total_advance = self.total_advance(text);
        let (mut pos_x, pos_y) = self.aling_position(v.align, total_advance);
        for i in 0..v.capacity {
            let idx = i * Self::CHAR_VERTEX_COUNT;
            let idx_next = idx + Self::CHAR_VERTEX_COUNT;
            if i < text.len() {
                let c = text.chars().nth(i).unwrap();
                if let Some(ch) = self.detail.characters.get(&c) {
                    self.set_uv(&mut v.uvs[idx..idx_next], ch);
                    self.set_vertex(&mut v.positions[idx..idx_next], ch, pos_x, pos_y);
                    pos_x += ch.advance as f32;
                }
            } else {
                // 文字列が短い場合は空白で埋める
                self.set_uv(&mut v.uvs[idx..idx_next], &self.detail.characters[&' ']);
                self.set_vertex(
                    &mut v.positions[idx..idx_next],
                    &self.detail.characters[&' '],
                    pos_x,
                    pos_y,
                );
            }
        }
    }
}

/// reference from: https://evanw.github.io/font-texture-generator/
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FontTextureDetail {
    // フォント名称
    name: String,
    // 画像のフォントのポイント数
    size: u32,
    // boldか
    bold: bool,
    // italicか
    italic: bool,
    // 画像の幅と高さ
    width: u32,
    height: u32,
    // 各文字の情報
    characters: fxhash::FxHashMap<char, Character>,
}

impl FontTextureDetail {
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
}

/// テキスト描画の整列情報
#[derive(
    Debug, Clone, Copy, Default, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct Align {
    pub text: TextAlign,
    pub vertical: VerticalAlign,
}

impl Align {
    pub fn left_top() -> Self {
        Self {
            text: TextAlign::Left,
            vertical: VerticalAlign::Top,
        }
    }
    pub fn left_bottom() -> Self {
        Self {
            text: TextAlign::Left,
            vertical: VerticalAlign::Bottom,
        }
    }
    pub fn center_middle() -> Self {
        Self {
            text: TextAlign::Center,
            vertical: VerticalAlign::Middle,
        }
    }
}

/// 0,0原点にテキストの左右中央がいずれが来るかのことを指す
#[derive(
    Debug, Clone, Copy, Default, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

/// 0,0原点にテキストの上下中央がいずれが来るかのことを指す
#[derive(
    Debug, Clone, Copy, Default, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub enum VerticalAlign {
    Top,
    #[default]
    Middle,
    Bottom,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    origin_x: i32,
    origin_y: i32,
    // 文字の送り幅: グリフの開始位置から次のグリフの開始位置までの距離
    advance: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_font_detail() {
        let detail_file = "testdata/Ubuntu_Mono_64px.json";
        let str = std::fs::read_to_string(detail_file)
            .unwrap_or_else(|_| panic!("Failed to read file {detail_file}"));
        let parsed: FontTextureDetail = serde_json::from_str(&str).unwrap();

        println!("{:?}", parsed);
    }
}
