use serde::{Deserialize, Serialize};

// context for serde: https://serde.rs/enum-representations.html
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Mode {
    Frame(i32),
    Animation,
    Section { start: i32, end: i32 },
}
