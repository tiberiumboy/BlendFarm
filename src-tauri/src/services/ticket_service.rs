use std::path::PathBuf;
use blender_rs::blender::Frame;
use crate::models::{job::{Job, JobId}, ticket::Ticket, with_id::WithId};



#[derive(Debug, Clone)]
struct TicketService {
    max_frame_alloc: Frame
}

impl TicketService {
    pub fn new( max_frame_alloc: Frame ) -> Self {
        TicketService {
            max_frame_alloc: i32::abs(max_frame_alloc)
        }
    }

    pub fn generate_tickets(&self, job: WithId<Job, JobId>) -> Vec<Ticket> {
        // check and see if we have any completed images

        // then check and see if we have any existing tickets created for this job

        // if all fails, then create a new list of tickets to complete the job
        let item = job.item;
        let id = job.id;
        let mut collection: Vec<Ticket> = Vec::new();
        let (mut idx, end) = item.get_range();

        while end - idx > 0 {
            let until = end.min(idx + self.max_frame_alloc);
            // how do we generate a new output for this ticket?
            let output = PathBuf::new();
            let ticket = Ticket::new(id, item.blend_file.to_path().to_path_buf(), item.get_blender_version().clone(), output, idx, until);
            collection.push(ticket);
            idx = until;
        }

        collection
    }
}