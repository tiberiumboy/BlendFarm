use crate::{
    domains::worker_store::WorkerStore,
    models::worker::{Worker, WorkerError},
};
use std::sync::Arc;
use surrealdb::{engine::local::Db, Surreal};
use tokio::sync::RwLock;
use uuid::Uuid;

const WORKER_TABLE_NAME: &str = "workers";

pub struct SurrealDbWorkerStore {
    conn: Arc<RwLock<Surreal<Db>>>,
}

impl SurrealDbWorkerStore {
    pub async fn new(conn: Arc<RwLock<Surreal<Db>>>) -> Self {
        {
            let db = conn.write().await;
            db.query(
                /*
                    machine_id: String,
                    spec: ComputerSpec,
                */
                r#"
                DEFINE TABLE IF NOT EXISTS worker SCHEMALESS;
                DEFINE FIELD IF NOT EXISTS machine_id ON TABLE worker TYPE string;
                DEFINE FIELD IF NOT EXISTS spec ON TABLE worker FLEXIBLE TYPE object;
                "#,
            )
            .await
            .expect("Fail to create worker schema");
        }
        Self { conn }
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
            None => Err(WorkerError::Database(
                "Fail to add worker to database!".to_owned(),
            )),
        }
    }

    async fn list_worker(&self) -> Result<Vec<Worker>, WorkerError> {
        let db = self.conn.read().await;
        let result: Vec<Worker> = db
            .select(WORKER_TABLE_NAME)
            .await
            .map_err(|e| WorkerError::Database(e.to_string()))?;

        // TODO: Find a way to parse the data here?
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
            None => Err(WorkerError::Database(
                "Fail to delete worker from database!".to_owned(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use machine_info::Machine;

    use crate::models::{computer_spec::ComputerSpec, worker::Worker};

    #[tokio::test]
    async fn should_pass() {
        let mut machine = Machine::new();
        let dummyspec = ComputerSpec::new(&mut machine);
        let dummyworker = Worker::new("test".to_owned(), dummyspec);
        // let db = Surreal<Db> =
    }
}
