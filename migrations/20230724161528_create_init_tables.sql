CREATE TABLE IF NOT EXISTS paths (
    id bigserial PRIMARY KEY,
    path text NOT NULL
);

CREATE TABLE IF NOT EXISTS calls (
    id bigserial PRIMARY KEY,
    path_id bigserial NOT NULL REFERENCES paths,
    timestamp timestamp NOT NULL DEFAULT LOCALTIMESTAMP(0)
);
