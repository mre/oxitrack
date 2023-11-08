ALTER TABLE IF EXISTS referrers
RENAME CONSTRAINT referrers_domain_key TO unique_domain;

ALTER TABLE IF EXISTS visits
RENAME CONSTRAINT calls_pkey TO visits_pkey;

ALTER TABLE IF EXISTS visits
RENAME CONSTRAINT calls_path_id_fkey TO visits_path_id_fkey;

ALTER SEQUENCE IF EXISTS calls_id_seq RENAME TO visits_id_seq;

CREATE TABLE IF NOT EXISTS paths (
    id bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    path text NOT NULL CONSTRAINT unique_path UNIQUE
);

CREATE TABLE IF NOT EXISTS referrers (
    id bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    domain varchar(255) NOT NULL CONSTRAINT unique_domain UNIQUE
);

CREATE TABLE IF NOT EXISTS visits (
    id bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    path_id bigint NOT NULL REFERENCES paths,
    registered_at timestamptz NOT NULL,
    referrer_id bigint REFERENCES referrers,
    left_at timestamptz
);
