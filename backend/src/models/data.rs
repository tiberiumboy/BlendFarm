use serde::{Deserialize, Serialize};

use super::job::Job;

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct Data {
    // I may not need this anymore?
    pub jobs: Vec<Job>, // Keeps track of current job process
}
