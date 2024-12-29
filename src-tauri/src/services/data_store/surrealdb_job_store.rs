use crate::{
    domains::job_store::{JobError, JobStore},
    models::job::Job,
};
use std::sync::Arc;
use surrealdb::{engine::local::Db, opt::Resource, Surreal};
use tokio::sync::RwLock;
use uuid::Uuid;

const JOB_TABLE_NAME: &str = "jobs";

pub struct SurrealDbJobStore {
    pub conn: Arc<RwLock<Surreal<Db>>>,
}

impl SurrealDbJobStore {
    pub async fn new(conn: Arc<RwLock<Surreal<Db>>>) -> Self {
        {
            let db = conn.write().await;
            db.query(
                /*
                   pub id: Uuid,
                   pub mode: Mode,
                   project_file: PathBuf,
                   blender_version: Version,
                   output: PathBuf,
                   renders: HashMap<Frame, PathBuf>,
                */
                r#"
                DEFINE TABLE IF NOT EXISTS job SCHEMALESS;
                DEFINE FIELD IF NOT EXISTS id ON TABLE job TYPE uuid;
                DEFINE FIELD IF NOT EXISTS mode ON TABLE job FLEXIBLE TYPE object;
                DEFINE FIELD IF NOT EXISTS project_file ON TABLE job TYPE string;
                DEFINE FIELD IF NOT EXISTS blender_version ON TABLE job TYPE string;
                DEFINE FIELD IF NOT EXISTS renders ON TABLE job FLEXIBLE TYPE object;
                "#,
            )
            .await
            .expect("Should have permission to check for database schema");
        }
        Self { conn }
    }
}

#[async_trait::async_trait]
impl JobStore for SurrealDbJobStore {
    async fn add_job(&mut self, job: Job) -> Result<(), JobError> {
        let db = self.conn.write().await;
        let _ = db
            .create(Resource::from((JOB_TABLE_NAME, job.id.to_string())))
            .content(job)
            .await
            .map_err(|e| JobError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn update_job(&mut self, job: Job) -> Result<(), JobError> {
        let db = self.conn.write().await;
        let id = job.as_ref().to_string();
        let entry: Option<Job> = db
            .update((JOB_TABLE_NAME, id))
            .merge(job)
            .await
            .map_err(|e| JobError::DatabaseError(e.to_string()))?;
        match entry {
            Some(_) => Ok(()),
            None => Err(JobError::DatabaseError(
                "Unable to update job! Maybe a mismatch id somewhere?".to_owned(),
            )),
        }
    }

    async fn list_all(&self) -> Result<Vec<Job>, JobError> {
        let db = self.conn.read().await;
        let entry: Vec<Job> = db
            .select(JOB_TABLE_NAME)
            .await
            .map_err(|e| JobError::DatabaseError(e.to_string()))?;
        Ok(entry)
    }

    async fn delete_job(&mut self, id: Uuid) -> Result<(), JobError> {
        let db = self.conn.write().await;
        let entry: Option<Job> = db
            .delete((JOB_TABLE_NAME, id.to_string()))
            .await
            .map_err(|e| JobError::DatabaseError(e.to_string()))?; // TODO: find out the code to delete specific person name.

        // TODO: Find out how I can handle this? What does None means?
        match entry {
            Some(_) => Ok(()),
            None => Err(JobError::DatabaseError(
                "Fail to delete job? Why?".to_owned(),
            )),
        }
    }
}
