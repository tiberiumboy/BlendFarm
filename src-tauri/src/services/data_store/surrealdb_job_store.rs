use crate::{domains::job_store::{JobError, JobStore}, models::job::Job};
use surrealdb::{engine::local::Db, Surreal};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

const JOB_TABLE_NAME: &str = "jobs";

pub struct SurrealDbJobStore {
    pub conn: Arc<RwLock<Surreal<Db>>>,
}

impl SurrealDbJobStore {
    pub fn new(connection: Arc<RwLock<Surreal<Db>>>) -> Self {
        Self { conn: connection }
    }
}

#[async_trait::async_trait]
impl JobStore for SurrealDbJobStore {
    async fn add_job(&mut self, job: Job) -> Result<(), JobError> {
        let db = self.conn.write().await;
        let entry: Option<Job> = db
            .create(JOB_TABLE_NAME)
            .content(job)
            .await
            .map_err(|e| JobError::DatabaseError(e.to_string()))?;
        match entry {
            Some(_) => Ok(()),
            None => Err(JobError::DatabaseError("Unable to create new job entry!".to_owned()))
        }
    }

    async fn update_job(&mut self, job: Job) -> Result<(), JobError> {
        let db = self.conn.write().await;
        let id = job.as_ref().to_string();
        let entry: Option<Job> = db.update((JOB_TABLE_NAME, id))
            .merge(job)
            .await
            .map_err(|e| JobError::DatabaseError(e.to_string()))?;
        match entry {
            Some(_) => Ok(()),
            None => Err(JobError::DatabaseError("Unable to update job! Maybe a mismatch id somewhere?".to_owned())) 
        }
    }
    
    async fn list_all(&self) -> Result<Vec<Job>, JobError> {
        let db = self.conn.read().await;
        let entry: Vec<Job> = db
            .query(r#"SELECT * FROM task WHERE id = $record_id;"#)
            .bind(("record_id", self.))
            .await
            .map_err(|e| JobError::DatabaseError(e.to_string()))?;
        Ok(entry)
    }

    async fn delete_job(&mut self, id: Uuid) -> Result<(), JobError> {
        let db = self.conn.write().await;
        let entry: Option<Job> = db
        .delete((JOB_TABLE_NAME, id.to_string()))
        .await
        .map_err(|e| JobError::DatabaseError(e.to_string()))?;   // TODO: find out the code to delete specific person name.
    
        // TODO: Find out how I can handle this? What does None means?
        match entry {
            Some(_) => Ok(()),
            None => Err(JobError::DatabaseError("Fail to delete job? Why?".to_owned())) 
        }
    }
}