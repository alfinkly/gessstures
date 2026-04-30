use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;

use crate::camera_capture::CameraResource;

pub use hand_tracking_core::{HandLandmark, HandLandmarkData, HandLandmarkResource};

#[derive(Resource)]
pub struct UseSidecar;

pub use hand_tracking_core::HandOverlayConfig;

pub struct HandTrackingPlugin;

impl Plugin for HandTrackingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HandLandmarkResource>()
            .init_resource::<HandOverlayConfig>()
            .add_systems(Startup, start_hand_tracking);
    }
}

fn start_hand_tracking(
    mut commands: Commands,
    camera_resource: Res<CameraResource>,
    sidecar_mode: Option<Res<UseSidecar>>,
) {
    if sidecar_mode.is_some() {
        info!("Hand tracking: sidecar mode active, skipping detection loop");
        return;
    }
    let landmark_resource = HandLandmarkResource::new();
    let shared = landmark_resource.inner.clone();
    commands.insert_resource(landmark_resource);

    let camera_frame = camera_resource.frame.clone();
    let is_active = camera_resource.is_active.clone();

    info!("Spawning hand-tracking inference thread ...");
    std::thread::spawn(move || {
        if let Err(e) = detection_loop(camera_frame, is_active, shared) {
            error!("Hand-tracking detection loop exited: {}", e);
        }
    });
}

fn detection_loop(
    camera_frame: Arc<Mutex<Option<crate::camera_capture::CameraFrame>>>,
    is_active: Arc<AtomicBool>,
    output: Arc<Mutex<HandLandmarkData>>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        if !is_active.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(100));
            continue;
        }

        let frame = {
            let mut guard = camera_frame.lock().unwrap();
            guard.as_ref().cloned()
        };

        if let Some(frame) = frame {
            let start = std::time::Instant::now();
            let data = detect_hand_cv(&frame);
            if let Ok(mut guard) = output.lock() {
                *guard = data;
            }
            let elapsed = start.elapsed();
            if elapsed < std::time::Duration::from_millis(33) {
                std::thread::sleep(std::time::Duration::from_millis(33) - elapsed);
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }
}

fn detect_hand_cv(frame: &crate::camera_capture::CameraFrame) -> HandLandmarkData {
    let img = match image::RgbaImage::from_raw(frame.width, frame.height, frame.data.clone()) {
        Some(i) => i,
        None => return HandLandmarkData::default(),
    };

    let w = img.width() as f32;
    let h = img.height() as f32;

    let mut skin_pixels: Vec<(f32, f32)> = Vec::new();

    for y in 0..img.height() {
        for x in 0..img.width() {
            let px = img.get_pixel(x, y);
            let r = px[0] as f32;
            let g = px[1] as f32;
            let b = px[2] as f32;

            let max_rgb = r.max(g).max(b);
            let min_rgb = r.min(g).min(b);
            if max_rgb < 40.0 || max_rgb == min_rgb {
                continue;
            }

            let c_max = max_rgb;
            let c_min = min_rgb;
            let delta = c_max - c_min;

            let saturation = if c_max == 0.0 { 0.0 } else { delta / c_max };
            let value = c_max / 255.0;

            if saturation > 0.15 && saturation < 0.68 && value > 0.15 && value < 0.95 {
                skin_pixels.push((x as f32, y as f32));
            }
        }
    }

    if skin_pixels.len() < 100 {
        return HandLandmarkData::default();
    }

    let cx = skin_pixels.iter().map(|p| p.0).sum::<f32>() / skin_pixels.len() as f32;
    let cy = skin_pixels.iter().map(|p| p.1).sum::<f32>() / skin_pixels.len() as f32;

    let (min_x, max_x) = skin_pixels.iter().fold(
        (w, 0.0_f32),
        |(mn, mx), p| (mn.min(p.0), mx.max(p.0)),
    );
    let (min_y, max_y) = skin_pixels.iter().fold(
        (h, 0.0_f32),
        |(mn, mx), p| (mn.min(p.1), mx.max(p.1)),
    );

    let area = (max_x - min_x) * (max_y - min_y);
    let bbox_area_ratio = skin_pixels.len() as f32 / area.max(1.0);
    let aspect = (max_x - min_x) / (max_y - min_y).max(1.0);

    let num_fingers;
    if bbox_area_ratio > 0.5 && aspect > 0.5 && aspect < 2.0 {
        num_fingers = 5;
    } else if aspect > 0.8 && aspect < 1.2 {
        num_fingers = 0;
    } else {
        num_fingers = skin_pixels.len().min(5) as u32;
    }

    let mut landmarks = Vec::with_capacity(21);

    landmarks.push(HandLandmark {
        x: cx / w,
        y: cy / h,
        z: 0.0,
    });

    let tip_offsets: [(f32, f32); 20] = [
        (0.0, -0.05),
        (0.0, -0.10),
        (0.0, -0.15),
        (0.0, -0.20),
        (0.05, -0.08),
        (0.08, -0.12),
        (0.10, -0.16),
        (0.12, -0.20),
        (0.02, -0.10),
        (0.02, -0.16),
        (0.02, -0.20),
        (0.02, -0.25),
        (-0.02, -0.10),
        (-0.02, -0.16),
        (-0.02, -0.20),
        (-0.02, -0.24),
        (-0.05, -0.06),
        (-0.06, -0.10),
        (-0.06, -0.14),
        (-0.06, -0.18),
    ];

    for (ox, oy) in &tip_offsets {
        landmarks.push(HandLandmark {
            x: (cx + ox * (max_x - min_x)) / w,
            y: (cy + oy * (max_y - min_y)) / h,
            z: 0.0,
        });
    }

    if num_fingers == 5 {
        let hand_size = (max_x - min_x).max(max_y - min_y);
        let spread = skin_pixels.iter().map(|p| {
            let dx = p.0 - cx;
            let dy = p.1 - cy;
            (dx * dx + dy * dy).sqrt()
        }).sum::<f32>() / skin_pixels.len() as f32;

        if spread / hand_size.max(1.0) < 0.15 {
            let thumb_tip = &landmarks[1];
            let index_tip = &landmarks[5];
            let dx = thumb_tip.x - index_tip.x;
            let dy = thumb_tip.y - index_tip.y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < 0.04 {
                return HandLandmarkData {
                    landmarks: Some(vec![
                        HandLandmark { x: cx / w, y: cy / h, z: 0.0 },
                    ]),
                    hand_count: 1,
                    timestamp: std::time::Instant::now(),
                };
            }
        }
    }

    HandLandmarkData {
        landmarks: Some(landmarks),
        hand_count: 1,
        timestamp: std::time::Instant::now(),
    }
}
