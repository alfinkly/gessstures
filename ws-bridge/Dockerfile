# ── Stage 1: Build Rust binary ─────────────────────────────────
FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY graph-core graph-core
COPY data-sources data-sources
COPY engine engine
COPY physics-core physics-core
COPY ws-bridge ws-bridge

RUN cargo build --release -p ws_bridge && cp target/release/ws-bridge /ws-bridge

# ── Stage 2: Runtime ──────────────────────────────────────────
FROM python:3.12-slim

# Python deps for body + face detection
RUN pip install --no-cache-dir \
    mediapipe \
    opencv-python-headless \
    numpy \
    insightface

# System deps for MediaPipe + OpenCV
RUN apt-get update && apt-get install -y \
    libgl1-mesa-glx libglib2.0-0 libsm6 libxext6 libxrender-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Rust binary
COPY --from=builder /ws-bridge .

# Python scripts
COPY app/mediapipe_body.py app/face_detect.py ./app/

# Models dir (actual files mounted as volume at runtime: ./models:/app/models)
RUN mkdir -p models

EXPOSE 3030

CMD ["./ws-bridge", "-n", "demo-notes"]
