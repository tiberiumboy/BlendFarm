use crate::services::category::BlenderCategoryError;
use crate::services::packages::bundle::Bundle;
use crate::services::packages::package::{Package, PackageT};
#[cfg(target_os = "macos")]
use crate::utils::MACOS_PATH;
use crate::{services::packages::download_link::DownloadLink, utils::get_extension};
use semver::Version;
use serde::{Deserialize, Serialize};
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use std::env::consts::OS;
use std::io::Error as IoError;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Downloaded {
    pub origin: DownloadLink,
    pub content: PathBuf,
}

impl Downloaded {
    // return the path of execution entry point (mac specific)
    fn get_executable_path(&self) -> Result<PathBuf, BlenderCategoryError> {
        let path = self.get_content_path()?;
        // Do we want to return the absolute executable path, or path to application source?
        #[cfg(target_os = "macos")]
        return Ok(path.join("Blender.app").join(MACOS_PATH));
        #[cfg(target_os = "linux")]
        return Ok(path.join("blender"));
        #[cfg(target_os = "windows")]
        return Ok(path.join("Blender.exe"));
    }

    // return the destination of application source and bundle (mac specific)
    fn get_content_path(&self) -> Result<PathBuf, BlenderCategoryError> {
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        return Err(BlenderCategoryError::UnsupportedOS(OS.into()));

        let ext = get_extension().map_err(|e| BlenderCategoryError::UnsupportedOS(e.into()))?;
        // A hack- get_extension does not include period, so we need to include the period to generate the folder name correctly
        // TODO: is there a better way to handle this extension replacement?
        let folder_name = self.origin.file_name.replace(&format!(".{ext}"), ""); // remove the extension
        Ok(self.content.parent().unwrap().join(folder_name))
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
        destination: impl AsRef<Path>,
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

        let destination = destination.as_ref();

        // extract content to destination
        archive.unpack(destination)?;

        // return extracted executable path
        Ok(destination.join("blender"))
    }

    /// Mounts dmg target to volume, then extract the contents to a new folder using the folder_name,
    /// lastly, provide a path to the blender executable inside the content.
    #[cfg(target_os = "macos")]
    fn extract_content(
        download_path: impl AsRef<Path>,
        destination: impl AsRef<Path>,
    ) -> Result<PathBuf, IoError> {
        use crate::utils::MACOS_PATH;
        use dmg::Attach;
        use std::fs;
        const APP_NAME: &str = "Blender.app";

        let source = download_path.as_ref();
        let dst = destination.as_ref();

        if !dst.exists() {
            let _ = fs::create_dir_all(&dst)?;
        }

        // now append the app name and set that as our unpack destination.
        let dst = dst.join(APP_NAME);

        let dmg = Attach::new(&source).attach()?; // attach dmg to volume
        let src = PathBuf::from(&dmg.mount_point.join(APP_NAME)); // create source path from mount point
        Self::copy_dir_all(&src, &dst)?; // Extract content inside Blender.app to destination
        dmg.detach()?; // detach dmg volume
        Ok(dst.join(MACOS_PATH)) // return path with additional path to invoke blender directly
    }

    // TODO: verify this is working for windows (.zip)?
    #[cfg(target_os = "windows")]
    fn extract_content(
        download_path: impl AsRef<Path>,
        folder_name: &str, // TODO: Change this to destination instead.
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

    pub fn extract(self) -> Package {
        let destination = match self.get_content_path() {
            Ok(path) => path,
            Err(e) => {
                eprintln!("Unable to find content path! {e:?}");
                return Package::Downloaded(self);
            }
        };
        match Self::extract_content(&self.content, destination) {
            Ok(executable_path) => Package::Bundle(Bundle::new(self, executable_path)),
            Err(e) => {
                eprintln!("Unable to Extract Contents: {e:?}");
                Package::Downloaded(self)
            }
        }
    }
}

impl PackageT for Downloaded {
    fn get_version(&self) -> &Version {
        self.origin.get_version()
    }
}
