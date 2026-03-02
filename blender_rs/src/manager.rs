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
use crate::blender::Blender;
use crate::models::blender_config::BlenderConfig;
use crate::page_cache::PageCache;
use crate::services::category;
use crate::services::portal::Portal;


use semver::Version;
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
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serde_Json: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Category error: {0}")]
    Category(#[from] category::BlenderCategoryError),
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

#[derive(Debug)]
pub struct Manager {
    /// Store all known installation of blender directory information
    /// Manager's rulebook. Should only be available in this struct scope
    config: BlenderConfig,
    // List of Department. 
    // TODO: Extract this out as a separate component, like manager.
    portal: Portal,
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

// This struct is becoming a mess for a manager to take on.
// I need to separate out components and pieces.
// I have a config file, which contains list of local installed blender
// and install path. This Config struct is serialized and store in persistent folder location.

// Take the online download part into a separate components.
// Manager should only govern local installed blenders (Or blenders that was added by users)

impl Manager {
    /// Load the manager data from the config file.
    // TODO: How can I get page cache?
    pub fn load(page_cache: &mut PageCache) -> Self {
        // load from a known file path (Maybe a persistence storage solution somewhere?)
        // if the config file does not exist on the system, create a new one and return a new struct instead.
        let path = Self::get_config_path();
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(mut config) = serde_json::from_str::<BlenderConfig>(&content) {
                config.remove_invalid_blender();
                let download_path = &config.install_path;
                let portal = Portal::new(download_path.clone(), page_cache);
                let manager = Self {
                    config: config,
                    portal,
                };
                return manager;
            } else {
                println!("Fail to deserialize manager config file!");
            }
        } else {
            println!("File not found! Creating a new default one!");
        };


        // default case, create a new manager data and save it.
        let download_path = dirs::download_dir().unwrap().join("Blender");
        let portal = Portal::new(download_path, page_cache);
        let data = Manager {
            config: BlenderConfig::new(None, path),
            portal,
        };
        
        // TODO: Remove expects
        // We only need to get this far if we cannot load the file based on the condition above
        &data.save().expect("Should be able to save to storage");
        data
    }

    // Save the configuration, and restore to Unmodified state
    pub fn save(&self) -> Result<(), ManagerError> {
        // TODO: handle unwrap
        let data = serde_json::to_string(&self.config).map_err(ManagerError::SerdeJson)?;
        let path = Self::get_config_path();
        fs::write(path, data).map_err(ManagerError::IoError);
        Ok(())
    }

    #[deprecated(note = "Provide me an example where this would be useful?")]
    fn set_config(self, config: BlenderConfig) -> Manager {
        Self {
            config: config,
            portal: self.portal
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

    /// Return a reference to the vector list of all known blender installations
    pub fn get_blenders(&self) -> Vec<&Blender> {
        self.config.get_blenders()
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
    
    // It's used to display the information on the website.
    pub fn get_install_path(&self) -> &Path {
        &self.config.install_path
    }

    /// Set path for blender download and installation
    pub fn set_install_path(mut self, new_path: &Path) -> Manager {
        // Consider the design behind this. Should we move blender installations to new path?
        self.config.install_path = new_path.to_path_buf().clone();
        
        Self {
            config: self.config,
            portal: self.portal,
        }
    }

    /// Add a new blender installation to the manager list.
    // would require consuming manager.
    /// Returns old blender value that was replaced by the new updated value.
    pub fn add_blender(&mut self, blender: &Blender) -> Result<Option<Blender>, ManagerError> {
        // make sure it doesn't exist already.
        // Returns None if previously doesn't exist, or Some(old_value) when the record has been updated.
        Ok(self.config.insert_blender(blender))
    }

    /// Check and add a local installation of blender to manager's registry of blender version to use from.
    /// We should expect 
    pub fn add_blender_path(&mut self, path: &impl AsRef<Path>) -> Result<Blender, ManagerError> {
        // Here is where we verify the integrity of blender before adding to manager collection.
        let blender =
            Blender::from_executable(path).map_err(|e| ManagerError::BlenderError { source: e })?;

        if let Some(_old_value) = self.add_blender(&blender)? {
            eprintln!("Record updated");
        }

        // TODO: This is a hack - Would prefer to understand why program does not auto save file after closing.
        // Or look into better saving mechanism than this.
        let _ = self.save()?;
        Ok(blender)
    }

    /// Remove blender installation from the manager list.
    pub fn remove_blender(mut self, blender: &Blender) -> Result<(), ManagerError> {
        &self.config.remove_blender(blender);
        Ok(())
    }

    /// Deletes the parent directory that blender reside in. This might be a dangerous function as this involves removing the directory blender executable is in.
    /// TODO: verify that this doesn't break macos path executable... Why mac gotta be special with appbundle?
    // If this is a dangerous function, we should instead make this private and handle it carefully.
    // TODO: Limiting scope visibility until we can make it private. I'm not sure where it's used atm, but making it work atm. 1 hour work
    pub(crate) fn delete_blender(self, blender: &Blender) -> Result<(), ManagerError> {
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
        self.remove_blender(blender)?;
        Ok(())
    }

    /// This will first check if blender is installed locally, otherwise download the version online.
    pub fn fetch_blender(&mut self, version: &Version) -> Result<Blender, ManagerError> {
        match self.config.get_blender(version) {
            Some(blender) => Ok(blender.clone()),
            None => {
                let blender = self.portal.download_blender(version)?;
                // Expects no history previously stored due to match conditions above. If it breaks, something is seriously wrong.
                if let Some(old_value) = self.add_blender(&blender)? {
                    panic!("Record contain existing record, but filter above assure we didn't have it? {old_value:?}\n{:?}", &blender);
                }
                Ok(blender)
            },
        }
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
}

impl AsRef<PathBuf> for Manager {
    fn as_ref(&self) -> &PathBuf {
        &self.config.install_path
    }
}

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
