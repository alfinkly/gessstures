# Architecture — Gessstures

## Layered Architecture (4 Levels)

```
┌────────────────────────────────────────────────────────────┐
│                    Interface Layer                          │
│  CLI, Bevy Plugins, Plugin Orchestration                   │
│  main.rs → plugin selection → system registration          │
├────────────────────────────────────────────────────────────┤
│                    App Layer                                │
│  Use cases, business logic, domain orchestration           │
│  HandTrackingService, GestureDetectionService              │
├────────────────────────────────────────────────────────────┤
│                    Domain Layer                             │
│  Pure types — zero framework dependencies                  │
│  HandLandmark, Gesture, HandOverlayConfig                  │
├────────────────────────────────────────────────────────────┤
│                 Infrastructure Layer                        │
│  External adapters: nokhwa (camera), Python sidecar (ML)   │
│  CaptureAdapter, MediaPipeSidecarAdapter                   │
└────────────────────────────────────────────────────────────┘
```

### Dependency Rule
**Outer layers depend on inner layers.** Never the reverse.
Domain knows nothing about Bevy, nokhwa, or Python.

### Crate Layout
- **hand-tracking-core/** — Domain: HandLandmark, Gesture, HandOverlayConfig, Config constants.
  Zero dependencies (Bevy is optional via "bevy" feature).
- **graph-core/** — Domain: GraphNode, GraphResource, events.
- **app/** — Interface + App + Infrastructure layers.

### File Convention: One Public Entity = One File
| Rule | Example |
|------|---------|
| One struct → one file | hand_landmark.rs → HandLandmark |
| One plugin → one file | skeleton_renderer_plugin.rs → SkeletonRendererPlugin |
| One adapter → one file | capture_adapter.rs → CaptureAdapter |

### Directory Structure
```
app/src/
├── main.rs              # CLI dispatch only (~30 lines)
├── interface/
│   ├── cli.rs           # Cli, Commands structs
│   └── plugins/
│       ├── preview_plugin.rs
│       ├── skeleton_renderer_plugin.rs
│       ├── hand_tracking_plugin.rs
│       └── ...
├── app/
│   └── *_service.rs     # Use cases / business logic
└── infrastructure/
    ├── camera/           # nokhwa capture, video overlay
    ├── ml/               # MediaPipe sidecar
    └── graph/            # Graph rendering adapters
```

### Naming Conventions
| Bevy Component | Suffix | Example |
|----------------|--------|---------|
| Plugin | Plugin | SkeletonRendererPlugin |
| Service | Service | HandTrackingService |
| Adapter | Adapter | CaptureAdapter |
| Config | Config | HandOverlayConfig |
