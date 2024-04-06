pub struct BlenderManager {
    installed: Vec<BlenderVersion>,
}

const VERSIONS_URL: &str = "https://download.blender.org/release/";
impl BlenderManager {
    pub fn new() -> BlenderManager {
        BlenderManager { installed: vec![] }
    }

    pub fn download_latest(&mut self) {
        let version = self.get_latest_version().unwrap();
        let path = self.get_latest_path().unwrap();
        let url = format!("{}/{}", VERSIONS_URL, version);

        let version = BlenderVersion {
            url,
            path: path.clone(),
            version: version.clone(),
        };

        version.download();
        self.install(version);
    }
}
