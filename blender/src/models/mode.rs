// use std::default;
use std::ops::Range;
use serde::{Deserialize, Serialize};

// context for serde: https://serde.rs/enum-representations.html
#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub enum Mode {
    // JSON: "Frame": "i32",
    // render a single frame.
    Frame(i32),

    // JSON: "Animation": {"start":"i32", "end":"i32"}
    // contains the target start frame to the end target frame.
    Animation(Range<i32>),
    
    // future project - allow network node to only render section of the frame instead of whole to visualize realtime rendering view solution.
    // JSON: "Section": {"frame":"i32", "coord":{"i32", "i32"}, "size": {"i32", "i32"} }
    // Section {
    //     frame: i32,
    //     coord: (i32, i32),
    //     size: (i32, i32),
    // },
}
