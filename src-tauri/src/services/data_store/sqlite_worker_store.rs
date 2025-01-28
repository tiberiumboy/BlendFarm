use crate::{
    domains::worker_store::WorkerStore,
    models::{
        computer_spec::ComputerSpec,
        worker::{Worker, WorkerError},
    },
};
use sqlx::{prelude::FromRow, SqlitePool};

pub struct SqliteWorkerStore {
    conn: SqlitePool,
}

impl SqliteWorkerStore {
    pub fn new(conn: SqlitePool) -> Self {
        Self { conn }
    }
}

#[derive(FromRow)]
struct WorkerRecord {
    machine_id: String,
    spec: String,
}

#[async_trait::async_trait]
impl WorkerStore for SqliteWorkerStore {
    async fn add_worker(&mut self, worker: Worker) -> Result<(), WorkerError> {
        let spec = serde_json::to_string(&worker.spec).unwrap();

        if let Err(e) = sqlx::query(
            r"
            INSERT INTO workers (machine_id, spec)
            VALUES($1, $2);
        ",
        )
        .bind(worker.machine_id)
        .bind(spec)
        .execute(&self.conn)
        .await
        .map_err(|e| WorkerError::Database(e.to_string()))
        {
            eprintln!("{e}");
        }

        Ok(())
    }

    async fn list_worker(&self) -> Result<Vec<Worker>, WorkerError> {
        // we'll add a limit here for now.
        let sql = r"SELECT machine_id, spec FROM workers LIMIT 255";
        sqlx::query_as(sql)
            .fetch_all(&self.conn)
            .await
            .map_err(|e| WorkerError::Database(e.to_string()))
            .and_then(|r: Vec<WorkerRecord>| {
                Ok(r.into_iter()
                    .map(|r: WorkerRecord| {
                        let spec: ComputerSpec = serde_json::from_str(&r.spec).unwrap();
                        Worker::new(r.machine_id, spec)
                    })
                    .collect::<Vec<Worker>>())
            })
        // result.map_err(|e| WorkerError::Database(e.to_string()))
    }

    async fn delete_worker(&mut self, machine_id: &str) -> Result<(), WorkerError> {
        let _ = sqlx::query(r"DELETE * FROM workers WHERE machine_id = $1")
            .bind(machine_id.to_string())
            .execute(&self.conn)
            .await;
        Ok(())
    }
}
