use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::postgres::PgPool;

use crate::db;

const SIMILARITY_THRESHOLD: f32 = 0.4;
const MATCH_BUFFER_SIZE: usize = 5;
const MAX_EMBEDDINGS_PER_PERSON: usize = 3;
const STALE_SECS: f64 = 5.0;

#[derive(Debug, Clone)]
pub struct Visit {
    pub start: f64,
    pub end: f64,
    pub camera_id: String,
}

#[derive(Debug, Clone)]
pub struct FaceSnapshot {
    pub bbox: [f32; 4],
    pub embedding: Vec<f32>,
    pub face_jpeg: Vec<u8>,
    pub camera_id: String,
    pub timestamp: f64,
}

#[derive(Clone)]
pub struct CachedPerson {
    pub id: i32,
    pub first_seen: f64,
    pub last_seen: f64,
    pub total_seen: f64,
    pub best_face_jpeg: Vec<u8>,
    pub ref_embeddings: Vec<CachedEmbedding>,
    pub open_visit_id: Option<i64>,
    pub visits: Vec<Visit>,
}

#[derive(Clone)]
pub struct CachedEmbedding {
    pub db_id: i64,
    pub vector: Vec<f32>,
    pub face_jpeg: Vec<u8>,
}

/// A pending face being buffered (tracked by approximate position).
struct PendingBuffer {
    embeddings: Vec<Vec<f32>>,
    face_jpegs: Vec<Vec<u8>>,
    bboxes: Vec<[f32; 4]>,
    first_seen: f64,
    camera_id: String,
}

pub struct FaceTracker {
    pool: PgPool,
    pub people: Vec<CachedPerson>,
    next_temp_id: u64,
    pending: HashMap<u64, PendingBuffer>,
}

impl FaceTracker {
    pub async fn new(pool: PgPool) -> Self {
        let persons = db::load_all_persons(&pool).await;
        let open_visits = db::load_open_visits(&pool).await;

        let mut open_map: HashMap<i32, i64> = HashMap::new();
        for v in &open_visits {
            open_map.insert(v.person_id, v.id);
        }

        let mut people = Vec::with_capacity(persons.len());
        for p in persons {
            let ref_embeddings: Vec<CachedEmbedding> = p
                .embeddings
                .into_iter()
                .map(|e| CachedEmbedding {
                    db_id: e.id,
                    vector: e.vector.into_iter().map(|v| v as f32).collect(),
                    face_jpeg: e.face_jpeg,
                })
                .collect();
            let visits: Vec<Visit> = db::load_person_visits(&pool, p.id)
                .await
                .into_iter()
                .map(|v| Visit {
                    start: v.start_time,
                    end: v.end_time,
                    camera_id: v.camera_id,
                })
                .collect();
            people.push(CachedPerson {
                id: p.id,
                first_seen: p.first_seen,
                last_seen: p.last_seen,
                total_seen: p.total_seen,
                best_face_jpeg: Vec::new(),
                ref_embeddings,
                open_visit_id: open_map.get(&p.id).copied(),
                visits,
            });
        }

        println!("[tracker] loaded {} persons from DB", people.len());
        Self {
            pool,
            people,
            next_temp_id: 1000,
            pending: HashMap::new(),
        }
    }

    pub fn all_people(&self) -> &[CachedPerson] {
        &self.people
    }

    pub async fn ingest(&mut self, faces: Vec<FaceSnapshot>) {
        for face in faces {
            if let Some(pid) = self.match_cached(&face.embedding) {
                self.update_existing(pid, face).await;
            } else {
                self.buffer_face(face);
            }
        }
        self.flush_buffers().await;
    }

    fn match_cached(&self, emb: &[f32]) -> Option<i32> {
        let mut best_score = 0.0f32;
        let mut best_id = None;

        for person in &self.people {
            for ref_emb in &person.ref_embeddings {
                let sim = cosine_similarity(emb, &ref_emb.vector);
                if sim > best_score {
                    best_score = sim;
                    best_id = Some(person.id);
                }
            }
        }

        if best_score > SIMILARITY_THRESHOLD {
            best_id
        } else {
            None
        }
    }

    fn buffer_face(&mut self, face: FaceSnapshot) {
        let cx = (face.bbox[0] + face.bbox[2]) / 2.0;
        let cy = (face.bbox[1] + face.bbox[3]) / 2.0;

        let matched_buffer = self.pending.iter_mut().find(|(_, buf)| {
            if buf.bboxes.is_empty() {
                return false;
            }
            let last = buf.bboxes.last().unwrap();
            let lcx = (last[0] + last[2]) / 2.0;
            let lcy = (last[1] + last[3]) / 2.0;
            ((cx - lcx).powi(2) + (cy - lcy).powi(2)).sqrt() < 0.15
        });

        if let Some((_, buf)) = matched_buffer {
            if buf.embeddings.len() < MATCH_BUFFER_SIZE {
                buf.embeddings.push(face.embedding);
                buf.face_jpegs.push(face.face_jpeg);
                buf.bboxes.push(face.bbox);
            }
        } else {
            let tid = self.next_temp_id;
            self.next_temp_id += 1;
            self.pending.insert(
                tid,
                PendingBuffer {
                    embeddings: vec![face.embedding],
                    face_jpegs: vec![face.face_jpeg],
                    bboxes: vec![face.bbox],
                    first_seen: face.timestamp,
                    camera_id: face.camera_id,
                },
            );
        }
    }

