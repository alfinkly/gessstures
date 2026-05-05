"""
MediaPipe Pose Landmarker Sidecar — reads JPEG frames from stdin.
Protocol: <4-byte big-endian size><JPEG bytes> per frame.
Outputs: one JSON line per frame to stdout.
"""

import json
import os
import struct
import sys
import time

import cv2
import mediapipe as mp
import numpy as np
from mediapipe.tasks import python
from mediapipe.tasks.python import vision


def _read_frame():
    """Read one frame from binary stdin: 4-byte size prefix + JPEG bytes."""
    buf = sys.stdin.buffer
    raw_len = buf.read(4)
    if not raw_len or len(raw_len) < 4:
        return None
    size = struct.unpack(">I", raw_len)[0]
    if size == 0 or size > 10 * 1024 * 1024:
        return None
    return buf.read(size)


def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    model_path = os.path.normpath(
        os.path.join(script_dir, "..", "models", "pose_landmarker_full.task")
    )
    if not os.path.exists(model_path):
        err = {"person_count": 0, "persons": [], "error": f"Model not found: {model_path}"}
        print(json.dumps(err), flush=True)
        sys.exit(1)

    base_options = python.BaseOptions(model_asset_path=model_path)
    options = vision.PoseLandmarkerOptions(
        base_options=base_options,
        running_mode=vision.RunningMode.IMAGE,
        num_poses=4,
        min_pose_detection_confidence=0.5,
    )
    landmarker = vision.PoseLandmarker.create_from_options(options)

    try:
        while True:
            jpeg_data = _read_frame()
            if jpeg_data is None or len(jpeg_data) == 0:
                break

            np_arr = np.frombuffer(jpeg_data, dtype=np.uint8)
            frame = cv2.imdecode(np_arr, cv2.IMREAD_COLOR)
            if frame is None:
                continue

            frame_rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            mp_image = mp.Image(image_format=mp.ImageFormat.SRGB, data=frame_rgb)
            result = landmarker.detect(mp_image)

            if result.pose_landmarks:
                persons = []
                for pose in result.pose_landmarks:
                    xs = [lm.x for lm in pose]
                    ys = [lm.y for lm in pose]
                    margin = 0.1
                    cx, cy = (min(xs) + max(xs)) / 2.0, (min(ys) + max(ys)) / 2.0
                    w = (max(xs) - min(xs)) * (1 + 2 * margin)
                    h = (max(ys) - min(ys)) * (1 + 2 * margin)
                    persons.append({
                        "bbox": [cx, cy, w, h],
                        "keypoints": [[lm.x, lm.y, lm.z] for lm in pose],
                    })
                output = {
                    "person_count": len(persons),
                    "persons": persons,
                    "timestamp": time.time(),
                }
            else:
                output = {"person_count": 0, "persons": [], "timestamp": time.time()}

            print(json.dumps(output), flush=True)
    except (KeyboardInterrupt, SystemExit):
        pass
    finally:
        landmarker.close()


if __name__ == "__main__":
    main()
