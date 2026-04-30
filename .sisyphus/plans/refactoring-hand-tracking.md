# Refactoring: Clean Architecture for Hand Tracking + Camera Overlay

## TL;DR

> **Quick Summary**: Привести код hand tracking, camera overlay и gesture detection к Clean Architecture (4 уровня: Domain → App → Interface → Infrastructure). Разделить на workspace крейты. Декомпозировать файлы: одна публичная сущность = один файл.
>
> **Deliverables**:
> - `hand-tracking-core/` новый workspace crate (Domain: HandLandmark, Gesture, PipRect, Config)
> - `ARCHITECTURE.md` в корне проекта
> - `REFACTORING.md` с описанием принятых решений
> - Рефакторинг `app/src/` по слоям (app/, interface/, infrastructure/)
> - Слияние `preview_video` + `video_overlay` → `HandOverlayPlugin`
> - Переименование: PipRect → HandOverlayConfig, sidecar → mediapipe_sidecar
> - Извлечение хардкода в конфиг
>
> **Estimated Effort**: Large (15-20 tasks)
> **Parallel Execution**: YES — 4 waves

---

## Context

### Original Request
> "Напиши план рефакторинга, потому что много фигового кода который тяжело читать, вне конвенции чистого кода и чистой архитектуры"

### User Preferences (from interview)
| Preference | Choice |
|---|---|
| File decomposition | One public entity = one file |
| Plugin architecture | Bevy community convention |
| Code duplication | Extract shared to module |
| Architecture levels | Domain → App → Interface → Infrastructure |
| Crate structure | Workspace crates |
| Self-documenting | Both: rename AND docs |

### Current Codebase Issues

| Issue | Severity | File(s) | Lines |
|---|---|---|---|
| Mix of 6 public types in one file | 🔴 HIGH | `hand_tracking.rs` | 273 |
| 403 lines, 3 types mixed with test data | 🔴 HIGH | `gesture_classifier.rs` | 403 |
| Duplicate: preview_video vs video_overlay | 🔴 HIGH | `preview_video.rs`, `video_overlay.rs` | 115+91 |
| main.rs: CLI parsing + markdown loader + plugin wiring | 🔴 HIGH | `main.rs` | 306 |
| Undescriptive names: PipRect, sidecar | 🟡 MED | `hand_tracking.rs`, `sidecar.rs` | |
| Hardcoded 640/480/0.2/8.0 scattered everywhere | 🟡 MED | multiple files | |
| Duplicate: hand_tracking vs sidecar (both manage tracking) | 🟡 MED | `hand_tracking.rs`, `sidecar.rs` | |
| gesture_actions ambiguous naming | 🟢 LOW | `gesture_actions.rs` | |

---

## Target Architecture

```
gessstures/
├── hand-tracking-core/        # Domain layer (NEW)
│   ├── src/lib.rs             # re-exports all public types
│   ├── src/hand_landmark.rs   # HandLandmark, HandLandmarkData, HandLandmarkResource
│   ├── src/gesture.rs         # Gesture enum, GestureClassifier, GestureResult
│   ├── src/overlay_config.rs  # HandOverlayConfig (renamed from PipRect)
│   └── src/config.rs          # CAMERA_WIDTH, HEIGHT, FPS, PIP_WIDTH_RATIO, etc.
│
├── graph-core/                # Domain layer (unchanged)
│   ├── src/lib.rs
│   └── src/...
│
├── app/                       # Interface + App + Infrastructure layers
│   ├── src/
│   │   ├── main.rs            # ONLY: cli match + app.run()
│   │   │
│   │   ├── app/               # App Layer — use cases / services
│   │   │   ├── mod.rs
│   │   │   ├── hand_tracking_service.rs
│   │   │   ├── gesture_detection_service.rs
│   │   │   └── gesture_action_service.rs
│   │   │
│   │   ├── interface/         # Interface Layer — Bevy plugins + CLI
│   │   │   ├── mod.rs
│   │   │   ├── cli.rs
│   │   │   └── plugins/
│   │   │       ├── mod.rs
│   │   │       ├── desktop_plugin.rs
│   │   │       ├── preview_plugin.rs
│   │   │       ├── hand_overlay_plugin.rs      # merged preview_video + video_overlay
│   │   │       ├── skeleton_renderer_plugin.rs  # renamed from hand_renderer
│   │   │       ├── gesture_detector_plugin.rs
│   │   │       ├── gesture_action_plugin.rs
│   │   │       └── graph_navigation_plugin.rs
│   │   │
│   │   └── infrastructure/   # Infrastructure Layer — external adapters
│   │       ├── mod.rs
│   │       ├── camera/
│   │       │   ├── mod.rs
│   │       │   ├── capture_adapter.rs
│   │       │   └── overlay_adapter.rs
│   │       ├── ml/
│   │       │   ├── mod.rs
│   │       │   └── mediapipe_sidecar.rs
│   │       └── graph/
│   │           ├── mod.rs
│   │           ├── builder_adapter.rs
│   │           ├── renderer_adapter.rs
│   │           ├── physics_adapter.rs
│   │           └── labels_adapter.rs
│   │
│   └── Cargo.toml             # depends on hand-tracking-core + graph-core + bevy
│
└── Cargo.toml                 # workspace: 6 members (adds hand-tracking-core)
```

