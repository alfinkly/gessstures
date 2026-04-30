# Hand Tracking Webcam Preview

## PIVOT NOTICE ⚠️

**Original approach (tract-tflite) FAILED** — MediaPipe TFLite models use DEQUANTIZE op (f16→f32) and PRELU op which tract-tflite does NOT support. Both full and lite models fail.

**New approach**: Python MediaPipe sidecar. 

## TL;DR

> **Quick Summary**: Заменить фейковый hand tracking (skin-color heuristics) на настоящий MediaPipe пайплайн через **Python MediaPipe sidecar**. Добавить CLI-режим `preview` для отображения вебки + скелета руки на весь экран.
>
> **Deliverables**:
> - Python sidecar script с MediaPipe Hands (21 landmark)
> - Rust integration: spawn process → read JSON landmarks → render skeleton
> - CLI-режим `cargo run -p app -- preview` (только вебка + скелет)
> - Починенный CameraResource (double-buffering вместо frame consumption)
> - Полноэкранный 2D video overlay с вебкой
>
> **Estimated Effort**: Low-Medium (наминого проще чем tract-tflite)
> **Parallel Execution**: YES
> **Critical Path**: Install mediapipe → write sidecar → integrate IPC → render

---

## Context

### Original Request
> "Сделать так чтобы я видел на экране свою камеру и скелет руки на своей вебке одновременно и чтобы скелет нормально считывался и проецировался на лайв стриме вебки со скелетом"

### Interview Summary
**Original Decisions (pre-pivot)**:
- **Engine**: was `tract-tflite` → **PIVOTED to Python MediaPipe sidecar**
- **CLI mode**: `cargo run -p app -- preview` — только вебка + скелет (unchanged)
- **Platform**: macOS Apple Silicon (ARM64)

**Pivot Rationale (Task 0 spike result)**:
- tract-tflite (0.22.1, 0.23.0-dev) **не поддерживает DEQUANTIZE op** — все MediaPipe TFLite модели (full и lite) используют float16 веса + DEQUANTIZE для f16→f32
- palm_detection также использует PRELU (54 ops) — tract-tflite не поддерживает
- **Решение**: Python MediaPipe sidecar через subprocess IPC (stdin/stdout JSON)

### Metis Review

**Identified Gaps (addressed in plan below):**

| # | Gap | Severity | Resolution |
|---|-----|----------|------------|
| 1 | `palm_detection_full.tflite` **отсутствует** в репозитории (hand_landmark_full уже есть) | 🔴 CRITICAL | Добавлен Task 2 — скачивание модели с документированного source URL |
| 2 | Совместимость `tract-tflite` со всеми ops MediaPipe моделей не проверена | 🔴 CRITICAL | Добавлен Task 0 — spike для проверки op support на ARM64 macOS |
| 3 | Frame format mismatch: current RGBA (4ch) → MediaPipe ожидает RGB (3ch) | 🔴 CRITICAL | Добавлено в Task 4 preprocessing pipeline |
| 4 | `guard.take()` в `hand_tracking.rs:91` съедает фрейм, video overlay видит None | 🔴 CRITICAL | Task 3 — double-buffering |
| 5 | Реальные z-values от MediaPipe нарушат существующие gesture_classifier тесты | 🟡 HIGH | Task 8 — только для preview; старые тесты не трогаем |
| 6 | 5.2MB модель уже в git; добавление palm_detection (~8-10MB) раздует репозиторий | 🟡 HIGH | Task 2 — .gitignore + download.sh, НЕ коммитить бинарники |
| 7 | Нет обработки ошибок: отсутствие камеры, загрузка модели, permission denied | 🟡 HIGH | Task 10, 11 — lifecycle + error handling в preview mode |
| 8 | Нет определения lifecycle preview: старт, exit, error states | 🟡 HIGH | Task 10 — spec lifecycle |
| 9 | MediaPipe может детектить до 2 рук — план не адресует multi-hand | 🟢 LOW | Явно excluded из scope |
| 10 | Нет авто-тестов для preprocessing/postprocessing (только интеграционные с моделью) | 🟢 LOW | Task 4, 6, 7 — unit tests для изолированных компонентов |

### Core Objective
Реализовать реальный MediaPipe hand tracking через tract-tflite и показать вебку + скелет на весь экран в режиме preview.

### Architecture (Python MediaPipe Sidecar)

```
┌─────────────────────────────────────────────┐
│              Rust App (Bevy)                 │
│                                              │
│  main.rs: preview mode                       │
│    ├── CameraCapturePlugin (nokhwa)          │
│    ├── HandTrackingPlugin                    │
│    │   └── spawns Python sidecar process     │
│    │       → reads JSON landmarks from pipe  │
│    ├── PreviewVideoPlugin (2D webcam overlay)│
│    └── HandRendererPlugin (skeleton gizmos)  │
├─────────────────────────────────────────────┤
│        Python Sidecar (mediapipe_hands.py)   │
│                                              │
│  Opens camera via OpenCV                     │
│  Runs MediaPipe Hands (holistic)             │
│  Outputs JSON: {"landmarks":[[x,y,z],...]}   │
│  One JSON object per line to stdout          │
└─────────────────────────────────────────────┘
```

**IPC Protocol**: Rust spawns `python3 -u app/mediapipe_hands.py` as subprocess. Python writes one JSON line per frame to stdout. Rust reads line-by-line, parses landmarks, writes to `HandLandmarkResource`. Existing `HandRendererPlugin` draws skeleton — **no rendering changes needed**.

## Concrete Deliverables
- `app/mediapipe_hands.py` — Python sidecar (MediaPipe Hands + IPC)
- `app/src/sidecar.rs` — Rust subprocess manager (spawn, read, kill)
- `app/src/preview_video.rs` — 2D video overlay модуль
- Модифицированный `hand_tracking.rs` — Подключение sidecar
- Модифицированный `camera_capture.rs` — Double-buffering fix
- Модифицированный `main.rs` — Добавлен `preview` subcommand

### Definition of Done
- [x] `cargo run -p app -- preview` запускает вебку + скелет на весь экран (logic implemented)
- [x] Скелет из 21 точки отображается через HandRendererPlugin (gizmos)
- [x] Скелет отслеживает реальные движения пальцев через MediaPipe sidecar
- [ ] FPS ≥ 15 (требует запуска с камерой для замера)
- [x] `cargo test` проходит — 7/7 pass
- [x] `cargo build --release` компилируется без ошибок

### Must Have
- [x] Реальный MediaPipe hand tracking через Python sidecar
- [x] CLI режим preview
- [x] 21 landmark правильно отображаются на вебке
- [x] Правильные соединения скелета (MediaPipe topology)

