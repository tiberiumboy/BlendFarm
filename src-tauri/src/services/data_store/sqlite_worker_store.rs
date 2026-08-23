use std::str::FromStr;

use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, types::Json};

use crate::{
    domains::worker_store::{WorkerError, WorkerStore},
    models::{computer_spec::ComputerSpec, worker::Worker},
};

pub struct SqliteWorkerStore {
    conn: SqlitePool,
}

#[derive(FromRow, Serialize, Deserialize, Debug)]
struct WorkerDTO {
    peer_id: String,
    spec: Json<ComputerSpec>,
}

impl WorkerDTO {
    pub fn dto_to_obj(&self) -> Worker {
        Worker {
            // TODO: Should this be multi-address instead?
            peer_id: PeerId::from_str(&self.peer_id).expect("ID was mutated!"),
            spec: self.spec.0.clone(),
        }
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
        let mut result: Vec<WorkerDTO> = sqlx::query_as!(
            WorkerDTO,
            r#"SELECT peer_id, spec as "spec: Json<ComputerSpec>" FROM workers"#
        )
        .fetch_all(&self.conn)
        .await
        .map_err(|e| WorkerError::Database(e.to_string()))?;

        let list = result.iter_mut().fold(Vec::new(), |mut list, element| {
            list.push(element.dto_to_obj());
            list
        });

        Ok(list)
    }

    // Create
    async fn add_worker(&mut self, worker: Worker) -> Result<(), WorkerError> {
        let id = worker.peer_id.to_base58();
        // TODO replace this on using sqlx::json
        let spec = serde_json::to_string(&worker.spec).expect("Fail to parse specs");
        // TODO: Update the record if it exist by marking it status "Active", relearn SQL again?
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
        let peer_id = id.to_base58();
        // Is there a way I could do optional instead of result?
        let result: Result<WorkerDTO, sqlx::Error> = sqlx::query_as!(
            WorkerDTO,
            r#"SELECT peer_id, spec as "spec: Json<ComputerSpec>" FROM workers WHERE peer_id=$1"#,
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
        // TODO: mark the worker inactive instead.
        let _ = sqlx::query!(r"DELETE FROM workers WHERE peer_id = $1", peer_id)
            // my mind goes on a brainfart moment overcomplicating simplification and data requirement.
            // should status be a enum type, then should it be a string instead?
            // let _ = sqlx::query!("UPDATE workers SET status=false,  ")
            // .bind(peer_id)
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
