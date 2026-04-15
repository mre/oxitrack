-- SQLite-compatible equivalent of the add_time_s migration.
-- Adds the time_s column and drops the now-unused left_at column.
--
-- SQLite does not support EXTRACT / EPOCH arithmetic in SQL, so we cannot
-- back-fill time_s from left_at the way the PostgreSQL migration does.
-- In practice this only matters when migrating an existing SQLite database
-- that already has left_at data; for a fresh install the column is always
-- empty anyway.

ALTER TABLE visits ADD COLUMN time_s INTEGER;
ALTER TABLE visits DROP COLUMN left_at;