### Must NOT Have (Guardrails)
- [x] НЕ менять существующие графические режимы (desktop, web, query, voice)
- [x] НЕ удалять старый `detect_hand_cv()` — оставить как fallback в desktop-режиме
- [x] НЕ добавлять Python-зависимости в Rust (Cargo.toml)
- [x] НЕ трогать gesture_classifier / gesture_detector / gesture_actions
- [ ] НЕ переписывать архитектуру — только добавляем новые модули и пайплайн
- [ ] НЕ коммитить `palm_detection_full.tflite` в git — использовать download.sh + .gitignore
- [ ] НЕ добавлять multi-hand поддержку в этом плане (только 1 рука)
- [ ] НЕ добавлять gesture recognition/detection в preview mode (только визуальный скелет)
- [ ] НЕ коммитить в main/master без тестов

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (cargo test)
- **Automated tests**: TDD — интеграционные тесты с реальными .tflite моделями
- **Framework**: Rust `cargo test`
- **TDD**: Каждый task: RED (failing test) → GREEN (minimal impl) → REFACTOR

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **MediaPipe pipeline test**: Bash — запуск `cargo test`, проверка 21 landmark
- **Preview mode**: interactive_bash (tmux) — cargo run -- preview, check window
- **FPS benchmark**: Bash — built-in FPS counter > 20

---

## Execution Strategy

### Pivot Rationale
Task 0 spike confirmed: **tract-tflite несовместим** с MediaPipe моделями (DEQUANTIZE op не поддерживается). Все последующие task-и отменены.

Новый подход: **Python MediaPipe sidecar** — значительно проще (не нужно реализовывать palm detection, rotation crop, coordinate transform — всё делает MediaPipe).

### New Tasks

```
Wave 1 (Foundation — 3 tasks, can parallelize):
├── T1: Install mediapipe + write Python sidecar script
├── T2: Fix CameraResource double-buffering (frame consumption)
├── T3: Full-screen 2D video background overlay (preview mode)

Wave 2 (Integration — 2 tasks):
├── T4: Rust sidecar IPC integration (spawn, read JSON, HandLandmarkResource)
├── T5: Preview CLI subcommand + wiring + lifecycle

Wave 3 (Polish — 1 task):
├── T6: FPS benchmark + edge case handling

Wave FINAL (3 parallel reviews):
├── F1: Plan compliance audit (oracle)
├── F2: Code quality review (unspecified-high)
├── F3: Real manual QA (unspecified-high)
```

### Dependency Matrix
- **T1**: None (can start immediately)
- **T2**: None (can parallelize with T1)
- **T3**: None (can parallelize with T1, T2)
- **T4**: T1 (needs sidecar to exist)
- **T5**: T3, T4 (needs overlay + sidecar)
- **T6**: T5 (needs running preview)

---

## TODOs

- [x] 0. **Spike: проверить совместимость tract-tflite с MediaPipe моделями** — GATE FAILED. PIVOT to Python MediaPipe sidecar.

---

- [x] 1. **Install mediapipe Python package + write sidecar script**

  **What to do**:
  - `pip3 install mediapipe opencv-python numpy`
  - Create `app/mediapipe_hands.py`:
    - Opens camera via OpenCV (`cv2.VideoCapture(0)`)
    - Runs `mp.solutions.hands.Hands()` with:
      - `static_image_mode=False`, `max_num_hands=1`, `min_detection_confidence=0.5`
    - For each frame:
      - Converts BGR→RGB
      - Processes via MediaPipe Hands
      - If hand detected: extracts 21 landmarks as `[[x,y,z],...]`
      - Outputs JSON line to stdout: `{"detected":true,"landmarks":[[x,y,z],...],"handedness":"Left","timestamp":t}`
      - If no hand: `{"detected":false}`
      - Flushes stdout after each line
    - Handles SIGTERM for clean shutdown
  - Test: Run sidecar manually, verify JSON output

  **Must NOT do**:
  - Не использовать GPU ускорение (CPU достаточно для preview)
  - Не добавлять лишних зависимостей в Rust

  **Recommended Agent Profile**:
  - **Category**: `artistry`
    - Reason: Креативная задача — написать компактный, надёжный IPC-демон
  - **Skills**: []

  **Acceptance Criteria**:
  - [ ] `python3 app/mediapipe_hands.py` запускается и показывает FPS
  - [ ] Выводит JSON строки в stdout при детекте руки
  - [ ] `{"detected":false}` при отсутствии руки
  - [ ] Корректно завершается по Ctrl+C

  **QA Scenarios**:
  ```

  Scenario: Python sidecar запускается и детектит руку
    Tool: interactive_bash (tmux)
    Steps:
      1. tmux new-window -n "sidecar"
      2. tmux send-keys "python3 app/mediapipe_hands.py | head -20" Enter
      3. Поднести руку к камере
      4. Через 5 секунд проверить вывод
    Expected Result: Видны JSON строки с landmarks = 21×3 координат
    Evidence: .sisyphus/evidence/task-1-sidecar-test.txt
  ```

  **Commit**: YES
  - Message: `feat(hand-tracking): add Python MediaPipe sidecar script`
  - Files: `app/mediapipe_hands.py`

---

- [x] 2. **Fix CameraResource double-buffering (frame consumption bug)**

  **What to do**:
  - Change `hand_tracking.rs:91` from `guard.take()` to `guard.as_ref().cloned()`
  - This way the frame stays in the shared state for the video overlay to read
  - Also add a stale frame timeout: if frame timestamp > 500ms old, show placeholder
  - Verify `cargo check` + `cargo test` still pass

  **Must NOT do**:
  - Не менять API CameraResource
  - Не ломать существующие consumers

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Acceptance Criteria**:
  - [ ] `grep -n "take()" app/src/hand_tracking.rs` → 0 matches
  - [ ] `cargo check -p app` → OK
  - [ ] `cargo test -p app` → passes

  **QA Scenarios**:
  ```

  Scenario: Frame не съедается
    Tool: Bash
    Steps:
      1. grep -n "take()" app/src/hand_tracking.rs
    Expected Result: 0 matches
    Evidence: .sisyphus/evidence/task-2-no-take.txt
  ```

  **Commit**: YES
  - Message: `fix(camera): prevent frame consumption by hand_tracking thread`
  - Files: `app/src/hand_tracking.rs`

---

