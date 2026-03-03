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

use semver::Version;
use std::path::Path;
use std::{
    fs,
    io::{Error, ErrorKind},
    path::PathBuf,
};
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
    // Online interface (Download blender, look up version, etc)
    portal: Portal, // Todo this will get extracted away, leaving only blender configs.
    // page cache
    page_cache: PageCache,
}

// I have a config file, which contains list of local installed blender
// and install path. This Config struct is serialized and st

// Manager should only govern local installed blenders (Or blenders that was added by users)
impl Manager {
    pub fn new(config: BlenderConfig, portal: Portal, page_cache: PageCache) -> Self {
        Manager {
            config,
            portal,
            page_cache,
        }
    }

    /// Load the manager data from the config file.
    pub fn load(config_path: impl AsRef<Path>) -> Result<Self, ManagerError> {
        // load from a known file path (Maybe a persistence storage solution somewhere?)
        // if the config file does not exist on the system, create a new one and return a new struct instead.
        let config = BlenderConfig::load(config_path)?;
        let download_path = &config.install_path;
        // TODO: we'll load cache services here
        // let cache_path = &config.cache_path;

        let mut page_cache = PageCache::load().expect("Had issue loading PageCache!");
        let portal =
            Portal::new(download_path.clone(), &mut page_cache).expect("Must have portal running!");

        Ok(Self::new(config, portal, page_cache))
    }

    // Save the configuration, and restore to Unmodified state
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ManagerError> {
        let data = serde_json::to_string(&self.config).map_err(ManagerError::SerdeJson)?;
        fs::write(path, data).map_err(ManagerError::IoError)?;
        Ok(())
    }

    #[deprecated(note = "Provide me an example where this would be useful?")]
    #[allow(dead_code)]
    fn set_config(self, config: BlenderConfig) -> Manager {
        Self {
            config: config,
            portal: self.portal,
            page_cache: self.page_cache,
        }
    }

    /// Return a reference to the vector list of all known blender installations
    pub fn get_blenders(&self) -> Vec<&Blender> {
        self.config.get_blenders()
    }

    // TODO: provide a description what this function means?
    pub fn get_online_version(&self) -> Vec<(&Url, &Version)> {
        self.portal
            .get_downloads()
            .iter()
            .map(|package| {
                match package {
                    Package::Metadata(download_link) => {
                        (&download_link.download_url, download_link.get_version())
                    }
                    Package::Downloaded(downloaded) => {
                        (&downloaded.origin.download_url, downloaded.get_version())
                    }
                    Package::Bundle(bundle) => {
                        (&bundle.content.origin.download_url, bundle.get_version())
                    } // Package::Executable(custom) => ,
                }
                // (package.get_version())
            })
            .collect::<Vec<(&Url, &Version)>>()
    }

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
            page_cache: self.page_cache,
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

    // This is weird and a hack. We should let people try to give us valid blender struct. That's all we care about.
    /// Check and add a local installation of blender to manager's registry of blender version to use from.
    #[deprecated(
        note = "Consider asking for valid blender struct. Let the client try to get blender working first"
    )]
    pub fn add_blender_path(&mut self, path: &impl AsRef<Path>) -> Result<Blender, ManagerError> {
        // Here is where we verify the integrity of blender before adding to manager collection.
        let blender =
            Blender::from_executable(path).map_err(|e| ManagerError::BlenderError { source: e })?;

        if let Some(_old_value) = self.add_blender(&blender)? {
            eprintln!("Record updated");
        }

        // TODO: This is a hack - Would prefer to understand why program does not auto save file after closing.
        // Or look into better saving mechanism than this.

        // let _ = self.save()?;
        Ok(blender)
    }

    /// Remove blender installation from the manager list.
    pub fn remove_blender(mut self, blender: &Blender) -> Result<(), ManagerError> {
        let _ = &self.config.remove_blender(blender);
        Ok(())
    }

    /// Deletes the parent directory that blender reside in. This might be a dangerous function as this involves removing the directory blender executable is in.
    /// TODO: verify that this doesn't break macos path executable... Why mac gotta be special with appbundle?
    // If this is a dangerous function, we should instead make this private and handle it carefully.
    // TODO: Limiting scope visibility until we can make it private. I'm not sure where it's used atm, but making it work atm. 1 hour work
    pub fn delete_blender(self, blender: &Blender) -> Result<(), ManagerError> {
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
            }
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
