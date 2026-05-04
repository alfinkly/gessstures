use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use nokhwa::utils::{
    CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType,
};
use nokhwa::Camera;

use hand_tracking_core::config::*;

#[derive(Clone)]
pub struct CameraFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Resource)]
pub struct CameraResource {
    pub frame: Arc<Mutex<CameraFrame>>,
    pub is_active: Arc<AtomicBool>,
}

impl CameraResource {
    fn new() -> Self {
        Self {
            frame: Arc::new(Mutex::new(CameraFrame {
                data: vec![128u8; (640 * 480 * 4) as usize],
                width: 640,
                height: 480,
            })),
            is_active: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub struct CameraCapturePlugin;

impl Plugin for CameraCapturePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CameraResource::new())
            .add_systems(Startup, start_capture);

        fn start_capture(mut res: ResMut<CameraResource>) {
            let shared_frame = res.frame.clone();
            let shared_active = res.is_active.clone();
            info!("Spawning camera capture thread ...");
            std::thread::spawn(move || capture_loop(shared_frame, shared_active));
        }
    }
}

fn capture_loop(shared_frame: Arc<Mutex<CameraFrame>>, is_active: Arc<AtomicBool>) {
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

                            let data = frame_to_rgba(&raw, w, h, frame.source_frame_format());

                            if let Ok(mut guard) = shared_frame.lock() {
                                guard.data = data;
                                guard.width = w;
                                guard.height = h;
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
        (CAMERA_WIDTH, CAMERA_HEIGHT, CAMERA_FPS, FrameFormat::MJPEG),
        (CAMERA_WIDTH, CAMERA_HEIGHT, 15, FrameFormat::MJPEG),
        (1280, 720, CAMERA_FPS, FrameFormat::MJPEG),
        (CAMERA_WIDTH, CAMERA_HEIGHT, CAMERA_FPS, FrameFormat::YUYV),
        (CAMERA_WIDTH, CAMERA_HEIGHT, CAMERA_FPS, FrameFormat::NV12),
        (CAMERA_WIDTH, CAMERA_HEIGHT, CAMERA_FPS, FrameFormat::RAWRGB),
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
        "capture_adapter".to_string(),
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
    let expected = (width * height * 2) as usize;
    if data.len() < expected {
        warn!("YUYV buffer too small: {} < {}", data.len(), expected);
        return vec![0u8; (width * height * 4) as usize];
    }
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let x_even = (x & !1);
            let base = ((y * width + x_even) * 2) as usize;
            let yi = base + (x & 1) as usize;
            let yv = data[yi] as f32;
            let u = data[base + 1] as f32;
            let v = data[base + 3] as f32;
            let cy = yv - 16.0;
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
    let y_size = (width * height) as usize;
    if data.len() < y_size + y_size / 2 {
        warn!("NV12 buffer too small: {} < {}", data.len(), y_size + y_size / 2);
        return vec![0u8; (width * height * 4) as usize];
    }
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let yi = (y * width + x) as usize;
            let yv = data[yi] as f32;
            let uv_base = y_size + ((y / 2) * (width / 2) + (x / 2)) as usize * 2;
            let u = data.get(uv_base).copied().unwrap_or(128) as f32;
            let v = data.get(uv_base + 1).copied().unwrap_or(128) as f32;
            let cy = yv - 16.0;
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
