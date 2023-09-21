CREATE TABLE IF NOT EXISTS referrers (
    id bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    domain varchar(255) NOT NULL UNIQUE
);

ALTER TABLE visits
ADD referrer_id bigint REFERENCES referrers;
