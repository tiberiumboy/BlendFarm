use serde::{Deserialize, Serialize};

// context for serde: https://serde.rs/enum-representations.html
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Mode {
    Frame(i32), // serialize to "Frame": "i32",
    Animation,  // serialize to "Animation",
    // Issue with this command arguments - it will not render teh scene and it stats the following error message:
    // "asset protocol not configured to allow the path: "
    // ??? What path???
    Section { start: i32, end: i32 }, // Serialize to "Section":{"start":"i32", "end":"i32"}
}
