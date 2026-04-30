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

    // Render camera feed as a full-screen UI Image node.
    // Bevy's UI camera renders this behind the gizmo Camera2d (order:1).
    commands.spawn((
        ImageNode {
            image: texture_handle,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        PreviewVideoRoot,
    ));
}

fn update_preview_texture(
    camera_res: Option<Res<CameraResource>>,
    mut images: ResMut<Assets<Image>>,
    query: Query<&ImageNode, With<PreviewVideoRoot>>,
) {
    let Some(camera_res) = camera_res else { return };

    let frame_data = {
        let guard = camera_res.frame.lock().unwrap();
        guard.as_ref().map(|f| (f.data.clone(), f.width, f.height))
    };

    let Ok(image_node) = query.get_single() else { return };

    if let Some((data, w, h)) = frame_data {
        if let Some(image) = images.get_mut(&image_node.image) {
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
    }
}
