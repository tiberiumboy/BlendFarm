use std::{ops::Range, path::PathBuf, str::FromStr};

use sqlx::{Row, SqlitePool};
use semver::Version;
use uuid::Uuid;

use crate::{
    domains::task_store::{TaskError, TaskStore},
    models::task::{CreatedTaskDto, NewTaskDto, Task},
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
    async fn add_task(&self, task: NewTaskDto) -> Result<CreatedTaskDto, TaskError> {
        let id = Uuid::new_v4();
        let host = &task.requestor;
        let job_id = &task.job_id.to_string();
        let blend_file_name = &task.blend_file_name.to_str().unwrap().to_string();
        let blender_version = &task.blender_version.to_string();
        let start = &task.range.start;
        let end = &task.range.end;
        if let Err(e) = sqlx::query(
            r"INSERT INTO tasks(id, requestor, job_id, blend_file_name, blender_version, start_frame, end_frame) 
            VALUES($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id.to_string())
        .bind(host)
        .bind(job_id)
        .bind(blend_file_name)
        .bind(blender_version)
        .bind(start)
        .bind(end)
        .execute(&self.conn).await {
            eprintln!("Fail to add Task to database! {e:?}");
        }
        
        Ok(CreatedTaskDto { id, item: task })
    }

    // TODO: Clarify definition here?
    async fn poll_task(&self) -> Result<CreatedTaskDto, TaskError> {
        // the idea behind this is to get any pending task.
        let result = sqlx::query(
                r"SELECT id, requestor, job_id, blend_file_name, blender_version, start_frame, end_frame FROM tasks LIMIT 1")
            .fetch_all(&self.conn).await.map_err(|e| TaskError::DatabaseError(e.to_string()))?;

        for(_, row) in result.iter().enumerate() {
            let id = Uuid::from_str(&row.get::<String, &str>("id")).expect("ID cannot be null!");
            let requestor = row.get::<String, &str>("requestor");
            let job_id = Uuid::from_str(&row.get::<String, &str>("job_id")).expect("Job ID cannot be null!");
            let blend_file_name = PathBuf::from_str( &row.get::<String, &str>("blend_file_name")).expect("Must have valid file name!");
            let blender_version = Version::from_str(&row.get::<String, &str>("blender_version")).expect("Must have valid target blender version!");
            let start_frame = row.get::<i32, &str>("start_frame");
            let end_frame = row.get::<i32, &str>("end_frame");
            
            let range = Range { start: start_frame, end: end_frame };
            let task = Task::new(requestor, job_id, blend_file_name, blender_version, range);
            return Ok( CreatedTaskDto { id, item: task } );
        };

        Err(TaskError::DatabaseError("None found".to_owned()))
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
