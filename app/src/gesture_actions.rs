use bevy::prelude::*;

use hand_tracking_core::Gesture;
use crate::gesture_detector::GestureEvent;

// ---------------------------------------------------------------------------
// Public event types
// ---------------------------------------------------------------------------

/// Discrete graph-level actions derived from gesture transitions.
#[derive(Event, Debug, Clone)]
pub enum GraphAction {
    /// Continuous navigation — always fires while a hand is detected.
    Navigate {
        delta_x: f32,
        delta_y: f32,
        cursor_x: f32,
        cursor_y: f32,
    },
    /// Pan the camera (open palm with lateral movement).
    Pan {
        delta_x: f32,
        delta_y: f32,
    },
    /// Orbit the camera (fist with movement).
    Orbit {
        delta_yaw: f32,
        delta_pitch: f32,
    },
    /// Zoom the camera (open palm with z-depth change).
    Zoom {
        amount: f32,
    },
    /// Enter vertex-pinning mode (pinch → hold).
    PinVertex,
    /// Read the content of the pinned vertex (fist).
    ReadContent,
    /// Return from vertex-pinning back to the graph view (open palm).
    ExitPin,
}

/// Bevy event wrapping a single graph action.
#[derive(Event, Debug, Clone)]
pub struct GraphActionEvent {
    pub action: GraphAction,
}

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

/// Tracks the previous gesture and a cooldown timer for discrete actions.
#[derive(Resource)]
struct GestureActionState {
    prev_gesture: Gesture,
    last_action_time: std::time::Instant,
    cooldown: std::time::Duration,
}

impl Default for GestureActionState {
    fn default() -> Self {
        Self {
            prev_gesture: Gesture::Unknown,
            // Start with an already-expired cooldown so the very first
            // transition is not accidentally blocked.
            last_action_time: std::time::Instant::now()
                - std::time::Duration::from_secs(10),
            cooldown: std::time::Duration::from_millis(500),
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct GestureActionPlugin;

impl Plugin for GestureActionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GraphActionEvent>()
            .init_resource::<GestureActionState>()
            .add_systems(Update, process_gesture_actions);
    }
}

// ---------------------------------------------------------------------------
// Core system
// ---------------------------------------------------------------------------

fn process_gesture_actions(
    mut gesture_events: EventReader<GestureEvent>,
    mut graph_action_events: EventWriter<GraphActionEvent>,
    mut state: ResMut<GestureActionState>,
) {
    let cooldown = state.cooldown;

    // Zoom sensitivity factor for z-depth changes
    const ZOOM_FACTOR: f32 = 10.0;

    for event in gesture_events.read() {
        // --- Continuous action (always fires, no cooldown) ---
        match event.gesture {
            Gesture::OpenPalm => {
                let has_movement = event.delta_x != 0.0 || event.delta_y != 0.0;
                let has_zoom = event.delta_z != 0.0;

                if has_movement {
                    graph_action_events.send(GraphActionEvent {
                        action: GraphAction::Pan {
                            delta_x: event.delta_x,
                            delta_y: event.delta_y,
                        },
                    });
                }
                if has_zoom {
                    graph_action_events.send(GraphActionEvent {
                        action: GraphAction::Zoom {
                            amount: event.delta_z * ZOOM_FACTOR,
                        },
                    });
                }
            }
            Gesture::Fist => {
                graph_action_events.send(GraphActionEvent {
                    action: GraphAction::Orbit {
                        delta_yaw: event.delta_x * 3.0,
                        delta_pitch: -event.delta_y * 3.0,
                    },
                });
            }
            _ => {
                // Fallback: Navigate for backward compat
                graph_action_events.send(GraphActionEvent {
                    action: GraphAction::Navigate {
                        delta_x: event.delta_x,
                        delta_y: event.delta_y,
                        cursor_x: event.cursor_x,
                        cursor_y: event.cursor_y,
                    },
                });
            }
        }

        // --- Discrete actions: fire only on gesture TRANSITION ---
        let now = std::time::Instant::now();
        if now.duration_since(state.last_action_time) < cooldown {
            // Still in cooldown — skip discrete-action processing for this
            // event, but still allow the next iteration to check.
            continue;
        }

        if event.gesture != state.prev_gesture {
            match event.gesture {
                Gesture::Pinch => {
                    graph_action_events.send(GraphActionEvent {
                        action: GraphAction::PinVertex,
                    });
                    state.last_action_time = now;
                }
                Gesture::Fist => {
                    graph_action_events.send(GraphActionEvent {
                        action: GraphAction::ReadContent,
                    });
                    state.last_action_time = now;
                }
                Gesture::OpenPalm => {
                    graph_action_events.send(GraphActionEvent {
                        action: GraphAction::ExitPin,
                    });
                    state.last_action_time = now;
                }
                _ => {}
            }
        }

        state.prev_gesture = event.gesture;
    }
}
