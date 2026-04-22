-- Speed up all time-range and per-path queries.
-- visits has 300k+ rows and previously had zero indexes beyond the PK.

-- Composite index: satisfies "WHERE path_id = ? AND registered_at >= ? AND < ?"
-- in one B-tree lookup.  Also covers plain path_id-only lookups.
CREATE INDEX IF NOT EXISTS idx_visits_path_reg
    ON visits (path_id, registered_at);

-- Standalone registered_at index: used when path_id IS NULL (dashboard all-paths queries)
-- and for ORDER BY registered_at LIMIT 1 (WholeDaysSinceFirstVisit).
CREATE INDEX IF NOT EXISTS idx_visits_registered_at
    ON visits (registered_at);

-- referrer_id index: speeds up the JOIN visits → referrers when filtering by date range.
CREATE INDEX IF NOT EXISTS idx_visits_referrer_id
    ON visits (referrer_id);