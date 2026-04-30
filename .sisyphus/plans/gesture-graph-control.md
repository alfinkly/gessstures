# Gesture-to-Graph Control

## TL;DR
**Полная карта жестов** для управления 3D графом через MediaPipe hand tracking.

## Existing Pipeline
```
Sidecar → HandLandmarkResource → GestureDetector → Gesture → GestureActions → GraphAction → GraphNavigation → CameraCommand
```
Already: GestureClassifier (OpenPalm/Fist/Pinch/Movement), CameraCommand (Orbit/Pan/Zoom/Reset)
Need: Point, V-sign, Swipe; Pan/Orbit/Zoom nav; Point/Pinch selection; system commands.

## Gesture Map

| Mode | Gesture | Action | Details |
|------|---------|--------|---------|
| Nav | Open Palm + move XY | Pan | CameraCommand::Pan |
| Nav | Fist + rotate | Orbit | CameraCommand::Orbit |
| Nav | Open Palm + move Z | Zoom | CameraCommand::Zoom |
| Select | One finger (point) | Hover | nearest_vertex + highlight |
| Select | Point + finger curl | Tap/Click | open content |
| Select | Pinch on node | Grab + Drag | move node in 3D |
| System | Swipe left | Undo | GraphAction::Undo |
| System | V-sign (2 fingers) | Filter | GraphAction::Filter |
| System | Open palm hold 2s | Reset | CameraCommand::Reset |

## Waves

### Wave 1: Enhanced GestureClassifier
- [ ] 1. Add `Point`, `VSIGN` variants to `Gesture` enum
- [ ] 2. Implement `Point` detection (only index extended, others curled)
- [ ] 3. Implement `VSIGN` detection (index + middle extended)
- [ ] 4. Add hand open/close ratio heuristic (pan depth)
- [ ] 5. Add dead zone + configurable alpha to GestureClassifier
- [ ] 6. Add hold timer (`GestureHold` resource)
- [ ] 7. Add swipe detection (rapid delta accumulation)
- [ ] 8. Tests for all new gestures

### Wave 2: Navigation (Pan/Orbit/Zoom)
- [ ] 9. Map OpenPalm+XY → CameraCommand::Pan
- [ ] 10. Map Fist+XY → CameraCommand::Orbit (already partial)
- [ ] 11. Map OpenPalm Z-depth → CameraCommand::Zoom
- [ ] 12. Add gesture smoothing to CameraCommand pipeline
- [ ] 13. Mode indicator UI (text overlay showing current nav mode)

### Wave 3: Selection + Manipulation
- [ ] 14. Point gesture → laser cursor (line from index tip)
- [ ] 15. Nearest vertex on point → highlight (enhance VertexHighlightPlugin)
- [ ] 16. Point + hold → node selection + content view
- [ ] 17. Pinch on hovered node → grab + drag in 3D
- [ ] 18. Pinch in void → context menu (stub)

### Wave 4: System Commands
- [ ] 19. Swipe left → GraphAction::Undo (stub → event only)
- [ ] 20. V-sign → GraphAction::Filter (stub)
- [ ] 21. Open palm hold 2s → CameraCommand::Reset
- [ ] 22. Hold progress visual indicator (circular progress)

### Wave 5: Polish + QA
- [ ] 23. Gesture sensitivity tuning
- [ ] 24. Edge cases: fast movement, occlusion, 2 hands
- [ ] 25. FPS benchmark
- [ ] 26. Final integration test

---

## Extensions needed

### hand-tracking-core/src/gesture.rs
- Add `Point`, `VSIGN` to Gesture enum
- Add `GestureConfig` resource (alpha, dead_zone_xy, dead_zone_z, hold_frames, swipe_threshold)
- Add `GestureHold` resource (hold_frames, target_frames, gesture)
- Implement point detection, V-sign, swipe direction

### app/src/gesture_detector.rs
- Add Z-depth tracking to GestureState
- Add GestureHold handling
- Add dead zone filtering
- Point → update cursor to index tip (landmark[8]) instead of palm center

### app/src/gesture_actions.rs
- Add Undo, Filter to GraphAction
- Map hold → Reset, swipe → Undo, V → Filter

### app/src/graph_navigation.rs
- Implement Pan, Zoom handling (already stubbed)
- Implement node drag

### app/src/vertex_highlight.rs
- Enhanced: pulsing glow on hover, color change on grab
