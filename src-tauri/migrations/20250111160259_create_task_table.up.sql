-- Add up migration script here
CREATE TABLE IF NOT EXISTS tasks(
    id TEXT NOT NULL PRIMARY KEY,
    job_id TEXT NOT NULL,
    blender_version TEXT NOT NULL,
    blend_file_name TEXT NOT NULL,
    start INTEGER NOT NULL,
    end INTEGER NOT NULL
);