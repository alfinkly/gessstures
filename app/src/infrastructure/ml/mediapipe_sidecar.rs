use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::prelude::*;
use serde::Deserialize;

use hand_tracking_core::{HandLandmark, HandLandmarkData, HandLandmarkResource};
use crate::interface::plugins::hand_tracking_plugin::UseSidecar;

#[derive(Deserialize)]
struct SidecarOutput {
    detected: bool,
    landmarks: Option<Vec<[f32; 3]>>,
    timestamp: f64,
}

#[derive(Resource)]
pub struct SidecarResource {
    child: Arc<Mutex<Option<Child>>>,
    shutdown: Arc<AtomicBool>,
}

impl Drop for SidecarResource {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

pub struct SidecarPlugin;

impl Plugin for SidecarPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UseSidecar)
            .add_systems(Startup, start_sidecar);
    }
}

fn start_sidecar(mut commands: Commands, hand_landmarks: Res<HandLandmarkResource>) {
    let landmarks_inner = hand_landmarks.inner.clone();
    let child_handle: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    let child_handle_clone = child_handle.clone();
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    std::thread::Builder::new()
        .name("sidecar-reader".into())
        .spawn(move || {
            while !shutdown_clone.load(Ordering::SeqCst) {
                let mut child: Child = match Command::new("python3")
                    .args(["-u", "app/mediapipe_hands.py"])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::inherit())
                    .spawn()
                {
                    Ok(c) => {
                        info!("Sidecar spawned (PID {})", c.id());
                        c
                    }
                    Err(e) => {
                        error!("Failed to spawn sidecar: {}", e);
                        std::thread::sleep(Duration::from_secs(2));
                        continue;
                    }
                };

                let stdout = match child.stdout.take() {
                    Some(s) => s,
                    None => {
                        error!("Sidecar stdout not available");
                        let _ = child.kill();
                        let _ = child.wait();
                        std::thread::sleep(Duration::from_secs(1));
                        continue;
                    }
                };

                *child_handle_clone.lock().unwrap() = Some(child);

                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if shutdown_clone.load(Ordering::SeqCst) {
                        break;
                    }

                    let line = match line {
                        Ok(l) => l,
                        Err(_) => break,
                    };

                    if line.trim().is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<SidecarOutput>(&line) {
                        Ok(output) => {
                            let data = if output.detected && output.landmarks.is_some() {
                                let lms: Vec<HandLandmark> = output
                                    .landmarks
                                    .unwrap()
                                    .iter()
                                    .map(|&[x, y, z]| HandLandmark { x, y, z })
                                    .collect();
                                HandLandmarkData {
                                    landmarks: Some(lms),
                                    hand_count: 1,
                                    timestamp: std::time::Instant::now(),
                                }
                            } else {
                                HandLandmarkData::default()
                            };

                            if let Ok(mut guard) = landmarks_inner.lock() {
                                *guard = data;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse sidecar JSON line: {}", e);
                        }
                    }
                }

                if let Some(mut child) = child_handle_clone.lock().unwrap().take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }

                if !shutdown_clone.load(Ordering::SeqCst) {
                    info!("Sidecar exited \u{2013} restarting in 1 s \u{2026}");
                    std::thread::sleep(Duration::from_secs(1));
                }
            }

            info!("Sidecar reader thread exiting.");
        })
        .expect("failed to spawn sidecar-reader thread");

    commands.insert_resource(SidecarResource {
        child: child_handle,
        shutdown,
    });

    info!("SidecarPlugin started \u{2013} hand tracking via Python MediaPipe.");
}
