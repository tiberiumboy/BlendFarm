use super::job::CreatedJobDto;
use crate::{
    domains::task_store::TaskError,
    models::{job::Job, with_id::WithId},
};
use blender::{blender::Frame, constant::MIN_THRESHOLD_FETCH};
use serde::{Deserialize, Serialize};
use std::{
    ops::Range,
    path::PathBuf,
};
use uuid::Uuid;

pub type CreatedTaskDto = WithId<Task, Uuid>;

// pub enum TaskStatus {
    // use this to describe what's going on with this task.
// }

/*
    Task is used to send Worker individual task to work on
    this can be customize to determine what and how many frames to render.
*/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    // status: 

    /// Id used to identify the job
    job_id: Uuid,

    /// job reference. // May no longer needed?
    /// This really should expand out to the required info to run the job such as blender file, version, frames, etc.
    job: Job,

    // temp output destination - used to hold render image in temp on client machines
    // this should not be visible/present for host to obtain.
    temp_output: PathBuf,

    /// Render range frame to perform the task
    pub(crate) start: Frame,
    pub(crate) end: Frame,
}

// To better understand Task, this is something that will be save to the database and maintain a record copy for data recovery
// This act as a pending work order to fulfill when resources are available.
impl Task {
    // private method, less validation.
    fn new(job_id: Uuid, job: Job, temp_output: PathBuf, start: i32, end: i32 ) -> Self {
        Self {
            job_id,
            job,
            temp_output,
            start,
            end
        }
    }

    pub fn from(job: CreatedJobDto, start: i32, end: i32) -> Result<Self, TaskError> {
        match dirs::cache_dir() {
            Some(tmp) => Ok(Task::new(job.id, job.item, tmp, start, end)),
            None => Err(TaskError::CacheError),
        }
    }

    // TODO: Instead
    /// The behaviour of this function returns the percentage of the remaining jobs in poll.
    /// E.g. 102 (out of 255- 80%) of 120 remaining would return 96 end frames.
    /// TODO: Allow other node or host to fetch end frames from this task and distribute to other requesting workers.
    pub fn fetch_end_frames(&mut self, percentage: u8) -> Option<Range<i32>> {
        // Here we'll determine how many franes left, and then pass out percentage of that frames back.
        let perc = percentage as f32 / u8::MAX as f32;
        let end = self.end;
        let delta = (end - self.start) as f32;
        let trunc = (perc * (delta.powf(2.0)).sqrt()).floor() as usize;

        if trunc <= MIN_THRESHOLD_FETCH {
            return None;
        }

        let start = end - trunc as i32;
        let range = Range { start, end };
        self.end = start - 1; // Update end value accordingly.
        Some(range)
    }


    // not currently in used, was originally using this for blender advance batch render feedback system
    #[cfg(test)]
    fn get_next_frame(&mut self) -> Option<i32> {
        // we will use this to generate a temporary frame record on database for now.
        if self.start < (self.end + 1) {
            let value = Some(self.start);
            self.start = self.start + 1;
            value
        } else {
            None
        }
    }
}

impl AsRef<Uuid> for Task {
    fn as_ref(&self) -> &Uuid {
        &self.job_id
    }
}

impl AsRef<Job> for Task {
    fn as_ref(&self) -> &Job {
        &self.job
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::models::job::test::scaffold_job;
    use uuid::Uuid;

    fn scaffold_task(start: i32, end: i32) -> Task {
        let data = WithId {
            id: Uuid::new_v4(),
            item: scaffold_job(),
        };
        Task::from(data, start, end).expect("Should have valid task")
    }

    #[test]
    fn fetch_end_frame_success() {
        // we should run two scenario, one with actual frames, and another with limited or no frames left.
        // if we tried to call with enough buffer pending, we should expect Some(value) back
        // otherwise if the node is almost done and it was called, None should return.
        let mut task = scaffold_task(0, 50);
        let data = task.fetch_end_frames(255);
        assert!(data.is_some());

        let data = task.fetch_end_frames(5);
        assert!(data.is_none());
    }

    #[test]
    fn get_next_frame_success() {
        // We should expect two successful result
        // one result is that we should have remaining frames, so we should expect to get Some(value)
        // otherwise None should return that we've completed the job.
        let mut task = scaffold_task(0, 1);
        let data = task.get_next_frame();
        assert!(data.is_some());

        let data = task.get_next_frame();
        assert!(data.is_some());

        let data = task.get_next_frame();
        assert!(data.is_none());
    }
}
