use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::camera_capture::CameraResource;

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
    windows: Query<&Window>,
) {
    let width = 640u32;
    let height = 480u32;
    let data = vec![128u8; (width * height * 4) as usize];

    let image = Image::new(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let texture_handle = images.add(image);

    commands.spawn((
        Camera2d,
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Default,
            ..default()
        },
    ));
    info!("[DBG] preview_video: Camera2d spawned");

    let win_size = windows.get_single().ok();
    let (ww, wh) = win_size
        .map(|w| (w.width(), w.height()))
        .unwrap_or((1280.0, 720.0));
    info!("[DBG] preview_video: setup, window={}x{}", ww, wh);

    commands.spawn((
        Sprite {
            image: texture_handle,
            custom_size: Some(Vec2::new(width as f32, height as f32)),
            ..default()
        },
        Transform::from_scale(Vec3::new(
            if ww > 0.0 && wh > 0.0 { ww / width as f32 } else { 1.0 },
            if ww > 0.0 && wh > 0.0 { wh / height as f32 } else { 1.0 },
            1.0,
        )),
        PreviewVideoRoot,
    ));
    info!("[DBG] preview_video: Sprite spawned ({}{})", width, height);
}

fn update_preview_texture(
    camera_res: Option<Res<CameraResource>>,
    mut images: ResMut<Assets<Image>>,
    query: Query<&Sprite, With<PreviewVideoRoot>>,
    windows: Query<&Window>,
    mut q_sprite: Query<&mut Transform, (With<PreviewVideoRoot>, Without<Camera>)>,
) {
    let camera_res = match camera_res {
        Some(r) => r,
        None => {
            static mut COUNT: u32 = 0;
            unsafe { COUNT += 1; if COUNT % 60 == 0 { info!("[DBG] preview: No CameraResource yet"); } }
            return;
        }
    };

    let frame_data = {
        let guard = camera_res.frame.lock().unwrap();
        guard.as_ref().cloned()
    };

    let frame = match frame_data {
        Some(d) => d,
        None => {
            static mut COUNT: u32 = 0;
            unsafe { COUNT += 1; if COUNT % 60 == 0 { info!("[DBG] preview: CameraResource exists but frame is None"); } }
            return;
        }
    };

    let sprite = match query.get_single() {
        Ok(s) => s,
        Err(_) => {
            static mut COUNT: u32 = 0;
            unsafe { COUNT += 1; if COUNT % 60 == 0 { info!("[DBG] preview: No PreviewVideoRoot Sprite found"); } }
            return;
        }
    };

    if let Some(image) = images.get_mut(&sprite.image) {
        let expected = (frame.width * frame.height * 4) as usize;
        if image.data.len() == expected && frame.data.len() >= expected {
            image.data[..expected].copy_from_slice(&frame.data[..expected]);
            static mut COPY_COUNT: u32 = 0;
            unsafe { COPY_COUNT += 1; if COPY_COUNT == 1 {
                info!("[DBG] preview: First frame copied ({}x{})", frame.width, frame.height);
            }}
        } else if frame.data.len() >= expected {
            *image = Image::new(
                Extent3d { width: frame.width, height: frame.height, depth_or_array_layers: 1 },
                TextureDimension::D2,
                frame.data.clone(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
            info!("[DBG] preview: Replaced Image asset ({}x{})", frame.width, frame.height);
        }
    }

    if let Ok(mut transform) = q_sprite.get_single_mut() {
        if let Ok(window) = windows.get_single() {
            let ww = window.width();
            let wh = window.height();
            if ww > 0.0 && wh > 0.0 {
                transform.scale = Vec3::new(
                    ww / frame.width as f32,
                    wh / frame.height as f32,
                    1.0,
                );
            }
        }
    }
}
