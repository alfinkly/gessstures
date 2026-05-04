use bevy::prelude::*;
use serde::Deserialize;
use std::io::{BufRead, BufReader, Write};

use crate::infrastructure::camera::capture_adapter::CameraResource;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Bounding box of a detected person in normalized coordinates [0,1]
/// cx, cy = center, w, h = width and height
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PersonBbox {
    pub cx: f32,
    pub cy: f32,
    pub w: f32,
    pub h: f32,
}

/// Full data for a single detected person, including bounding box and optional pose keypoints.
#[derive(Debug, Clone)]
pub struct PersonData {
    pub bbox: PersonBbox,
    pub keypoints: Option<Vec<[f32; 3]>>,
}

/// Full JSON output from the Python sidecar
#[derive(Debug, Clone, Deserialize)]
struct SidecarPersonOutput {
    person_count: u32,
    persons: Vec<PersonEntry>,
    timestamp: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct PersonEntry {
    #[allow(dead_code)]
    bbox: [f32; 4], // [cx, cy, w, h]
    #[allow(dead_code)]
    keypoints: Option<Vec<[f32; 3]>>,
}

/// Bevy resource holding detected person data for the current frame
#[derive(Resource, Default)]
pub struct PersonTrackerResource {
    pub persons: Vec<PersonData>,
    pub person_count: u32,
    pub timestamp: f64,
}

pub struct BodyTrackingPlugin;

impl Plugin for BodyTrackingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PersonTrackerResource>()
            .add_systems(Startup, start_body_tracker)
            .add_systems(PreUpdate, update_body_tracking);
    }
}

/// Internal resource managing the sidecar process lifecycle.
#[derive(Resource)]
struct BodyTrackerProcess {
    child: Arc<Mutex<Option<Child>>>,
    shutdown: Arc<AtomicBool>,
    latest: Arc<Mutex<PersonTrackerResource>>,
}

impl Drop for BodyTrackerProcess {
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

fn start_body_tracker(
    mut commands: Commands,
    camera_resource: Res<CameraResource>,
) {
    let latest: Arc<Mutex<PersonTrackerResource>> =
        Arc::new(Mutex::new(PersonTrackerResource::default()));
    let child_handle: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    let shutdown = Arc::new(AtomicBool::new(false));
    let camera_frame = camera_resource.frame.clone();

    let latest_clone = latest.clone();
    let child_clone = child_handle.clone();
    let shutdown_clone = shutdown.clone();

    std::thread::Builder::new()
        .name("body-tracker".into())
        .spawn(move || {
            while !shutdown_clone.load(Ordering::SeqCst) {
                if let Some(mut old) = child_clone.lock().unwrap().take() {
                    let _ = old.kill();
                    let _ = old.wait();
                }

                let mut child = match Command::new("python3")
                    .args(["-u", "app/mediapipe_body.py"])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::inherit())
                    .spawn()
                {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Body tracker: failed to spawn python3: {}", e);
                        std::thread::sleep(std::time::Duration::from_secs(2));
                        continue;
                    }
                };

                let mut child_stdin = match child.stdin.take() {
                    Some(s) => s,
                    None => {
                        error!("Body tracker: no stdin from child process");
                        let _ = child.kill();
                        let _ = child.wait();
                        continue;
                    }
                };

                let child_stdout = match child.stdout.take() {
                    Some(s) => s,
                    None => {
                        error!("Body tracker: no stdout from child process");
                        let _ = child.kill();
                        let _ = child.wait();
                        continue;
                    }
                };

                *child_clone.lock().unwrap() = Some(child);

                let latest_clone2 = latest_clone.clone();
                let shutdown_clone2 = shutdown_clone.clone();
                std::thread::spawn(move || {
                    let reader = BufReader::new(child_stdout);
                    for line in reader.lines() {
                        if shutdown_clone2.load(Ordering::SeqCst) {
                            break;
                        }
                        let line = match line {
                            Ok(l) => l,
                            Err(_) => break,
                        };
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        match serde_json::from_str::<SidecarPersonOutput>(trimmed) {
                            Ok(output) => {
                                let mut state = latest_clone2.lock().unwrap();
                                state.timestamp = output.timestamp;
                                state.person_count = output.person_count;
                                state.persons = output
                                    .persons
                                    .iter()
                                    .map(|p| PersonData {
                                        bbox: PersonBbox {
                                            cx: p.bbox[0],
                                            cy: p.bbox[1],
                                            w: p.bbox[2],
                                            h: p.bbox[3],
                                        },
                                        keypoints: p.keypoints.clone(),
                                    })
                                    .collect();
                            }
                            Err(e) => {
                                error!(
                                    "Body tracker: failed to parse JSON from sidecar: {}",
                                    e
                                );
                            }
                        }
                    }
                });

                while !shutdown_clone.load(Ordering::SeqCst) {
                    let frame = {
                        let guard = camera_frame.lock().unwrap();
                        guard.as_ref().map(|f| (f.width, f.height, f.data.clone()))
                    };

                    if let Some((w, h, data)) = frame {
                        if writeln!(child_stdin, "{} {}", w, h).is_err() {
                            break;
                        }
                        if child_stdin.write_all(&data).is_err() {
                            break;
                        }
                        if child_stdin.flush().is_err() {
                            break;
                        }
                    } else {
                        std::thread::sleep(std::time::Duration::from_millis(16));
                    }
                }
            }
        })
        .expect("failed to spawn body-tracker thread");

    commands.insert_resource(BodyTrackerProcess {
        child: child_handle,
        shutdown,
        latest,
    });
}

/// Copies latest person data from the background thread into the frame resource.
fn update_body_tracking(
    process: Option<Res<BodyTrackerProcess>>,
    mut resource: ResMut<PersonTrackerResource>,
) {
    let Some(process) = process else {
        return;
    };
    if let Ok(mut guard) = process.latest.lock() {
        let snapshot = std::mem::take(&mut *guard);
        *resource = snapshot;
    };

}
