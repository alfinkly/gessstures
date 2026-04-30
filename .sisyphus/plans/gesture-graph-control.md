# Gesture-Controlled Knowledge Graph

## TL;DR

> **Quick Summary**: Добавить hand tracking (nokhwa + ort + MediaPipe TFLite) для управления 3D графом жестами рук, как интерфейс Тони Старка.
> 
> **Deliverables**:
> - Camera feed rendering в Bevy окне
> - Hand skeleton overlay (21 точек)
> - Gesture recognition (movement → follow cursor, pinch → pin, fist → read, open palm → exit)
> - Graph navigation state machine (3 режима)
> - Vertex highlight/selection visual feedback
> - Vertex content viewer
> 
> **Estimated Effort**: XL (много новых подсистем)
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: T1 (spike) → T3 (camera) → T5 (tracking) → T7 (gestures) → T9 (state machine) → T13 (graph navigation) → T15 (UI overlay)

---

## Context

### Original Request
Хочу управлять графом знаний (markdown файлы со ссылками) жестами рук через камеру. Как интерфейс Тони Старка - движение руки по осям X/Y напрямую маппится на движение по графу. Ближайшая вершина к тому куда направляется рука - фоллоу.

Gesture mappings:
- **Hand movement (X/Y)** → перемещение курсора по графу
- **Pinch** (большой + указательный) → войти в вершину
- **Fist** (кулак) → прочитать содержимое файла
- **Open palm** → выйти из вершины

Navigation modes:
- **Graph View** - весь граф, бегаю по вершинам
- **Vertex Pinned** - закреплена одна вершина, бегаю по её исходящим ссылкам
- **Vertex Content** - открыт просмотр содержимого файла

### Interview Summary

**Key Discussions**:
- Iron Man style interface - движение руки напрямую связано с навигацией по графу
- MediaPipe - да, использовать Rust (ort + TFLite)
- Текстовая визуализация распознанного жеста - достаточно
- Текущее управление (клавиатура/мышь) оставить как резерв, поддерживать не нужно

**Research Findings**:
- **Camera**: nokhwa crate - cross-platform, 234k downloads, wgpu integration
- **Hand Tracking**: ort (ONNX Runtime) + MediaPipe TFLite hand landmark model (21 точка)
- **Note**: mediapipe-rs от WasmEdge только для WASM, не подходит для desktop

### Metis Review

**Identified Gaps** (addressed):
1. **Stack choice**: ort + TFLite vs mediapipe-rs vs tflite crate - spike нужен перед полной реализацией
2. **Threading model**: Как запускать инференс не блокируя Bevy 60fps - нужен отдельный thread
3. **2D→3D cursor**: Пространственный запрос "nearest vertex" - нужна система поиска ближайшей вершины
4. **Camera overlay**: PiP vs debug vs AR - определить один режим (AR overlay выбран)
5. **Edge cases**: рука уходит из кадра, две руки, низкое освещение - добавить в acceptance criteria

---

## Work Objectives

### Core Objective
Добавить gesture-based навигацию по 3D графу знаний с real-time hand tracking и визуализацией.

### Concrete Deliverables
- nokhwa integration для camera capture
- ort + TFLite hand landmark model для 21-точечного скелета
- Gesture classification (movement, pinch, fist, open palm)
- GraphInteractionMode state machine (3 режима)
- Camera feed + hand skeleton overlay в Bevy
- Vertex highlight system (scale, color, bold)
- Vertex content viewer (markdown текст в UI)

### Definition of Done
- [ ] Hand tracking работает в реальном времени (>15 FPS)
- [ ] Все 4 жеста распознаются корректно
- [ ] Перемещение руки по X/Y → ближайшая вершина в графе
- [ ] Pinch → вход в вершину
- [ ] Fist → просмотр содержимого файла
- [ ] Open palm → выход из вершины
- [ ] Видео с камеры видно в окне приложения
- [ ] Скелет рук отображается поверху видео

