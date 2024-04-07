use crate::blender::version::Blender;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BlenderCollection {
    collection: Vec<Blender>,
}

// impl BlenderCollection {
//     fn load() -> Self {}
// }
