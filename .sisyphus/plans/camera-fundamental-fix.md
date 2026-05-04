# Camera Fundamental Fix

## Problem
Capture thread writes `CameraFrame` to `Arc<Mutex<Option<CameraFrame>>>`, but readers always see `None`. Root cause unknown — could be mutex poisoning, timing, or consumer interference.

## Solution
**Remove `Option` entirely.** Frame is always present. No `take()`, no `None`, no ambiguity.

## Changes

### 1. `capture_adapter.rs` — CameraFrame always Some
- Remove `Option` from `Arc<Mutex<Option<CameraFrame>>>` → `Arc<Mutex<CameraFrame>>`
- Remove `format`, `timestamp` fields (unused everywhere)
- Initialize with placeholder data (gray 640×480)
- Capture loop: always write to `*guard` (no `if let Ok`)
- Remove `CameraResource::new()` tuple return (no longer needed)

### 2. All consumers — read without Option
Update all files that read `CameraResource.frame`:
- `camera_preview.rs` — `guard.as_ref().map(...)` → `guard.data.clone(), guard.width, guard.height`
- `overlay_adapter.rs` — same change
- `camera.rs` (if it reads frame) — same change
- `hand_tracking_plugin.rs` — same change
- `mediapipe_sidecar.rs` — same change
- `person_tracker.rs` — same change
- `capture_adapter.rs` itself — same change

### 3. The `new()` function
```rust
impl CameraResource {
    pub fn new() -> Self {
        Self {
            frame: Arc::new(Mutex::new(CameraFrame {
                data: vec![128u8; (640 * 480 * 4) as usize],
                width: 640,
                height: 480,
            })),
            is_active: Arc::new(AtomicBool::new(false)),
        }
    }
}
```

### 4. Capture loop write
```rust
if let Ok(mut guard) = shared_frame.lock() {
    *guard = CameraFrame { data, width: w, height: h };
}
```
Note: still uses `if let Ok` to avoid panic on poisoned lock, but now ALWAYS writes valid data.

### Files to modify (8 total):
1. `app/src/infrastructure/camera/capture_adapter.rs`
2. `app/src/infrastructure/camera/camera_preview.rs`
3. `app/src/infrastructure/camera/overlay_adapter.rs`
4. `app/src/interface/plugins/hand_tracking_plugin.rs`
5. `app/src/infrastructure/ml/mediapipe_sidecar.rs`
6. `app/src/infrastructure/body/person_tracker.rs`
7. `app/src/renderer.rs` (if it reads camera — check)
8. `app/src/interface/plugins/preview_plugin.rs` (check)
