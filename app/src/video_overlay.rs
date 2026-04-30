use bevy::asset::RenderAssetUsages;
use bevy::math::primitives::Rectangle;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::camera_capture::CameraResource;

#[derive(Component)]
pub struct VideoBackground;

#[derive(Component)]
pub struct VideoTexture(pub Handle<Image>);

pub struct VideoOverlayPlugin;

impl Plugin for VideoOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_video_background)
            .add_systems(Update, update_video_texture);
    }
}

fn setup_video_background(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
        RenderAssetUsages::MAIN_WORLD,
    );
    let texture_handle = images.add(image);

    let mesh = Mesh::from(Rectangle::from_size(Vec2::new(100.0, 75.0)));
    let mesh_handle = meshes.add(mesh);

    let material = StandardMaterial {
        base_color_texture: Some(texture_handle.clone()),
        base_color: Color::WHITE,
        unlit: true,
        ..default()
    };
    let material_handle = materials.add(material);

    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        Transform::from_xyz(0.0, 0.0, -50.0),
        VideoBackground,
        VideoTexture(texture_handle),
    ));
}

fn update_video_texture(
    camera_res: Option<Res<CameraResource>>,
    mut images: ResMut<Assets<Image>>,
    query: Query<&VideoTexture, With<VideoBackground>>,
) {
    let Some(camera_res) = camera_res else { return };
    let frame_data = {
        let guard = camera_res.frame.lock().unwrap();
        guard.as_ref().map(|f| (f.data.clone(), f.width, f.height))
    };
    let Some((data, w, h)) = frame_data else { return };
    let Ok(handle) = query.get_single() else { return };
    let Some(image) = images.get_mut(&handle.0) else { return };

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
