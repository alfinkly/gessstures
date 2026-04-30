#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODELS_DIR="$SCRIPT_DIR/../models"
mkdir -p "$MODELS_DIR"

echo "Downloading MediaPipe Hand Landmarker model..."
curl -sL "https://storage.googleapis.com/mediapipe-models/hand_landmarker/hand_landmarker/float16/1/hand_landmarker.task" \
  -o "$MODELS_DIR/hand_landmarker.task"

echo "Done: $(ls -lh "$MODELS_DIR/hand_landmarker.task" | awk '{print $5}')"
