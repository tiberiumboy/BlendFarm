/*
    Developer blog:
    This manager class will serve the following purpose:
    - Keep track of blender installation on this active machine.
    - Prevent downloading of the same blender version if we have one already installed.
    - If user fetch for list of installation, verify all path exist before returning the list.
    - Implements download and install code
*/
use crate::blender::Blender;
use crate::models::download_link::{BlenderCategory, BlenderHome, DownloadLink};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{fs, path::PathBuf};
use thiserror::Error;

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
    //     // TODO: Find meaningful error message to represent from this struct class?
    #[error("IO Error: {0}")]
    IoError(String),
    #[error("Url ParseError: {0}")]
    UrlParseError(String),
    // TODO: may contain at least 272 bytes?
    #[error("Page cache error: {0}")]
    PageCacheError(String),
    // TODO: may contain at least 272 bytes?
    #[error("Blender error: {source}")]
    BlenderError {
        #[from]
        source: crate::blender::BlenderError,
    },
}

// #[cfg(feature = "manager")]
// I wanted to keep this struct private only to this library crate?
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manager {
    /// Store all known installation of blender directory information
    blenders: Vec<Blender>,
    install_path: PathBuf,
    auto_save: bool,
    #[serde(skip)]
    has_modified: bool,
}

// #[cfg(feature = "manager")]
impl Default for Manager {
    fn default() -> Self {
        let install_path = dirs::download_dir().unwrap().join("Blender");
        Self {
            blenders: Vec::new(),
            install_path,
            auto_save: true,
            has_modified: false,
        }
    }
}

// #[cfg(feature = "manager")]
impl Manager {
    // this path should always be fixed and stored under machine specific.
    // this path should not be shared across machines.
    fn get_config_path() -> PathBuf {
        let path = dirs::config_dir().unwrap().join("Blender");
        fs::create_dir_all(&path).expect("Unable to create directory!");
        path.join("BlenderManager.json")
    }

    // Download the specific version from download.blender.org
    fn download(&mut self, _version: &Version) -> Result<Blender, ManagerError> {
        // TODO: As a extra security measure, I would like to verify the hash of the content before extracting the files.
        // TODO: How did BlendFarm fetch all blender version?
        let mut blender_home =
            BlenderHome::new().map_err(|e| ManagerError::RequestError(e.to_string()))?;

        blender_home.list.sort();
        let category = blender_home.list.first().unwrap();
        dbg!(&category);

        let filter = category
            .fetch()
            .map_err(|e| ManagerError::FetchError(e.to_string()))?;
        let download_link = filter.first().unwrap();

        let destination = self.install_path.join(&category.name);
        fs::create_dir_all(&destination).unwrap();

        // TODO: verify this is working for windows (.zip)?
        println!("Begin downloading blender and extract content!");
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
    fn save(&self) -> Result<(), ManagerError> {
        // strictly speaking, this function shouldn't crash...
        let data = serde_json::to_string(&self).unwrap();
        let path = Self::get_config_path();
        fs::write(path, data).map_err(|e| ManagerError::IoError(e.to_string()))
    }

    /// Return a reference to the vector list of all known blender installations
    pub fn get_blenders(&self) -> &Vec<Blender> {
        &self.blenders
    }

    /// Load the manager data from the config file.
    pub fn load() -> Self {
        // load from a known file path (Maybe a persistence storage solution somewhere?)
        // if the config file does not exist on the system, create a new one and return a new struct instead.
        let path = Self::get_config_path();
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(manager) = serde_json::from_str(&content) {
                return manager;
            } else {
                println!("Fail to deserialize manager data input!");
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

    /// Set path for blender download and installation
    pub fn set_install_path(&mut self, new_path: &Path) {
        // Consider the design behind this. Should we move blender installations to new path?
        self.install_path = new_path.to_path_buf().clone();
        self.has_modified = true;
    }

    /// Add a new blender installation to the manager list.
    pub fn add_blender(&mut self, blender: Blender) {
        self.blenders.push(blender);
        self.has_modified = true;
    }

    /// Check and add a local installation of blender to manager's registry of blender version to use from.
    pub fn add_blender_path(&mut self, path: &impl AsRef<Path>) -> Result<(), ManagerError> {
        let path = path.as_ref();
        let extension = BlenderCategory::get_extension().map_err(ManagerError::UnsupportedOS)?;
        // let str_path = path.as_os_str().to_str().unwrap().to_owned();

        let path = if path.ends_with(&extension) {
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

        self.add_blender(blender);
        Ok(())
    }

    /// Remove blender installation from the manager list.
    pub fn remove_blender(&mut self, blender: &Blender) {
        self.blenders.retain(|x| x.eq(blender));
        self.has_modified = true;
    }

    // TODO: Name ambiguous - clarify method name to clear and explicit
    pub fn fetch_blender(&mut self, version: &Version) -> Result<Blender, ManagerError> {
        match self.blenders.iter().find(|&x| x.get_version() == version) {
            Some(blender) => Ok(blender.to_owned()),
            None => {
                // could use as a warning message?
                println!(
                    "Target version is not installed! Downloading Blender {}!",
                    version
                );
                self.download(version)
            }
        }
    }

    pub fn have_blender(&self, version: &Version) -> bool {
        self.blenders.iter().any(|x| x.get_version() == version)
    }

    /// Fetch the latest version of blender available from Blender.org
    /// this function might be ambiguous. Should I use latest_local or latest_online?
    pub fn latest_local_avail(&mut self) -> Option<Blender> {
        // in this case I need to contact Manager class or BlenderDownloadLink somewhere and fetch the latest blender information
        let mut data = self.blenders.clone();
        data.sort();
        data.first().map(|v: &Blender| v.to_owned())
    }

    pub fn download_latest_version(&mut self) -> Result<Blender, ManagerError> {
        // in this case - we need to fetch the latest version from somewhere, download.blender.org will let us fetch the parent before we need to dive into

        // Dive into the parent directory, and get the last update version
        let mut home = BlenderHome::new().expect("Unable to get data");

        // sort by descending order
        home.list.sort_by(|a, b| b.cmp(a));
        let newest = home.list.first().unwrap();
        let link = newest.fetch_latest().unwrap();
        let path = link.download_and_extract(&self.install_path).unwrap();
        let blender =
            Blender::from_executable(path).map_err(|e| ManagerError::BlenderError { source: e })?;
        self.blenders.push(blender.clone());
        Ok(blender)
    }
}

// #[cfg(feature = "manager")]
impl Drop for Manager {
    fn drop(&mut self) {
        if self.has_modified || self.auto_save {
            if let Err(e) = self.save() {
                println!("Error saving manager file: {}", e);
            }
        }
    }
}
