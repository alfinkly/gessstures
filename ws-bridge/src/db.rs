use sqlx::postgres::PgPool;
use sqlx::Row;

/// A person row loaded from DB with up to 3 reference embeddings.
pub struct DbPerson {
    pub id: i32,
    pub first_seen: f64,
    pub last_seen: f64,
    pub total_seen: f64,
    pub embeddings: Vec<DbEmbedding>,
}

pub struct DbEmbedding {
    pub id: i64,
    pub vector: Vec<f64>,
    pub face_jpeg: Vec<u8>,
    pub captured_at: f64,
}

pub struct DbVisit {
    pub id: i64,
    pub person_id: i32,
    pub start_time: f64,
    pub end_time: f64,
    pub camera_id: String,
}

// ── Initialization ─────────────────────────────────────────────

pub async fn run_migrations(pool: &PgPool) {
    let sql = include_str!("../migrations/001_init.sql");
    for statement in sql.split(';') {
        let s = statement.trim();
        if !s.is_empty() {
            sqlx::query(s).execute(pool).await.unwrap();
        }
    }
    println!("[db] migrations applied");
}

// ── Persons ────────────────────────────────────────────────────

pub async fn load_all_persons(pool: &PgPool) -> Vec<DbPerson> {
    let rows = sqlx::query(
        "SELECT id, first_seen, last_seen, total_seen FROM persons ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id: i32 = row.get("id");
        let embeddings = load_embeddings(pool, id).await;
        out.push(DbPerson {
            id,
            first_seen: row.get("first_seen"),
            last_seen: row.get("last_seen"),
            total_seen: row.get("total_seen"),
            embeddings,
        });
    }
    out
}

pub async fn load_embeddings(pool: &PgPool, person_id: i32) -> Vec<DbEmbedding> {
    let rows = sqlx::query(
        "SELECT id, embedding, face_jpeg, captured_at FROM face_embeddings \
         WHERE person_id = $1 ORDER BY captured_at DESC LIMIT 3",
    )
    .bind(person_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| {
            let vec: Vec<f64> = r.get("embedding");
            DbEmbedding {
                id: r.get("id"),
                vector: vec,
                face_jpeg: r.get::<Option<Vec<u8>>, _>("face_jpeg").unwrap_or_default(),
                captured_at: r.get("captured_at"),
            }
        })
        .collect()
}

pub async fn create_person(pool: &PgPool, now: f64) -> i32 {
    let row = sqlx::query(
        "INSERT INTO persons (first_seen, last_seen, total_seen) VALUES ($1, $2, 0) RETURNING id",
    )
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .unwrap();
    row.get("id")
}

pub async fn update_person_touch(pool: &PgPool, id: i32, now: f64, delta: f64) {
    sqlx::query(
        "UPDATE persons SET last_seen = $1, total_seen = total_seen + $2 WHERE id = $3",
    )
    .bind(now)
    .bind(delta)
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
}

// ── Embeddings ─────────────────────────────────────────────────

pub async fn add_embedding(
    pool: &PgPool,
    person_id: i32,
    embedding: &[f32],
    face_jpeg: &[u8],
    now: f64,
) {
    let vec_f64: Vec<f64> = embedding.iter().map(|v| *v as f64).collect();
    sqlx::query(
        "INSERT INTO face_embeddings (person_id, embedding, face_jpeg, captured_at) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(person_id)
    .bind(&vec_f64)
    .bind(face_jpeg)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();
}

/// Keep only the 3 newest embeddings for this person.
pub async fn trim_embeddings(pool: &PgPool, person_id: i32) {
    sqlx::query(
        "DELETE FROM face_embeddings WHERE id NOT IN ( \
         SELECT id FROM face_embeddings WHERE person_id = $1 ORDER BY captured_at DESC LIMIT 3 \
         ) AND person_id = $1",
    )
    .bind(person_id)
    .execute(pool)
    .await
    .unwrap();
}

// ── Visits ─────────────────────────────────────────────────────

pub async fn create_visit(pool: &PgPool, person_id: i32, now: f64, camera_id: &str) -> i64 {
    let row = sqlx::query(
        "INSERT INTO person_visits (person_id, camera_id, start_time, end_time) \
         VALUES ($1, $2, $3, $3) RETURNING id",
    )
    .bind(person_id)
    .bind(camera_id)
    .bind(now)
    .fetch_one(pool)
    .await
    .unwrap();
    row.get("id")
}

pub async fn update_visit_end(pool: &PgPool, visit_id: i64, now: f64) {
    sqlx::query("UPDATE person_visits SET end_time = $1 WHERE id = $2")
        .bind(now)
        .bind(visit_id)
        .execute(pool)
        .await
        .unwrap();
}

pub async fn load_open_visits(pool: &PgPool) -> Vec<DbVisit> {
    let rows = sqlx::query(
        "SELECT id, person_id, start_time, end_time, camera_id FROM person_visits \
         WHERE end_time = start_time ORDER BY person_id",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| DbVisit {
            id: r.get("id"),
            person_id: r.get("person_id"),
            start_time: r.get("start_time"),
            end_time: r.get("end_time"),
            camera_id: r.get("camera_id"),
        })
        .collect()
}

pub async fn load_person_visits(pool: &PgPool, person_id: i32) -> Vec<DbVisit> {
    let rows = sqlx::query(
        "SELECT id, person_id, start_time, end_time, camera_id FROM person_visits \
         WHERE person_id = $1 ORDER BY start_time",
    )
    .bind(person_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| DbVisit {
            id: r.get("id"),
            person_id: r.get("person_id"),
            start_time: r.get("start_time"),
            end_time: r.get("end_time"),
            camera_id: r.get("camera_id"),
        })
        .collect()
}

pub async fn close_stale_visits(pool: &PgPool, now: f64, stale_threshold: f64) {
    sqlx::query(
        "UPDATE person_visits SET end_time = $1 \
         WHERE end_time = start_time AND start_time < $2",
    )
    .bind(now)
    .bind(now - stale_threshold)
    .execute(pool)
    .await
    .unwrap();
}
