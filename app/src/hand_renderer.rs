use bevy::prelude::*;

use crate::gesture_detector::{gesture_display_name, GestureState};
use crate::hand_tracking::HandLandmarkResource;

/// Skeleton edges connecting 21 hand landmarks.
/// Index mapping: 0=wrist, 1-4=thumb, 5-8=index, 9-12=middle,
/// 13-16=ring, 17-20=pinky.
const SKELETON_CONNECTIONS: [(usize, usize); 23] = [
    (0, 1), (1, 2), (2, 3), (3, 4),   // Thumb
    (0, 5), (5, 6), (6, 7), (7, 8),   // Index
    (0, 9), (9, 10), (10, 11), (11, 12), // Middle
    (0, 13), (13, 14), (14, 15), (15, 16), // Ring
    (0, 17), (17, 18), (18, 19), (19, 20), // Pinky
    (5, 9), (9, 13), (13, 17),          // Palmar arch
];

fn finger_color(index: usize) -> Color {
    match index {
        0 => Color::WHITE,
        1..=4 => Color::srgb(1.0, 0.5, 0.0),   // thumb: orange
        5..=8 => Color::srgb(1.0, 0.0, 0.0),   // index: red
        9..=12 => Color::srgb(0.0, 1.0, 0.0),  // middle: green
        13..=16 => Color::srgb(0.0, 0.0, 1.0), // ring: blue
        17..=20 => Color::srgb(1.0, 1.0, 0.0), // pinky: yellow
        _ => Color::WHITE,
    }
}

fn connection_color(a: usize, b: usize) -> Color {
    let idx = if a == 0 { b } else { a };
    finger_color(idx)
}

pub struct HandRendererPlugin;

impl Plugin for HandRendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_overlay)
            .add_systems(Update, (draw_skeleton, update_status_text));
    }
}

#[derive(Component)]
struct StatusLabel;

fn setup_overlay(
    mut commands: Commands,
    camera_q: Query<&Camera, With<Camera2d>>,
) {
    // Only create Camera2d if none exists (e.g. desktop mode).
    // In preview mode, PreviewVideoPlugin already provides one.
    if camera_q.is_empty() {
        commands.spawn((
            Camera2d,
            Camera {
                order: 1,
                clear_color: ClearColorConfig::None,
                ..default()
            },
        ));
    }

    commands.spawn((
        Text::new("Hand: \u{2014} | Gesture: \u{2014}"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
        StatusLabel,
    ));
}

fn draw_skeleton(
    hand_landmarks: Res<HandLandmarkResource>,
    mut gizmos: Gizmos,
    windows: Query<&Window>,
) {
    let Ok(data) = hand_landmarks.inner.lock() else { return };
    let Some(landmarks) = &data.landmarks else { return };

    let Ok(window) = windows.get_single() else { return };
    let w = window.width();
    let h = window.height();
    if w == 0.0 || h == 0.0 {
        return;
    }

    let to_world = |x: f32, y: f32| -> Vec2 {
        Vec2::new(x * w - w * 0.5, h * 0.5 - y * h)
    };

    for &(a, b) in &SKELETON_CONNECTIONS {
        if a >= landmarks.len() || b >= landmarks.len() {
            continue;
        }
        let la = &landmarks[a];
        let lb = &landmarks[b];
        gizmos.line_2d(
            to_world(la.x, la.y),
            to_world(lb.x, lb.y),
            connection_color(a, b),
        );
    }

    for (i, lm) in landmarks.iter().enumerate() {
        let color = finger_color(i);
        let pos = to_world(lm.x, lm.y);
        gizmos.circle_2d(pos, 6.0, color);
        gizmos.circle_2d(pos, 3.0, color.with_alpha(0.7));
    }
}

fn update_status_text(
    hand_landmarks: Res<HandLandmarkResource>,
    gesture_state: Option<Res<GestureState>>,
    mut query: Query<&mut Text, With<StatusLabel>>,
) {
    let Ok(data) = hand_landmarks.inner.lock() else { return };
    let Ok(mut text) = query.get_single_mut() else { return };

    let gesture_name = gesture_state
        .map(|gs| gesture_display_name(gs.current_gesture))
        .unwrap_or("\u{2014}");
    let status = if data.landmarks.is_some() {
        "detected"
    } else {
        "\u{2014}"
    };
    text.0 = format!("Hand: {} | Gesture: {}", status, gesture_name);
}
