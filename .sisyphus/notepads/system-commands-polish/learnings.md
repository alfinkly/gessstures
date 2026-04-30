# Learnings — System Commands + Polish (T19-T26)

## Gesture Action Pipeline
- `GestureEvent.swipe_direction: Option<f32>` contains `Some(1.0)` or `Some(-1.0)` for horizontal swipes (only during `Movement` gesture when cumulative delta exceeds threshold).
- Swipe dedup: track `last_swipe_direction` in `GestureActionState`, fire only on `None → Some(-1.0)` transition.
- Gesture transitions tracked via `state.prev_gesture`. VSIGN → FilterMode on enter, ← ExitFilterMode on leave.
- `GestureHoldEvent` fires exactly once when `frames_held >= target_frames`. Used for both Point (ReadContent) and OpenPalm (ResetView).

## Bevy Gizmos API (0.15)
- `Gizmos::linestrip(points, color)` — draws a 3D line strip through a vec of Vec3 points.
- `arc_3d(half_angle, radius, position, color)` — returns `Arc3dBuilder` (no `.rotation()` method in 0.15).
- For custom arc rotation: manually construct points in local XY plane, transform with `Quat::from_rotation_arc(Vec3::Z, direction)`.

## GraphAction enum pattern
- Keep `#[derive(Event, Debug, Clone)]` on GraphAction.
- All variants must be handled in `graph_navigation.rs::handle_graph_actions` match arm — compiler enforces exhaustiveness after adding new variants.
