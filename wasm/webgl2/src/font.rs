//! フォントレンダリング機能を提供します。

use std::rc::Rc;

use web_sys::{WebGlTexture, WebGlUniformLocation};

use crate::{
    error::Result,
    gl, uniform_location,
    vertex::{Vao, VertexVbo},
    GlPoint2d, Program,
};

pub struct TextShader {
    program: Program,
    window_mat: WebGlUniformLocation,
    texture: WebGlUniformLocation,
}

impl TextShader {
    const VERT: &'static str = r#"#version 300 es
layout(location = 0) in vec2 position;
layout(location = 1) in vec2 coord;

uniform mat3 window_mat;

out vec2 tex_coord;

void main() {
    gl_Position = vec4((window_mat * vec3(position, 1.0)).xy, 0.0, 1.0);
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
    const LOCATION_POSITION: u32 = 0;
    const LOCATION_COORD: u32 = 1;

    pub fn new(gl: &gl) -> Result<Self> {
        let program = Program::new(gl, Self::VERT, Self::FRAG)?;
        let window_mat = uniform_location(gl, &program, "window_mat")?;
        let texture = uniform_location(gl, &program, "u_texture")?;

        Ok(Self {
            program,
            window_mat,
            texture,
        })
    }

    /// テキストの描画に使うVBOを作成する
    pub fn link_vertex(&self, gl: &gl, v_text: &TextVertex2d) -> Result<TextVbo> {
        self.program.use_program(gl);

        let vao = Vao::new(gl)?;
        let vertex = VertexVbo::new(gl, &v_text.positions, Self::LOCATION_POSITION)?;
        let uv = VertexVbo::new(gl, &v_text.uvs, Self::LOCATION_COORD)?;
        vao.unbind(gl);
        Ok(TextVbo {
            texture: v_text.texture.clone(),
            vao,
            vertex,
            uv,
        })
    }

    pub fn set_mat(&self, gl: &gl, mat: &[f32]) {
        self.program.use_program(gl);
        gl.uniform_matrix3fv_with_f32_array(Some(&self.window_mat), false, mat);
    }

    pub fn draw(&self, gl: &gl, vbo: &TextVbo) {
        self.program.use_program(gl);
        gl.active_texture(gl::TEXTURE0);
        gl.bind_texture(gl::TEXTURE_2D, Some(&vbo.texture));
        vbo.vao.bind(gl);
        gl.draw_arrays(gl::TRIANGLES, 0, vbo.vertex.count());
        vbo.vao.unbind(gl);
    }
}

/// 画面に対して文字列を表示するための頂点情報
///
/// 三次元空間内での文字描画は想定しない
#[derive(Debug, PartialEq)]
pub struct TextVertex2d {
    texture: Rc<WebGlTexture>,
    // テキストのピクセルサイズで頂点を作る
    pub positions: Vec<GlPoint2d>,
    pub uvs: Vec<GlPoint2d>,
    // テキストの標準サイズ
    pub text_pt: f32,
    // 文字列長の最大値と現在値
    pub capacity: usize,
    pub len: usize,
}

impl TextVertex2d {
    pub fn update_uv(&self, gl: &gl, vbo: &TextVbo) {
        vbo.uv.update_vertex(gl, &self.uvs);
    }
}

/// テキスト描画用のVBO
pub struct TextVbo {
    texture: Rc<WebGlTexture>,
    vao: Vao,
    vertex: VertexVbo,
    uv: VertexVbo,
}

/// フォントテクスチャと切り出し情報を保持する構造体
pub struct Font {
    texture: Rc<WebGlTexture>,
    detail: FontTextureDetail,
}

impl Font {
    const CHAR_VERTEX_COUNT: usize = 6;
    pub fn new(texture: WebGlTexture, detail: FontTextureDetail) -> Self {
        Self {
            texture: Rc::new(texture),
            detail,
        }
    }

    pub fn texture(&self) -> &WebGlTexture {
        self.texture.as_ref()
    }

    /// 文字列情報から頂点情報を作成する
    pub fn create_text_vertex(&self, text: &str) -> TextVertex2d {
        let mut positions = vec![];
        let mut uvs = vec![];

        // テキストの全幅
        let total_advance = text
            .chars()
            .map(|c| {
                self.detail
                    .characters
                    .get(&c)
                    .map(|ch| ch.advance)
                    .unwrap_or(0)
            })
            .sum::<i32>() as f32;
        // 現在の文字の配置位置。中央揃えを想定
        let mut pos_x = -total_advance / 2.;
        // テキストの高さ(改行なしを想定)
        let pos_y = self.detail.size as f32 / 2.;

        // (x0,y1) --- (x1,y1)
        // |         /  |
        // |     /      |
        // |  /         |
        // (x0,y0) --- (x1,y0)
        for c in text.chars() {
            if let Some(ch) = self.detail.characters.get(&c) {
                // 4つの頂点を作る
                let x0 = pos_x - ch.origin_x as f32;
                let y0 = pos_y + ch.origin_y as f32 - ch.height as f32;
                let x1 = x0 + ch.width as f32;
                let y1 = y0 + ch.height as f32;

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
        TextVertex2d {
            texture: self.texture.clone(),
            positions,
            uvs,
            text_pt: self.detail.size as f32,
            capacity: text.len(),
            len: text.len(),
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

    pub fn update_text(&self, v: &mut TextVertex2d, text: &str) {
        for (i, c) in text.chars().enumerate() {
            let uv_index = i * Self::CHAR_VERTEX_COUNT;
            if let Some(ch) = self.detail.characters.get(&c) {
                self.set_uv(&mut v.uvs[uv_index..uv_index + Self::CHAR_VERTEX_COUNT], ch);
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
