use std::env::consts;

/// Return extension matching to the current operating system (Only display Windows(.zip), Linux(.tar.xz), or macos(.dmg)).
pub(crate) fn get_extension() -> Result<String, String> {
    match consts::OS {
        "windows" => Ok(".zip".to_owned()),
        "macos" => Ok(".dmg".to_owned()),
        "linux" => Ok(".tar.xz".to_owned()),
        os => Err(os.to_string()),
    }
}

/// fetch current architecture (Currently support x86_64 or aarch64 (apple silicon))
pub(crate) fn get_valid_arch() -> Result<String, String> {
    match consts::ARCH {
        "x86_64" => Ok("x64".to_owned()),
        "aarch64" => Ok("arm64".to_owned()),
        arch => Err(arch.to_string()),
    }
}