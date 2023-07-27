CREATE TABLE IF NOT EXISTS paths (
    id bigserial PRIMARY KEY,
    path text NOT NULL
);

CREATE TABLE IF NOT EXISTS calls (
    id bigserial PRIMARY KEY,
    path_id bigserial NOT NULL REFERENCES paths,
    timestamp timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP(0)
);
