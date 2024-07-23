/*
    Developer blog:
    This manager class will serve the following purpose:
    - Keep track of blender installation on this active machine.
    - Prevent downloading of the same blender version if we have one already installed.
    - If user fetch for list of installation, verify all path exist before returning the list.
    - Implements download and install code
*/
use crate::blender::Blender;
use crate::models::download_link::DownloadLink;
use crate::page_cache::{PageCache, PageCacheError};
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{env::consts, fs, io, path::PathBuf, time::SystemTime};
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum ManagerError {
    #[error("Unsupported OS: {0}")]
    UnsupportedOS(String),
    #[error("Unsupported Archtecture: {0}")]
    UnsupportedArch(String),
    #[error("Unable to fetch download from the source! {0}")]
    FetchError(String),
    #[error("Cannot find target download link for blender! os: {os} | arch: {arch} | url: {url}")]
    DownloadNotFound {
        arch: String,
        os: String,
        url: String,
    },
    #[error("Unable to fetch blender! {source}")]
    Reqwest {
        #[from]
        source: reqwest::Error,
    },
    //     // TODO: Find meaningful error message to represent from this struct class?
    #[error("IO Error: {source}")]
    Io {
        #[from]
        source: io::Error,
    },
    #[error("Url ParseError: {source}")]
    UrlParseError {
        #[from]
        source: url::ParseError,
    },
    #[error("Page cache error: {source}")]
    PageCacheError {
        #[from]
        source: PageCacheError,
    },
}

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

impl Manager {
    // this path should always be fixed and stored under machine specific.
    // this path should not be shared across machines.
    fn get_config_path() -> PathBuf {
        let path = dirs::config_dir().unwrap().join("Blender");
        fs::create_dir_all(&path).expect("Unable to create directory!");
        path.join("BlenderManager.json")
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

    fn save(&self) -> Result<(), ManagerError> {
        let data = serde_json::to_string(&self).unwrap();
        let path = Self::get_config_path();
        fs::write(path, data).or_else(|e| Err(ManagerError::Io { source: e }))
    }

    /// Add a new blender installation to the manager list.
    pub fn add(&mut self, blender: &Blender) {
        self.blenders.push(blender.clone());
        self.has_modified = true;
    }

    /// Remove blender installation from the manager list.
    pub fn remove(&mut self, blender: &Blender) {
        self.blenders.retain(|x| x.eq(blender));
        self.has_modified = true;
    }

    /// Return extension matching to the current operating system (Only display Windows(zip), Linux(tar.xz), or macos(.dmg)).
    pub fn get_extension() -> Result<String, ManagerError> {
        // TODO: Find a better way to re-write this - I assume we could take advantage of the compiler tags to return string literal without switch statement like this?
        let extension = match consts::OS {
            "windows" => ".zip",
            "macos" => ".dmg",
            "linux" => ".tar.xz",
            os => return Err(ManagerError::UnsupportedOS(os.to_string())),
        };

        Ok(extension.to_owned())
    }

    /// fetch current architecture (Currently support x86_64 or aarch64 (apple silicon))
    fn get_valid_arch() -> Result<String, ManagerError> {
        match consts::ARCH {
            "x86_64" => Ok("64".to_owned()),
            "aarch64" => Ok("arm64".to_owned()),
            value => return Err(ManagerError::UnsupportedArch(value.to_string())),
        }
    }

    /// Return the pattern matching to identify correct blender download link
    fn generate_blender_pattern_matching(version: &Version) -> Result<String, ManagerError> {
        let extension = Self::get_extension()?;
        let arch = Self::get_valid_arch()?;

        // Regex rules - Find the url that matches version, computer os and arch, and the extension.
        // - There should only be one entry matching for this. Otherwise return error stating unable to find download path
        let match_pattern = format!(
            r#"(<a href=\"(?<url>.*)\">(?<name>.*-{}\.{}\.{}.*{}.*{}.*\.[{}].*)<\/a>)"#,
            version.major,
            version.minor,
            version.patch,
            consts::OS,
            arch,
            extension
        );

        Ok(match_pattern)
    }

    pub fn get_blender(&mut self, version: &Version) -> Result<Blender, ManagerError> {
        match self.blenders.iter().find(|x| x.get_version() == version) {
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

    fn download(&mut self, version: &Version) -> Result<Blender, ManagerError> {
        // if the manager already have the blender version installed, use that instead of downloading a new instance of version.

        // TODO: As a extra security measure, I would like to verify the hash of the content before extracting the files.
        // I would hope that this line should never fail...? Unless the user isn't connected to the internet, or the url path is blocked by IT infrastructure?
        let url = match Url::parse("https://download.blender.org/release/") {
            Ok(url) => url,
            Err(e) => return Err(ManagerError::UrlParseError { source: e }),
        };

        // In the original code - there's a comment implying we should use cache as much as possible to avoid IP Blacklisted. TODO: Verify this in Blender community about this.
        let mut cache = match PageCache::load(SystemTime::now()) {
            Ok(cache) => cache,
            Err(e) => return Err(ManagerError::PageCacheError { source: e }),
        };

        // TODO: How did BlendFarm fetch all blender version?
        // working out a hack to rely on website availability for now. Would like to simply get the url I need to download and run blender.
        // could this be made into a separate function?
        let path = format!("Blender{}.{}/", version.major, version.minor);
        let url = match url.join(&path) {
            Ok(url) => url,
            Err(e) => return Err(ManagerError::UrlParseError { source: e }),
        };

        // fetch the content of the subtree information
        let content = match cache.fetch(&url) {
            Ok(content) => content,
            Err(e) => return Err(ManagerError::PageCacheError { source: e }),
        };

        let match_pattern = Self::generate_blender_pattern_matching(&version)?;

        // unwrap() is used here explicitly because I know that the above regex command will not fail.
        let regex = Regex::new(&match_pattern).unwrap();
        let download_link = match regex.captures(&content) {
            Some(info) => {
                // remove extension from file name
                let name = info["name"].to_string();
                let path = info["url"].to_string();
                let url = &url.join(&path).unwrap();
                let ext = Self::get_extension().unwrap();
                println!("Download link generated!");
                DownloadLink::new(name, ext, url.clone())
            }
            None => {
                return Err(ManagerError::DownloadNotFound {
                    arch: consts::ARCH.to_string(),
                    os: consts::OS.to_string(),
                    url: url.to_string(),
                })
            }
        };

        let destination = self.install_path.join(&path);
        fs::create_dir_all(&destination).unwrap();

        // TODO: verify this is working for windows (.zip)?
        println!("Begin downloading blender and extract content!");
        match download_link.download_and_extract(&destination) {
            Ok(destination) => {
                dbg!(&destination);
                let blender = Blender::from_executable(destination.to_owned()).unwrap();
                self.add(&blender);
                self.save().unwrap();
                Ok(blender)
            }
            Err(e) => Err(e),
        }
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        if self.has_modified || self.auto_save {
            self.save().unwrap();
        }
    }
}