    /// Flush buffers that have >= MATCH_BUFFER_SIZE frames: batch-match against stored persons.
    async fn flush_buffers(&mut self) {
        let ready: Vec<u64> = self
            .pending
            .iter()
            .filter(|(_, buf)| buf.embeddings.len() >= MATCH_BUFFER_SIZE)
            .map(|(k, _)| *k)
            .collect();

        for tid in ready {
            if let Some(buf) = self.pending.remove(&tid) {
                let result = self.match_buffered(&buf.embeddings);

                let best_idx = buf
                    .face_jpegs
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, j)| j.len())
                    .map(|(i, _)| i)
                    .unwrap_or(0);

                let now = now_secs();
                let snapshot = FaceSnapshot {
                    bbox: buf.bboxes[best_idx],
                    embedding: buf.embeddings[best_idx].clone(),
                    face_jpeg: buf.face_jpegs[best_idx].clone(),
                    camera_id: buf.camera_id.clone(),
                    timestamp: buf.first_seen,
                };

                if let Some(pid) = result {
                    self.update_existing(pid, snapshot).await;
                } else {
                    self.create_new(snapshot, now).await;
                }
            }
        }
    }

    fn match_buffered(&self, buffered: &[Vec<f32>]) -> Option<i32> {
        if buffered.is_empty() || self.people.is_empty() {
            return None;
        }

        let mut person_scores: HashMap<i32, f32> = HashMap::new();

        for buf_emb in buffered {
            for person in &self.people {
                let best_sim = person
                    .ref_embeddings
                    .iter()
                    .map(|r| cosine_similarity(buf_emb, &r.vector))
                    .fold(0.0f32, f32::max);
                *person_scores.entry(person.id).or_insert(0.0) += best_sim;
            }
        }

        let n = buffered.len() as f32;
        let mut best_id = None;
        let mut best_avg = 0.0;

        for (&pid, total) in &person_scores {
            let avg = total / n;
            if avg > best_avg {
                best_avg = avg;
                best_id = Some(pid);
            }
        }

        if best_avg > SIMILARITY_THRESHOLD {
            best_id
        } else {
            None
        }
    }

    async fn update_existing(&mut self, pid: i32, face: FaceSnapshot) {
        let now = face.timestamp;

        if let Some(person) = self.people.iter_mut().find(|p| p.id == pid) {
            let delta = now - person.last_seen;
            if delta > 0.0 {
                person.total_seen += delta;
            }
            person.last_seen = now;
            if face.face_jpeg.len() > person.best_face_jpeg.len() {
                person.best_face_jpeg = face.face_jpeg.clone();
            }
        }

        db::update_person_touch(&self.pool, pid, now, (now - face.timestamp).max(0.0)).await;

        if let Some(person) = self.people.iter().find(|p| p.id == pid) {
            match person.open_visit_id {
                Some(vid) => {
                    db::update_visit_end(&self.pool, vid, now).await;
                }
                None => {
                    let vid = db::create_visit(&self.pool, pid, now, &face.camera_id).await;
                    if let Some(p) = self.people.iter_mut().find(|p| p.id == pid) {
                        p.open_visit_id = Some(vid);
                    }
                }
            }
        }

        self.maybe_add_reference(pid, &face.embedding, &face.face_jpeg, now)
            .await;
    }

    async fn create_new(&mut self, face: FaceSnapshot, now: f64) {
        let pid = db::create_person(&self.pool, now).await;
        let vid = db::create_visit(&self.pool, pid, now, &face.camera_id).await;

        db::add_embedding(&self.pool, pid, &face.embedding, &face.face_jpeg, now).await;

        self.people.push(CachedPerson {
            id: pid,
            first_seen: now,
            last_seen: now,
            total_seen: 0.0,
            best_face_jpeg: face.face_jpeg,
            ref_embeddings: vec![CachedEmbedding {
                db_id: 0,
                vector: face.embedding,
                face_jpeg: Vec::new(),
            }],
            open_visit_id: Some(vid),
            visits: vec![Visit {
                start: now,
                end: now,
                camera_id: face.camera_id,
            }],
        });

        println!("[tracker] created person #{}", pid);
    }

    /// Add a new reference embedding if room (max 3) and not too similar to existing.
    async fn maybe_add_reference(
        &mut self,
        pid: i32,
        embedding: &[f32],
        face_jpeg: &[u8],
        now: f64,
    ) {
        let person = match self.people.iter_mut().find(|p| p.id == pid) {
            Some(p) => p,
            None => return,
        };

        for ref_emb in &person.ref_embeddings {
            if cosine_similarity(embedding, &ref_emb.vector) > 0.92 {
                return;
            }
        }

        db::add_embedding(&self.pool, pid, embedding, face_jpeg, now).await;
        person.ref_embeddings.push(CachedEmbedding {
            db_id: 0,
            vector: embedding.to_vec(),
            face_jpeg: Vec::new(),
        });

        if person.ref_embeddings.len() > MAX_EMBEDDINGS_PER_PERSON {
            person.ref_embeddings.remove(0);
            db::trim_embeddings(&self.pool, pid).await;
        }
    }

    pub async fn close_stale_visits(&mut self) {
        let now = now_secs();
        db::close_stale_visits(&self.pool, now, STALE_SECS).await;

        for person in &mut self.people {
            if person.open_visit_id.is_some() && now - person.last_seen > STALE_SECS {
                person.open_visit_id = None;
            }
        }
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum();
    let nb: f32 = b.iter().map(|x| x * x).sum();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

pub fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}
