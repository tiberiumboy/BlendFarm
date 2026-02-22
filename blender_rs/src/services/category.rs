use crate::blender::Blender;
use crate::models::blender_config::BlenderConfig;
use crate::models::download_link::DownloadLink;
use crate::utils::{get_extension, get_valid_arch};
use crate::page_cache::PageCache;
use std::collections::HashMap;
use std::env::consts;
use std::marker::PhantomData;
use regex::Regex;
use lazy_regex::{self, regex_captures_iter};
use semver::Version;
use thiserror::Error;
use url::Url;

// I have a situation where I can create this object, but not yet populate the download list.
// There are two ways to load the list, one from page cache, assuming we have already visited the website
// and the second is to load the website content, but also update the page cache to avoid revisitation and suspectible to DDoS/IP ban

pub(crate) struct NotLoaded;
pub(crate) struct Loaded;

pub(crate) struct BlenderCategory<State = NotLoaded> {
    url: Url,
    major: u64,
    minor: u64,
    links: HashMap<Version, DownloadLink>,   // how can this vector hold various of state?
    state: PhantomData<State>
}

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

impl BlenderCategory<NotLoaded> {
    pub fn new(url: Url, major: u64, minor: u64) -> BlenderCategory<NotLoaded> {
        // This would be a great place to load the links to validate the urls anyway.
        Self { url, major, minor, links: HashMap::new(), state: PhantomData::<NotLoaded> }
    }

    // TODO: [BUG] for some reason I was fetching this multiple of times already. This seems expensive to call for some reason?
    pub fn fetch(self, cache: &mut PageCache) -> Result<BlenderCategory<Loaded>, BlenderCategoryError> {
        // this function is called everytime fetch is called. This seems to be slowing down the performance for this application usage.
        let content = cache.fetch_or_update(&self.url).map_err(BlenderCategoryError::Io)?;
        let arch = get_valid_arch().map_err(BlenderCategoryError::InvalidArch)?;
        let ext = get_extension().map_err(BlenderCategoryError::UnsupportedOS)?;

        // Regex rules - Find the url that matches version, computer os and arch, and the extension.
        // Don't cache this. Only used once and forget. Design to get information from website template. May change one day.
        // - There should only be one entry matching for this. Otherwise return error stating unable to find download path
        let pattern = format!(
            r#"<a href=\"(?<url>.*)\">(?<name>.*-{}\.{}\.(?<patch>\d*.)-{}.*{}*.{})<\/a>"#,
            self.major,
            self.minor,
            consts::OS,
            arch,
            ext,
        );

        let regex = regex_captures_iter!(format!(
            r#"<a href=\"(?<url>.*)\">(?<name>.*-{}\.{}\.(?<patch>\d*.)-{}.*{}*.{})<\/a>"#,
            self.major,
            self.minor,
            consts::OS,
            arch,
            ext,
        ));
        let mut vec: Vec<DownloadLink> = vec![];
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
            url: self.url,
            major: self.major,
            minor: self.minor,
            links: links,
            state: PhantomData::<Loaded>,
        })
    }
}

impl BlenderCategory<Loaded> {

    pub(crate) fn fetch_latest(
        &mut self,
        config: &BlenderConfig
    ) -> Result<Blender, BlenderCategoryError> {
        // first I need to pop the entry from the links vector, as we're going to mutate the value.
        let link = self
        // let link = self.links.first().ok_or(BlenderCategoryError::NotFound)?;
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
        &self,
        config: &BlenderConfig,
        target_version: &Version,
    ) -> Result<Blender, BlenderCategoryError> {

        let entry = self.links
            .iter()
            .find(|(version, download_link)| version.eq())
            .ok_or(BlenderCategoryError::NotFound)?;
        let destination = config.get_download_destination(link); 
        let download_link = entry.1.download(destination).unwrap();
        let extracted_link = download_link.extract().unwrap();        
        let blender = extracted_link.get_blender().unwrap();
        Ok(blender)
    }
}

// content of https://download.blender.org/release/Blender{major}.{minor}/
impl<State> BlenderCategory<State> {
    pub fn partial_version_match(&self, major: u64, minor: u64) -> bool {
        self.major.eq(&major) && self.minor.eq(&minor)
    }

    pub fn version_match(&self, version: &Version) -> bool {
        self.partial_version_match(version.major, version.minor)
    }
}

impl PartialEq for BlenderCategory {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url && self.major.eq(&other.major) && self.minor.eq(&other.minor)
    }
}

impl Eq for BlenderCategory {}

impl PartialOrd for BlenderCategory {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.major.partial_cmp(&other.major) {
            Some(core::cmp::Ordering::Equal) => return self.minor.partial_cmp(&other.minor),
            ord => return ord,
        }
    }
}

impl Ord for BlenderCategory {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.major.cmp(&other.major) {
            std::cmp::Ordering::Equal => self.minor.cmp(&other.minor),
            all => return all,
        }
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