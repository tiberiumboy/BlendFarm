use std::sync::Arc;
use sqlx::{Pool, Sqlite};
use tauri_plugin_sql::{Migration, MigrationKind};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{domains::task_store::{TaskError, TaskStore}, models::task::Task};


pub struct SqliteTaskStore {
    conn: Arc<RwLock<Pool<Sqlite>>>
}

impl SqliteTaskStore {

    pub fn get_migration() -> Migration {
        Migration {
            version: 1,
            description: "create_task_table",
            sql: "CREATE TABLE IF NOT EXISTS dbo.tasks (id TEXT, peer_id TEXT, job_id TEXT, blender_version TEXT, range TEXT)",
            kind: MigrationKind::Up
        }
    }
    pub async fn new(conn: Arc<RwLock<Pool<Sqlite>>>) -> Result<Self, sqlx::Error> {
        Ok( Self { conn } )
    }
}

#[async_trait::async_trait]
impl TaskStore for SqliteTaskStore {
    async fn add_task(&mut self, task: Task) -> Result<(), TaskError> {
        let conn = self.conn.write().await;
        let mut pool = conn.acquire().await.unwrap().detach();
        let id = task.id.to_string();
        let peer_id = task.get_peer_id().to_base58();
        let job_id = task.job_id.to_string();
        let blend_file_name = task.blend_file_name.to_str().unwrap().to_string();
        let blender_version = task.blender_version.to_string();
        let range = serde_json::to_string(&task.range).unwrap();
        sqlx::query(r"INSERT INTO tasks(id, peer_id, job_id, blend_file_name, blender_version, range) 
            VALUES($1, $2, $3, $4, $5, $6)")
        .bind(id)
        .bind(peer_id)
        .bind(job_id)
        .bind(blend_file_name)
        .bind(blender_version)
        .bind(range)
        .execute(&mut pool);
        Ok(())
    }

    async fn poll_task(&mut self) -> Result<Task, TaskError> {
        todo!("poll pending task?");
    }
    
    async fn delete_task(&mut self, task: Task) -> Result<(), TaskError> {
        let conn = self.conn.write().await;
        let mut pool = conn.acquire().await.unwrap().detach();
        let id = task.id.to_string();
        sqlx::query(r"DELETE * FROM tasks WHERE id = $1").bind(task.id).execute(&mut pool).await;
        Ok(())
    }
    
    async fn delete_job_task(&mut self, job_id: Uuid) -> Result<(), TaskError> {
        let conn = self.conn.write().await;
        let mut pool = conn.acquire().await.unwrap().detach();
        sqlx::query(r"DELETE * FROM tasks WHERE job_id = $1").bind(job_id.to_string()).execute(&mut pool).await;
        Ok(())
    }
}