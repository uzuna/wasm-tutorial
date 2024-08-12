//! OpenGLのブレンドモードを指定

use crate::gl;

// example: https://www.andersriggelsen.dk/glblendfunc.php
#[derive(Debug, Clone, Copy, Default)]
pub enum BlendMode {
    /// アルファを考慮して重ねる。明るさは2つの平均になる
    #[default]
    Alpha,
    /// 加算: 2つの色を足し合わせる。明るさは2つの合計になる
    Add,
    /// 除算: 2つの色を引き算する。明るさは2つの差になる
    Sub,
    /// 乗算: 2つの色を掛け合わせる。暗いところがより暗くなる
    Mul,
    /// スクリーン: 新しい色をアルファの係数分だけ足し合わせる
    Screen,
    /// 比較(明): 明るい色を採用する。アルファ値は無視される
    Lighten,
    /// 比較(暗): 暗い色を採用する。アルファ値は無視される
    Darken,
}

impl BlendMode {
    fn gl_values(&self) -> (u32, u32, u32) {
        match self {
            BlendMode::Alpha => (gl::FUNC_ADD, gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA),
            BlendMode::Add => (gl::FUNC_ADD, gl::SRC_ALPHA, gl::ONE),
            BlendMode::Sub => (gl::FUNC_SUBTRACT, gl::SRC_ALPHA, gl::ONE),
            BlendMode::Mul => (gl::FUNC_ADD, gl::ZERO, gl::SRC_COLOR),
            BlendMode::Screen => (gl::FUNC_ADD, gl::ONE_MINUS_DST_COLOR, gl::ONE),
            BlendMode::Lighten => (gl::MAX, gl::ONE, gl::ONE),
            BlendMode::Darken => (gl::MIN, gl::ONE, gl::ONE),
        }
    }

    pub fn enable(&self, gl: &gl) {
        gl.enable(gl::BLEND);
        let (eq, src, dst) = self.gl_values();
        gl.blend_equation(eq);
        gl.blend_func(src, dst);
    }

    pub fn disable(gl: &gl) {
        gl.disable(gl::BLEND);
    }
}

impl From<&str> for BlendMode {
    fn from(s: &str) -> Self {
        match s {
            "alpha" => BlendMode::Alpha,
            "add" => BlendMode::Add,
            "sub" => BlendMode::Sub,
            "mul" => BlendMode::Mul,
            "screen" => BlendMode::Screen,
            "lighten" => BlendMode::Lighten,
            "darken" => BlendMode::Darken,
            _ => BlendMode::Alpha,
        }
    }
}

impl AsRef<str> for BlendMode {
    fn as_ref(&self) -> &str {
        match self {
            BlendMode::Alpha => "alpha",
            BlendMode::Add => "add",
            BlendMode::Sub => "sub",
            BlendMode::Mul => "mul",
            BlendMode::Screen => "screen",
            BlendMode::Lighten => "lighten",
            BlendMode::Darken => "darken",
        }
    }
}
