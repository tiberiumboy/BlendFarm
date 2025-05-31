use sqlx::{query_as, SqlitePool};
use uuid::Uuid;

use crate::{
    domains::task_store::{TaskError, TaskStore},
    models::task::Task,
};

pub struct SqliteTaskStore {
    conn: SqlitePool,
}

impl SqliteTaskStore {
    pub fn new(conn: SqlitePool) -> Self {
        Self { conn }
    }
}

#[async_trait::async_trait]
impl TaskStore for SqliteTaskStore {
    async fn add_task(&self, task: Task) -> Result<(), TaskError> {
        let  sql = r"INSERT INTO tasks(id, requestor, job_id, blend_file_name, blender_version, start, end) 
            VALUES($1, $2, $3, $4, $5, $6, $7)";
        
        let _ = sqlx::query( sql )
            .bind(Uuid::new_v4().to_string())
            .bind(task.requestor)
            .bind(task.job_id)
            .bind(task.blend_file_name.to_str())
            .bind(task.blender_version)
            .bind(task.range.start)
            .bind(task.range.end)
            .execute(&self.conn).await.map_err(|e| TaskError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }

    // TODO: Clarify definition here?
    async fn poll_task(&self) -> Result<Task, TaskError> {
        // the idea behind this is to get any pending task.
        let sql = r"SELECT id, requestor, job_id, blend_file_name, blender_version, start, end FROM tasks LIMIT 1";
        let result: Task = query_as(sql)
            .fetch_one(&self.conn)
            .await
            .map_err(|e| TaskError::DatabaseError(e.to_string()))?;
        
        Ok(result)
    }

    async fn list_tasks(&self) -> Result<Option<Vec<Task>>, TaskError> {
        let sql = r"SELECT id, requestor, job_id, blend_file_name, blender_version, start, end FROM tasks LIMIT 10";

        let result: Vec<Task> = sqlx::query_as(sql).fetch_all(&self.conn)
            .await
            .map_err(|e| TaskError::DatabaseError(e.to_string()))?;

        Ok(Some(result))
    }

    async fn delete_task(&self, id: &Uuid) -> Result<(), TaskError> {
        let _ = sqlx::query(r"DELETE * FROM tasks WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.conn)
            .await;
        Ok(())
    }

    async fn delete_job_task(&self, job_id: &Uuid) -> Result<(), TaskError> {
        let _ = sqlx::query(r"DELETE FROM tasks WHERE job_id = $1")
            .bind(job_id.to_string())
            .execute(&self.conn)
            .await;
        Ok(())
    }
}
