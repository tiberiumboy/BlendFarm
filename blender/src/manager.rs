/*
    Developer blog:
    This manager class will serve the following purpose:
    - Keep track of blender installation on this active machine.
    - Prevent downloading of the same blender version if we have one already installed.
    - If user fetch for list of installation, verify all path exist before returning the list.
    - Implements download and install code
*/
use crate::blender::Blender;
use crate::models::{category::BlenderCategory, download_link::DownloadLink};
use crate::page_cache::PageCache;

use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::{fs, path::PathBuf};
use thiserror::Error;
use url::Url;

// I would like this to be a feature only crate. blender by itself should be lightweight and interface with the program directly.
// could also implement serde as optionals?
#[derive(Debug, Error)]
pub enum ManagerError {
    #[error("Unsupported OS: {0}")]
    UnsupportedOS(String),
    #[error("Unsupported Archtecture: {0}")]
    UnsupportedArch(String),
    #[error("Unable to extract content: {0}")]
    UnableToExtract(String),
    #[error("Unable to fetch download from the source! {0}")]
    FetchError(String),
    #[error("Cannot find target download link for blender! os: {os} | arch: {arch} | url: {url}")]
    DownloadNotFound {
        arch: String,
        os: String,
        url: String,
    },
    #[error("Unable to fetch blender! {0}")]
    RequestError(String),
    // TODO: Find meaningful error message to represent from this struct class?
    #[error("IO Error: {0}")]
    IoError(String),
    #[error("Url ParseError: {0}")]
    UrlParseError(String),
    #[error("Page cache error: {0}")]
    PageCacheError(String),
    #[error("Blender error: {source}")]
    BlenderError {
        #[from]
        source: crate::blender::BlenderError,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlenderConfig {
    /// List of installed blenders
    blenders: Vec<Blender>,

    /// Install path. By default set to `$HOME/Downloads/Blender`
    install_path: PathBuf,

    /// Auto save on drop
    auto_save: bool,
}

impl BlenderConfig {
    /// Remove any invalid blender path entry from BlenderConfig
    pub fn remove_invalid_blender_path(&mut self) {
        self.blenders.retain(|x| x.get_executable().exists());
    }
}

pub struct Manager {
    /// Store all known installation of blender directory information
    config: BlenderConfig,
    list: Vec<BlenderCategory>,
    download_links: Vec<DownloadLink>,
    cache: PageCache,
    has_modified: bool, // detect if the configuration has changed.
}

impl Default for Manager {
    // the default method implement should be private because I do not want people to use this function.
    // instead they should rely on "load" function instead.
    fn default() -> Self {
        let install_path = dirs::download_dir().unwrap().join("Blender");
        let config = BlenderConfig {
            blenders: Vec::new(),
            install_path,
            auto_save: true,
        };
        let mut cache =
            PageCache::load().expect("Page Cache should have permission to load content!");

        let list = Self::fetch_categories(&mut cache).unwrap_or_else(|_| Vec::new());

        Self {
            config,
            list,
            download_links: Vec::new(),
            cache,
            has_modified: false,
        }
    }
}

impl Manager {
    fn fetch_categories(cache: &mut PageCache) -> Result<Vec<BlenderCategory>, Error> {
        let parent = Url::parse("https://download.blender.org/release/").unwrap();
        let content = cache.fetch(&parent)?;

        // Omit any blender version 2.8 and below
        let pattern =
            r#"<a href=\"(?<url>.*)\">Blender(?<major>[3-9]|\d{2,}).(?<minor>\d*).*\/<\/a>"#;
        let regex = Regex::new(pattern).map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!("Unable to create new Regex pattern! {e:?}"),
            )
        })?;

        let mut list: Vec<BlenderCategory> = regex
            .captures_iter(&content)
            .map(|c| {
                let (_, [url, major, minor]) = c.extract();
                let url = parent.join(url).ok()?;
                let major = major.parse().ok()?;
                let minor = minor.parse().ok()?;
                Some(BlenderCategory::new(url, major, minor))
            })
            .flatten()
            .collect();

