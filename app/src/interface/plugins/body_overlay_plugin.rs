use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use crate::infrastructure::camera::capture_adapter::CameraResource;
use crate::infrastructure::body::person_tracker::PersonTrackerResource;
use hand_tracking_core::config::{CAMERA_WIDTH, CAMERA_HEIGHT};

/// Marker component for the full-frame background sprite.
#[derive(Component)]
pub struct BodyOverlayRoot;

/// Marks a sprite as the tile for a person, storing index and screen rect for skeleton drawing.
#[derive(Component)]
pub struct PersonTile {
    pub index: usize,
    pub screen_rect: Rect,
}

/// Holds the shared texture handle used by both background and tile sprites.
#[derive(Resource)]
pub struct CameraTexture(pub Handle<Image>);

/// Screen-space layout info for a single tile.
pub struct TileInfo {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub struct BodyOverlayPlugin;

impl Plugin for BodyOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_body_overlay)
            .add_systems(Update, (update_fullscreen_texture, update_person_tiles));
    }
}

fn setup_body_overlay(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window>,
    camera_q: Query<&Camera, With<Camera2d>>,
) {
    // Ensure a Camera2d exists for sprite rendering (standalone mode without SkeletonRendererPlugin)
    if camera_q.is_empty() {
        commands.spawn((
            Camera2d,
            Camera {
                order: 1,
                clear_color: ClearColorConfig::None,
                ..default()
            },
        ));
    }

    let width = CAMERA_WIDTH;
    let height = CAMERA_HEIGHT;
    let data = vec![0u8; (width * height * 4) as usize];

    let image = Image::new(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let texture_handle = images.add(image);

    commands.insert_resource(CameraTexture(texture_handle.clone()));

    let (ww, wh) = windows
        .get_single()
        .map(|w| (w.width(), w.height()))
        .unwrap_or((1280.0, 720.0));
    let scale = fit_scale(ww, wh);

    commands.spawn((
        Sprite {
            image: texture_handle,
            rect: None,
            custom_size: Some(Vec2::new(width as f32, height as f32)),
            ..default()
        },
        Transform::from_scale(scale),
        GlobalTransform::default(),
        BodyOverlayRoot,
    ));
}

fn update_fullscreen_texture(
    camera_res: Option<Res<CameraResource>>,
    mut images: ResMut<Assets<Image>>,
    query: Query<&Sprite, With<BodyOverlayRoot>>,
    mut q_transform: Query<&mut Transform, (With<BodyOverlayRoot>, Without<Camera>)>,
    windows: Query<&Window>,
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
            if let Ok(mut transform) = q_transform.get_single_mut() {
                transform.scale = fit_scale(ww, wh);
            }
        }
    }
}

fn fit_scale(ww: f32, wh: f32) -> Vec3 {
    let img_aspect = CAMERA_WIDTH as f32 / CAMERA_HEIGHT as f32;
    let win_aspect = ww / wh;
    let scale = if win_aspect > img_aspect {
        wh / CAMERA_HEIGHT as f32
    } else {
        ww / CAMERA_WIDTH as f32
    };
    Vec3::splat(scale)
}

