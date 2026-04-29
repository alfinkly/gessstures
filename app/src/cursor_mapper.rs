use bevy::prelude::*;

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