### Must Have
- Реальный hand tracking (не симуляция)
- Работа на desktop (macOS/Linux/Windows)
- 60 FPS рендеринг (инференс может быть медленнее)

### Must NOT Have (Guardrails)
- Не изменять существующую функциональность графа без необходимости
- Не удалять keyboard/mouse управление (оставить как fallback)
- Не добавлять AI-based semantic search (это отдельная фича)
- Не добавлять voice input

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** - ALL verification is agent-executed.

### Test Decision
- **Infrastructure exists**: NO - нужно добавить
- **Automated tests**: YES (tests-after) - юнит тесты для gesture classification
- **Framework**: custom test harness (синтетические landmark данные)

### QA Policy
Every task MUST include agent-executed QA scenarios.

- **Frontend/UI**: Playwright - verify video feed renders, skeleton overlay visible
- **Hand Tracking**: Bash (run benchmark, check FPS)
- **Gesture Classification**: unit tests с synthetic landmarks
- **Graph Navigation**: integration tests с fake hand movements

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation - CAN START IMMEDIATELY):
├── Task 1: Add nokhwa + ort dependencies, download hand landmark model [unspecified-high]
├── Task 2: Camera capture system (nokhwa integration) [unspecified-high]
├── Task 3: Hand landmark inference (ort + TFLite model) [unspecified-high]
└── Task 4: Camera video to Bevy texture [unspecified-high]
```

### Dependency Matrix
- **1**: - - 2, 3, 4
- **2**: 1 - 5, 7
- **3**: 1 - 5, 6, 7
- **4**: 1 - 5, 7
- **5**: 2, 3, 4 - 6, 8
- **6**: 3 - 8
- **7**: 2, 3, 4 - 8, 9
- **8**: 5, 6, 7 - 9, 10, 11
- **9**: 7, 8 - 12, 13, 14, 15, 16
- **10**: 8 - 13
- **11**: 8 - 13
- **12**: 9 - 13
- **13**: 9, 10, 11, 12 - F1
- **14**: 9 - F1
- **15**: 9 - F1
- **16**: 9 - F1
- **F1**: 13, 14, 15, 16 - F2, F3
- **F2**: F1 - -
- **F3**: F1 - -

### Agent Dispatch Summary
- **1**: **1** - T1 → `deep`
- **2**: **3** - T2 → `unspecified-high`, T3 → `unspecified-high`, T4 → `unspecified-high`
- **3**: **4** - T5 → `visual-engineering`, T6 → `deep`, T7 → `deep`, T8 → `unspecified-high`
- **4**: **5** - T9 → `unspecified-high`, T10 → `deep`, T11 → `unspecified-high`, T12 → `visual-engineering`, T13 → `deep`
- **5**: **3** - T14 → `visual-engineering`, T15 → `unspecified-high`, T16 → `deep`
- **FINAL**: **3** - F1 → `unspecified-high`, F2 → `unspecified-high`, F3 → `unspecified-high`

---

## TODOs

- [x] 1. Add nokhwa + ort dependencies, download hand landmark model

  **What to do**:
  - Add nokhwa and ort to Cargo.toml dependencies
  - Download MediaPipe hand landmark TFLite model from official source
  - Integrate both crates into Bevy app structure

  **Must NOT do**:
  - Not implement full inference pipeline yet - just dependencies + model

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Setup work - add deps and download files
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with T2, T3, T4)
  - **Blocks**: All subsequent tasks
  - **Blocked By**: None (can start immediately)

  **References**:
  - `app/Cargo.toml` - Current dependencies
  - nokhwa docs: `https://docs.rs/nokhwa/latest/nokhwa/`
  - ort docs: `https://ort.pyke.io/introduction`
  - MediaPipe hand landmark: `https://developers.google.com/mediapipe/solutions/vision/hand_landmarker`

  **Acceptance Criteria**:
  - [ ] nokhwa in Cargo.toml with input-native feature
  - [ ] ort in Cargo.toml
  - [ ] hand_landmark.tflite model file downloaded

  **QA Scenarios**:
  (embedded in subsequent tasks - no standalone QA needed)

  **Commit**: YES
  - Message: `chore(gesture): add nokhwa and ort dependencies, download hand landmark model`
  - Files: `app/Cargo.toml`, `models/hand_landmark.tflite`