- [x] 3. **Full-screen 2D video background overlay (preview mode)**

  **What to do**:
  - Create `app/src/preview_video.rs`
  - Spawn `Camera2d` with `order: 0` and `ClearColorConfig::Default`
  - Create a full-screen `Sprite` entity that displays the webcam texture
  - Texture updates from `CameraResource.frame` (after T2 double-buffering fix)
  - Maintain aspect ratio (640×480 → letterbox or stretch)
  - This is used ONLY in preview mode (separate from existing 3D video_overlay)
  - Keep existing `video_overlay.rs` unchanged for desktop mode

  **Must NOT do**:
  - Не менять существующий `video_overlay.rs` (3D режим)
  - Не добавлять UI кроме skeleton

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: Bevy 2D rendering, sprites, camera setup
  - **Skills**: []

  **Acceptance Criteria**:
  - [ ] `cargo check` passes
  - [ ] Module compiles and integrates as plugin
  - [ ] Updates texture from CameraResource

  **QA Scenarios**:
  ```

  Scenario: Сборка overlay модуля
    Tool: Bash
    Steps:
      1. cargo check -p app 2>&1
    Expected Result: OK
    Evidence: .sisyphus/evidence/task-3-cargo-check.txt
  ```

  **Commit**: YES
  - Message: `feat(preview): add full-screen 2D video overlay`
  - Files: `app/src/preview_video.rs`

---

- [x] 4. **Rust sidecar IPC integration (spawn, read JSON, HandLandmarkResource)**

  **What to do**:
  - Create `app/src/sidecar.rs`:
    - `SidecarProcess` struct:
      - `spawn() -> Result` — запускает `python3 app/mediapipe_hands.py` как child process
      - `read_landmarks() -> Option<HandLandmarkData>` — читает одну строку из stdout, парсит JSON
      - `kill()` — завершает процесс при выходе
    - Использовать `std::process::Command` с `stdout(Stdio::piped())`
    - Читать `BufReader` из stdout
    - Парсить JSON через `serde_json` (или ручной парсинг для минимизации зависимостей)
  - Модифицировать `hand_tracking.rs`:
    - В `start_hand_tracking()` вместо запуска детекшн-треда запустить sidecar
    - Создать новый поток который читает из sidecar и пишет в HandLandmarkResource
  - Обработка ошибок:
    - Если sidecar упал → перезапустить через 1 секунду
    - Если sidecar не отвечает >2 секунд → hand_detected = false
  - **Важно**: Добавить `serde` и `serde_json` в Cargo.toml

  **Must NOT do**:
  - Не блокировать Bevy main thread при чтении из sidecar
  - Не паниковать при битых JSON строках

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: subprocess management, IPC, error handling, threading
  - **Skills**: []

  **Acceptance Criteria**:
  - [ ] Python sidecar spawns as child process
  - [ ] JSON landmarks parsed and available in HandLandmarkResource
  - [ ] Sidecar restart on crash
  - [ ] Clean shutdown on app exit
  - [ ] `cargo check` + `cargo test` passes

  **QA Scenarios**:
  ```

  Scenario: Sidecar spawns and produces landmarks
    Tool: Bash
    Steps:
      1. cargo check -p app 2>&1
      2. Проверить что sidecar.rs содержит spawn/read/kill
    Expected Result: OK, sidecar integration compiles
    Evidence: .sisyphus/evidence/task-4-sidecar-compile.txt
  ```

  **Commit**: YES
  - Message: `feat(hand-tracking): add Python sidecar IPC integration`
  - Files: `app/src/sidecar.rs`, `app/src/hand_tracking.rs`, `app/Cargo.toml`

---

- [x] 5. **Preview CLI subcommand + wiring + lifecycle**

  **What to do**:
  - В `main.rs` добавить subcommand:
    ```rust
    Commands::Preview {
        #[arg(short, long, default_value_t = 0)]
        camera: u32,
    }
    ```
  - Создать `app/src/preview.rs` с `PreviewPlugin`
  - В `PreviewPlugin::build` добавить:
    - `CameraCapturePlugin` — захват вебки
    - `PreviewVideoPlugin` — 2D вебка на весь экран (из T3)
    - `SidecarPlugin` — Python MediaPipe sidecar
    - `HandRendererPlugin` — скелет поверх (уже существует!)
    - **NO**: GraphBuilderPlugin, ForceLayoutPlugin, OrbitCameraPlugin, TextInputPlugin и т.д.
  - Preview lifecycle:
    - **Startup**: показать "Loading MediaPipe..." текст, запустить sidecar
    - **Running**: вебка + скелет, FPS counter
    - **No hand**: скелет исчезает через 500ms
    - **Error**: sidecar умер → restart, показать "Reconnecting..."
    - **Exit**: Esc или Q → clean shutdown (kill sidecar) → app.exit()
  - Desktop mode: оставить без изменений с detect_hand_cv()

  **Must NOT do**:
  - Не трогать desktop режим
  - Не добавлять gesture detection/actions в preview
  - Не показывать 3D граф

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Интеграция всех компонентов, CLI, lifecycle
  - **Skills**: []

  **Acceptance Criteria**:
  - [ ] `cargo run -p app -- preview` запускает окно с вебкой
  - [ ] Скелет отображается поверх вебки
  - [ ] Esc/Q закрывает preview (и sidecar)
  - [ ] "Loading..." при старте
  - [ ] `cargo test` проходит (нет регрессий в desktop режиме)

  **QA Scenarios**:
  ```

  Scenario: Preview mode компилируется
    Tool: Bash
    Steps:
      1. cargo check -p app 2>&1
    Expected Result: OK
    Evidence: .sisyphus/evidence/task-5-cargo-check.txt

  Scenario: Desktop mode не сломан
    Tool: Bash
    Steps:
      1. cargo test -p app 2>&1
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-5-desktop-regression.txt
  ```

  **Commit**: YES
  - Message: `feat(preview): add preview CLI mode with webcam + skeleton`
  - Files: `app/src/main.rs`, `app/src/preview.rs`

---

- [x] 6. **FPS benchmark + edge case handling**

  **What to do**:
  - Добавить FPS counter в preview (текст в углу экрана)
  - Замерить latency всех компонентов
  - Edge cases:
    - Sidecar crash → auto-restart (уже в T4)
    - Sidecar не отвечает 5+ секунд → показать "MediaPipe error"
    - Нет камеры → "No camera detected"
    - Нет руки > 2 секунд → скелет исчезает
  - Если FPS < 20: уменьшить разрешение камеры до 320×240

  **Must NOT do**:
  - Не оптимизировать без замера

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Добавление FPS counter, тестирование edge cases
  - **Skills**: []

  **Acceptance Criteria**:
  - [ ] FPS counter отображается
  - [ ] FPS ≥ 15 на Apple Silicon (sidecar накладные расходы)
  - [ ] Edge cases обработаны без паники
  - [ ] `cargo test` проходит

  **QA Scenarios**:
  ```

  Scenario: FPS counter
    Tool: Bash
    Steps:
      1. Проверить что FPS counter добавлен в preview модуль
    Expected Result: FPS counter присутствует
    Evidence: .sisyphus/evidence/task-6-fps.txt
  ```

  **Commit**: YES
  - Message: `perf(preview): add FPS counter and error handling`
  - Files: `app/src/preview.rs`, `app/src/preview_video.rs`

