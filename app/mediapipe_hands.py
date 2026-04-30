"""
MediaPipe Hands Sidecar — Python subprocess for real-time hand landmark detection.

Uses MediaPipe 0.10+ Task API (HandLandmarker).
Outputs JSON landmarks to stdout for the Rust app to consume.
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
    model_path = os.path.normpath(os.path.join(script_dir, "..", "models", "hand_landmarker.task"))

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
    options = vision.HandLandmarkerOptions(
        base_options=base_options,
        running_mode=vision.RunningMode.IMAGE,
        num_hands=1,
        min_hand_detection_confidence=0.5,
        min_tracking_confidence=0.5,
    )
    landmarker = vision.HandLandmarker.create_from_options(options)

    try:
        while True:
            ret, frame = cap.read()
            if not ret:
                time.sleep(0.1)
                continue

            frame_rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            mp_image = mp.Image(image_format=mp.ImageFormat.SRGB, data=frame_rgb)

            detection_result = landmarker.detect(mp_image)

            if detection_result.hand_landmarks:
                hand = detection_result.hand_landmarks[0]
                landmarks = [[lm.x, lm.y, lm.z] for lm in hand]
                output = {
                    "detected": True,
                    "landmarks": landmarks,
                    "handedness": (
                        detection_result.handedness[0][0].category_name
                        if detection_result.handedness
                        else "Unknown"
                    ),
                    "timestamp": time.time(),
                }
            else:
                output = {"detected": False, "timestamp": time.time()}

            print(json.dumps(output), flush=True)

    except (KeyboardInterrupt, SystemExit):
        pass
    finally:
        cap.release()
        landmarker.close()


if __name__ == "__main__":
    main()
