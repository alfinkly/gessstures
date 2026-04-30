# Problems - Hand Tracking Webcam Preview

## Blocking Problems

### P-1: tract-tflite compatibility with MediaPipe ops [RESOLVED]
Severity: BLOCKING → PIVOT REQUIRED
Description: tract-tflite CANNOT run MediaPipe TFLite models on ARM64 macOS due to missing DEQUANTIZE op support. Both hand_landmark_full and palm_detection_full fail.
Check: Task 0 spike COMPLETE (2026-04-30)
Resolution: Plan needs to pivot. Options:
  A) Add model conversion step: Use Python to convert f16→f32 TFLite models (strip DEQUANTIZE/PRELU)
  B) Replace tract-tflite with ort (ONNX Runtime Rust)
  C) Use Python MediaPipe sidecar process
  D) Fork tract-tflite to add DEQUANTIZE+PRELU support (simplest code change, ~20 lines)
Recommended path: D (fork tract-tflite) or A (convert models) - keeps Rust-only stack
