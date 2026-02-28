use crate::blender::Blender;
use crate::models::blender_config::BlenderConfig;
use crate::models::download_link::{DownloadLink, Downloaded, NotDownloaded, Unpacked};
use crate::utils::{get_extension, get_valid_arch};
use crate::page_cache::PageCache;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::env::consts;
use std::path::Path;
use lazy_regex::{self, regex_captures_iter};
use semver::Version;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

// I have a situation where I can create this object, but not yet populate the download list.
// There are two ways to load the list, one from page cache, assuming we have already visited the website
// and the second is to load the website content, but also update the page cache to avoid revisitation and suspectible to DDoS/IP ban


#[derive(Debug, Error)]
pub enum BlenderCategoryError {
    #[error("Architecture type \"{0}\" is not supported!")]
    InvalidArch(String),
    #[error("Unsupported operating system: {0}")]
    UnsupportedOS(String),
    #[error("Not found")]
    NotFound,
    #[error("Io Error")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Default)]
pub(crate) struct NotLoaded;
#[derive(Debug, Default)]
pub(crate) struct Loaded {
    links: HashMap<Version, Package>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
enum Package {
    Metadata(DownloadLink<NotDownloaded>),
    Downloaded(DownloadLink<Downloaded>),
    Executable(DownloadLink<Unpacked>),
}

impl Package {
    pub fn get_version(&self) -> &Version {
        match self {
            Package::Metadata(link) => link.get_version(),
            Package::Downloaded(link) => link.get_version(),
            Package::Executable(link) => link.get_version(),
        }
    }

    pub fn get_package_ready(&self, destination: impl AsRef<Path>) -> Result<DownloadLink<Unpacked>, BlenderCategoryError> {
        match self {
            Package::Metadata(link) => {
                let download_link = link.clone().download(destination)?;
                Ok(download_link.extract()?)
            },
            Package::Downloaded(link) => {
                Ok(link.clone().extract()?)
            },
            Package::Executable(link) => 
                Ok(link.clone()),
        }
    } 
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct BlenderCategory<State> {
    base_url: Url,
    major: u64, 
    minor: u64,
    state: State
}

impl<State> PartialOrd for BlenderCategory<State> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.major.partial_cmp(&other.major) {
            Some(core::cmp::Ordering::Equal) => {
                self.minor.partial_cmp(&other.minor)
            }
            ord => return ord,
        }
        // self.state.partial_cmp(&other.state)
    }
}

impl<State> Ord for BlenderCategory<State> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            core::cmp::Ordering::Equal => {
                self.minor.cmp(&other.minor)
            },
            ord => ord
        }
    }
}

// TODO: Figure out how I can handle it here?
impl<State> PartialEq for BlenderCategory<State> {
    fn eq(&self, other: &Self) -> bool {
        match self.base_url.partial_cmp(&other.base_url) {
            Some(ord) => ord.is_eq(),
            None => false
        }
    }
}

impl<State> Eq for BlenderCategory<State> {}


impl BlenderCategory<NotLoaded> {
    pub fn new(base_url: Url, major: u64, minor: u64) -> BlenderCategory<NotLoaded> {
        // This would be a great place to load the links to validate the urls anyway.
        Self { 
            base_url, 
            major,
            minor, 
            state: NotLoaded 
        }
    }

