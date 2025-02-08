use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{domains::task_store::{TaskError, TaskStore}, models::task::Task};


pub struct SqliteTaskStore {
    conn: SqlitePool
}

impl SqliteTaskStore {
    pub fn new(conn: SqlitePool) -> Self {
        Self { conn }
    }
}

#[async_trait::async_trait]
impl TaskStore for SqliteTaskStore {
    async fn add_task(&mut self, task: Task) -> Result<(), TaskError> {
        let id = task.id.to_string();
        let peer_id = task.get_peer_id().to_base58();
        let job_id = task.job_id.to_string();
        let blend_file_name = task.blend_file_name.to_str().unwrap().to_string();
        let blender_version = task.blender_version.to_string();
        let range = serde_json::to_string(&task.range).unwrap();
        let _ = sqlx::query(r"INSERT INTO tasks(id, peer_id, job_id, blend_file_name, blender_version, range) 
            VALUES($1, $2, $3, $4, $5, $6)")
        .bind(id)
        .bind(peer_id)
        .bind(job_id)
        .bind(blend_file_name)
        .bind(blender_version)
        .bind(range)
        .execute(&self.conn);
        Ok(())
    }

    async fn poll_task(&mut self) -> Result<Task, TaskError> {
        todo!("poll pending task?");
    }
    
    async fn delete_task(&mut self, task: Task) -> Result<(), TaskError> {
        let id = task.id.to_string();
        let _ = sqlx::query(r"DELETE * FROM tasks WHERE id = $1")
            .bind(id)
            .execute(&self.conn).await;
        Ok(())
    }
    
    async fn delete_job_task(&mut self, job_id: Uuid) -> Result<(), TaskError> {
        let _ = sqlx::query(r"DELETE * FROM tasks WHERE job_id = $1")
            .bind(job_id.to_string())
            .execute(&self.conn).await;
        Ok(())
    }
}