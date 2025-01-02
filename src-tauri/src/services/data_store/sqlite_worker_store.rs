use std::sync::Arc;
use sqlx::{Pool, Sqlite};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{domains::worker_store::WorkerStore, models::worker::{Worker, WorkerError}};

pub struct SqliteWorkerStore {
    conn : Arc<RwLock<Pool<Sqlite>>>,
}

impl SqliteWorkerStore {
    pub fn new(conn: Arc<RwLock<Pool<Sqlite>>>) -> Result<Self, WorkerError> {
        Ok( Self { conn } )
    }
}

#[async_trait::async_trait]
impl WorkerStore for SqliteWorkerStore {
    async fn add_worker(&mut self, worker: Worker) -> Result<(), WorkerError> {
        Ok(())
    }

    async fn list_worker(&self) -> Result<Vec<Worker>, WorkerError> {
        todo!("get the list of worker here");
    }
    
    async fn delete_worker(&mut self, id: Uuid) -> Result<(), WorkerError> {
        Ok(())
    }
}