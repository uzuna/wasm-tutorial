use web_sys::{WebGlBuffer, WebGlUniformLocation};

use super::program::{gl, GlEnum, GlPoint, GlPoint2D, Program};

use crate::error::{Error, Result};

pub struct ParticleShader {
    program: Program,
    particle: Particle,
    vbo: VertexVbo,
    uniform: ParticleUniform,
}

impl ParticleShader {
    /// reference: https://wgld.org/d/webgl/w082.html
    const VERT: &'static str = r#"#version 300 es

layout(location = 0) in vec2 position;

uniform float pointSize;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    gl_PointSize = pointSize;
}
"#;

    const FRAG: &'static str = r#"#version 300 es

precision mediump float;

uniform vec4 pointColor;
out vec4 fragmentColor;

void main() {
    fragmentColor = pointColor;
}
"#;

    pub fn new(gl: &gl, res: Resolution) -> Result<Self> {
        let program = Program::new(gl, Self::VERT, Self::FRAG)?;
        let particle = Particle::new(res);
        let vbo = VertexVbo::new(gl, &particle.position, 0)?;
        program.use_program(gl);
        let uniform = ParticleUniform::new(gl, &program)?;
        uniform.init(gl);
        Ok(Self {
            program,
            particle,
            vbo,
            uniform,
        })
    }

    pub fn set_color(&self, gl: &gl, color: [f32; 4]) {
        self.uniform.set_color(gl, color);
    }

    pub fn update(&mut self, gl: &gl, target: Point, vector_update: bool) {
        self.particle.update(target, vector_update);
        self.vbo.update_vertex(gl, &self.particle.position);
        self.uniform
            .set_size(gl, self.particle.velocity * 1.25 + 0.25);
    }

    pub fn draw(&self, gl: &gl) {
        self.program.use_program(gl);
        gl.draw_arrays(gl::POINTS, 0, self.particle.position.len() as i32);
    }
}

struct ParticleUniform {
    size: WebGlUniformLocation,
    color: WebGlUniformLocation,
}

impl ParticleUniform {
    pub fn new(gl: &gl, program: &Program) -> Result<Self> {
        let size = gl
            .get_uniform_location(program.program(), "pointSize")
            .ok_or(Error::gl("Failed to get uniform location".into()))?;
        let color = gl
            .get_uniform_location(program.program(), "pointColor")
            .ok_or(Error::gl("Failed to get uniform location".into()))?;
        Ok(Self { size, color })
    }

    pub fn init(&self, gl: &gl) {
        gl.uniform1f(Some(&self.size), 1.0);
        gl.uniform4f(Some(&self.color), 1.0, 1.0, 1.0, 1.0);
    }

    pub fn set_size(&self, gl: &gl, size: f32) {
        gl.uniform1f(Some(&self.size), size);
    }

    pub fn set_color(&self, gl: &gl, color: [f32; 4]) {
        gl.uniform4f(Some(&self.color), color[0], color[1], color[2], color[3]);
    }
}

pub struct Resolution {
    pub x: u32,
    pub y: u32,
}

impl Resolution {
    pub const DEFAULT: Self = Self { x: 64, y: 64 };
}

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<Point> for GlPoint2D {
    fn from(p: Point) -> GlPoint2D {
        GlPoint2D::new(p.x, p.y)
    }
}

impl std::ops::Sub for Point {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Div for Point {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl std::ops::Div<f32> for Point {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl std::ops::Mul<f32> for Point {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

pub struct Particle {
    position: Vec<GlPoint2D>,
    vector: Vec<GlPoint2D>,
    res: Resolution,
    velocity: f32,
}

impl Particle {
    const SPEED: f32 = 0.02;
    const MAX_VELOCITY: f32 = 2.0;

    pub fn new(res: Resolution) -> Self {
        let mut position = Vec::new();
        let mut vector = Vec::new();
        // OpenGL空間を指定解像度で分割して点を配置
        let (ix, iy) = (1. / res.x as f32, 1. / res.y as f32);
        for y in 0..res.y {
            for x in 0..res.x {
                position.push(GlPoint2D::new(
                    x as f32 * ix * 2.0 - 1.0,
                    y as f32 * iy * 2.0 - 1.0,
                ));
                vector.push(GlPoint2D::new(0.0, 0.0));
            }
        }
        Self {
            position,
            vector,
            res,
            velocity: Self::MAX_VELOCITY,
        }
    }

    fn index(&self, x: u32, y: u32) -> usize {
        y as usize * self.res.x as usize + x as usize
    }

    // 移動ベクトルの更新
    fn update_vector(pos: GlPoint2D, target: Point, vector: GlPoint2D) -> GlPoint2D {
        let mut delta = GlPoint2D::from(target) - pos;
        let r = delta.norm() * 5.0;
        if r != 0.0 {
            delta /= r;
        }
        delta += vector;
        let r = delta.norm();
        if r != 0.0 {
            delta /= r;
        }
        delta
    }

    // 目標点に向かって移動
    pub fn update(&mut self, target: Point, vector_update: bool) {
        match vector_update {
            true => {
                self.velocity = Self::MAX_VELOCITY;
            }
            false => {
                self.velocity *= 0.95;
            }
        }
        for x in 0..self.res.x {
            for y in 0..self.res.y {
                let i = self.index(x, y);
                if vector_update {
                    self.vector[i] = Self::update_vector(self.position[i], target, self.vector[i]);
                }
                self.position[i] += self.vector[i] * self.velocity * Self::SPEED;
            }
        }
    }
}

pub struct VertexVbo {
    vbo: WebGlBuffer,
    location: u32,
}

impl VertexVbo {
    const TARGET: GlEnum = gl::ARRAY_BUFFER;
    pub fn new(gl: &gl, data: &[GlPoint2D], location: u32) -> Result<Self> {
        let vbo = Self::create_vertex_buffer(
            gl,
            unsafe {
                std::slice::from_raw_parts(
                    data.as_ptr() as *const f32,
                    data.len() * GlPoint2D::size() as usize,
                )
            },
            location,
            gl::DYNAMIC_DRAW,
        )?;

        Ok(Self { vbo, location })
    }

    fn create_vertex_buffer(
        gl: &gl,
        data: &[f32],
        location: u32,
        usage: GlEnum,
    ) -> Result<WebGlBuffer> {
        let buffer = gl
            .create_buffer()
            .ok_or(Error::gl("Failed to create buffer object".into()))?;
        gl.bind_buffer(Self::TARGET, Some(&buffer));
        unsafe {
            let view = js_sys::Float32Array::view(&data);
            gl.buffer_data_with_array_buffer_view(Self::TARGET, &view, usage);
        }
        gl.enable_vertex_attrib_array(location);
        gl.vertex_attrib_pointer_with_i32(location, GlPoint2D::size(), gl::FLOAT, false, 0, 0);

        // GLES2.0と違ってVAOにつなぐのでunbing不要
        Ok(buffer)
    }

    // VBOの更新
    pub fn update_vertex(&self, gl: &gl, data: &[GlPoint2D]) {
        let data = unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const f32,
                data.len() * GlPoint2D::size() as usize,
            )
        };
        gl.bind_buffer(Self::TARGET, Some(&self.vbo));
        unsafe {
            let view = js_sys::Float32Array::view(&data);
            gl.buffer_sub_data_with_i32_and_array_buffer_view(Self::TARGET, 0, &view);
        }
        gl.enable_vertex_attrib_array(self.location);
        gl.vertex_attrib_pointer_with_i32(self.location, GlPoint2D::size(), gl::FLOAT, false, 0, 0);
    }
}
