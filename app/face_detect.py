"""
Face Detection Sidecar — reads JPEG from stdin, outputs JSON with face bboxes + embeddings.
Protocol: <4-byte big-endian size><JPEG bytes> per frame.
Uses InsightFace for detection + recognition (512-dim embeddings).
"""

import base64
import json
import os
import struct
import sys
import time
import cv2
import numpy as np

model_loaded = False
app = None


def load_model():
    global app, model_loaded
    try:
        import insightface
        from insightface.app import FaceAnalysis
        app = FaceAnalysis(name="buffalo_l", providers=["CPUExecutionProvider"])
        app.prepare(ctx_id=-1)
        model_loaded = True
    except ImportError:
        model_loaded = False


def _read_frame():
    """Read one frame from binary stdin: 4-byte size prefix + JPEG bytes."""
    buf = sys.stdin.buffer
    raw_len = buf.read(4)
    if not raw_len or len(raw_len) < 4:
        return None
    size = struct.unpack(">I", raw_len)[0]
    if size == 0 or size > 20 * 1024 * 1024:
        return None
    return buf.read(size)


def main():
    load_model()

    while True:
        jpeg_data = _read_frame()
        if jpeg_data is None or len(jpeg_data) == 0:
            break

        np_arr = np.frombuffer(jpeg_data, dtype=np.uint8)
        frame = cv2.imdecode(np_arr, cv2.IMREAD_COLOR)

        faces_data = []
        if frame is not None:
            frame_rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            h, w = frame.shape[:2]

            if model_loaded and app is not None:
                faces = app.get(frame_rgb)
                for face in faces:
                    bbox = face.bbox.astype(int).tolist()
                    embedding = face.normed_embedding.tolist()

                    x1, y1, x2, y2 = bbox
                    x1, y1 = max(0, x1), max(0, y1)
                    x2, y2 = min(w, x2), min(h, y2)
                    face_crop = frame[y1:y2, x1:x2]

                    _, buf = cv2.imencode(".jpg", face_crop, [cv2.IMWRITE_JPEG_QUALITY, 85])
                    face_b64 = base64.b64encode(buf).decode("ascii")

                    faces_data.append({
                        "bbox_norm": [x1 / w, y1 / h, x2 / w, y2 / h],
                        "embedding": embedding,
                        "confidence": float(face.det_score),
                        "face_jpeg_b64": face_b64,
                    })

        output = {"face_count": len(faces_data), "faces": faces_data, "timestamp": time.time()}
        sys.stdout.write(json.dumps(output) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
