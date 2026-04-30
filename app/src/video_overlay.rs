use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::camera_capture::CameraResource;

#[derive(Component)]
struct VideoImage;

pub struct VideoOverlayPlugin;

impl Plugin for VideoOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_video_background)
            .add_systems(Update, update_video_texture);
    }
}

fn setup_video_background(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let w = 640u32;
    let h = 480u32;

    let image = Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        vec![0u8; (w * h * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD,
    );
    let handle = images.add(image);

    commands.spawn((
        Camera2d,
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
    ));

    commands.spawn((
        ImageNode {
            image: handle.clone(),
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
        VideoImage,
    ));
}

fn update_video_texture(
    camera_res: Option<Res<CameraResource>>,
    mut images: ResMut<Assets<Image>>,
    query: Query<&ImageNode, With<VideoImage>>,
) {
    let Some(camera_res) = camera_res else { return };
    let frame_data = camera_res.frame.lock().unwrap().as_ref().map(|f| (f.data.clone(), f.width, f.height));
    let Some((data, w, h)) = frame_data else { return };
    let Ok(handle) = query.get_single() else { return };

    if let Some(image) = images.get_mut(&handle.image) {
        let expected = (w * h * 4) as usize;
        if image.data.len() == expected {
            image.data.copy_from_slice(&data);
        } else {
            *image = Image::new(
                Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                TextureDimension::D2,
                data,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD,
            );
        }
    }
}
