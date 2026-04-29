use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use ndarray::Array4;
use ort::session::Session;
use ort::value::Tensor;
use ort::inputs;

use crate::camera_capture::CameraResource;

/// A single 2.5D hand landmark with normalized coordinates in [0, 1].
#[derive(Debug, Clone, Copy)]
pub struct HandLandmark {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Internal data produced by the inference thread and consumed by Bevy systems.
pub struct HandLandmarkData {
    /// `Some(landmarks)` when a hand is detected, `None` otherwise.
    pub landmarks: Option<Vec<HandLandmark>>,
    /// Number of hands currently detected (0 or 1 for this model).
    pub hand_count: u32,
    /// Timestamp of the latest inference result.
    pub timestamp: std::time::Instant,
}

impl Default for HandLandmarkData {
    fn default() -> Self {
        Self {
            landmarks: None,
            hand_count: 0,
            timestamp: std::time::Instant::now(),
        }
    }
}

/// Bevy resource holding the latest hand-landmark inference results.
///
/// Thread-safe: the background inference thread writes to `inner` via its
/// `Arc` clone, while Bevy systems read on the main thread.
#[derive(Resource)]
pub struct HandLandmarkResource {
    pub inner: Arc<Mutex<HandLandmarkData>>,
}

impl HandLandmarkResource {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HandLandmarkData::default())),
        }
    }
}

impl Default for HandLandmarkResource {
    fn default() -> Self {
        Self::new()
    }
}

/// Bevy plugin that starts hand-landmark inference on a background thread.
pub struct HandTrackingPlugin;

impl Plugin for HandTrackingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HandLandmarkResource>()
            .add_systems(Startup, start_hand_tracking);
    }
}

fn start_hand_tracking(
    mut commands: Commands,
    camera_resource: Option<Res<CameraResource>>,
) {
    let landmark_resource = HandLandmarkResource::new();
    let shared = landmark_resource.inner.clone();
    commands.insert_resource(landmark_resource);

    let Some(cam) = camera_resource else {
        warn!("HandTracking: CameraResource not available – inference disabled");
        return;
    };

    let camera_frame = cam.frame.clone();
    let is_active = cam.is_active.clone();

    info!("Spawning hand-tracking inference thread ...");
    std::thread::spawn(move || {
        if let Err(e) = inference_loop(camera_frame, is_active, shared) {
            error!("Hand-tracking inference loop exited: {}", e);
        }
    });
}

fn inference_loop(
    camera_frame: Arc<Mutex<Option<crate::camera_capture::CameraFrame>>>,
    is_active: Arc<AtomicBool>,
    output: Arc<Mutex<HandLandmarkData>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut model = Session::builder()?
        .commit_from_file("models/hand_landmark_full.tflite")?;
    info!("Hand-landmark model loaded successfully");

    std::thread::sleep(std::time::Duration::from_millis(500));

    loop {
        if !is_active.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(100));
            continue;
        }

        let frame = {
            let mut guard = camera_frame.lock().unwrap();
            guard.take()
        };

        if let Some(frame) = frame {
            let start = std::time::Instant::now();
            match process_frame(&mut model, &frame) {
                Ok(data) => {
                    if let Ok(mut guard) = output.lock() {
                        *guard = data;
                    }
                }
                Err(e) => warn!("Inference error: {}", e),
            }
            let elapsed = start.elapsed();
            if elapsed < std::time::Duration::from_millis(16) {
                std::thread::sleep(std::time::Duration::from_millis(16) - elapsed);
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }
}

fn process_frame(
    model: &mut Session,
    frame: &crate::camera_capture::CameraFrame,
) -> Result<HandLandmarkData, Box<dyn std::error::Error>> {
    let img = image::RgbaImage::from_raw(frame.width, frame.height, frame.data.clone())
        .ok_or("invalid frame dimensions for RgbaImage")?;

    let resized =
        image::imageops::resize(&img, 224, 224, image::imageops::FilterType::Triangle);

    let mut rgb_data = Vec::with_capacity(224 * 224 * 3);
    for pixel in resized.pixels() {
        rgb_data.push(pixel[0] as f32 / 255.0);
        rgb_data.push(pixel[1] as f32 / 255.0);
        rgb_data.push(pixel[2] as f32 / 255.0);
    }

    let array = Array4::from_shape_vec((1, 224, 224, 3), rgb_data)?;
    let tensor = Tensor::from_array(array)?;

    let outputs = model.run(inputs!["input" => tensor])?;

    let output_tensor = outputs[0].try_extract_array::<f32>()?;
    let flat: &[f32] = output_tensor
        .as_slice()
        .ok_or("ORT output tensor is not contiguous")?;

    let sum_abs: f32 = flat.iter().map(|v| v.abs()).sum();
    if sum_abs < 0.01 {
        return Ok(HandLandmarkData::default());
    }

    let mut landmarks = Vec::with_capacity(21);
    for i in 0..21 {
        let base = i * 3;
        landmarks.push(HandLandmark {
            x: flat[base],
            y: flat[base + 1],
            z: flat[base + 2],
        });
    }

    Ok(HandLandmarkData {
        landmarks: Some(landmarks),
        hand_count: 1,
        timestamp: std::time::Instant::now(),
    })
}