### Layer Dependency Rules

```
interface/  →  app/  →  domain (hand-tracking-core)
    ↓                    ↑
    └── infrastructure/ ┘      (infra implements domain traits)
```

- **Domain** (`hand-tracking-core`): NO dependencies (pure Rust)
- **App** (`app/src/app/`): depends ONLY on `hand-tracking-core`
- **Interface** (`app/src/interface/`): depends on `app/` + `infrastructure/` + bevy
- **Infrastructure** (`app/src/infrastructure/`): depends on `hand-tracking-core` + external crates (nokhwa, serde)

---

## Wave 1: hand-tracking-core Crate (Domain)

> Create the domain crate with ZERO dependencies. Move pure types out of `app/`.

### Tasks

- [x] 1. **Create `hand-tracking-core/` workspace member**

  **What to do**:
  - `cargo new hand-tracking-core --lib` in workspace
  - Add to workspace `Cargo.toml`: `members.push("hand-tracking-core")`
  - `Cargo.toml` must have `edition = "2021"` and NO dependencies
  - Add `version.workspace = true`, `edition.workspace = true`

  **Files**: `hand-tracking-core/Cargo.toml`, `Cargo.toml` (workspace root)

- [x] 2. **Extract `HandLandmark`, `HandLandmarkData`, `HandLandmarkResource`**
- [x] 3. **Extract `Gesture` enum and `GestureClassifier`**
- [x] 4. **Extract `HandOverlayConfig` (rename from `PipRect`)**
- [x] 5. **Extract config constants**

  **What to do**:
  - Create `hand-tracking-core/src/config.rs`
  - Move: `CAPTURE_WIDTH = 640`, `CAPTURE_HEIGHT = 480`, `CAPTURE_FPS = 30`
  - Add: `PIP_WIDTH_RATIO = 0.2`, `PIP_MARGIN = 8.0`
  - Add: `VIDEO_ASPECT = 4.0 / 3.0`
  - All `pub const` — imported by infrastructure code

- [x] 6. **Wire up in `app/Cargo.toml` and update imports**

  **What to do**:
  - Replace local type definitions with `use hand_tracking_core::*`
  - Remove old type definitions from `app/src/hand_tracking.rs`
  - Verify `cargo check -p app` passes
  - Verify `cargo test -p app` passes (GestureClassifier tests move to hand-tracking-core)

---

## Wave 2: Restructure app/src by Layers

> Split files into app/, interface/, infrastructure/ directories.

### Tasks

- [ ] 7. **Extract `cli.rs` from `main.rs`**

  **What to do**:
  - Create `app/src/interface/cli.rs`
  - Move `Cli`, `Commands` structs and `clap` derive
  - Move `Parse` impl (CLI parsing only)
  - `main.rs` becomes ~30 lines: `Cli::parse()` → match → app.run()

- [ ] 8. **Create `app/` layer with service modules**

  **What to do**:
  - Create `app/src/app/mod.rs`
  - Create `app/src/app/hand_tracking_service.rs`
  - Create `app/src/app/gesture_detection_service.rs`
  - Create `app/src/app/gesture_action_service.rs`
  - Extract gesture detection logic from `gesture_detector.rs`
  - Extract gesture action logic from `gesture_actions.rs`

- [ ] 9. **Rename + restructure plugins into `interface/plugins/`**

  **What to do**:
  - Create `app/src/interface/plugins/mod.rs`
  - Extract each Bevy Plugin into its own file:
    - `desktop_plugin.rs` — orchestrates ALL plugins for desktop mode
    - `preview_plugin.rs` — orchestrates ONLY preview plugins
    - `graph_navigation_plugin.rs` (from `graph_navigation.rs`)
  - Rename `gesture_detector.rs` → plugin file
  - Rename `gesture_actions.rs` → plugin file

- [ ] 10. **Create `infrastructure/camera/` module**

  **What to do**:
  - Create `app/src/infrastructure/camera/mod.rs`
  - Create `app/src/infrastructure/camera/capture_adapter.rs` — nokhwa wrapper
  - Create `app/src/infrastructure/camera/overlay_adapter.rs` — merged video rendering
  - Rename `camera_capture.rs` → capture_adapter
  - Rename `video_overlay.rs` → merged into overlay_adapter

- [ ] 11. **Create `infrastructure/ml/` module**

  **What to do**:
  - Create `app/src/infrastructure/ml/mod.rs`
  - Rename `sidecar.rs` → `mediapipe_sidecar.rs`
  - Rename structs: `SidecarPlugin` → `MediaPipeSidecarPlugin`, `SidecarResource` → `MediaPipeSidecarResource`

