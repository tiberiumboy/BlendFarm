use crate::blender::Blender;
use crate::models::blender_config::BlenderConfig;
use crate::models::download_link::DownloadLink;
use crate::utils::{get_extension, get_valid_arch};
use crate::page_cache::PageCache;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::env::consts;
use std::marker::PhantomData;
use std::path::PathBuf;
use lazy_regex::{self, regex_captures_iter};
use semver::Version;
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
pub(crate) struct Loaded;

// It may be a scalable thing in the future to add more features and rules, E.g. Groups or Remote/Pool/Shared?
#[derive(Debug)]
pub(crate) enum Kind {
    Website{base_url: Url, major: u64, minor: u64},
    Local{install_folder: PathBuf}
}

#[derive(Debug)]
pub(crate) struct BlenderCategory<State = NotLoaded> {
    kind: Kind,
    links: HashMap<Version, DownloadLink>,   // how can this vector hold various of state?
    state: PhantomData<State>
}

impl BlenderCategory<NotLoaded> {
    pub fn new(kind: Kind) -> BlenderCategory<NotLoaded> {
        // This would be a great place to load the links to validate the urls anyway.
        Self { kind, links: HashMap::new(), state: PhantomData::<NotLoaded> }
    }

    // TODO: [BUG] for some reason I was fetching this multiple of times already. Expensive to call. Profile test?
    pub fn fetch(self, cache: &mut PageCache) -> Result<BlenderCategory<Loaded>, BlenderCategoryError> {
        
        let mut vec = match self.kind {
            // I think this is just a link path instead?
            Kind::Local{ install_folder} => {
                // This ensures and checks Blender local directory. 
                // we will still provide download_url for the links, however, download_path will default to None.
                // Here we have created a blender category folder setup, treat it as Blender4.0 or something similar.
                // We should expect a .zip compressed of blender executables and maybe unzipped blender executables.
                // And sometimes blender executables without zip packages. (Externally Installed)


                Vec::new()
            },
            Kind::Website { base_url, major, minor } => {
                // this function is called everytime fetch is called. This seems to be slowing down the performance for this application usage.
                // TODO: because we changed the methodology of BlenderCategory's kind mechanism.. We will rely on the api call it can provide to us. 
                let content = cache.fetch_or_update(&base_url).map_err(BlenderCategoryError::Io)?;
                let current_arch = get_valid_arch().map_err(BlenderCategoryError::InvalidArch)?;
                let valid_ext = get_extension().map_err(BlenderCategoryError::UnsupportedOS)?;
    
                // <a href="(?<url>\w*-(?<major>\d*).(?<minor>\d*).(?<patch>\d*.)-(?<os>\w.*)-(?<arch>\w*)\.(?<ext>.*))">
                let iter = regex_captures_iter!(r#"<a href="(?<url>\w*-(?<major>\d*).(?<minor>\d*).(?<patch>\d*.)-(?<os>\w.*)-(?<arch>\w*)\.(?<ext>.*))">"#,&content);
                let mut list = Vec::with_capacity(iter.count());
                for (_, [url, major, minjor, patch, os, arch, ext]) in iter.map(|c| c.extract()) {
                    // Must match running operating system.
                    if os.ne(consts::OS) {
                        continue;
                    }
                    
                    // Compatible with existing archtecture
                    if arch.ne(&current_arch) {
                        continue;
                    }

                    let version = Version::new(major.parse().ok()?, minor.parse().ok()?, patch.parse().ok()?);
                    let download_link = DownloadLink::new(url.to_owned(), parent.join(url), version);
                    list.push(download_link);
                }

                list
            }};
        
        // let mut vec: Vec<DownloadLink> = regex
        //     .captures_iter(&content)
        //     .filter_map(|c| {
        //         let (_, [url, name, patch]) = c.extract();
        //         let url = self.url.join(url).ok()?;
        //         let patch = patch.parse().ok()?;
        //         let version = Version::new(self.major, self.minor, patch);
        //         Some(DownloadLink::new(name.to_owned(), url, version))
        //     })
        //     .collect();

        vec.sort_by(|a, b| b.cmp(a));

        let links = vec.iter()
            .fold(HashMap::with_capacity(vec.len()), |mut map, item| {
            map.insert(item.get_version().to_owned(), item.to_owned());
            map
        });

        Ok(BlenderCategory::<Loaded>{
            kind: self.kind,
            links: links,
            state: PhantomData::<Loaded>,
        })
    }
}

impl BlenderCategory<Loaded> {

