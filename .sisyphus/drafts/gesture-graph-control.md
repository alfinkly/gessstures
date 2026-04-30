# Draft: Gesture-Controlled Knowledge Graph

## User Requirements (confirmed)

### Core Goal
Управлять 3D графом знаний (markdown файлы со ссылками) через камеру с помощью жестов рук.

### Interaction Modes

**Mode 1: Graph View (обзор всего графа)**
- Бегать по вершинам жестами
- Вершины выделяются (увеличиваются, становятся жирнее)
- Вижу все файлы в графе

**Mode 2: Vertex View (внутри одной вершины)**
- Жест "закрепления" - переход в режим одной вершины
- Ходить по узлам/связям внутри этой вершины

### Gesture Types (Iron Man style)
1. **Hand movement (X/Y axis)** → перемещение курсора по графу, фоллоу ближайшая вершина
2. **Pinch** (большой + указательный = "защепление") → войти в вершину (закрепить файл)
3. **Fist (кулак)** → прочитать содержимое файла (открыть markdown в UI)
4. **Open palm** (открытая ладонь) → выйти из вершины (вернуться к обзору графа)

### Navigation Modes
- **Graph View**: вижу весь граф, рука управляет "фоллоу курсором" по ближайшей вершине
- **Vertex Pinned**: закрепил файл, могу бегать по его исходящим ссылкам ( outgoing links)
- **Vertex Content**: открыт просмотр содержимого файла (markdown text)

### Real-time Visualization (CRITICAL)
- Видеть картинку себя (камера)
- Видеть как код видит мои руки (hand skeleton overlay)
- Видеть распознанный жест в реальном времени

---

## Current State Analysis

### What's Implemented ✓
- GraphCore: структуры графа, события Bevy
- DataSources: парсинг markdown (H1, [[wikilinks]], [md](links))
- Renderer: 3D сферы (размер по degree), рёбра (линии), 3D лейблы
- Physics: force-directed layout
- Camera: orbit camera (мышь/клавиатура)
- Input: текстовый ввод запросов
- CLI: clap subcommands

### What's NOT Implemented ✗
1. **Hand tracking** - нет интеграции с mediapipe
2. **Gesture recognition** - нет распознавания жестов
3. **Gesture → Graph binding** - нет связи жестов с навигацией по графу
4. **Real-time camera overlay** - нет отображения видео с камеры
5. **Hand skeleton visualization** - нет отрисовки скелета рук поверх видео

---

## Gap Analysis

### CRITICAL Missing Components

| Component | Status | Priority |
|-----------|--------|----------|
| MediaPipe hand tracking integration | NOT STARTED | P0 |
| Camera feed rendering (webcam → Bevy) | NOT STARTED | P0 |
| Hand skeleton overlay rendering | NOT STARTED | P0 |
| Gesture classification algorithm | NOT STARTED | P0 |
| Graph navigation state machine | NOT STARTED | P0 |
| Vertex highlight/selection system | NOT STARTED | P1 |
| Vertex content viewing mode | NOT STARTED | P1 |

### Technical Questions

1. **Hand tracking**: mediapipe-rs или внешний процесс?
2. **Camera input**: как получать кадры в Bevy? (device.rs / capture crate)
3. **Gesture gestures**: какие конкретно жесты? Нужно определить:
   - Какой жест = "следующая вершина"
   - Какой жест = "предыдущая вершина"
   - Какой жест = "закрепить/войти"
   - Какой жест = "прочитать"
   - Какой жест = "выйти из вершины"
4. **Visual feedback**: как показывать распознанный жест? (text label, color, icon)

---

## Scope Boundaries

### INCLUDE
- MediaPipe hand tracking integration
- Real-time camera feed in Bevy window
- Hand skeleton overlay on camera feed
- Gesture classification system
- Graph navigation via gestures (2 modes)
- Vertex selection visual feedback (highlight, scale, bold)
- Vertex content viewer

### EXCLUDE (for now)
- AI/embedding-based graph search
- Voice input
- WebSocket server mode
- Complex gesture sequences (multi-hand)

---

## Open Questions

1. ~~Какие конкретно жесты~~ - ОПРЕДЕЛЕНО (iron man style)
2. ~~MediaPipe~~ - ДА
3. ~~Текстовая визуализация~~ - ХВАТИТ
4. **Режим клавиатуры параллельно?** - нужно ли оставить мышь/клавиатуру как резерв?

## Technical Decisions Needed
- ~~Как получать video stream в Bevy?~~ - выбрать наиболее производительный crate
- ~~MediaPipe в Rust~~ - ДА, в том же процессе
- Оставить текущее управление, поддерживать не нужно (резерв)

## Research Completed

### Camera Capture
- **nokhwa** - Лучший выбор
  - Cross-platform: macOS (AVFoundation), Windows (MSMF), Linux (V4L2)
  - 234k+ downloads, активно поддерживается
  - wgpu integration для Bevy
  - Threaded mode для real-time

### Hand Tracking
- **mediapipe-rs (WasmEdge)** - Только WASM, не подходит для desktop
- **ort (ONNX Runtime)** - Подходит для desktop, поддерживает TFLite модели
- **Рекомендуемый подход**: ort + MediaPipe hand landmark TFLite модель
  - Скачать модель с MediaPipe
  - Запускать инференс через ort
  - 21 точка руки (x, y, z координаты)