use futures::StreamExt;
use libp2p::PeerId;
use std::sync::Arc;
use surrealdb::{engine::local::Db, opt::Resource, Action, Notification, Surreal};
use tokio::sync::RwLock;

use crate::{domains::task_store::{TaskError, TaskStore}, models::task::Task};

const TASK_STORE_TABLE: &str = "tasks";

pub struct SurrealTaskStore {
    pub conn: Arc<RwLock<Surreal<Db>>>,
}

impl SurrealTaskStore {
    async fn handle(&mut self, result: Result<Notification<Task>,TaskError>) -> Result<Task, TaskError> {
        
        match result {
            Ok(notify) if notify.action == Action::Create => Ok( notify.data ),
            Err(e) => Err(e),
            _ => todo!("Unhandle result? {result:?}")
        }
    }
}

#[async_trait::async_trait]
impl TaskStore for SurrealTaskStore {

    async fn add_task(&mut self, requestor: PeerId, task: Task) -> Result<(), TaskError> {
        let db = self.conn.write().await; 
        db
            .create(Resource::new((TASK_STORE_TABLE, requestor.public())))
            .content(task).await.map_err(|e| TaskError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn poll_task(&mut self) -> Result<(PeerId, Task), TaskError> {
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
    
    async fn delete_task(&mut self, task: Task ) -> Result<(), TaskError> {

        let db = self.conn.write().await;
        // is it possible that this function could help identify target record or criteria?
        db.delete(Resource::new((TASK_STORE_TABLE, task))).await;
        Ok(())
    }
}