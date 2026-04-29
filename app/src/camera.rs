use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use graph_core::{CameraCommand, CameraCommandKind};

pub struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OrbitState>()
            .add_systems(Update, orbit_camera);
    }
}

#[derive(Resource)]
struct OrbitState {
    focal_point: Vec3,
    radius: f32,
    yaw: f32,
    pitch: f32,
    dragging: bool,
    panning: bool,
    last_mouse: Option<Vec2>,
}

impl Default for OrbitState {
    fn default() -> Self {
        Self {
            focal_point: Vec3::ZERO,
            radius: 20.0,
            yaw: 0.0,
            pitch: 0.3,
            dragging: false,
            panning: false,
            last_mouse: None,
        }
    }
}

fn orbit_camera(
    mut state: ResMut<OrbitState>,
    mut query: Query<&mut Transform, With<Camera3d>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut scroll: EventReader<MouseWheel>,
    mut camera_commands: EventReader<CameraCommand>,
    windows: Query<&Window>,
    graph_query: Query<&Transform, (With<crate::renderer::GraphNode>, Without<Camera3d>)>,
) {
    let Ok(mut transform) = query.get_single_mut() else {
        return;
    };

    let window = windows.single();
    let cursor = window.cursor_position();

    // Handle CameraCommand events (from gestures or other sources)
    for cmd in camera_commands.read() {
        match &cmd.kind {
            CameraCommandKind::Orbit {
                delta_yaw,
                delta_pitch,
            } => {
                state.yaw -= *delta_yaw;
                state.pitch = (state.pitch - *delta_pitch).clamp(-1.5, 1.5);
            }
            CameraCommandKind::Pan { delta_x, delta_y } => {
                let right = transform.right().as_vec3() * -*delta_x;
                let up = transform.up().as_vec3() * *delta_y;
                state.focal_point += right + up;
            }
            CameraCommandKind::Zoom(amount) => {
                state.radius = (state.radius - amount).clamp(2.0, 100.0);
            }
            CameraCommandKind::Reset => {
                state.focal_point = Vec3::ZERO;
                state.radius = 20.0;
                state.yaw = 0.0;
                state.pitch = 0.3;
            }
        }
    }

    // Auto-focus on graph centroid (F key)
    if keys.just_pressed(KeyCode::KeyF) {
        let mut sum = Vec3::ZERO;
        let count = graph_query.iter().count();
        if count > 0 {
            for t in &graph_query {
                sum += t.translation;
            }
            state.focal_point = sum / count as f32;
        }
    }

    // Scroll to zoom
    for ev in scroll.read() {
        state.radius = (state.radius - ev.y * 2.0).clamp(2.0, 100.0);
    }

    // Reset view
    if keys.just_pressed(KeyCode::Space) {
        state.focal_point = Vec3::ZERO;
        state.radius = 20.0;
        state.yaw = 0.0;
        state.pitch = 0.3;
    }

    // Right mouse — orbit
    let right_pressed = mouse.pressed(MouseButton::Right);
    let middle_pressed = mouse.pressed(MouseButton::Middle);

    if right_pressed && !state.panning {
        if let Some(pos) = cursor {
            if let Some(last) = state.last_mouse {
                let delta = pos - last;
                if !state.dragging && delta.length() > 2.0 {
                    state.dragging = true;
                }
                if state.dragging {
                    state.yaw -= delta.x * 0.005;
                    state.pitch = (state.pitch - delta.y * 0.005)
                        .clamp(-1.5, 1.5);
                }
            }
        }
        state.last_mouse = cursor;
    } else if middle_pressed {
        if let Some(pos) = cursor {
            if let Some(last) = state.last_mouse {
                let delta = pos - last;
                if !state.panning && delta.length() > 2.0 {
                    state.panning = true;
                }
                if state.panning {
                    let right = transform.right().as_vec3() * -delta.x * 0.02;
                    let up = transform.up().as_vec3() * delta.y * 0.02;
                    state.focal_point += right + up;
                }
            }
        }
        state.last_mouse = cursor;
    } else {
        state.dragging = false;
        state.panning = false;
        state.last_mouse = cursor;
    }

    // Update camera position
    let pos = Vec3::new(
        state.radius * state.pitch.cos() * state.yaw.sin(),
        state.radius * state.pitch.sin(),
        state.radius * state.pitch.cos() * state.yaw.cos(),
    );
    transform.translation = state.focal_point + pos;
    transform.look_at(state.focal_point, Vec3::Y);
}
