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

pub struct CameraFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: FrameFormat,
    pub timestamp: std::time::Instant,
}

#[derive(Resource)]
pub struct CameraResource {
    pub frame: Arc<Mutex<Option<CameraFrame>>>,
    pub is_active: Arc<AtomicBool>,
}

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
                            let w = frame.resolution().width();
                            let h = frame.resolution().height();
                            let src_fmt = frame.source_frame_format();

                            let data = frame_to_rgba(&raw, w, h, src_fmt);

                            let new_frame = CameraFrame {
                                data,
                                width: w,
                                height: h,
                                format: src_fmt,
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

        info!("Camera disconnected or unavailable \u{2013} retrying in 2 s ...");
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}

fn open_camera() -> Result<Camera, nokhwa::NokhwaError> {
    let attempts: &[(u32, u32, u32, FrameFormat)] = &[
        (640, 480, 30, FrameFormat::MJPEG),
        (640, 480, 15, FrameFormat::MJPEG),
        (1280, 720, 30, FrameFormat::MJPEG),
        (640, 480, 30, FrameFormat::YUYV),
        (640, 480, 30, FrameFormat::NV12),
        (640, 480, 30, FrameFormat::RAWRGB),
    ];

    for &(w, h, fps, fmt) in attempts {
        let index = CameraIndex::Index(0);
        let cf = CameraFormat::new_from(w, h, fmt, fps);
        let formats = [fmt];
        let req = RequestedFormat::with_formats(RequestedFormatType::Closest(cf), &formats);
        if let Ok(mut camera) = Camera::new(index, req) {
            if camera.open_stream().is_ok() {
                let actual = camera.camera_format();
                info!("Camera opened: {}x{} {:?} {}fps",
                    actual.resolution().width(),
                    actual.resolution().height(),
                    actual.format(),
                    actual.frame_rate());
                return Ok(camera);
            }
        }
    }

    Err(nokhwa::NokhwaError::OpenDeviceError(
        "camera_capture".to_string(),
        "no supported camera format found".to_string(),
    ))
}

fn frame_to_rgba(data: &[u8], width: u32, height: u32, fmt: FrameFormat) -> Vec<u8> {
    match fmt {
        FrameFormat::MJPEG => decode_jpeg(data).unwrap_or_else(|| data.to_vec()),
        FrameFormat::YUYV => yuyv_to_rgba(data, width, height),
        FrameFormat::NV12 => nv12_to_rgba(data, width, height),
        FrameFormat::RAWRGB | FrameFormat::RAWBGR => {
            raw_to_rgba(data, width, height, fmt == FrameFormat::RAWBGR)
        }
        FrameFormat::GRAY => {
            data.iter().take((width * height) as usize)
                .flat_map(|&g| vec![g, g, g, 255])
                .collect()
        }
    }
}

fn decode_jpeg(jpeg_data: &[u8]) -> Option<Vec<u8>> {
    image::load_from_memory(jpeg_data)
        .ok()
        .map(|img| img.to_rgba8().into_raw())
}

fn yuyv_to_rgba(data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let i = ((y * width + x) * 2) as usize;
            let y_val = data.get(i).copied().unwrap_or(128) as f32;
            let u = data.get(i + 1).copied().unwrap_or(128) as f32;
            let v = if x % 2 == 0 {
                data.get(i + 3).copied().unwrap_or(128) as f32
            } else {
                data.get(i - 1).copied().unwrap_or(128) as f32
            };
            let cy = y_val - 16.0;
            let cu = u - 128.0;
            let cv = v - 128.0;
            rgba.extend_from_slice(&[
                (1.164 * cy + 1.596 * cv).clamp(0.0, 255.0) as u8,
                (1.164 * cy - 0.392 * cu - 0.813 * cv).clamp(0.0, 255.0) as u8,
                (1.164 * cy + 2.017 * cu).clamp(0.0, 255.0) as u8,
                255,
            ]);
        }
    }
    rgba
}

fn nv12_to_rgba(data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    let frame_size = (width * height) as usize;
    for y in 0..height {
        for x in 0..width {
            let yi = (y * width + x) as usize;
            let y_val = data.get(yi).copied().unwrap_or(128) as f32;
            let uv_offset = frame_size + ((y / 2) * (width / 2) + (x / 2)) as usize * 2;
            let u = data.get(uv_offset).copied().unwrap_or(128) as f32;
            let v = data.get(uv_offset + 1).copied().unwrap_or(128) as f32;
            let cy = y_val - 16.0;
            let cu = u - 128.0;
            let cv = v - 128.0;
            rgba.extend_from_slice(&[
                (1.164 * cy + 1.596 * cv).clamp(0.0, 255.0) as u8,
                (1.164 * cy - 0.392 * cu - 0.813 * cv).clamp(0.0, 255.0) as u8,
                (1.164 * cy + 2.017 * cu).clamp(0.0, 255.0) as u8,
                255,
            ]);
        }
    }
    rgba
}

fn raw_to_rgba(data: &[u8], width: u32, height: u32, bgr: bool) -> Vec<u8> {
    let pixels = (width * height) as usize;
    let mut rgba = Vec::with_capacity(pixels * 4);
    for i in 0..pixels {
        let base = i * 3;
        if base + 2 >= data.len() {
            break;
        }
        let (r, g, b) = if bgr {
            (data[base + 2], data[base + 1], data[base])
        } else {
            (data[base], data[base + 1], data[base + 2])
        };
        rgba.extend_from_slice(&[r, g, b, 255]);
    }
    rgba
}
