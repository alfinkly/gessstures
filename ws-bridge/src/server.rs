use std::sync::Arc;

use engine::{Engine, GraphSnapshot};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message;

use base64::Engine as _;

use super::face_tracker;

#[derive(Deserialize)]
struct CmdMsg {
    #[serde(rename = "t")]
    msg_type: String,
}

// ── Detector: spawns a Python subprocess (body or face) ────────

#[derive(Clone)]
struct Detector {
    inner: Arc<Mutex<(String, DetectorInner)>>,
}

struct DetectorInner {
    _child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

impl Detector {
    async fn spawn_process(py_script: &str) -> Result<DetectorInner, String> {
        let mut child = Command::new("python3")
            .args(["-u", py_script])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| format!("spawn {py_script}: {e}"))?;

        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = BufReader::new(child.stdout.take().ok_or("no stdout")?);

        Ok(DetectorInner { _child: child, stdin, stdout })
    }

    async fn new(py_script: &str) -> Result<Self, String> {
        let inner = Self::spawn_process(py_script).await?;
        Ok(Self {
            inner: Arc::new(Mutex::new((py_script.to_string(), inner))),
        })
    }

    async fn detect(&self, jpeg: &[u8]) -> Result<String, String> {
        let mut guard = self.inner.lock().await;
        let (ref script, ref mut inner) = &mut *guard;

        let do_detect = async {
            let size = (jpeg.len() as u32).to_be_bytes();
            inner.stdin.write_all(&size).await.map_err(|e| format!("stdin: {e}"))?;
            inner.stdin.write_all(jpeg).await.map_err(|e| format!("stdin: {e}"))?;
            inner.stdin.flush().await.map_err(|e| format!("flush: {e}"))?;
            let mut line = String::new();
            inner.stdout.read_line(&mut line).await.map_err(|e| format!("stdout: {e}"))?;
            Ok::<_, String>(line.trim().to_string())
        };

        match do_detect.await {
            Ok(r) => Ok(r),
            Err(e) => {
                eprintln!("detector process died, restarting...");
                match Self::spawn_process(script).await {
                    Ok(new_inner) => {
                        *inner = new_inner;
                        // retry once
                        let size = (jpeg.len() as u32).to_be_bytes();
                        inner.stdin.write_all(&size).await.map_err(|e| format!("stdin: {e}"))?;
                        inner.stdin.write_all(jpeg).await.map_err(|e| format!("stdin: {e}"))?;
                        inner.stdin.flush().await.map_err(|e| format!("flush: {e}"))?;
                        let mut line = String::new();
                        inner.stdout.read_line(&mut line).await.map_err(|e| format!("stdout: {e}"))?;
                        Ok(line.trim().to_string())
                    }
                    Err(restart_e) => {
                        eprintln!("restart failed: {restart_e}");
                        Err(e)
                    }
                }
            }
        }
    }
}

// ── Server ─────────────────────────────────────────────────────

pub async fn start(port: u16, engine: Arc<Mutex<Engine>>, tracker: Arc<Mutex<face_tracker::FaceTracker>>) {
    let body_det = Detector::new("app/mediapipe_body.py").await.ok();
    let face_det = Detector::new("app/face_detect.py").await.ok();

    if body_det.is_none() {
        eprintln!("WARN: body detector not available");
    }
    if face_det.is_none() {
        eprintln!("WARN: face detector not available (install insightface)");
    }

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
    println!("WebSocket server listening on ws://{}", addr);

    let (snapshot_tx, _) = broadcast::channel::<GraphSnapshot>(16);
    let (people_tx, _) = broadcast::channel::<String>(16);

    // physics tick
    let eng = engine.clone();
    let tx = snapshot_tx.clone();
    tokio::spawn(async move {
        let mut int = tokio::time::interval(std::time::Duration::from_secs_f64(1.0 / 60.0));
        int.tick().await;
        loop {
            int.tick().await;
            let e = eng.lock().await;
            let snap = e.snapshot();
            drop(e);
            let _ = tx.send(snap);
        }
    });

    let tr = tracker.clone();
    let ptx = people_tx.clone();
    tokio::spawn(async move {
        let mut int = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            int.tick().await;
            let t = tr.lock().await;
            let now_s = face_tracker::now_secs();
            let people: Vec<PersonView> = t
                .all_people()
                .iter()
                .map(|p| {
                    let current = now_s - p.last_seen < 5.0;
                    PersonView {
                        id: p.id,
                        first_seen: p.first_seen,
                        last_seen: p.last_seen,
                        total_seen_secs: p.total_seen,
                        visit_count: p.visits.len(),
                        face_jpeg_b64: if p.best_face_jpeg.is_empty() { String::new() } else { base64::engine::general_purpose::STANDARD.encode(&p.best_face_jpeg) },
                        visits: p.visits.iter().map(|v| VisitView { start: v.start, end: v.end, camera_id: v.camera_id.clone() }).collect(),
                        current,
                    }
                })
                .collect();
            eprintln!("[people] broadcasting {} people", people.len());
            if let Ok(json) = serde_json::to_string(&PeopleList { people }) {
                let _ = ptx.send(json);
            }
        }
    });

    while let Ok((stream, peer)) = listener.accept().await {
        println!("Client connected: {}", peer);
        let eng = engine.clone();
        let tr = tracker.clone();
        let snap_rx = snapshot_tx.subscribe();
        let people_rx = people_tx.subscribe();
        let bd = body_det.clone();
        let fd = face_det.clone();
        tokio::spawn(async move {
            handle_connection(stream, eng, tr, snap_rx, people_rx, bd, fd).await;
        });
    }
}

