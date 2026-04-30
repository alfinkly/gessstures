# T6 — Wire app/Cargo.toml and update all imports

## Summary
Successfully migrated `app` crate from local type definitions to `hand-tracking-core` external crate.

## Changes made
1. **hand-tracking-core/src/overlay_config.rs** — Added `#[cfg_attr(feature = "bevy", derive(...))] Resource` + `Clone, Debug` to `HandOverlayConfig` so it can be used as a Bevy resource (replacing `PipRect`).
2. **app/Cargo.toml** — Added `hand-tracking-core = { path = "../hand-tracking-core", features = ["bevy"] }`.
3. **app/src/gesture_classifier.rs** — Replaced entire file with `pub use hand_tracking_core::{Gesture, GestureClassifier, GestureResult};` since all logic moved to core.
4. **app/src/gesture_detector.rs** — Changed import from `crate::gesture_classifier` to `hand_tracking_core`.
5. **app/src/gesture_actions.rs** — Changed import from `crate::gesture_classifier` to `hand_tracking_core`.
6. **app/src/hand_tracking.rs** — Removed local `HandLandmark`, `HandLandmarkData`, `HandLandmarkResource`, `PipRect` definitions; replaced with `pub use hand_tracking_core::*`; kept `UseSidecar`, plugin code, and CV detection loop.
7. **app/src/hand_renderer.rs** — Changed `PipRect` → `HandOverlayConfig` in imports and type annotations.
8. **app/src/sidecar.rs** — Changed imports: `HandLandmark`, `HandLandmarkData`, `HandLandmarkResource` from `hand_tracking_core`; `UseSidecar` stays from `crate::hand_tracking`.
9. **app/src/preview_video.rs** — Changed `PipRect` → `HandOverlayConfig` in imports and `ResMut` type.
10. **app/src/preview.rs** — Changed `PipRect` → `HandOverlayConfig` in imports and `init_resource` calls.

## Verification
- `cargo check -p app` — zero errors
- `cargo test -p app` — 0 passed, 0 failed (tests moved to core)
- `cargo test -p hand-tracking-core` — 7 passed, 0 failed
