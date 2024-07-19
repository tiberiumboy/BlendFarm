use crate::blender::BlenderError;
use serde::{Deserialize, Serialize};
use std::env::consts;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct BlenderDownloadLink {
    name: String,
    ext: String,
    url: Url,
}

impl BlenderDownloadLink {
    pub fn new(name: String, ext: String, url: Url) -> Self {
        Self { name, ext, url }
    }

    // Currently being used for MacOS (I wonder if I need to do the same for windows?)
    #[cfg(target_os = "macos")]
    fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<(), BlenderError> {
        fs::create_dir_all(&dst).unwrap();
        for entry in fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                Self::copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name())).unwrap();
            } else {
                fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
            }
        }
        Ok(())
    }

    /// Extract tar.xz file from destination path, and return blender executable path
    #[cfg(target_os = "linux")]
    pub fn extract_content(
        download_path: impl AsRef<Path>,
        folder_name: &str,
    ) -> Result<PathBuf, BlenderError> {
        use std::fs::File;
        use tar::Archive;
        use xz::read::XzDecoder;

        // Get file handler to download location
        let file = File::open(&download_path).unwrap();

        // decode compressed xz file
        let tar = XzDecoder::new(file);

        // unarchive content from decompressed file
        let mut archive = Archive::new(tar);

        // generate destination path
        let destination = download_path.as_ref().parent().unwrap();

        // extract content to destination
        archive.unpack(destination).unwrap();

        // return extracted executable path
        Ok(destination.join(folder_name).join("blender"))
    }

    // TODO: Test this on macos
    /// Mounts dmg target to volume, then extract the contents to a new folder using the folder_name,
    /// lastly, provide a path to the blender executable inside the content.
    #[cfg(target_os = "macos")]
    pub fn extract_content(
        download_path: impl AsRef<Path>,
        folder_name: &str,
    ) -> Result<PathBuf, BlenderError> {
        use dmg::Attach;

        let source = download_path.as_ref();

        // generate destination path
        let dst = source
            .parent()
            .unwrap()
            .join(folder_name)
            .join("Blender.app");

        // TODO: wonder if this is a good idea?
        if !dst.exists() {
            let _ = fs::create_dir_all(&dst)?;
        }

        // attach dmg to volume
        let dmg = Attach::new(&source).attach()?;

        // create source path from mount point
        let src = PathBuf::from(&dmg.mount_point.join("Blender.app"));

        // Extract content inside Blender.app to destination
        let _ = Self::copy_dir_all(&src, &dst).unwrap();

        // detach dmg volume
        dmg.detach()?;

        // return path with additional path to invoke blender directly
        Ok(dst.join("Contents/MacOS/Blender"))
    }

    // TODO: implement handler to unpack .zip files
    // TODO: Check and see if we need to return the .exe extension or not?
    #[cfg(target_os = "windows")]
    pub fn extract_content(
        download_path: impl AsRef<Path>,
        folder_name: &str,
    ) -> Result<PathBuf, BlenderError> {
        let output = download_path.parent().unwrap().join(folder_name);
        todo!("Need to impl. window version of file extraction here");
        Ok(output.join("/blender.exe"))
    }

    pub fn download_and_extract(
        &self,
        destination: impl AsRef<Path>,
    ) -> Result<PathBuf, BlenderError> {
        let dir = destination.as_ref();

        // Download the file from the internet and save it to blender data folder
        let body = match reqwest::blocking::get(self.url.as_str()) {
            Ok(response) => match response.bytes() {
                Ok(body) => body,
                Err(e) => {
                    return Err(BlenderError::FetchError(format!(
                        "Error while fetching downloads: {}",
                        e
                    )))
                }
            },
            Err(_) => {
                return Err(BlenderError::DownloadNotFound {
                    arch: consts::ARCH.to_string(),
                    os: consts::OS.to_string(),
                    url: self.url.to_string(),
                })
            }
        };

        let target = &dir.join(&self.name);

        if let Err(e) = fs::write(target, &body) {
            return Err(e.into());
        }
        let extract_folder = self.name.replace(&self.ext, "");

        let executable_path = Self::extract_content(target, &extract_folder).unwrap();
        Ok(executable_path)
    }
}