#[derive(Serialize)]
struct PersonView {
    id: i32,
    first_seen: f64,
    last_seen: f64,
    total_seen_secs: f64,
    visit_count: usize,
    face_jpeg_b64: String,
    visits: Vec<VisitView>,
    current: bool,
}

#[derive(Serialize)]
struct VisitView {
    start: f64,
    end: f64,
    camera_id: String,
}

#[derive(Serialize)]
struct PeopleList {
    people: Vec<PersonView>,
}

// ── Connection handler ─────────────────────────────────────────

async fn handle_connection(
    stream: TcpStream,
    engine: Arc<Mutex<Engine>>,
    tracker: Arc<Mutex<face_tracker::FaceTracker>>,
    mut snapshot_rx: broadcast::Receiver<GraphSnapshot>,
    mut people_rx: broadcast::Receiver<String>,
    body_det: Option<Detector>,
    face_det: Option<Detector>,
) {
    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("WebSocket handshake failed: {e}");
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (response_tx, mut response_rx) = mpsc::channel::<String>(32);

    // receiver task: handle incoming messages
    let tr = tracker.clone();
    tokio::spawn(async move {
        let mut last_face_det = tokio::time::Instant::now();
        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Binary(data) => {
                    // body detection (every frame)
                    if let Some(ref det) = body_det {
                        match det.detect(&data).await {
                            Ok(r) => { let _ = response_tx.send(r).await; }
                            Err(e) => eprintln!("body detect err: {e}"),
                        }
                    }
                    // face detection (at most once per second)
                    if let Some(ref det) = face_det {
                        if last_face_det.elapsed() >= std::time::Duration::from_secs(1) {
                            last_face_det = tokio::time::Instant::now();
                            eprintln!("[face_det] running detection...");
                            match det.detect(&data).await {
                                Ok(r) => {
                                    if let Ok(fr) = serde_json::from_str::<FaceResponse>(&r) {
                                        eprintln!("[face_det] got {} faces", fr.faces.len());
                                        let now = face_tracker::now_secs();
                                        let snapshots: Vec<face_tracker::FaceSnapshot> = fr.faces.into_iter().map(|f| {
                                            let jpeg_bytes = base64_decode(&f.face_jpeg_b64);
                                            face_tracker::FaceSnapshot {
                                                bbox: f.bbox_norm,
                                                embedding: f.embedding,
                                                face_jpeg: jpeg_bytes,
                                                camera_id: "cam_0".into(),
                                                timestamp: now,
                                            }
                                        }).collect();
                                        if !snapshots.is_empty() {
                                            eprintln!("[face_det] ingesting {} faces into tracker", snapshots.len());
                                            let mut t = tr.lock().await;
                                            t.ingest(snapshots).await;
                                        }
                                    } else {
                                        eprintln!("[face_det] failed to parse response: {}", r);
                                    }
                                }
                                Err(e) => eprintln!("face detect err: {e}"),
                            }
                        }
                    }
                }
                Message::Text(text) => {
                    if let Ok(cmd) = serde_json::from_str::<CmdMsg>(&text) {
                        let mut eng = engine.lock().await;
                        match cmd.msg_type.as_str() {
                            "pause" => eng.paused = true,
                            "resume" => eng.paused = false,
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    });

    // main send loop
    loop {
        tokio::select! {
            snap = snapshot_rx.recv() => {
                match snap {
                    Ok(s) => {
                        if let Ok(j) = serde_json::to_string(&s) {
                            if ws_sender.send(Message::Text(j.into())).await.is_err() { break; }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => eprintln!("lagged {n}"),
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            ppl = people_rx.recv() => {
                if let Ok(j) = ppl {
                    if ws_sender.send(Message::Text(format!(r#"{{"t":"people","d":{}}}"#, j))).await.is_err() { break; }
                }
            }
            resp = response_rx.recv() => {
                if let Some(r) = resp {
                    if ws_sender.send(Message::Text(r.into())).await.is_err() { break; }
                }
            }
        }
    }
}

// ── Face response types ────────────────────────────────────────

#[derive(Deserialize)]
struct FaceResponse {
    #[allow(dead_code)]
    face_count: usize,
    faces: Vec<FaceEntry>,
}

#[derive(Deserialize)]
struct FaceEntry {
    bbox_norm: [f32; 4],
    embedding: Vec<f32>,
    #[allow(dead_code)]
    confidence: f32,
    face_jpeg_b64: String,
}

fn base64_decode(s: &str) -> Vec<u8> {
    use base64::engine::general_purpose;
    general_purpose::STANDARD.decode(s).unwrap_or_default()
}
