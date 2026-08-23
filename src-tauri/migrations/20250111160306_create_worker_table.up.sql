CREATE TABLE IF NOT EXISTS workers (
    peer_id TEXT NOT NULL,
    -- TODO: See how I can use sqlx::json for storage?
    spec TEXT NOT NULL,
    PRIMARY KEY (peer_id)
);