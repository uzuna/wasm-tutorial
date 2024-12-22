use nalgebra::{Matrix4, Point3, Vector3};
use wasm_bindgen::JsError;
use web_sys::{WebGl2RenderingContext as gl, WebGlBuffer};

use crate::{
    error::Result,
    program::{self, uniform_block_binding},
};

pub type Point3f = Point3<f32>;
pub type Vec3f = Vector3<f32>;
pub type Mat4f = Matrix4<f32>;

/// 視線に関する情報
pub struct ViewMatrix {
    /// カメラの位置
    pub eye: Point3f,
    /// 注視点
    pub center: Point3f,
    /// 上方向の軸
    pub up: Vec3f,
}

impl ViewMatrix {
    pub const DEFAULT: Self = Self {
        eye: Point3f::new(0.0, 0.0, 3.0),
        center: Point3f::new(0.0, 0.0, 0.0),
        up: Vec3f::new(0.0, 1.0, 0.0),
    };

    pub const fn new(eye: Point3f, center: Point3f, up: Vec3f) -> Self {
        Self { eye, center, up }
    }

    pub fn look_at(&self) -> Mat4f {
        Mat4f::look_at_rh(&self.eye, &self.center, &self.up)
    }
}

impl Default for ViewMatrix {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// カメラパラメータ
pub struct Camera {
    /// アスペクト比
    pub aspect: f32,
    /// 視野角(縦)
    pub fovy: f32,
    /// レンダリングする最近距離
    pub near: f32,
    /// レンダリングする最遠距離
    pub far: f32,
}

impl Camera {
    const DEFAULT: Self = Self {
        aspect: 1.0,
        fovy: 45.0,
        near: 0.1,
        far: 100.0,
    };

    pub fn perspective(&self) -> nalgebra::Perspective3<f32> {
        nalgebra::Perspective3::new(
            self.aspect,
            self.fovy * std::f32::consts::PI / 180.0,
            self.near,
            self.far,
        )
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// CameraのUniform Bufferを使った実装
pub struct CameraUbo {
    ubo: WebGlBuffer,
}

impl CameraUbo {
    /// VertexShaderに渡すUniform Buffer Objectの定義スクリプト
    pub const SCRIPT: &'static str = r#"
layout(std140, binding = 0) uniform Camera {
    mat4 mvp;
};
"#;
    const MVP_UBI: u32 = 0;

    /// CameraUboの新しいインスタンスを生成する
    pub fn new(gl: &gl, camera: &Camera, view: &ViewMatrix) -> Result<Self> {
        let ubo = gl
            .create_buffer()
            .ok_or(JsError::new("failed to create buffer"))?;
        let mvp = Self::gen_matrix(camera, view);

        gl.bind_buffer(gl::UNIFORM_BUFFER, Some(&ubo));
        unsafe {
            let view = js_sys::Float32Array::view(&mvp);
            gl.buffer_data_with_array_buffer_view(gl::UNIFORM_BUFFER, &view, gl::DYNAMIC_DRAW);
        }
        gl.bind_buffer(gl::UNIFORM_BUFFER, None);
        Ok(Self { ubo })
    }

    fn gen_matrix(camera: &Camera, view: &ViewMatrix) -> Vec<f32> {
        let mvp = camera.perspective().as_matrix() * view.look_at();
        let mvp_arrays: [[f32; 4]; 4] = mvp.into();
        mvp_arrays.iter().flat_map(|a| *a).collect::<Vec<_>>()
    }

    /// カメラのパラメータ、位置関係を示すパラメータを渡してMVP行列を更新する
    pub fn update_mvp(&self, gl: &gl, camera: &Camera, view: &ViewMatrix) {
        let mvp = Self::gen_matrix(camera, view);

        gl.bind_buffer(gl::UNIFORM_BUFFER, Some(&self.ubo));
        unsafe {
            let view = js_sys::Float32Array::view(&mvp);
            gl.buffer_sub_data_with_i32_and_array_buffer_view(gl::UNIFORM_BUFFER, 0, &view);
        }
        gl.bind_buffer(gl::UNIFORM_BUFFER, None);
    }

    /// シェーダープログラムに対してUniform Buffer Objectをバインドする
    ///
    /// 通常のUniformはプログラムコンパイル時に変数領域を確保されるが、共有変数は別のところで初期化されているので、
    /// コンパイル後に一度バインドする必要がある。
    /// 変数の位置はプログラムに依存するので[Self::SCRIPT]の定義の場合に対応しており、他の記述をした場合は変数名や変数位置の変更が必要
    pub fn bind_ubo(&self, gl: &gl, program: &program::Program) {
        uniform_block_binding(gl, program.program(), "matrix", Self::MVP_UBI);
        gl.bind_buffer_base(gl::UNIFORM_BUFFER, Self::MVP_UBI, Some(&self.ubo));
    }
}
