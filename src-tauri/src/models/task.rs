use blender::{
    blender::{Args, Blender},
    models::status::Status,
};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, path::PathBuf};
use thiserror::Error;
use uuid::Uuid;

use super::job::Job;

#[derive(Debug, Error)]
pub enum TaskError {
    #[from(BlenderError)]
    BlenderError,
}

/*
    Task is used to send Worker individual task to work on
    this can be customize to determine what and how many frames to render.
    contains information about who requested the job in the first place so that the worker knows how to communicate back notification.
*/
#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    // reference to the parent that creates this task.
    job_id: Uuid,
    // Used to fetch blender file from the network shared
    blend_path_name: String,
    // used to reference to the blender path for blender to render
    // frames remaining to conduct
    frames: VecDeque<i32>,
}

impl Task {
    pub fn new(job: &Job, frames: VecDeque<i32>) -> Self {
        Self {
            job_id: job.as_ref().clone(),
            blend_path_name: job.get_file_name().unwrap().to_owned(),
            frames,
        }
    }

    fn offload_frame_for_workers(&mut self, percentage: i) -> Option<Vec<i32>> {
        // here we'll determine how many franes left, and then pass out percentage of that frames back.
    }

    fn get_next_frame(&mut self) -> impl Option<i32> + Send + Sync + 'static {
        self.frames.pop_front()
    }

    // Invoke blender to run the job
    pub async fn run(
        &mut self,
        // output is used to create local path storage to save frame path to
        output: PathBuf,
        // reference to the blender executable path to run this task.
        blender: &Blender,
    ) -> Result<std::sync::mpsc::Receiver<Status>, TaskError> {
        let file = self.project_file.clone();
        let args = Args::new(file, output, mode);

        // TODO: How can I adjust blender jobs?
        let receiver = blender.render(args, get_next_frame).await;
        Ok(receiver)
    }
}
