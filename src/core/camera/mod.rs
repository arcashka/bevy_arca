use bevy::prelude::*;

use crate::render::ResizeEvent;

#[derive(Component)]
pub struct Camera {
    pub fov: f32,
    pub aspect_ratio: f32,
}

impl Camera {
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_lh(self.fov, self.aspect_ratio, 0.1, 100.0)
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_aspect_ratio);
    }
}

fn update_aspect_ratio(
    mut cameras: Query<&mut Camera>,
    mut resize_event: EventReader<ResizeEvent>,
) {
    let mut camera = cameras
        .get_single_mut()
        .expect("only 1 camera is supported right now");

    for resize_event in resize_event.read() {
        camera.aspect_ratio = resize_event.width / resize_event.height;
        info!("Aspect ratio of camera is {}", camera.aspect_ratio);
    }
}
