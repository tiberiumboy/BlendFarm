use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    env::consts,
    fs,
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
};
use url::Url;

use crate::page_cache::PageCache;

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

        // TODO: wonder if this is a good idea?
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlenderCategory {
    pub name: String,
    pub url: Url,
    pub major: u64,
    pub minor: u64,
}

impl BlenderCategory {
    /// fetch current architecture (Currently support x86_64 or aarch64 (apple silicon))
    fn get_valid_arch() -> Result<String, String> {
        match consts::ARCH {
            "x86_64" => Ok("x64".to_owned()),
            "aarch64" => Ok("arm64".to_owned()),
            arch => Err(format!(
                "Architecture type \"{}\" is not supported!",
                arch.to_string()
            )),
        }
    }

    /// Return extension matching to the current operating system (Only display Windows(.zip), Linux(.tar.xz), or macos(.dmg)).
    pub fn get_extension() -> Result<String, String> {
        match consts::OS {
            "windows" => Ok(".zip".to_owned()),
            "macos" => Ok(".dmg".to_owned()),
            "linux" => Ok(".tar.xz".to_owned()),
            os => Err(format!("Unsupported operating system: {}", os.to_string())),
        }
    }

    pub fn new(name: String, url: Url, major: u64, minor: u64) -> Self {
        Self {
            name,
            url,
            major,
            minor,
        }
    }

    // for some reason I was fetching this multiple of times already. This seems expensive to call for some reason?
    // also, strange enough, the pattern didn't pick up anything?
    pub fn fetch(&self) -> Result<Vec<DownloadLink>, Error> {
        let mut cache = PageCache::load()?;
        let content = cache.fetch(&self.url)?;
        let arch = Self::get_valid_arch().map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
        let ext = Self::get_extension().map_err(|e| Error::new(ErrorKind::Unsupported, e))?;

        // Regex rules - Find the url that matches version, computer os and arch, and the extension.
        // - There should only be one entry matching for this. Otherwise return error stating unable to find download path
        let pattern = format!(
            r#"<a href=\"(?<url>.*)\">(?<name>.*-{}\.{}\.(?<patch>\d*.)-{}.*{}*.{})<\/a>"#,
            self.major,
            self.minor,
            consts::OS,
            arch,
            ext,
        );

        let regex = Regex::new(&pattern).unwrap();
        // for (_, [url, name, patch]) in
        let vec = regex
            .captures_iter(&content)
            .map(|c| {
                let (_, [url, name, patch]) = c.extract();
                let url = self.url.join(url).ok()?;
                let patch = patch.parse().ok()?;
                let version = Version::new(self.major, self.minor, patch);
                Some(DownloadLink::new(name.to_owned(), url, version))
            })
            .flatten()
            .collect();

        Ok(vec)
    }

    pub fn fetch_latest(&self) -> Result<DownloadLink, Error> {
        let mut list = self.fetch()?;
        list.sort_by(|a, b| b.cmp(a));
        let entry = list.first().ok_or(Error::other("Not found?"))?;
        Ok(entry.clone())
    }

    pub fn retrieve(&self, version: &Version) -> Result<DownloadLink, Error> {
        let list = self.fetch()?;
        let entry = list
            .iter()
            .find(|dl| dl.version.eq(version))
            .ok_or(Error::new(
                ErrorKind::NotFound,
                format!("Unable to find matching blender version! {version}"),
            ))?;
        Ok(entry.to_owned())
    }
}

#[derive(Debug)]
pub struct BlenderHome {
    pub list: Vec<BlenderCategory>,
}

impl BlenderHome {
    // this might be a bit dangerous? we'll see?
    pub fn new() -> Result<Self, Error> {
        // I would hope that this line should never fail...?
        let home = "https://download.blender.org/release/";
        let parent = Url::parse(home).unwrap();

        // In the original code - there's a comment implying we should use cache as much as possible to avoid IP Blacklisted. TODO: Verify this in Blender community about this.
        let mut cache = PageCache::load()?;

        // fetch the content of the subtree information
        let content = cache.fetch(&parent)?;

        // Omit any blender version 2.8 and below, according to discord user "tigos" from Blender - https://discord.com/channels/185590609631903755/869790788530352188/1286766670475559036
        let pattern = r#"<a href=\"(?<url>.*)\">(?<name>Blender(?<major>[3-9]|\d{2,}).(?<minor>\d*).*)\/<\/a>"#;
        let regex = Regex::new(pattern).unwrap();
        let collection = regex
            .captures_iter(&content)
            .map(|c| {
                let (_, [url, name, major, minor]) = c.extract();
                let url = parent.join(url).ok()?;
                let major = major.parse().ok()?;
                let minor = minor.parse().ok()?;
                Some(BlenderCategory::new(name.to_owned(), url, major, minor))
            })
            .flatten()
            .collect();

        Ok(Self { list: collection })
    }
}
