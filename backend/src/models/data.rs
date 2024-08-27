use crate::models::server_setting::ServerSetting;
use serde::{Deserialize, Serialize};

use super::job::Job;

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct Data {
    pub server_setting: ServerSetting, // this local host machine configuration
    // May not need this anymore - we'll rely on the most trusted list by checking the directory on application start
    // pub project_files: Vec<ProjectFile>, //Vec<ProjectFile>, // Project library
    pub jobs: Vec<Job>, // Keeps track of current job process
}
