use super::job::CreatedJobDto;
use crate::{
    domains::ticket_store::TicketError,
    models::with_id::WithId,
};
use blender_rs::{blend_file::BlendFile, blender::{Args, Blender, ComputerGraphicsProgram, Frame}, models::event::BlenderEvent};
use semver::Version;
use serde::{Deserialize, Serialize};
use tokio::spawn;
use std::{
    collections::HashMap, path::PathBuf, sync::mpsc::{self, Receiver}
};
use uuid::Uuid;

pub type CreatedTicketDto = WithId<Ticket, Uuid>;

// pub enum TaskStatus {
// use this to describe what's going on with this task.
// }

/*
    Task is used to send Worker individual task to work on
    this can be customize to determine what and how many frames to render.

    pub(crate) is used to help decode struct into sqlx query, and vice versa.
    See if there's a better way to handle this without modifying sqlx table migration?
*/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    /// Id used to identify the job
    pub(crate) job_id: Uuid,
    
    // Path to specific blender file to render for this ticket. Must be valid and exist, otherwise skip.
    pub(crate) blend_path: PathBuf,

    // target blender version to use for this job
    pub(crate) blender_version: Version,

    // temp output destination - used to hold render image in temp on client machines
    temp_output: PathBuf,

    /// collection of completed render images
    renders: HashMap<Frame, PathBuf>,

    /// Render range frame to perform the task
    /// TODO: Could this be used as a "Range" struct? Is Range serializable?
    pub(crate) start: Frame,
    pub(crate) end: Frame,  
}

// To better understand Task, this is something that will be save to the database and maintain a record copy for data recovery
// This act as a pending work order to fulfill when resources are available.
impl Ticket {
    // private method, less validation.
    pub(crate) fn new(job_id: Uuid, blend_path: PathBuf, blender_version: Version, temp_output: PathBuf, start: i32, end: i32 ) -> Self {
        Self {
            job_id,
            blend_path,
            blender_version,
            temp_output,
            renders: HashMap::new(),
            start,
            end,
        }
    }

    pub fn add_render(mut self, frame: Frame, path: PathBuf ) -> Self {
        self.renders.insert(frame,  path);
        self
    }

    pub fn from(job: CreatedJobDto, start: i32, end: i32) -> Result<Self, TicketError> {
        match dirs::cache_dir() {
            Some(tmp) => {
                let id = job.id;
                let blender_version = job.item.get_blender_version().clone();
                let blend_path = job.item.blend_file.to_path().to_path_buf();
                Ok(Ticket::new(id, blend_path, blender_version, tmp, start, end))
            },
            None => Err(TicketError::CacheError),
        }
    }
    
    pub async fn render(&mut self, blender: &Blender) -> Result<Receiver<BlenderEvent>, TicketError> {
        // first thing first, 
        // check and see if we have any renders completed in our pool of resources.
        // let count = (self.end - self.start) as usize;
        // let range = HashMap::<i32, PathBuf>::with_capacity(count).iter_mut();
        // self.renders.iter().for_each(|).collect();

        let blend_file = &self.blend_path;
        let file = BlendFile::try_from(blend_file.clone()).map_err(TicketError::BlenderError)?;
        let args = Args::new(file, self.temp_output.clone(), self.start, self.end);
        let (tx, rx) = mpsc::channel();
        let mut process = blender.render(args).map_err(TicketError::BlenderError)?;
        spawn(async move{
            while let Some(event) = process.read() {
                if let Err(e) = tx.send(event) {
                    eprintln!("Unable to transmit blender event! {e:?}");
                }
            }
        });

        Ok(rx)
    }
}

impl AsRef<Uuid> for Ticket {
    fn as_ref(&self) -> &Uuid {
        &self.job_id
    }
}

/* 
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
*/
