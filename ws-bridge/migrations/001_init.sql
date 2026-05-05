CREATE TABLE IF NOT EXISTS persons (
    id          SERIAL PRIMARY KEY,
    first_seen  DOUBLE PRECISION NOT NULL,
    last_seen   DOUBLE PRECISION NOT NULL,
    total_seen  DOUBLE PRECISION NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS person_visits (
    id          BIGSERIAL PRIMARY KEY,
    person_id   INTEGER NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    camera_id   VARCHAR(50) NOT NULL DEFAULT 'cam_0',
    start_time  DOUBLE PRECISION NOT NULL,
    end_time    DOUBLE PRECISION NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_visits_person ON person_visits(person_id);

CREATE TABLE IF NOT EXISTS face_embeddings (
    id          BIGSERIAL PRIMARY KEY,
    person_id   INTEGER NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    embedding   DOUBLE PRECISION[] NOT NULL,
    face_jpeg   BYTEA,
    captured_at DOUBLE PRECISION NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_embeddings_person ON face_embeddings(person_id);
