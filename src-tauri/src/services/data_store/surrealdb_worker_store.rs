use crate::{domains::worker_store::WorkerStore, models::worker::{Worker, WorkerError}};
use std::sync::Arc;
use surrealdb::{engine::local::Db, Surreal};
use tokio::sync::RwLock;
use uuid::Uuid;

const WORKER_TABLE_NAME: &str = "workers";

pub struct SurrealDbWorkerStore {
    conn: Arc<RwLock<Surreal<Db>>>,
}

impl SurrealDbWorkerStore {
    pub fn new(connection: Arc<RwLock<Surreal<Db>>>) -> Self {
        Self { conn: connection }
    }
}

#[async_trait::async_trait]
impl WorkerStore for SurrealDbWorkerStore {
    async fn add_worker(&mut self, worker: Worker) -> Result<(), WorkerError> {
        let db = self.conn.write().await;
        let result: Option<Worker> = db
            .create(WORKER_TABLE_NAME)
            .content(worker)
            .await
            .map_err(|e| WorkerError::Database(e.to_string()))?;
        match result {
            Some(_) => Ok(()),
            None => Err(WorkerError::Database("Fail to add worker to database!".to_owned())) 
        }
    }

    async fn list_worker(&self) -> Result<Vec<Worker>, WorkerError> {
        let db = self.conn.read().await;
        let result: Vec<Worker> = db
            .select(WORKER_TABLE_NAME)
            .await
            .map_err(|e| WorkerError::Database(e.to_string()))?;
        Ok(result)
    }

    async fn delete_worker(&mut self, id: Uuid) -> Result<(), WorkerError> {
        let db = self.conn.write().await;
        let result: Option<Worker> = db
            .delete((WORKER_TABLE_NAME, id.to_string()))
            .await
            .map_err(|e| WorkerError::Database(e.to_string()))?;
        match result {
            Some(_) => Ok(()),
            None => Err(WorkerError::Database("Fail to delete worker from database!".to_owned())) 
        }
    }
}