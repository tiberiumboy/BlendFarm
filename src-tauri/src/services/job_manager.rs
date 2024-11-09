use uuid::Uuid;

use crate::{
    domain::job_manager::{JobManager, JobManagerError},
    models::job::Job,
};

#[derive(Default)]
pub struct InMemoryJobManager {
    jobs: Vec<Job>,
}

#[async_trait::async_trait]
impl JobManager for InMemoryJobManager {
    async fn add_to_queue(&mut self, job: Job) -> Result<(), JobManagerError> {
        self.jobs.push(job);
        Ok(())
    }

    async fn get_queues(&self) -> Result<&Vec<Job>, JobManagerError> {
        Ok(&self.jobs)
    }

    async fn remove_from_queue(&mut self, id: &Uuid) -> Result<(), JobManagerError> {
        self.jobs.retain(|v| v.as_ref().eq(id));
        Ok(())
    }
}
