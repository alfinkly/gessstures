# Gessstures

3D-граф знаний с управлением жестами — визуализация Markdown-заметок в пространстве с AI-связыванием и нативным hand tracking.

## Требования

- **Rust** 1.80+ (стабильный)
- Для desktop-режима: системные зависимости для `bevy` + `winit` (см. [Bevy Setup](https://bevyengine.org/learn/quick-start/getting-started/setup/))
- Для AI-пайплайна: Python-сервер (опционально, реализация в будущем)

### macOS
```sh
xcode-select --install  # если ещё не установлены developer tools
```

### Linux (Ubuntu/Debian)
```sh
sudo apt install pkg-config libx11-dev libasound2-dev libudev-dev
```

### Windows
Установите [Build Tools for Visual Studio](https://visualstudio.microsoft.com/downloads/) с компонентом "Desktop development with C++".

## Сборка и запуск

```sh
# Проверка компиляции всех крейтов
cargo check

# Desktop-режим (полноценное 3D-приложение)
cargo run -p app -- desktop -n ./path/to/notes

# Текстовый запрос к графу (CLI)
cargo run -p app -- query "какие технологии используются?" -n ./path/to/notes

# WebSocket-сервер (для браузерного расширения, в разработке)
cargo run -p app -- web -p 3030

# Голосовое управление (в разработке)
cargo run -p app -- voice
```

## Управление в 3D

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

## Структура проекта

```
gessstures/
├── graph-core/     # Граф (petgraph), события Bevy, NodeData/EdgeData
├── ai-pipeline/    # Трейты EmbeddingProvider/LlmReasoner, HTTP-заглушка
├── data-sources/   # Трейт DataSource, MarkdownDataSource
├── input-sources/  # Трейт InputSource, TextQuerySource
├── app/            # CLI (clap), Bevy-приложение, рендерер, физика, камера, ввод
└── docs/
    └── adr/        # Architecture Decision Records (001–008)
```

## Дорожная карта

- [x] ADR (001–008)
- [x] Cargo workspace (5 крейтов)
- [x] Базовые структуры и события (`graph-core`)
- [x] HTTP-заглушка AI-пайплайна (`ai-pipeline`)
- [x] Чтение Markdown-заметок (`data-sources`)
- [x] Текстовый ввод (`input-sources`)
- [x] CLI с подкомандами (`app`)
- [x] Сборка графа: H1-заголовки, `[[wikilinks]]`, `[md](links)` парсятся, рёбра только по реальным ссылкам
- [x] Рендеринг 3D-сфер (размер ∝ degree) + линий-рёбер
- [x] 3D-лейблы (названия заметок проецируются на экран над сферами)
- [x] Force-directed физика (spring rest=8, отталкивание, centering, граница)
- [x] Orbit-камера (ПКМ/СКМ/колёсико, Space/F)
- [x] Клавиатурный ввод текстовых запросов (P — пауза физики)
- [ ] Python-сервер для AI (FastAPI + sentence-transformers)
- [ ] Hand tracking через `mediapipe-rs`
- [ ] Жестовое управление камерой (pinch-zoom, pan, orbit)

## Архитектура

Все межкомпонентные сообщения — через Bevy Events. Поток данных:

```
DataSource → NewContent → AI Pipeline → GraphUpdate → Renderer
InputSource → GraphQuery → AI Pipeline → GraphUpdate → Renderer
Hand Tracking → CameraCommand → Renderer
```

Подробнее — в [ADR](docs/adr/).
