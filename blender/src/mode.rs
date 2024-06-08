use serde::{Deserialize, Serialize};

// context for serde: https://serde.rs/enum-representations.html
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Mode {
    Frame(i32),                       // serialize to "Single": "i32",
    Section { start: i32, end: i32 }, // Serialize to "Section":{"start":"i32", "end":"i32"}
}
