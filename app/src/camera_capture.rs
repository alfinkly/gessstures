/// Camera capture module using nokhwa.
///
/// Runs the camera in a background thread and exposes the latest frame
/// via the `CameraResource` Bevy resource. Handles disconnect/reconnect
/// gracefully by logging errors and retrying.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use nokhwa::utils::{
    CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType,
};
use nokhwa::Camera;

const CAPTURE_WIDTH: u32 = 640;
const CAPTURE_HEIGHT: u32 = 480;
const CAPTURE_FPS: u32 = 30;

/// A single frame captured from the camera.
///
/// `data` contains the pixel data. If the camera delivers MJPEG, the
/// capture thread decodes it to RGBA so downstream consumers always
/// get raw RGBA bytes (4 bytes per pixel, row-major).
pub struct CameraFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: FrameFormat,
    pub timestamp: std::time::Instant,
}

/// Bevy resource holding the latest captured frame.
///
/// Clone the `Arc` handles to share with a background capture thread.
#[derive(Resource)]
pub struct CameraResource {
    pub frame: Arc<Mutex<Option<CameraFrame>>>,
    pub is_active: Arc<AtomicBool>,
}

/// Plugin that initialises camera capture in a background thread on startup.
pub struct CameraCapturePlugin;

impl Plugin for CameraCapturePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_camera_capture);
    }
}

fn start_camera_capture(mut commands: Commands) {
    let frame: Arc<Mutex<Option<CameraFrame>>> = Arc::new(Mutex::new(None));
    let is_active = Arc::new(AtomicBool::new(false));

    commands.insert_resource(CameraResource {
        frame: frame.clone(),
        is_active: is_active.clone(),
    });

    info!("Spawning camera capture thread ...");
    std::thread::spawn(move || capture_loop(frame, is_active));
}

/// Main capture loop. Runs forever. Reconnects on camera errors.
fn capture_loop(shared_frame: Arc<Mutex<Option<CameraFrame>>>, is_active: Arc<AtomicBool>) {
    loop {
        match open_camera() {
            Ok(mut camera) => {
                info!("Camera opened successfully");
                is_active.store(true, Ordering::SeqCst);

                loop {
                    match camera.frame() {
                        Ok(frame) => {
                            let raw = frame.buffer().to_vec();
                            let resolution = frame.resolution();
                            let source_fmt = frame.source_frame_format();

                            let data = if source_fmt == FrameFormat::MJPEG {
                                match decode_jpeg(&raw) {
                                    Some(rgba) => rgba,
                                    None => {
                                        warn!("MJPEG decode failed, storing raw");
                                        raw
                                    }
                                }
                            } else {
                                raw
                            };

                            let new_frame = CameraFrame {
                                data,
                                width: resolution.width(),
                                height: resolution.height(),
                                format: source_fmt,
                                timestamp: std::time::Instant::now(),
                            };

                            if let Ok(mut guard) = shared_frame.lock() {
                                *guard = Some(new_frame);
                            }
                        }
                        Err(e) => {
                            error!("Camera frame error: {}", e);
                            is_active.store(false, Ordering::SeqCst);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to open camera: {}", e);
                is_active.store(false, Ordering::SeqCst);
            }
        }

        info!("Camera disconnected or unavailable – retrying in 2 s ...");
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}

fn open_camera() -> Result<Camera, nokhwa::NokhwaError> {
    let index = CameraIndex::Index(0);
    let cam_format =
        CameraFormat::new_from(CAPTURE_WIDTH, CAPTURE_HEIGHT, FrameFormat::MJPEG, CAPTURE_FPS);
    let requested = RequestedFormat::with_formats(
        RequestedFormatType::Closest(cam_format),
        &[FrameFormat::MJPEG],
    );
    let mut camera = Camera::new(index, requested)?;
    camera.open_stream()?;
    Ok(camera)
}

fn decode_jpeg(jpeg_data: &[u8]) -> Option<Vec<u8>> {
    use image::load_from_memory;
    match load_from_memory(jpeg_data) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            Some(rgba.into_raw())
        }
        Err(e) => {
            warn!("JPEG decode error: {}", e);
            None
        }
    }
}
