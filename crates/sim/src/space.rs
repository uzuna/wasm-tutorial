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

    /// 速度が確定しているなら位置が更新可能
    pub fn update_pos(&mut self, dt: Duration) {
        self.pos += self.vel * dt.as_secs_f32();
    }

    /// 速度を更新する
    pub fn update_vel(&mut self, dt: Duration, f: impl Fn(&Self, Duration) -> Vec3f) {
        self.vel = f(self, dt);
    }
}
