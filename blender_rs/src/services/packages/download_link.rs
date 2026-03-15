use crate::services::{
    category::BlenderCategoryError,
    packages::{downloaded::Downloaded, package::PackageT},
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Error as IoError, Read},
    path::{Path, PathBuf},
};
use url::Url;

// TODO: Could I implement Hash traits? Use version as hash id
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DownloadLink {
    pub version: Version,
    pub file_name: String, // contains extensions!
    pub download_url: Url,
}

impl DownloadLink {
    pub fn new(url: Url, version: Version) -> Result<DownloadLink, BlenderCategoryError> {
        let name = url
            .path_segments()
            .ok_or(BlenderCategoryError::NotFound)?
            .last()
            .ok_or(BlenderCategoryError::NotFound)?
            .to_owned();

        Ok(Self {
            file_name: name,
            download_url: url,
            version,
        })
    }

    fn download_path(&self, install_path: impl AsRef<Path>) -> PathBuf {
        install_path.as_ref().join(&self.file_name)
    }

    // Destination expects absolute path
    pub fn content_exist(self, destination: impl AsRef<Path>) -> Result<Downloaded, DownloadLink> {
        let path = self.download_path(destination);
        if path.exists() {
            let downloaded = Downloaded {
                origin: self,
                content: path,
            };
            return Ok(downloaded);
        }
        Err(self)
    }

    // at this point here we will download the link and return an updated state
    pub fn download(self, destination: impl AsRef<Path>) -> Result<Downloaded, IoError> {
        // got a permission denied here? Interesting?
        // I need to figure out why and how I can stop this from happening?
        fs::create_dir_all(&destination)?;

        // create a target name
        let target = self.download_path(destination);

        // Check and see if we haven't download the file already
        if !target.exists() {
            // Download the file from the internet
            let mut response = ureq::get(self.download_url.as_str())
                .call()
                .map_err(IoError::other)?;
            let mut body: Vec<u8> = Vec::new();
            // TODO: See if there's a better way to save or store the file?
            // It's like why can't we stream directly to io?
            if let Err(e) = response.body_mut().as_reader().read_to_end(&mut body) {
                eprintln!("Fail to read data from response! {e:?}");
            }
            // save the content to target
            fs::write(&target, &body)?;
        }

        // Assume the file we download are zipped/compressed.
        Ok(Downloaded {
            origin: self,
            content: target,
        })
    }
}

impl PackageT for DownloadLink {
    fn get_version(&self) -> &Version {
        &self.version
    }
}

impl AsRef<Version> for DownloadLink {
    fn as_ref(&self) -> &Version {
        &self.version
    }
}
