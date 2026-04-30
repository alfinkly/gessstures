# Learnings

## Bevy 0.15 Camera API
- `Camera` component does NOT have `projection_matrix()` method (removed in Bevy 0.14+)
- Use `Camera::ndc_to_world(&self, camera_transform: &GlobalTransform, ndc: Vec3) -> Option<Vec3>` instead
- `ndc_to_world` takes NDC coordinates [-1,1] and returns world-space position at that NDC depth
- For ray casting: call with z=-1 (near plane) and z=1 (far plane) to get two world-space points, then compute ray direction and intersect with target plane
- Camera query pattern: `Query<(&Camera, &GlobalTransform), With<Camera3d>>`

## Module Structure
- `cursor_mapper.rs` and `nearest_vertex.rs` are self-contained utility modules
- They only depend on Bevy prelude and `graph_core` types
- `GraphNode` component is re-exported from `renderer::GraphNode` and accessed via `crate::renderer::GraphNode` path
- `GestureState` is in `gesture_detector` module, imported via `crate::gesture_detector::GestureState`
- `InteractionState` comes from `graph_core::InteractionState`
