import json
import time
import sys
import os

import cv2
import numpy as np
import mediapipe as mp
from mediapipe.tasks import python
from mediapipe.tasks.python import vision


def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    model_path = os.path.normpath(os.path.join(script_dir, "..", "models", "pose_landmarker_full.task"))

    if not os.path.exists(model_path):
        print(json.dumps({"error": f"Model not found: {model_path}"}), flush=True)
        sys.exit(1)

    base_options = python.BaseOptions(model_asset_path=model_path)
    options = vision.PoseLandmarkerOptions(
        base_options=base_options,
        running_mode=vision.RunningMode.IMAGE,
        num_poses=4,
        min_pose_detection_confidence=0.5,
        min_tracking_confidence=0.5,
    )
    landmarker = vision.PoseLandmarker.create_from_options(options)

    try:
        while True:
            dims_line = sys.stdin.readline()
            if not dims_line:
                break
            dims_line = dims_line.strip()
            parts = dims_line.split()
            if len(parts) < 2:
                continue
            width, height = int(parts[0]), int(parts[1])

            frame_size = width * height * 4
            raw_data = sys.stdin.buffer.read(frame_size)
            if len(raw_data) < frame_size:
                break

            img_array = np.frombuffer(raw_data, dtype=np.uint8).reshape((height, width, 4))
            frame_rgb = img_array[:, :, :3].copy()

            mp_image = mp.Image(image_format=mp.ImageFormat.SRGB, data=frame_rgb)

            detection_result = landmarker.detect(mp_image)

            if detection_result.pose_landmarks:
                persons = []
                for pose in detection_result.pose_landmarks:
                    xs = [lm.x for lm in pose]
                    ys = [lm.y for lm in pose]
                    min_x, max_x = min(xs), max(xs)
                    min_y, max_y = min(ys), max(ys)
                    margin = 0.1
                    cx = (min_x + max_x) / 2.0
                    cy = (min_y + max_y) / 2.0
                    w = (max_x - min_x) * (1.0 + 2.0 * margin)
                    h = (max_y - min_y) * (1.0 + 2.0 * margin)
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
    except Exception as e:
        print(json.dumps({"error": str(e), "person_count": 0, "persons": []}), flush=True)
    finally:
        landmarker.close()


if __name__ == "__main__":
    main()