- [x] 2. Camera capture system

  **What to do**:
  - Integrate nokhwa into Bevy app
  - Create camera capture plugin/system
  - Handle camera initialization, frame grabbing, buffer management
  - Implement threaded capture for non-blocking operation
  - Handle camera disconnect/reconnect gracefully

  **Must NOT do**:
  - Not render video yet - just capture and store frames

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Integration work with existing Bevy codebase, moderate complexity
  - **Skills**: []
  - **Skills Evaluated but Omitted**:

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with T1, T3, T4)
  - **Blocks**: T5 (skeleton rendering needs camera frames)
  - **Blocked By**: T1 (need to verify nokhwa works first)

  **References**:
  - `app/src/main.rs` - How plugins are added to Bevy
  - `app/src/camera.rs` - Camera system pattern

  **Acceptance Criteria**:
  - [ ] nokhwa integrated in Cargo.toml
  - [ ] CameraPlugin created with proper Bevy Resource
  - [ ] Frames captured at >15 FPS without blocking main thread
  - [ ] Graceful handling when camera disconnects

  **QA Scenarios**:

  Scenario: Camera capture in Bevy app
    Tool: Bash
    Preconditions: nokhwa integrated, camera available
    Steps:
      1. Build and run app with camera enabled
      2. Check that frames are being captured
      3. Verify frame rate is adequate (>15 FPS)
    Expected Result: Camera capture running without blocking
    Evidence: .sisyphus/evidence/task-2-capture.log

  **Commit**: NO

- [x] 3. Hand landmark inference

  **What to do**:
  - Load hand landmark TFLite model via ort
  - Preprocess camera frames (resize, normalize, convert format)
  - Run inference to get 21 hand landmarks
  - Postprocess landmarks (denormalize, convert to screen coords)
  - Optimize for real-time (>15 FPS)

  **Must NOT do**:
  - Not create rendering yet - just output landmark data

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: ML inference integration, moderate complexity
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with T1, T2, T4)
  - **Blocks**: T5, T6, T7
  - **Blocked By**: T1 (need model loaded first)

  **Acceptance Criteria**:
  - [ ] Hand landmark model loads at startup
  - [ ] Inference runs on each camera frame
  - [ ] Output: 21 landmarks with (x, y, z) coordinates
  - [ ] FPS: >15 on CPU

  **Commit**: NO

- [x] 4. Camera video to Bevy texture

  **What to do**:
  - Convert nokhwa frame to Bevy texture
  - Create texture and update each frame
  - Render camera feed as background in Bevy window
  - Handle different camera resolutions
  - Performance: don't reallocate texture each frame

  **Must NOT do**:
  - Not implement hand skeleton yet

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Texture handling and rendering integration
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with T1, T2, T3)
  - **Blocks**: T5 (skeleton overlay needs video as base)
  - **Blocked By**: T2 (need camera frames first)

  **Acceptance Criteria**:
  - [ ] Camera feed visible in Bevy window
  - [ ] Frame rate: >15 FPS rendering
  - [ ] No memory leaks from texture reallocation

  **Commit**: NO

- [x] 5. Hand skeleton rendering

  **What to do**:
  - Render 21 hand landmarks as dots/points on top of camera feed
  - Draw lines connecting landmarks to show hand structure
  - Color-code by hand (left vs right) if both detected
  - Update skeleton in real-time with landmarks

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: UI/visual rendering work
  - **Skills**: [`visual-engineering`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with T6, T7, T8)
  - **Blocks**: T8 (gesture detection needs skeleton)
  - **Blocked By**: T1, T2, T3, T4

  **Acceptance Criteria**:
  - [ ] 21 dots visible at landmark positions
  - [ ] Lines connecting finger joints visible
  - [ ] Skeleton updates in real-time with hand movement

  **Commit**: NO

