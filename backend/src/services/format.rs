// TODO: Provide file format explicitly define by user
// More context: https://docs.blender.org/manual/en/latest/advanced/command_line/arguments.html#format-options
#[derive(Debug, Serialize, Deserialize)]
pub enum Format {
    TGA,
    RAWTGA,
    JPEG,
    IRIS,
    AVIRAW,
    AVIJPEG,
    PNG,
    BMP,
    HDR,
    TIFF,
}

impl Format {
    #[allow(dead_code)]
    fn to_str(&self) -> String {
        match self {
            Format::TGA => "TARGA",
            Format::RAWTGA => "RAW TARGA",
            Format::JPEG => "JPEG",
            Format::IRIS => "IRIS",
            Format::AVIRAW => "AVI RAW",
            Format::AVIJPEG => "AVI JPEG",
            Format::PNG => "PNG",
            Format::BMP => "BMP",
            Format::HDR => "HDR",
            Format::TIFF => "TIFF",
        }
        .to_string()
    }

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
