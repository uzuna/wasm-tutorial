use nalgebra_glm::Vec3;

/// 1つのボイドを表す構造体
#[derive(Debug, Clone, Copy)]
pub struct Boid {
    pos: Vec3,
    vel: Vec3,
}

impl Boid {
    pub fn new(pos: Vec3, vel: Vec3) -> Self {
        Self { pos, vel }
    }

    pub fn pos(&self) -> Vec3 {
        self.pos
    }

    pub fn zero() -> Self {
        Self {
            pos: Vec3::zeros(),
            vel: Vec3::zeros(),
        }
    }
}

#[derive(Debug)]
pub struct Boids {
    pub boids: Vec<Boid>,
    bounds: CubeBounds,
}

impl Boids {
    pub fn new(boids: Vec<Boid>) -> Self {
        Self {
            boids,
            bounds: CubeBounds::default(),
        }
    }

    pub fn new_circle(num: u32, radius: f32, velocity: f32) -> Self {
        let mut boids = Vec::with_capacity(num as usize);
        for i in 0..num {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / num as f32;
            let pos = Vec3::new(radius * angle.cos(), radius * angle.sin(), 0.0);
            let vel = Vec3::new(velocity * angle.cos(), velocity * angle.sin(), 0.0);
            boids.push(Boid::new(pos, vel));
        }

        Self {
            boids,
            bounds: CubeBounds::default(),
        }
    }

    pub fn update(&mut self) {
        for boid in &mut self.boids {
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
