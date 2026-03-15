use crate::{
    domains::task_store::{TaskError, TaskStore},
    models::{
        job::Job,
        task::{CreatedTaskDto, Task},
        with_id::WithId,
    },
};
use sqlx::{FromRow, SqlitePool, types::Uuid};
use std::str::FromStr;

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
    job_id: String,
    job: String,
    start: i64,
    end: i64,
}

impl TaskDAO {
    fn dto_to_task(self) -> WithId<Task, Uuid> {
        let id = Uuid::from_str(&self.id).expect("id was mutated");
        let job_id = Uuid::from_str(&self.job_id).expect("job_id was mutated");
        let job = serde_json::from_str::<Job>(&self.job).expect("job record was malformed!");
        let start = self.start as i32;
        let end=  self.end as i32;

        // at this point here, we shouldn't have to worry about Job's original rendering mode,
        let job_record = WithId {
            id: job_id,
            item: job,
        };
        // TODO: Find a way to handle expect()
        let item = Task::from(job_record, start, end).expect("Malformed data detected!");
        WithId { id, item }
    }
}

#[async_trait::async_trait]
impl TaskStore for SqliteTaskStore {
    async fn add_task(&self, task: Task) -> Result<CreatedTaskDto, TaskError> {
        let sql = r"INSERT INTO tasks(id, job_id, job, start, end) 
            VALUES($1, $2, $3, $4, $5)";
        let id = Uuid::new_v4();
        let job = serde_json::to_string::<Job>(task.as_ref())
            .expect("Should be able to convert job into json");

        let job_id = AsRef::<Uuid>::as_ref(&task).to_string();
        let _ = sqlx::query(sql)
            .bind(id.to_string())
            .bind(job_id)
            .bind(job)
            .bind(&task.start)
            .bind(&task.end)
            .execute(&self.conn)
            .await
            .map_err(|e| TaskError::DatabaseError(e.to_string()))?;

        Ok(WithId { id, item: task })
    }

    // Poll next available task if there any.
    async fn poll_task(&self) -> Result<Option<CreatedTaskDto>, TaskError> {
        // fetch next available task to work on
        // TODO: Implement creation date to order by
        let query = sqlx::query_as!(
            TaskDAO,
            r"
            SELECT id, job_id, job, start, end
            FROM tasks 
            LIMIT 1
            "
        );

        let result = query
            .fetch_optional(&self.conn)
            .await
            .map_err(|e| TaskError::DatabaseError(e.to_string()))?;

        match result {
            Some(data) => Ok(Some(data.dto_to_task())),
            None => Ok(None),
        }
    }

    async fn list_tasks(&self) -> Result<Option<Vec<CreatedTaskDto>>, TaskError> {
        let result = sqlx::query_as!(
            TaskDAO,
            r"
            SELECT id, job_id, job, start, end
            FROM tasks 
            LIMIT 10
        "
        )
        .fetch_all(&self.conn)
        .await;

        match result {
            Ok(list) => Ok(Some(list.iter().map(|d| d.clone().dto_to_task()).collect())),
            Err(e) => Err(TaskError::DatabaseError(e.to_string())),
        }
    }

    async fn delete_task(&self, id: &Uuid) -> Result<(), TaskError> {
        let _ = sqlx::query(r"DELETE FROM tasks WHERE id = $1")
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
