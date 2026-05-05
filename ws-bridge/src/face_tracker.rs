use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const SIMILARITY_THRESHOLD: f32 = 0.5;

#[derive(Debug, Clone)]
pub struct FaceSnapshot {
    pub bbox: [f32; 4],
    pub embedding: Vec<f32>,
    pub face_jpeg: Vec<u8>,
    pub camera_id: String,
    pub timestamp: f64,
}

#[derive(Debug, Clone)]
pub struct Visit {
    pub start: f64,
    pub end: f64,
    pub camera_id: String,
    pub face_snapshots: Vec<FaceSnapshot>,
}

#[derive(Debug, Clone)]
pub struct Person {
    pub id: usize,
    pub first_seen: f64,
    pub last_seen: f64,
    pub total_seen_secs: f64,
    pub visits: Vec<Visit>,
    pub best_face_jpeg: Vec<u8>,
}

pub struct FaceTracker {
    next_id: usize,
    people: Vec<Person>,
    current_visits: HashMap<usize, Visit>,
}

impl FaceTracker {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            people: Vec::new(),
            current_visits: HashMap::new(),
        }
    }

    pub fn all_people(&self) -> &[Person] {
        &self.people
    }

    pub fn get_person(&self, id: usize) -> Option<&Person> {
        self.people.iter().find(|p| p.id == id)
    }

    pub fn ingest(&mut self, faces: Vec<FaceSnapshot>, now: f64) {
        for face in faces {
            let matched_id = self.match_embedding(&face.embedding);

            if let Some(pid) = matched_id {
                let person = self.people.iter_mut().find(|p| p.id == pid).unwrap();
                person.last_seen = now;
                person.total_seen_secs += 1.0;

                if face.face_jpeg.len() > person.best_face_jpeg.len() {
                    person.best_face_jpeg = face.face_jpeg.clone();
                }

                if let Some(visit) = self.current_visits.get_mut(&pid) {
                    visit.end = now;
                    visit.face_snapshots.push(face);
                }
            } else {
                let pid = self.next_id;
                self.next_id += 1;

                let visit = Visit {
                    start: now,
                    end: now,
                    camera_id: face.camera_id.clone(),
                    face_snapshots: vec![face.clone()],
                };

                self.current_visits.insert(pid, visit);

                self.people.push(Person {
                    id: pid,
                    first_seen: now,
                    last_seen: now,
                    total_seen_secs: 0.0,
                    visits: Vec::new(),
                    best_face_jpeg: face.face_jpeg.clone(),
                });
            }
        }
    }

    pub fn close_visit(&mut self, person_id: usize, now: f64) {
        if let Some(visit) = self.current_visits.remove(&person_id) {
            if let Some(person) = self.people.iter_mut().find(|p| p.id == person_id) {
                let mut v = visit;
                v.end = now;
                person.visits.push(v);
            }
        }
    }

    pub fn close_all_visits(&mut self, now: f64) {
        let ids: Vec<usize> = self.current_visits.keys().copied().collect();
        for id in ids {
            self.close_visit(id, now);
        }
    }

    fn match_embedding(&self, emb: &[f32]) -> Option<usize> {
        if emb.is_empty() {
            return None;
        }
        let mut best = (0usize, 0.0f32);
        for person in &self.people {
            // check closed visits first, then current (open) visit — may not be closed yet
            let last_emb = person.visits.last()
                .and_then(|v| v.face_snapshots.last())
                .map(|s| &s.embedding[..])
                .or_else(|| {
                    self.current_visits.get(&person.id)
                        .and_then(|v| v.face_snapshots.last())
                        .map(|s| &s.embedding[..])
                });
            if let Some(last_emb) = last_emb {
                let sim = cosine_similarity(emb, last_emb);
                if sim > best.1 {
                    best = (person.id, sim);
                }
            }
        }
        if best.1 > SIMILARITY_THRESHOLD {
            Some(best.0)
        } else {
            None
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
