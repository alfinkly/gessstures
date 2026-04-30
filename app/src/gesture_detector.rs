use bevy::prelude::*;

use hand_tracking_core::{Gesture, GestureClassifier, GestureConfig};
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
    pub delta_z: f32,
    pub hand_open_ratio: f32,
    pub swipe_direction: Option<f32>,
}

/// Emitted when a gesture is held for longer than the configured hold duration.
#[derive(Event, Debug, Clone)]
pub struct GestureHoldEvent {
    pub gesture: Gesture,
    pub held_duration: f32,
}

/// Tracks hold state for the current stable gesture.
#[derive(Resource)]
pub struct GestureHold {
    pub gesture: Gesture,
    pub frames_held: u32,
    pub target_frames: u32,
    pub triggered: bool,
}

impl Default for GestureHold {
    fn default() -> Self {
        Self {
            gesture: Gesture::Unknown,
            frames_held: 0,
            target_frames: 30,
            triggered: false,
        }
    }
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
    pub delta_z: f32,
    pub hand_open_ratio: f32,
    pub hand_detected: bool,
    pub no_hand_frames: u32,

    classifier: GestureClassifier,
    stable_gesture: Gesture,
    gesture_hold_counter: u32,
    cumulative_delta_x: f32,
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
            delta_z: 0.0,
            hand_open_ratio: 0.0,
            hand_detected: false,
            no_hand_frames: 0,
            classifier: GestureClassifier::new(GestureConfig::default()),
            stable_gesture: Gesture::Unknown,
            gesture_hold_counter: 0,
            cumulative_delta_x: 0.0,
        }
    }
}

pub fn gesture_display_name(g: Gesture) -> &'static str {
    match g {
        Gesture::OpenPalm => "Open Palm",
        Gesture::Fist => "Fist",
        Gesture::Pinch => "Pinch",
        Gesture::Point => "Point",
        Gesture::VSIGN => "V-Sign",
        Gesture::Movement => "Movement",
        Gesture::Unknown => "\u{2014}",
    }
}

pub struct GestureDetectorPlugin;

impl Plugin for GestureDetectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GestureState>()
            .init_resource::<GestureHold>()
            .add_event::<GestureEvent>()
            .add_event::<GestureHoldEvent>()
            .add_systems(Update, detect_gesture_system);
    }
}

fn detect_gesture_system(
    hand_landmarks: Res<HandLandmarkResource>,
    mut gesture_state: ResMut<GestureState>,
    mut gesture_hold: ResMut<GestureHold>,
    mut gesture_events: EventWriter<GestureEvent>,
    mut gesture_hold_events: EventWriter<GestureHoldEvent>,
) {
    let landmarks = {
        let guard = hand_landmarks.inner.lock().unwrap();
        guard.landmarks.clone()
    };

    if let Some(lm) = landmarks {
        let result = gesture_state.classifier.classify(&lm);

        let raw_cursor_x = (lm[5].x + lm[9].x + lm[13].x + lm[17].x) / 4.0;
        let raw_cursor_y = (lm[5].y + lm[9].y + lm[13].y + lm[17].y) / 4.0;

        if result.gesture != gesture_state.stable_gesture {
            gesture_state.cumulative_delta_x = 0.0;
        }
        gesture_state.cumulative_delta_x += result.delta_x;

        let swipe_threshold = gesture_state.classifier.config.swipe_threshold;
        let swipe_direction = if result.gesture == Gesture::Movement
            && gesture_state.cumulative_delta_x.abs() > swipe_threshold
        {
            Some(gesture_state.cumulative_delta_x.signum())
        } else {
            None
        };

        if result.gesture == gesture_state.stable_gesture {
            gesture_state.gesture_hold_counter += 1;
        } else {
            gesture_state.stable_gesture = result.gesture;
            gesture_state.gesture_hold_counter = 0;
        }

        if gesture_state.gesture_hold_counter >= 3 {
            gesture_state.current_gesture = gesture_state.stable_gesture;
        }

        if gesture_state.current_gesture != gesture_hold.gesture {
            gesture_hold.gesture = gesture_state.current_gesture;
            gesture_hold.frames_held = 0;
            gesture_hold.triggered = false;
        }
        gesture_hold.frames_held += 1;

        if gesture_hold.frames_held >= gesture_hold.target_frames && !gesture_hold.triggered {
            gesture_hold.triggered = true;
            gesture_hold_events.send(GestureHoldEvent {
                gesture: gesture_hold.gesture,
                held_duration: gesture_hold.frames_held as f32 / 60.0,
            });
        }

        gesture_state.hand_detected = true;
        gesture_state.no_hand_frames = 0;
        gesture_state.confidence = result.confidence;

        // For Point gesture, use index finger tip (landmark[8]) as cursor
        // instead of palm center for more precise targeting.
        if result.gesture == Gesture::Point {
            gesture_state.cursor_x = lm[8].x;
            gesture_state.cursor_y = lm[8].y;
        } else {
            gesture_state.cursor_x = result.cursor_x;
            gesture_state.cursor_y = result.cursor_y;
        }
        gesture_state.raw_cursor_x = raw_cursor_x;
        gesture_state.raw_cursor_y = raw_cursor_y;
        gesture_state.delta_x = result.delta_x;
        gesture_state.delta_y = result.delta_y;
        gesture_state.delta_z = result.delta_z;
        gesture_state.hand_open_ratio = result.hand_open_ratio;

        gesture_events.send(GestureEvent {
            gesture: gesture_state.current_gesture,
            confidence: gesture_state.confidence,
            cursor_x: gesture_state.cursor_x,
            cursor_y: gesture_state.cursor_y,
            delta_x: gesture_state.delta_x,
            delta_y: gesture_state.delta_y,
            delta_z: gesture_state.delta_z,
            hand_open_ratio: gesture_state.hand_open_ratio,
            swipe_direction,
        });
    } else {
        gesture_state.no_hand_frames += 1;

        gesture_state.classifier = GestureClassifier::new(GestureConfig::default());
        gesture_state.cumulative_delta_x = 0.0;

        gesture_hold.gesture = Gesture::Unknown;
        gesture_hold.frames_held = 0;
        gesture_hold.triggered = false;

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
                    delta_z: 0.0,
                    hand_open_ratio: 0.0,
                    swipe_direction: None,
                });
            }
        }
    }
}
