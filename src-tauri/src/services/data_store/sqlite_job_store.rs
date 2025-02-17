use std::{path::PathBuf, str::FromStr};

use crate::{
    domains::job_store::{JobError, JobStore},
    models::job::Job,
};
use blender::models::mode::Mode;
use semver::Version;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

pub struct SqliteJobStore {
    conn: SqlitePool,
}

impl SqliteJobStore {
    pub fn new(conn: SqlitePool) -> Self {
        Self { conn }
    }
}

#[derive(FromRow)]
struct JobDb {
    id: String,
    mode: String,
    project_file: String,
    blender_version: String,
    output_path: String,
}

#[async_trait::async_trait]
impl JobStore for SqliteJobStore {
    async fn add_job(&mut self, job: Job) -> Result<(), JobError> {
        let id = job.id.to_string();
        let mode = serde_json::to_string(&job.mode).unwrap();
        let project_file = job.project_file.to_str().unwrap().to_owned();
        let blender_version = job.blender_version.to_string();
        let output = job.output.to_str().unwrap().to_owned();

        sqlx::query(
            r"
                INSERT INTO jobs (id, mode, project_file, blender_version, output_path)
                VALUES($1, $2, $3, $4, $5);
            ",
        )
        .bind(id)
        .bind(mode)
        .bind(project_file)
        .bind(blender_version)
        .bind(output)
        .execute(&self.conn)
        .await
        .map_err(|e| JobError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn get_job(&self, job_id: &Uuid) -> Result<Job, JobError> {
        let sql =
            "SELECT id, mode, project_file, blender_version, output_path FROM Jobs WHERE id=$1";
        match sqlx::query_as::<_, JobDb>(sql)
            .bind(job_id.to_string())
            .fetch_one(&self.conn)
            .await
        {
            Ok(r) => {
                let id = Uuid::parse_str(&r.id).unwrap();
                let mode: Mode = serde_json::from_str(&r.mode).unwrap();
                let project = PathBuf::from(r.project_file);
                let version = Version::from_str(&r.blender_version).unwrap();
                let output = PathBuf::from(r.output_path);
                let job = Job::new(id, mode, project, version, output, Default::default());
                Ok(job)
            }
            Err(e) => Err(JobError::DatabaseError(e.to_string())),
        }
    }

    async fn update_job(&mut self, _job: Job) -> Result<(), JobError> {
        todo!("Update job to database");
    }

    async fn list_all(&self) -> Result<Vec<Job>, JobError> {
        let sql = r"SELECT id, mode, project_file, blender_version, output_path FROM jobs";
        let mut data: Vec<Job> = Vec::new();
        let results = sqlx::query_as::<_, JobDb>(sql).fetch_all(&self.conn).await;
        match results {
            Ok(records) => {
                for r in records {
                    let id = Uuid::parse_str(&r.id).unwrap();
                    let mode: Mode = serde_json::from_str(&r.mode).unwrap();
                    let project = PathBuf::from(r.project_file);
                    let version = Version::from_str(&r.blender_version).unwrap();
                    let output = PathBuf::from(r.output_path);
                    let job = Job::new(id, mode, project, version, output, Default::default());
                    data.push(job);
                }
            }
            Err(e) => return Err(JobError::DatabaseError(e.to_string())),
        }
        Ok(data)
    }

    async fn delete_job(&mut self, id: &Uuid) -> Result<(), JobError> {
        if let Err(e) = sqlx::query("DELETE FROM jobs WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.conn)
            .await
        {
            eprintln!("Fail to delete job! {e:?}");
        }
        Ok(())
    }
}