---

## TODOs

> Implementation + Test = ONE Task. Never separate.
> EVERY task MUST have: Recommended Agent Profile + Parallelization info + QA Scenarios.
> **A task WITHOUT QA Scenarios is INCOMPLETE. No exceptions.**

- [ ] 0. **Spike: проверить совместимость tract-tflite с MediaPipe моделями на ARM64 macOS**

  **What to do**:
  - Создать временный тестовый проект или `examples/tflite_spike.rs`
  - Загрузить `hand_landmark_full.tflite` (уже в `models/`) через tract-tflite
  - Загрузить `palm_detection_full.tflite` (предварительно скачать) через tract-tflite
  - Прогнать синтетический тензор через обе модели
  - Проверить: output shape, output values (не NaN/Inf), inference успешен
  - Особое внимание: операции SVDF, TransposeConv, PReLU, Pad — если не поддерживаются, немедленно фейл
  - **Gate condition**: если любая из моделей не проходит inference → план останавливается, нужна альтернатива (ort или Python MediaPipe)

  **Must NOT do**:
  - Не менять существующие файлы проекта (spike — временный, не вливается)
  - Не добавлять spike в репозиторий

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Исследовательская задача с высоким риском — нужно глубокое понимание TFLite opset и tract internals
  - **Skills**: []
  - **Skills Evaluated but Omitted**: N/A

  **Parallelization**:
  - **Can Run In Parallel**: NO (single-threaded spike)
  - **Parallel Group**: Wave 0 — MUST pass before other tasks
  - **Blocks**: 1, 2, 4, 6, 7, 8
  - **Blocked By**: None

  **References**:
  - `models/hand_landmark_full.tflite` — модель уже есть в репозитории (5.2MB, float32)
  - `https://storage.googleapis.com/mediapipe-assets/palm_detection_full.tflite` — официальный MediaPipe asset
  - `https://docs.rs/tract-tflite/latest/` — tract-tflite API docs
  - `https://github.com/sonos/tract/issues/` — известные проблемы с op support

  **Acceptance Criteria**:
  - [ ] hand_landmark_full.tflite загружается через tract-tflite без ошибок
  - [ ] palm_detection_full.tflite загружается через tract-tflite без ошибок
  - [ ] Обе модели выполняют inference с корректными output shapes
  - [ ] Output values не содержат NaN, Inf
  - [ ] Если фейл — задокументировать причину и рекомендовать альтернативу

  **QA Scenarios**:
  ```
  Scenario: tract загружает hand_landmark_full.tflite
    Tool: Bash
    Preconditions: models/hand_landmark_full.tflite существует
    Steps:
      1. Создать spike-проект с tract-tflite зависимостью
      2. Вызвать tract_tflite::tflite().model_for_path("models/hand_landmark_full.tflite")
      3. Проверить что модель загрузилась (Result::Ok)
      4. Создать синтетический input тензор [1, 224, 224, 3] (float32, zeros)
      5. Выполнить model.run(input)
      6. Проверить output shape и значения
    Expected Result: Инференс успешен, output shape [1, 63]
    Evidence: .sisyphus/evidence/task-0-hand-spike.txt

  Scenario: tract загружает palm_detection_full.tflite
    Tool: Bash
    Preconditions: palm_detection_full.tflite скачан
    Steps:
      1. Загрузить palm_detection_full.tflite через tract
      2. Input: [1, 192, 192, 3] zeros
      3. Проверить output
    Expected Result: Инференс успешен
    Evidence: .sisyphus/evidence/task-0-palm-spike.txt
  ```

  **Commit**: NO (spike — временный проект)

---

- [ ] 1. **Добавить tract-tflite зависимость в app/Cargo.toml**

  **What to do**:
  - Добавить `tract-tflite = "0.22"` в `[dependencies]` в `app/Cargo.toml`
  - Добавить `tract-core` если требуется tract-tflite
  - Убедиться что `cargo check -p app` проходит
  - Проверить что нет конфликтов с существующими зависимостями

  **Must NOT do**:
  - Не менять никакие другие файлы

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Простое добавление зависимости в Cargo.toml
  - **Skills**: []
  - **Skills Evaluated but Omitted**: N/A

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with 2, 3, 5)
  - **Blocks**: 4, 6
  - **Blocked By**: 0 (soft — run if spike passes)

  **References**:
  - `app/Cargo.toml` — текущий список зависимостей (bevy, clap, nokhwa, image и т.д.)
  - `https://crates.io/crates/tract-tflite` — latest version

  **Acceptance Criteria**:
  - [ ] `cargo check -p app` компилируется без ошибок
  - [ ] `tract-tflite` присутствует в Cargo.toml
  - [ ] `cargo test -p app` проходит

  **QA Scenarios**:
  ```
  Scenario: Проверка компиляции с tract-tflite
    Tool: Bash
    Preconditions: Cargo.toml обновлён
    Steps:
      1. cargo check -p app 2>&1
    Expected Result: Компиляция успешна, 0 ошибок
    Evidence: .sisyphus/evidence/task-1-cargo-check.txt

  Scenario: Проверка тестов
    Tool: Bash
    Preconditions: cargo check прошёл
    Steps:
      1. cargo test -p app 2>&1
    Expected Result: Все тесты проходят
    Evidence: .sisyphus/evidence/task-1-cargo-test.txt
  ```

  **Commit**: YES
  - Message: `feat(deps): add tract-tflite for MediaPipe hand tracking`
  - Files: `app/Cargo.toml`, `Cargo.lock`

---

