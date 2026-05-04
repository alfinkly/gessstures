"""
MediaPipe Pose Landmarker Sidecar — detects people in camera frame.
Outputs JSON bounding boxes to stdout for the Rust app to consume.
"""

import json
import time
import sys
import os

import cv2
import mediapipe as mp
from mediapipe.tasks import python
from mediapipe.tasks.python import vision


def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    model_path = os.path.normpath(os.path.join(script_dir, "..", "models", "pose_landmarker_full.task"))

    if not os.path.exists(model_path):
        print(json.dumps({"error": f"Model not found: {model_path}", "timestamp": time.time()}), flush=True)
        sys.exit(1)

    cap = cv2.VideoCapture(0)
    if not cap.isOpened():
        print(json.dumps({"error": "Cannot open camera", "timestamp": time.time()}), flush=True)
        sys.exit(1)

    cap.set(cv2.CAP_PROP_FRAME_WIDTH, 640)
    cap.set(cv2.CAP_PROP_FRAME_HEIGHT, 480)

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
            ret, frame = cap.read()
            if not ret:
                time.sleep(0.1)
                continue

            frame_rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            mp_image = mp.Image(image_format=mp.ImageFormat.SRGB, data=frame_rgb)

            detection_result = landmarker.detect(mp_image)

            if detection_result.pose_landmarks:
                persons = []
                for pose in detection_result.pose_landmarks:
                    xs = [lm.x for lm in pose]
                    ys = [lm.y for lm in pose]
                    min_x, max_x = min(xs), max(xs)
                    min_y, max_y = min(ys), max(ys)
                    cx = (min_x + max_x) / 2.0
                    cy = (min_y + max_y) / 2.0
                    w = max_x - min_x
                    h = max_y - min_y
                    margin = 0.1
                    w_margin = w * (1.0 + 2.0 * margin)
                    h_margin = h * (1.0 + 2.0 * margin)
                    persons.append({
                        "bbox": [cx, cy, w_margin, h_margin],
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
        cap.release()
        landmarker.close()


if __name__ == "__main__":
    main()
