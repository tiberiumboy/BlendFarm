CREATE TABLE IF NOT EXISTS jobs(
    id TEXT NOT NULL,
    mode TEXT NOT NULL,
    project_file TEXT NOT NULL,
    blender_version TEXT NOT NULL,
    output_path TEXT NOT NULL,
    PRIMARY KEY (id)
);