# F2 Code Quality Review - Learnings

## Session: 2026-04-30

### Process
- Ran `cargo check -p app` and `cargo test -p app` — both pass (10 warnings, 7/7 tests)
- Reviewed 6 files: sidecar.rs, hand_tracking.rs, main.rs, preview.rs, preview_video.rs, mediapipe_hands.py

### Key Findings
1. **Mutex poisoning** is the #1 risk — 4 `.lock().unwrap()` calls in hot paths that would panic on poisoned mutex
2. **Detached threads** in hand_tracking.rs and sidecar.rs — fire-and-forget spawning with no JoinHandle
3. **10 compiler warnings** — mostly dead code (unused constants, fields, unnecessary `mut`)
4. **Python/Rust schema mismatch** — `handedness` field sent by Python but silently dropped by serde
5. No AI slop, no resource leaks, no deadlocks found

### Patterns to Follow
- Sidecar restart-with-backoff pattern is solid
- Drop impl on SidecarResource correctly kills child + waits
- Serde default behavior ignores unknown fields (no `deny_unknown_fields`)
