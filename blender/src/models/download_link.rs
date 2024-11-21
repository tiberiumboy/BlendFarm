use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Error,
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

        // generate destination path
        let dst = source
            .parent()
            .unwrap()
            .join(folder_name)
            .join("Blender.app");
        if !dst.exists() {
            let _ = fs::create_dir_all(&dst)?;
        }

        let dmg = Attach::new(&source).attach()?; // attach dmg to volume
        let src = PathBuf::from(&dmg.mount_point.join("Blender.app")); // create source path from mount point
        let _ = Self::copy_dir_all(&src, &dst).unwrap(); // Extract content inside Blender.app to destination
        dmg.detach()?; // detach dmg volume
        Ok(dst.join("Contents/MacOS/Blender")) // return path with additional path to invoke blender directly
    }

    // TODO: implement handler to unpack .zip files
    // TODO: Check and see if we need to return the .exe extension or not?
    #[cfg(target_os = "windows")]
    pub fn extract_content(
        download_path: impl AsRef<Path>,
        folder_name: &str,
    ) -> Result<PathBuf, Error> {
        use std::fs::File;
        use zip::ZipArchive;
        let output = download_path.as_ref().join(folder_name);

        let file = File::open(download_path).unwrap();

        // how do I unzip files?
        let mut archive = ZipArchive::new(file).unwrap();
        archive.extract(&output).unwrap();

        Ok(output.join("Blender.exe".to_owned()))
    }

    pub fn fetch_version_url(version: &Version) -> Result<DownloadLink, Error> {
        // TODO: Find a good reason to keep this?

        Ok(DownloadLink::new(
            "".to_owned(),
            Url::parse("https://www.google.com").unwrap(),
            version.clone(),
        ))
    }

    pub fn download_and_extract(
        &self,
        destination: impl AsRef<Path>,
        // TODO: Find out why the warning appears - It seems like I might be wrapping something huge inside error?
    ) -> Result<PathBuf, Error> {
        let dir = destination.as_ref();

        // Download the file from the internet and save it to blender data folder
        let response = ureq::get(self.url.as_str())
            .call()
            .map_err(|e: ureq::Error| Error::other(e))?;

        let len: usize = response
            .header("Content-Length")
            .unwrap()
            .parse()
            .unwrap_or(0);

        let mut body: Vec<u8> = Vec::with_capacity(len);
        let mut heap = response.into_reader();
        // TODO: Maybe this is the culprit?
        heap.read_to_end(&mut body)?;

        let target = &dir.join(&self.name);
        fs::write(target, &body)?;

        let executable_path = Self::extract_content(target, &self.name)?;
        Ok(executable_path)
    }
}

impl AsRef<Version> for DownloadLink {
    fn as_ref(&self) -> &Version {
        &self.version
    }
}
