CREATE TABLE IF NOT EXISTS ticket(
    id TEXT NOT NULL,
    job_id TEXT NOT NULL,
    blend_path TEXT NOT NULL,
    blender_version TEXT NOT NULL,
    start INTEGER NOT NULL,
    end INTEGER NOT NULL,
    PRIMARY KEY (id)
);