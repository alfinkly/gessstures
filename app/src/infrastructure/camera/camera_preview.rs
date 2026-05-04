use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use crate::infrastructure::camera::capture_adapter::CameraResource;
use hand_tracking_core::config::{CAMERA_WIDTH, CAMERA_HEIGHT};

#[derive(Component)]
pub struct CameraBackground;

#[derive(Resource)]
pub struct CameraViewTexture(pub Handle<Image>);

pub struct CameraViewPlugin;

impl Plugin for CameraViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera_view)
            .add_systems(Update, update_camera_texture);
    }
}

fn setup_camera_view(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window>,
) {
    let width = CAMERA_WIDTH;
    let height = CAMERA_HEIGHT;
    let data = vec![128u8; (width * height * 4) as usize];

    let image = Image::new(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let texture_handle = images.add(image);
    commands.insert_resource(CameraViewTexture(texture_handle.clone()));

    commands.spawn((
        Camera2d,
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Default,
            ..default()
        },
    ));

    let (ww, wh) = windows
        .get_single()
        .map(|w| (w.width(), w.height()))
        .unwrap_or((1280.0, 720.0));

    let scale = Vec3::splat(if ww > 0.0 && wh > 0.0 {
        (ww / width as f32).min(wh / height as f32)
    } else {
        1.0
    });

    commands.spawn((
        Sprite {
            image: texture_handle,
            rect: None,
            custom_size: Some(Vec2::new(width as f32, height as f32)),
            ..default()
        },
        Transform::from_scale(scale).with_translation(Vec3::Z),
        Visibility::Visible,
        CameraBackground,
    ));
}

fn update_camera_texture(
    camera_res: Option<Res<CameraResource>>,
    mut images: ResMut<Assets<Image>>,
    mut query: Query<(&mut Transform, &Sprite), With<CameraBackground>>,
    windows: Query<&Window>,
) {
    let camera_res = match camera_res {
        Some(r) => r,
        None => return,
    };

    let frame_data = {
        let guard = camera_res.frame.lock().unwrap();
        guard.as_ref().map(|f| (f.data.clone(), f.width, f.height))
    };

    let Some((data, w, h)) = frame_data else { return };

    let Ok((mut transform, sprite)) = query.get_single_mut() else { return };

    let expected = (w * h * 4) as usize;
    if data.len() < expected { return; }

    let new_image = Image::new(
        Extent3d { width: fw, height: fh, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    if let Some(image) = images.get_mut(&sprite.image) {
        *image = new_image;
        static ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !ONCE.swap(true, std::sync::atomic::Ordering::Relaxed) {
            eprintln!("[CAM] First frame COPIED to texture");
        }
    } else {
        eprintln!("[CAM] Image handle not found in Assets!");
    }

    if let Ok(window) = windows.get_single() {
        let ww = window.width();
        let wh = window.height();
        if ww > 0.0 && wh > 0.0 {
            transform.scale = Vec3::splat(
                (ww / CAMERA_WIDTH as f32).min(wh / CAMERA_HEIGHT as f32),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_fit_landscape() {
        // 16:9 window with 4:3 camera → fit by height (letterbox)
        let ww = 1920.0;
        let wh = 1080.0;
        let scale = (ww / CAMERA_WIDTH as f32).min(wh / CAMERA_HEIGHT as f32);
        assert!((scale - 2.25).abs() < 0.01, "Expected letterbox scale for 16:9 window, got {scale}");
    }

    #[test]
    fn test_scale_fit_portrait() {
        // 9:16 portrait window → fit by width
        let ww = 600.0;
        let wh = 1066.0;
        let scale = (ww / CAMERA_WIDTH as f32).min(wh / CAMERA_HEIGHT as f32);
        assert!((scale - 0.9375).abs() < 0.01, "Expected fit scale for portrait window, got {scale}");
    }

    #[test]
    fn test_no_panic_empty_window() {
        let ww = 0.0;
        let wh = 0.0;
        let scale = if ww > 0.0 && wh > 0.0 {
            (ww / CAMERA_WIDTH as f32).min(wh / CAMERA_HEIGHT as f32)
        } else {
            1.0
        };
        assert_eq!(scale, 1.0, "Should default to 1 for empty window");
    }
}
