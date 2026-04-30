use bevy::prelude::*;
use crate::infrastructure::camera::capture_adapter::CameraCapturePlugin;
use crate::infrastructure::camera::overlay_adapter::PreviewVideoPlugin;
use crate::infrastructure::ml::mediapipe_sidecar::SidecarPlugin;
use crate::interface::plugins::skeleton_renderer_plugin::SkeletonRendererPlugin;
use hand_tracking_core::{HandLandmarkResource, HandOverlayConfig};

pub struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FpsCounter>()
           .init_resource::<HandOverlayConfig>()
           .add_systems(Startup, show_loading)
           .add_systems(Update, (check_exit, loading_fade, update_fps_counter));

        app.add_plugins((
            CameraCapturePlugin,
            PreviewVideoPlugin,
        ));

        app.init_resource::<HandLandmarkResource>()
           .add_plugins(SidecarPlugin);

        app.add_plugins(SkeletonRendererPlugin);
    }
}

#[derive(Component)]
struct LoadingText;

#[derive(Resource)]
struct FpsCounter {
    frame_count: u32,
    elapsed: f64,
    fps: u32,
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self { frame_count: 0, elapsed: 0.0, fps: 0 }
    }
}

#[derive(Component)]
struct FpsText;

fn show_loading(mut commands: Commands) {
    commands.spawn((
        Text::new("Loading MediaPipe..."),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            left: Val::Px(16.0),
            ..default()
        },
        LoadingText,
    ));

    commands.spawn((
        Text::new("FPS: --"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.0, 1.0, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            right: Val::Px(8.0),
            ..default()
        },
        FpsText,
    ));
}

fn loading_fade(
    hand_landmarks: Res<HandLandmarkResource>,
    mut query: Query<&mut Visibility, With<LoadingText>>,
) {
    if let Ok(data) = hand_landmarks.inner.lock() {
        if data.hand_count > 0 || data.timestamp.elapsed().as_secs() > 5 {
            if let Ok(mut vis) = query.get_single_mut() {
                *vis = Visibility::Hidden;
            }
        }
    }
}

fn update_fps_counter(
    time: Res<Time>,
    mut counter: ResMut<FpsCounter>,
    mut query: Query<&mut Text, With<FpsText>>,
) {
    counter.frame_count += 1;
    counter.elapsed += time.delta_secs_f64();

    if counter.elapsed >= 1.0 {
        counter.fps = (counter.frame_count as f64 / counter.elapsed).round() as u32;
        counter.frame_count = 0;
        counter.elapsed = 0.0;

        if let Ok(mut text) = query.get_single_mut() {
            text.0 = format!("FPS: {}", counter.fps);
        }
    }
}

fn check_exit(
    keys: Res<ButtonInput<KeyCode>>,
    mut exit: EventWriter<AppExit>,
) {
    if keys.pressed(KeyCode::Escape) || keys.pressed(KeyCode::KeyQ) {
        exit.send(AppExit::Success);
    }
}
