use std::path::PathBuf;
use crate::models::{job::{Job, JobId}, ticket::Ticket, with_id::WithId};



#[allow(dead_code)]
#[derive(Debug, Clone)]
struct TicketService {
    max_frame_alloc: u32
}

#[allow(dead_code)]
impl TicketService {
    pub fn new( max_frame_alloc: u32 ) -> Self {
        TicketService {
            max_frame_alloc
        }
    }

    // probably best to be used under Job model?
    pub fn generate_tickets(&self, job: WithId<Job, JobId>) -> Vec<Ticket> {
        // check and see if we have any completed images

        // then check and see if we have any existing tickets created for this job

        // if all fails, then create a new list of tickets to complete the job
        let item = job.item;
        let id = job.id;
        let mut collection: Vec<Ticket> = Vec::new();
        let (mut idx, end) = item.get_range();

        while end - idx > 0 {
            let until = end.min(idx + self.max_frame_alloc as i32 );
            // how do we generate a new output for this ticket?
            let output = PathBuf::new();
            let ticket = Ticket::new(id, item.blend_file.to_path().to_path_buf(), item.get_blender_version().clone(), output, idx, until);
            collection.push(ticket);
            idx = until;
        }

        collection
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use uuid::Uuid;
    use crate::models::job::test::scaffold_job;
    use super::*;

    fn mock_ticket_service(max_frame_alloc: Option<u32>) -> TicketService {
        TicketService::new(max_frame_alloc.unwrap_or(15))
    }

    #[test]
    fn assure_generate_ticket_succeed() {
        let services = mock_ticket_service(None);
        let job = scaffold_job();
        let id = Uuid::new_v4();
        
        let collection = services.generate_tickets(WithId{ id, item: job });
        assert!(collection.iter().count() > 1);
    }
}