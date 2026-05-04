# Draft: Project Overview — Gessstures

## Что это за проект
3D-граф знаний с управлением жестами. Визуализирует Markdown-заметки в 3D-пространстве, связывает их через ссылки, позволяет управлять камерой жестами рук.

## Архитектура

### Cargo Workspace (6 крейтов)
1. **graph-core** — ядро графа: petgraph, Bevy Events, NodeData/EdgeData, GraphResource, InteractionState. Все остальные от него зависят.
2. **hand-tracking-core** — доменные типы для hand tracking: HandLandmark, Gesture, GestureClassifier. Zero Bevy-зависимости (Bevy optional).
3. **ai-pipeline** — трейты EmbeddingProvider + LlmReasoner, HTTP-заглушка для Python-сервера (reqwest).
4. **data-sources** — трейт DataSource + MarkdownDataSource (walkdir + pulldown-cmark).
5. **input-sources** — трейт InputSource + TextQuerySource.
6. **app** — CLI (clap), Bevy-приложение, плагины, рендерер, физика, камера, ввод. 

### Layered Architecture (hand-tracking-core)
- Interface Layer: CLI, Bevy Plugins
- App Layer: Use cases (HandTrackingService, GestureDetectionService)
- Domain Layer: Pure types (HandLandmark, Gesture — zero framework deps)
- Infrastructure Layer: External adapters (nokhwa camera, MediaPipe sidecar)

### Событийно-ориентированная (Bevy Events)
- NewContent — новые заметки из файлов
- GraphUpdate — обновление графа
- GraphChanged — сигнал, что граф изменился
- CameraCommand — управление камерой (Orbit/Pan/Zoom/Reset)
- GraphQuery — текстовый запрос к графу
- InteractionState — Resource: режим взаимодействия (GraphView / VertexPinned / VertexContent)

### Поток данных
DataSource → NewContent → AI Pipeline → GraphUpdate → Renderer
InputSource → GraphQuery → AI Pipeline → GraphUpdate → Renderer
Hand Tracking → CameraCommand → Renderer

## Ключевые технологии
- **Rust** 1.80+
- **Bevy** 0.15 — ECS, рендеринг, окно, камера, плагины, события
- **petgraph** — граф (StableGraph)
- **clap** — CLI (derive macros)
- **pulldown-cmark** — парсинг Markdown
- **walkdir** — обход файлов
- **nokhwa** — захват с веб-камеры
- **mediapipe-rs** (планируется) — hand tracking
- **image** — обработка изображений

## Текущее состояние
- ✅ ADR (001–008)
- ✅ Cargo workspace (6 крейтов)
- ✅ Базовые структуры и события
- ✅ Чтение Markdown, сборка графа (H1, wikilinks, md-links)
- ✅ 3D рендеринг (сферы ∝ degree + линии-рёбра + лейблы)
- ✅ Force-directed физика (spring rest=8, отталкивание, centering)
- ✅ Orbit-камера (ПКМ/СКМ/колёсико, Space/F)
- ✅ Hand tracking (захват камеры, детекция жестов: OpenPalm/Fist/Pinch/Point/V-sign/Movement)
- ✅ Классификатор жестов с тестами (>10 тестов)
- ❌ Python-сервер для AI (FastAPI)
- ❌ Жестовое управление камерой (pinch-zoom, pan, orbit) — частично

## Как запустить
cargo run -p app -- desktop -n ./path/to/notes
cargo run -p app -- preview -c 0  # preview mode with webcam + skeleton
cargo run -p app -- query "текст" -n ./path/to/notes
