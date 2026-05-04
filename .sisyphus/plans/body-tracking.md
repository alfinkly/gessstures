# Body Tracking — Multi-Person Crop Tiles

## TL;DR

> MediaPipe Pose Landmarker детектит людей в кадре. Rust рендерит N квадратиков (tiles) с обрезкой кадра под каждого человека.
> 0 людей = полный кадр, 1 = на весь, 2 = 2 tiles, 3-4 = сетка.

## Architecture

```
Window (1920×1080) ← Camera (640×480) ← Pose Landmarker → person bboxes

     ▼                          ▼
  Full frame (fit)          Person tiles (crop + fit)
  ┌────────────────────┐    ┌────┬────┐
  │                    │    │ P1 │ P2 │
  │    camera feed     │    ├────┼────┤
  │    (fit, no warp)  │    │ P3 │ P4 │
  │                    │    └────┴────┘
  └────────────────────┘     overlaid on top
```

### Как рендерится (fit без искажений):

1. **Фон** = вся камера 640×480, scaled to window с сохранением пропорций.
   - Если окно 1920×1080 (16:9), а камера 640×480 (4:3):
   - `scale = min(1920/640, 1080/480) = min(3.0, 2.25) = 2.25`
   - Итоговый размер кадра: 1440×1080, центрирован. По бокам letterbox (240px).
   - **Никакого stretch, только fit.**

2. **Тайлы людей** = тот же texture (640×480), но каждый показывает ТОЛЬКО свою область:
   - `Sprite::rect` = bounding box человека в texel coordinates
   - Например человек в центре: `rect = Some(Rect { min: (160, 24), max: (352, 264) })`
   - Тайл рендерится с `custom_size` = размер тайла в сетке
   - Aspect ratio сохраняется внутри каждого тайла
   - **Никакого искажения лиц.**

3. **0 людей** → только фон. Никаких тайлов.

4. **Тайлы поверх фона** — все спрайты используют ОДИН texture handle. GPU ничего не копирует.

## Data Flow

```
nokhwa (640×480 RGBA) ───→ CameraResource.frame
     │
     ├──→ Rust: PersonTracker sidecar reader
     │       reads JSON ← mediapipe_body.py
     │       stores Vec<PersonBbox> in PersonTrackerResource
     │
     └──→ Rust: body_overlay_plugin
             reads PersonTrackerResource
             creates/destroys Sprite entities per person
             each Sprite: same image handle, different rect
```

## Layout Engine

| Persons | Layout | Tile Size (fit) |
|---------|--------|-----------------|
| 0 | Full frame only | — |
| 1 | Center, 60% window height | `min(ww*0.6, wh*0.8)` preserving bbox aspect |
| 2 | Side by side, bottom half | Each 45% width, 50% height |
| 3 | Top row 2, bottom row 1 centered | Top: 45%×45%, Bottom: 45%×45% |
| 4 | 2×2 grid | Each 45%×45% |
| 5+ | Scrollable row at bottom | Fixed 200×150px thumbnails |

## Key Design Decisions

1. **Sprite::rect approach** — все тайлы + фон = один `Handle<Image>`. Разные `rect` поля. Нулевое копирование пикселей.
2. **Fit everywhere** — и фон, и каждый тайл сохраняет aspect ratio. Distortion = 0.
3. **3840 bytes per person** — только bounding box float[4]. Не храним кропнутые текстуры.

## Tasks

### Wave 1: Pose Landmarker sidecar
- [ ] 1. Download `pose_landmarker_full.task` model
- [ ] 2. Create `app/mediapipe_body.py` — Pose Landmarker, person bboxes, stdout JSON
- [ ] 3. Handle 0-N persons, output normalized bbox [cx, cy, w, h] per person

### Wave 2: Rust person tracker
- [ ] 4. Create `infrastructure/body/person_tracker.rs` — JSON parser + PersonTrackerResource
- [ ] 5. `PersonBbox` struct, `Vec<PersonBbox>`, timestamp
- [ ] 6. Spawn sidecar process + reader thread

### Wave 3: Tile rendering
- [ ] 7. Create `body_overlay.rs` — read PersonTrackerResource → compute tile layout
- [ ] 8. For each person: spawn ImageNode with crop rect
- [ ] 9. Update tiles on person count/position change
- [ ] 10. 0 persons → show full-frame placeholder
- [ ] 11. Layout engine: 0/1/2/3/4+ persons → responsive grid

### Wave 4: Integration + polish
- [ ] 12. Wire into desktop mode (add to plugin list)
- [ ] 13. Performance: run pose detection every 3rd frame (~10 FPS)
- [ ] 14. Smoothing: interpolate bounding boxes for stable tiles

## Files to Create
- `app/mediapipe_body.py`
- `app/src/infrastructure/body/mod.rs`
- `app/src/infrastructure/body/person_tracker.rs`
- `app/src/interface/plugins/body_overlay_plugin.rs`

## Model
- `pose_landmarker_full.task` (or lite) — download from Google MediaPipe model zoo
- Size: ~10-15MB
- Input: 256×256 RGB
- Output: per-person bounding box + 33 keypoints
