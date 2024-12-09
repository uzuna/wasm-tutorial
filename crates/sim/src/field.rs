//! 速度に作用する力を計算する関数

use crate::unit::Vec3f;

/// 重力様の力のフィールド
///
/// 中心に向かって力が発生する
/// 力の大きさは距離の二乗に反比例する
/// 距離が一定値以下の場合は力の置きさを0とする
pub struct Gravity {
    /// 重力発生の中心
    pos: Vec3f,
    /// 力の大きさ
    g: f32,
    /// 力を0にする近距離境界
    eps: f32,
}

impl Gravity {
    /// 重力の新しいインスタンスを生成する
    pub fn new(pos: Vec3f, g: f32, eps: f32) -> Self {
        Self { pos, g, eps }
    }

    /// 重力の中心を取得する
    pub fn pos(&self) -> Vec3f {
        self.pos
    }

    /// 重力の大きさを取得する
    pub fn g(&self) -> f32 {
        self.g
    }

    /// 重力の中心を更新する
    pub fn set_pos(&mut self, pos: Vec3f) {
        self.pos = pos;
    }

    /// 重力の大きさを更新する
    pub fn set_g(&mut self, g: f32) {
        self.g = g;
    }

    /// 重力の中心から物体にかかる力を計算する
    pub fn calc_force(&self, pos: Vec3f) -> Vec3f {
        let dir = self.pos - pos;
        let dist = dir.norm();
        if dist < self.eps {
            Vec3f::new(0.0, 0.0, 0.0)
        } else {
            dir.normalize() * self.g / dist.powi(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gravity() {
        let g = 9.8;
        let eps = 0.02;
        let gf = Gravity::new(Vec3f::new(0.0, 0.0, 0.0), g, eps);
        let td = [
            (0.0, 0.0),
            (0.01, 0.0),
            (0.5, -9.8 / 0.5f32.powi(2)),
            (1.0, -9.8),
            (2.0, -9.8 / 2f32.powi(2)),
            (3.0, -9.8 / 3f32.powi(2)),
        ];
        for (d, f) in td.iter() {
            let pos = Vec3f::new(*d, 0.0, 0.0);
            let force = gf.calc_force(pos);
            assert_eq!(force, Vec3f::new(*f, 0.0, 0.0));
        }
    }
}
