use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Mode {
    Frame(i32),
    Animation,
    Section { start: i32, end: i32 },
}
