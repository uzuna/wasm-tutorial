use nalgebra_glm::Vec3;

/// 1つのボイドを表す構造体
#[derive(Debug, Clone, Copy)]
pub struct Boid {
    pos: Vec3,
    vel: Vec3,
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

    pub fn next_velocity(&self, boids: &[Boid]) -> Vec3 {
        let center = self.get_swarm_center_in_visual_range(boids);
        let avoid = self.get_avoidance(boids);
        let v = self.vel
            + (center - self.pos) * self.param.center_factor
            + avoid * self.param.avoid_factor;
        let norm = v.norm();
        if norm < self.param.speed_limit.0 {
            v * self.param.speed_limit.0 / norm
        } else if norm > self.param.speed_limit.1 {
            v / norm * self.param.speed_limit.1
        } else {
            v
        }
    }
}

/// ボイドの制御パラメータ
#[derive(Debug, Clone, Copy)]
pub struct BoidsParameter {
    speed_limit: (f32, f32),
    visual_range: f32,
    center_factor: f32,
    avoid_distance: f32,
    avoid_factor: f32,
}

impl Default for BoidsParameter {
    fn default() -> Self {
        Self {
            speed_limit: (0.005, 0.01),
            visual_range: 0.2,
            center_factor: 0.01,
            avoid_distance: 0.05,
            avoid_factor: 0.01,
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
}

impl CubeBounds {
    fn keep_within(&self, b: &mut Boid) {
        // 現時点では単純に移動方向を反転させるだけ
        if b.pos.x < self.x.0 || b.pos.x > self.x.1 {
            b.vel.x = -b.vel.x
        }

        if b.pos.y < self.y.0 || b.pos.y > self.y.1 {
            b.vel.y = -b.vel.y
        }

        if b.pos.z < self.z.0 || b.pos.z > self.z.1 {
            b.vel.z = -b.vel.z
        }
    }
}

impl Default for CubeBounds {
    fn default() -> Self {
        Self {
            x: (-1.0, 1.0),
            y: (-1.0, 1.0),
            z: (-1.0, 1.0),
        }
    }
}
