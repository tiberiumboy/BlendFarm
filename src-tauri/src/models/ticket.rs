use super::job::CreatedJobDto;
use crate::{
    domains::ticket_store::TicketError,
    models::{job::Job, with_id::WithId},
};
use blender::{blend_file::BlendFile, blender::{Args, Blender, Frame}, models::event::BlenderEvent};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;
use std::{
    collections::HashMap, path::PathBuf
};
use uuid::Uuid;

pub type CreatedTicketDto = WithId<Ticket, Uuid>;

// pub enum TaskStatus {
    // use this to describe what's going on with this task.
// }

/*
    Task is used to send Worker individual task to work on
    this can be customize to determine what and how many frames to render.
*/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    // status: 

    /// Id used to identify the job
    job_id: Uuid,

    /// This really should expand out to the required info to run the job such as blender file, version, frames, etc.
    pub(crate) job: Job,

    // temp output destination - used to hold render image in temp on client machines
    // this should not be visible/present for host to obtain.
    temp_output: PathBuf,

    /// collection of completed render images
    renders: HashMap<Frame, PathBuf>,

    /// Render range frame to perform the task
    pub(crate) start: Frame,
    pub(crate) end: Frame,
}

// To better understand Task, this is something that will be save to the database and maintain a record copy for data recovery
// This act as a pending work order to fulfill when resources are available.
impl Ticket {
    // private method, less validation.
    fn new(job_id: Uuid, job: Job, temp_output: PathBuf, start: i32, end: i32 ) -> Self {
        Self {
            job_id,
            job,
            temp_output,
            renders: HashMap::new(),
            start,
            end
        }
    }

    pub fn from(job: CreatedJobDto, start: i32, end: i32) -> Result<Self, TicketError> {
        match dirs::cache_dir() {
            Some(tmp) => Ok(Ticket::new(job.id, job.item, tmp, start, end)),
            None => Err(TicketError::CacheError),
        }
    }
    
    pub async fn render(&mut self, blender: &Blender) -> Result<Receiver<BlenderEvent>, TicketError> {
        let job = &self.job;
        let blend_file = AsRef::<BlendFile>::as_ref(&job);
        let args = Args::new(blend_file.clone(), self.temp_output.clone(), self.start, self.end);
        blender.render(args).await.map_err(TicketError::BlenderError)
    }    
}

impl AsRef<Uuid> for Ticket {
    fn as_ref(&self) -> &Uuid {
        &self.job_id
    }
}

impl AsRef<Job> for Ticket {
    fn as_ref(&self) -> &Job {
        &self.job
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