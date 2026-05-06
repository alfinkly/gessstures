# ── Stage 1: Build Rust binary ─────────────────────────────────
FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 1. Copy only manifests — cache dependency download
COPY Cargo.toml Cargo.lock ./
COPY graph-core/Cargo.toml graph-core/
COPY ai-pipeline/Cargo.toml ai-pipeline/
COPY data-sources/Cargo.toml data-sources/
COPY input-sources/Cargo.toml input-sources/
COPY engine/Cargo.toml engine/
COPY physics-core/Cargo.toml physics-core/
COPY app/Cargo.toml app/
COPY hand-tracking-core/Cargo.toml hand-tracking-core/
COPY ws-bridge/Cargo.toml ws-bridge/

# 2. Create dummy main/lib files so cargo can resolve the workspace
RUN mkdir -p graph-core/src ai-pipeline/src data-sources/src input-sources/src \
         engine/src physics-core/src app/src hand-tracking-core/src ws-bridge/src \
 && touch graph-core/src/lib.rs ai-pipeline/src/lib.rs data-sources/src/lib.rs \
        input-sources/src/lib.rs engine/src/lib.rs physics-core/src/lib.rs \
        hand-tracking-core/src/lib.rs \
 && echo 'fn main() {}' > app/src/main.rs \
 && echo 'fn main() {}' > ws-bridge/src/main.rs

# 3. Build — downloads & caches ALL dependencies
RUN cargo build --release -p ws_bridge 2>/dev/null || true

# 4. Remove dummy files so COPY doesn't conflict
RUN rm -rf graph-core/src ai-pipeline/src data-sources/src input-sources/src \
         engine/src physics-core/src app/src hand-tracking-core/src ws-bridge/src

# 5. Copy actual source
COPY graph-core/src graph-core/src
COPY ai-pipeline/src ai-pipeline/src
COPY data-sources/src data-sources/src
COPY input-sources/src input-sources/src
COPY engine/src engine/src
COPY physics-core/src physics-core/src
COPY app/src app/src
COPY hand-tracking-core/src hand-tracking-core/src
COPY ws-bridge/src ws-bridge/src

# 6. Final build (only recompiles crates that changed)
RUN cargo build --release -p ws_bridge && cp target/release/ws-bridge /ws-bridge

# ── Stage 2: Runtime ──────────────────────────────────────────
FROM python:3.12-slim

# System deps (install BEFORE Python deps for layer caching)
RUN apt-get update && apt-get install -y \
    g++ build-essential \
    libgl1-mesa-glx libglib2.0-0 libsm6 libxext6 libxrender-dev \
    && rm -rf /var/lib/apt/lists/*

# Python deps for body + face detection
RUN pip install --no-cache-dir \
    numpy \
    opencv-python-headless \
    mediapipe \
    insightface

WORKDIR /app

# Rust binary
COPY --from=builder /ws-bridge .

# Python scripts
COPY app/mediapipe_body.py app/face_detect.py ./app/

# Models dir (mounted as volume: ./models:/app/models)
RUN mkdir -p models

EXPOSE 3030

CMD ["./ws-bridge", "-n", "demo-notes"]