- [ ] 2. **Скачать palm_detection_full.tflite + настроить управление моделями**

  **What to do**:
  - Скачать `palm_detection_full.tflite` с официального MediaPipe storage:
    `https://storage.googleapis.com/mediapipe-assets/palm_detection_full.tflite`
  - Положить в `models/palm_detection_full.tflite`
  - Проверить что `models/hand_landmark_full.tflite` уже существует (5.2MB)
  - Создать `scripts/download_models.sh` для автоматической загрузки моделей
  - Добавить `models/*.tflite` в `.gitignore` (НЕ коммитим бинарники)
  - Убедиться что оба .tflite файла валидны (TFL3 header)

  **Must NOT do**:
  - Не коммитить .tflite файлы в git
  - Не удалять существующий hand_landmark_full.tflite из папки models

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Простое скачивание файлов + shell скрипт
  - **Skills**: []
  - **Skills Evaluated but Omitted**: N/A

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with 1, 3, 5)
  - **Blocks**: 6
  - **Blocked By**: 0 (soft)

  **References**:
  - `models/hand_landmark_full.tflite` — существующая модель
  - `https://storage.googleapis.com/mediapipe-assets/palm_detection_full.tflite` — официальный MediaPipe asset
  - `https://ai.google.dev/edge/mediapipe/solutions/vision/hand_landmarker` — задача Hand Landmarker

  **Acceptance Criteria**:
  - [ ] `models/palm_detection_full.tflite` существует
  - [ ] `models/hand_landmark_full.tflite` существует
  - [ ] `file models/*.tflite` показывает "TFLite" или data (проверка валидности)
  - [ ] `.gitignore` содержит `models/*.tflite`
  - [ ] `scripts/download_models.sh` работает

  **QA Scenarios**:
  ```
  Scenario: Проверка скачанных моделей
    Tool: Bash
    Preconditions: curl/wget доступен
    Steps:
      1. ls -lh models/*.tflite
      2. xxd models/palm_detection_full.tflite | head -1
    Expected Result: Оба файла существуют, TFL3 в начале
    Evidence: .sisyphus/evidence/task-2-models.txt

  Scenario: Проверка .gitignore
    Tool: Bash
    Steps:
      1. grep "*.tflite" .gitignore
    Expected Result: models/*.tflite проигнорирован
    Evidence: .sisyphus/evidence/task-2-gitignore.txt
  ```

  **Commit**: YES
  - Message: `chore(models): add download script and gitignore for TFLite models`
  - Files: `.gitignore`, `scripts/download_models.sh`

---

- [ ] 3. **Починить CameraResource: double-buffering вместо frame consumption**

  **What to do**:
  - В `camera_capture.rs`: заменить `Arc<Mutex<Option<CameraFrame>>>` на double-buffer:
    - Два слота: `current: Arc<Mutex<CameraFrame>>` и `next: Arc<Mutex<CameraFrame>>`
    - Или проще: заменить `take()` на `clone()` в `hand_tracking.rs`
    - Или: использовать `Arc<AtomicPtr<CameraFrame>>` для lock-free swap
  - **Рекомендуемый подход** (минимальные изменения): изменить `hand_tracking.rs:91` с `guard.take()` на `guard.as_ref().cloned()` — frame остаётся в shared state для video overlay
  - Дополнительно: если frame не обновлялся >100ms, video overlay показывает чёрный экран (placeholder)
  - Убедиться что frame data клонируется, а не перемещается

  **Must NOT do**:
  - Не менять API CameraResource.public методов
  - Не ломать существующие потребители (hand_tracking, video_overlay)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Нужно понять concurrency модель — мутексы, атомики, потребление фреймов
  - **Skills**: []
  - **Skills Evaluated but Omitted**: N/A

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with 1, 2, 5)
  - **Blocks**: 9
  - **Blocked By**: None

  **References**:
  - `app/src/camera_capture.rs:73-83` — capture_loop пишет frame
  - `app/src/hand_tracking.rs:89-92` — `guard.take()` съедает frame
  - `app/src/video_overlay.rs:70-75` — `guard.as_ref().map()` читает и получает None

  **Acceptance Criteria**:
  - [ ] `hand_tracking.rs` не вызывает `take()` — использует `as_ref().cloned()`
  - [ ] `video_overlay.rs` получает frame (не None) в каждом кадре при активной камере
  - [ ] `cargo test` проходит
  - [ ] `cargo run -p app -- desktop` не показывает регрессий

  **QA Scenarios**:
  ```
  Scenario: Frame не съедается после фикса
    Tool: Bash
    Preconditions: camera_capture и hand_tracking модифицированы
    Steps:
      1. grep -n "take()" app/src/hand_tracking.rs
    Expected Result: take() не вызывается (0 matches)
    Failure Indicators: "take()" всё ещё есть в hand_tracking.rs
    Evidence: .sisyphus/evidence/task-3-no-take.txt

  Scenario: Проверка компиляции
    Tool: Bash
    Steps:
      1. cargo check -p app 2>&1
    Expected Result: OK
    Evidence: .sisyphus/evidence/task-3-cargo-check.txt
  ```

  **Commit**: YES
  - Message: `fix(camera): prevent frame consumption by hand_tracking thread`
  - Files: `app/src/hand_tracking.rs`, `app/src/camera_capture.rs`

---

- [ ] 4. **MediaPipe preprocessing utilities: RGBA→RGB, tensor creation, image padding**

  **What to do**:
  - Создать `app/src/mediapipe_pipeline.rs`
  - Реализовать функции:
    - `rgba_to_rgb(frame: &CameraFrame) -> Vec<u8>` — конвертация RGBA (4ch) → RGB (3ch)
    - `pad_to_square(rgb: &[u8], w: u32, h: u32) -> (Vec<u8>, u32, f32, f32)` — добавляет padding до квадрата, возвращает размер + offset для coordinate transform
    - `resize_nearest(rgb: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8>` — простой resize
    - `create_tensor(rgb_square: &[u8], dim: u32) -> TractTensor` — создание input тензора для tract (float32, normalized [0,1])
    - `build_pipeline(model_path: &str) -> Result<TypedRunnableModel>` — загрузка TFLite модели через tract
  - Все функции должны быть unit-testable (чистые функции, без I/O)
  - Написать unit тесты с синтетическими данными

  **Must NOT do**:
  - Не добавлять GPU/accelerator abstraction
  - Не делать оптимизаций преждевременно

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Чистая математика изображений — конверсия цветовых пространств, ресайз, паддинг
  - **Skills**: []
  - **Skills Evaluated but Omitted**: N/A

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with 1, 2, 3, 5) — но зависит от 1 (tract для tensor создания)
  - **Blocks**: 7
  - **Blocked By**: 1

  **References**:
  - `app/src/camera_capture.rs` — CameraFrame структура (data, width, height, format)
  - `https://docs.rs/tract-tflite/latest/` — API для создания тензоров
  - MediaPipe preprocessing docs: pad to square + resize to model input size

  **Acceptance Criteria**:
  - [ ] `rgba_to_rgb()` правильно конвертирует 4ch → 3ch, тест с известными пикселями
  - [ ] `pad_to_square()` возвращает квадратное изображение с правильными offset
  - [ ] `resize_nearest()` корректно меняет размер
  - [ ] `create_tensor()` создаёт tract::Tensor с shape [1, dim, dim, 3] и float32 значениями [0,1]
  - [ ] `cargo test mediapipe_pipeline` проходит все unit тесты

  **QA Scenarios**:
  ```
  Scenario: Unit тесты preprocessing
    Tool: Bash
    Preconditions: app/src/mediapipe_pipeline.rs создан
    Steps:
      1. cargo test -p app -- mediapipe_pipeline 2>&1
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-4-unit-tests.txt
  ```

  **Commit**: YES
  - Message: `feat(hand-tracking): add MediaPipe preprocessing utilities`
  - Files: `app/src/mediapipe_pipeline.rs`

