ALTER TABLE visits
ALTER timestamp DROP DEFAULT;

ALTER TABLE visits
RENAME COLUMN timestamp TO registered_at;
