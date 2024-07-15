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
}

#[derive(Debug)]
pub struct Boids {
    boids: Vec<Boid>,
}

impl Boids {
    pub fn new(boids: Vec<Boid>) -> Self {
        Self { boids }
    }

    pub fn update(&mut self) {
        for boid in &mut self.boids {
            boid.pos += boid.vel;
        }
    }
}
