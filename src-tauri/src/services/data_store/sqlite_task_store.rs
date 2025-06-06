use sqlx::{types::Uuid, SqlitePool, FromRow};
use std::{ops::Range, path::PathBuf, str::FromStr};
use semver::Version;
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


#[derive(Debug, Clone, FromRow)]
struct TaskDAO {
    id: String,
    requestor: String,
    job_id: String,
    blender_version: String,
    blend_file_name: String,
    start: i64,
    end: i64
}

impl TaskDAO {    
    fn dto_to_task(self) -> Task {
        Task {
            id: Uuid::from_str(&self.id).expect("id was mutated"),
            requestor: self.requestor,
            job_id: Uuid::from_str(&self.job_id).expect("job_id was mutated"),
            blender_version: Version::from_str(&self.blender_version).expect("version was mutated"),
            blend_file_name: PathBuf::from_str(&self.blend_file_name).expect("file name was mutated"),
            range: Range { start: self.start as i32, end: self.end as i32 }
        }        
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
            .bind(task.blender_version.to_string())
            .bind(task.range.start)
            .bind(task.range.end)
            .execute(&self.conn).await.map_err(|e| TaskError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }

    // Poll next available task if there any.
    async fn poll_task(&self) -> Result<Option<Task>, TaskError> {
        // the idea behind this is to get any pending task.
        let query = sqlx::query_as!(TaskDAO, 
        r"
            SELECT id, requestor, job_id, blend_file_name, blender_version, start, end
            FROM tasks 
            LIMIT 1
        ");
        
        let result = query
            .fetch_optional(&self.conn)
            .await
            .map_err(|e| TaskError::DatabaseError(e.to_string()))?;
        
        match result {
            Some(data) => Ok(Some(data.dto_to_task())),
            None => Ok(None)
        }
    }

    async fn list_tasks(&self) -> Result<Option<Vec<Task>>, TaskError> {
        let result = sqlx::query_as!(TaskDAO, 
            r"
            SELECT id, requestor, job_id, blend_file_name, blender_version, start, end
            FROM tasks 
            LIMIT 10
        ").fetch_all(&self.conn)
            .await;
        
        match result {
            Ok(list) => Ok(Some(list.iter().map(|d| d.clone().dto_to_task()).collect())),
            Err(e) => Err(TaskError::DatabaseError(e.to_string()))
        }
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
