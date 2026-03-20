CREATE TABLE IF NOT EXISTS renders(
    id TEXT NOT NULL,     
    job_id TEXT NOT NULL,
    frame INTEGER NOT NULL,
    render_path TEXT NOT NULL,
    PRIMARY KEY (id)
    UNIQUE (job_id, frame)
);