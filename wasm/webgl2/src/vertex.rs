use bytemuck::{NoUninit, Pod};
use wasm_bindgen::JsError;
use web_sys::WebGlBuffer;

use crate::{error::Result, gl, GlEnum, GlInt, GlPoint};

/// VAOを作成。VBOはVAOに紐づけて適宜bindingして使うのが望ましい
pub fn create_vao(gl: &gl) -> Result<web_sys::WebGlVertexArrayObject> {
    gl.create_vertex_array()
        .ok_or(JsError::new("Failed to create vertex array object"))
}

pub struct Vao {
    vao: web_sys::WebGlVertexArrayObject,
}

impl Vao {
    pub fn new(gl: &gl) -> Result<Self> {
        let vao = create_vao(gl)?;
        gl.bind_vertex_array(Some(&vao));
        Ok(Self { vao })
    }

    pub fn bind(&self, gl: &gl) {
        gl.bind_vertex_array(Some(&self.vao));
    }

    pub fn unbind(&self, gl: &gl) {
        gl.bind_vertex_array(None);
    }
}

pub struct VertexVbo {
    vbo: WebGlBuffer,
    location: u32,
    count: GlInt,
}

impl VertexVbo {
    const TARGET: GlEnum = gl::ARRAY_BUFFER;

    /// GlPointトレイトを実装した構造体のデータでVBOを作成
    #[inline]
    pub fn new<P: Pod + GlPoint>(gl: &gl, data: &[P], location: u32) -> Result<Self> {
        let count = data.len() as GlInt;
        let data = bytemuck::cast_slice(data);
        Self::new_raw(gl, data, location, count, P::size())
    }

    /// f32にcast済みのデータでVBOを作成
    pub fn new_raw(
        gl: &gl,
        data: &[f32],
        location: u32,
        count: GlInt,
        sizeof: GlInt,
    ) -> Result<Self> {
        let vbo = Self::create_vertex_buffer(gl, data, location, gl::DYNAMIC_DRAW, sizeof)?;
        Ok(Self {
            vbo,
            location,
            count,
        })
    }

    fn create_vertex_buffer(
        gl: &gl,
        data: &[f32],
        location: u32,
        usage: GlEnum,
        sizeof: GlInt,
    ) -> Result<WebGlBuffer> {
        let buffer = gl
            .create_buffer()
            .ok_or(JsError::new("Failed to create buffer object"))?;
        gl.bind_buffer(Self::TARGET, Some(&buffer));
        buffer_data_f32(gl, Self::TARGET, data, usage);
        gl.enable_vertex_attrib_array(location);
        gl.vertex_attrib_pointer_with_i32(location, sizeof, gl::FLOAT, false, 0, 0);

        Ok(buffer)
    }

    /// VBOの更新
    pub fn update_vertex<P: Pod + GlPoint>(&self, gl: &gl, data: &[P]) {
        let data = bytemuck::cast_slice(data);
        gl.bind_buffer(Self::TARGET, Some(&self.vbo));
        unsafe {
            let view = js_sys::Float32Array::view(data);
            gl.buffer_sub_data_with_i32_and_array_buffer_view(Self::TARGET, 0, &view);
        }
        gl.enable_vertex_attrib_array(self.location);
        gl.vertex_attrib_pointer_with_i32(self.location, P::size(), gl::FLOAT, false, 0, 0);
    }

    /// VBOの一部を更新
    pub fn update_vertex_sub<P: Pod + GlPoint>(&self, gl: &gl, data: &[P], offset: GlInt) {
        let data = bytemuck::cast_slice(data);
        gl.bind_buffer(Self::TARGET, Some(&self.vbo));
        unsafe {
            let view = js_sys::Float32Array::view(data);
            gl.buffer_sub_data_with_i32_and_array_buffer_view_and_src_offset_and_length(
                Self::TARGET,
                offset * P::size() * std::mem::size_of::<f32>() as i32,
                &view,
                0,
                P::size() as u32,
            );
        }
        gl.enable_vertex_attrib_array(self.location);
        gl.vertex_attrib_pointer_with_i32(self.location, P::size(), gl::FLOAT, false, 0, 0);
    }

    pub fn count(&self) -> GlInt {
        self.count
    }
}

/// VBOにデータを書き込む
pub fn buffer_data_f32(gl: &gl, target: u32, data: &[f32], usage: u32) {
    unsafe {
        let view = js_sys::Float32Array::view(data);
        gl.buffer_data_with_array_buffer_view(target, &view, usage);
    }
}

/// VBOにデータを書き込む
#[inline]
pub fn buffer_data<P: GlPoint + NoUninit>(gl: &gl, target: u32, data: &[P], usage: u32) {
    let data = bytemuck::cast_slice(data);
    buffer_data_f32(gl, target, data, usage)
}
