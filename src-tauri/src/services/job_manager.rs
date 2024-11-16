use thiserror::Error;
use uuid::Uuid;

use crate::models::job::Job;

#[derive(Debug, Error)]
pub enum JobManagerError {
    #[error("Received bad input")]
    BadInput,
    #[error("Invalid job")]
    InvalidJob,
    #[error("Blender errors")]
    BlenderError,
    #[error("Unexpected error")]
    UnexpectedError, // shouldn't really happen but we'll see!
}

// the principle design of having a job manager is to be able to hold all of the job collections.
// TODO: find a better schematics to handle different active jobs allocated for different nodes.
#[derive(Default)]
pub struct JobManager {
    jobs: Vec<Job>,
}

impl JobManager {
    pub fn add_to_queue(&mut self, job: Job) -> Result<(), JobManagerError> {
        self.jobs.push(job);
        Ok(())
    }

    pub fn remove_from_queue(&mut self, id: &Uuid) -> Result<(), JobManagerError> {
        self.jobs.retain(|v| v.as_ref().eq(id));
        Ok(())
    }
}

impl AsRef<Vec<Job>> for JobManager {
    fn as_ref(&self) -> &Vec<Job> {
        &self.jobs
    }
}