fn update_person_tiles(
    mut commands: Commands,
    person_tracker: Res<PersonTrackerResource>,
    camera_texture: Res<CameraTexture>,
    mut tile_query: Query<(Entity, &mut Sprite, &mut Transform, &mut PersonTile)>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.get_single() else { return };
    let ww = window.width();
    let wh = window.height();
    if ww == 0.0 || wh == 0.0 {
        return;
    }

    let count = person_tracker.person_count as usize;
    let layout = compute_tile_layout(count, ww, wh);
    let mut found = vec![false; layout.len()];
    let mut to_despawn = Vec::new();

    for (entity, mut sprite, mut transform, mut pt) in tile_query.iter_mut() {
        if pt.index >= layout.len() {
            to_despawn.push(entity);
            continue;
        }

        found[pt.index] = true;
        let info = &layout[pt.index];

        sprite.rect = person_tracker.persons.get(pt.index).map(|p| {
            let tex_w = CAMERA_WIDTH as f32;
            let tex_h = CAMERA_HEIGHT as f32;
            Rect {
                min: Vec2::new(
                    (p.bbox.cx - p.bbox.w / 2.0) * tex_w,
                    (p.bbox.cy - p.bbox.h / 2.0) * tex_h,
                ),
                max: Vec2::new(
                    (p.bbox.cx + p.bbox.w / 2.0) * tex_w,
                    (p.bbox.cy + p.bbox.h / 2.0) * tex_h,
                ),
            }
        });

        sprite.custom_size = Some(Vec2::new(info.w, info.h));

        let screen_cx = info.x + info.w / 2.0;
        let screen_cy = info.y + info.h / 2.0;
        transform.translation = Vec3::new(screen_cx - ww / 2.0, wh / 2.0 - screen_cy, 1.0);

        pt.screen_rect = Rect {
            min: Vec2::new(info.x, info.y),
            max: Vec2::new(info.x + info.w, info.y + info.h),
        };
    }

    for entity in to_despawn {
        commands.entity(entity).despawn();
    }

    for (i, info) in layout.iter().enumerate() {
        if found[i] {
            continue;
        }

        let rect = person_tracker.persons.get(i).map(|p| {
            let tex_w = CAMERA_WIDTH as f32;
            let tex_h = CAMERA_HEIGHT as f32;
            Rect {
                min: Vec2::new(
                    (p.bbox.cx - p.bbox.w / 2.0) * tex_w,
                    (p.bbox.cy - p.bbox.h / 2.0) * tex_h,
                ),
                max: Vec2::new(
                    (p.bbox.cx + p.bbox.w / 2.0) * tex_w,
                    (p.bbox.cy + p.bbox.h / 2.0) * tex_h,
                ),
            }
        });

        let screen_cx = info.x + info.w / 2.0;
        let screen_cy = info.y + info.h / 2.0;

        let screen_rect = Rect {
            min: Vec2::new(info.x, info.y),
            max: Vec2::new(info.x + info.w, info.y + info.h),
        };

        commands.spawn((
            Sprite {
                image: camera_texture.0.clone(),
                rect,
                custom_size: Some(Vec2::new(info.w, info.h)),
                ..default()
            },
            Transform::from_xyz(screen_cx - ww / 2.0, wh / 2.0 - screen_cy, 1.0),
            GlobalTransform::default(),
            PersonTile {
                index: i,
                screen_rect,
            },
        ));
    }
}

fn compute_tile_layout(person_count: usize, window_w: f32, window_h: f32) -> Vec<TileInfo> {
    match person_count {
        0 => vec![],
        1 => {
            let tile_h = window_h * 0.6;
            let tile_w = tile_h * 0.75;
            vec![TileInfo {
                x: (window_w - tile_w) / 2.0,
                y: (window_h - tile_h) / 2.0,
                w: tile_w,
                h: tile_h,
            }]
        }
        2 => {
            let tile_h = window_h * 0.45;
            let tile_w = window_w * 0.45;
            vec![
                TileInfo {
                    x: window_w * 0.03,
                    y: window_h - tile_h - 10.0,
                    w: tile_w,
                    h: tile_h,
                },
                TileInfo {
                    x: window_w * 0.52,
                    y: window_h - tile_h - 10.0,
                    w: tile_w,
                    h: tile_h,
                },
            ]
        }
        3 => {
            let th = window_h * 0.42;
            let tw = window_w * 0.42;
            vec![
                TileInfo {
                    x: window_w * 0.03,
                    y: window_h - 2.0 * th - 15.0,
                    w: tw,
                    h: th,
                },
                TileInfo {
                    x: window_w * 0.52,
                    y: window_h - 2.0 * th - 15.0,
                    w: tw,
                    h: th,
                },
                TileInfo {
                    x: (window_w - tw) / 2.0,
                    y: window_h - th - 10.0,
                    w: tw,
                    h: th,
                },
            ]
        }
        _ => {
            let th = window_h * 0.42;
            let tw = window_w * 0.42;
            vec![
                TileInfo {
                    x: window_w * 0.03,
                    y: window_h - 2.0 * th - 15.0,
                    w: tw,
                    h: th,
                },
                TileInfo {
                    x: window_w * 0.52,
                    y: window_h - 2.0 * th - 15.0,
                    w: tw,
                    h: th,
                },
                TileInfo {
                    x: window_w * 0.03,
                    y: window_h - th - 10.0,
                    w: tw,
                    h: th,
                },
                TileInfo {
                    x: window_w * 0.52,
                    y: window_h - th - 10.0,
                    w: tw,
                    h: th,
                },
            ]
        }
    }
}
