use uuid::Uuid;

use crate::models::job::Job;

pub enum JobManagerError {
    BadInput,
    InvalidJob,
    BlenderError,
    UnexpectedError, // shouldn't really happen but we'll see!
}

#[async_trait::async_trait]
pub trait JobManager: Send + Sync {
    async fn add_to_queue(&mut self, job: Job) -> Result<(), JobManagerError>;
    async fn get_queues(&self) -> Result<&Vec<Job>, JobManagerError>;
    async fn remove_from_queue(&mut self, job_id: &Uuid) -> Result<(), JobManagerError>;
}
