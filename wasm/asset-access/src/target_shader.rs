use std::rc::Rc;

use wasm_utils::error::Result;
use web_sys::WebGlUniformLocation;
use webgl2::{
    context::Context,
    gl,
    program::Program,
    vertex::{Vao, VaoDefine},
    GlPoint2d,
};

/// 表示切り替え指示
pub enum TargetRequest {
    Enable(bool),
    Position(GlPoint2d),
}

pub struct TargetShader {
    prog: Program,
    uniform: TargetUniform,
    params: TargetParams,
    vao: Vao<TargetVd>,
    vertex_count: i32,
}

impl TargetShader {
    const VERT: &'static str = r#"#version 300 es
uniform vec2 target;
layout(location = 0) in vec2 position;

void main() {
    vec2 pos = target + position;
    gl_Position = vec4(pos, 0.0, 1.0);
}
"#;

    const FRAG: &'static str = r#"#version 300 es
precision mediump float;

uniform vec4 color;
uniform float alpha;
out vec4 fragmentColor;

void main() {
    fragmentColor = vec4(color.rgb, color.w * alpha);
}
"#;
    // 画面全体を覆うクロスの頂点
    const CROSS_VERTEX: [GlPoint2d; 4] = [
        GlPoint2d { x: -2.0, y: 0.0 },
        GlPoint2d { x: 2.0, y: 0.0 },
        GlPoint2d { x: 0.0, y: -2.0 },
        GlPoint2d { x: 0.0, y: 2.0 },
    ];

    pub fn new(ctx: &Context) -> Result<Self> {
        let prog = ctx.program(Self::VERT, Self::FRAG)?;
        prog.use_program();
        let uniform = TargetUniform::new(&prog)?;
        uniform.init();
        let params = TargetParams::default();
        let mut vao = prog.create_vao()?;
        vao.buffer_data(TargetVd::Position, &Self::CROSS_VERTEX, gl::STATIC_DRAW);
        vao.unbind();

        Ok(Self {
            prog,
            uniform,
            params,
            vao,
            vertex_count: Self::CROSS_VERTEX.len() as i32,
        })
    }

    pub fn apply_requests(&mut self, reqs: &[TargetRequest]) {
        let mut last_pos = None;
        for req in reqs {
            match req {
                TargetRequest::Enable(enable) => {
                    self.params.showing = *enable;
                }
                TargetRequest::Position(pos) => {
                    last_pos = Some(*pos);
                }
            }
        }
        if self.params.showing {
            if let Some(pos) = last_pos {
                self.prog.use_program();
                self.uniform.set_target(pos.x, pos.y);
            }
        }
    }

    pub fn update(&mut self, elapsed_sec: f32) {
        self.params.update(elapsed_sec);
    }

    pub fn draw(&self) {
        self.prog.use_program();
        self.uniform.set_alpha(self.params.alpha);
        let gl: &Rc<gl> = self.prog.gl();
        gl.line_width(self.params.line_width);
        self.vao.bind();
        gl.draw_arrays(gl::LINES, 0, self.vertex_count);
    }
}

pub struct TargetUniform {
    gl: Rc<gl>,
    target: WebGlUniformLocation,
    color: WebGlUniformLocation,
    alpha: WebGlUniformLocation,
}

impl TargetUniform {
    pub fn new(prog: &Program) -> Result<Self> {
        let gl = prog.gl().clone();
        let target = prog.uniform_location("target")?;
        let color = prog.uniform_location("color")?;
        let alpha = prog.uniform_location("alpha")?;
        Ok(Self {
            gl,
            target,
            color,
            alpha,
        })
    }

    fn init(&self) {
        self.set_target(0.0, 0.0);
        self.set_color([1.0, 1.0, 1.0, 1.0]);
        self.set_alpha(0.0);
    }

    pub fn set_target(&self, x: f32, y: f32) {
        self.gl.uniform2f(Some(&self.target), x, y);
    }

    pub fn set_color(&self, color: [f32; 4]) {
        self.gl.uniform4fv_with_f32_array(Some(&self.color), &color);
    }

    pub fn set_alpha(&self, alpha: f32) {
        self.gl.uniform1f(Some(&self.alpha), alpha);
    }
}

// レンダリングエフェクト効果パラメータ
#[derive(Debug, Clone, Copy)]
struct TargetParams {
    // ラインの太さ
    line_width: f32,
    // 表示有効中の太さ
    line_width_sustine: f32,
    // ラインの太さの減衰率
    line_width_release: f32,

    // alpha値
    alpha: f32,
    alpha_sustine: f32,
    alpha_release: f32,
    // 表示状態
    showing: bool,
}

impl Default for TargetParams {
    fn default() -> Self {
        Self {
            line_width: 0.0,
            line_width_sustine: Self::LINE_WIDTH_SUSTINE,
            line_width_release: Self::LINE_WIDTH_RELEASE,
            alpha: 0.0,
            alpha_sustine: Self::ALPHA_SUSTINE,
            alpha_release: Self::ALPHA_RELEASE,
            showing: false,
        }
    }
}

impl TargetParams {
    const LINE_WIDTH_SUSTINE: f32 = 3.0;
    const LINE_WIDTH_RELEASE: f32 = 0.22;
    const ALPHA_SUSTINE: f32 = 1.0;
    const ALPHA_RELEASE: f32 = 0.02;

    fn update(&mut self, elapsed_sec: f32) {
        if self.showing {
            self.alpha = self.alpha_sustine;
            self.line_width = self.line_width_sustine;
        } else {
            // 時間に対する一次減衰でモデリング
            self.alpha *= self.alpha_release.powf(elapsed_sec);
            self.line_width *= self.line_width_release.powf(elapsed_sec);
        }
    }
}

#[derive(Debug, PartialEq)]
enum TargetVd {
    Position,
}

impl VaoDefine for TargetVd {
    fn name(&self) -> &'static str {
        match self {
            TargetVd::Position => "position",
        }
    }

    fn size_of(&self) -> i32 {
        use webgl2::GlPoint;
        match self {
            TargetVd::Position => GlPoint2d::size(),
        }
    }

    fn iter() -> std::slice::Iter<'static, Self> {
        static VD: [TargetVd; 1] = [TargetVd::Position];
        VD.iter()
    }
}
