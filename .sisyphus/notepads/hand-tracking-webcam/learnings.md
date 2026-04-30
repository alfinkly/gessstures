# Learnings - Hand Tracking Webcam Preview

## Project Conventions
- Bevy 0.15, nokhwa 0.10 (input-native), image 0.25
- All plugins in `app/src/` as separate files, registered in `main.rs`
- Camera capture: dedicated std::thread with Arc<Mutex<Option<CameraFrame>>>
- Hand tracking: separate std::thread, writes to HandLandmarkResource
- Hand rendering: Camera2d (order:1) with gizmos for skeleton + text HUD
- Rendering order: Camera3d (order:0) → Camera2d (order:1, no clear)
- Landmark coordinates: normalized [0,1]
- SKELETON_CONNECTIONS: standard MediaPipe 21-point topology

## Tract-tflite Integration Patterns
- tract-tflite = "0.22" for stable
- Model loading: tract_tflite::tflite().model_for_path(path)?
- Input tensor: tract::Tensor::from_shape(&[1, dim, dim, 3], &float32_data)?
- Run: model.run(tvec!(input))?
- Output: TypedSimpleState[0] → Tensor → .to_array_view::<f32>()?

## MediaPipe Model Details
- hand_landmark_full.tflite input: [1, 224, 224, 3] float32, normalized [0,1]
- hand_landmark_full.tflite output: [1, 63] (21 landmarks × xyz)
- palm_detection_full.tflite input: [1, 192, 192, 3] float32, normalized [0,1]
- palm_detection_full.tflite output: [1, 7] or [1, 1, 7, 1] (cx, cy, w, h, rot, score, handedness)
- Rotation crop: affine transform based on palm rect → 192×192

## Task 0 - tract-tflite Spike Results (2026-04-30)

### Result: FAIL - tract-tflite CANNOT load MediaPipe "full" TFLite models

**Root cause**: Both `hand_landmark_full.tflite` and `palm_detection_full.tflite`
use the TFLite DEQUANTIZE operator (builtin_code=6), which is NOT implemented
in tract-tflite (neither 0.22.1 nor 0.23.0-dev.5).

- hand_landmark: 102 F16 tensors, 102 DEQUANTIZE ops
- palm_detection: 133 F16 tensors, 133 DEQUANTIZE ops

The models store weights in float16 format and convert to float32 at runtime
via DEQUANTIZE ops. tract-tflite doesn't register a handler for DEQUANTIZE
in its op registry.

**Additional issue**: palm_detection also uses PRELU (builtin_code=54), which
is also NOT registered in tract-tflite (only LEAKY_RELU is supported).

**Recommended fallback**: Either:
1. Convert models to float32-only (strip DEQUANTIZE, convert f16→f32 weights)
2. Use Python MediaPipe sidecar for hand tracking
3. Switch to ONNX Runtime (`ort` crate) with ONNX-converted models
4. Fork tract-tflite and add DEQUANTIZE+PRELU support

### Op Compatibility Summary
| Op | hand_landmark | palm_detection | tract-tflite |
|----|--------------|----------------|--------------|
| CONV_2D | ✅ | ✅ | ✅ Supported |
| DEPTHWISE_CONV_2D | ✅ | ✅ | ✅ Supported |
| ADD | ✅ | ✅ | ✅ Supported |
| MAX_POOL_2D | - | ✅ | ✅ Supported |
| RESHAPE | ✅ | ✅ | ✅ Supported |
| LOGISTIC | ✅ | - | ✅ Supported |
| PAD | - | ✅ | ✅ Supported |
| CONCATENATION | - | ✅ | ✅ Supported |
| MEAN | ✅ | - | ✅ Supported |
| DEQUANTIZE | ✅ | ✅ | ❌ NOT Supported |
| PRELU | - | ✅ | ❌ NOT Supported |

## Task 1 - Python MediaPipe Sidecar (2026-04-30)

### Result: SUCCESS
- `pip3 install mediapipe opencv-python numpy` completed successfully
- `app/mediapipe_hands.py` created — Python sidecar with MediaPipe Hands
- Sidecar opens camera, runs MediaPipe, outputs JSON to stdout with `flush=True`
- Tested: graceful error when no camera available, valid JSON output
- Camera not available in headless env; would show 21 landmarks when camera present

### Key Details
- MediaPipe 0.10.35, OpenCV 4.13.0, NumPy 2.3.2
- Resolution set to 640x480 (matching nokhwa capture)
- Output format: `{"detected": true/false, "landmarks": [[x,y,z]*21], "handedness": "Left/Right", "timestamp": float}`
- Clean shutdown via KeyboardInterrupt/SystemExit handling
- Camera and model resources released in `finally` block

## Common Gotchas
- guard.take() in hand_tracking.rs consumes frame — use .as_ref().cloned() instead
- nokhwa outputs RGBA (4ch), MediaPipe expects RGB (3ch)
- Camera2d must NOT clear (ClearColorConfig::None) to overlay on Camera3d
- Bevy Plugins trait implemented for tuples up to 15 elements; split into multiple `.add_plugins()` calls for more
- For full-screen 2D sprite with letterboxing: spawn `Sprite` with `Transform::from_scale(scale)`, compute `scale = (win_w / video_w).min(win_h / video_h)`
- PreviewVideoPlugin uses `Camera2d` with `order: 0` (background), HandRendererPlugin uses `order: 1` (skeleton overlay)
- `Sprite` component now used directly (no `SpriteBundle`) in Bevy 0.15
- Texture update pattern: lock CameraResource.frame → clone data → images.get_mut → copy_from_slice

## Task 4 - Rust Sidecar IPC Integration (2026-04-30)

### Key Details
- `app/src/sidecar.rs` created — SidecarPlugin that spawns Python sidecar process
- Sidecar reads JSON lines from stdout, parses into HandLandmarkResource
- `UseSidecar` marker resource added to `hand_tracking.rs`
- `start_hand_tracking()` checks for UseSidecar and skips detection loop when present
- Clean shutdown via Drop impl on SidecarResource (kills child + signals reader thread)
- Auto-restart with 1s delay on sidecar crash
- serde + serde_json added to app/Cargo.toml

### Architecture
- SidecarPlugin inserts `UseSidecar` + `start_sidecar` Startup system
- Reader thread manages child lifecycle: spawn → read → restart on crash
- UseSidecar prevents old detection_loop from running in preview mode
- Desktop mode unchanged (no UseSidecar → detection_loop runs as before)
- SidecarPlugin wired via `mod sidecar` in main.rs but not added to app yet (Task 5)

## Task 5 - Preview Plugin Integration (2026-04-30)

### Result: SUCCESS
- `app/src/preview.rs` created with `PreviewPlugin`
- `main.rs` updated: `mod preview`, `use preview::PreviewPlugin`, `Commands::Preview { camera: u32 }`, match arm
- Preview mode starts: CameraCapturePlugin → PreviewVideoPlugin → SidecarPlugin → HandRendererPlugin
- No 3D graph, physics, orbit camera, or gesture detection in preview mode
- "Loading MediaPipe..." text shown on startup, fades after 5s or first hand detection
- Esc or Q closes preview (sends AppExit)
- Desktop mode unchanged
- `cargo check -p app` passes (no new warnings)
- `cargo test -p app` passes (7/7 tests pass)
- HandRendererPlugin uses `Res<GestureState>` which doesn't exist in preview mode — Bevy 0.15 gracefully skips the `update_status_text` system when the resource is absent
- CameraCapturePlugin currently hardcodes camera index 0; `_cam_idx` parameter reserved for future use