- [ ] 12. **Create `infrastructure/graph/` module**

  **What to do**:
  - Create `app/src/infrastructure/graph/mod.rs`
  - Move graph-related plugins:
    - `builder_adapter.rs` (from `graph_builder.rs`)
    - `renderer_adapter.rs` (from `renderer.rs`)
    - `physics_adapter.rs` (from `physics.rs`)
    - `labels_adapter.rs` (from `labels.rs`)

---

## Wave 3: Merge + Rename + Clean

> Eliminate duplication, fix names, extract config.

### Tasks

- [ ] 13. **Merge `preview_video` + `video_overlay` → `HandOverlayPlugin`**

  **What to do**:
  - New unified plugin: `HandOverlayPlugin`
  - In preview mode: renders as Sprite at top-right (20% width)
  - In desktop mode: renders as Sprite at top-right (20% width)
  - Remove separate `VideoOverlayPlugin` (was 3D quad)
  - Both modes use the SAME unified code path
  - Config via `HandOverlayConfig` resource

- [ ] 14. **Rename: `hand_renderer.rs` → `SkeletonRendererPlugin`**

  **What to do**:
  - Rename `app/src/interface/plugins/skeleton_renderer_plugin.rs`
  - Rename `HandRendererPlugin` → `SkeletonRendererPlugin`
  - Rename `StatusLabel` → `SkeletonStatusLabel`
  - No logic changes — just rename

- [ ] 15. **Rename: `hand_tracking.rs` → `HandTrackingPlugin` only**

  **What to do**:
  - After extracting domain types to `hand-tracking-core`, the remaining Bevy plugin code stays minimal
  - Rename to reflect single responsibility: `HandTrackingPlugin` only

- [ ] 16. **Extract all hardcoded constants → `hand-tracking-core/src/config.rs`**

  **What to do**:
  - Replace `640`, `480`, `0.2`, `8.0` in all infrastructure files with `use hand_tracking_core::config::*`
  - Remove local `CAPTURE_WIDTH`, `CAPTURE_HEIGHT`, `CAPTURE_FPS` from `camera_capture.rs`
  - Verify no magic numbers remain

- [ ] 17. **Write `ARCHITECTURE.md`**

  **What to do**:
  - Create `/ARCHITECTURE.md` at project root
  - Document: layer diagram, dependency rules, naming conventions, crate layout
  - Document: "one file = one public entity" rule
  - Template already drafted in this plan's Target Architecture section

- [ ] 18. **Write `REFACTORING.md`**

  **What to do**:
  - Create `/REFACTORING.md` at project root
  - Document why each refactoring was done
  - Document obsolete file names and their new locations
  - Migration notes for future developers

---

## Wave FINAL: Verification

- [ ] F1. **Plan Compliance Audit** — `oracle`
  - Verify ALL files follow the new architecture
  - Check no circular dependencies
  - Verify domain crate has zero non-Rust deps

- [ ] F2. **Build + Test** — `unspecified-high`
  - `cargo check -p hand-tracking-core` (no bevy!)
  - `cargo check -p app` (with bevy feature)
  - `cargo test` (all workspace)
  - `cargo build --release`

- [ ] F3. **Manual QA** — `unspecified-high`
  - `cargo run -p app -- preview` → PIP webcam + skeleton
  - `cargo run -p app -- desktop -n ./tests/notes` → 3D graph + PIP

---

## Commit Strategy

| Wave | Scope | Message |
|------|-------|---------|
| 1 | hand-tracking-core crate | `refactor(domain): extract hand-tracking-core crate with HandLandmark, Gesture, config` |
| 2 | app/src layer restructure | `refactor(app): restructure app/src by layers (app/, interface/, infrastructure/)` |
| 3 | Merge + rename + clean | `refactor(overlay): merge preview_video + video_overlay into HandOverlayPlugin` |
| 3 | Rename | `refactor(naming): rename PipRect→HandOverlayConfig, sidecar→mediapipe_sidecar` |
| 3 | Config extract | `refactor(config): extract hardcoded constants to hand-tracking-core/config.rs` |
| 3 | Architecture docs | `docs: add ARCHITECTURE.md and REFACTORING.md` |
| FINAL | Fixes from review | `fix: address review feedback` |

---

## Success Criteria

### Verification Commands
```bash
cargo check -p hand-tracking-core              # Expected: success, 0 deps
cargo check -p app                              # Expected: success
cargo test -p hand-tracking-core                # Expected: all pass
cargo test -p app                               # Expected: all pass
grep -r "640\|480\|0\.2" app/src/ | grep -v config.rs  # Expected: no matches
```

### Final Checklist
- [ ] `hand-tracking-core` crate compiles with ZERO dependencies
- [ ] No file exceeds 80 lines (except main.rs ≤40 lines)
- [ ] No duplicate camera/video rendering code
- [ ] PipRect renamed to HandOverlayConfig everywhere
- [ ] `ARCHITECTURE.md` present and accurate
- [ ] `REFACTORING.md` present
- [ ] `cargo test --workspace` passes
- [ ] Preview mode works: PIP webcam + skeleton
- [ ] Desktop mode works: 3D graph + PIP webcam + skeleton
