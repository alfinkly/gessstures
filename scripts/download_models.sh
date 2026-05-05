#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODELS_DIR="$SCRIPT_DIR/../models"
mkdir -p "$MODELS_DIR"

echo "Downloading MediaPipe Pose Landmarker (full) model..."
curl -sL "https://storage.googleapis.com/mediapipe-models/pose_landmarker/pose_landmarker_full/float16/1/pose_landmarker_full.task" \
  -o "$MODELS_DIR/pose_landmarker_full.task"
echo "  pose_landmarker_full.task: $(ls -lh "$MODELS_DIR/pose_landmarker_full.task" | awk '{print $5}')"

echo "Downloading MediaPipe Hand Landmarker model..."
curl -sL "https://storage.googleapis.com/mediapipe-models/hand_landmarker/hand_landmarker/float16/1/hand_landmarker.task" \
  -o "$MODELS_DIR/hand_landmarker.task"
echo "  hand_landmarker.task: $(ls -lh "$MODELS_DIR/hand_landmarker.task" | awk '{print $5}')"

echo ""
echo "All models downloaded to $MODELS_DIR"
