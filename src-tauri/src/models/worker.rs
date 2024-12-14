use super::computer_spec::ComputerSpec;
use serde::{Deserialize, Serialize};

// we will use this to store data into database at some point.
#[derive(Serialize, Deserialize)]
pub struct Worker {
    id: String,
    spec: ComputerSpec,
}
