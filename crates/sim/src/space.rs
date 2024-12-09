use std::time::Duration;

use crate::unit::Vec3f;

/// 1つのシミュレーション物体の位置と位置に付随パラメータを保持する構造体
#[derive(Debug, Clone, Copy)]
pub struct Object {
    // 物体の生成時刻
    start_at: Duration,
    // 物体の位置
    pos: Vec3f,
    // 物体の速度
    vel: Vec3f,
}

impl Object {
    pub fn new(start_at: Duration, pos: Vec3f, vel: Vec3f) -> Self {
        Self { start_at, pos, vel }
    }

    pub fn start_at(&self) -> Duration {
        self.start_at
    }

    pub fn pos(&self) -> Vec3f {
        self.pos
    }

    pub fn update_pos(&mut self) {
        self.pos += self.vel;
    }

    pub fn update_vel(&mut self, f: impl Fn(Vec3f) -> Vec3f) {
        self.vel = f(self.vel);
    }
}
