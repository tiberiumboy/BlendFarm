use crate::{domains::job_store::{JobError, JobStore}, models::job::Job};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteJobStore {
    conn : SqlitePool
}

impl SqliteJobStore {
    pub fn new(conn: SqlitePool) -> Self {
        Self { conn }
    }
}

#[async_trait::async_trait]
impl JobStore for SqliteJobStore {
    async fn add_job(&mut self, job: Job) -> Result<(),JobError>  {
        let id = job.id.to_string();
        let mode = serde_json::to_string(&job.mode).unwrap();
        let project_file = job.project_file.to_str().unwrap().to_owned();
        let blender_version = job.blender_version.to_string();
        let output = job.output.to_str().unwrap().to_owned();

        sqlx::query(r"
                INSERT INTO jobs (id, mode, project_file, blender_version, output_path)
                VALUES($1, $2, $3, $4, $5);
            ")
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

    async fn update_job(&mut self, _job: Job) -> Result<(), JobError> {
        todo!("Update job to database");
    }

    async fn list_all(&self) -> Result<Vec<Job>, JobError> {
        // TODO: Find a better way to use this? I need pool from Pool<Sqlite> connection...?
        // let data = sqlx::query_as!( Job, r"SELECT * FROM jobs").fetch_all(&self.conn).await;
        let data = Vec::new();
        Ok(data)
    }

    async fn delete_job(&mut self, id: Uuid) -> Result<(), JobError> {
        let _ = sqlx::query("DELETE * FROM job WHERE id = $1").bind(id.to_string()).execute(&self.conn).await;
        Ok(())
    }
}