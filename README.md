# Gessstures

3D-граф знаний с управлением жестами + People Tracking через WebSocket.

- **3D-граф**: визуализация Markdown-заметок в пространстве с AI-связыванием
- **Cafe**: веб-страница для отслеживания людей через камеру (body pose + face recognition)
- **ws-bridge**: Rust-сервер, который принимает кадры с камеры, прогоняет через MediaPipe и возвращает детекцию

## Требования

- **Rust** 1.80+ (стабильный)
- **Node.js** 18+ (для фронтенда)
- **Python 3.10+** (для MediaPipe-детекторов)
- **System libs**: см. [Bevy Setup](https://bevyengine.org/learn/quick-start/getting-started/setup/)

### macOS
```sh
xcode-select --install
```

### Linux (Ubuntu/Debian)
```sh
sudo apt install pkg-config libx11-dev libasound2-dev libudev-dev
```

### Windows
Установите [Build Tools for Visual Studio](https://visualstudio.microsoft.com/downloads/) с компонентом "Desktop development with C++".

## Установка

### 1. Модели MediaPipe
```sh
bash scripts/download_models.sh
```
Скачивает `pose_landmarker_full.task` и `hand_landmarker.task` в `models/`.

### 2. Python-зависимости (детекторы)
```sh
pip install mediapipe opencv-python numpy
# Для face detection (опционально):
pip install insightface
```

### 3. Фронтенд
```sh
cd frontend && npm install
```

### 4. Rust-зависимости
```sh
cargo check
```

## Запуск

**Терминал 1 — WebSocket-сервер:**
```sh
cargo run -p ws_bridge -- -n demo-notes
# или через Makefile:
make serv
```
Сервер слушает `ws://0.0.0.0:3030`. Принимает JPEG-кадры, возвращает body pose + face detection.

**Терминал 2 — Фронтенд (Cafe):**
```sh
cd frontend && npm run dev
# или через Makefile:
make front
```
Страница `/cafe` — камера + people tracking. Открой `http://localhost:3000/cafe`.

**Телефон как камера:** открой `/cam` с телефона в той же сети — кадры пойдут на сервер, скелет появится на `/cafe`.

## Структура проекта

```
gessstures/
├── app/                 # Bevy desktop-приложение (3D граф)
│   ├── mediapipe_body.py    # Body pose детектор (Python subprocess)
│   └── face_detect.py       # Face detection + embeddings (Python subprocess)
├── frontend/            # Next.js веб-клиент
│   └── app/cafe/page.tsx   # Страница people tracking
├── graph-core/          # Граф (petgraph), события Bevy
├── engine/              # Движок графа (snapshot, tick)
├── physics-core/        # Force-directed физика
├── ws-bridge/           # WebSocket-сервер (Rust)
│   └── src/
│       ├── server.rs        # WS handler, Detector (Python subprocess manager)
│       ├── face_tracker.rs  # Face matching + visit tracking
│       └── loader.rs        # Markdown loader
├── models/              # MediaPipe модели (.task)
├── scripts/
│   └── download_models.sh
└── docs/
    └── adr/             # Architecture Decision Records (001–008)
```

## Как это работает

```
Камера → WebSocket → ws-bridge → body_det (Python/MediaPipe) → person_count + keypoints → клиент
                               → face_det (Python/InsightFace) → embeddings → FaceTracker → people list → клиент
```

- **Body detection** (`mediapipe_body.py`): Pose Landmarker — 33 keypoints, bbox. Ответ приходит на каждый кадр.
- **Face detection** (`face_detect.py`, опционально): InsightFace — bbox, embedding, face JPEG. Запускается раз в секунду. Результаты матчатся через cosine similarity в `FaceTracker`.
- **People list** бродкастится всем клиентам раз в секунду — id, total_seen_secs, visit_count, face_jpeg_b64, список визитов.

## Cafe page (people tracking)

`/cafe` — дашборд с:
- Live camera + pose skeleton overlay
- Статистика: total people, currently in view, visits
- Список персон с историей визитов
- Timeline для каждой персоны

Данные хранятся в `localPeopleRef` на клиенте (история не очищается). При наличии FaceTracker на сервере — фото персоны подтягиваются из `face_jpeg_b64`.

## Управление в 3D (desktop)

| Действие | Клавиша/мышь |
|---|---|
| Вращение камеры | ПКМ + перетаскивание |
| Zoom | Колёсико мыши |
| Панорамирование | СКМ + перетаскивание |
| Сброс камеры | `Space` |
| Фокус на граф | `F` |
| Пауза физики | `P` |
| Ввод текстового запроса | Просто печатайте |
| Отправить запрос | `Enter` |
| Удалить символ | `Backspace` |

## Дорожная карта

- [x] ADR (001–008)
- [x] Cargo workspace (5 крейтов)
- [x] Базовые структуры и события (`graph-core`)
- [x] HTTP-заглушка AI-пайплайна (`ai-pipeline`)
- [x] Чтение Markdown-заметок (`data-sources`)
- [x] Текстовый ввод (`input-sources`)
- [x] CLI с подкомандами (`app`)
- [x] Сборка графа: H1-заголовки, `[[wikilinks]]`, `[md](links)`, рёбра по реальным ссылкам
- [x] Рендеринг 3D-сфер (размер ∝ degree) + линий-рёбер
- [x] 3D-лейблы (названия заметок проецируются на экран над сферами)
- [x] Force-directed физика (`physics-core`)
- [x] Orbit-камера (ПКМ/СКМ/колёсико, Space/F)
- [x] People tracking — body pose детекция через MediaPipe
- [x] WebSocket-сервер (`ws-bridge`) с body + face детекцией
- [x] Next.js фронтенд — cafe page с live camera + статистикой
- [x] Face tracker с матчингом по embedding'ам
- [x] Авто-рестарт Python-процессов при падении
- [ ] Hand tracking через `mediapipe-rs`
- [ ] Жестовое управление камерой (pinch-zoom, pan, orbit)
- [ ] Python-сервер для AI (FastAPI + sentence-transformers)

## Архитектура

Все межкомпонентные сообщения — через Bevy Events. Поток данных:

```
DataSource → NewContent → Engine → GraphUpdate → Renderer
InputSource → GraphQuery → Engine → GraphUpdate → Renderer
Camera Frame → ws-bridge → body_det/face_det → клиент
```

Подробнее — в [ADR](docs/adr/).
