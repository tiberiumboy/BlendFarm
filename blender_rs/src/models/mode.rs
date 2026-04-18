use serde::{Deserialize, Serialize};
use std::num::ParseIntError;

// context for serde: https://serde.rs/enum-representations.html
#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub enum RenderMode {
    // JSON: "Frame": "i32",
    // render a single frame.
    Frame(i32),

    // JSON: "Animation": {"start":"i32", "end":"i32"}
    // contains the target start frame to the end target frame.
    Animation { start: i32, end: i32 },
    // future project - allow network node to only render section of the frame instead of whole to visualize realtime rendering view solution.
    // JSON: "Section": {"frame":"i32", "coord":{"i32", "i32"}, "size": {"i32", "i32"} }
    // Section {
    //     frame: i32,
    //     coord: (i32, i32),
    //     size: (i32, i32),
    // },
}

impl RenderMode {
    pub fn new(start: i32, end: i32) -> RenderMode {
        let mut start = start;
        let mut end = end;

        // start needs to be the lowest number of all. If it's backward, flip it around.
        if start > end {
            (start, end) = (end, start);
        }

        if start + 1 == end {
            RenderMode::Frame(start)
        } else {
            RenderMode::Animation { start, end }
        }
    }

    pub fn try_new(start: &str, end: &str) -> Result<RenderMode, ParseIntError> {
        // stop if the parser fail to parse.
        let start = start.parse::<i32>()?;
        let end = end.parse::<i32>()?;

        Ok(RenderMode::new(start, end))
    }
}
