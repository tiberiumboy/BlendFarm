use crate::{
    blender::Blender,
    services::{
        category::BlenderCategoryError,
        packages::{
            bundle::Bundle, /* custom::Custom, */ download_link::DownloadLink,
            downloaded::Downloaded, BlenderPath,
        },
    },
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub(crate) trait PackageT {
    fn get_version(&self) -> &Version;
}

/*
    Package is thought of having a single source of truth to get blender specific versions.
    Depends on the phase, we would need to download if it's not found within local system.
    Otherwise, use the uncompressed version of the executable and treat as final source of truth.
    We have method implementations to gracefully fetch the package.
*/
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Package {
    // Only contains download link
    Metadata(DownloadLink),
    // contains download origin and path to downloaded content
    Downloaded(Downloaded),
    // Contains complete set, do not download, do not unpact, should provide executable path
    Bundle(Bundle),
    // Only contains executable location, user defined variable
    // Executable(Custom),
    // TODO: Feature request - Would there ever be a chances for any of the data above would mutate and become invalid? Test this out?
    // In some extreme cases - if something goes wrong, we can put them in malform state until user corrects them into Bundle state, or lesser state known.
    // Malformed { origin: Option<Url>, downloaded: Option<PathBuf>, executable: Option<PathBuf> },
}

impl Package {
    // This is design to check internal source and verify the package is indeed correct, otherwise return the current state it failed in
    // we are only provided with a source.
    pub fn check_package(
        link: DownloadLink,
        destination: impl AsRef<Path>,
    ) -> Result<Package, BlenderCategoryError> {
        // This ideally should return something...
        // we'll start here first
        let downloaded = match link.content_exist(destination) {
            Ok(downloaded) => downloaded,
            Err(download_link) => return Ok(Package::Metadata(download_link)),
        };

        match downloaded.check_unpacked() {
            Ok(bundle) => Ok(Package::Bundle(bundle)),
            // Do not unzip, simply return the current state and move on.
            Err(downloaded) => Ok(Package::Downloaded(downloaded)),
        }
    }

    // This is an attempt to download from url, extract, and provide package ready to be used for blender.
    pub fn get_package_ready(
        self,
        destination: impl AsRef<Path>,
    ) -> Result<Package, BlenderCategoryError> {
        match self {
            Package::Metadata(link) => Ok(link
                .download(&destination)
                .map_err(BlenderCategoryError::Io)?
                .extract()),
            Package::Downloaded(link) => Ok(link.extract()),
            // These two are ok since they were already ready to begin with
            // Package::Executable(..) => Ok(self),
            Package::Bundle(..) => Ok(self),
        }
    }
}

impl PackageT for Package {
    fn get_version(&self) -> &Version {
        match self {
            Package::Metadata(link) => link.get_version(),
            Package::Downloaded(content) => content.get_version(),
            Package::Bundle(bundle) => bundle.get_version(),
        }
    }
}

impl BlenderPath for Package {
    // without modifying itself, we can only provide as much.
    fn get_blender(&self) -> Option<Blender> {
        match self {
            Package::Bundle(bundle) => bundle.get_blender(),
            _ => None,
        }
    }
}
