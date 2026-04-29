use bevy::prelude::*;
use graph_core::NodeIndex;
use crate::renderer::GraphNode;

/// Find the nearest graph vertex to a 3D cursor position.
///
/// Iterates over all graph node entities, computes the Euclidean distance
/// to `cursor_pos`, and returns the closest vertex within `max_distance`.
///
/// Returns `(NodeIndex, distance)` or `None` if there are no vertices within
/// the search radius.
pub fn find_nearest_vertex(
    cursor_pos: Vec3,
    node_transforms: &Query<(&Transform, &GraphNode)>,
    max_distance: f32,
) -> Option<(NodeIndex, f32)> {
    let mut nearest: Option<(NodeIndex, f32)> = None;

    for (transform, graph_node) in node_transforms.iter() {
        let dist = transform.translation.distance(cursor_pos);
        if dist < max_distance {
            match nearest {
                Some((_, current_best)) if dist < current_best => {
                    nearest = Some((graph_node.0, dist));
                }
                None => {
                    nearest = Some((graph_node.0, dist));
                }
                _ => {}
            }
        }
    }

    nearest
}
