-- SQLite-compatible initial schema.
-- Equivalent to the PostgreSQL fresh_start migration but using SQLite syntax.
-- Notable differences:
--   - INTEGER PRIMARY KEY AUTOINCREMENT instead of bigint GENERATED ALWAYS AS IDENTITY
--   - TEXT for timestamps (sqlx stores OffsetDateTime / PrimitiveDateTime as ISO-8601 text)
--   - No RENAME CONSTRAINT (SQLite does not support it; names are set inline below)

CREATE TABLE IF NOT EXISTS paths (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL CONSTRAINT unique_path UNIQUE
);

CREATE TABLE IF NOT EXISTS referrers (
    id     INTEGER PRIMARY KEY AUTOINCREMENT,
    domain TEXT NOT NULL CONSTRAINT unique_domain UNIQUE
);

CREATE TABLE IF NOT EXISTS visits (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    path_id       INTEGER NOT NULL REFERENCES paths (id),
    registered_at TEXT    NOT NULL,
    referrer_id   INTEGER REFERENCES referrers (id),
    left_at       TEXT
);