    // I wonder about this... What am I'm fetching the latest from?
    pub(crate) fn fetch_latest(
        &mut self,
        config: &BlenderConfig
    ) -> Result<Blender, BlenderCategoryError> {
        // first I need to pop the entry from the links vector, as we're going to mutate the value.
        let link = self.links.iter().fold(None, | latest: Option<&DownloadLink>, (version, link)| {
            if let Some(current) = latest {
                return match current.get_version().gt(version) {
                    true => latest,
                    false => Some(link) 
                }
            }
            Some(link)
        });

        let blender = match link { 
            Some(dl) => {
                match dl {
                    Some(file: DownloadLink::<Downloaded>) => {

                    },
                    Some(executable: DownloadLink::<Unpacked>) => {
                        executable.get_blender()
                    }
                }
            },
            None => {
                return Err(BlenderCategoryError::NotFound)
            }
        };
        let destination = config.get_download_destination(&link);
        let download = link.download(destination).unwrap();
        let blender = download.extract().unwrap().get_blender().unwrap();
        Ok(blender)
    }

    // May not be in used yet?
    pub fn get_parent(&self) -> String {
        format!("Blender{}.{}", self.major, self.minor)
    }

    // for the sake of this, we will trust that the user wants Blender from this.
    pub fn retrieve(
        &mut self,
        config: &BlenderConfig,
        target_version: &Version,
    ) -> Result<Blender, BlenderCategoryError> {

        let entry = self.links.raw_entry_mut()
            .from_key(target_version)
            .or_insert( target_version, || {
                DownloadLink::new(name, url, version)
            })?;
        let destination = config.get_download_destination(link); 
        let download_link = entry.1.download(destination).unwrap();
        let extracted_link = download_link.extract().unwrap();        
        let blender = extracted_link.get_blender().unwrap();
        Ok(blender)
    }
}

// content of https://download.blender.org/release/Blender{major}.{minor}/
impl<State> BlenderCategory<State> {
    // Use this to compare major/minor version without patch
    pub fn partial_version_match(&self, major: u64, minor: u64) -> Ordering {
        match self.kind {
            Kind::Website { major: maj, minor: min, .. } => {
                match maj.cmp(&major) {
                    Ordering::Equal => min.cmp(&minor),
                    itself => itself
                }
            },
            Kind::Local { install_folder } => {
                self.links.fold
            }
        }
    }

    pub fn version_match(&self, version: &Version) -> Ordering {
        self.partial_version_match(version.major, version.minor)
    }
}

// TODO: Figure out how I can handle it here?
impl PartialEq for BlenderCategory {
    fn eq(&self, other: &Self) -> bool {
        match self.kind {
            Kind::Website { .. } => true,
            Kind::Local { .. } => false,
        }
        // self.major.eq(&other.major) && 
        // self.minor.eq(&other.minor)
    }
}

impl Eq for BlenderCategory {}

impl PartialOrd for BlenderCategory {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.partial_version_match(other.major, other.minor))
    }
}

impl Ord for BlenderCategory {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_version_match(other.major, other.minor)
    }
}

impl PartialEq for BlenderCategory<Loaded> {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url && self.major.eq(&other.major) && self.minor.eq(&other.minor)
    }
}

impl Eq for BlenderCategory<Loaded> {}

impl PartialOrd for BlenderCategory<Loaded> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.major.partial_cmp(&other.major) {
            Some(core::cmp::Ordering::Equal) => return self.minor.partial_cmp(&other.minor),
            ord => return ord,
        }
    }
}

impl Ord for BlenderCategory<Loaded> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.major.cmp(&other.major) {
            std::cmp::Ordering::Equal => self.minor.cmp(&other.minor),
            all => return all,
        }
    }
}