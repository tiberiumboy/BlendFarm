use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Mode {
    Frame(i32),
    Animation,
    Section(i32, i32),
}
