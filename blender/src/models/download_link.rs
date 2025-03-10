use super::category::BlenderCategory;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Error, Read},
    path::{Path, PathBuf},
};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DownloadLink {
    pub name: String,
    url: Url,
    version: Version,
}

impl DownloadLink {
    /* private function impl */

    pub fn new(name: String, url: Url, version: Version) -> Self {
        Self { name, url, version }
    }

    pub fn get_version(&self) -> &Version {
        &self.version
    }

    // Currently being used for MacOS (I wonder if I need to do the same for windows?)
    #[cfg(target_os = "macos")]
    fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<(), Error> {
        fs::create_dir_all(&dst)?;
        for entry in fs::read_dir(src)? {
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
    // TODO: Tested on Linux - something didn't work right here. Need to investigate/debug through
    #[cfg(target_os = "linux")]
    pub fn extract_content(
        download_path: impl AsRef<Path>,
        folder_name: &str,
    ) -> Result<PathBuf, Error> {
        use std::fs::File;
        use tar::Archive;
        use xz::read::XzDecoder;

        // Get file handler to download location
        let file = File::open(&download_path)?;

        // decode compressed xz file
        let tar = XzDecoder::new(file);

        // unarchive content from decompressed file
        let mut archive = Archive::new(tar);

        // generate destination path
        let destination = download_path.as_ref().parent().unwrap();

        // extract content to destination
        archive.unpack(destination)?;

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
    ) -> Result<PathBuf, Error> {
        use dmg::Attach;

        let source = download_path.as_ref();
        let dst = source // generate destination path
            .parent()
            .unwrap()
            .join(folder_name)
            .join("Blender.app");

        if !dst.exists() {
            let _ = fs::create_dir_all(&dst)?;
        }

        let dmg = Attach::new(&source).attach()?; // attach dmg to volume
        let src = PathBuf::from(&dmg.mount_point.join("Blender.app")); // create source path from mount point
        Self::copy_dir_all(&src, &dst)?; // Extract content inside Blender.app to destination
        dmg.detach()?; // detach dmg volume
        Ok(dst.join("Contents/MacOS/Blender")) // return path with additional path to invoke blender directly
    }

    #[cfg(target_os = "windows")]
    pub fn extract_content(
        download_path: impl AsRef<Path>,
        folder_name: &str,
    ) -> Result<PathBuf, Error> {
        use std::fs::File;
        use zip::ZipArchive;

        let source = download_path.as_ref();
        //  On windows, unzipped content includes a new folder underneath. Instead of doing this, we will just unzip from the parent instead... weird
        let zip_loc = source.parent().unwrap();
        let output = zip_loc.join(folder_name);

        // check if the directory exist
        match &output.exists() {
            // if it does, check and see if blender exist.
            true => {
                // if it does exist, then we can skip extracting the file entirely.
                if output.join("Blender.exe").exists() {
                    return Ok(output.join("Blender.exe"));
                }
            }
            _ => {}
        }

        let file = File::open(source).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        if let Err(e) = archive.extract(zip_loc) {
            println!("Unable to extract content to target: {e:?}");
        }

        Ok(output.join("Blender.exe"))
    }

    // contains intensive IO operation
    // TODO: wonder why I'm not using BlenderError for this?
    pub fn download_and_extract(&self, destination: impl AsRef<Path>) -> Result<PathBuf, Error> {
        // precheck qualification
        let ext = BlenderCategory::get_extension()
            .map_err(|e| Error::other(format!("Cannot run blender under this OS: {}!", e)))?;

        let target = &destination.as_ref().join(&self.name);

        // Check and see if we haven't already download the file
        if !target.exists() {
            // Download the file from the internet and save it to blender data folder
            let mut response = ureq::get(self.url.as_str())
                .call()
                .map_err(|e: ureq::Error| Error::other(e))?;

            let mut body: Vec<u8> = Vec::new();
            if let Err(e) = response.body_mut().as_reader().read_to_end(&mut body) {
                eprintln!("Fail to read data from response! {e:?}");
            }
            fs::write(target, &body)?;
        }

        // create a target folder name to extract content to.
        let folder_name = &self.name.replace(&ext, "");
        let executable_path = Self::extract_content(target, folder_name)?;
        Ok(executable_path)
    }
}

impl AsRef<Version> for DownloadLink {
    fn as_ref(&self) -> &Version {
        &self.version
    }
}
