use crate::blender::Blender;
use crate::services::category::BlenderCategory;
use crate::services::packages::package::Package;
use crate::{blender::ManagerError, page_cache::PageCache};
use lazy_regex::regex_captures_iter;
use semver::Version;
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Debug)]
pub(crate) struct Portal {
    // list of category on download.blender.org
    list: Vec<BlenderCategory>,

    // Path to install and download zip content - Usually driven by BlenderConfig
    download_path: PathBuf,
}

impl Portal {
    const ROOT_URL: &str = "https://download.blender.org/release/";

    pub fn new(download_path: PathBuf, cache: &mut PageCache) -> Result<Self, ManagerError> {
        let list = Self::fetch(&download_path, cache)?;
        Ok(Portal {
            list,
            download_path,
        })
    }

    fn fetch(
        download_path: impl AsRef<Path>,
        cache: &mut PageCache,
    ) -> Result<Vec<BlenderCategory>, ManagerError> {
        let parent = Url::parse(Self::ROOT_URL).unwrap();

        // we fetch the content from the website above.
        // TODO: This could be dependency injected?
        let content = cache
            .fetch_or_update(&parent)
            .map_err(ManagerError::IoError)?;

        // Omit any blender version 2.8 and below
        let iter = regex_captures_iter!(
            r#"<a href="(?<url>.*)">Blender(?<major>[3-9]|\d{1,}).(?<minor>\d*)/</a>"#,
            &content
        );

        let mut list = iter.map(|c| c.extract()).fold(
            Vec::new(),
            |mut map: Vec<BlenderCategory>, (_, [url, major, minor])| {
                // Find a way to return the map instead? If it's invalid, log it and skip it.
                let url = match parent.join(url) {
                    Ok(url) => url,
                    Err(e) => {
                        eprintln!("{e:?}");
                        return map;
                    }
                };

                let major: u64 = match major.parse() {
                    Ok(val) => val,
                    Err(e) => {
                        eprintln!("{e:?}");
                        return map;
                    }
                };

                let minor: u64 = match minor.parse() {
                    Ok(val) => val,
                    Err(e) => {
                        eprintln!("{e:?}");
                        return map;
                    }
                };

                let category = BlenderCategory::new(url, major, minor, &download_path, cache);
                if let Ok(entry) = category {
                    map.push(entry);
                }
                map
            },
        );

        list.sort_by(|a, b| b.cmp(a));

        Ok(list)
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

    // find a way to hold reference to blender home here?
    // split this function
    /*
    pub fn download_latest_version(&mut self, cache: &mut PageCache) -> Result<Blender, ManagerError> {
        // in this case - we need to fetch the latest version from somewhere, download.blender.org will let us fetch the parent before we need to dive into
        // TODO: Find a way to replace these unwrap()
        let category =
            self.list.
                first()
                .map_or(
                    Err(
                        ManagerError::RequestError("Category list is empty! Did you clear the cache? Please connect to the internet to retrieve blender download list".to_string()))
                        , |c| Ok(c))?;

        category.get_blender(self.download_path, target_version)
        // let loaded = category.fetch(&mut self.cache).map_err(|e| ManagerError::FetchError(e.to_string()))?;
        // let blender = loaded.fetch_latest(&self.config).map_err(|e| ManagerError::FetchError(e.to_string()))?;
        // self.config.insert_blender(&blender);
        Ok(blender)
    }
    */

    /// Download Blender of matching version, install on this machine, and returns blender struct.
    /// This function will update PageCache if not previously visited. Hence mutation requirement.
    // TODO: Consider making a non-ambiguous function call get_target_blender(version)
    // TODO: Describe the action perform here then write down the instruction that should be used here.
    // could this be made async?
    pub(crate) fn download_blender(&mut self, version: &Version) -> Result<Blender, ManagerError> {
        // TODO: As a extra security measure, I would like to verify the hash of the content before extracting the files.
        // Main reason for fetching consts lib was to identify the host target hardware machine to provide extended diagnostic to manager for more info debugging through.
        let arch = std::env::consts::ARCH.to_owned();
        let os = std::env::consts::OS.to_owned();
        let download_path = &self.download_path.clone();
        let category =
            self.get_blender_state_by_version(version)
                .ok_or(ManagerError::DownloadNotFound {
                    arch,
                    os,
                    url: format!(
                        "Blender version {}.{} was not found!",
                        version.major, version.minor
                    ),
                })?;
        category
            .get_blender(download_path, &version)
            .map_err(ManagerError::Category)
    }

    // TODO: Write Unit test
    // Provide a minimum version to fetch the latest package.
    // This function will lock to the same major version, then picks minor version if it's greater than zero. Otherwise greatest known minor will be picked.
    // Patch will always pick the latest version as possible to follow with security updates.
    // Need to mut itself to populate latest download links.
    /*

    fn get_latest_download_link(&mut self, minimum_version: Option<&Version>) -> Option<Blender> {
        match minimum_version {
            Some(min_version) => {
                // TODO: Need to pop entry out of the list if it not pre-loaded, and update the record with loaded struct instead.
                let mut category = self.list.iter().fold(None, |result: Option<&BlenderCategoryState>, phase| {
                    // for this specific rule, we will lock to the major version and minor version, but pick the latest patch if possible.
                    let current_version = phase.get_version();

                    if min_version.major.ne(&current_version.major) {
                        return result;
                    }

                    // If the user picks 0 for minor, then we will pick the latest minor if possible.
                    if min_version.minor != 0 && min_version.minor.ne(&current_version.minor) {
                        return result;
                    }

                    if let Some(latest) = result {
                        if latest.get_version().ge(&current_version) {
                            return result
                        }
                    }

                    Some(phase)
                })?.clone();

                match category {
                    // I wonder how we can fetch latest?
                    BlenderCategoryState::Loaded(mut loaded) => match loaded.fetch_latest(&self.config) {
                        Ok(blender) => Some(blender),
                        Err(e) => {
                            eprintln!("[Fail to fetch latest! Returning None instead {e:?}");
                            None
                        }
                    },
                    BlenderCategoryState::NotLoaded(mut unloaded) => {
                        // first we need to load the category in. Otherwise return None with eprintln!
                        let fetched = unloaded.fetch(&mut self.cache);
                        match fetched {
                            Ok(mut loaded) => return match loaded.get_blender(&self.config, min_version) {
                                Ok(blender) => Some(blender),
                                Err(e) => {
                                    eprintln!("{e:?}");
                                    return None;
                                }
                            },
                            Err(e) => {
                                eprintln!("{e:?}");
                                return None;
                            }
                        }
                    }
                }
            },
            None =>  {
                let mut category = self.list.iter().fold(None, |result: Option<&BlenderCategoryState>, phase: &BlenderCategoryState| {
                    if let Some(latest) = result {
                        if latest.get_version().gt(&phase.get_version()) {
                            return result;
                        }
                    }
                    Some(phase)
                }).or_else(|| None)?;

                // Here I do some weird magic fuckery and all hell broke loose.
                match category {
                    BlenderCategoryState::Loaded(mut category) => {
                        category.fetch_latest(&self.config).ok()
                    },
                    BlenderCategoryState::NotLoaded(unloaded_category) => {

                        let mut loaded = unloaded_category.fetch(&mut self.cache).ok()?;
                        // TODO: It would be nice to update itself to append blender to config?
                        let blender = loaded.fetch_latest(&self.config).ok()?;
                        if let Some(old_value) = self.config.insert_blender(&blender) {
                            eprintln!("Blender updated! Old value: {old_value:?}");
                        }
                        Some(blender)
                    },
                }
            }
        }
    }
    */
}
