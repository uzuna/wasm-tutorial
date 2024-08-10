use std::rc::Rc;

use crate::webgl::{gl, uniform_location, GlPoint, GlPoint2d, GlPoint3d, GlPoint4d, Program};
use wasm_bindgen::JsError;
use wasm_utils::{error::*, info};
use web_sys::{WebGlBuffer, WebGlProgram, WebGlUniformLocation};

fn create_buffer(gl: &gl) -> Result<web_sys::WebGlBuffer> {
    gl.create_buffer()
        .ok_or(JsError::new("Failed to create_buffer"))
}

pub struct SimpleShader {
    gl: Rc<gl>,
    program: Program,
    color: WebGlUniformLocation,
    position: u32,
    vbo: WebGlBuffer,
}

impl SimpleShader {
    pub const VERT: &'static str = r#"attribute vec2 position;
void main(void){
	gl_Position = vec4(position.xy, 0.0, 1.0);
}
"#;

    pub const FRAG: &'static str = r#"precision mediump float;

uniform vec4 u_color;

void main(void){
    gl_FragColor = u_color;
}
"#;

    pub fn new(gl: Rc<gl>, data: &[f32]) -> Result<Self> {
        let program = Program::new(&gl, Self::VERT, Self::FRAG)?;
        program.use_program(&gl);
        let color = uniform_location(&gl, &program, "u_color")?;
        let position = gl.get_attrib_location(program.program(), "position") as u32;
        info!(
            "get_attrib_location = {}, error: {}",
            position,
            gl.get_error()
        );
        let vbo = create_buffer(&gl)?;
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(&vbo));
        #[rustfmt::skip]
        VertexObject::buffer_data(
            &gl,
            gl::ARRAY_BUFFER,
            data,
            gl::STATIC_DRAW,
        );

        info!("buffer_data {}", gl.get_error());

        gl.enable_vertex_attrib_array(position);
        info!("enable_vertex_attrib_array {}", gl.get_error());

        gl.vertex_attrib_pointer_with_i32(position, 2, gl::FLOAT, false, 0, 0);
        info!("vertex_attrib_pointer_with_i32 {}", gl.get_error());

        let s = Self {
            gl,
            program,
            color,
            position,
            vbo,
        };
        s.init();
        Ok(s)
    }

    pub fn init(&self) {
        self.set_color([1.0, 0.0, 0.0, 1.0]);
    }

    pub fn use_program(&self) {
        self.program.use_program(&self.gl);
    }

    pub fn set_color(&self, color: [f32; 4]) {
        self.gl.uniform4fv_with_f32_array(Some(&self.color), &color);
    }

    pub fn draw(&self) {
        self.use_program();
        self.gl.draw_arrays(gl::TRIANGLE_STRIP, 0, 4);
        info!("draw_arrays {}", self.gl.get_error());
    }
}

/// 平面を描くデバッグ用シェーダー
pub struct PlaneShader {
    gl: Rc<gl>,
    program: Program,
    uniforms: PlaneUniforms,
    attribute: PlaneAttribute,
}

impl PlaneShader {
    // x方向は時間情報なので、表示範囲の指定にwindow_matを使う
    const VERT: &'static str = r#"attribute vec3  position;
attribute vec4  color;
attribute vec2  textureCoord;
uniform mat4  mvpMatrix;
uniform float vertexAlpha;
varying vec4  vColor;
varying vec2  vTextureCoord;

void main(void){
	vColor        = vec4(color.rgb, color.a * vertexAlpha);
	vTextureCoord = textureCoord;
	gl_Position   = mvpMatrix * vec4(position, 1.0);
}
"#;

    const FRAG: &'static str = r#"precision mediump float;

uniform sampler2D texture;
uniform int       useTexture;
varying vec4      vColor;
varying vec2      vTextureCoord;

void main(void){
    vec4 destColor = vec4(0.0);
    if(bool(useTexture)){
        vec4 smpColor = texture2D(texture, vTextureCoord);
        destColor = vColor * smpColor;
    }else{
        destColor = vColor;
    }
    gl_FragColor = destColor;
}
"#;

