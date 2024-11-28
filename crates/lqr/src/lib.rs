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
    use ndarray::{array, Array2};

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
}
