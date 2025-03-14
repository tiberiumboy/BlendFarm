use std::str::FromStr;

use crate::{
    domains::worker_store::WorkerStore,
    models::{
        computer_spec::ComputerSpec,
        worker::{Worker, WorkerError},
    },
};
use libp2p::PeerId;
use serde::Deserialize;
use sqlx::{ query, query_as, SqlitePool};

pub struct SqliteWorkerStore {
    conn: SqlitePool,
}

impl SqliteWorkerStore {
    pub fn new(conn: SqlitePool) -> Self {
        Self { conn }
    }
}

#[derive(Debug, Deserialize, sqlx::FromRow)]
struct WorkerDb {
    machine_id: String,
    spec: Vec<u8>,
}

impl WorkerDb {
    pub fn new(worker: &Worker) -> WorkerDb {
        let machine_id = worker.machine_id.to_base58();
        // TODO: Fix the unwrap()
        let spec = serde_json::to_string(&worker.spec).unwrap().into_bytes();
        WorkerDb { machine_id, spec }
    }

    pub fn from(&self) -> Worker {
        // TODO: remove clone and unwrap functions
        let machine_id = PeerId::from_str(&self.machine_id).unwrap();
        let data = String::from_utf8(self.spec.clone()).unwrap();
        let spec = serde_json::from_str::<ComputerSpec>(&data).unwrap();
        Worker::new(machine_id, spec)
    }
}

#[async_trait::async_trait]
impl WorkerStore for SqliteWorkerStore {
    // List
    async fn list_worker(&self) -> Result<Vec<Worker>, WorkerError> {
        // we'll add a limit here for now.
        let sql = r"SELECT machine_id, spec FROM workers LIMIT 255";
        sqlx::query_as(sql)
            .fetch_all(&self.conn)
            .await
            .map_err(|e| WorkerError::Database(e.to_string()))
            .and_then(|r: Vec<WorkerDb>| {
                Ok(r.into_iter()
                    .map(|r: WorkerDb| {
                        // TODO: Find a better way to handle the unwraps()
                        let spec: ComputerSpec = serde_json::from_str(&r.spec).unwrap();
                        let peer = PeerId::from_str(&r.machine_id).unwrap();
                        Worker::new(peer, spec)
                    })
                    .collect::<Vec<Worker>>())
            })
    }

    // Create
    async fn add_worker(&mut self, worker: Worker) -> Result<(), WorkerError> {
        let record = WorkerDb::new(&worker);
        if let Err(e) = sqlx::query(
            r"
            INSERT INTO workers (machine_id, spec)
            VALUES($1, $2);
        ",
        )
        .bind(record.machine_id)
        .bind(record.spec)
        .execute(&self.conn)
        .await
        {
            eprintln!("{e}");
        }

        Ok(())
    }

    // Read
    async fn get_worker(&self, id: &str) -> Option<Worker> {
        let row: WorkerDb = query_as::<_, WorkerDb>(
            r#"SELECT machine_id, spec FROM workers WHERE machine_id=$1"#,
            id
        )
        .fetch_one(&self.conn)
        .await.unwrap();
        
        // {
        //     Ok(record) => {
        // I'm a bit confused? where is Vec8 coming from?
        let worker = row.from();
        Some(worker)
        //     }
        //     Err(e) => {
        //         eprintln!("{:?}", e.to_string());
        //         return None;
        //     }
        // }
    }

    // no update?

    // Delete
    async fn delete_worker(&mut self, machine_id: &str) -> Result<(), WorkerError> {
        let _ = sqlx::query(r"DELETE FROM workers WHERE machine_id = $1")
            .bind(machine_id)
            .execute(&self.conn)
            .await;
        Ok(())
    }
}
