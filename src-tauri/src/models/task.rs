use blender::{
    blender::{Args, Blender},
    models::status::Status,
};
use libp2p::PeerId;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    ops::Range,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use uuid::Uuid;
use crate::domains::task_store::TaskError;
use super::job::Job;

/*
    Task is used to send Worker individual task to work on
    this can be customize to determine what and how many frames to render.
    contains information about who requested the job in the first place so that the worker knows how to communicate back notification.
*/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique id for this task
    pub id: Uuid,

    /// peer's id that sent us this task, use this to callback
    peer_id: Vec<u8>,

    /// reference to the job id
    pub job_id: Uuid,

    /// target blender version to use
    pub blender_version: Version,

    /// generic blender file name from job's reference.
    pub blend_file_name: PathBuf,

    /// Render range frame to perform the task
    pub range: Range<i32>,
}

impl Task {
    pub fn new(
        peer_id: PeerId,
        job_id: Uuid,
        blend_file_name: PathBuf,
        blender_version: Version,
        range: Range<i32>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            peer_id: peer_id.to_bytes(),
            job_id,
            blend_file_name,
            blender_version,
            range,
        }
    }

    pub fn from(
        peer_id: PeerId,
        job: Job,
        range: Range<i32>
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            peer_id: peer_id.to_bytes(),
            job_id: job.id,
            blend_file_name: PathBuf::from(job.project_file.file_name().unwrap()),
            blender_version: job.blender_version,
            range
        }
    }

    // this could be async? we'll see.

    /// The behaviour of this function returns the percentage of the remaining jobs in poll.
    /// E.g. 102 (80%) of 120 remaining would return 96 end frames.
    /// TODO: Allow other node or host to fetch end frames from this task and distribute to other requesting workers.
    pub fn fetch_end_frames(&mut self, percentage: i8) -> Option<Range<i32>> {
        // Here we'll determine how many franes left, and then pass out percentage of that frames back.
        let perc = percentage as f32 / i8::MAX as f32;
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

    pub fn get_peer_id(&self) -> PeerId {
        PeerId::from_bytes(&self.peer_id).expect("Peer Id was posioned!")
    }

    fn get_next_frame(&mut self) -> Option<i32> {
        // we will use this to generate a temporary frame record on database for now.
        if self.range.start < self.range.end {
            let value = Some(self.range.start);
            self.range.start = self.range.start + 1;
            value
        } else {
            None
        }
    }

    // Invoke blender to run the job
    // how do I stop this? Will this be another async container?
    pub async fn run(
        self,
        blend_file: PathBuf,
        // output is used to create local path storage to save frame path to
        output: PathBuf,
        // reference to the blender executable path to run this task.
        blender: &Blender,
    ) -> Result<std::sync::mpsc::Receiver<Status>, TaskError> {
        let args = Args::new(blend_file, output);
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
