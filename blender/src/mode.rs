// use std::default;

use serde::{Deserialize, Serialize};

// context for serde: https://serde.rs/enum-representations.html
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    None,
    Frame(i32), // serialize to "Single": "i32",
    Animation {
        start: i32,
        end: i32,
    }, // Serialize to "Section":{"start":"i32", "end":"i32"}
                // future project - allow network node to only render section of the frame instead of whole to visualize realtime rendering view solution.
                // Section {
                //     frame: i32,
                //     top_left: (i32, i32),
                //     bottom_right: (i32, i32),
                // },
}
