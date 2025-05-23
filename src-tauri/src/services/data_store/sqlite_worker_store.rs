use sqlx::{query_as, SqlitePool};

use crate::{domains::worker_store::WorkerStore, models::{computer_spec::ComputerSpec, job::CreatedJobDto, network::PeerIdString, worker::{self, Worker, WorkerError}}};

pub struct SqliteWorkerStore {
    conn: SqlitePool,
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
        let sql = r"SELECT spec, machine_id FROM workers LIMIT 255";
        let result: Result<Vec<Worker>, sqlx::Error> = sqlx::query_as(sql)
            .fetch_all(&self.conn)
            .await
            .map_err(|e| WorkerError::Database(e.to_string()));
        
        result
    }

    // Create
    async fn add_worker(&mut self, worker: Worker) -> Result<(), WorkerError> {
        if let Err(e) = sqlx::query(
            r"
            INSERT INTO workers (machine_id, spec)
            VALUES($1, $2);
        ",
        )
        .bind(worker.id)
        .bind(worker.item)
        .execute(&self.conn)
        .await
        {
            eprintln!("Fail to insert new worker: {e}");
        }

        Ok(())
    }

    // Read
    async fn get_worker(&self, id: &PeerIdString) -> Option<Worker> {
        // so this panic when there's no record?
        let sql = r#"SELECT machine_id AS id, spec AS item FROM workers WHERE machine_id=$1"#;
        let result: Result<Worker, sqlx::Error> = query_as::<_, Worker>(sql)
            .bind(id)
            .fetch_one(&self.conn)
            .await;
        
        result.ok()
    }

    // no update?

    // Delete
    async fn delete_worker(&mut self, machine_id: &PeerIdString) -> Result<(), WorkerError> {
        let _ = sqlx::query(r"DELETE FROM workers WHERE machine_id = $1")
            .bind(machine_id.inner)
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
