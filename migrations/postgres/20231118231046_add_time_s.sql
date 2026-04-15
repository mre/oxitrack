ALTER TABLE visits
ADD time_s int;

UPDATE visits
SET time_s = ROUND(EXTRACT(EPOCH FROM left_at - registered_at));

ALTER TABLE visits
DROP left_at;
