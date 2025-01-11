use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{domains::worker_store::WorkerStore, models::worker::{Worker, WorkerError}};

pub struct SqliteWorkerStore {
    conn : SqlitePool,
}

impl SqliteWorkerStore {
    pub fn new(conn: SqlitePool) -> Self {
        Self { conn }
    }
}

#[async_trait::async_trait]
impl WorkerStore for SqliteWorkerStore {
    async fn add_worker(&mut self, _worker: Worker) -> Result<(), WorkerError> {
        Ok(())
    }

    async fn list_worker(&self) -> Result<Vec<Worker>, WorkerError> {
        todo!("get the list of worker here");
    }
    
    async fn delete_worker(&mut self, id: Uuid) -> Result<(), WorkerError> {
        let _ = sqlx::query(r"DELETE * FROM workers WHERE machine_id = $1")
            .bind(id.to_string())
            .execute(&self.conn).await;
        Ok(())
    }
}