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
    pub async fn new(conn: Arc<RwLock<Surreal<Db>>>) -> Self {
        {
            let db = conn.write().await;
            db.query(
                // TODO: Find a way to get the types I need to store and create database definitions
                r#"
                DEFINE TABLE IF NOT EXISTS task SCHEMALESS;
                DEFINE FIELD IF NOT EXISTS peer_id ON TABLE task TYPE array<int>;
                DEFINE FIELD IF NOT EXISTS job_id ON TABLE task TYPE uuid;
                DEFINE FIELD IF NOT EXISTS blender_version ON TABLE task TYPE string;
                DEFINE FIELD IF NOT EXISTS blend_file_name ON TABLE task TYPE string;
                DEFINE FIELD IF NOT EXISTS start ON TABLE task TYPE int;
                DEFINE FIELD IF NOT EXISTS end ON TABLE task TYPE int;
                "#,
            )
            .await
            .expect("Should have permission to check for database schema");
        }
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
