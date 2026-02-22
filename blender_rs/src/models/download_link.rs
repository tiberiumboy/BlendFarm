use crate::{blender::Blender, utils::get_extension};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    fs, io::{Error as IoError, Read}, marker::PhantomData, path::{Path, PathBuf}
};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct NotDownloaded;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct Downloaded;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct Unpacked;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DownloadLink<State = NotDownloaded> {
    // Why is this method public?
    /*pub*/ name: String,
    url: Url,
    version: Version,
    download_path: Option<PathBuf>,
    executable_path: Option<PathBuf>,
    state: PhantomData<State>,
}

impl DownloadLink<NotDownloaded> {
    pub fn new(name: String, url: Url, version: Version) -> Self {
        Self { 
            name, 
            url, 
            version, 
            download_path: None,
            executable_path: None,
            state: PhantomData::<NotDownloaded> }
    }

    // at this point here we will download the link and return an updated state
    pub fn download(self, destination: impl AsRef<Path>) -> Result<DownloadLink<Downloaded>, IoError> {

        // got a permission denied here? Interesting?
        // I need to figure out why and how I can stop this from happening?
        fs::create_dir_all(&destination)?;

        // create a target name
        let target = &destination.as_ref().join(&self.name);
        
        // Check and see if we haven't download the file already
        if !target.exists() {
            // Download the file from the internet
            let mut response = ureq::get(self.url.as_str()).call().map_err(IoError::other)?;
            let mut body: Vec<u8> = Vec::new();
            // TODO: See if there's a better way to save or store the file?
            // It's like why can't we stream directly to io?
            if let Err(e) = response.body_mut().as_reader().read_to_end(&mut body) {
                eprintln!("Fail to read data from response! {e:?}");
            }
            // save the content to target
            fs::write(target, &body)?;
        }
        
        // Assume the file we download are zipped/compressed.
        Ok(DownloadLink::<Downloaded>{
            name: self.name,
            url: self.url,
            version: self.version,
            download_path: Some(target.to_path_buf()),
            executable_path: None,
            state: PhantomData::<Downloaded>,
        })
    }
}

impl DownloadLink<Downloaded> {

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
    fn extract_content(
        &self,
        folder_name: &str,
    ) -> Result<PathBuf, IoError> {
        use std::fs::File;
        use tar::Archive;
        use xz::read::XzDecoder;

        let path = &self.download_path.as_ref().expect("Should have valid path!");
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
        &self,
        folder_name: &str,
    ) -> Result<PathBuf, Error> {
        use dmg::Attach;

        let source = &self.download_path.as_ref().expect("Should have valid path!");
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

    // TODO: verify this is working for windows (.zip)?
    #[cfg(target_os = "windows")]
    fn extract_content(
        &self,
        folder_name: &str,
    ) -> Result<PathBuf, Error> {
        use std::fs::File;
        use zip::ZipArchive;

        let source = &self.download_path.as_ref().expect("Must have valid path!");
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

    // pub fn from_path(path: PathBuf) -> Result<DownloadLink::<Downloaded> {
    //     Ok(DownloadLink::<Downloaded> {
    //         name
    //     })
    // }

    pub fn extract(self) -> Result<DownloadLink::<Unpacked>, IoError> {
        // as painful as it may be, I wish I didn't do this weird cfg trick...
        // precheck qualification
        let ext = get_extension()
            .map_err(|e| IoError::other(format!("Cannot run blender under this OS: {}!", e)))?;
        // create a target folder name to extract content to.
        let folder_name = &self.name.replace(&ext, "");
        let executable_path = &self.extract_content(folder_name)?;
        
        Ok(DownloadLink::<Unpacked>{ 
            name: self.name,
            url: self.url,
            download_path: self.download_path,
            executable_path: Some(executable_path.to_path_buf()),
            version: self.version,
            state: PhantomData::<Unpacked>
        })     
    }
}

impl DownloadLink<Unpacked> {

    pub fn get_blender(&self) -> Result<Blender, IoError> {
        // TODO: Eliminate clone + expect() methods
        let executable = self.executable_path.clone().expect("Should have valid blender?");
        let blender = Blender::from_executable(executable).map_err(|e| IoError::other(e))?;
        Ok(blender)
    }
}

impl<State> DownloadLink<State> {
    pub fn get_version(&self) -> &Version {
        &self.version
    }

    pub fn get_parent(&self) -> String {
        format!("Blender{}.{}", self.version.major, self.version.minor)
    }

    pub fn get_url(&self) -> &Url {
        &self.url
    }
}

impl AsRef<Version> for DownloadLink {
    fn as_ref(&self) -> &Version {
        &self.version
    }
}
