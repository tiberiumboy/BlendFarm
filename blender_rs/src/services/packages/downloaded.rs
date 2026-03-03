use crate::services::category::BlenderCategoryError;
use crate::services::packages::bundle::Bundle;
use crate::services::packages::package::PackageT;
use crate::utils::MACOS_PATH;
use crate::{services::packages::download_link::DownloadLink, utils::get_extension};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::env::consts::OS;
use std::io::Error as IoError;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Downloaded {
    pub origin: DownloadLink,
    pub content: PathBuf,
}

impl Downloaded {
    fn get_executable_path(&self) -> Result<PathBuf, BlenderCategoryError> {
        let ext = get_extension()
            .map_err(|e| IoError::other(format!("Cannot run blender under this OS: {}!", e)))?;
        let folder_name = self.origin.file_name.replace(&ext, ""); // remove the extension
        let parent_folder = self.content.parent().unwrap().join(folder_name);

        // per different operating system, we need to craft a path that points to blender executable. It various across all operating system.
        match OS {
            "macos" => Ok(parent_folder.join("Blender.app").join(MACOS_PATH)),
            "linux" => Ok(parent_folder.join("blender")),
            "windows" => Ok(parent_folder.join("Blender.exe")),
            _ => Err(BlenderCategoryError::UnsupportedOS(OS.into())),
        }
    }

    // Currently being used for MacOS (I wonder if I need to do the same for windows?)
    #[cfg(target_os = "macos")]
    fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<(), IoError> {
        use std::fs;

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
    fn extract_content(
        download_path: impl AsRef<Path>,
        folder_name: &str,
    ) -> Result<PathBuf, IoError> {
        use std::fs::File;
        use tar::Archive;
        use xz::read::XzDecoder;

        let path = download_path.as_ref();
        // Get file handler to download location
        let file = File::open(path)?;

        // decode compressed xz file
        let tar = XzDecoder::new(file);

        // unarchive content from decompressed file
        let mut archive = Archive::new(tar);

        // generate destination path
        let destination = path.parent().unwrap();

        // extract content to destination
        archive.unpack(destination)?;

        // return extracted executable path
        Ok(destination.join(folder_name).join("blender"))
    }

    // TODO: Test this on macos
    /// Mounts dmg target to volume, then extract the contents to a new folder using the folder_name,
    /// lastly, provide a path to the blender executable inside the content.
    #[cfg(target_os = "macos")]
    fn extract_content(
        download_path: impl AsRef<Path>,
        folder_name: &str,
    ) -> Result<PathBuf, IoError> {
        use crate::utils::MACOS_PATH;
        use dmg::Attach;
        use std::fs;

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
        Ok(dst.join(MACOS_PATH)) // return path with additional path to invoke blender directly
    }

    // TODO: verify this is working for windows (.zip)?
    #[cfg(target_os = "windows")]
    fn extract_content(
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

    pub fn check_unpacked(self) -> Result<Bundle, Downloaded> {
        // here we would navigate to the extracted directory based on the rules generated in this struct, if the path to executable exist, then return Bundle, otherwise return itself.
        // assuming the logic goes - in the same path destination as compressed content, there should be a folder containing the extracted content.
        if let Ok(executable_path) = self.get_executable_path() {
            if executable_path.exists() {
                return Ok(Bundle::new(self, executable_path));
            }
        }
        Err(self)
    }

    pub fn extract(self, destination: PathBuf) -> Result<Bundle, IoError> {
        let ext = get_extension()
            .map_err(|e| IoError::other(format!("Cannot run blender under this OS: {}!", e)))?;
        // create a target folder name to extract content to.
        let name = &self.origin.file_name;
        let folder_name = &name.replace(&ext, "");
        let executable_path = Self::extract_content(destination, folder_name)?;
        Ok(Bundle::new(self, executable_path))
    }
}

impl PackageT for Downloaded {
    fn get_version(&self) -> &Version {
        self.origin.get_version()
    }
}