- [x] 6. Gesture classification system

  **What to do**:
  - Implement gesture recognition based on hand landmarks
  - Detect: open palm, fist, pinch (thumb + index touching)
  - Track hand movement direction (left, right, up, down)
  - Smooth gesture detection to avoid jitter

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Algorithm development for gesture recognition
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with T5, T7, T8)
  - **Blocks**: T8
  - **Blocked By**: T3

  **Acceptance Criteria**:
  - [ ] Open palm detected correctly (>90% accuracy)
  - [ ] Fist detected correctly (>90% accuracy)
  - [ ] Pinch detected correctly (>90% accuracy)
  - [ ] Hand movement direction detected

  **Commit**: NO

- [x] 7. Real-time gesture detection loop

  **What to do**:
  - Connect camera frames → inference → landmarks → gesture
  - Run detection loop at appropriate rate
  - Handle multiple hands (pick primary)
  - Add gesture debouncing (require gesture held for X frames)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Real-time system integration
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with T5, T6, T8)
  - **Blocks**: T8, T9, T13
  - **Blocked By**: T2, T3, T4

  **Acceptance Criteria**:
  - [ ] Detection runs continuously
  - [ ] Latency: <200ms from frame to gesture output
  - [ ] Handles no-hand-present state gracefully

  **Commit**: NO

- [x] 8. Gesture state machine

  **What to do**:
  - Map gestures to graph actions
  - Implement state transitions
  - Handle gesture sequences (e.g., pinch→release = pin action)
  - Add debounce and cooldown between actions

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: State management logic
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with T5, T6, T7)
  - **Blocks**: T9, T10, T11
  - **Blocked By**: T5, T6, T7

  **Acceptance Criteria**:
  - [ ] Each gesture maps to correct action
  - [ ] Actions fire on gesture transition (not continuously)
  - [ ] Cooldown prevents accidental double-actions

  **Commit**: NO

- [x] 9. GraphInteractionMode state machine

  **What to do**:
  - Add GraphInteractionMode resource to graph-core
  - Implement 3 modes: GraphView, VertexPinned, VertexContent
  - Handle mode transitions
  - Store pinned vertex ID and navigation state

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: State machine implementation
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with T10, T11, T12, T13)
  - **Blocks**: T12, T13, T14, T15, T16
  - **Blocked By**: T7, T8

  **Acceptance Criteria**:
  - [ ] GraphInteractionMode resource exists
  - [ ] 3 modes defined and switchable
  - [ ] Mode persists across frames

  **Commit**: NO

- [x] 10. 2D hand position → 3D graph cursor mapping

  **What to do**:
  - Map hand X/Y position to screen coordinates
  - Map screen coordinates to 3D world space
  - Handle camera perspective/FOV
  - Smooth cursor movement

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Coordinate transformation math
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with T9, T11, T12, T13)
  - **Blocks**: T13
  - **Blocked By**: T8

  **Acceptance Criteria**:
  - [ ] Hand position maps to 3D cursor position
  - [ ] Movement feels responsive and natural

  **Commit**: NO

- [x] 11. Nearest vertex spatial query

  **What to do**:
  - For current cursor position, find nearest graph vertex
  - Use spatial data structure for efficient search
  - Update "followed vertex" when cursor moves
  - Handle case when cursor is far from any vertex

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Spatial query implementation
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with T9, T10, T12, T13)
  - **Blocks**: T13
  - **Blocked By**: T8

  **Acceptance Criteria**:
  - [ ] Nearest vertex correctly identified
  - [ ] Query is fast enough for real-time

  **Commit**: NO