    // TODO: [BUG] for some reason I was fetching this multiple of times already. Expensive to call. Profile test?
    pub fn fetch(self, cache: &mut PageCache) -> Result<BlenderCategory<Loaded>, BlenderCategoryError> {
        // this function is called everytime fetch is called. This seems to be slowing down the performance for this application usage.
        // TODO: because we changed the methodology of BlenderCategory's kind mechanism.. We will rely on the api call it can provide to us. 
        let content = cache.fetch_or_update(&self.base_url).map_err(BlenderCategoryError::Io)?;
        let current_arch = get_valid_arch().map_err(BlenderCategoryError::InvalidArch)?;
        let valid_ext = get_extension().map_err(BlenderCategoryError::UnsupportedOS)?;

        // <a href="(?<url>\w*-(?<major>\d*).(?<minor>\d*).(?<patch>\d*.)-(?<os>\w.*)-(?<arch>\w*)\.(?<ext>.*))">
        let iter = regex_captures_iter!(r#"<a href="(?<url>\w*-(?<major>\d*).(?<minor>\d*).(?<patch>\d*.)-(?<os>\w.*)-(?<arch>\w*)\.(?<ext>.*))">"#,&content);
        let links = iter.map(|c| c.extract()).fold(HashMap::new(), |mut map, (_, [url, major, minor, patch, os, arch, ext])| {
            
            // Check and see if the extension is valid
            if ext.ne(&valid_ext) {
                return map;
            }
            
            // Must match running operating system.
            if os.ne(consts::OS) {
                return map;
            }
            
            // Compatible with existing archtecture
            if arch.ne(&current_arch) {
                return map;
            }   

            let major: u64 = match major.parse() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{e:?}");
                    return map;
                }
            };

            let minor: u64 = match minor.parse() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{e:?}");
                    return map;
                }
            };

            let patch: u64 = match patch.parse() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{e:?}");
                    return map;
                }
            };

            let download_path = match self.base_url.join(url) {
                Ok(url) => url,
                Err(e) => {
                    eprintln!("{e:?}");
                    return map;
                }
            };

            let version = Version::new(major, minor, patch);
            let download_link = DownloadLink::new(url.to_string(), download_path, version.clone());
            let package = Package::Metadata(download_link);
            map.insert(version, package);
            map
        });

        Ok(BlenderCategory::<Loaded>{
            base_url: self.base_url,
            major: self.major,
            minor: self.minor,
            state: Loaded { links },
        })
    }
}

impl BlenderCategory<Loaded> {

    // Only used in this state.
    fn get_parent(&self) -> String {
        format!("Blender{}.{}", self.major, self.minor)
    }

    // fetch latest version of blender if it's available.
    pub(crate) fn fetch_latest(
        &mut self,
        config: &BlenderConfig
    ) -> Result<Blender, BlenderCategoryError> {
        // first I need is pop the entry from the links vector, as we're going to mutate the value.
        let package = &self.state.links.iter().fold(None, | latest: Option<&Package>, (version, link)| {
            if let Some(current) = latest {
                return match current.get_version().gt(version) {
                    true => latest,
                    false => Some(link) 
                }
            }
            Some(link)
        }).ok_or(BlenderCategoryError::NotFound)?;

        // repeated method as described below:
        let destination = config.get_download_destination(&self.get_parent()); 
        let link = package.get_package_ready(destination)?;
        self.state.links.insert(link.get_version().clone(), Package::Executable(link.clone()));
        let blender = link.get_blender().map_err(BlenderCategoryError::Io)?;
        Ok(blender)
    }

    // for the sake of this, we will trust that the user wants Blender from this.
    // Function renamed from retrieve
    /// Retrieve blender if it already installed, otherwise install from known source and return blender.
    pub fn get_blender(
        &mut self,
        config: &BlenderConfig,
        target_version: &Version,
    ) -> Result<Blender, BlenderCategoryError> {
        let package = self.state.links.get(&target_version).ok_or(BlenderCategoryError::NotFound)?;
        
        // repeated method as described above:
        let destination = config.get_download_destination(&self.get_parent()); 
        let link = package.get_package_ready(destination)?;
        self.state.links.insert(link.get_version().clone(), Package::Executable(link.clone()));
        let blender = link.get_blender().map_err(BlenderCategoryError::Io)?;
        Ok(blender)
    }
}

// content of https://download.blender.org/release/Blender{major}.{minor}/
impl<State> BlenderCategory<State> {
    // Use this to compare major/minor version without patch
    pub fn partial_version_match(&self, major: u64, minor: u64) -> Ordering {
        match self.major.cmp(&major) {
            Ordering::Equal => self.minor.cmp(&minor),
            itself => itself
        }
    }

    pub fn version_match(&self, version: &Version) -> Ordering {
        self.partial_version_match(version.major, version.minor)
    }
}
