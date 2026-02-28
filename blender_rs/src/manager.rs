/*
    Developer blog:
    This manager class will serve the following purpose:
    - Keep track of blender installation on this active machine.
    - Prevent downloading of the same blender version if we have one already installed.
    - If user fetch for list of installation, verify all path exist before returning the list.
    - Implements download and install code

    Story:
        Pretend this as a factory. What should a manager do to perform this program execution.
        This manager responsibility accounts for holding the list of known blender installation.
            If the installation does not exist, we provide customer the ability to install Blender from known location. (Blender.org)
            We download, extract, and symbolic link (Feature). 
            - Updated BlenderCategory to use different method of blender location.
                Originally default to use BlenderOrg, but could point to Local (Can request intranet distribution service- Feature)?)
            - Manager implements PhantomData to acknowledge modified data. This expose additional function to help ensure user can save the
                configuration modification (New blender installation, download new version, cache refresh, etc). Limits API usage once we update phantom state to save or load. 

    */
use crate::blender::Blender; // , BlenderError
// use crate::models::blender_scene::BlenderScene;
// use crate::models::peek_response::PeekResponse;
// use crate::models::render_setting::RenderSetting;
use crate::models::blender_config::BlenderConfig;
use crate::models::download_link::Unpacked;
use crate::models::{download_link::DownloadLink};
use crate::services::category::{BlenderCategory, Loaded, NotLoaded};
use crate::page_cache::PageCache;

use lazy_regex::regex_captures_iter;
use semver::Version;
use std::marker::PhantomData;
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
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum BlenderCategoryState {
    Loaded(BlenderCategory<Loaded>),
    NotLoaded(BlenderCategory<NotLoaded>),
}

#[derive(Debug)]
pub struct Manager {
    /// Store all known installation of blender directory information
    /// Manager's rulebook. Should only be available in this struct scope
    config: BlenderConfig,
    // List of Department. 
    list: Vec<BlenderCategoryState>,
    // Accountant 
    cache: PageCache
}

/*
impl Default for Manager<Unmodified> {
    // the default method implement should be private because I do not want people to use this function.
    // instead they should rely on "load" function instead.
    fn default() -> Manager<Unmodified> {
        let install_path = dirs::download_dir().unwrap().join("Blender");
        let config = BlenderConfig::new(None,install_path);
        let mut cache =
            PageCache::load().expect("Page Cache should have permission to load content!");

        let list = self.fetch_categories(&mut cache).unwrap_or_else(|_| Vec::new());

        Self {
            config,
            list,
            // cache,   Could be used as dependency injection?
            state: PhantomData::<Unmodified>,
        }
    }
} */

impl Manager {
    /// Load the manager data from the config file.
    // TODO: How can I get page cache?
    pub fn load(page_cache: PageCache) -> Self {
        // load from a known file path (Maybe a persistence storage solution somewhere?)
        // if the config file does not exist on the system, create a new one and return a new struct instead.
        let path = Self::get_config_path();
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(mut config) = serde_json::from_str::<BlenderConfig>(&content) {
                config.remove_invalid_blender_path();
                let manager = Self {
                    config: config,
                    // TODO: Find a way to load Blender Category here?
                    list:Vec::new(),    
                    cache: page_cache,
                };
                return manager;
            } else {
                println!("Fail to deserialize manager config file!");
            }
        } else {
            println!("File not found! Creating a new default one!");
        };


        // default case, create a new manager data and save it.
        let data = Manager {
            config: BlenderConfig::new(None, path),
            list: Vec::new(),
            cache: page_cache,
            state: PhantomData::<Modified>,
        };
        
