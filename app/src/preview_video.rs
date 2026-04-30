use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::camera_capture::CameraResource;
use crate::hand_tracking::PipRect;

#[derive(Component)]
pub struct PreviewVideoRoot;

pub struct PreviewVideoPlugin;

impl Plugin for PreviewVideoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_preview_video)
            .add_systems(Update, update_preview_texture);
    }
}

fn setup_preview_video(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let width = 640u32;
    let height = 480u32;
    let data = vec![0u8; (width * height * 4) as usize];

    let image = Image::new(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let texture_handle = images.add(image);

    commands.spawn((
        Sprite {
            image: texture_handle,
            custom_size: Some(Vec2::new(width as f32, height as f32)),
            ..default()
        },
        PreviewVideoRoot,
    ));
}

fn update_preview_texture(
    camera_res: Option<Res<CameraResource>>,
    mut images: ResMut<Assets<Image>>,
    query: Query<&Sprite, With<PreviewVideoRoot>>,
    mut q_transform: Query<&mut Transform, (With<PreviewVideoRoot>, Without<Camera>)>,
    windows: Query<&Window>,
    mut pip_rect: ResMut<PipRect>,
) {
    let camera_res = match camera_res {
        Some(r) => r,
        None => return,
    };

    let frame_data = {
        let guard = camera_res.frame.lock().unwrap();
        guard.as_ref().cloned()
    };

    let frame = match frame_data {
        Some(d) => d,
        None => return,
    };

    let Ok(sprite) = query.get_single() else { return };

    if let Some(image) = images.get_mut(&sprite.image) {
        let expected = (frame.width * frame.height * 4) as usize;
        if image.data.len() == expected && frame.data.len() >= expected {
            image.data[..expected].copy_from_slice(&frame.data[..expected]);
        } else if frame.data.len() >= expected {
            *image = Image::new(
                Extent3d { width: frame.width, height: frame.height, depth_or_array_layers: 1 },
                TextureDimension::D2,
                frame.data,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
        }
    }

    if let Ok(window) = windows.get_single() {
        let ww = window.width();
        let wh = window.height();
        if ww > 0.0 && wh > 0.0 {
            let pip_w = ww * 0.2;
            let pip_h = pip_w * (480.0 / 640.0);
            let margin = 8.0;

            pip_rect.enabled = true;
            pip_rect.left = ww - pip_w - margin;
            pip_rect.top = margin;
            pip_rect.width = pip_w;
            pip_rect.height = pip_h;

            if let Ok(mut transform) = q_transform.get_single_mut() {
                transform.translation = Vec3::new(
                    ww * 0.5 - margin - pip_w * 0.5,
                    wh * 0.5 - margin - pip_h * 0.5,
                    0.0,
                );
                transform.scale = Vec3::new(
                    pip_w / frame.width as f32,
                    pip_h / frame.height as f32,
                    1.0,
                );
            }
        }
    }
}
