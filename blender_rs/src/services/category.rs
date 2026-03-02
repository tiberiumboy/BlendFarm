use crate::blender::Blender;
use crate::services::packages::{package::Package, download_link::DownloadLink};
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

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct BlenderCategory {
    base_url: Url,
    major: u64, 
    minor: u64,
    links: HashMap<Version, Package>,
}

impl PartialOrd for BlenderCategory {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let result= match self.major.cmp(&other.major) {
            Ordering::Equal => self.minor.cmp(&other.minor),
            ord => ord
        };
        Some(result)
    }
}

impl Ord for BlenderCategory {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal =>  self.minor.cmp(&other.minor),
            ord => ord
        }
    }
}

impl PartialEq for BlenderCategory {
    fn eq(&self, other: &Self) -> bool {
        self.base_url.cmp(&other.base_url).is_eq()
    }
}

impl Eq for BlenderCategory {}

// content of https://download.blender.org/release/Blender{major}.{minor}/
impl BlenderCategory {
    
    // TODO: [BUG] for some reason I was fetching this multiple of times already. Expensive to call. Profile test?
    // should only be called once when this class is created.
    fn parse_content(content: &str) -> Result<HashMap<Version, &str>, BlenderCategoryError> {
        // this function is called everytime fetch is called. This seems to be slowing down the performance for this application usage.
        // TODO: because we changed the methodology of BlenderCategory's kind mechanism.. We will rely on the api call it can provide to us. 
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

            let version = Version::new(major, minor, patch);
            map.insert(version, url);
            map
        });

        Ok(links)
    }
    
    pub fn new(base_url: Url, major: u64, minor: u64, page_cache: &mut PageCache) -> Result<BlenderCategory, BlenderCategoryError> {
        // This would be a great place to load the links to validate the urls anyway.
        let content = page_cache.fetch_or_update(&base_url).map_err(BlenderCategoryError::Io)?;
        let links = Self::parse_content(&content)?;

        // replace this to handle this properly.
        let links = links.iter().fold( HashMap::new(), |map, (version, path)| {
            
            let url = match &base_url.join(path) {
                Ok(path) => path,
                Err(e) => {
                    eprintln!("{e:?}");
                    return map;
                }
            };

            let link = DownloadLink::new(url.to_owned(), version.to_owned())?; 
            
            let destination = ""; // TODO: where is install path?

            if let Ok(package) = Package::check_package(link, destination) {
                map.insert(version.to_owned(), package);
            }
            map
            // Package::get_package_ready(&self, destination)
        });

        Ok(Self { 
            base_url, 
            major,
            minor, 
            links
        })
    }

    // Only used in this state.
    fn get_parent(&self) -> String {
        format!("Blender{}.{}", self.major, self.minor)
    }

    // fetch latest version of blender if it's available.
    // TODO: Refactor this class down.
    pub(crate) fn fetch_latest(
        &mut self,
        download_path: impl AsRef<Path>,
    ) -> Result<Blender, BlenderCategoryError> {
        // first I need is pop the entry from the links vector, as we're going to mutate the value.
        let package = self.links.iter().fold(None, | result: Option<&Package>, (version, link)| {
            if let Some(latest) = result {
                if latest.get_version().ge(version) {
                    return result;
                }
            }
            Some(link)
        }).ok_or(BlenderCategoryError::NotFound)?;

        let link = package.get_package_ready(download_path)?;
        let _ = self.links.insert(link.get_version().clone(), Package::Executable(link.clone()));
        let blender = link.get_blender().map_err(BlenderCategoryError::Io)?;
        Ok(blender)
    }

    // for the sake of this, we will trust that the user wants Blender from this.
    // Function renamed from retrieve
    /// Retrieve blender if it already installed, otherwise install from known source and return blender.
    pub fn get_blender(
        &mut self,
        download_path: impl AsRef<Path>,
        target_version: &Version,
    ) -> Result<Blender, BlenderCategoryError> {
        let package = self.links.get(&target_version).ok_or(BlenderCategoryError::NotFound)?;
        
        // repeated method as described above:
        let link = package.get_package_ready(download_path)?;
        self.links.insert(link.get_version().clone(), Package::Executable(link.clone()));
        let blender = link.get_blender().map_err(BlenderCategoryError::Io)?;
        Ok(blender)
    }
    
    // return the version range for this category
    pub fn get_version(&self) -> Version {
        Version::new(self.major, self.minor, 0)    // will always be the lowest patch for category only.
    }
    
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
