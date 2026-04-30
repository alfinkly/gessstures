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
    let data = vec![0u8; (width * height * 4) as usize];

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
            ..default()
        },
    ));

    let scale = if let Ok(window) = windows.get_single() {
        let vw = width as f32;
        let vh = height as f32;
        let ww = window.width();
        let wh = window.height();
        if ww > 0.0 && wh > 0.0 {
            (ww / vw).min(wh / vh)
        } else {
            1.0
        }
    } else {
        1.0
    };

    commands.spawn((
        Sprite {
            image: texture_handle,
            ..default()
        },
        Transform::from_scale(Vec3::splat(scale)),
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
    let Some((data, w, h)) = frame_data else { return };

    let Ok((mut transform, sprite)) = query.get_single_mut() else { return };

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
        let vw = w as f32;
        let vh = h as f32;
        let ww = window.width();
        let wh = window.height();
        if ww > 0.0 && wh > 0.0 {
            let scale = (ww / vw).min(wh / vh);
            transform.scale = Vec3::splat(scale);
        }
    }
}
