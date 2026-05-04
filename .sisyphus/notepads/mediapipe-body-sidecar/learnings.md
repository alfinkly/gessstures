# Learnings — MediaPipe Body Sidecar

- Sidecar follows same pattern as `mediapipe_hands.py`: OpenCV camera → MediaPipe Task API → JSON stdout
- `vision.PoseLandmarker` from `mediapipe.tasks.python.vision` with `running_mode=vision.RunningMode.IMAGE`
- Bounding box computed from min/max of all 33 landmark x/y coordinates, then expanded by margin
- Coordinates are normalized [0,1] (MediaPipe convention)
- `num_poses=4` to detect up to 4 people
- Model: `models/pose_landmarker_full.task` (~9MB)