    pub fn new(gl: Rc<gl>) -> Result<Self> {
        let program = Program::new(&gl, Self::VERT, Self::FRAG)?;
        let attribute = PlaneAttribute::new(&gl, program.program())?;
        let uniforms = PlaneUniforms::new(gl.clone(), &program)?;
        program.use_program(&gl);
        uniforms.init();
        Ok(Self {
            gl,
            program,
            uniforms,
            attribute,
        })
    }

    pub fn use_program(&self) {
        self.program.use_program(&self.gl);
    }

    pub fn uniforms(&self) -> &PlaneUniforms {
        &self.uniforms
    }

    pub fn draw(&self, v: &VertexObject) {
        self.program.use_program(&self.gl);
        self.attribute.set_attribute(&self.gl, v);
        self.gl
            .draw_elements_with_i32(gl::TRIANGLES, v.index_count, gl::UNSIGNED_SHORT, 0);
    }
}

pub struct PlaneUniforms {
    gl: Rc<gl>,
    pub mvp_matrix: WebGlUniformLocation,
    pub vertex_alpha: WebGlUniformLocation,
    pub texture: WebGlUniformLocation,
    pub use_texture: WebGlUniformLocation,
}

impl PlaneUniforms {
    pub fn new(gl: Rc<gl>, program: &Program) -> Result<Self> {
        let mvp_matrix = uniform_location(&gl, program, "mvpMatrix")?;
        info!("uniform_location {}", gl.get_error());
        let vertex_alpha = uniform_location(&gl, program, "vertexAlpha")?;
        let texture = uniform_location(&gl, program, "texture")?;
        let use_texture = uniform_location(&gl, program, "useTexture")?;

        Ok(Self {
            gl,
            mvp_matrix,
            vertex_alpha,
            texture,
            use_texture,
        })
    }

    pub fn init(&self) {
        self.set_mvp(nalgebra::Matrix4::identity());
        self.set_vertex_alpha(1.0);
        self.set_texture_unit(0);
        self.set_use_texture(false);
    }

    pub fn set_mvp(&self, mat: nalgebra::Matrix4<f32>) {
        self.gl
            .uniform_matrix4fv_with_f32_array(Some(&self.mvp_matrix), false, mat.as_slice());
    }

    pub fn set_vertex_alpha(&self, vertex_alpha: f32) {
        self.gl.uniform1f(Some(&self.vertex_alpha), vertex_alpha);
    }

    pub fn set_texture_unit(&self, texture: i32) {
        self.gl.uniform1i(Some(&self.texture), texture);
    }

    pub fn set_use_texture(&self, use_texture: bool) {
        self.gl
            .uniform1i(Some(&self.use_texture), if use_texture { 1 } else { 0 });
    }
}

pub struct PlaneAttribute {
    position: u32,
    color: u32,
    texture_coord: u32,
}

impl PlaneAttribute {
    fn new(gl: &gl, program: &WebGlProgram) -> Result<Self> {
        let position = gl.get_attrib_location(program, "position") as u32;
        let color = gl.get_attrib_location(program, "color") as u32;
        let texture_coord = gl.get_attrib_location(program, "textureCoord") as u32;

        Ok(Self {
            position,
            color,
            texture_coord,
        })
    }

    fn set_attribute(&self, gl: &gl, v: &VertexObject) {
        self.set_attribute_inner::<GlPoint3d>(gl, self.position, &v.vertex);
        self.set_attribute_inner::<GlPoint4d>(gl, self.color, &v.color);
        self.set_attribute_inner::<GlPoint2d>(gl, self.texture_coord, &v.coord);
    }

