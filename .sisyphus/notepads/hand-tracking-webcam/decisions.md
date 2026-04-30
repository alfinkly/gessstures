# Decisions - Hand Tracking Webcam Preview

## Architecture Decisions

### AD-1: tract-tflite as ML engine
- Status: ACCEPTED
- Rationale: mediapipe-rs not available on crates.io; ux-mediapipe (1 star) unreliable; tract-tflite actively maintained with ARM64 macOS support
- Alternative considered: Python MediaPipe sidecar (rejected for Rust-native requirement)
- Date: 2026-04-30

### AD-2: Preview mode as separate subcommand
- Status: ACCEPTED
- Rationale: Clean separation from desktop 3D graph mode; enables focused optimization; no risk of breaking existing functionality
- CLI: `cargo run -p app -- preview`

### AD-3: Models not committed to git
- Status: ACCEPTED
- Rationale: 5.2MB + ~8-10MB binary bloat; use scripts/download_models.sh instead
- Exemption: hand_landmark_full.tflite already committed in previous plan

### AD-4: MediaPipe pipeline only in preview mode
- Status: ACCEPTED
- Rationale: Desktop mode keeps old detect_hand_cv() to prevent regressions; MediaPipe pipeline preview-only
- Revisit: Could be enabled for desktop in future plan

### AD-5: Single hand only
- Status: ACCEPTED
- Rationale: MediaPipe supports up to 2 hands but adds complexity; single hand sufficient for current use case
- Scope: Explicitly excluded

### AD-6: TDD for pipeline components
- Status: ACCEPTED
- Rationale: Complex coordinate math needs validation; synthetic tensors for isolated testing
- Pattern: RED (failing test) → GREEN (minimal impl) → REFACTOR
