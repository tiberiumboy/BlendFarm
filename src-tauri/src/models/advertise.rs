use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug)]
pub struct Advertise {
    pub id: Uuid,
    pub ad_name: String,
    pub file_path: PathBuf,
}

impl Advertise {
    pub fn new(ad_name: String, file_path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            ad_name,
            file_path,
        }
    }
}