        // TODO: Remove expects
        data.save().expect("Should be able to save to storage")
    }

    // Save the configuration, and restore to Unmodified state
    pub fn save(self) -> Result<(), ManagerError> {
        // strictly speaking, this function shouldn't crash...
        let data = serde_json::to_string(&self.config).unwrap();
        let path = Self::get_config_path();
        fs::write(path, data).map_err(ManagerError::IoError);
        Ok(())
    }

    // TODO: split this up into handling kinds.
    fn fetch(self, cache: &mut PageCache) -> Result<Manager, ManagerError> {
        let parent = Url::parse("https://download.blender.org/release/").unwrap();
        
        // we fetch the content from the website above.
        // TODO: This could be dependency injected?
        let content = cache
            .fetch_or_update(&parent)
            .map_err(ManagerError::IoError)?;

        // Omit any blender version 2.8 and below
        let iter = regex_captures_iter!(
            r#"<a href="(?<url>.*)">Blender(?<major>[3-9]|\d{1,}).(?<minor>\d*)/</a>"#,
            &content);
        
        let mut list = iter
            .map(|c| c.extract())
            .fold(Vec::new(), |mut map: Vec<BlenderCategoryState>, (_, [url, major, minor])| {
                // Find a way to return the map instead? If it's invalid, log it and skip it.
                let url = match parent.join(url) {
                    Ok(url) => url,
                    Err(_e) => {
                        // TODO: Implement logger here for debugging purposes.
                        return map
                    }
                };

                let major: u64 = match major.parse() {
                    Ok(val) => val,
                    Err(e) => {
                        // TODO: Implement logger here for debugging purposes.
                        return map
                    }
                };
                let minor: u64 = match minor.parse() {
                    Ok(val) => val,
                    Err(e) => {
                        // TODO: Implement logger here for debugging purposes.
                        return map
                    }
                };
                let category = BlenderCategory::new(url, major, minor);
                if let Ok(category) = category.fetch(cache) {
                    let state = BlenderCategoryState::Loaded(category);
                    map.push(state);
                }

                map
        });
                
        list.sort_by(|a, b| b.cmp(a));

        Ok(Manager::<Modified, > {
            config: self.config,
            list: list,
            cache: self.cache,
            state: PhantomData::<Modified>
        })
    }

    fn set_config(self, config: BlenderConfig) -> Manager<Modified> {
        Manager::<Modified> {
            config: config,
            list: self.list,
            cache: self.cache,
            state: PhantomData::<Modified>,
        }
    }

    /// Returns the directory path where the configuration file is stored.
    /// This is stored under the library usage of dirs::config_dir() + "BlendFarm" - the application name by default.
    /// This ensure directory must exist before returning PathBuf, else report back as permission issue. We must have a place to save the files to.
    fn get_config_dir(user_pref: Option<PathBuf>) -> PathBuf {
        let path = match user_pref {
            Some(path) => path.join("BlendFarm"),
            None => dirs::config_dir().unwrap().join("BlendFarm"),
        };

        // ensure path location must exist - we guarantee permission access here.
        fs::create_dir_all(&path).expect("Unable to create directory!");
        path
    }

    // this path should always be fixed and stored under machine specific.
    // this path should not be shared across machines.
    fn get_config_path() -> PathBuf {
        // TODO: see about getting user pref?
        Self::get_config_dir(None).join("BlenderManager.json")
    }

    /// Download Blender of matching version, install on this machine, and returns blender struct.
    /// This function will update PageCache if not previously visited. Hence mutation requirement.
    // TODO: Is this Manager Responsibility? Refactor this down?
    // TODO: Consider making a non-ambiguous function call get_target_blender(version)
    pub fn download_blender(&mut self, version: &Version) -> Result<Blender, ManagerError> {
        // TODO: As a extra security measure, I would like to verify the hash of the content before extracting the files.
        let arch = std::env::consts::ARCH.to_owned();
        let os = std::env::consts::OS.to_owned();

        let blender =
            &self.get_blender_by_version(version)
                .ok_or(ManagerError::DownloadNotFound {
                    arch,
                    os,
                    url: format!(
                        "Blender version {}.{} was not found!",
                        version.major, version.minor
                    ),
                })?;
        
        // let destination = self.config.get_download_destination(&download_link);
        // let download_link = download_link.download(destination).map_err(|e| ManagerError::IoError(e.to_string()))?;
        // let download_link = download_link.extract().map_err(|e| ManagerError::IoError(e.to_string()))?;
        // let blender = download_link.get_blender().map_err(|e| ManagerError::IoError(e.to_string()))?;
        
        let manager = self.add_blender(&blender);
        manager.save().unwrap();
        Ok(blender.clone())
    }
    
    /// Return a reference to the vector list of all known blender installations
    // TODO: Identify where this is used and see if it make sense in general architecture design?
    pub fn get_blenders(&self) -> Vec<Blender> {
        todo!("read description");
        // &self.config.get_blenders()
    }

    // May no longer in use?
    fn get_download_link(&self, _target_version: &Version) -> Option<&DownloadLink<Unpacked>> {
        todo!("Return blender object instead. Please rewrite the API to use Blender struct");
    }

    // TODO: Write Unit test
    fn get_latest_download_link(&self, minimum_version: Option<&Version>) -> Option<&DownloadLink<Unpacked>> {
        match minimum_version { 
            Some(min_version) => {
                self.download_links.iter().fold(None, |result, (version, downloadlink)| {
                    if min_version.gt(version) {
                        return result
                    } 

                    if let Some(prev) = result {
                        if prev.get_version().gt(version) {
                            return result
                        }
                    }
                    Some(downloadlink)   
                })
            },
            None =>    
                self.download_links.iter().fold(None, |result: Option<&DownloadLink>, (version, item)| {
                    if let Some(latest) = result {
                        return match latest.get_version().lt(version) {
                            true => Some(item),
                            false => Some(latest)
                        }
                    }
                    Some(item)
                })
            }
        }
    
    /// Peek is a function design to read and fetch information about the blender file.
    // TODO: see where this is used, as this seems like blendfile already have information?
    // Is this code even in used at all?
    /* 
    pub async fn peek(&mut self, blendfile: BlendFile) -> Result<PeekResponse, BlenderError> {
        todo!("Please see note. Where is this funciton used, and consider refactoring on using BlendFile information instead.");
        let (major, minor) = blendfile.get_partial_version();
        // simple upcast
        let (major, minor) = (major as u64, minor as u64);
        
        // using scope to drop manager usage.
        let blend_version = {
            // TODO: Refactor this script so we can ask the manager to fetch the information without accessing category at all.
            match self.have_blender_partial(major, minor) {
                Some(blend) => blend.get_version().clone(),
                None => self
                .get_latest_version_patch(major, minor)
                .unwrap_or(Version::new(major, minor, 0)),
            }
        };
        
        let scene_info: SceneInfo = blendfile.into();
        let selected_scene = scene_info.selected_scene();
        let selected_camera = scene_info.selected_camera();
        
        let render_setting: RenderSetting = scene_info.clone().render_setting();
        let current = BlenderScene::new(selected_scene, selected_camera, render_setting);
        
        // TODO: Rethink structure?
        let result = PeekResponse::new(
            blend_version, // Why?
            scene_info.frame_start,
            scene_info.frame_end,
            scene_info.cameras,
            scene_info.scenes,
            current,
        );
        
        Ok(result)
    }
    */

    pub fn get_install_path(&self) -> &Path {
        &self.config.install_path
    }

    /// Set path for blender download and installation
    pub fn set_install_path(mut self, new_path: &Path) -> Manager::<Modified> {
        // Consider the design behind this. Should we move blender installations to new path?
        self.config.install_path = new_path.to_path_buf().clone();
        
        Manager::<Modified> {
            config: self.config,
            list: self.list,
            cache: self.cache,
            state: PhantomData::<Modified>,
        }
    }

    /// Add a new blender installation to the manager list.
    // would require consuming manager.
    pub fn add_blender(mut self, blender: &Blender) -> Manager::<Modified> {
        // make sure it doesn't exist already.
        // Use Manager::<Modify>() method here!
        if let Some(old) = &self.config.append_blender(blender) {
            println!("Blender was updated! Old config: {old:?}")
        }
        
        Manager::<Modified> {
            config: self.config,
            list: self.list,
            cache: self.cache,
            state: PhantomData::<Modified>
        }
    }

    /// Check and add a local installation of blender to manager's registry of blender version to use from.
    /// We should expect 
    pub fn add_blender_path(self, path: &impl AsRef<Path>) -> Result<Blender, ManagerError> {
        let path = path.as_ref();

        //     // Do not worry about this. For now, treat the url as content already unpacked by user.
        // let extension = get_extension().map_err(ManagerError::UnsupportedOS)?;
        // let path = if path
        //     .extension()
        //     .is_some_and(|e| extension.contains(e.to_str().unwrap()))
        // {
        //     // Create a folder name from given path
        //     let folder_name = &path
        //         .file_name()
        //         .unwrap()
        //         .to_os_string()
        //         .to_str()
        //         .unwrap()
        //         .replace(&extension, "");

        //     DownloadLink::extract_content(path, folder_name)
        //         .map_err(|e| ManagerError::UnableToExtract(e.to_string()))
        // } else {
        //     // for MacOS - User will select the app bundle instead of actual executable, We must include the additional path
        //     match std::env::consts::OS {
        //         "macos" => Ok(path.join("Contents/MacOS/Blender")),
        //         _ => Ok(path.to_path_buf()),
        //     }
        // }?;
        
        // Here is where we verify the integrity of blender before adding to manager collection.
        let blender =
            Blender::from_executable(path).map_err(|e| ManagerError::BlenderError { source: e })?;

        let manager = self.add_blender(&blender);
        // TODO: This is a hack - Would prefer to understand why program does not auto save file after closing.
        // Or look into better saving mechanism than this.
        let _ = manager.save()?;
        Ok(blender)
    }

    /// Remove blender installation from the manager list.
    pub fn remove_blender(mut self, blender: &Blender) -> Manager::<Modified> {
        &self.config.remove_blender(blender);
        Manager::<Modified> {
            config: self.config,
            list: self.list,
            cache: self.cache,
            state: PhantomData::<Modified>,
        }
    }

    /// Deletes the parent directory that blender reside in. This might be a dangerous function as this involves removing the directory blender executable is in.
    /// TODO: verify that this doesn't break macos path executable... Why mac gotta be special with appbundle?
    pub fn delete_blender(self, blender: &Blender) -> Manager::<Modified> {
        // this deletes blender from the system. You have been warn!
        // BEWARE - MacOS is special that the executable path is referencing inside the bundle. I would need to get the app path instead of the bundle inside.
        if std::env::consts::OS == "macos" {
            panic!(
                "Need to handle mac app path reference instead of path inside bundle! {:?}",
                blender.get_executable()
            );
        }
        // I'm still concern about this, why are we deleting the parent? Need to perform unit test for this to make sure it doesn't delete anything else.
        fs::remove_dir_all(blender.get_executable().parent().unwrap()).unwrap();
        self.remove_blender(blender)
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
    // TODO: Why do I need to make this public?
    // pub fn fetch_download_list(&self) -> Option<Vec<DownloadLink>> {
    //     match &self.download_links.is_empty() {
    //         false => Some(self.download_links.clone()),
    //         true => None,
    //     }
    // }

    pub fn have_blender(&self, version: &Version) -> Option<&Blender> {
        self.config.get_blender(version)
    }

    pub fn have_blender_partial(&self, major: u64, minor: u64) -> Option<&Blender> {
        self.config.get_blender_partial(major, minor)
    }

    /// Fetch the latest version of blender available from Blender.org
    /// this function might be ambiguous. Should I use latest_local or latest_online?
    pub fn latest_local_avail(&mut self, version: Option<&Version>) -> Option<&Blender> {
        // in this case I need to contact Manager class or BlenderDownloadLink somewhere and fetch the latest blender information
        // I think the data is already sorted to begin with? No need to resort this list again.
        self.config.get_latest_blender_available(version)
    }

    pub fn latest_online(&mut self) -> Result<Blender, ManagerError> {

        let link = self.get_latest_download_link(None);
        
        // TODO: It would be nice to fetch online if we received None from the link above.
        // However as of the time right now, I'm focus on functionality getting this working
        let link = link.expect("Must be connected online!");
        let destination = self.config.get_download_destination(&link);
        let download_link = link.download(destination).map_err(|e| ManagerError::IoError(e.to_string()))?;
        let download_link = download_link.extract().map_err(|e| ManagerError::IoError(e.to_string()))?;
        // Download the executable and extract the contents.
        // let blender = link.download_and_extract(self.config.install_path).map_err(|e: Error| ManagerError::UnableToExtract(e.to_string()))?;
        let blender = download_link.get_blender().map_err(|e| ManagerError::IoError(e.to_string()))?;
        self.add_blender(&blender);
        Ok(blender)
    }

    // find a way to hold reference to blender home here?
    // split this function
    pub fn download_latest_version(&mut self) -> Result<Blender, ManagerError> {
        // in this case - we need to fetch the latest version from somewhere, download.blender.org will let us fetch the parent before we need to dive into
        // TODO: Find a way to replace these unwrap()
        let category = 
            self.list.
                first()
                .map_or(
                    Err(
                        ManagerError::RequestError("Category list is empty! Did you clear the cache? Please connect to the internet to retrieve blender download list".to_string()))
                        , |c| Ok(c))?;
        
        let loaded = category.fetch(&mut self.cache).map_err(|e| ManagerError::FetchError(e.to_string()))?;
        let blender = loaded.fetch_latest(&self.config).map_err(|e| ManagerError::FetchError(e.to_string()))?;
        self.config.append_blender(&blender);
        Ok(blender)
    }

    fn get_blender_by_version(&self, version: &Version) -> Option<Blender> {
        self.list
            .raw_entry_mut()
            .from_key(version)
            .or_insert_with({
                    let name = "";
                    let url = Url::parse("").unwrap();
                    DownloadLink::new(name, url, &version)
                }
            )
            // .iter()
            // .find(|&c| c.version_match(version))
            // .map_or(None, |c| {
            //     c.retrieve(&self.config, version)
            //         .map_or(None, |l| Some(l.to_owned()))
            // })
    }

    // I may want to change this to see if I'm picking the one from locally installed or from remote
    pub fn get_latest_version_patch(&mut self, major: u64, minor: u64) -> Option<Version> {
        // Get the latest patch from blender home
        self.list
            .iter()
            .find(|v| v.partial_version_match(major, minor))
            .map_or(None, |c| {
                c.fetch_latest()
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

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn should_pass() {
        // let _manager = Manager::load();
    }
    /* 
        fn test_download_blender_home_link() {
            let mut manager = Manager::load();
            let link = manager.latest_local_avail(None).or(manager
                .download_latest_version()
                .map_or(None, |l| Some(l.to_owned())));
            match link {
                Some(link) => {
                    dbg!(link);
                }
                None => println!("No blender found and unable to connect to internet! Skipping!"),
            }
        }
    */    

    // TODO: Write unit test for Drop if that's possible?
}
