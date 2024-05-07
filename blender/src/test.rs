#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn run_blender() {
        // TODO: Replace this to reference correct blender version.
        let path = match env::consts::OS {
            "linux" => PathBuf::from("/home/jordan/Downloads/blender/blender"),
            "macos" => PathBuf::from("/Applications/Blender.app/Contents/MacOS/Blender"),
            _ => panic!("unsupported OS"),
        };
        let blender = blender::Blender::from_executable(path);
        assert!(blender.version, "4.1.0");
    }
}