        list.sort_by(|a, b| b.cmp(a));
        Ok(list)
    }

    fn set_config(&mut self, config: BlenderConfig) -> &mut Self {
        self.config = config;
        self
    }

    /// Returns the directory where the configuration file is placed.
    /// This is stored under 
    pub fn get_config_dir() -> PathBuf {
        let path = dirs::config_dir().unwrap().join("BlendFarm");
        fs::create_dir_all(&path).expect("Unable to create directory!");
        path
    }

    // this path should always be fixed and stored under machine specific.
    // this path should not be shared across machines.
    fn get_config_path() -> PathBuf {
        Self::get_config_dir().join("BlenderManager.json")
    }

    /// Download Blender of matching version, install on this machine, and returns blender struct.
    /// This function will update PageCache if not previously visited. Hence mutation requirement.
    pub fn download_blender(&mut self, version: &Version) -> Result<Blender, ManagerError> {
        // TODO: As a extra security measure, I would like to verify the hash of the content before extracting the files.
        let arch = std::env::consts::ARCH.to_owned();
        let os = std::env::consts::OS.to_owned();

        let download_link =
            self.get_blender_link_by_version(version)
                .ok_or(ManagerError::DownloadNotFound {
                    arch,
                    os,
                    url: format!(
                        "Blender version {}.{} was not found!",
                        version.major, version.minor
                    ),
                })?;

        // need to fetch category name such as "Blender4.1"
        let destination = self.config.install_path.join(&download_link.name);

        // got a permission denied here? Interesting?
        // I need to figure out why and how I can stop this from happening?
        fs::create_dir_all(&destination).unwrap();

        // TODO: verify this is working for windows (.zip)?
        let destination = download_link
            .download_and_extract(&destination)
            .map_err(|e| ManagerError::IoError(e.to_string()))?;

        let blender = Blender::from_executable(destination)
            .map_err(|e| ManagerError::BlenderError { source: e })?;

        self.add_blender(blender.clone());
        self.save().unwrap();
        Ok(blender)
    }

    // Save the configuration to local
    // do I need to save? What's the reason behind this?
    fn save(&self) -> Result<(), ManagerError> {
        // strictly speaking, this function shouldn't crash...
        let data = serde_json::to_string(&self.config).unwrap();
        let path = Self::get_config_path();
        fs::write(path, data).map_err(|e| ManagerError::IoError(e.to_string()))
    }

    /// Return a reference to the vector list of all known blender installations
    pub fn get_blenders(&self) -> &Vec<Blender> {
        &self.config.blenders
    }

    /// Load the manager data from the config file.
    pub fn load() -> Self {
        // load from a known file path (Maybe a persistence storage solution somewhere?)
        // if the config file does not exist on the system, create a new one and return a new struct instead.
        let path = Self::get_config_path();
        let mut data = Self::default();
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(mut config) = serde_json::from_str::<BlenderConfig>(&content) {
                config.remove_invalid_blender_path();
                data.set_config(config);
                return data;
            } else {
                println!("Fail to deserialize manager config file!");
            }
        } else {
            println!("File not found! Creating a new default one!");
        };
        // default case, create a new manager data and save it.
        let data = Self::default();
        match data.save() {
            Ok(()) => println!("New manager data created and saved!"),
            // TODO: Find a better way to handle this error.
            Err(e) => println!("Unable to save new manager data! {:?}", e),
        }
        data
    }

    pub fn get_install_path(&self) -> &Path {
        &self.config.install_path
    }

    /// Set path for blender download and installation
    pub fn set_install_path(&mut self, new_path: &Path) {
        // Consider the design behind this. Should we move blender installations to new path?
        self.config.install_path = new_path.to_path_buf().clone();
        self.has_modified = true;
    }

    /// Add a new blender installation to the manager list.
    pub fn add_blender(&mut self, blender: Blender) {
        self.config.blenders.push(blender);
        self.has_modified = true;
    }

    /// Check and add a local installation of blender to manager's registry of blender version to use from.
    pub fn add_blender_path(&mut self, path: &impl AsRef<Path>) -> Result<Blender, ManagerError> {
        let path = path.as_ref();
        let extension = BlenderCategory::get_extension().map_err(ManagerError::UnsupportedOS)?;

        let path = if path
            .extension()
            .is_some_and(|e| extension.contains(e.to_str().unwrap()))
        {
            // Create a folder name from given path
            let folder_name = &path
                .file_name()
                .unwrap()
                .to_os_string()
                .to_str()
                .unwrap()
                .replace(&extension, "");

            DownloadLink::extract_content(path, folder_name)
                .map_err(|e| ManagerError::UnableToExtract(e.to_string()))
        } else {
            // for MacOS - User will select the app bundle instead of actual executable, We must include the additional path
            match std::env::consts::OS {
                "macos" => Ok(path.join("Contents/MacOS/Blender")),
                _ => Ok(path.to_path_buf()),
            }
        }?;
        let blender =
            Blender::from_executable(path).map_err(|e| ManagerError::BlenderError { source: e })?;

        // I would have at least expect to see this populated?
        self.add_blender(blender.clone());
        // TODO: This is a hack - Would prefer to understand why program does not auto save file after closing.
        // Or look into better saving mechanism than this.
        let _ = self.save();
        Ok(blender)
    }

    /// Remove blender installation from the manager list.
    pub fn remove_blender(&mut self, blender: &Blender) {
        self.config.blenders.retain(|x| x.eq(blender));
        self.has_modified = true;
    }

    /// Deletes the parent directory that blender reside in. This might be a dangerous function as this involves removing the directory blender executable is in.
    /// TODO: verify that this doesn't break macos path executable... Why mac gotta be special with appbundle?
    pub fn delete_blender(&mut self, _blender: &Blender) {
        // this deletes blender from the system. You have been warn!
        // todo!("Exercise with caution!");
        // fs::remove_dir_all(_blender.get_executable().parent().unwrap()).unwrap();
        self.remove_blender(_blender);
    }

    // TODO: Name ambiguous - clarify method name to be clear and explicit
    /// This will first check if blender is installed locally, otherwise download the version online.
    pub fn fetch_blender(&mut self, version: &Version) -> Result<Blender, ManagerError> {
        match self.have_blender(version) {
            Some(blender) => Ok(blender.clone()),
            None => self.download_blender(version),
        }
    }

    // TODO: Refactor this method to provide already established DownloadLinks from the manager instead.
    // Category struct is going away and will be used to fetch download links only. Nothing more beyond that.
    pub fn fetch_download_list(&self) -> Option<Vec<DownloadLink>> {
        match &self.download_links.is_empty() {
            false => Some(self.download_links.clone()),
            true => None,
        }
    }

    pub fn have_blender(&self, version: &Version) -> Option<&Blender> {
        self.config
            .blenders
            .iter()
            .find(|x| x.get_version().eq(version))
    }

    pub fn have_blender_partial(&self, major: u64, minor: u64) -> Option<&Blender> {
        self.config.blenders.iter().find(|x| {
            let v = x.get_version();
            v.major.eq(&major) && v.minor.eq(&minor)
        })
    }

    // TODO: Try to remove unwrap as much as possible
    /// Fetch the latest version of blender available from Blender.org
    /// this function might be ambiguous. Should I use latest_local or latest_online?
    pub fn latest_local_avail(&mut self) -> Option<Blender> {
        // in this case I need to contact Manager class or BlenderDownloadLink somewhere and fetch the latest blender information
        let mut data = self.config.blenders.clone();
        data.sort();
        let value = data.first().map(|v| v.to_owned());
        value
    }

    // find a way to hold reference to blender home here?
    // split this function
    pub fn download_latest_version(&mut self) -> Result<Blender, ManagerError> {
        // in this case - we need to fetch the latest version from somewhere, download.blender.org will let us fetch the parent before we need to dive into
        // TODO: Find a way to replace these unwrap()
        let category = &self.list.first().map_or(Err(ManagerError::RequestError("Category list is empty! Did you clear the cache? Please connect to the internet to retrieve blender download list".to_string())), |c| Ok(c))?;

        // TODO how do I get around this? I moved PageCache to manager class instead of BlenderHome.
        // This kinda open up a whole can of worms.
        let link = category.fetch_latest(&mut self.cache).unwrap();
        let destination = self.config.install_path.join(&link.get_parent());

        // got a permission denied here? Interesting?
        // I need to figure out why and how I can stop this from happening?
        fs::create_dir_all(&destination).unwrap();

        let path = link
            .download_and_extract(&destination)
            .map_err(|e| ManagerError::IoError(e.to_string()))?;

        // I would expect this to always work?
        let blender = Blender::from_executable(path).expect("Invalid Blender executable!");
        self.config.blenders.push(blender.clone());
        Ok(blender)
    }

    pub fn get_blender_link_by_version(&mut self, version: &Version) -> Option<DownloadLink> {
        self.list
            .iter()
            .find(|c| c.version_match(version))
            .map_or(None, |c| {
                c.retrieve(version, &mut self.cache)
                    .map_or(None, |l| Some(l))
            })
    }

    // I may want to change this to see if I'm picking the one from locally installed or from remote
    pub fn get_latest_version_patch(&mut self, major: u64, minor: u64) -> Option<Version> {
        // Get the latest patch from blender home
        self.list
            .iter()
            .find(|v| v.partial_version_match(major, minor))
            .map_or(None, |c| {
                c.fetch_latest(&mut self.cache)
                    .map_or(None, |l| Some(l.get_version().clone()))
            })
    }
}

impl AsRef<PathBuf> for Manager {
    fn as_ref(&self) -> &PathBuf {
        &self.config.install_path
    }
}

// impl AsRef<Vec<BlenderCategory>> for Manager {
//     fn as_ref(&self) -> &Vec<BlenderCategory> {
//         &self.list
//     }
// }

impl Drop for Manager {
    fn drop(&mut self) {
        if self.has_modified || self.config.auto_save {
            if let Err(e) = self.save() {
                eprintln!("Error saving manager file: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_pass() {
        let _manager = Manager::load();
    }
}
