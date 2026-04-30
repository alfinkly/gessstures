"""
MediaPipe Hands Sidecar — Python subprocess for real-time hand landmark detection.

Opens camera, runs MediaPipe Hands, outputs JSON landmarks to stdout.
Designed to be spawned by the Rust app as a child process.
"""

import json
import time
import sys

import cv2
import mediapipe as mp


def main():
    cap = cv2.VideoCapture(0)
    if not cap.isOpened():
        print(json.dumps({"error": "Cannot open camera", "timestamp": time.time()}), flush=True)
        sys.exit(1)

    cap.set(cv2.CAP_PROP_FRAME_WIDTH, 640)
    cap.set(cv2.CAP_PROP_FRAME_HEIGHT, 480)

    mp_hands = mp.solutions.hands.Hands(
        static_image_mode=False,
        max_num_hands=1,
        min_detection_confidence=0.5,
        min_tracking_confidence=0.5,
    )

    try:
        while True:
            ret, frame = cap.read()
            if not ret:
                time.sleep(0.1)
                continue

            frame_rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            results = mp_hands.process(frame_rgb)

            if results.multi_hand_landmarks:
                hand = results.multi_hand_landmarks[0]
                landmarks = [[lm.x, lm.y, lm.z] for lm in hand.landmark]
                output = {
                    "detected": True,
                    "landmarks": landmarks,
                    "handedness": (
                        results.multi_handedness[0].classification[0].label
                        if results.multi_handedness
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
        mp_hands.close()


if __name__ == "__main__":
    main()
