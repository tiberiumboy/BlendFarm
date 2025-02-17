use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// here we will store the information of the gpu/cpu/memory usage of the workers
#[derive(Debug, Serialize, Deserialize)]
pub struct Activity {
    id: Uuid, // id to identify the worker to filter the query out
    cpu: f32,
    gpu: f32,
    mem: f32,
}

#[derive(Debug, Error)]
pub enum ActivityError {
    #[error("Received database error! {0}")]
    Database(String),
    #[error("An unknown just happen!")]
    Unknown,    // should not happen and should be identified before compiling this app!
}

#[async_trait::async_trait]
pub trait ActivityStore {
    async fn add(&mut self, activity: Activity) -> Result<(), ActivityError>;
    async fn list(&self) -> Result<Vec<Activity>, ActivityError>;
}