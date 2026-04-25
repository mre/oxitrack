-- Persist visitor sessions so in-flight visitors survive a service restart.
-- The `sessions_ttl` trigger sweeps rows older than 1 hour on every INSERT,
-- so cleanup is automatic without a background task.

CREATE TABLE IF NOT EXISTS sessions (
    visitor_id    INTEGER PRIMARY KEY,
    path_id       INTEGER NOT NULL REFERENCES paths (id),
    registered_at TEXT    NOT NULL,
    visit_id      INTEGER REFERENCES visits (id)
);

CREATE INDEX IF NOT EXISTS idx_sessions_path_id
    ON sessions (path_id) WHERE visit_id IS NULL;

CREATE TRIGGER IF NOT EXISTS sessions_ttl
AFTER INSERT ON sessions
BEGIN
    DELETE FROM sessions
    WHERE datetime(registered_at) < datetime('now', '-1 hour');
END;