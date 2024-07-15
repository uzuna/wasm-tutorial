use nalgebra_glm::{TMat4, Vec3};

pub struct ViewMatrix {
    eye: Vec3,
    center: Vec3,
    up: Vec3,
}

impl ViewMatrix {
    pub const DEFAULT: Self = Self {
        eye: Vec3::new(0.0, 0.0, 3.0),
        center: Vec3::new(0.0, 0.0, 0.0),
        up: Vec3::new(0.0, 1.0, 0.0),
    };

    pub const fn new(eye: Vec3, center: Vec3, up: Vec3) -> Self {
        Self { eye, center, up }
    }

    pub fn look_at(&self) -> TMat4<f32> {
        nalgebra_glm::look_at(&self.eye, &self.center, &self.up)
    }
}

impl Default for ViewMatrix {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub struct Camera {
    aspect: f32,
    fovy: f32,
    near: f32,
    far: f32,
}

impl Camera {
    const DEFAULT: Self = Self {
        aspect: 1.0,
        fovy: 45.0,
        near: 0.1,
        far: 100.0,
    };

    const fn new(aspect: f32, fovy: f32, near: f32, far: f32) -> Self {
        Self {
            aspect,
            fovy,
            near,
            far,
        }
    }

    pub fn perspective(&self) -> TMat4<f32> {
        nalgebra_glm::perspective(
            self.aspect,
            self.fovy * std::f32::consts::PI / 180.0,
            self.near,
            self.far,
        )
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::DEFAULT
    }
}
