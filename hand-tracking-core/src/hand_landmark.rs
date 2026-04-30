use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy)]
pub struct HandLandmark {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone)]
pub struct HandLandmarkData {
    pub landmarks: Option<Vec<HandLandmark>>,
    pub hand_count: u32,
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

#[cfg_attr(feature = "bevy", derive(bevy::prelude::Resource))]
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
