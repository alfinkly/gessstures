use bevy::prelude::*;

use hand_tracking_core::{Gesture, GestureClassifier};
use hand_tracking_core::HandLandmarkResource;

/// Per-frame event with the debounced gesture and cursor position.
#[derive(Event, Debug, Clone)]
pub struct GestureEvent {
    pub gesture: Gesture,
    pub confidence: f32,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub delta_x: f32,
    pub delta_y: f32,
}

/// Debounced gesture state, readable by any system (renderer, navigation).
#[derive(Resource)]
pub struct GestureState {
    pub current_gesture: Gesture,
    pub confidence: f32,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub raw_cursor_x: f32,
    pub raw_cursor_y: f32,
    pub delta_x: f32,
    pub delta_y: f32,
    pub hand_detected: bool,
    pub no_hand_frames: u32,

    classifier: GestureClassifier,
    stable_gesture: Gesture,
    gesture_hold_counter: u32,
}

impl Default for GestureState {
    fn default() -> Self {
        Self {
            current_gesture: Gesture::Unknown,
            confidence: 0.0,
            cursor_x: 0.5,
            cursor_y: 0.5,
            raw_cursor_x: 0.5,
            raw_cursor_y: 0.5,
            delta_x: 0.0,
            delta_y: 0.0,
            hand_detected: false,
            no_hand_frames: 0,
            classifier: GestureClassifier::new(),
            stable_gesture: Gesture::Unknown,
            gesture_hold_counter: 0,
        }
    }
}

pub fn gesture_display_name(g: Gesture) -> &'static str {
    match g {
        Gesture::OpenPalm => "Open Palm",
        Gesture::Fist => "Fist",
        Gesture::Pinch => "Pinch",
        Gesture::Movement => "Movement",
        Gesture::Unknown => "\u{2014}",
    }
}

pub struct GestureDetectorPlugin;

impl Plugin for GestureDetectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GestureState>()
            .add_event::<GestureEvent>()
            .add_systems(Update, detect_gesture_system);
    }
}

fn detect_gesture_system(
    hand_landmarks: Res<HandLandmarkResource>,
    mut gesture_state: ResMut<GestureState>,
    mut gesture_events: EventWriter<GestureEvent>,
) {
    let landmarks = {
        let guard = hand_landmarks.inner.lock().unwrap();
        guard.landmarks.clone()
    };

    if let Some(lm) = landmarks {
        let result = gesture_state.classifier.classify(&lm);

        let raw_cursor_x = (lm[5].x + lm[9].x + lm[13].x + lm[17].x) / 4.0;
        let raw_cursor_y = (lm[5].y + lm[9].y + lm[13].y + lm[17].y) / 4.0;

        if result.gesture == gesture_state.stable_gesture {
            gesture_state.gesture_hold_counter += 1;
        } else {
            gesture_state.stable_gesture = result.gesture;
            gesture_state.gesture_hold_counter = 0;
        }

        if gesture_state.gesture_hold_counter >= 3 {
            gesture_state.current_gesture = gesture_state.stable_gesture;
        }

        gesture_state.hand_detected = true;
        gesture_state.no_hand_frames = 0;
        gesture_state.confidence = result.confidence;
        gesture_state.cursor_x = result.cursor_x;
        gesture_state.cursor_y = result.cursor_y;
        gesture_state.raw_cursor_x = raw_cursor_x;
        gesture_state.raw_cursor_y = raw_cursor_y;
        gesture_state.delta_x = result.delta_x;
        gesture_state.delta_y = result.delta_y;

        gesture_events.send(GestureEvent {
            gesture: gesture_state.current_gesture,
            confidence: gesture_state.confidence,
            cursor_x: gesture_state.cursor_x,
            cursor_y: gesture_state.cursor_y,
            delta_x: gesture_state.delta_x,
            delta_y: gesture_state.delta_y,
        });
    } else {
        gesture_state.no_hand_frames += 1;

        gesture_state.classifier = GestureClassifier::new();

        if gesture_state.no_hand_frames > 5 {
            let was_detected = gesture_state.hand_detected;
            gesture_state.hand_detected = false;
            gesture_state.current_gesture = Gesture::Unknown;
            gesture_state.stable_gesture = Gesture::Unknown;
            gesture_state.gesture_hold_counter = 0;
            gesture_state.confidence = 0.0;

            if was_detected {
                gesture_events.send(GestureEvent {
                    gesture: Gesture::Unknown,
                    confidence: 0.0,
                    cursor_x: gesture_state.cursor_x,
                    cursor_y: gesture_state.cursor_y,
                    delta_x: 0.0,
                    delta_y: 0.0,
                });
            }
        }
    }
}
