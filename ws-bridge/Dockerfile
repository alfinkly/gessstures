# syntax=docker/dockerfile:1
FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release -p ws_bridge && \
    cp target/release/ws-bridge /ws-bridge

# ── Runtime ────────────────────────────────────────────────────
FROM python:3.12-slim

RUN apt-get update && apt-get install -y \
    g++ build-essential \
    libgl1 libglib2.0-0 libsm6 libxext6 libxrender-dev \
    && rm -rf /var/lib/apt/lists/*

RUN --mount=type=cache,target=/root/.cache/pip \
    pip install numpy opencv-python-headless mediapipe insightface

# Pre-cache insightface models
RUN --mount=type=cache,target=/root/.cache/pip \
    --mount=type=cache,target=/root/.insightface \
    python3 -c "from insightface.app import FaceAnalysis; app = FaceAnalysis(name='buffalo_l', providers=['CPUExecutionProvider']); app.prepare(ctx_id=-1); print('insightface models cached')" 2>/dev/null || true

WORKDIR /app

COPY --from=builder /ws-bridge .
COPY app/mediapipe_body.py app/face_detect.py ./app/
RUN mkdir -p models

EXPOSE 3030
CMD ["./ws-bridge", "-n", "demo-notes"]
