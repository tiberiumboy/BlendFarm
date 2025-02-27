-- Add up migration script here
CREATE TABLE IF NOT EXISTS tasks(
    id TEXT NOT NULL PRIMARY KEY,
    requestor TEXT NOT NULL,
    job_id TEXT NOT NULL,
    blender_version TEXT NOT NULL,
    blend_file_name TEXT NOT NULL,
    start_frame INTEGER NOT NULL,
    end_frame INTEGER NOT NULL
);