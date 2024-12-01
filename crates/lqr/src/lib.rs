pub mod sim {
    //! シミュレーションモデルの定義

    pub type Vec2 = nalgebra::Vector2<f32>;
    pub type Mat2 = nalgebra::Matrix2<f32>;

    /// 2次元の空間内で指定した分だけ移動する単純なモデル
    pub struct DeltaPoint2 {
        p: Vec2,
    }

    impl DeltaPoint2 {
        pub fn new(p: Vec2) -> Self {
            Self { p }
        }

        pub fn apply(&mut self, p: Vec2) -> Vec2 {
            self.p += p;
            self.p
        }

        pub fn pos(&self) -> Vec2 {
            self.p
        }
    }
}

#[cfg(test)]
mod tests {
    use ndarray::{array, Array1, Array2, Array3};

    #[test]
    fn test_dare() {
        let a = array!([[1.1, 2.0], [0.0, 0.95]]);
        let b = array!([0.0, 0.0787]);
        let c = array!([-2.0, 1.0]);
        let x = &a * &b;
        let x = &a * &b.t();
        let x = &b.t() * &b;

        let p: Array2<f32> = Array2::eye(2);
        let z = &a.t() * &p + &p * &a - &p * &b.t() * &b * &p + &c.t() * &c;
        println!("{:?}", z);
    }

    #[test]
    fn test_troc() {
        // https://qiita.com/harmegiddo/items/ddd33f40d5e368a210df

        pub struct Troc {
            pos: Array1<f32>,
            vel: Array1<f32>,
        }

        // トロッコの新地を作る部分
        // 位置と加速度
        let x: Array2<f32> = array!([0.], [0.]);

        // 運動方程式
        let dt = 0.1_f32;
        let f = array![[1., dt], [0., 1.]];

        // ノイズモデル
        let g = array![[0.5 * dt.powi(2)], [dt]];

        let w = 0.0_32;

        let mut x_t = x.clone();
        for _ in 0..10 {
            // 状態更新、
            x_t = (&f * &x_t) + (&g);
            println!("{:?}", x_t);
        }
    }
}
