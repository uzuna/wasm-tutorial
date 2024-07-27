use nalgebra_glm::Vec3;

/// 1つのボイドを表す構造体
#[derive(Debug, Clone, Copy)]
pub struct Boid {
    pos: Vec3,
    vel: Vec3,
    // ボイドの制御パラメータ。
    // 遺伝的アルゴリズムで最適化することを考えているので個別にもたせる
    param: BoidsParameter,
}

impl Boid {
    pub fn new(pos: Vec3, vel: Vec3, param: BoidsParameter) -> Self {
        Self { pos, vel, param }
    }

    pub fn pos(&self) -> Vec3 {
        self.pos
    }

    pub fn zero() -> Self {
        Self {
            pos: Vec3::zeros(),
            vel: Vec3::zeros(),
            param: BoidsParameter::default(),
        }
    }

    pub fn distance(&self, other: &Boid) -> f32 {
        (self.pos - other.pos).norm()
    }

    fn get_swarm_center_in_visual_range(&self, boids: &[Boid]) -> Vec3 {
        let mut center = Vec3::zeros();
        let mut count = 0;
        for boid in boids {
            if self.distance(boid) < self.param.visual_range {
                center += boid.pos;
                count += 1;
            }
        }

        if count == 0 {
            return center;
        }

        center / count as f32
    }

    fn get_avoidance(&self, boids: &[Boid]) -> Vec3 {
        let mut avoid = Vec3::zeros();
        for boid in boids {
            if self.distance(boid) < self.param.avoid_distance {
                avoid += self.pos - boid.pos;
            }
        }

        avoid
    }

    fn get_alingment(&self, boids: &[Boid]) -> Vec3 {
        let mut align = Vec3::zeros();
        let mut count = 0;
        for boid in boids {
            if self.distance(boid) < self.param.visual_range {
                align += boid.vel;
                count += 1;
            }
        }

        if count == 0 {
            return align;
        }

        align / count as f32
    }

    pub fn next_velocity(&self, boids: &[Boid]) -> Vec3 {
        let center = self.get_swarm_center_in_visual_range(boids);
        let avoid = self.get_avoidance(boids);
        let align = self.get_alingment(boids);
        let v = self.vel
            + (center - self.pos) * self.param.center_factor
            + avoid * self.param.avoid_factor
            + align * self.param.alignment_factor;
        let norm = v.norm();
        if norm < self.param.speed_limit.0 {
            v * self.param.speed_limit.0 / norm
        } else if norm > self.param.speed_limit.1 {
            v / norm * self.param.speed_limit.1
        } else {
            v
        }
    }

    pub fn get_param_mut(&mut self) -> &mut BoidsParameter {
        &mut self.param
    }
}

/// ボイドの制御パラメータ
#[derive(Debug, Clone, Copy)]
pub struct BoidsParameter {
    // 速度の制限min, max
    speed_limit: (f32, f32),
    // centering, alignmentで使う可視範囲
    visual_range: f32,
    // 見えている群れの中央に向かう力の強さ
    center_factor: f32,
    // 避ける対象となる最小距離
    avoid_distance: f32,
    // 避ける力の強さ
    avoid_factor: f32,
    // 同じ方向に進もうとする力の強さ
    alignment_factor: f32,
}

impl BoidsParameter {
    pub fn set_visual_range(&mut self, visual_range: f32) {
        self.visual_range = visual_range;
    }
    pub fn set_center_factor(&mut self, center_factor: f32) {
        self.center_factor = center_factor;
    }
    pub fn set_alignment_factor(&mut self, alignment_factor: f32) {
        self.alignment_factor = alignment_factor;
    }
    pub fn set_avoid_distance(&mut self, avoid_distance: f32) {
        self.avoid_distance = avoid_distance;
    }
    pub fn set_avoid_factor(&mut self, avoid_factor: f32) {
        self.avoid_factor = avoid_factor;
    }
    pub fn set_speed_min(&mut self, speed_min: f32) {
        self.speed_limit.0 = speed_min;
    }
    pub fn set_speed_max(&mut self, speed_max: f32) {
        self.speed_limit.1 = speed_max;
    }
}

impl Default for BoidsParameter {
    fn default() -> Self {
        Self {
            speed_limit: (0.005, 0.01),
            visual_range: 0.2,
            center_factor: 0.005,
            avoid_distance: 0.05,
            avoid_factor: 0.01,
            alignment_factor: 0.05,
        }
    }
}

#[derive(Debug)]
pub struct Boids {
    pub boids: Vec<Boid>,
    vel_cache: Vec<Vec3>,
    bounds: CubeBounds,
}

impl Boids {
    pub fn new(boids: Vec<Boid>) -> Self {
        let vel_cache = vec![Vec3::zeros(); boids.len()];
        Self {
            boids,
            bounds: CubeBounds::default(),
            vel_cache,
        }
    }

    pub fn new_circle(num: u32, radius: f32, velocity: f32) -> Self {
        let mut boids = Vec::with_capacity(num as usize);
        for i in 0..num {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / num as f32;
            let pos = Vec3::new(radius * angle.cos(), radius * angle.sin(), 0.0);
            let vel = Vec3::new(velocity * angle.cos(), velocity * angle.sin(), 0.0);
            boids.push(Boid::new(pos, vel, BoidsParameter::default()));
        }

        Self::new(boids)
    }

    pub fn update(&mut self) {
        for (b, v) in self.boids.iter().zip(self.vel_cache.iter_mut()) {
            *v = b.next_velocity(&self.boids);
        }
        for (boid, v) in self.boids.iter_mut().zip(self.vel_cache.iter()) {
            boid.vel = *v;
            self.bounds.keep_within(boid);
        }

        for boid in &mut self.boids {
            boid.pos += boid.vel;
        }
    }
}

/// キューブ上の空間境界を表す構造体
#[derive(Debug)]
pub struct CubeBounds {
    x: (f32, f32),
    y: (f32, f32),
    z: (f32, f32),
    gain: f32,
}

impl CubeBounds {
    fn keep_within(&self, b: &mut Boid) {
        // 現時点では単純に移動方向を反転させるだけ
        if b.pos.x < self.x.0 {
            b.vel.x += self.gain
        } else if b.pos.x > self.x.1 {
            b.vel.x -= self.gain
        }
        if b.pos.y < self.y.0 {
            b.vel.y += self.gain
        } else if b.pos.y > self.y.1 {
            b.vel.y -= self.gain
        }
    }
}

impl Default for CubeBounds {
    fn default() -> Self {
        Self {
            x: (-1.0, 1.0),
            y: (-1.0, 1.0),
            z: (-1.0, 1.0),
            gain: 0.0005,
        }
    }
}
