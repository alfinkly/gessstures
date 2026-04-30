# Learnings - Graph Navigation Plugin

## Rust Module Privacy
- In Rust 2021 edition, private (non-pub) functions in a module are NOT accessible from sibling modules via path.
- They ARE accessible from parent modules (e.g., `main.rs` can reference child module's private items).
- To use a function as a Bevy `.after()` ordering label, both the source and target functions must be accessible.
- Workaround: make the function `pub(crate)` or skip explicit `.after()` — Bevy's event double-buffering provides natural one-frame delay.

## CameraCommand handling
- `OrbitState` fields can remain private. Instead of exposing the struct, added `EventReader<CameraCommand>` to the `orbit_camera` system.
- `CameraCommandKind::Orbit` applies deltas directly to `state.yaw` / `state.pitch` with clamping.
- Cam commands from gestures are consumed each frame, same as mouse input.

## Pre-existing bugs fixed
- `cursor` variable was missing from `camera.rs` — backported `let window = windows.single(); let cursor = window.cursor_position();`
- `Dir3` multiplication (`transform.right() * -delta.x * 0.02`) in Bevy 0.15 — fixed with `.as_vec3()`

## Files changed
- `app/src/camera.rs` — added `CameraCommand` event handling + fixed pre-existing bugs (`cursor`, `Dir3`)
- `app/src/graph_navigation.rs` — new module with `GraphNavigationPlugin` + `handle_graph_actions`
- `app/src/main.rs` — registered `GraphNavigationPlugin` + `mod graph_navigation`