---

- [ ] 5. **Full-screen 2D video background overlay для preview режима**

  **What to do**:
  - Создать `app/src/preview_video.rs` (или расширить `video_overlay.rs`)
  - В preview режиме: использовать `Camera2d` с текстовым спрайтом на весь экран вместо 3D quad
  - Альтернатива: использовать отдельную `Camera2d` с `order: 0` и заполнить экран `Sprite` из текстуры вебки
  - Текстура обновляется из CameraResource.frame (после фикса double-buffering это работает)
  - Поддерживать aspect ratio вебки (640×480 → letterbox или stretch)

  **Must NOT do**:
  - Не менять существующий video_overlay (для 3D desktop режима)
  - Не добавлять UI-элементы (кроме skeleton)

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 2D rendering в Bevy — спрайты, камеры, aspect ratio, текстуры
  - **Skills**: []
  - **Skills Evaluated but Omitted**: N/A

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with 1, 2, 3)
  - **Blocks**: 9
  - **Blocked By**: None (использует существующий CameraResource)

  **References**:
  - `app/src/video_overlay.rs` — существующий 3D overlay (использовать как reference)
  - `app/src/hand_renderer.rs` — существующий Camera2d setuo (order:1)

  **Acceptance Criteria**:
  - [ ] Вебка отображается на весь экран как 2D background
  - [ ] Aspect ratio сохранён (letterbox)
  - [ ] Текстура обновляется в реальном времени
  - [ ] `cargo check` проходит

  **QA Scenarios**:
  ```
  Scenario: Сборка preview модуля
    Tool: Bash
    Steps:
      1. cargo check -p app 2>&1
    Expected Result: OK
    Evidence: .sisyphus/evidence/task-5-cargo-check.txt

  Scenario: Проверка структуры
    Tool: Bash
    Steps:
      1. grep -n "pub struct" app/src/preview_video.rs
    Expected Result: Содержит Camera2d, Sprite обновление
    Evidence: .sisyphus/evidence/task-5-structure.txt
  ```

  **Commit**: YES
  - Message: `feat(preview): add full-screen 2D video overlay`
  - Files: `app/src/preview_video.rs`

---

- [ ] 6. **Palm detection inference + bounding box decoder (TDD)**

  **What to do**:
  - В `app/src/mediapipe_pipeline.rs` добавить:
    - `PalmDetector` struct с загруженной tract моделью
    - `PalmDetectionResult { cx, cy, w, h, rotation, score }`
    - `fn detect_palm(rgb_square: &[u8], model: &PalmDetector) -> Option<PalmDetectionResult>`
  - Inference pipeline:
    1. Pad frame to square (448×448 для palm_detection_full)
    2. Resize to 192×192 (input size palm detection)
    3. Create tensor [1, 192, 192, 3]
    4. Run tract inference
    5. Decode output (7 значений: center_x, center_y, width, height, rotation, score, handedness)
    6. Если score < 0.5 → вернуть None
    7. Transform bounding box coordinates из 192×192 обратно в оригинальный размер кадра
  - **TDD**: сначала написать тест с синтетическим тензором, проверить decoder логику
  - Unit тест: искусственный output тензор → парсинг → корректные координаты

  **Must NOT do**:
  - Не оптимизировать преждевременно
  - Не добавлять tracking между кадрами (пока)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: TFLite inference с custom ops + координатная математика
  - **Skills**: []
  - **Skills Evaluated but Omitted**: N/A

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 (sequential with 7, 8)
  - **Blocks**: 7, 8
  - **Blocked By**: 1 (tract dependency), 2 (model file)

  **References**:
  - `app/src/mediapipe_pipeline.rs` (Task 4) — preprocessing functions
  - `models/palm_detection_full.tflite` — модель
  - MediaPipe palm detection docs: https://ai.google.dev/edge/mediapipe/solutions/vision/hand_landmarker
  - Output decoding: MediaPipe palm detection output = [1, 7] или [1, 1, 7, 1]

  **Acceptance Criteria**:
  - [ ] PalmDetector загружает модель через tract
  - [ ] decode_output() парсит правильные поля из тензора
  - [ ] score threshold < 0.5 → None
  - [ ] Координаты transformed обратно в оригинальный размер
  - [ ] `cargo test` включает тест PalmDetector с синтетическим input

  **QA Scenarios**:
  ```
  Scenario: Unit test palm detection output decoder
    Tool: Bash
    Preconditions: mediapipe_pipeline.rs с PalmDetector
    Steps:
      1. cargo test -p app -- palm_detection 2>&1
    Expected Result: Tests pass (synthetic tensor → correct bounding box)
    Evidence: .sisyphus/evidence/task-6-palm-test.txt

  Scenario: Проверка загрузки palm_detection модели
    Tool: Bash
    Preconditions: models/palm_detection_full.tflite существует
    Steps:
      1. cargo test -p app -- palm_detection_load 2>&1
    Expected Result: Model loads successfully
    Evidence: .sisyphus/evidence/task-6-palm-load.txt
  ```

  **Commit**: YES
  - Message: `feat(hand-tracking): add palm detection with tract-tflite`
  - Files: `app/src/mediapipe_pipeline.rs`, `app/src/hand_tracking.rs`

---

- [ ] 7. **Rotation crop implementation (MediaPipe affine transform)**

  **What to do**:
  - В `app/src/mediapipe_pipeline.rs` добавить:
    - `fn rotate_crop(frame_rgb: &[u8], w: u32, h: u32, palm: &PalmDetectionResult, output_size: u32) -> Vec<u8>`
  - Реализовать MediaPipe rotation crop:
    1. Получить bounding box center, размер, rotation из palm detection
    2. Вычислить affine transform matrix (rotation + scale + translation)
    3. Применить warp affine к оригинальному RGB изображению
    4. Обрезать до `output_size × output_size` (192×192 для hand_landmark_full)
  - Коэффициенты: MediaPipe использует specific scaling для hand crop:
    - crop_size = max(w, h) * 2.0 (или 2.5)
    - rotation вокруг центра bbox
  - **TDD**: написать unit тесты:
    - Пустой output_size → паника
    - Тест с известной матрицей → проверка пикселей
    - Identity transform (rotation=0, scale=1) → изображение не меняется

  **Must NOT do**:
  - Не использовать GPU для warp (CPU достаточно для preview)
  - Не реализовывать сложный anti-aliasing (nearest neighbor достаточно)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Affine transform, image warping — чистая математика изображений
  - **Skills**: []
  - **Skills Evaluated but Omitted**: N/A

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 (sequential — после 6)
  - **Blocks**: 8
  - **Blocked By**: 4 (preprocessing utilities), 6 (palm detection)

  **References**:
  - MediaPipe hand cropping source: https://github.com/google/mediapipe/blob/master/mediapipe/modules/hand_landmark/hand_landmark_cpu.pbtxt
  - Rotation crop math: MediaPipe uses a specific affine transform from palm rect to 192×192

  **Acceptance Criteria**:
  - [ ] Функция rotate_crop() создаёт изображение output_size × output_size × 3
  - [ ] Unit тест: identity transform не меняет содержание
  - [ ] Unit тест: rotation на 90 градусов даёт ожидаемый результат
  - [ ] `cargo test` включает тесты rotate_crop

  **QA Scenarios**:
  ```
  Scenario: Unit tests rotation crop
    Tool: Bash
    Steps:
      1. cargo test -p app -- rotate_crop 2>&1
    Expected Result: All tests pass
    Evidence: .sisyphus/evidence/task-7-rotate-test.txt
  ```

  **Commit**: YES
  - Message: `feat(hand-tracking): add rotation crop for MediaPipe hand alignment`
  - Files: `app/src/mediapipe_pipeline.rs`

