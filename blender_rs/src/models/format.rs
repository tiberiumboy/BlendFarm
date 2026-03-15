use serde::{Deserialize, Serialize};

pub enum FormatError {
    InvalidInput,
}

// More context: https://docs.blender.org/manual/en/latest/advanced/command_line/arguments.html#format-options
#[derive(Debug, Copy, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum Format {
    TGA,
    RAWTGA,
    JPEG,
    IRIS,
    AVIRAW,
    AVIJPEG,
    #[default]
    PNG,
    BMP,
    HDR,
    TIFF,
}
