use futures::StreamExt;
use std::sync::Arc;
use surrealdb::{engine::local::Db, opt::Resource, Action, Notification, Surreal};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    domains::task_store::{TaskError, TaskStore},
    models::task::Task,
};

const TASK_STORE_TABLE: &str = "tasks";

pub struct SurrealDbTaskStore {
    conn: Arc<RwLock<Surreal<Db>>>,
}

impl SurrealDbTaskStore {
    pub fn new(conn: Arc<RwLock<Surreal<Db>>>) -> Self {
        Self { conn }
    }

    // huh?
    // async fn handle(
    //     &mut self,
    //     result: Result<Notification<Task>, TaskError>,
    // ) -> Result<Task, TaskError> {
    //     match result {
    //         Ok(notify) if notify.action == Action::Create => Ok(notify.data),
    //         Err(e) => Err(e),
    //         _ => todo!("Unhandle result? {result:?}"),
    //     }
    // }
}

#[async_trait::async_trait]
impl TaskStore for SurrealDbTaskStore {
    async fn add_task(&mut self, task: Task) -> Result<(), TaskError> {
        let db = self.conn.write().await;
        db.create(Resource::from((
            TASK_STORE_TABLE,
            task.get_peer_id().to_base58(),
        )))
        .content(task)
        .await
        .map_err(|e| TaskError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn poll_task(&mut self) -> Result<Task, TaskError> {
        let db = self.conn.write().await;
        let mut stream = db
            .select(TASK_STORE_TABLE)
            .live()
            .await
            .map_err(|e| TaskError::DatabaseError(e.to_string()))?;
        match stream.next().await {
            Some(Ok(notify)) => Ok(notify.data),
            _ => Err(TaskError::Unknown),
        }
    }

    async fn delete_task(&mut self, task: Task) -> Result<(), TaskError> {
        let db = self.conn.write().await;
        let _: Option<Task> = db
            .delete((TASK_STORE_TABLE, task.get_peer_id().to_base58()))
            .await
            .map_err(|e| TaskError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn delete_job_task(&mut self, job_id: Uuid) -> Result<(), TaskError> {
        let db = self.conn.write().await;
        let _ = db
            .query(r#"DELETE task WHERE job_id = $job_id"#)
            .bind(("job_id", job_id.to_string()))
            .await
            .map_err(|e| TaskError::DatabaseError(e.to_string()))?;
        Ok(())
    }
}