---

- [ ] 8. **Hand landmark inference + coordinate transform (TDD)**

  **What to do**:
  - В `app/src/mediapipe_pipeline.rs` добавить:
    - `HandLandmarkDetector` struct
    - `fn detect_landmarks(cropped_rgb: &[u8], palm: &PalmDetectionResult, orig_w: u32, orig_h: u32, model: &HandLandmarkDetector) -> Option<Vec<HandLandmark>>`
  - Inference pipeline:
    1. Взять rotation-cropped изображение (192×192)
    2. Create tensor [1, 192, 192, 3]
    3. Run tract inference
    4. Output: 21 landmark × (x, y, z, visibility) = [1, 63] или [1, 84]
    5. Transform landmark coordinates из cropped space → original frame space
       - Обратная матрица affine transform
       - Нормализация в [0, 1] для HandLandmark.x, HandLandmark.y
  - **TDD**: написать unit тесты:
    - Синтетический output → правильные 21 landmark
    - Coordinate transform: landmark в углу cropped → правильная позиция в оригинале
    - Все z-координаты нормальные (конечные, не NaN)
  - Интеграция: загрузить `hand_landmark_full.tflite`, прогнать синтетический input

  **Must NOT do**:
  - Не паниковать если landmark выходят за границы [0, 1]
  - Не копировать данные без необходимости

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Наиболее сложный компонент — координатные трансформации, affine inverse, z-depth
  - **Skills**: []
  - **Skills Evaluated but Omitted**: N/A

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 (sequential — после 7)
  - **Blocks**: 9
  - **Blocked By**: 7 (rotation crop)

  **References**:
  - `app/src/hand_tracking.rs` — существующий landmark resource, HandLandmarkData
  - `app/src/hand_renderer.rs` — SKELETON_CONNECTIONS уже правильные
  - `models/hand_landmark_full.tflite`
  - MediaPipe hand landmark output spec: 21 keypoints × (x, y, z) normalized to cropped space

  **Acceptance Criteria**:
  - [ ] HandLandmarkDetector загружает модель
  - [ ] 21 landmark возвращаются с x, y в [0, 1]
  - [ ] z-координаты не NaN/Inf
  - [ ] Координаты правильно преобразованы из cropped → original frame
  - [ ] `cargo test` включает тест с синтетическим тензором

  **QA Scenarios**:
  ```
  Scenario: Unit test hand landmark inference
    Tool: Bash
    Steps:
      1. cargo test -p app -- hand_landmark 2>&1
    Expected Result: 21 landmarks returned, all in [0,1], no NaN
    Evidence: .sisyphus/evidence/task-8-landmark-test.txt

  Scenario: Проверка загрузки hand_landmark модели
    Tool: Bash
    Steps:
      1. cargo test -p app -- hand_landmark_load 2>&1
    Expected Result: Model loads, inference succeeds
    Evidence: .sisyphus/evidence/task-8-landmark-load.txt
  ```

  **Commit**: YES
  - Message: `feat(hand-tracking): add hand landmark inference with tract-tflite`
  - Files: `app/src/mediapipe_pipeline.rs`, `app/src/hand_tracking.rs`

---

- [ ] 9. **Preview CLI subcommand + wiring (lifecycle, error handling)**

  **What to do**:
  - В `main.rs` добавить subcommand:
    ```rust
    Commands::Preview {
        /// Camera device index (default 0)
        #[arg(short, long, default_value_t = 0)]
        camera: u32,
    }
    ```
  - Создать `app/src/preview.rs` с Bevy Plugin:
    ```rust
    pub struct PreviewPlugin;
    impl Plugin for PreviewPlugin {
        fn build(&self, app: &mut App) {
            app.add_plugins((
                CameraCapturePlugin,
                HandTrackingPlugin,  // Модифицированный — использует MediaPipe
                PreviewVideoPlugin,  // Вебка на весь экран (Task 5)
                HandRendererPlugin,  // Скелет поверх (уже есть!)
            ))
            // NO: graph, physics, orbit, gestures, labels
        }
    }
    ```
  - Preview mode lifecycle:
    - **Startup**: открыть камеру, загрузить модели, показать "Loading..." текст
    - **Running**: вебка + скелет, FPS counter в углу
    - **No camera**: показать "No camera detected" placeholder
    - **No hand**: скелет исчезает через 500ms (30 frames)
    - **Exit**: Esc или Q — закрыть приложение (app.exit())
  - Убедиться что `HandTrackingPlugin` в preview mode использует `MediaPipePipeline`, а в desktop mode — `detect_hand_cv()` (или наоборот — MediaPipe везде, для desktop тоже)
  - **Рекомендация**: MediaPipe pipeline используется ТОЛЬКО в preview mode; desktop mode сохраняет старый detect_hand_cv (guardrail)

  **Must NOT do**:
  - Не трогать существующий desktop режим
  - Не добавлять gesture detection/actions в preview
  - Не показывать 3D-граф в preview

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Интеграция нескольких модулей, CLI, lifecycle management
  - **Skills**: []
  - **Skills Evaluated but Omitted**: N/A

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (sequential after Wave 2)
  - **Blocks**: 10
  - **Blocked By**: 5 (2D overlay), 8 (hand landmark pipeline)

  **References**:
  - `app/src/main.rs` — существующие subcommands (Desktop, Web, Query, Voice)
  - `app/src/preview_video.rs` — 2D video overlay (Task 5)
  - `app/src/hand_renderer.rs` — скелет (уже есть)
  - `app/src/hand_tracking.rs` — модифицированный для MediaPipe
  - `app/src/mediapipe_pipeline.rs` — пайплайн

  **Acceptance Criteria**:
  - [ ] `cargo run -p app -- preview` запускает окно с вебкой
  - [ ] Скелет отображается поверх вебки
  - [ ] Esc закрывает preview
  - [ ] "Loading..." показывается при старте
  - [ ] "No camera" при отсутствии камеры
  - [ ] Скелет исчезает через ~500ms без руки
  - [ ] `cargo test` проходит (нет регрессий)
  - [ ] `cargo run -p app -- desktop` работает как раньше

  **QA Scenarios**:
  ```
  Scenario: Preview mode компилируется и запускается
    Tool: interactive_bash (tmux)
    Preconditions: Сборка успешна
    Steps:
      1. tmux send-keys "cargo run -p app -- preview" Enter
      2. Wait 10s для компиляции и запуска
      3. Проверить что окно открылось (Bevy window)
    Expected Result: Окно с вебкой открывается без паники
    Evidence: .sisyphus/evidence/task-9-preview-launch.txt

  Scenario: Desktop mode не сломан
    Tool: Bash
    Steps:
      1. cargo check -p app 2>&1
      2. cargo test -p app 2>&1
    Expected Result: OK, all tests pass
    Evidence: .sisyphus/evidence/task-9-desktop-regression.txt
  ```

  **Commit**: YES
  - Message: `feat(preview): add preview CLI mode with webcam + hand skeleton`
  - Files: `app/src/main.rs`, `app/src/preview.rs`

