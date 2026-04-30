# Decisions — System Commands + Polish (T19-T26)

- **Swipe placement**: Swipe handling is placed BEFORE the cooldown check because swipe events fire every frame during Movement gesture, and the cooldown is for gesture-transition actions. Swipe dedup uses its own `last_swipe_direction` tracking instead.
- **VSIGN transitions**: FilterMode fires on transition TO VSIGN; ExitFilterMode fires on transition FROM VSIGN. Both go through the cooldown mechanism (protected by `last_action_time`), which prevents rapid fire.
- **OpenPalm hold → Reset**: Uses the existing `GestureHoldEvent` mechanism — fires once when held duration reaches target. No cooldown needed since hold events are inherently one-shot.
- **Hold progress indicator**: Implemented as a separate system in `graph_navigation.rs` rather than a new module. Reuses `map_hand_to_3d` from `cursor_mapper.rs` for world-space positioning. Uses `Gizmos::linestrip` with manually computed arc points for camera-facing rotation control.
- **Stub handlers**: Undo, FilterMode, ExitFilterMode are logged via `info!()` with clear placeholder comments marking them as future work.
