-- Add up migration script here
CREATE TABLE IF NOT EXISTS renders(
    -- should be jobs_id + _ + frame number
    id TEXT NOT NULL PRIMARY KEY,     
    jobs_id TEXT NOT NULL,
    frame INTEGER NOT NULL,
    render_path TEXT NOT NULL
);