    fn set_attribute_inner<P: GlPoint>(&self, gl: &gl, attr: u32, buf: &WebGlBuffer) {
        // バッファをバインド -> Attirbute有効化 -> Attributeにバインドしているバッファ内容を登録
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(buf));
        gl.enable_vertex_attrib_array(attr);
        gl.vertex_attrib_pointer_with_i32(attr, P::size(), gl::FLOAT, false, 0, 0);
        info!("set_attribute_inner {}", gl.get_error());
    }
}

pub struct VertexObject {
    gl: Rc<gl>,
    vertex: web_sys::WebGlBuffer,
    coord: web_sys::WebGlBuffer,
    color: web_sys::WebGlBuffer,
    index: web_sys::WebGlBuffer,
    index_count: i32,
}

impl VertexObject {
    const RECT_VERTEX: [GlPoint3d; 4] = [
        GlPoint3d::new(-1.0, 1.0, 0.0),
        GlPoint3d::new(1.0, 1.0, 0.0),
        GlPoint3d::new(-1.0, -1.0, 0.0),
        GlPoint3d::new(1.0, -1.0, 0.0),
    ];

    const RECT_COORD: [GlPoint2d; 4] = [
        GlPoint2d::new(0.0, 0.0),
        GlPoint2d::new(1.0, 0.0),
        GlPoint2d::new(0.0, 1.0),
        GlPoint2d::new(1.0, 1.0),
    ];

    const RECT_COLOR: [GlPoint4d; 4] = [
        GlPoint4d::new(1.0, 0.0, 0.0, 1.0),
        GlPoint4d::new(0.0, 1.0, 0.0, 1.0),
        GlPoint4d::new(0.0, 0.0, 1.0, 1.0),
        GlPoint4d::new(1.0, 1.0, 1.0, 1.0),
    ];

    const RECT_INDEX: [u16; 6] = [0, 1, 2, 3, 2, 1];
    pub fn new(gl: Rc<gl>) -> Result<Self> {
        let vertex = create_buffer(&gl)?;
        let coord = create_buffer(&gl)?;
        let color = create_buffer(&gl)?;
        let index = create_buffer(&gl)?;

        Ok(Self {
            gl,
            vertex,
            coord,
            color,
            index,
            index_count: 0,
        })
    }

    pub fn rect(gl: Rc<gl>) -> Result<Self> {
        let mut v = Self::new(gl)?;
        v.rect_inner();
        Ok(v)
    }

    fn rect_inner(&mut self) {
        let gl = &self.gl;
        let data = bytemuck::cast_slice(&Self::RECT_VERTEX);
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(&self.vertex));
        Self::buffer_data(gl, gl::ARRAY_BUFFER, data, gl::STATIC_DRAW);
        gl.bind_buffer(gl::ARRAY_BUFFER, None);
        info!("bind_buffer {}", gl.get_error());

        let data = bytemuck::cast_slice(&Self::RECT_COORD);
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(&self.coord));
        Self::buffer_data(gl, gl::ARRAY_BUFFER, data, gl::STATIC_DRAW);
        gl.bind_buffer(gl::ARRAY_BUFFER, None);

        let data = bytemuck::cast_slice(&Self::RECT_COLOR);
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(&self.color));
        Self::buffer_data(gl, gl::ARRAY_BUFFER, data, gl::STATIC_DRAW);
        gl.bind_buffer(gl::ARRAY_BUFFER, None);

        let data = bytemuck::cast_slice(&Self::RECT_INDEX);
        gl.bind_buffer(gl::ELEMENT_ARRAY_BUFFER, Some(&self.index));
        Self::buffer_data(gl, gl::ELEMENT_ARRAY_BUFFER, data, gl::STATIC_DRAW);
        gl.bind_buffer(gl::ARRAY_BUFFER, None);

        self.index_count = Self::RECT_INDEX.len() as i32;
    }

    pub fn buffer_data(gl: &gl, target: u32, data: &[f32], usage: u32) {
        unsafe {
            let view = js_sys::Float32Array::view(data);
            gl.buffer_data_with_array_buffer_view(target, &view, usage);
        }
    }
}
