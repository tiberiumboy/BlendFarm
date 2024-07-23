use serde::{Deserialize, Serialize};
use std::io::Result;

// TODO: Provide file format explicitly define by user
// More context: https://docs.blender.org/manual/en/latest/advanced/command_line/arguments.html#format-options
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
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

impl Format {
    #[allow(dead_code)]
    fn parse(format: String) -> Result<Format> {
        match format.to_uppercase().as_str() {
            "TGA" => Ok(Format::TGA),
            "RAWTGA" => Ok(Format::RAWTGA),
            "JPEG" => Ok(Format::JPEG),
            "IRIS" => Ok(Format::IRIS),
            "AVIRAW" => Ok(Format::AVIRAW),
            "AVIJPEG" => Ok(Format::AVIJPEG),
            "PNG" => Ok(Format::PNG),
            "BMP" => Ok(Format::BMP),
            "HDR" => Ok(Format::HDR),
            "TIFF" => Ok(Format::TIFF),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid format",
            )),
        }
    }
}

impl ToString for Format {
    fn to_string(&self) -> String {
        match self {
            Format::TGA => "TARGA".to_owned(),
            Format::RAWTGA => "RAWTARGA".to_owned(),
            Format::JPEG => "JPEG".to_owned(),
            Format::IRIS => "IRIS".to_owned(),
            Format::AVIRAW => "AVIRAW".to_owned(),
            Format::AVIJPEG => "AVIJPEG".to_owned(),
            Format::PNG => "PNG".to_owned(),
            Format::BMP => "BMP".to_owned(),
            Format::HDR => "HDR".to_owned(),
            Format::TIFF => "TIFF".to_owned(),
        }
    }
}