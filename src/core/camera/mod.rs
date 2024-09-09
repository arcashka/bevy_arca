use bevy::prelude::*;

#[derive(Component)]
pub struct Camera {
    pub fov: f32,
    pub aspect_ratio: f32,
}

impl Camera {
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect_ratio, 0.1, 100.0)
    }
}
