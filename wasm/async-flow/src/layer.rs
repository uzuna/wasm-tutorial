use webgl2::{
    context::Context,
    shader::pointing::{PointingRequest, PointingShader},
    GlPoint2d,
};

use wasm_utils::{error::*, mouse::MouseEventMessage};

pub struct MouseShader {
    s: PointingShader,
}

impl MouseShader {
    pub fn new(ctx: &Context) -> Result<Self> {
        let mut s = PointingShader::new(ctx)?;
        s.enable(true);
        Ok(Self { s })
    }

    // mouseイベントを適用
    pub fn apply_event(&mut self, req: MouseEventMessage) {
        if let MouseEventMessage::Move { pos } = req {
            let pos = PointingRequest::Position(GlPoint2d {
                x: pos.x,
                y: pos.y,
            });
            self.s.apply_requests(&[pos]);
        }
    }

    pub fn update(&mut self, ts: f64) {
        self.s.update(ts as f32);
    }

    pub fn draw(&self) {
        self.s.draw();
    }
}
