//! Video overlay plugin.
//!
//! Renders the camera feed as a textured 3D background plane behind the
//! graph scene. Reads [`CameraResource`] frames (RGBA), updates a Bevy
//! [`Image`] texture each frame without re-allocation, and applies it to
//! a large unlit quad positioned behind the graph nodes.

use bevy::asset::RenderAssetUsages;
use bevy::math::primitives::Rectangle;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::camera_capture::CameraResource;

/// Marker component for the video background mesh entity.
#[derive(Component)]
pub struct VideoBackground;

/// Holds the texture handle so the update system can find and mutate it.
#[derive(Component)]
pub struct VideoTexture(pub Handle<Image>);

/// Plugin that renders the camera feed as a full-screen 3D background.
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

    // Black placeholder texture until the first camera frame arrives.
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

    // Large quad positioned behind the graph.  The camera starts at
    // (0, 8, 20) looking at the origin, so z = -20 is well behind the
    // graph nodes (which sit within a ~5-unit sphere around the origin).
    let mesh = Mesh::from(Rectangle::from_size(Vec2::new(100.0, 75.0)));
    let mesh_handle = meshes.add(mesh);

    let material = StandardMaterial {
        base_color_texture: Some(texture_handle.clone()),
        base_color: Color::WHITE,
        unlit: true, // Show raw video, no lighting calculations
        ..default()
    };
    let material_handle = materials.add(material);

    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        Transform::from_xyz(0.0, 0.0, -20.0),
        VideoBackground,
        VideoTexture(texture_handle),
    ));
}

/// Copies the latest camera frame into the video background texture every
/// Bevy update tick.
///
/// Clones frame data instead of taking it so the hand-tracking background
/// thread can also consume frames without racing.
fn update_video_texture(
    camera_res: Option<Res<CameraResource>>,
    mut images: ResMut<Assets<Image>>,
    query: Query<&VideoTexture, With<VideoBackground>>,
) {
    let Some(camera_res) = camera_res else {
        return;
    };

    // Clone the frame data (don't `.take()` – hand_tracking needs it too).
    let frame_data = {
        let guard = camera_res.frame.lock().unwrap();
        guard.as_ref().map(|f| (f.data.clone(), f.width, f.height))
    };

    let Some((data, width, height)) = frame_data else {
        return;
    };

    let Ok(handle) = query.get_single() else {
        return;
    };

    let Some(image) = images.get_mut(&handle.0) else {
        return;
    };

    let expected = (width * height * 4) as usize;
    if image.data.len() == expected {
        // Fast path: same resolution, zero-copy-ish update.
        image.data.copy_from_slice(&data);
    } else {
        // Resolution changed – recreate the image asset.
        *image = Image::new(
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
    }
}
