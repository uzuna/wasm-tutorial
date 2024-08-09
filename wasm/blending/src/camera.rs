type Point3f = nalgebra::Point3<f32>;
type Vec3f = nalgebra::Vector3<f32>;
type Mat4f = nalgebra::Matrix4<f32>;

pub struct ViewMatrix {
    pub eye: Point3f,
    pub center: Point3f,
    pub up: Vec3f,
}

impl ViewMatrix {
    pub const DEFAULT: Self = Self {
        eye: Point3f::new(0.0, 0.0, 5.0),
        center: Point3f::new(0.0, 0.0, 0.0),
        up: Vec3f::new(0.0, 1.0, 0.0),
    };

    pub const fn new(eye: Point3f, center: Point3f, up: Vec3f) -> Self {
        Self { eye, center, up }
    }

    pub fn look_at(&self) -> Mat4f {
        Mat4f::look_at_rh(&self.eye, &self.center, &self.up)
    }
}

impl Default for ViewMatrix {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub struct Camera {
    pub aspect: f32,
    pub fovy: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    const DEFAULT: Self = Self {
        aspect: 1.0,
        fovy: 90.0,
        near: 0.1,
        far: 100.0,
    };

    pub fn perspective(&self) -> nalgebra::Perspective3<f32> {
        nalgebra::Perspective3::new(
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
