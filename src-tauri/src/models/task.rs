use super::job::CreatedJobDto;
use crate::{domains::task_store::TaskError, models::with_id::WithId};
use blender::{
    blender::{Args, Blender},
    models::{engine::Engine, event::BlenderEvent},
};
use semver::Version;
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
    contains information about who requested the job in the first place so that the worker knows how to communicate back notification.
*/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// reference to the job id
    pub job_id: Uuid,

    /// target blender version to use
    pub blender_version: Version,

    /// generic blender file name from job's reference.
    pub blend_file_name: PathBuf,

    /// Render range frame to perform the task
    pub range: Range<i32>,
}

// To better understand Task, this is something that will be save to the database and maintain a record copy for data recovery
// This act as a pending work to fulfill when resources are available.
impl Task {
    pub fn new(
        job_id: Uuid,
        blend_file_name: PathBuf,
        blender_version: Version,
        range: Range<i32>,
    ) -> Self {
        Self {
            job_id,
            blend_file_name,
            blender_version,
            range,
        }
    }

    pub fn from(job: CreatedJobDto, range: Range<i32>) -> Self {
        Self {
            job_id: job.id,
            blend_file_name: PathBuf::from(job.item.project_file.file_name().unwrap()),
            blender_version: job.item.blender_version,
            range,
        }
    }

    /// The behaviour of this function returns the percentage of the remaining jobs in poll.
    /// E.g. 102 (80%) of 120 remaining would return 96 end frames.
    /// TODO: Allow other node or host to fetch end frames from this task and distribute to other requesting workers.
    /// TODO: Test this
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
    use async_std::path::PathBuf;
    use uuid::Uuid;

    fn scaffold_task(start: i32, end: i32) -> Task {
        let job_id = Uuid::new_v4();
        let path= PathBuf::from(".");
        let version = Version::new(1,1,1);
        let range = Range { start, end };
        Task::new(job_id, path.into(), version, range )
    }

    #[test]
    fn fetch_end_frame_success() {
        // we should run two scenario, one with actual frames, and another with limited or no frames left.
        // if we tried to call with enough buffer pending, we should expect Some(value) back
        // otherwise if the node is almost done and it was called, None should return.
        let mut task =  scaffold_task(0, 50);
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
