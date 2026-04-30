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
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD,
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

    let win_size = windows.get_single().ok();
    let (ww, wh) = win_size
        .map(|w| (w.width(), w.height()))
        .unwrap_or((1280.0, 720.0));

    let scale = if ww > 0.0 && wh > 0.0 {
        (ww / width as f32).min(wh / height as f32)
    } else {
        1.0
    };

    commands.spawn((
        Sprite {
            image: texture_handle,
            custom_size: Some(Vec2::new(width as f32, height as f32)),
            ..default()
        },
        Transform::from_scale(Vec3::splat(scale)),
        Visibility::Visible,
        PreviewVideoRoot,
    ));
}

fn update_preview_texture(
    camera_res: Option<Res<CameraResource>>,
    mut images: ResMut<Assets<Image>>,
    mut query: Query<(&mut Transform, &mut Sprite), With<PreviewVideoRoot>>,
    windows: Query<&Window>,
) {
    let Some(camera_res) = camera_res else { return };

    let frame_data = {
        let guard = camera_res.frame.lock().unwrap();
        guard.as_ref().map(|f| (f.data.clone(), f.width, f.height))
    };

    let Ok((mut transform, sprite)) = query.get_single_mut() else { return };

    if let Some((data, w, h)) = frame_data {
        if let Some(image) = images.get_mut(&sprite.image) {
            let expected = (w * h * 4) as usize;
            if image.data.len() == expected {
                image.data[..expected].copy_from_slice(&data[..expected]);
            } else {
                *image = Image::new(
                    Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    data,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::MAIN_WORLD,
                );
            }
        }

        if let Ok(window) = windows.get_single() {
            let ww = window.width();
            let wh = window.height();
            if ww > 0.0 && wh > 0.0 {
                let scale = (ww / w as f32).min(wh / h as f32);
                transform.scale = Vec3::splat(scale);
            }
        }
    }
}