- [x] 12. Vertex highlight system

  **What to do**:
  - Visual feedback for followed vertex
  - Scale up highlighted vertex
  - Add color change or glow effect
  - Make bold (thicker edges or brighter color)

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: Visual enhancement work
  - **Skills**: [`visual-engineering`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with T9, T10, T11, T13)
  - **Blocks**: T13
  - **Blocked By**: T9

  **Acceptance Criteria**:
  - [ ] Followed vertex is visually distinct
  - [ ] Highlight updates in real-time

  **Commit**: NO

- [x] 13. Graph navigation via gestures

  **What to do**:
  - Connect gesture actions to graph navigation
  - Hand movement → move cursor, update followed vertex
  - Pinch → pin current vertex, enter Pinned mode
  - Open palm (in Pinned mode) → exit to Graph mode

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Integration of gesture → action → graph
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with T9, T10, T11, T12)
  - **Blocks**: F1
  - **Blocked By**: T9, T10, T11, T12

  **Acceptance Criteria**:
  - [ ] Hand movement navigates graph correctly
  - [ ] Pinch enters Pinned mode
  - [ ] Open palm exits to Graph mode

  **Commit**: NO

- [x] 14. Vertex content viewer

  **What to do**:
  - Display markdown content when in Content mode
  - Render text in UI overlay or side panel
  - Show file path and title
  - Handle long content with scrolling

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: UI text rendering
  - **Skills**: [`visual-engineering`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with T15, T16)
  - **Blocks**: F1
  - **Blocked By**: T9

  **Acceptance Criteria**:
  - [ ] Content displays when entering Content mode
  - [ ] Text is readable and scrollable

  **Commit**: NO

- [x] 15. Vertex links navigation

  **What to do**:
  - In Pinned mode, show outgoing links from pinned vertex
  - Navigate to linked vertices with hand movement
  - Allow entering linked vertices with pinch

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Link navigation logic
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with T14, T16)
  - **Blocks**: F1
  - **Blocked By**: T9

  **Acceptance Criteria**:
  - [ ] Can navigate through outgoing links
  - [ ] Can enter linked vertices

  **Commit**: NO

- [x] 16. Mode transitions

  **What to do**:
  - Smooth transitions between Graph/Pinned/Content modes
  - Handle all transition paths correctly
  - Gesture: fist → enter Content mode from Pinned
  - Gesture: open palm → exit to Graph from Pinned

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex state transition logic
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with T14, T15)
  - **Blocks**: F1
  - **Blocked By**: T9

  **Acceptance Criteria**:
  - [ ] All mode transitions work correctly
  - [ ] User can always return to Graph mode

  **Commit**: NO

---

## Final Verification Wave

- [x] F1. **Integration test - full gesture workflow**

  Run through complete user journey:
  1. Start app → camera feed visible
  2. Move hand → cursor follows, vertex highlights
  3. Pinch → vertex pinned
  4. Move to link → navigate links
  5. Fist → view content
  6. Open palm → exit back to Graph

  Output: `Workflow [COMPLETE/INCOMPLETE] | VERDICT`

- [x] F2. **Performance benchmark**

  Measure:
  - Camera → gesture detection latency
  - Frame rate during normal operation
  - Memory usage

  Output: `Latency [N ms] | FPS [N] | Memory [N MB] | VERDICT`

- [x] F3. **Edge cases verification**

  Test:
  - Hand leaves frame
  - Two hands in frame
  - No hand detected for extended time
  - Rapid gesture changes

  Output: `Edge cases [N/N handled] | VERDICT`

---

## Commit Strategy

- Commit after each task that produces working code
- Message: `feat(gesture): description`

---

## Success Criteria

### Verification Commands
```bash
cargo build -p app  # Should compile without errors
cargo test         # Gesture unit tests pass
```

### Final Checklist
- [ ] All 4 gestures work correctly
- [ ] Graph navigation via hand movement works
- [ ] Mode transitions work
- [ ] Camera feed renders in window
- [ ] Hand skeleton visible
- [ ] Vertex content viewer works
- [ ] Performance acceptable (>15 FPS)