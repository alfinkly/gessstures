"""MediaPipe Pose Landmarker Sidecar — reads frames from stdin.
Receives WIDTH HEIGHT line + raw RGBA bytes, outputs JSON to stdout."""

import json
import time
import sys
import os
import numpy as np
import mediapipe as mp
from mediapipe.tasks import python
from mediapipe.tasks.python import vision


def _read_frame():
    """Read one frame: WIDTH HEIGHT\\n + raw RGBA bytes from binary stdin."""
    buf = sys.stdin.buffer
    dims = b""
    while True:
        ch = buf.read(1)
        if not ch:
            return None
        if ch == b"\n":
            break
        dims += ch
    parts = dims.split()
    if len(parts) < 2:
        return _read_frame()
    w, h = int(parts[0]), int(parts[1])
    frame_size = w * h * 4
    raw_data = buf.read(frame_size)
    if len(raw_data) < frame_size:
        return None
    return w, h, raw_data


def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    model_path = os.path.normpath(os.path.join(script_dir, "..", "models", "pose_landmarker_full.task"))
    if not os.path.exists(model_path):
        print(json.dumps({"error": f"Model not found: {model_path}", "person_count": 0, "persons": []}), flush=True)
        sys.exit(1)

    base_options = python.BaseOptions(model_asset_path=model_path)
    options = vision.PoseLandmarkerOptions(
        base_options=base_options, running_mode=vision.RunningMode.IMAGE,
        num_poses=4, min_pose_detection_confidence=0.5,
    )
    landmarker = vision.PoseLandmarker.create_from_options(options)

    try:
        while True:
            frame = _read_frame()
            if frame is None:
                break
            width, height, raw_data = frame
            if len(raw_data) < frame_size:
                break

            img_array = np.frombuffer(raw_data, dtype=np.uint8).reshape((height, width, 4))
            frame_rgb = img_array[:, :, :3].copy()
            mp_image = mp.Image(image_format=mp.ImageFormat.SRGB, data=frame_rgb)
            result = landmarker.detect(mp_image)

            if result.pose_landmarks:
                persons = []
                for pose in result.pose_landmarks:
                    xs = [lm.x for lm in pose]
                    ys = [lm.y for lm in pose]
                    margin = 0.1
                    cx, cy = (min(xs)+max(xs))/2.0, (min(ys)+max(ys))/2.0
                    w = (max(xs)-min(xs))*(1+2*margin)
                    h = (max(ys)-min(ys))*(1+2*margin)
                    persons.append({
                        "bbox": [cx, cy, w, h],
                        "keypoints": [[lm.x, lm.y, lm.z] for lm in pose],
                    })
                output = {"person_count": len(persons), "persons": persons, "timestamp": time.time()}
            else:
                output = {"person_count": 0, "persons": [], "timestamp": time.time()}
            print(json.dumps(output), flush=True)
    except (KeyboardInterrupt, SystemExit):
        pass
    finally:
        landmarker.close()

if __name__ == "__main__":
    main()
