-- Add up migration script here
CREATE TABLE IF NOT EXISTS workers (
    machine_id TEXT NOT NULL PRIMARY KEY,
    spec BLOB NOT NULL
);