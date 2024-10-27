use bytemuck::{Pod, Zeroable};
use wasm_bindgen::JsError;
pub use web_sys::WebGl2RenderingContext as gl;

pub mod blend;
pub mod error;
pub mod program;

#[cfg(feature = "vertex")]
pub mod vertex;

#[cfg(feature = "context")]
pub mod context;

#[cfg(feature = "font")]
pub mod font;

#[cfg(feature = "viewport")]
pub mod viewport;

#[cfg(feature = "shader")]
pub mod shader;

#[cfg(feature = "metrics")]
pub mod metrics;

#[cfg(feature = "texture")]
pub mod texture;

#[cfg(feature = "loader")]
pub mod loader;

pub type GlEnum = u32;
pub type GlInt = i32;

/// OpenGLに渡す頂点情報を扱いやすくするためのトレイト
///
/// OpenGLの頂点情報は、`f32`の配列として扱う。
/// `1点あたりのデータ数 * 頂点数の長さ`の配列を渡す必要があり、単純なf32配列で扱うので認知負荷が高い。
/// 点数毎に与える引数はわかっているのでトレイトで表現する
pub trait GlPoint {
    /// 1点あたりのデータ数
    fn size() -> GlInt;
    /// 頂点情報の型。精度はf32で十分
    fn type_() -> GlEnum {
        gl::FLOAT
    }
}

/// OpenGLに渡す2次元の点の情報。主に平面座標に使う
///
/// 連続する2つの`f32`のデータとして見えなければならないのでCの構造体として定義する  
#[derive(Debug, Clone, Copy, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct GlPoint1d {
    pub x: f32,
}

impl GlPoint1d {
    #[inline]
    pub const fn new(x: f32) -> Self {
        Self { x }
    }
}

impl GlPoint for GlPoint1d {
    fn size() -> GlInt {
        1
    }
}

/// OpenGLに渡す2次元の点の情報。主に平面座標に使う
///
/// 連続する2つの`f32`のデータとして見えなければならないのでCの構造体として定義する  
#[derive(Debug, Clone, Copy, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct GlPoint2d {
    pub x: f32,
    pub y: f32,
}

impl GlPoint2d {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn norm(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

impl GlPoint for GlPoint2d {
    fn size() -> GlInt {
        2
    }
}

impl std::ops::Sub for GlPoint2d {
    type Output = GlPoint2d;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::Mul<f32> for GlPoint2d {
    type Output = GlPoint2d;
    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl std::ops::DivAssign<f32> for GlPoint2d {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl std::ops::AddAssign for GlPoint2d {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

/// OpenGLに渡す3次元の点の情報。主に3次元空間の座標に使う
#[derive(Debug, Clone, Copy, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct GlPoint3d {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl GlPoint3d {
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    #[inline]
    pub const fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

impl GlPoint for GlPoint3d {
    fn size() -> GlInt {
        3
    }
}

/// OpenGLに渡す4次元の点の情報。主に色表現に使う
#[derive(Debug, Clone, Copy, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct GlPoint4d {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl GlPoint4d {
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

impl GlPoint for GlPoint4d {
    fn size() -> GlInt {
        4
    }
}

impl From<[f32; 4]> for GlPoint4d {
    fn from(v: [f32; 4]) -> Self {
        Self {
            x: v[0],
            y: v[1],
            z: v[2],
            w: v[3],
        }
    }
}
