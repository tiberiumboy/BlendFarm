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
use crate::services::packages::package::{Package, PackageT};
use crate::services::portal::Portal;

use figment::{
    providers::{Env, Format, /* Toml, Yaml, */ Json},
    Figment,
};
use semver::Version;
use std::path::Path;
use std::sync::{OnceLock, RwLock};
use std::{fs, path::PathBuf};
use thiserror::Error;
use url::Url;

// I know I added toml/yaml path here beside json...? Did I not pull down the updates?
pub(crate) const SETTINGS_PATH: &str = "BlendFarm/BlenderManager.json";
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
    #[error("Config Error: {0}")]
    FigmentError(#[from] figment::Error),
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

// TODO: Look into OnceCell andsee how I can utilize lazy implementations?
#[derive(Debug)]
pub struct Manager {
    /// Store all known installation of blender directory information
    /// Manager's rulebook. Should only be available in this struct scope
    // Soon to be replaced using Figment library
    config: RwLock<BlenderConfig>,
    portal: RwLock<Portal>,
    // page cache
    // page_cache: RwLock<PageCache>,
}

// I have a config file, which contains list of local installed blender
// and install path. This Config struct is serialized and st

// Manager should only govern local installed blenders (Or blenders that was added by users)
impl Manager {
    fn cache() -> &'static RwLock<PageCache> {
        static CACHE: OnceLock<RwLock<PageCache>> = OnceLock::new();
        CACHE.get_or_init(|| {
            let cache = PageCache::load().expect("Unable to load Page Cache!");
            RwLock::new(cache)
        })
    }

    fn new(config: BlenderConfig, portal: Portal) -> Self {
        Manager {
            config: RwLock::new(config),
            portal: RwLock::new(portal),
            // page_cache: RwLock::new(page_cache),
        }
    }

    pub fn check_compressed_by_file_name(&self, zip_file_name: &str) -> Option<PathBuf> {
        self.portal
            .read()
            .unwrap()
            .check_compressed_blender_by_file_name(zip_file_name)
    }

    /// Load the manager data from the config file.
    pub fn load() -> Result<Self, ManagerError> {
        // if the config file does not exist on the system, create a new one and return a new struct instead.
        // TODO: Figure out where Figment is loading the config path from?
        let config = match Figment::new()
            // .merge(Toml::file(SETTINGS_PATH))
            // .merge(Yaml::file(SETTINGS_PATH))
            .merge(Env::prefixed("BlendFarm_"))
            .join(Json::file(SETTINGS_PATH))
            .extract::<BlenderConfig>()
        {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Unable to extract figment! {e:?}");
                BlenderConfig::default()
            }
        };

        let download_path = &config.install_path;

        // TODO: we'll load cache services here
        // let cache_path = &config.cache_dir;
        // let mut page_cache = PageCache::load().expect("Had issue loading PageCache!");
        let mut cache = Self::cache().write().unwrap();
        let portal = Portal::fetch(&download_path, &mut cache)?;
        cache.save()?;
        Ok(Self::new(config, portal))
    }

    // Save the configuration
    pub fn save(&self) -> Result<(), ManagerError> {
        todo!("Missing implementation for saving BlenderConfig back to file using Figment");
        // TODO: Use Yaml instead of json
        // let data = serde_json::to_string(&self.config).map_err(ManagerError::SerdeJson)?;
        // fs::write(self.config.get_config_path(), data).map_err(ManagerError::IoError)
    }

    /// Returns a list of url path to download and version (For UI models)
    pub fn get_online_version(&self) -> Vec<(Url, Version)> {
        self.portal
            .read()
            .unwrap()
            .get_downloads()
            .iter()
            .map(|package| match package {
                Package::Metadata(download_link) => (
                    download_link.download_url.to_owned(),
                    download_link.get_version().to_owned(),
                ),
                Package::Downloaded(downloaded) => (
                    downloaded.origin.download_url.to_owned(),
                    downloaded.get_version().to_owned(),
                ),
                Package::Bundle(bundle) => (
                    bundle.content.origin.download_url.to_owned(),
                    bundle.get_version().to_owned(),
                ),
            })
            .collect::<Vec<(Url, Version)>>()
    }

    // It's used to display the information on the website.
    // pub fn get_install_path(&self) -> &Path {
    //     &self.config.read().unwrap().install_path
    // }
    pub fn get_config(&self) -> BlenderConfig {
        self.config.read().unwrap().clone()
    }

    /// Set path for blender download and installation
    pub fn set_install_path(&self, new_path: &Path) {
        // Consider the design behind this. Should we move blender installations to new path?
        self.config.write().unwrap().install_path = new_path.to_path_buf().clone();
    }

    /// Add a new blender installation to the manager list.
    /// Returns old blender value that was replaced by the new updated value.
    pub fn add_blender(&self, blender: &Blender) -> Result<Option<Blender>, ManagerError> {
        // Returns None if previously doesn't exist, or Some(old_value) when the record has been updated.
        Ok(self.config.write().unwrap().insert_blender(blender))
    }

    /// Remove blender installation from the manager list.
    pub fn remove_blender(&self, blender: &Blender) -> Result<(), ManagerError> {
        let _ = self.config.write().unwrap().remove_blender(blender);
        Ok(())
    }

    /// Deletes the parent directory that blender reside in. This might be a dangerous function as this involves removing the directory blender executable is in.
    /// TODO: verify that this doesn't break macos path executable... Why mac gotta be special with appbundle?
    // If this is a dangerous function, we should instead make this private and handle it carefully.
    // TODO: Limiting scope visibility until we can make it private. I'm not sure where it's used atm, but making it work atm. 1 hour work
    #[allow(dead_code)]
    pub(crate) fn delete_blender(&self, blender: &Blender) -> Result<(), ManagerError> {
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
        match self.config.read().unwrap().get_blender(version) {
            Some(blender) => Ok(blender.clone()),
            None => {
                let blender = self.portal.write().unwrap().download_blender(version)?;
                // Expects no history previously stored due to match conditions above. If it breaks, something is seriously wrong.
                if let Some(old_value) = self.add_blender(&blender)? {
                    panic!("Record contain existing record, but filter above assure we didn't have it? {old_value:?}\n{:?}", &blender);
                }
                Ok(blender)
            }
        }
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
