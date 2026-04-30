# Refactoring Log

## Wave 1: Domain Extraction (commit 62dce5b)
**Created hand-tracking-core crate** (zero deps).
- Moved: HandLandmark, HandLandmarkData, HandLandmarkResource
- Moved: Gesture, GestureClassifier, GestureResult (7 tests)
- Created: HandOverlayConfig (renamed from PipRect)
- Created: config.rs with all hardcoded constants
- Updated: app/Cargo.toml depends on hand-tracking-core with "bevy" feature

**Files removed from app/src/**: gesture_classifier.rs
**Files modified**: 8 files updated to use hand_tracking_core::*

## Wave 2: Layer Restructure (commit dc9dda4)
**Restructured app/src/ into layers**:
- interface/ — CLI + Bevy plugins
- app/ — Service modules (stubs)
- infrastructure/ — External adapters

**Files moved**:
| Old | New |
|-----|-----|
| camera_capture.rs | infrastructure/camera/capture_adapter.rs |
| preview_video.rs | infrastructure/camera/overlay_adapter.rs |
| sidecar.rs | infrastructure/ml/mediapipe_sidecar.rs |
| hand_tracking.rs | interface/plugins/hand_tracking_plugin.rs |
| preview.rs | interface/plugins/preview_plugin.rs |

## Wave 3: Cleanup (commit 2e28b45)
- Removed: video_overlay.rs (merged into overlay_adapter.rs)
- Renamed: HandRendererPlugin → SkeletonRendererPlugin
- Renamed: StatusLabel → SkeletonStatusLabel
- Extracted: All hardcoded constants (640, 480, 0.2, 8.0) to hand-tracking-core/src/config.rs
