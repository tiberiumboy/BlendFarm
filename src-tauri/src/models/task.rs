use super::job::CreatedJobDto;
use crate::{
    domains::task_store::TaskError,
    models::{job::Job, with_id::WithId},
};
use blender::{
    blender::{Args, Blender},
    models::{engine::Engine, event::BlenderEvent},
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{
    ops::Range,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use uuid::Uuid;

pub type CreatedTaskDto = WithId<Task, Uuid>;

/*
    Task is used to send Worker individual task to work on
    this can be customize to determine what and how many frames to render.
*/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Id used to identify the job
    job_id: Uuid,

    /// job reference.
    job: Job,

    // temp output destination - used to hold render image in temp on client machines
    temp_output: PathBuf,

    /// Render range frame to perform the task
    pub range: Range<i32>,
}

// To better understand Task, this is something that will be save to the database and maintain a record copy for data recovery
// This act as a pending work to fulfill when resources are available.
impl Task {
    // private method, less validation.
    fn new(job_id: Uuid, job: Job, temp_output: PathBuf, range: Range<i32>) -> Self {
        Self {
            job_id,
            job,
            temp_output,
            range,
        }
    }

    pub fn from(job: CreatedJobDto, range: Range<i32>) -> Result<Self, TaskError> {
        match dirs::cache_dir() {
            Some(tmp) => Ok(Task::new(job.id, job.item, tmp, range)),
            None => Err(TaskError::CacheError),
        }
    }

    pub fn get_id(&self) -> &Uuid {
        &self.job_id
    }

    pub fn get_job(&self) -> &Job {
        &self.job
    }

    /// The behaviour of this function returns the percentage of the remaining jobs in poll.
    /// E.g. 102 (out of 255- 80%) of 120 remaining would return 96 end frames.
    /// TODO: Allow other node or host to fetch end frames from this task and distribute to other requesting workers.
    pub fn fetch_end_frames(&mut self, percentage: u8) -> Option<Range<i32>> {
        // Here we'll determine how many franes left, and then pass out percentage of that frames back.
        let perc = percentage as f32 / u8::MAX as f32;
        let end = self.range.end;
        let delta = (end - self.range.start) as f32;
        let trunc = (perc * (delta.powf(2.0)).sqrt()).floor() as usize;

        if trunc.le(&2) {
            return None;
        }

        let start = end - trunc as i32;
        let range = Range { start, end };
        self.range.end = start - 1; // Update end value accordingly.
        Some(range)
    }

    fn get_next_frame(&mut self) -> Option<i32> {
        // we will use this to generate a temporary frame record on database for now.
        if self.range.start < (self.range.end + 1) {
            let value = Some(self.range.start);
            self.range.start = self.range.start + 1;
            value
        } else {
            None
        }
    }

    // Invoke blender to run the job
    // how do I stop this? Will this be another async container?
    pub async fn run<T: AsRef<Path>>(
        self,
        blend_file: T,
        // output is used to create local path storage to save frame path to
        output: T,
        // reference to the blender executable path to run this task.
        blender: &Blender,
    ) -> Result<std::sync::mpsc::Receiver<BlenderEvent>, TaskError> {
        let args = Args::new(
            blend_file.as_ref().to_path_buf(),
            output.as_ref().to_path_buf(),
            Engine::CYCLES,
        );
        let arc_task = Arc::new(RwLock::new(self)).clone();

        // TODO: How can I adjust blender jobs?
        // this always puzzle me. Is this still awaited after application closed?
        let receiver = blender
            .render(args, move || -> Option<i32> {
                let mut task = match arc_task.write() {
                    Ok(task) => task,
                    Err(_) => return None,
                };
                task.get_next_frame()
            })
            .await;
        Ok(receiver)
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
        let range = Range { start, end };
        Task::from(data, range).expect("Should have valid task")
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
