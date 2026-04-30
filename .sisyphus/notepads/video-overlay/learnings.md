# video-overlay

## Implementation decisions

- Used 3D background plane (not UI) because UI always renders on top of 3D in Bevy.
- `Image::new()` in Bevy 0.15 takes 5 arguments: `Extent3d, TextureDimension, Vec<u8>, TextureFormat, RenderAssetUsages`.
- `RenderAssetUsages::MAIN_WORLD` is the correct 5th arg for runtime textures.
- Frame data is **cloned** (not `.take()`'d) from `CameraResource` to avoid racing with hand_tracking's background thread which also consumes frames.
- Positioned the quad at z=-20 (camera is at z=20 looking at origin) so it's behind all graph nodes.
- Used `unlit: true` on the material so video shows through without lighting calculations.

## File structure
- `app/src/video_overlay.rs` — new module with `VideoOverlayPlugin`
- `app/src/main.rs` — added `mod video_overlay` + plugin registration
