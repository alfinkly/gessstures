# Issues - Hand Tracking Webcam Preview

## Open Issues

### I-1: tract-tflite op compatibility (CRITICAL) [RESOLVED]
- Status: FAILED (Task 0 spike completed 2026-04-30)
- Description: MediaPipe "full" models use DEQUANTIZE op (f16→f32 conversion) which tract-tflite does NOT support. Both hand_landmark_full and palm_detection_full fail at model translation.
- Root cause: tract-tflite does not register a handler for DEQUANTIZE (builtin_code=6). 102-133 DEQUANTIZE ops per model.
- Additional: palm_detection also uses PRELU (not registered).
- Resolution: Task 0 spike confirmed incompatibility. Plan needs fallback.
- Fallback: (a) Convert models to float32-only (strip DEQUANTIZE, convert f16→f32 weights) using Python flatbuffer manipulation. (b) Python MediaPipe sidecar via subprocess. (c) ONNX Runtime (`ort` crate).

### I-2: Palm detection model missing from repo
- Status: PENDING (to be resolved by Task 2)
- Description: Only hand_landmark_full.tflite exists; palm_detection_full.tflite must be downloaded
- Source: https://storage.googleapis.com/mediapipe-assets/palm_detection_full.tflite

### I-3: Frame consumption bug
- Status: PENDING (to be resolved by Task 3)
- Description: hand_tracking.rs:91 guard.take() removes frame, starving video_overlay
- Fix: Replace take() with as_ref().cloned()

### I-4: RGBA→RGB format mismatch
- Status: PENDING (to be resolved by Task 4)
- Description: nokhwa outputs RGBA (4ch), MediaPipe models expect RGB (3ch)
