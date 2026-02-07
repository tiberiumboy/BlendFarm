-- Add up migration script here
CREATE TABLE IF NOT EXISTS renders(
    id TEXT NOT NULL PRIMARY KEY,     
    job_id TEXT NOT NULL,
    frame INTEGER NOT NULL,
    render_path TEXT NOT NULL
);