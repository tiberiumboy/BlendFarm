use crate::blender::Blender;
use crate::services::category::BlenderCategory;
use crate::services::packages::package::Package;
use crate::{blender::ManagerError, page_cache::PageCache};
use lazy_regex::regex_captures_iter;
use semver::Version;
use std::env::consts::{ARCH, OS};
use std::path::{Path, PathBuf};
use url::Url;

// I want this struct to remain private for now.
// This struct should be used as an component to fetch from reliable resources.
// alternatively, I could swap this out and use my own custom storage solution.
#[derive(Debug)]
pub(crate) struct Portal {
    // list of category on download.blender.org
    list: Vec<BlenderCategory>,

    // Path to install and download zip content - Usually driven by BlenderConfig
    download_path: PathBuf,
}

impl Portal {
    const ROOT_URL: &str = "https://download.blender.org/release/";

    fn new(download_path: PathBuf, list: Vec<BlenderCategory>) -> Self {
        Self {
            list,
            download_path,
        }
    }

    // Only used in this state.
    #[inline]
    fn get_parent(major: u64, minor: u64) -> String {
        format!("Blender{major}.{minor}")
    }

    // function generator for closures in regex patterns.
    fn generate_blender_category(
        parent: &Url,
        url: &str,
        major: &str,
        minor: &str,
        download_path: &Path,
        cache: &mut PageCache,
    ) -> Option<BlenderCategory> {
        // create the link for blender category location
        let url = match parent.join(url) {
            Ok(path) => path,
            Err(e) => {
                eprintln!("unable to join paths! {e:?}");
                return None;
            }
        };

        let major: u64 = match major.parse() {
            Ok(val) if val >= 3 => val,
            Ok(_) => {
                // TODO: impl a debug switch mode to allow printing these verbose console logs.
                // eprintln!("Omitting outdated major version.");
                return None;
            }
            Err(e) => {
                eprintln!("{e:?}");
                return None;
            }
        };

        let minor: u64 = match minor.parse() {
            Ok(val) => val,
            Err(e) => {
                eprintln!("{e:?}");
                return None;
            }
        };

        // Append the download path to the category's folder path.
        // E.g. ~/Downloads/Blender/Blender4.2/
        let destination_path = download_path.join(Self::get_parent(major, minor));

        if let Ok(content) = &cache.fetch_or_update(&url) {
            if let Ok(links) = BlenderCategory::parse_content(&content, &url, &destination_path) {
                return Some(BlenderCategory::new(url, major, minor, links));
            }
        }
        None
    }

    /// This method will fetch the list of blender category that's listed under download.blender.org/releases webpage.
    /// This helps prefetch information ahead of time for cache lookup. It does require a bit of initial setup to ensure
    /// files are available and ready to be used. Note we will not download Blender until we receive user invocation to do so.
    pub fn fetch(
        download_path: impl AsRef<Path>,
        cache: &mut PageCache,
    ) -> Result<Self, ManagerError> {
        let parent = Url::parse(Self::ROOT_URL)
            .map_err(|e| return ManagerError::UrlParseError(e.to_string()))?;

        // we fetch the content from the website above.
        let content = cache
            .fetch_or_update(&parent)
            .map_err(ManagerError::IoError)?;

        // Omit any blender version 2.8 and below
        // TODO: BUG: It's not omitting version 2.8 and below. Would like to omit any version 3.8 and below for now.
        let iter = regex_captures_iter!(
            r#"<a href="(?<url>.*)">Blender(?<major>[3-9]|\d{1,}).(?<minor>\d*)/</a>"#,
            &content
        );

        let mut list = iter.map(|c| c.extract()).fold(
            Vec::new(),
            |mut map: Vec<BlenderCategory>, (_, [url, major, minor])| {
                if let Some(category) = Portal::generate_blender_category(
                    &parent,
                    url,
                    major,
                    minor,
                    download_path.as_ref(),
                    cache,
                ) {
                    map.push(category);
                }
                map
            },
        );

        list.sort_by(|a, b| b.cmp(a));
        Ok(Self::new(download_path.as_ref().to_path_buf(), list))
    }

    // TODO: Find a better way to deal with this
    // why do i want to get blender state?
    fn get_blender_state_by_version(&mut self, version: &Version) -> Option<&mut BlenderCategory> {
        // need to pop the element from the collection.
        self.list.iter_mut().fold(None, |result, item| {
            let current_version = item.get_version();

            if current_version.major.ne(&version.major) {
                return result;
            }

            if version.minor != 0 && current_version.minor.ne(&version.minor) {
                return result;
            }

            if let Some(latest) = &result {
                if latest.get_version().le(&current_version) {
                    return result;
                }
            }

            Some(item)
        })
    }

    pub fn get_downloads(&self) -> Vec<&Package> {
        let mut result = Vec::with_capacity(self.list.capacity());
        for item in &self.list {
            let mut col = item.get_packages();
            result.append(&mut col);
        }
        result
    }

    pub fn check_compressed_blender_by_file_name(&self, zip_file_name: &str) -> Option<PathBuf> {
        self.list.iter().fold(None, |_, category| {
            category.get_packages().iter().find_map(|package| {
                let path = match package {
                    Package::Downloaded(downloaded) => Some(downloaded.content.clone()),
                    Package::Bundle(bundle) => Some(bundle.content.content.clone()),
                    _ => None,
                };

                if let Some(zip) = &path {
                    if zip.eq(zip_file_name) {
                        return path;
                    }
                }
                None
            })
        })
    }

    /// retrieve the blender executable if it's already downloaded, otherwise download the executable and return Blender instance.
    /// Should we download the blender instances from the internet?
    #[deprecated(note = "This is not used? Is this true?")]
    #[allow(dead_code)]
    pub fn fetch_blender(&mut self, version: &Version) -> Result<Blender, ManagerError> {
        let download_path = self.download_path.clone();
        if let Some(category) = self.get_blender_state_by_version(version) {
            return category
                .get_blender(&download_path, version)
                .map_err(ManagerError::Category);
        }

        Err(ManagerError::FetchError("Unknown, reached EOF!".to_owned()))
    }

    /// Download Blender of matching version, install on this machine, and returns blender struct.
    /// This function will update PageCache if not previously visited. Hence mutation requirement.
    // TODO: could this be made async?
    pub(crate) fn download_blender(&mut self, version: &Version) -> Result<Blender, ManagerError> {
        // TODO: As a extra security measure, I would like to verify the hash of the content before extracting the files.
        // Main reason for fetching consts lib was to identify the host target hardware machine to provide extended diagnostic to manager for more info debugging through.
        let download_path = &self.download_path.clone();

        let category =
            self.get_blender_state_by_version(version)
                .ok_or(ManagerError::DownloadNotFound {
                    arch: ARCH.to_owned(),
                    os: OS.to_owned(),
                    url: format!(
                        "Blender version {}.{} for {}-{} was not found!",
                        version.major, version.minor, OS, ARCH
                    ),
                })?;
        // generate a destination for the folder path
        // e.g. ~/Downloads/Blender/Blender4.3/
        let destination = download_path.join(Self::get_parent(version.major, version.minor));
        category
            .get_blender(destination, &version)
            .map_err(ManagerError::Category)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn mock_portal() -> Portal {
        let list = Vec::new();
        let download_path = PathBuf::new();
        
        Portal {
            list, 
            download_path
        }
    }
}