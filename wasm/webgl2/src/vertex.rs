use bytemuck::NoUninit;
use wasm_bindgen::JsError;
use web_sys::{WebGlBuffer, WebGlProgram, WebGlVertexArrayObject};

use crate::{error::Result, gl, GlInt, GlPoint, GlPoint2d};

pub fn create_buffer(gl: &gl) -> Result<web_sys::WebGlBuffer> {
    gl.create_buffer()
        .ok_or(JsError::new("Failed to create_buffer"))
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

/// VBOの一部を更新
pub fn buffer_subdata<P: GlPoint + NoUninit>(gl: &gl, target: u32, data: &[P], offset: GlInt) {
    let data = bytemuck::cast_slice(data);
    unsafe {
        let view = js_sys::Float32Array::view(data);
        gl.buffer_sub_data_with_i32_and_array_buffer_view_and_src_offset(
            target,
            offset * P::size() * std::mem::size_of::<f32>() as i32,
            &view,
            0,
        );
    }
}

pub trait VaoDefine: 'static + Sized + PartialEq {
    // 頂点バッファのリスト
    fn iter() -> std::slice::Iter<'static, Self>;
    // 頂点バッファの名前
    fn name(&self) -> &'static str;
    // 頂点バッファの次元数
    fn size_of(&self) -> i32;
    // vboを配列に入れたときの位置を取得
    fn index(&self) -> usize {
        Self::iter().position(|x| x == self).unwrap()
    }
    fn has_index_buffer() -> bool {
        false
    }
}

pub struct Vao<T>
where
    T: VaoDefine,
{
    vao: WebGlVertexArrayObject,
    vbos: Vec<WebGlBuffer>,
    index: Option<WebGlBuffer>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Vao<T>
where
    T: VaoDefine,
{
    pub fn new(gl: &gl, prog: &WebGlProgram) -> Result<Self> {
        let vao = gl
            .create_vertex_array()
            .ok_or(JsError::new("Failed to create vao"))?;
        gl.bind_vertex_array(Some(&vao));
        let mut vbos = vec![];
        for v in T::iter() {
            // Attributeの位置を取得
            let loc = gl.get_attrib_location(prog, v.name()) as u32;
            // VBOを作成して紐付け
            let vbo = create_buffer(gl)?;
            gl.bind_buffer(gl::ARRAY_BUFFER, Some(&vbo));
            gl.enable_vertex_attrib_array(loc);
            gl.vertex_attrib_pointer_with_i32(loc, v.size_of(), gl::FLOAT, false, 0, 0);
            vbos.push(vbo);
        }
        let index = if T::has_index_buffer() {
            let index = create_buffer(gl)?;
            gl.bind_buffer(gl::ELEMENT_ARRAY_BUFFER, Some(&index));
            Some(index)
        } else {
            None
        };
        gl.bind_vertex_array(None);
        Ok(Self {
            vao,
            vbos,
            index,
            _phantom: std::marker::PhantomData,
        })
    }

    pub fn bind(&self, gl: &gl) {
        gl.bind_vertex_array(Some(&self.vao));
    }

    pub fn unbind(&self, gl: &gl) {
        gl.bind_vertex_array(None);
    }

    pub fn vbo(&self, vd: T) -> &WebGlBuffer {
        &self.vbos[vd.index()]
    }

    // usage: gl::STATIC_DRAW, gl::DYNAMIC_DRAW, gl::STREAM_DRAW
    pub fn buffer_data<P: GlPoint + NoUninit>(&self, gl: &gl, vd: T, data: &[P], usage: u32) {
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(&self.vbos[vd.index()]));
        buffer_data(gl, gl::ARRAY_BUFFER, data, usage);
    }

    pub fn buffer_sub_data<P: GlPoint + NoUninit>(&self, gl: &gl, vd: T, data: &[P], offset: i32) {
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(&self.vbos[vd.index()]));
        buffer_subdata(gl, gl::ARRAY_BUFFER, data, offset);
    }

    pub fn index_buffer_data(&self, gl: &gl, data: &[u16], usage: u32) {
        gl.bind_buffer(gl::ELEMENT_ARRAY_BUFFER, self.index.as_ref());
        unsafe {
            let view = js_sys::Uint16Array::view(data);
            gl.buffer_data_with_array_buffer_view(gl::ELEMENT_ARRAY_BUFFER, &view, usage);
        }
    }
}

/// 画面全体を覆う四角形の頂点座標
///
/// 左下, 右下, 左上, 右上の順
pub const UNIT_RECT: [GlPoint2d; 4] = [
    GlPoint2d::new(-1.0, -1.0),
    GlPoint2d::new(1.0, -1.0),
    GlPoint2d::new(-1.0, 1.0),
    GlPoint2d::new(1.0, 1.0),
];
