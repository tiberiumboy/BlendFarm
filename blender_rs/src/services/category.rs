use crate::blender::Blender;
use crate::services::packages::BlenderPath;
use crate::services::packages::{download_link::DownloadLink, package::Package};
use crate::utils::{get_extension, get_valid_arch};
use lazy_regex::{self, regex_captures_iter};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::env::consts;
use std::io::Error as IoError;
use std::path::Path;
use url::Url;

// I have a situation where I can create this object, but not yet populate the download list.
// There are two ways to load the list, one from page cache, assuming we have already visited the website
// and the second is to load the website content, but also update the page cache to avoid revisitation and suspectible to DDoS/IP ban

#[derive(Debug)]
pub enum BlenderCategoryError {
    InvalidArch(String),
    UnsupportedOS(String),
    NotFound,
    Io(IoError),
}

// Blender Category is a sub page within download.blender.org/release page, this page contains all of the urls associated with arch, os, and bits.
// In this struct, on initialization, we parse the content of this website and generate a structure data we can run functions on.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct BlenderCategory {
    base_url: Url,
    major: u64,
    minor: u64,
    links: HashMap<Version, Package>,
}

impl PartialOrd for BlenderCategory {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let result = match self.major.cmp(&other.major) {
            Ordering::Equal => self.minor.cmp(&other.minor),
            ord => ord,
        };
        Some(result)
    }
}

impl Ord for BlenderCategory {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => self.minor.cmp(&other.minor),
            ord => ord,
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
    // TODO: Try to make this private as much as possible! this parse content is a hack to help reduce function complexity.
    //       But instead it creates a spaghetti mess. Will handle this with context of some sort in the future.
    pub(crate) fn parse_content(
        content: &str,
        base_url: &Url,
        download_path: impl AsRef<Path>,
    ) -> Result<HashMap<Version, Package>, BlenderCategoryError> {
        let current_arch =
            get_valid_arch().map_err(|e| BlenderCategoryError::InvalidArch(e.into()))?;
        let valid_ext =
            get_extension().map_err(|e| BlenderCategoryError::UnsupportedOS(e.into()))?;

        // The rule has changed. The extension will not include a period symbol. Additional period will be treated as extension of extension, e.g. tar.xz
        let iter = regex_captures_iter!(
            r#"<a href="(?<url>\w*-(?<major>\d*).(?<minor>\d*).(?<patch>\d*.)-(?<os>\w.*)-(?<arch>\w*)\.(?<ext>.*))">"#,
            &content
        );
        let links = iter.map(|c| c.extract()).fold(
            HashMap::new(),
            |mut map, (_, [url, major, minor, patch, os, arch, ext])| {
                // Check and see if the extension is valid
                if ext.ne(valid_ext) {
                    return map;
                }

                // Must match running operating system.
                if os.ne(consts::OS) {
                    return map;
                }

                // Compatible with existing archtecture
                if arch.ne(current_arch) {
                    return map;
                }

                // *filter out any major version 3 or below. We will not be supporting legacy blender at the moment.
                let major: u64 = match major.parse() {
                    Ok(v) if v >= 3 => v,
                    Ok(_) => return map,
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
                let url = match base_url.join(&url) {
                    Ok(url) => url,
                    Err(e) => {
                        eprintln!("{e:?}");
                        return map;
                    }
                };
                let link = match DownloadLink::new(url, version.clone()) {
                    Ok(link) => link,
                    Err(e) => {
                        eprintln!("{e:?}");
                        return map;
                    }
                };
                if let Ok(package) = Package::check_package(link, &download_path) {
                    map.insert(version, package);
                }

                map
            },
        );

        Ok(links)
    }

    pub fn new(base_url: Url, major: u64, minor: u64, links: HashMap<Version, Package>) -> Self {
        Self {
            base_url,
            major,
            minor,
            links,
        }
    }

    // fetch latest version of blender if it's available.
    // TODO: Refactor this class down.
    // pub(crate) fn fetch_latest(
    //     &mut self,
    //     download_path: impl AsRef<Path>,
    // ) -> Result<Blender, BlenderCategoryError> {
    //     // first I need is pop the entry from the links vector, as we're going to mutate the value.
    //     let package = self
    //         .links
    //         .iter()
    //         .fold(None, |result: Option<&Package>, (version, link)| {
    //             if let Some(latest) = result {
    //                 if latest.get_version().ge(version) {
    //                     return result;
    //                 }
    //             }
    //             Some(link)
    //         })
    //         .ok_or(BlenderCategoryError::NotFound)?;

    //     let target_version = package.get_version().clone();
    //     let package = self
    //         .links
    //         .remove(&target_version)
    //         .expect("Would expect at least a valid location?");

    //     let link = package.get_package_ready(download_path)?;
    //     let blender = link.get_blender().ok_or(BlenderCategoryError::NotFound)?;
    //     if let Some(old_value) = self.links.insert(link.get_version().clone(), link) {
    //         eprintln!("Not possible? Value must have been popped to mutate value before insert back in \n{old_value:?}");
    //     }
    //     Ok(blender)
    // }

    // Function renamed from retrieve
    /// Retrieve blender if it already installed, otherwise install from known source and return blender.
    pub fn get_blender(
        &mut self,
        download_path: impl AsRef<Path>,
        target_version: &Version,
    ) -> Result<Blender, BlenderCategoryError> {
        // pop entry. we can mutate this now.
        let package = self
            .links
            .remove(target_version)
            .ok_or(BlenderCategoryError::NotFound)?;

        // repeated method as described above:
        let link = package.get_package_ready(download_path)?;
        let blender = link.get_blender().ok_or(BlenderCategoryError::NotFound)?;

        // append back to the record.
        if let Some(old_value) = self.links.insert(target_version.clone(), link) {
            eprintln!("Somehow received a record updated? Not possible? {old_value:?}");
        }
        Ok(blender)
    }

    pub fn get_packages(&self) -> Vec<&Package> {
        self.links
            .iter()
            .map(|(_, package)| package)
            .collect::<Vec<&Package>>()
    }

    // return the version range for this category
    pub fn get_version(&self) -> Version {
        Version::new(self.major, self.minor, 0) // will always be the lowest patch for category only.
    }
}
