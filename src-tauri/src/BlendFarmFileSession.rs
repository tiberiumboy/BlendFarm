use std::fs::File;
use std::io::prelude::*;
use uuid::Uuid;

struct BlendFarmFileSession {
    pub session_id: Uuid,
    pub blend_file: File,
}

impl BlendFarmFileSession {
    fn new(blend_file: String) -> BlendFarmFileSession {
        BlendFarmFileSession {
            blend_file: File::open(blend_file),
            session_id: Uuid::new_v4(),
        }
    }
}
