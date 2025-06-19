use std::str::FromStr;

use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::{
    domains::worker_store::WorkerStore,
    models::{
        computer_spec::ComputerSpec,
        worker::{Worker, WorkerError},
    },
};

pub struct SqliteWorkerStore {
    conn: SqlitePool,
}

#[derive(FromRow, Serialize, Deserialize, Debug)]
struct WorkerDTO {
    machine_id: String,
    // TODO: find a way to use #[sqlx(json)]?
    spec: String, // deserialize/serialize as json
}

impl WorkerDTO {
    pub fn dto_to_obj(&self) -> Worker {
        let id = PeerId::from_str(&self.machine_id).expect("ID was mutated!");
        let spec = serde_json::from_str::<ComputerSpec>(&self.spec).expect("spec was mutated!");
        Worker { id, spec }
    }
}

impl SqliteWorkerStore {
    pub fn new(conn: SqlitePool) -> Self {
        Self { conn }
    }
}

#[async_trait::async_trait]
impl WorkerStore for SqliteWorkerStore {
    // List
    async fn list_worker(&self) -> Result<Vec<Worker>, WorkerError> {
        // we'll add a limit here for now.
        let result: Vec<WorkerDTO> =
            sqlx::query_as!(WorkerDTO, r"SELECT machine_id, spec FROM workers")
                .fetch_all(&self.conn)
                .await
                .map_err(|e| WorkerError::Database(e.to_string()))?;

        Ok(result.iter().map(|e| e.dto_to_obj()).collect())
    }

    // Create
    async fn add_worker(&mut self, worker: Worker) -> Result<(), WorkerError> {
        let id = worker.id.to_base58();
        let spec = serde_json::to_string(&worker.spec).expect("Fail to parse specs");
        if let Err(e) = sqlx::query(
            r"
            INSERT INTO workers (machine_id, spec)
            VALUES($1, $2);
        ",
        )
        .bind(id)
        .bind(spec)
        .execute(&self.conn)
        .await
        {
            eprintln!("Fail to insert new worker: {e}");
        }

        Ok(())
    }

    // Read
    async fn get_worker(&self, id: &PeerId) -> Option<Worker> {
        // so this panic when there's no record?
        let peer_id = id.to_base58();
        let result: Result<WorkerDTO, sqlx::Error> = sqlx::query_as!(
            WorkerDTO,
            r#"SELECT machine_id, spec FROM workers WHERE machine_id=$1"#,
            peer_id
        )
        .fetch_one(&self.conn)
        .await;

        match result {
            Ok(data) => Some(data.dto_to_obj()),
            Err(e) => {
                eprintln!("SQLx generated an error: {e:?}");
                None
            }
        }
    }

    // no update?

    // Delete
    async fn delete_worker(&mut self, id: &PeerId) -> Result<(), WorkerError> {
        let peer_id = id.to_base58();
        let _ = sqlx::query(r"DELETE FROM workers WHERE machine_id = $1")
            .bind(peer_id)
            .execute(&self.conn)
            .await;
        Ok(())
    }

    // Clear worker table
    async fn clear_worker(&mut self) -> Result<(), WorkerError> {
        let _ = sqlx::query(r"DELETE FROM workers")
            .execute(&self.conn)
            .await
            .map_err(|e| WorkerError::Database(e.to_string()))?;
        Ok(())
    }
}
