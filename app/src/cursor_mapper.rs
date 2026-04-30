use bevy::prelude::*;
use hand_tracking_core::Gesture;

use crate::gesture_detector::GestureState;

/// Maps normalized 2D hand cursor [0,1] to 3D world space position on the z=0 graph plane.
///
/// Uses Bevy's [`Camera::ndc_to_world`] to construct a ray through the camera frustum,
/// then intersects with the z=0 plane where the graph sits.
pub fn map_hand_to_3d(
    hand_x: f32,
    hand_y: f32,
    camera_transform: &GlobalTransform,
    camera: &Camera,
) -> Option<Vec3> {
    // Convert hand position [0,1] to NDC [-1, 1]
    let ndc_x = hand_x * 2.0 - 1.0;
    let ndc_y = 1.0 - hand_y * 2.0; // flip Y (screen top-left to NDC bottom-left)

    // Get near and far plane points in world space
    let near = camera.ndc_to_world(camera_transform, Vec3::new(ndc_x, ndc_y, -1.0))?;
    let far = camera.ndc_to_world(camera_transform, Vec3::new(ndc_x, ndc_y, 1.0))?;

    // Ray direction
    let dir = (far - near).normalize();

    // Intersect ray with z = 0 plane
    if dir.z.abs() <= f32::EPSILON {
        return None; // Ray parallel to the graph plane
    }

    let t = -near.z / dir.z;
    if t <= 0.0 {
        return None; // Intersection is behind the camera
    }

    Some(near + dir * t)
}

/// Draws a laser line from the camera through the cursor position when a Point
/// gesture is active. The line is red when pointing into void, green when
/// hovering a node.
pub fn draw_laser_system(
    gesture_state: Res<GestureState>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    interaction: Res<graph_core::InteractionState>,
    mut gizmos: Gizmos,
) {
    if !gesture_state.hand_detected || gesture_state.current_gesture != Gesture::Point {
        return;
    }

    let Ok((cam, cam_transform)) = camera_q.get_single() else {
        return;
    };

    if let Some(world_pos) = map_hand_to_3d(
        gesture_state.cursor_x,
        gesture_state.cursor_y,
        cam_transform,
        cam,
    ) {
        let camera_pos = cam_transform.translation();
        let color = if interaction.hovered_node.is_some() {
            Color::srgb(0.0, 1.0, 0.0)
        } else {
            Color::srgb(1.0, 0.0, 0.0)
        };
        gizmos.line(camera_pos, world_pos, color);
    }
}