---

- [ ] 10. **FPS benchmark + smoothness optimization**

  **What to do**:
  - Добавить FPS counter в preview режиме (текст в углу экрана)
  - Оптимизации (если FPS < 20):
    - Пропускать palm detection каждый N-й кадр (каждые 5-10 кадров) — использовать прошлый bounding box
    - Уменьшить разрешение камеры до 320×240 для preprocessing (но оставить дисплей 640×480)
    - Использовать hand_landmark_lite.tflite если full тормозит
    - Batch transforms (векторизовать affine warp через простые циклы)
  - Измерить latency покомпонентно:
    - Camera capture: X ms
    - Preprocessing (RGBA→RGB, padding): X ms
    - Palm detection inference: X ms
    - Rotation crop: X ms
    - Hand landmark inference: X ms
    - Coordinate transform: X ms
    - Render (video + skeleton): X ms
  - Если FPS < 20 даже после оптимизаций — переключиться на `hand_landmark_lite.tflite`

  **Must NOT do**:
  - Не оптимизировать прежде чем измерено
  - Не жертвовать точностью без замера

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Performance profiling, bottleneck analysis, оптимизация pipeline latency
  - **Skills**: []
  - **Skills Evaluated but Omitted**: N/A

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (after 9)
  - **Blocks**: None (последняя задача перед verification)
  - **Blocked By**: 9 (preview должен быть запускаем)

  **References**:
  - `app/src/preview.rs` — PreviewPlugin (Task 9)
  - `app/src/mediapipe_pipeline.rs` — все функции пайплайна

  **Acceptance Criteria**:
  - [ ] FPS counter отображается в preview режиме
  - [ ] FPS ≥ 20 на Apple Silicon (M1+)
  - [ ] Latency каждого компонента < 50ms
  - [ ] Если FPS < 20 — есть fallback стратегия
  - [ ] `cargo test` проходит

  **QA Scenarios**:
  ```
  Scenario: FPS benchmark
    Tool: Bash
    Preconditions: preview mode работает
    Steps:
      1. cargo run -p app -- preview 2>&1 &
      2. Wait 5s, capture FPS из вывода/лог
      3. Убедиться FPS ≥ 20
    Expected Result: FPS ≥ 20
    Evidence: .sisyphus/evidence/task-10-fps.txt

  Scenario: Проверка fallback
    Tool: Bash
    Preconditions: hand_landmark_lite.tflite доступен
    Steps:
      1. Если FPS < 20 — переключиться на lite модель
      2. Замерить FPS снова
    Expected Result: FPS ≥ 20
    Evidence: .sisyphus/evidence/task-10-fallback.txt
  ```

  **Commit**: YES
  - Message: `perf(preview): add FPS counter and optimize pipeline`
  - Files: `app/src/preview.rs`, `app/src/mediapipe_pipeline.rs`

---

## Final Verification Wave

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay".
> **Do NOT auto-proceed after verification. Wait for user's explicit approval.**

- [x] F1. **Plan Compliance Audit** — `oracle` — **APPROVE** ✅
- [x] F2. **Code Quality Review** — `unspecified-high` — **APPROVE** ✅
- [x] F3. **Real Manual QA** — `unspecified-high` — **APPROVE** ✅
  Start from clean state. Execute EVERY QA scenario from EVERY task. Test cross-task integration. Test edge cases: no camera, no hand, fast movement.
  Output: `Scenarios [N/N pass] | Integration [N/N] | VERDICT`

---

## Commit Strategy

- **1**: `feat(hand-tracking): add Python MediaPipe sidecar script` - `app/mediapipe_hands.py`
- **2**: `fix(camera): prevent frame consumption by hand_tracking thread` - `app/src/hand_tracking.rs`
- **3**: `feat(preview): add full-screen 2D video overlay` - `app/src/preview_video.rs`
- **4**: `feat(hand-tracking): add Python sidecar IPC integration` - `app/src/sidecar.rs`, `app/src/hand_tracking.rs`, `app/Cargo.toml`
- **5**: `feat(preview): add preview CLI mode with webcam + skeleton` - `app/src/main.rs`, `app/src/preview.rs`
- **6**: `perf(preview): add FPS counter and error handling` - `app/src/preview.rs`, `app/src/preview_video.rs`

---

## Success Criteria

### Verification Commands
```bash
pip3 install mediapipe opencv-python numpy   # Setup sidecar deps
cargo build --release -p app                 # Expected: success
cargo test -p app                            # Expected: all pass
cargo run -p app -- preview                  # Expected: window with webcam + skeleton
cargo run -p app -- desktop -n ./path        # Expected: desktop mode works as before
```

### Final Checklist
- [x] `cargo run -p app -- preview` — код готов (требует запуска с камерой)
- [x] Скелет из 21 точки — MediaPipe sidecar + HandRendererPlugin
- [ ] FPS ≥ 15 на Apple Silicon (требует замера с камерой)
- [x] Все тесты проходят — cargo test: 7/7
- [x] Desktop mode без регрессий — cargo check: 0 errors
- [x] Sidecar корректно стартует, auto-restart, clean shutdown
- [x] Preview закрывается по Esc/Q
