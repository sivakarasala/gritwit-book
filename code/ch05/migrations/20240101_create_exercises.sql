-- Chapter 5: First migration — create exercises table

CREATE TABLE exercises (
    id          SERIAL PRIMARY KEY,
    name        TEXT NOT NULL,
    category    TEXT NOT NULL,
    scoring_type TEXT NOT NULL,
    created_by  INTEGER,
    deleted_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_exercises_category ON exercises(category);
CREATE INDEX idx_exercises_deleted_at ON exercises(deleted_at) WHERE deleted_at IS NULL;
